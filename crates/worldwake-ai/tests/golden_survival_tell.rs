//! Golden tests for the survival tell roadmap landing.

mod golden_harness;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use golden_harness::*;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{
    AgentBeliefStore, CommodityKind, DriveThresholds, EntityId, PerceptionSource, Tick,
    WorkstationTag,
};
use worldwake_sim::{ActionTraceDetail, ActionTraceKind, TellBeliefDeltaKind, TellCommitResult};

const SURVIVAL_TICKS: u32 = 1440;

#[derive(Clone, Debug, Eq, PartialEq)]
struct AgentSurvivalObservation {
    alive: bool,
    critical_thresholds: DriveThresholds,
    critical_need_runs: SurvivalNeedRunTracker,
    committed_actions: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TellRelayObservation {
    tell_tick: Tick,
    listener_subject_belief_tick: Tick,
    listener_first_arrival_tick: Tick,
    listener_first_food_commit_tick: Tick,
    told_subject: EntityId,
    orchard_place: EntityId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SurvivalTellObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    agents: BTreeMap<String, AgentSurvivalObservation>,
    stuck_idle_windows: Vec<StuckIdleWindow>,
    relay: TellRelayObservation,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-tell.ron")
}

fn load_survival_tell_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("survival tell scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival tell scenario should spawn");
    let mut harness = GoldenHarness::from_simulation_state(&spawned.state);
    let agents = harness
        .world
        .query_name_and_agent_data()
        .map(|(agent, _, _)| agent)
        .collect::<Vec<_>>();
    for agent in agents {
        let place = harness
            .world
            .effective_place(agent)
            .expect("scenario agents should have a starting place");
        let local_entities = harness
            .world
            .entities()
            .filter(|entity| harness.world.effective_place(*entity) == Some(place))
            .filter(|entity| *entity != agent)
            .collect::<Vec<_>>();
        seed_actor_beliefs(
            &mut harness.world,
            &mut harness.event_log,
            agent,
            &local_entities,
            Tick(0),
            PerceptionSource::DirectObservation,
        );
    }
    harness.driver.enable_tracing();
    harness.enable_action_tracing();
    (harness, def)
}

fn scenario_place_id(def: &ScenarioDef, place_name: &str) -> EntityId {
    let slot = def
        .places
        .iter()
        .position(|place| place.name == place_name)
        .and_then(|index| u32::try_from(index).ok())
        .expect("scenario place should exist within u32 slot bounds");
    EntityId {
        slot,
        generation: 0,
    }
}

fn named_agents(h: &GoldenHarness) -> BTreeMap<String, EntityId> {
    h.world
        .query_name_and_agent_data()
        .map(|(entity, name, _)| (name.0.clone(), entity))
        .collect()
}

fn known_food_entities_at_place(store: &AgentBeliefStore, place: EntityId) -> Vec<EntityId> {
    store
        .iter_known_entities()
        .filter_map(|(entity, state)| {
            (state.last_known_place == Some(place)
                && state.workstation_tag == Some(WorkstationTag::OrchardRow)
                && state
                    .resource_source
                    .as_ref()
                    .is_some_and(|source| source.commodity == CommodityKind::Apple))
            .then_some(*entity)
        })
        .collect()
}

fn contract_run_limit_overrides(
    limits: Option<&worldwake_cli::scenario::types::SurvivalCriticalRunLimitsDef>,
) -> SurvivalCriticalRunLimitOverrides {
    let Some(limits) = limits else {
        return SurvivalCriticalRunLimitOverrides::default();
    };

    SurvivalCriticalRunLimitOverrides {
        hunger: limits.hunger,
        thirst: limits.thirst,
        fatigue: limits.fatigue,
        bladder: limits.bladder,
        dirtiness: limits.dirtiness,
    }
}

