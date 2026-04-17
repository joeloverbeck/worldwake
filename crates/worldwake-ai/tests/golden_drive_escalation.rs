//! Golden tests for S116 drive escalation behavior.

mod golden_harness;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use golden_harness::*;
use worldwake_ai::{DecisionOutcome, GoalKind, PlanSearchOutcome};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{
    AgentBeliefStore, CommodityKind, EntityId, EventTag, EventView, ExplorationProfile,
    HomeostaticNeedId, HomeostaticNeeds, MetabolismProfile, PerceptionSource, Quantity,
    ResourceSource, Seed, Tick, UtilityProfile, WorkstationTag,
};
use worldwake_sim::ActionTraceKind;

const WASH_PRIORITY_TICKS: u32 = 800;
const BELIEF_ONLY_TICKS: u32 = 400;
const MAX_CRITICAL_DIRTINESS_RUN_TICKS: u32 = 250;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct NeedRunTracker {
    current: u32,
    max: u32,
}

impl NeedRunTracker {
    fn observe(&mut self, needs: &HomeostaticNeeds, threshold: u16) {
        if needs.dirtiness >= pm(threshold) {
            self.current += 1;
            self.max = self.max.max(self.current);
        } else {
            self.current = 0;
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WashPriorityAgentObservation {
    wash_commits: u32,
    critical_dirtiness_run_max: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WashPriorityObservation {
    agents: BTreeMap<String, WashPriorityAgentObservation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BeliefBarrierObservation {
    believed_wash_basins: Vec<EntityId>,
    exposure_ticks: u32,
    found_wash_plan_ticks: Vec<Tick>,
    committed_actions: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EscalationReliefObservation {
    agent_name: String,
    wash_commit_tick: Tick,
    reset_tick: Tick,
    end_event_tick: Tick,
    end_action_name: String,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/drive-escalation-wash-priority.ron")
}

fn load_drive_escalation_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("drive-escalation scenario should parse");
    let spawned = spawn_scenario(&def).expect("drive-escalation scenario should spawn");
    let mut harness = GoldenHarness::from_simulation_state(&spawned.state);
    harness.driver.enable_tracing();
    harness.enable_action_tracing();
    (harness, def)
}

fn named_agents(h: &GoldenHarness) -> BTreeMap<String, EntityId> {
    h.world
        .query_name_and_agent_data()
        .map(|(entity, name, _)| (name.0.clone(), entity))
        .collect()
}

fn named_agents_by_id(h: &GoldenHarness) -> BTreeMap<EntityId, String> {
    h.world
        .query_name_and_agent_data()
        .map(|(entity, name, _)| (entity, name.0.clone()))
        .collect()
}

fn run_dirtiness_wash_cycle_under_priority_override() -> WashPriorityObservation {
    let (mut h, _) = load_drive_escalation_harness();
    let agents = named_agents(&h);
    let thresholds = agents
        .iter()
        .map(|(name, agent)| {
            (
                name.clone(),
                h.world
                    .get_component_drive_thresholds(*agent)
                    .expect("scenario agents should have drive thresholds")
                    .dirtiness
                    .critical()
                    .value(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut runs = agents
        .keys()
        .cloned()
        .map(|name| (name, NeedRunTracker::default()))
        .collect::<BTreeMap<_, _>>();
    let mut wash_commits = agents
        .keys()
        .cloned()
        .map(|name| (name, 0u32))
        .collect::<BTreeMap<_, _>>();

    for tick_num in 0..WASH_PRIORITY_TICKS {
        h.step_once();

        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");

        for (agent_name, agent) in &agents {
            let needs = h
                .world
                .get_component_homeostatic_needs(*agent)
                .expect("scenario agents should have needs");
            runs.get_mut(agent_name)
                .expect("run tracker should exist")
                .observe(
                    needs,
                    *thresholds
                        .get(agent_name)
                        .expect("threshold should exist for agent"),
                );

            for event in action_sink.events_for_at(*agent, Tick(u64::from(tick_num))) {
                if matches!(event.kind, ActionTraceKind::Committed { .. })
                    && event.action_name == "wash"
                {
                    *wash_commits
                        .get_mut(agent_name)
                        .expect("wash counter should exist") += 1;
                }
            }
        }
    }

    let agents = agents
        .into_keys()
        .map(|name| {
            (
                name.clone(),
                WashPriorityAgentObservation {
                    wash_commits: *wash_commits
                        .get(&name)
                        .expect("final wash count should exist"),
                    critical_dirtiness_run_max: runs
                        .remove(&name)
                        .expect("final run tracker should exist")
                        .max,
                },
            )
        })
        .collect();

    WashPriorityObservation { agents }
}

fn build_belief_only_wash_harness() -> (GoldenHarness, EntityId) {
    let mut h = GoldenHarness::new(Seed([6; 32]));
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let metabolism = MetabolismProfile {
        hunger_rate: pm(0),
        thirst_rate: pm(0),
        fatigue_rate: pm(0),
        bladder_rate: pm(0),
        dirtiness_rate: pm(0),
        starvation_tolerance_ticks: nz(1000),
        dehydration_tolerance_ticks: nz(1000),
        exhaustion_collapse_ticks: nz(1000),
        bladder_accident_tolerance_ticks: nz(400),
        wilderness_relief_dirtiness_penalty: pm(200),
        ..MetabolismProfile::default()
    };
    let utility = UtilityProfile {
        bladder_weight: pm(900),
        dirtiness_weight: pm(625),
        hunger_weight: pm(0),
        thirst_weight: pm(0),
        fatigue_weight: pm(0),
        ..UtilityProfile::default()
    };
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Belief-Barrier Washer",
        ORCHARD_FARM,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(900), pm(850)),
        metabolism,
        utility,
    );
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_exploration_profile(
        agent,
        ExplorationProfile {
            curiosity_weight: pm(0),
            need_activation_threshold: pm(1000),
            ..ExplorationProfile::default()
        },
    )
    .expect("belief-barrier harness should keep exploration profile writable");
    commit_txn(txn, &mut h.event_log);

    let _remote_wash = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        VILLAGE_SQUARE,
        WorkstationTag::WashBasin,
        ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(20),
            max_quantity: Quantity(20),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    (h, agent)
}

fn run_escalation_respects_belief_only_planning() -> BeliefBarrierObservation {
    let (mut h, agent) = build_belief_only_wash_harness();

    for _ in 0..BELIEF_ONLY_TICKS {
        h.step_once();
    }

    let belief_store = h
        .world
        .get_component_agent_belief_store(agent)
        .cloned()
        .unwrap_or_else(AgentBeliefStore::new);
    let believed_wash_basins = belief_store
        .iter_known_entities()
        .filter_map(|(entity, state)| {
            (state.workstation_tag == Some(WorkstationTag::WashBasin)).then_some(*entity)
        })
        .collect::<Vec<_>>();
    let exposure_ticks = h
        .world
        .get_component_deprivation_exposure(agent)
        .expect("belief-barrier agent should have deprivation exposure")
        .ticks_at_critical(HomeostaticNeedId::Dirtiness);
    let found_wash_plan_ticks = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .traces_for(agent)
        .iter()
        .filter_map(|trace| {
            let DecisionOutcome::Planning(planning) = &trace.outcome else {
                return None;
            };
            planning
                .planning
                .attempts
                .iter()
                .any(|attempt| {
                    matches!(attempt.goal.kind, GoalKind::Wash)
                        && matches!(attempt.outcome, PlanSearchOutcome::Found { .. })
                })
                .then_some(trace.tick)
        })
        .collect::<Vec<_>>();
    let committed_actions = h
        .action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for(agent)
        .iter()
        .filter(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
        .map(|event| event.action_name.clone())
        .collect::<BTreeSet<_>>();

    BeliefBarrierObservation {
        believed_wash_basins,
        exposure_ticks,
        found_wash_plan_ticks,
        committed_actions,
    }
}

fn run_escalation_fades_after_relief() -> EscalationReliefObservation {
    let (mut h, _) = load_drive_escalation_harness();
    let agents = named_agents(&h);
    let agent_names = named_agents_by_id(&h);
    let mut last_wash_commit = BTreeMap::<EntityId, Tick>::new();
    let mut seen_escalation_events = 0usize;

    for tick_num in 0..WASH_PRIORITY_TICKS {
        h.step_once();

        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");
        for agent in agents.values() {
            for event in action_sink.events_for_at(*agent, Tick(u64::from(tick_num))) {
                if matches!(event.kind, ActionTraceKind::Committed { .. })
                    && event.action_name == "wash"
                {
                    last_wash_commit.insert(*agent, event.tick);
                }
            }
        }

        let escalation_events = h.event_log.events_by_tag(EventTag::Escalation);
        for event_id in escalation_events.iter().skip(seen_escalation_events) {
            let record = h
                .event_log
                .get(*event_id)
                .expect("escalation event id should resolve");
            let Some(action_name) = record.action_name() else {
                continue;
            };
            if !action_name.starts_with("escalation_end:Dirtiness:") {
                continue;
            }
            let actor = record
                .actor_id()
                .expect("escalation event should have actor");
            let wash_commit_tick = *last_wash_commit
                .get(&actor)
                .expect("dirtiness escalation end should follow a wash commit");
            let exposure = h
                .world
                .get_component_deprivation_exposure(actor)
                .expect("agent should have deprivation exposure");
            let needs = h
                .world
                .get_component_homeostatic_needs(actor)
                .expect("agent should have needs");
            let threshold = h
                .world
                .get_component_drive_thresholds(actor)
                .expect("agent should have drive thresholds")
                .dirtiness
                .critical();

            assert_eq!(
                exposure.ticks_at_critical(HomeostaticNeedId::Dirtiness),
                0,
                "dirtiness escalation should reset its counter when the end event is emitted"
            );
            assert!(
                needs.dirtiness < threshold,
                "dirtiness should be sub-critical when the escalation end event is emitted"
            );
            assert!(
                record.tick().0 <= wash_commit_tick.0 + 1,
                "dirtiness escalation should end within 1 tick of wash relief: wash_tick={wash_commit_tick:?}, end_tick={:?}",
                record.tick()
            );

            return EscalationReliefObservation {
                agent_name: agent_names
                    .get(&actor)
                    .expect("agent name should exist")
                    .clone(),
                wash_commit_tick,
                reset_tick: record.tick(),
                end_event_tick: record.tick(),
                end_action_name: action_name.to_string(),
            };
        }
        seen_escalation_events = escalation_events.len();
    }

    panic!(
        "scenario should emit a dirtiness escalation_end event within {WASH_PRIORITY_TICKS} ticks"
    );
}

// ---------------------------------------------------------------------------
// Scenario 164: Sustained Dirtiness Escalation Restores Wash Cycles
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Production
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, Wash, Relieve
// ActionDomains: Needs, Travel, Production
// Places: Base Camp, Central Crossing, Spring Basin, East Orchard
// Principles: 3, 11, 20, 22
//
// Setup: Load the authored `drive-escalation-wash-priority.ron` scenario with
// two agents whose dirtiness starts near or above critical, whose hunger
// weight (750) still exceeds dirtiness weight (625), and whose only wash-capable
// water source sits two hops from the orchard food hub.
//
// Proves: sustained critical dirtiness now produces repeated wash cycles
// instead of the chronic "stay at food, relieve outdoors, wash too rarely"
// equilibrium seen in the motivating contested report.
//
// Chain: outdoor relief at the orchard -> dirtiness remains critical long
// enough to grow escalation -> Wash motive overtakes hunger-driven orchard
// looping -> repeated wash commits reset the critical run length.
#[test]
fn dirtiness_wash_cycle_under_priority_override() {
    let observation = run_dirtiness_wash_cycle_under_priority_override();

    for (agent_name, agent) in &observation.agents {
        assert!(
            agent.wash_commits >= 4,
            "{agent_name} should complete at least 4 wash cycles in {WASH_PRIORITY_TICKS} ticks; observation={agent:?}"
        );
        assert!(
            agent.critical_dirtiness_run_max < MAX_CRITICAL_DIRTINESS_RUN_TICKS,
            "{agent_name} dirtiness should not stay critical for {MAX_CRITICAL_DIRTINESS_RUN_TICKS} consecutive ticks; observation={agent:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Scenario 165: Escalation Preserves Belief-Only Wash Planning
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, PlanningSnapshot
// GoalKinds: Wash, Relieve
// ActionDomains: Needs
// Places: Orchard Farm, Village Square
// Principles: 7, 14, 15, 20
//
// Setup: Direct-harness agent at outdoor Orchard Farm with no food, thirst, or
// fatigue pressure, repeated wilderness-relief dirtiness penalties, and an
// authoritative remote WashBasin at Village Square that is never seeded into
// the agent's beliefs.
//
// Proves: drive escalation does not synthesize remote wash knowledge. Dirtiness
// exposure can grow past `start_after_ticks` without any found Wash plan or
// committed wash action appearing.
//
// Chain: repeated relieve_wilderness -> dirtiness crosses critical and stays
// there -> escalation multiplier grows -> no believed wash basin means search
// never finds a lawful Wash plan -> no wash commit occurs.
#[test]
fn escalation_respects_belief_only_planning() {
    let observation = run_escalation_respects_belief_only_planning();
    let mut h = GoldenHarness::new(Seed([0; 32]));
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Threshold Probe",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let start_after = h
        .world
        .get_component_drive_escalation_profile(agent)
        .expect("default drive-escalation profile should exist")
        .params_for(HomeostaticNeedId::Dirtiness)
        .start_after_ticks;

    assert!(
        observation.believed_wash_basins.is_empty(),
        "agent should not believe in any wash basin; observation={observation:?}"
    );
    assert!(
        observation.found_wash_plan_ticks.is_empty(),
        "agent should not find a Wash plan without a believed wash basin; observation={observation:?}"
    );
    assert!(
        !observation.committed_actions.contains("wash"),
        "agent should not commit wash without a believed wash basin; observation={observation:?}"
    );
    assert!(
        observation.exposure_ticks > start_after,
        "dirtiness exposure should grow past the escalation start threshold; observation={observation:?}"
    );
}

// ---------------------------------------------------------------------------
// Scenario 166: Dirtiness Escalation Ends Immediately After Wash Relief
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, Event Log
// GoalKinds: Wash, Relieve
// ActionDomains: Needs, Travel, Production
// Places: Base Camp, Central Crossing, Spring Basin, East Orchard
// Principles: 3, 11, 29
//
// Setup: Reuse the authored drive-escalation scenario and watch for the first
// `EventTag::Escalation` dirtiness end transition after a committed wash.
//
// Proves: escalation falls away through the physical wash relief path itself:
// the dirtiness counter resets to zero and the authoritative hidden
// `escalation_end:Dirtiness:*` event is emitted within one tick of the wash.
//
// Chain: sustained critical dirtiness -> escalation begin -> committed wash ->
// needs-system counter reset -> authoritative escalation-end event.
#[test]
fn escalation_fades_after_relief() {
    let observation = run_escalation_fades_after_relief();

    assert!(
        observation
            .end_action_name
            .starts_with("escalation_end:Dirtiness:"),
        "expected a dirtiness escalation-end event; observation={observation:?}"
    );
    assert!(
        observation.reset_tick.0 <= observation.wash_commit_tick.0 + 1,
        "dirtiness escalation should reset within one tick of wash relief; observation={observation:?}"
    );
}
