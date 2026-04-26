//! Golden tests for the survival preferences roadmap landing.

mod golden_harness;

use std::collections::BTreeSet;
use std::path::PathBuf;

use golden_harness::*;
use worldwake_ai::DecisionOutcome;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{
    AcquisitionQuantity, CommodityKind, CommodityPurpose, DriveThresholds, EntityId,
    ExplorationMotivation, GoalKind, OpportunityAnchor, PerceptionSource, SourceKey, Tick,
    WorkstationTag,
};
use worldwake_sim::ActionTraceKind;

const SURVIVAL_TICKS: u32 = 1440;

#[derive(Clone, Debug, Eq, PartialEq)]
struct AgentSurvivalObservation {
    alive: bool,
    critical_thresholds: DriveThresholds,
    critical_need_runs: SurvivalNeedRunTracker,
    committed_actions: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SurvivalPreferencesObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    agent: AgentSurvivalObservation,
    stuck_idle_windows: Vec<StuckIdleWindow>,
    proactive_tick: Tick,
    proactive_arrival_tick: Tick,
    novel_success_tick: Tick,
    familiar_memory_tick: Tick,
    familiar_failed_attempts: u16,
    discounted_familiar_retry_tick: Tick,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-preferences.ron")
}

fn load_survival_preferences_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("survival preferences scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival preferences scenario should spawn");
    let mut harness = GoldenHarness::from_simulation_state(&spawned.state);
    let agent = harness
        .world
        .query_name_and_agent_data()
        .find_map(|(entity, name, _)| (name.0 == "Scout Ilen").then_some(entity))
        .expect("scenario should include Scout Ilen");
    seed_actor_local_beliefs(
        &mut harness.world,
        &mut harness.event_log,
        agent,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
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

fn orchard_at_place(h: &GoldenHarness, place: EntityId) -> EntityId {
    h.world
        .entities_effectively_at(place)
        .into_iter()
        .find(|entity| {
            h.world.get_component_workstation_marker(*entity)
                == Some(&worldwake_core::WorkstationMarker(
                    WorkstationTag::OrchardRow,
                ))
        })
        .unwrap_or_else(|| panic!("expected orchard workstation at place {place:?}"))
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

fn run_survival_preferences() -> SurvivalPreferencesObservation {
    let (mut h, def) = load_survival_preferences_harness();
    let contract = expect_survival_health_contract(
        def.survival_health_contract.as_ref(),
        "survival preferences",
    )
    .clone();
    let agent = h
        .world
        .query_name_and_agent_data()
        .find_map(|(entity, name, _)| (name.0 == "Scout Ilen").then_some(entity))
        .expect("scenario should include Scout Ilen");
    let familiar_orchard_place = scenario_place_id(&def, "Familiar Orchard");
    let novel_grove_place = scenario_place_id(&def, "Novel Grove");
    let familiar_orchard = orchard_at_place(&h, familiar_orchard_place);
    let novel_orchard = orchard_at_place(&h, novel_grove_place);
    let thresholds = h
        .world
        .get_component_drive_thresholds(agent)
        .copied()
        .expect("survival preferences agent should have drive thresholds");
    let mut critical_need_runs = SurvivalNeedRunTracker::default();
    let mut stuck_idle_windows = Vec::new();
    let mut idle_state: (Option<u32>, u16, u32) = (None, 0, 0);

    let mut proactive_tick = None;
    let mut proactive_arrival_tick = None;
    let mut novel_success_tick = None;
    let mut familiar_memory_tick = None;
    let mut discounted_familiar_retry_tick = None;

    for tick_num in 0..SURVIVAL_TICKS {
        h.step_once();
        let tick = Tick(u64::from(tick_num));
        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");

        let needs = h
            .world
            .get_component_homeostatic_needs(agent)
            .expect("survival preferences agent should always have needs");
        critical_need_runs.observe(needs, &thresholds);

        let had_action = action_sink
            .events_for_at(agent, tick)
            .iter()
            .any(|e| !matches!(e.kind, ActionTraceKind::StartFailed { .. }));
        let (start, max_need, count) = &mut idle_state;
        if had_action {
            if let Some(s) = start.take()
                && *count >= contract.max_idle_window_ticks_with_elevated_need
                && *max_need > contract.elevated_need_floor.value()
            {
                stuck_idle_windows.push(StuckIdleWindow {
                    agent_name: "Scout Ilen".to_string(),
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

        if proactive_tick.is_none() {
            let maybe_tick = h
                .driver
                .trace_sink()
                .expect("decision tracing should be enabled")
                .trace_at(agent, tick)
                .and_then(|trace| match &trace.outcome {
                    DecisionOutcome::Planning(planning)
                        if planning.selection.selected_goal().is_some_and(|goal| {
                            matches!(
                                goal.kind,
                                worldwake_core::GoalKind::ExploreLocation {
                                    target_place,
                                    motivating_need: ExplorationMotivation::Proactive,
                                } if target_place == novel_grove_place
                            )
                        }) =>
                    {
                        Some(trace.tick)
                    }
                    _ => None,
                });
            proactive_tick = proactive_tick.or(maybe_tick);
        }

        if proactive_tick.is_some()
            && proactive_arrival_tick.is_none()
            && h.world.effective_place(agent) == Some(novel_grove_place)
        {
            proactive_arrival_tick = Some(tick);
        }

        if novel_success_tick.is_none()
            && h.world
                .get_component_source_reliability(agent)
                .and_then(|reliability| {
                    reliability.sources.get(&SourceKey {
                        entity: novel_orchard,
                        commodity: CommodityKind::Apple,
                    })
                })
                .is_some_and(|record| record.successful_acquisitions > 0)
        {
            novel_success_tick = Some(tick);
        }

        if familiar_memory_tick.is_none()
            && h.world
                .get_component_source_reliability(agent)
                .and_then(|reliability| {
                    reliability.sources.get(&SourceKey {
                        entity: familiar_orchard,
                        commodity: CommodityKind::Apple,
                    })
                })
                .is_some_and(|record| record.failed_attempts > 0)
        {
            familiar_memory_tick = Some(tick);
        }

        if familiar_memory_tick.is_some()
            && discounted_familiar_retry_tick.is_none()
            && h.driver
                .trace_sink()
                .expect("decision tracing should be enabled")
                .trace_at(agent, tick)
                .is_some_and(|trace| match &trace.outcome {
                    DecisionOutcome::Planning(planning) => {
                        planning.selection.selected_goal().is_some_and(|goal| {
                            goal.kind
                                == GoalKind::AcquireCommodity {
                                    commodity: CommodityKind::Apple,
                                    purpose: CommodityPurpose::SelfConsume,
                                    quantity: AcquisitionQuantity::single(),
                                }
                        }) && planning.selection.selected_opportunity
                            == Some(worldwake_core::OpportunityKey {
                                goal_key: worldwake_core::GoalKey::from(
                                    GoalKind::AcquireCommodity {
                                        commodity: CommodityKind::Apple,
                                        purpose: CommodityPurpose::SelfConsume,
                                        quantity: AcquisitionQuantity::single(),
                                    },
                                ),
                                anchor: OpportunityAnchor::Place(novel_grove_place),
                            })
                            && planning.candidates.ranked.iter().any(|ranked| {
                                ranked.opportunity.goal_key.kind
                                    == GoalKind::AcquireCommodity {
                                        commodity: CommodityKind::Apple,
                                        purpose: CommodityPurpose::SelfConsume,
                                        quantity: AcquisitionQuantity::single(),
                                    }
                                    && ranked.opportunity.anchor
                                        == OpportunityAnchor::Place(familiar_orchard_place)
                                    && ranked.source_reliability_discount.as_ref().is_some_and(
                                        |discount| {
                                            discount.source_entity == familiar_orchard
                                                && discount.commodity == CommodityKind::Apple
                                                && discount.failure_ratio_permille > 0
                                        },
                                    )
                            })
                    }
                    _ => false,
                })
        {
            discounted_familiar_retry_tick = Some(tick);
        }
    }

    let committed_actions = h
        .action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for(agent)
        .iter()
        .filter(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
        .map(|event| event.action_name.clone())
        .collect::<BTreeSet<_>>();

    let trace_summaries = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .traces_for(agent)
        .iter()
        .map(|trace| format!("{:?}: {}", trace.tick, trace.outcome.summary()))
        .collect::<Vec<_>>();
    assert!(
        h.world.effective_place(agent) != Some(familiar_orchard_place)
            || !committed_actions.is_empty(),
        "sanity check: scenario should exercise real actions",
    );

    SurvivalPreferencesObservation {
        contract,
        agent: AgentSurvivalObservation {
            alive: !h.agent_is_dead(agent),
            critical_thresholds: thresholds,
            critical_need_runs,
            committed_actions: committed_actions.clone(),
        },
        stuck_idle_windows,
        proactive_tick: proactive_tick.expect("scenario should proactively select Novel Grove"),
        proactive_arrival_tick: proactive_arrival_tick.unwrap_or_else(|| {
            panic!(
                "scenario should reach Novel Grove after proactive selection; traces={trace_summaries:?}"
            )
        }),
        novel_success_tick: novel_success_tick.unwrap_or_else(|| {
            panic!(
                "scenario should later use the proactively discovered Novel Grove for successful apple acquisition; committed_actions={committed_actions:?}; traces={trace_summaries:?}"
            )
        }),
        familiar_memory_tick: familiar_memory_tick.unwrap_or_else(|| {
            panic!(
                "scenario should turn a locally observed familiar-source depletion into durable failure memory; committed_actions={committed_actions:?}; traces={trace_summaries:?}"
            )
        }),
        familiar_failed_attempts: h
            .world
            .get_component_source_reliability(agent)
            .and_then(|reliability| {
                reliability.sources.get(&SourceKey {
                    entity: familiar_orchard,
                    commodity: CommodityKind::Apple,
                })
            })
            .map_or(0, |record| record.failed_attempts),
        discounted_familiar_retry_tick: discounted_familiar_retry_tick.unwrap_or_else(|| {
            panic!(
                "scenario should later rank the familiar orchard with a stored failure discount while selecting Novel Grove for apples; traces={trace_summaries:?}"
            )
        }),
    }
}

// ---------------------------------------------------------------------------
// Scenario 171: Survival Preferences Keeps Proactive Diversification Alive Under Survival
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Production, proactive diversification
// GoalKinds: ExploreLocation, AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, Sleep, Relieve, Wash
// ActionDomains: Travel, Production, Needs
// Places: Willow Camp, Familiar Orchard, Novel Grove
// Principles: 6, 7, 14, 20, 22, 22A
//
// Setup: Run the authored survival preferences scenario for 1440 ticks. The
// tracked scout starts beside a familiar orchard, survives through camp-based
// water/wash loops, and proactively discovers a novel grove before later using
// that grove for real apple recovery inside the same survival envelope.
//
// Proves: the agent satisfies the authored survival contract; proactive
// exploration reaches Novel Grove; and that discovered grove later becomes a
// concrete successful apple source rather than remaining a decorative visit.
//
// Chain: proactive ExploreLocation selection -> travel arrival at Novel Grove
// -> retained survival loop through camp water/wash -> later apple acquisition
// succeeds at the discovered grove.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_preferences_keeps_proactive_diversification_alive_under_survival() {
    let observation = run_survival_preferences();
    let run_limit_overrides =
        contract_run_limit_overrides(observation.contract.critical_run_limits.as_ref());

    assert!(
        observation.agent.alive,
        "Scout Ilen should remain alive for the full {SURVIVAL_TICKS}-tick scenario; observation={observation:?}"
    );
    assert_authored_critical_runs_with_overrides(
        observation.contract.max_authored_critical_run_ticks,
        run_limit_overrides,
        "Scout Ilen",
        &observation.agent.critical_thresholds,
        &observation.agent.critical_need_runs,
    );
    assert_required_self_care_families(
        &observation.contract.required_self_care_families,
        "Scout Ilen",
        &observation.agent.committed_actions,
        "survival-preferences",
    );
    assert_no_stuck_idle_windows(
        observation
            .contract
            .max_idle_window_ticks_with_elevated_need,
        observation.contract.elevated_need_floor.value(),
        "survival-preferences",
        &observation.stuck_idle_windows,
    );
    assert!(
        observation.proactive_tick <= observation.proactive_arrival_tick,
        "Novel Grove arrival should follow a proactive selection rather than a non-proactive branch; observation={observation:?}"
    );
    assert!(
        observation.proactive_arrival_tick < observation.novel_success_tick,
        "the discovered grove should later become a real apple source after the proactive arrival, not just a visited place; observation={observation:?}"
    );
    assert!(
        observation.familiar_memory_tick <= observation.discounted_familiar_retry_tick,
        "the later apple-choice divergence should happen only after familiar-source depletion becomes durable memory; observation={observation:?}"
    );
}

#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_preferences_replays_deterministically() {
    assert_eq!(run_survival_preferences(), run_survival_preferences());
}
