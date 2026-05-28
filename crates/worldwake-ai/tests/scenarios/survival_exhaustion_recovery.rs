//! S175 Scenario B golden coverage: recovery before collapse (the dampener).
//!
//! Proves the collapse path is escapable. A tired, danger-averse agent fails
//! rest at the hostile "Wilds" (`HostileProximity` interruptions), flees to the
//! safe "Haven", and rough-sleeps successfully there. Recovery drops fatigue
//! below the critical threshold, so `DeprivationExposure.fatigue_critical_ticks`
//! resets to 0, no `Exhaustion` wound is ever created, and the derived
//! `CriticalWindowReport.exhaustion_collapse_observed` flag stays `false`.

use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_ai::survival_forensics::FailedRestKind;
use worldwake_ai::{CriticalWindowReport, SurvivalForensicExtractor};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{
    DeprivationKind, EntityId, HomeostaticNeedId, Permille, SleepFailureCause, StateHash, Tick,
    hash_serializable,
};
use worldwake_sim::{ActionRequestMode, ActionTraceKind, InputKind, RequestProvenance};

const TICK_BUDGET: u32 = 120;

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct ExhaustionRecoveryObservation {
    reached_haven: bool,
    fatigue_dropped_below_critical: bool,
    fatigue_ticks_reset_after_recovery: bool,
    exhaustion_wound_created: bool,
    actor_dead: bool,
    exhaustion_collapse_observed: bool,
    hostile_failed_rest_at_wilds: usize,
    min_fatigue: Permille,
    max_fatigue_critical_ticks: u32,
    event_log_hash: StateHash,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/survival-exhaustion-recovery.ron")
}

