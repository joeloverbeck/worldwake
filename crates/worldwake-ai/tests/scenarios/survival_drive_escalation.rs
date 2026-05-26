//! Golden tests for the survival drive-escalation roadmap landing.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_ai::{DecisionOutcome, GoalKind, PlanSearchOutcome};
use worldwake_cli::scenario::{
    load_scenario_file, spawn_scenario,
    types::{ScenarioDef, SurvivalCriticalRunLimitsDef},
};
use worldwake_core::{
    ActionDefId, AgentBeliefStore, CommodityKind, ContentionGrant, ContentionQueue,
    DriveThresholds, EntityId, EventTag, EventView, ExplorationProfile, GoalKey, HomeostaticNeedId,
    HomeostaticNeeds, MetabolismProfile, PerceptionSource, Quantity, ResourceSource, Seed,
    SelfCareOccupancy, SelfCareUseKind, Tick, UtilityProfile, WashBasinState, WorkstationTag,
};
use worldwake_sim::{
    ActionTraceKind, FacilityBeliefView, GoalBeliefView, PerAgentBeliefView, TemporalBeliefView,
};

const SURVIVAL_TICKS: u32 = 1440;
const BELIEF_ONLY_TICKS: u32 = 400;
const WASH_ACTION: ActionDefId = ActionDefId(4);
#[derive(Clone, Debug, Eq, PartialEq)]
struct AgentSurvivalObservation {
    alive: bool,
    critical_thresholds: DriveThresholds,
    critical_need_runs: SurvivalNeedRunTracker,
    wash_commits: u32,
    committed_actions: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SurvivalDriveEscalationObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    agents: BTreeMap<String, AgentSurvivalObservation>,
    stuck_idle_windows: Vec<StuckIdleWindow>,
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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-drive-escalation.ron")
}

fn load_drive_escalation_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("survival drive-escalation scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival drive-escalation scenario should spawn");
    let mut harness = GoldenHarness::from_simulation_state(&spawned.state);
    let agents = harness
        .world
        .query_name_and_agent_data()
        .map(|(agent, _, _)| agent)
        .collect::<Vec<_>>();
    for agent in agents {
        seed_actor_world_beliefs(
            &mut harness.world,
            &mut harness.event_log,
            agent,
            Tick(0),
            PerceptionSource::DirectObservation,
        );
    }
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

