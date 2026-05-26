//! S174 Scenario C golden coverage for structured hostile-proximity sleep interruption.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_ai::survival_forensics::{FailedRestKind, SurvivalForensicExtractor};
use worldwake_ai::{DecisionOutcome, GoalKind, OpportunityAnchor, OpportunityKey};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{
    CombatWeaponRef, DecisionEventPayload, EntityId, EventTag, EventView, GoalKey,
    HomeostaticNeedId, Permille, SleepEpisodeEndedPayload, SleepFailureCause, StateHash, Tick,
    WakeReason, hash_serializable,
};
use worldwake_sim::{
    ActionPayload, ActionRequestMode, ActionTraceDetail, ActionTraceKind, CombatActionPayload,
    InputKind, RequestProvenance,
};

const TICK_BUDGET: u32 = 80;

#[derive(Clone, Debug, Eq, PartialEq)]
struct RestInterruptedObservation {
    sleep_start_tick: Tick,
    travel_start_tick: Tick,
    hostile_arrival_tick: Tick,
    attack_start_tick: Tick,
    interrupted_tick: Tick,
    accumulated_recovery: Permille,
    final_fatigue: Permille,
    trace_recovery: Permille,
    interruption_state: RestInterruptionState,
    post_interrupt_trace: PostInterruptTrace,
    event_log_hash: StateHash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RestInterruptionState {
    trace_was_rough_sleep: bool,
    rest_occupancy_released: bool,
    failed_rest_recorded: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PostInterruptTrace {
    post_interrupt_sleep_candidate: bool,
    post_interrupt_non_sleep_goal: bool,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/survival-rest-interrupted-by-danger.ron")
}

fn load_harness() -> (GoldenHarness, ScenarioDef) {
    let def = load_scenario_file(&scenario_path())
        .expect("survival rest interrupted by danger scenario should parse");
    let spawned =
        spawn_scenario(&def).expect("survival rest interrupted by danger scenario should spawn");
    let mut h = GoldenHarness::from_simulation_state(&spawned.state);
    h.driver.enable_tracing();
    h.enable_action_tracing();
    (h, def)
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

fn action_id(h: &GoldenHarness, action_name: &str) -> worldwake_core::ActionDefId {
    h.defs
        .iter()
        .find(|def| def.name == action_name)
        .map_or_else(
            || panic!("action registry should include {action_name}"),
            |def| def.id,
        )
}

fn request_action(
    h: &mut GoldenHarness,
    actor: EntityId,
    action_name: &str,
    targets: Vec<EntityId>,
    payload_override: Option<ActionPayload>,
) {
    let def_id = action_id(h, action_name);
    let tick = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        tick,
        InputKind::RequestAction {
            actor,
            def_id,
            targets,
            payload_override,
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
}

fn first_action_tick(h: &GoldenHarness, actor: EntityId, action_name: &str) -> Option<Tick> {
    h.action_trace_sink()?
        .events_for(actor)
        .iter()
        .find_map(|event| {
            (event.action_name == action_name
                && matches!(event.kind, ActionTraceKind::Started { .. }))
            .then_some(event.tick)
        })
}

fn interrupted_sleep_detail(
    h: &GoldenHarness,
    sleeper: EntityId,
) -> Option<(Tick, ActionTraceDetail)> {
    h.action_trace_sink()?
        .events_for(sleeper)
        .iter()
        .find_map(|event| {
            (event.action_name == "sleep" && matches!(event.kind, ActionTraceKind::Aborted { .. }))
                .then(|| event.detail.clone().map(|detail| (event.tick, detail)))?
        })
}

fn sleep_ended_payload(h: &GoldenHarness, sleeper: EntityId) -> Option<SleepEpisodeEndedPayload> {
    h.event_log
        .events_by_tag(EventTag::SleepEpisodeEnded)
        .iter()
        .filter_map(|id| h.event_log.get(*id))
        .find_map(|record| match record.decision_payload()? {
            DecisionEventPayload::SleepEpisodeEnded(payload) if payload.sleeper == sleeper => {
                Some(payload.clone())
            }
            _ => None,
        })
}

fn post_interrupt_trace_facts(
    h: &GoldenHarness,
    actor: EntityId,
    shelter: EntityId,
    interrupted_tick: Tick,
) -> (bool, bool) {
    let sleep_at_shelter = OpportunityKey {
        goal_key: GoalKey::from(GoalKind::Sleep),
        anchor: OpportunityAnchor::Place(shelter),
    };

    for tick in interrupted_tick.0 + 1..=interrupted_tick.0 + 4 {
        let Some(trace) = h
            .driver
            .trace_sink()
            .and_then(|sink| sink.trace_at(actor, Tick(tick)))
        else {
            continue;
        };
        let DecisionOutcome::Planning(planning) = &trace.outcome else {
            continue;
        };
        let sleep_candidate = planning
            .candidates
            .generated_contains_opportunity(sleep_at_shelter);
        let non_sleep_goal = planning
            .selection
            .selected_goal()
            .is_some_and(|selected| selected.kind != GoalKind::Sleep);
        return (sleep_candidate, non_sleep_goal);
    }

    (false, false)
}

fn observe_interruption() -> RestInterruptedObservation {
    let (mut h, def) = load_harness();
    let shelter = scenario_place_id(&def, "Shelter");
    let agents = named_agents(&h);
    let aster = *agents.get("Aster").expect("scenario should spawn Aster");
    let marauder = *agents
        .get("Marauder")
        .expect("scenario should spawn Marauder");

    let thresholds = h
        .world
        .get_component_drive_thresholds(aster)
        .copied()
        .expect("Aster should carry drive thresholds");
    let mut extractor = SurvivalForensicExtractor::new(aster);
    let mut attack_requested = false;
    let mut hostile_arrival_tick = None;

    request_action(&mut h, marauder, "travel", vec![shelter], None);

    for tick_num in 0..TICK_BUDGET {
        h.step_once();
        let tick = Tick(u64::from(tick_num));
        let needs = h
            .world
            .get_component_homeostatic_needs(aster)
            .expect("Aster should carry needs");
        observe_critical_windows(&mut extractor, &h, aster, tick, needs, &thresholds);

        if hostile_arrival_tick.is_none() && h.world.effective_place(marauder) == Some(shelter) {
            hostile_arrival_tick = Some(tick);
        }

        if !attack_requested
            && hostile_arrival_tick.is_some()
            && h.world.effective_place(marauder) == Some(shelter)
        {
            request_action(
                &mut h,
                marauder,
                "attack",
                vec![aster],
                Some(ActionPayload::Combat(CombatActionPayload {
                    target: aster,
                    weapon: CombatWeaponRef::Unarmed,
                })),
            );
            attack_requested = true;
        }

        if let Some((interrupted_tick, detail)) = interrupted_sleep_detail(&h, aster) {
            let payload = sleep_ended_payload(&h, aster)
                .expect("interrupted sleep should emit SleepEpisodeEnded");
            let ActionTraceDetail::SleepInterrupted {
                place,
                cause,
                accumulated_recovery,
                was_rough_sleep,
            } = detail
            else {
                panic!("sleep abort should use SleepInterrupted detail");
            };
            assert_eq!(place, shelter);
            assert_eq!(cause, SleepFailureCause::HostileProximity);
            assert_eq!(payload.place, shelter);
            assert_eq!(
                payload.end_reason,
                WakeReason::LocalDisturbance {
                    cause: SleepFailureCause::HostileProximity,
                }
            );
            assert_eq!(payload.accumulated_recovery, accumulated_recovery);

            for offset in 1..=4 {
                h.step_once();
                let tick = Tick(interrupted_tick.0 + offset);
                let needs = h
                    .world
                    .get_component_homeostatic_needs(aster)
                    .expect("Aster should carry needs");
                observe_critical_windows(&mut extractor, &h, aster, tick, needs, &thresholds);
            }

            let reports = extractor.finalize();
            let failed_rest_recorded = reports
                .iter()
                .filter(|report| report.need == HomeostaticNeedId::Fatigue)
                .flat_map(|report| &report.frames)
                .flat_map(|frame| &frame.failed_rest_opportunities)
                .any(|opportunity| {
                    opportunity.place == shelter
                        && opportunity.kind
                            == FailedRestKind::Interrupted {
                                cause: SleepFailureCause::HostileProximity,
                            }
                        && !opportunity.was_rough
                });
            let (post_interrupt_sleep_candidate, post_interrupt_non_sleep_goal) =
                post_interrupt_trace_facts(&h, aster, shelter, interrupted_tick);

            return RestInterruptedObservation {
                sleep_start_tick: first_action_tick(&h, aster, "sleep")
                    .expect("Aster should start sleep"),
                travel_start_tick: first_action_tick(&h, marauder, "travel")
                    .expect("Marauder should start travel"),
                hostile_arrival_tick: hostile_arrival_tick
                    .expect("Marauder should arrive before sleep interruption"),
                attack_start_tick: first_action_tick(&h, marauder, "attack")
                    .expect("Marauder should start attack"),
                interrupted_tick,
                accumulated_recovery: payload.accumulated_recovery,
                final_fatigue: payload.final_fatigue,
                trace_recovery: accumulated_recovery,
                interruption_state: RestInterruptionState {
                    trace_was_rough_sleep: was_rough_sleep,
                    rest_occupancy_released: h
                        .world
                        .get_component_rest_occupancy(shelter)
                        .is_none_or(|occupancy| !occupancy.occupants.contains(&aster)),
                    failed_rest_recorded,
                },
                post_interrupt_trace: PostInterruptTrace {
                    post_interrupt_sleep_candidate,
                    post_interrupt_non_sleep_goal,
                },
                event_log_hash: hash_serializable(&h.event_log)
                    .expect("event log should hash canonically"),
            };
        }
    }

    panic!(
        "scenario did not interrupt sleep within {TICK_BUDGET} ticks; \
         sleep_start={:?}, travel_start={:?}, arrival={:?}, attack_start={:?}, \
         sleep_end={:?}, sleep_abort_detail={:?}, \
         aster_place={:?}, marauder_place={:?}, aster_active={:?}, marauder_active={:?}",
        first_action_tick(&h, aster, "sleep"),
        first_action_tick(&h, marauder, "travel"),
        hostile_arrival_tick,
        first_action_tick(&h, marauder, "attack"),
        sleep_ended_payload(&h, aster),
        interrupted_sleep_detail(&h, aster),
        h.world.effective_place(aster),
        h.world.effective_place(marauder),
        h.scheduler.active_actions().values().find_map(|instance| {
            (instance.actor == aster)
                .then(|| h.defs.get(instance.def_id).map(|def| def.name.clone()))
        }),
        h.scheduler.active_actions().values().find_map(|instance| {
            (instance.actor == marauder)
                .then(|| h.defs.get(instance.def_id).map(|def| def.name.clone()))
        })
    );
}

// Scenario 484: S174 Hostile-Proximity Rest Interruption
// Setup: A fatigued agent sleeps at a one-slot roofed shelter while a hostile agent travels in from an adjacent outpost.
// Proves: hostile co-location interrupts active Sleep with HostileProximity cause, preserves partial recovery, releases RestOccupancy, records failed-rest forensics, and replans away from local Sleep.
// Chain: external hostile travel -> local hostile co-location -> authoritative sleep interruption -> SleepInterrupted trace -> SleepEpisodeEnded wake cause -> SurvivalForensicExtractor -> post-interrupt decision trace.
#[test]
fn scenario_c_hostile_proximity_wake() {
    let observation = observe_interruption();

    assert!(
        observation.sleep_start_tick <= Tick(2),
        "Aster should start sleep in the opening planning cycle: {observation:#?}"
    );
    assert_eq!(
        observation.travel_start_tick,
        Tick(0),
        "Marauder travel is the authored hostile-arrival trigger: {observation:#?}"
    );
    assert!(
        observation.hostile_arrival_tick > observation.sleep_start_tick
            && observation.hostile_arrival_tick < observation.interrupted_tick,
        "hostile should arrive after sleep starts and before interruption: {observation:#?}"
    );
    assert!(
        observation.attack_start_tick <= observation.interrupted_tick,
        "attack pressure should exist by the interruption tick: {observation:#?}"
    );
    assert!(
        observation.accumulated_recovery > Permille::ZERO
            && observation.accumulated_recovery < Permille::new(1000).unwrap(),
        "interrupted sleep should preserve partial recovery: {observation:#?}"
    );
    assert_eq!(observation.trace_recovery, observation.accumulated_recovery);
    assert!(
        !observation.interruption_state.trace_was_rough_sleep,
        "Scenario C uses the known rest site, not rough sleep: {observation:#?}"
    );
    assert!(
        observation.final_fatigue < Permille::new(650).unwrap(),
        "partial recovery should reduce fatigue from the authored start value: {observation:#?}"
    );
    assert!(
        observation.interruption_state.rest_occupancy_released,
        "interrupted sleep should release RestOccupancy: {observation:#?}"
    );
    assert!(
        observation.interruption_state.failed_rest_recorded,
        "critical-window forensics should record the hostile interrupted rest: {observation:#?}"
    );
    assert!(
        !observation
            .post_interrupt_trace
            .post_interrupt_sleep_candidate,
        "Aster should not immediately emit another Sleep candidate targeting the hostile-occupied shelter: {observation:#?}"
    );
    assert!(
        observation
            .post_interrupt_trace
            .post_interrupt_non_sleep_goal,
        "Aster should replan away from Sleep after hostile interruption: {observation:#?}"
    );
}

#[test]
fn scenario_c_hostile_proximity_wake_replays_deterministically() {
    let first = observe_interruption();
    let second = observe_interruption();

    assert_eq!(first, second);
}