fn load_harness() -> (GoldenHarness, ScenarioDef) {
    let def =
        load_scenario_file(&scenario_path()).expect("exhaustion recovery scenario should parse");
    let spawned = spawn_scenario(&def).expect("exhaustion recovery scenario should spawn");
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

fn named_agent(h: &GoldenHarness, name: &str) -> EntityId {
    h.world
        .query_name_and_agent_data()
        .find_map(|(entity, agent_name, _)| (agent_name.0 == name).then_some(entity))
        .expect("named scenario agent should exist")
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

/// Enqueue a best-effort travel request so the danger-averse agent leaves the
/// hostile Wilds for the safe Haven. The recovery sleep at Haven remains
/// emergent (the planner chooses it once the agent is safe and still tired).
fn request_travel(h: &mut GoldenHarness, actor: EntityId, destination: EntityId) {
    let def_id = action_id(h, "travel");
    let tick = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        tick,
        InputKind::RequestAction {
            actor,
            def_id,
            targets: vec![destination],
            payload_override: None,
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
}

fn sleep_was_interrupted(h: &GoldenHarness, sleeper: EntityId) -> bool {
    h.action_trace_sink().is_some_and(|sink| {
        sink.events_for(sleeper).iter().any(|event| {
            event.action_name == "sleep" && matches!(event.kind, ActionTraceKind::Aborted { .. })
        })
    })
}

fn has_exhaustion_wound(h: &GoldenHarness, agent: EntityId) -> bool {
    h.world
        .get_component_wound_list(agent)
        .is_some_and(|wounds| {
            wounds
                .find_deprivation_wound(DeprivationKind::Exhaustion)
                .is_some()
        })
}

fn observe_recovery() -> ExhaustionRecoveryObservation {
    let (mut h, def) = load_harness();
    let haven = scenario_place_id(&def, "Haven");
    let wilds = scenario_place_id(&def, "Wilds");
    let aster = named_agent(&h, "Aster");
    let thresholds = h
        .world
        .get_component_drive_thresholds(aster)
        .copied()
        .expect("Aster should carry drive thresholds");
    let fatigue_critical = thresholds.critical(HomeostaticNeedId::Fatigue);
    let mut extractor = SurvivalForensicExtractor::new(aster);

    let mut reached_haven = false;
    let mut fatigue_dropped_below_critical = false;
    let mut fatigue_ticks_reset_after_recovery = false;
    let mut exhaustion_wound_created = false;
    let mut min_fatigue = Permille::new(1000).unwrap();
    let mut max_fatigue_critical_ticks = 0;
    let mut was_critical = false;
    let mut travel_requested = false;

    for tick_num in 0..TICK_BUDGET {
        h.step_once();
        let tick = Tick(u64::from(tick_num));
        let needs = *h
            .world
            .get_component_homeostatic_needs(aster)
            .expect("Aster should carry needs");
        observe_critical_windows(&mut extractor, &h, aster, tick, &needs, &thresholds);

        // Once the hostile has interrupted at least one rest at the Wilds, the
        // danger-averse agent flees to the safe Haven. The recovery sleep there
        // is the agent's own emergent choice.
        if !travel_requested
            && h.world.effective_place(aster) == Some(wilds)
            && sleep_was_interrupted(&h, aster)
        {
            request_travel(&mut h, aster, haven);
            travel_requested = true;
        }

        if h.world.effective_place(aster) == Some(haven) {
            reached_haven = true;
        }
        if has_exhaustion_wound(&h, aster) {
            exhaustion_wound_created = true;
        }
        min_fatigue = min_fatigue.min(needs.fatigue);

        let exposure = h
            .world
            .get_component_deprivation_exposure(aster)
            .expect("Aster should carry deprivation exposure");
        max_fatigue_critical_ticks =
            max_fatigue_critical_ticks.max(exposure.fatigue_critical_ticks);

        if needs.fatigue >= fatigue_critical {
            was_critical = true;
        } else if was_critical {
            // Fatigue recovered below critical after having been critical.
            fatigue_dropped_below_critical = true;
            if exposure.fatigue_critical_ticks == 0 {
                fatigue_ticks_reset_after_recovery = true;
            }
            was_critical = false;
        }

        // Stop once recovery is firmly proven and the agent is safe at Haven.
        if fatigue_dropped_below_critical && reached_haven && needs.fatigue < fatigue_critical {
            // Run a few more ticks to confirm collapse does not re-trigger.
            for _ in 0..4 {
                h.step_once();
            }
            break;
        }
    }

    let reports = extractor.finalize();
    let exhaustion_collapse_observed = reports
        .iter()
        .any(|report| report.exhaustion_collapse_observed);
    let hostile_failed_rest_at_wilds = count_hostile_failed_rest(&reports);

    ExhaustionRecoveryObservation {
        reached_haven,
        fatigue_dropped_below_critical,
        fatigue_ticks_reset_after_recovery,
        exhaustion_wound_created,
        actor_dead: h.world.get_component_dead_at(aster).is_some(),
        exhaustion_collapse_observed,
        hostile_failed_rest_at_wilds,
        min_fatigue,
        max_fatigue_critical_ticks,
        event_log_hash: hash_serializable(&h.event_log).expect("event log should hash"),
    }
}

fn count_hostile_failed_rest(reports: &[CriticalWindowReport]) -> usize {
    reports
        .iter()
        .filter(|report| report.need == HomeostaticNeedId::Fatigue)
        .flat_map(|report| &report.frames)
        .flat_map(|frame| &frame.failed_rest_opportunities)
        .filter(|opportunity| {
            opportunity.kind
                == (FailedRestKind::Interrupted {
                    cause: SleepFailureCause::HostileProximity,
                })
        })
        .count()
}

// Scenario 487: S175 Exhaustion Recovery Dampener
// Setup: A tired, danger-averse agent starts co-located with a hostile at the Wilds (sleep interrupted via HostileProximity) and flees along a short edge to the safe Haven to rough-sleep.
// Proves: recovery before terminal wound load prevents collapse — fatigue drops below critical, the fatigue critical exposure counter resets, no Exhaustion wound forms, and exhaustion_collapse_observed stays false. The collapse path is escapable (FND-11 dampener).
// Chain: hostile co-location -> HostileProximity interruption -> flee travel to safe place -> successful rough sleep -> fatigue below critical -> DeprivationExposure reset -> no Exhaustion wound.
#[test]
#[ignore = "CI-only: travel-and-recover survival horizon; run via golden-survival workflow"]
fn scenario_b_exhaustion_recovery() {
    let observation = observe_recovery();

    assert!(
        observation.reached_haven,
        "the danger-averse agent should flee to the safe Haven: {observation:#?}"
    );
    assert!(
        observation.fatigue_dropped_below_critical,
        "successful rough sleep at Haven should drop fatigue below critical: {observation:#?}"
    );
    assert!(
        observation.fatigue_ticks_reset_after_recovery,
        "recovery below critical should reset fatigue_critical_ticks to 0: {observation:#?}"
    );
    assert!(
        !observation.exhaustion_wound_created,
        "no Exhaustion wound should form when the agent recovers in time: {observation:#?}"
    );
    assert!(
        !observation.actor_dead,
        "the recovering agent must not collapse: {observation:#?}"
    );
    assert!(
        !observation.exhaustion_collapse_observed,
        "a recovering window must report exhaustion_collapse_observed == false: {observation:#?}"
    );
    assert!(
        observation.hostile_failed_rest_at_wilds > 0,
        "the window should record the hostile failed-rest attempts at the Wilds: {observation:#?}"
    );
}

#[test]
#[ignore = "CI-only: travel-and-recover survival horizon; run via golden-survival workflow"]
fn scenario_b_exhaustion_recovery_replays_deterministically() {
    let first = observe_recovery();
    let second = observe_recovery();

    assert_eq!(first.event_log_hash, second.event_log_hash);
    assert_eq!(first, second);
}