fn contract_run_limit_overrides(
    limits: Option<&SurvivalCriticalRunLimitsDef>,
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

fn run_survival_drive_escalation() -> SurvivalDriveEscalationObservation {
    let (mut h, def) = load_drive_escalation_harness();
    let contract = expect_survival_health_contract(
        def.survival_health_contract.as_ref(),
        "survival drive escalation",
    )
    .clone();
    let agents = named_agents(&h);
    let critical_thresholds = agents
        .iter()
        .map(|(name, agent)| {
            (
                name.clone(),
                h.world
                    .get_component_drive_thresholds(*agent)
                    .expect("scenario agents should have drive thresholds")
                    .to_owned(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut critical_need_runs = agents
        .keys()
        .cloned()
        .map(|name| (name, SurvivalNeedRunTracker::default()))
        .collect::<BTreeMap<_, _>>();
    let mut wash_commits = agents
        .keys()
        .cloned()
        .map(|name| (name, 0u32))
        .collect::<BTreeMap<_, _>>();
    let mut idle_state: BTreeMap<String, (Option<u32>, u16, u32)> = agents
        .keys()
        .cloned()
        .map(|name| (name, (None, 0, 0)))
        .collect();
    let mut stuck_idle_windows = Vec::new();

    for tick_num in 0..SURVIVAL_TICKS {
        h.step_once();
        let tick = Tick(u64::from(tick_num));

        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");

        for (agent_name, agent) in &agents {
            let needs = h
                .world
                .get_component_homeostatic_needs(*agent)
                .expect("scenario agents should have needs");
            critical_need_runs
                .get_mut(agent_name)
                .expect("run tracker should exist")
                .observe(
                    needs,
                    critical_thresholds
                        .get(agent_name)
                        .expect("threshold should exist for agent"),
                );

            let had_action = crate::golden_harness::agent_has_non_failed_action_or_active(
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

            for event in action_sink.events_for_at(*agent, tick) {
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
                    wash_commits: *wash_commits
                        .get(&name)
                        .expect("final wash count should exist"),
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

    SurvivalDriveEscalationObservation {
        contract,
        agents,
        stuck_idle_windows,
    }
}

fn build_belief_only_wash_harness() -> (GoldenHarness, EntityId, EntityId) {
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

    let remote_wash = place_workstation_with_source(
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
            extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
        },
        ProductionOutputOwner::Actor,
    );
    let mut txn = new_txn(&mut h.world, 0);
    let remote_holder = txn
        .create_agent("Remote Wash Holder", worldwake_core::ControlSource::Ai)
        .expect("belief-barrier harness should create remote holder");
    txn.set_ground_location(remote_holder, VILLAGE_SQUARE)
        .expect("belief-barrier harness should place remote holder");
    commit_txn(txn, &mut h.event_log);

    let mut remote_queue = ContentionQueue {
        granted: Some(ContentionGrant {
            actor: remote_holder,
            intended_action: WASH_ACTION,
            granted_at: Tick(7),
            expires_at: Tick(12),
        }),
        ..ContentionQueue::default()
    };
    remote_queue
        .enqueue(agent, WASH_ACTION, Tick(8), None)
        .expect("remote queue should accept a diagnostic waiter");
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_wash_basin_state(
        remote_wash,
        WashBasinState {
            clean_water_units: 9,
            max_clean_water: 12,
            refill_per_tick: 3,
            units_per_full_wash: 4,
            dirtiness_level: pm(700),
            dirtiness_per_use: pm(90),
        },
    )
    .expect("belief-barrier harness should keep remote wash basin state writable");
    txn.set_component_contention_queue(remote_wash, remote_queue)
        .expect("belief-barrier harness should keep remote wash basin contention writable");
    txn.set_component_self_care_occupancy(
        remote_wash,
        SelfCareOccupancy {
            occupant: remote_holder,
            use_kind: SelfCareUseKind::Wash,
            started_tick: Tick(8),
            goal_key: GoalKey::from(GoalKind::Wash),
        },
    )
    .expect("belief-barrier harness should keep remote wash basin occupancy writable");
    commit_txn(txn, &mut h.event_log);
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    (h, agent, remote_wash)
}

fn run_escalation_respects_belief_only_planning() -> BeliefBarrierObservation {
    let (mut h, agent, _) = build_belief_only_wash_harness();

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

fn remote_wash_basin_pov_reads() -> (
    EntityId,
    WashBasinState,
    Option<WashBasinState>,
    Option<u32>,
    bool,
    Option<EntityId>,
    EntityId,
    Option<EntityId>,
) {
    let (h, agent, remote_wash) = build_belief_only_wash_harness();
    let authoritative_state = *h
        .world
        .get_component_wash_basin_state(remote_wash)
        .expect("remote basin should carry authoritative non-default wash state");
    let authoritative_occupant = h
        .world
        .get_component_self_care_occupancy(remote_wash)
        .expect("remote basin should carry authoritative self-care occupancy")
        .occupant;
    let view = PerAgentBeliefView::from_world(agent, &h.world);
    let goal_view_state = GoalBeliefView::wash_basin_state(&view, agent, remote_wash);
    let facility_view_state = FacilityBeliefView::wash_basin_state(&view, remote_wash);
    let remote_queue_position =
        TemporalBeliefView::facility_queue_position(&view, remote_wash, agent);
    let remote_grant_visible = TemporalBeliefView::facility_grant(&view, remote_wash).is_some();
    let remote_self_care_occupant = FacilityBeliefView::self_care_occupant(&view, remote_wash);
    let colocated_view = PerAgentBeliefView::from_world(authoritative_occupant, &h.world);
    let colocated_self_care_occupant =
        FacilityBeliefView::self_care_occupant(&colocated_view, remote_wash);

    assert_ne!(
        authoritative_state,
        WashBasinState::default(),
        "remote basin {remote_wash} must have non-default authoritative state for the leak assertion"
    );

    (
        remote_wash,
        goal_view_state,
        facility_view_state,
        remote_queue_position,
        remote_grant_visible,
        remote_self_care_occupant,
        authoritative_occupant,
        colocated_self_care_occupant,
    )
}

fn build_escalation_relief_harness() -> (GoldenHarness, EntityId) {
    let mut h = GoldenHarness::new(Seed([16; 32]));
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Escalation Relief Washer",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(950)),
        MetabolismProfile {
            hunger_rate: pm(0),
            thirst_rate: pm(0),
            fatigue_rate: pm(0),
            bladder_rate: pm(0),
            dirtiness_rate: pm(0),
            ..MetabolismProfile::default()
        },
        UtilityProfile {
            hunger_weight: pm(0),
            thirst_weight: pm(0),
            fatigue_weight: pm(0),
            bladder_weight: pm(0),
            dirtiness_weight: pm(900),
            ..UtilityProfile::default()
        },
    );

    let params = h
        .world
        .get_component_drive_escalation_profile(agent)
        .expect("default drive-escalation profile should exist")
        .params_for(HomeostaticNeedId::Dirtiness);
    let mut exposure = h
        .world
        .get_component_deprivation_exposure(agent)
        .copied()
        .expect("agent should have deprivation exposure");
    exposure.dirtiness_critical_ticks = params.start_after_ticks + 1;
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_deprivation_exposure(agent, exposure)
        .expect("relief harness should keep deprivation exposure writable");
    txn.set_component_homeostatic_needs(
        agent,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(950)),
    )
    .expect("relief harness should keep needs writable");
    commit_txn(txn, &mut h.event_log);

    let wash_basin = place_workstation(
        &mut h.world,
        &mut h.event_log,
        VILLAGE_SQUARE,
        WorkstationTag::WashBasin,
        ProductionOutputOwner::Actor,
    );
    // Mirror the scenario loader: WashBasin facilities carry default WashBasinState
    // so the candidate filter and authoritative wash precondition can both observe
    // a stocked basin co-located with the well below.
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_wash_basin_state(wash_basin, WashBasinState::default())
        .expect("relief harness should keep wash basin state writable");
    commit_txn(txn, &mut h.event_log);

    let _water_source = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        VILLAGE_SQUARE,
        WorkstationTag::Well,
        ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(10),
            max_quantity: Quantity(10),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
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

fn run_escalation_fades_after_relief() -> EscalationReliefObservation {
    let (mut h, agent) = build_escalation_relief_harness();
    let agent_name = h
        .world
        .get_component_name(agent)
        .expect("relief harness agent should have a name")
        .0
        .clone();
    let mut wash_commit_tick = None;
    let mut seen_escalation_events = 0usize;
    let mut observed_actions = Vec::new();
    let mut observed_escalation_events = Vec::new();

    for tick_num in 0..SURVIVAL_TICKS {
        h.step_once();

        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");
        for event in action_sink.events_for_at(agent, Tick(u64::from(tick_num))) {
            observed_actions.push(format!("{:?}:{}", event.tick, event.action_name));
            if matches!(event.kind, ActionTraceKind::Committed { .. })
                && event.action_name == "wash"
            {
                wash_commit_tick = Some(event.tick);
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
            observed_escalation_events.push(format!("{:?}:{}", record.tick(), action_name));
            if !action_name.starts_with("escalation_end:Dirtiness:") {
                continue;
            }
            let committed_wash_tick = wash_commit_tick.unwrap_or_else(|| {
                panic!(
                    "dirtiness escalation end should follow a wash commit; observed_actions={observed_actions:?}; observed_escalation_events={observed_escalation_events:?}"
                )
            });
            let exposure = h
                .world
                .get_component_deprivation_exposure(agent)
                .expect("agent should have deprivation exposure");
            let needs = h
                .world
                .get_component_homeostatic_needs(agent)
                .expect("agent should have needs");
            let threshold = h
                .world
                .get_component_drive_thresholds(agent)
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
                record.tick().0 <= committed_wash_tick.0 + 1,
                "dirtiness escalation should end within 1 tick of wash relief: wash_tick={committed_wash_tick:?}, end_tick={:?}",
                record.tick()
            );

            return EscalationReliefObservation {
                agent_name: agent_name.clone(),
                wash_commit_tick: committed_wash_tick,
                reset_tick: record.tick(),
                end_event_tick: record.tick(),
                end_action_name: action_name.to_string(),
            };
        }
        seen_escalation_events = escalation_events.len();
    }

    panic!(
        "scenario should emit a dirtiness escalation_end event within {SURVIVAL_TICKS} ticks; observed_actions={observed_actions:?}; observed_escalation_events={observed_escalation_events:?}"
    );
}

// ---------------------------------------------------------------------------
// Scenario: Survival Drive Escalation
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Production
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, Wash, Relieve
// ActionDomains: Needs, Travel, Production
// Places: Base Camp, Central Crossing, Spring Basin, East Orchard
// Principles: 3, 11, 20, 22
//
// Setup: Load the authored `survival-drive-escalation.ron` scenario with two
// agents whose dirtiness starts near or above critical, whose hunger weight
// (750) still exceeds dirtiness weight (625), and whose only wash-capable
// water source sits two hops from the orchard food hub.
//
// Proves: sustained critical dirtiness now produces repeated wash cycles
// inside a 1440-tick survival-health-contract scenario instead of the chronic
// "stay at food, relieve outdoors, wash too rarely" equilibrium seen in the
// motivating contested report.
//
// Chain: outdoor relief at the orchard -> dirtiness remains critical long
// enough to grow escalation -> Wash motive overtakes hunger-driven orchard
// looping -> repeated wash commits reset the critical run length.
#[test]
#[ignore = "CI-only: long-running drive-escalation scenario; run via golden-drive-escalation workflow"]
fn survival_drive_escalation_lands_row_four() {
    let observation = run_survival_drive_escalation();
    let run_limit_overrides =
        contract_run_limit_overrides(observation.contract.critical_run_limits.as_ref());
    let repeated_wash_agent_exists = observation
        .agents
        .values()
        .any(|agent| agent.wash_commits >= 4);

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
            "survival-drive-escalation",
        );
        assert!(
            agent.wash_commits >= 1,
            "{agent_name} should complete at least 1 wash cycle in {SURVIVAL_TICKS} ticks; observation={agent:?}"
        );
        assert!(
            agent.committed_actions.contains("relieve_wilderness"),
            "{agent_name} should still incur wilderness relief pressure in the authored branch; observation={agent:?}"
        );
    }
    assert!(
        repeated_wash_agent_exists,
        "the scenario should produce at least one repeated wash-cycling agent within {SURVIVAL_TICKS} ticks; observation={observation:?}"
    );
    assert_no_stuck_idle_windows(
        observation
            .contract
            .max_idle_window_ticks_with_elevated_need,
        observation.contract.elevated_need_floor.value(),
        "survival-drive-escalation",
        &observation.stuck_idle_windows,
    );
}

// ---------------------------------------------------------------------------
// Scenario: Escalation Preserves Belief-Only Wash Planning
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
#[ignore = "CI-only: long-running drive-escalation scenario; run via golden-drive-escalation workflow"]
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

#[test]
fn cli_does_not_leak_remote_wash_basin_state_for_controlled_agent() {
    let (
        remote_wash,
        goal_view_state,
        facility_view_state,
        remote_queue_position,
        remote_grant_visible,
        remote_self_care_occupant,
        authoritative_occupant,
        colocated_self_care_occupant,
    ) = remote_wash_basin_pov_reads();

    assert_eq!(
        goal_view_state,
        WashBasinState::default(),
        "controlled agent POV leaked remote basin {remote_wash} through GoalBeliefView::wash_basin_state: state={goal_view_state:?}"
    );
    assert_eq!(
        facility_view_state, None,
        "controlled agent POV leaked remote basin {remote_wash} through FacilityBeliefView::wash_basin_state: state={facility_view_state:?}"
    );
    assert_eq!(
        remote_queue_position, None,
        "controlled agent POV leaked remote basin {remote_wash} queue position: position={remote_queue_position:?}"
    );
    assert!(
        !remote_grant_visible,
        "controlled agent POV leaked remote basin {remote_wash} contention grant"
    );
    assert_eq!(
        remote_self_care_occupant, None,
        "controlled agent POV leaked remote basin {remote_wash} self-care occupant: occupant={remote_self_care_occupant:?}"
    );
    assert_eq!(
        colocated_self_care_occupant,
        Some(authoritative_occupant),
        "co-located controlled-agent POV hid observable self-care occupancy on basin {remote_wash}"
    );
}

// ---------------------------------------------------------------------------
// Scenario: Dirtiness Escalation Ends Immediately After Wash Relief
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, Event Log
// GoalKinds: Wash, Relieve
// ActionDomains: Needs, Travel, Production
// Places: Base Camp, Central Crossing, Spring Basin, East Orchard
// Principles: 3, 11, 29
//
// Setup: Reuse the authored survival drive-escalation scenario and watch for
// the first `EventTag::Escalation` dirtiness end transition after a committed
// wash.
//
// Proves: escalation falls away through the physical wash relief path itself:
// the dirtiness counter resets to zero and the authoritative hidden
// `escalation_end:Dirtiness:*` event is emitted within one tick of the wash.
//
// Chain: sustained critical dirtiness -> escalation begin -> committed wash ->
// needs-system counter reset -> authoritative escalation-end event.
#[test]
#[ignore = "CI-only: long-running drive-escalation scenario; run via golden-drive-escalation workflow"]
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