fn run_survival_tell() -> SurvivalTellObservation {
    let (mut h, def) = load_survival_tell_harness();
    let contract =
        expect_survival_health_contract(def.survival_health_contract.as_ref(), "survival tell")
            .clone();
    let agents = named_agents(&h);
    let scout = *agents
        .get("Scout Una")
        .expect("scenario should include Scout Una");
    let listener = *agents
        .get("Listener Bea")
        .expect("scenario should include Listener Bea");
    let orchard_place = scenario_place_id(&def, "North Orchard");
    let critical_thresholds = agents
        .iter()
        .map(|(name, agent)| {
            (
                name.clone(),
                h.world
                    .get_component_drive_thresholds(*agent)
                    .copied()
                    .expect("survival tell agents should have drive thresholds"),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut critical_need_runs = agents
        .keys()
        .cloned()
        .map(|name| (name, SurvivalNeedRunTracker::default()))
        .collect::<BTreeMap<_, _>>();
    let mut idle_state: BTreeMap<String, (Option<u32>, u16, u32)> = agents
        .keys()
        .cloned()
        .map(|name| (name, (None, 0, 0)))
        .collect();
    let mut stuck_idle_windows = Vec::new();

    let listener_start_food_beliefs = known_food_entities_at_place(
        h.world
            .get_component_agent_belief_store(listener)
            .expect("listener should have a belief store"),
        orchard_place,
    );
    assert!(
        listener_start_food_beliefs.is_empty(),
        "listener should begin without any orchard food belief; beliefs={listener_start_food_beliefs:?}"
    );

    let scout_start_food_beliefs = known_food_entities_at_place(
        h.world
            .get_component_agent_belief_store(scout)
            .expect("scout should have a belief store"),
        orchard_place,
    );
    assert!(
        !scout_start_food_beliefs.is_empty(),
        "scout should begin with a local orchard food belief from same-place observation"
    );

    let mut relay: Option<TellRelayObservation> = None;
    let mut listener_first_arrival_tick: Option<Tick> = None;
    let mut listener_first_food_commit_tick: Option<Tick> = None;
    let mut ever_colocated_after_start = false;

    for tick_num in 0..SURVIVAL_TICKS {
        h.step_once();
        let tick = Tick(u64::from(tick_num));
        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");

        if h.world.effective_place(scout) == h.world.effective_place(listener) {
            ever_colocated_after_start = true;
        }

        for (agent_name, agent) in &agents {
            let needs = h
                .world
                .get_component_homeostatic_needs(*agent)
                .expect("scenario agents should always have needs");
            critical_need_runs
                .get_mut(agent_name)
                .expect("run tracker should exist")
                .observe(
                    needs,
                    critical_thresholds
                        .get(agent_name)
                        .expect("threshold should exist for agent"),
                );

            let had_action = golden_harness::agent_has_non_failed_action_or_active(
                &h,
                action_sink,
                *agent,
                tick,
            );
            let (start, max_need, count) = idle_state
                .get_mut(agent_name)
                .expect("every agent should have idle state");
            if had_action {
                if let Some(s) = start.take()
                    && *count >= contract.max_idle_window_ticks_with_elevated_need
                    && *max_need > contract.elevated_need_floor.value()
                {
                    stuck_idle_windows.push(StuckIdleWindow {
                        agent_name: agent_name.clone(),
                        start_tick: s,
                        end_tick: tick_num.saturating_sub(1),
                        max_need_at_start: *max_need,
                    });
                }
                *count = 0;
            } else {
                if start.is_none() {
                    *start = Some(tick_num);
                    *max_need = max_need_value(needs);
                }
                *count += 1;
            }
        }

        if listener_first_arrival_tick.is_none()
            && h.world.effective_place(listener) == Some(orchard_place)
        {
            listener_first_arrival_tick = Some(tick);
        }

        for event in action_sink.events_for_at(listener, tick) {
            if matches!(event.kind, ActionTraceKind::Committed { .. })
                && matches!(event.action_name.as_str(), "harvest_apples" | "eat")
                && listener_first_food_commit_tick.is_none()
            {
                listener_first_food_commit_tick = Some(tick);
            }
        }

        if relay.is_none() {
            for event in action_sink.events_for_at(scout, tick) {
                if event.action_name != "tell"
                    || !matches!(event.kind, ActionTraceKind::Committed { .. })
                {
                    continue;
                }
                let Some(ActionTraceDetail::Tell {
                    listener: told_listener,
                    topic,
                }) = &event.detail
                else {
                    continue;
                };
                if *told_listener != listener {
                    continue;
                }
                let worldwake_core::TellTopic::EntityBelief { subject } = topic else {
                    continue;
                };
                if h.world.effective_place(listener) == Some(orchard_place) {
                    continue;
                }
                let listener_store = h
                    .world
                    .get_component_agent_belief_store(listener)
                    .expect("listener should retain a belief store");
                let listener_food_beliefs =
                    known_food_entities_at_place(listener_store, orchard_place);
                if !listener_food_beliefs.contains(subject) {
                    continue;
                }
                let listener_subject_belief_tick = listener_store
                    .get_entity(subject)
                    .and_then(worldwake_core::BelievedEntityState::last_observed_tick)
                    .unwrap_or(tick);
                assert_eq!(
                    event.tell_commit_result(),
                    Some(TellCommitResult::Accepted),
                    "survival tell landing must use an accepted tell relay; event={event:?}"
                );
                assert_eq!(
                    event.tell_belief_delta(),
                    Some(TellBeliefDeltaKind::EntityBelief),
                    "survival tell landing must relay an entity belief about the orchard resource; event={event:?}"
                );
                relay = Some(TellRelayObservation {
                    tell_tick: tick,
                    listener_subject_belief_tick,
                    listener_first_arrival_tick: Tick(u64::MAX),
                    listener_first_food_commit_tick: Tick(u64::MAX),
                    told_subject: *subject,
                    orchard_place,
                });
                break;
            }
        }
    }

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let scout_committed_actions = action_sink
        .events_for(scout)
        .iter()
        .filter(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
        .map(|event| event.action_name.clone())
        .collect::<BTreeSet<_>>();
    let scout_tell_events = action_sink
        .events_for(scout)
        .iter()
        .filter(|event| event.action_name == "tell")
        .map(|event| format!("{event:?}"))
        .collect::<Vec<_>>();
    let relay = relay.unwrap_or_else(|| {
        panic!(
            "scout should commit an accepted tell about the orchard food belief; scout_committed_actions={scout_committed_actions:?}; scout_tell_events={scout_tell_events:?}; ever_colocated_after_start={ever_colocated_after_start}; listener_first_arrival_tick={listener_first_arrival_tick:?}; listener_first_food_commit_tick={listener_first_food_commit_tick:?}"
        )
    });
    let listener_first_arrival_tick = listener_first_arrival_tick
        .expect("listener should eventually reach the orchard after the relay");
    let listener_first_food_commit_tick = listener_first_food_commit_tick
        .expect("listener should eventually harvest or eat after learning the orchard");
    assert!(
        relay.tell_tick < listener_first_arrival_tick,
        "listener should not reach the orchard before the tell relay; relay={relay:?}, first_arrival={listener_first_arrival_tick:?}"
    );
    assert!(
        relay.tell_tick <= listener_first_food_commit_tick,
        "listener should not secure orchard food before the tell relay; relay={relay:?}, first_food_commit={listener_first_food_commit_tick:?}"
    );

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let agents = agents
        .into_iter()
        .map(|(name, agent)| {
            (
                name.clone(),
                AgentSurvivalObservation {
                    alive: !h.agent_is_dead(agent),
                    critical_thresholds: *critical_thresholds
                        .get(&name)
                        .expect("final thresholds should exist"),
                    critical_need_runs: critical_need_runs
                        .remove(&name)
                        .expect("final run tracker should exist"),
                    committed_actions: action_sink
                        .events_for(agent)
                        .iter()
                        .filter(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
                        .map(|event| event.action_name.clone())
                        .collect(),
                },
            )
        })
        .collect();

    SurvivalTellObservation {
        contract,
        agents,
        stuck_idle_windows,
        relay: TellRelayObservation {
            listener_first_arrival_tick,
            listener_first_food_commit_tick,
            ..relay
        },
    }
}

// ---------------------------------------------------------------------------
// Scenario 169: Survival Tell Lands Roadmap Row Five
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Production, Tell
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, Wash, Sleep, Relieve, ShareBelief
// ActionDomains: Needs, Travel, Production, Social
// Places: Rill Camp, North Orchard
// Principles: 6, 7, 14, 15, 20
//
// Setup: Run the authored survival tell scenario for 1440 ticks. The scout
// begins at the only orchard and returns to camp for water/wash survival,
// while the hungry listener begins at camp without any orchard food belief.
//
// Proves: both agents satisfy the authored survival-health contract, and the
// listener's first orchard approach happens only after an accepted tell
// transfers the orchard food belief.
//
// Chain: same-place orchard observation by scout -> return to camp under
// survival pressure -> committed tell relays the orchard entity belief ->
// listener acquires that belief -> listener later travels to the orchard and
// consumes orchard food.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_tell_lands_row_five() {
    let observation = run_survival_tell();
    let run_limit_overrides =
        contract_run_limit_overrides(observation.contract.critical_run_limits.as_ref());

    for (agent_name, agent) in &observation.agents {
        assert!(
            agent.alive,
            "{agent_name} should remain alive for the full {SURVIVAL_TICKS}-tick scenario; observation={agent:?}"
        );
        assert_authored_critical_runs_with_overrides(
            observation.contract.max_authored_critical_run_ticks,
            run_limit_overrides,
            agent_name,
            &agent.critical_thresholds,
            &agent.critical_need_runs,
        );
        assert_required_self_care_families(
            &observation.contract.required_self_care_families,
            agent_name,
            &agent.committed_actions,
            "survival-tell",
        );
    }
    assert_no_stuck_idle_windows(
        observation
            .contract
            .max_idle_window_ticks_with_elevated_need,
        observation.contract.elevated_need_floor.value(),
        "survival-tell",
        &observation.stuck_idle_windows,
    );
    assert!(
        observation.relay.tell_tick <= observation.relay.listener_subject_belief_tick,
        "listener belief update should not predate the tell commit; observation={observation:?}"
    );
}

#[test]
#[ignore = "CI-only: focused survival-tell regression; run via golden-survival workflow"]
fn listener_with_critical_dirtiness_breaks_off_tell_to_wash() {
    let (mut h, def) = load_survival_tell_harness();
    let agents = named_agents(&h);
    let listener = *agents
        .get("Listener Bea")
        .expect("scenario should include Listener Bea");
    let orchard_place = scenario_place_id(&def, "North Orchard");
    let thresholds = *h
        .world
        .get_component_drive_thresholds(listener)
        .expect("listener should have thresholds");

    let known_basins = h
        .world
        .get_component_agent_belief_store(listener)
        .expect("listener should have a belief store")
        .iter_known_entities()
        .filter_map(|(entity, state)| {
            (state.workstation_tag == Some(WorkstationTag::WashBasin)
                && state.wash_basin_state.is_some())
            .then_some(*entity)
        })
        .collect::<Vec<_>>();
    assert!(
        !known_basins.is_empty(),
        "listener should begin with observed Camp Washbasin state"
    );

    let mut needs = *h
        .world
        .get_component_homeostatic_needs(listener)
        .expect("listener should have needs");
    needs.hunger = thresholds.hunger.low();
    needs.thirst = thresholds.thirst.low();
    needs.fatigue = thresholds.fatigue.low();
    needs.bladder = thresholds.bladder.low();
    needs.dirtiness = thresholds.dirtiness.critical();

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_ground_location(listener, orchard_place)
        .expect("test should be able to move listener to tell-side orchard");
    txn.set_component_homeostatic_needs(listener, needs)
        .expect("test should be able to set critical dirtiness");
    commit_txn(txn, &mut h.event_log);

    for _ in 0..160 {
        h.step_once();
        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");
        if action_sink.events_for(listener).iter().any(|event| {
            event.action_name == "wash" && matches!(event.kind, ActionTraceKind::Committed { .. })
        }) {
            return;
        }
    }

    let trace = h
        .driver
        .trace_sink()
        .and_then(|sink| sink.trace_at(listener, Tick(159)))
        .map_or_else(|| "no trace".to_string(), |trace| trace.outcome.summary());
    panic!("critical listener should commit wash after remote tell-side placement; trace={trace}");
}
