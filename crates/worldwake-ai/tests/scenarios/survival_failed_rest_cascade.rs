//! S174 Scenario E golden coverage for the failed-rest carrier S175 extends.

use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_ai::survival_forensics::FailedRestKind;
use worldwake_ai::{
    CriticalWindowReport, DecisionOutcome, GoalKind, OpportunityAnchor, OpportunityKey,
    SurvivalForensicExtractor,
};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{
    EntityId, EventTag, GoalKey, HomeostaticNeedId, Permille, StateHash, Tick, hash_serializable,
};
use worldwake_sim::ActionTraceKind;

const TICK_BUDGET: u32 = 80;
const MIN_FAILED_REST_OPPORTUNITIES: usize = 5;

#[derive(Clone, Debug, Eq, PartialEq)]
struct FailedRestCascadeObservation {
    known_rest_site_candidate_ticks: Vec<Tick>,
    start_failed_ticks: Vec<Tick>,
    rough_sleep_start_ticks: Vec<Tick>,
    failed_rest_opportunities: usize,
    rough_fallback_failed_rest: usize,
    unrelated_failed_rest: usize,
    fatigue_samples: Vec<Permille>,
    fatigue_critical_ticks: Vec<u32>,
    collapse_abort_count: usize,
    actor_dead: bool,
    hidden_fatigue_refills: usize,
    event_log_hash: StateHash,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/survival-failed-rest-cascade.ron")
}

fn load_harness() -> (GoldenHarness, ScenarioDef) {
    let def =
        load_scenario_file(&scenario_path()).expect("failed-rest cascade scenario should parse");
    let spawned = spawn_scenario(&def).expect("failed-rest cascade scenario should spawn");
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

fn known_rest_site_candidate_emitted(
    h: &GoldenHarness,
    agent: EntityId,
    tick: Tick,
    shelter: EntityId,
) -> bool {
    let opportunity = OpportunityKey {
        goal_key: GoalKey::from(GoalKind::Sleep),
        anchor: OpportunityAnchor::Place(shelter),
    };
    let Some(trace) = h
        .driver
        .trace_sink()
        .and_then(|sink| sink.trace_at(agent, tick))
    else {
        return false;
    };
    let DecisionOutcome::Planning(planning) = &trace.outcome else {
        return false;
    };
    planning
        .candidates
        .generated_contains_opportunity(opportunity)
}

fn finalize_reports(extractor: SurvivalForensicExtractor) -> Vec<CriticalWindowReport> {
    extractor.finalize()
}

fn failed_rest_counts(reports: &[CriticalWindowReport]) -> (usize, usize, usize) {
    let mut precondition = 0;
    let mut rough_fallback = 0;
    let mut unrelated = 0;
    for opportunity in reports
        .iter()
        .filter(|report| report.need == HomeostaticNeedId::Fatigue)
        .flat_map(|report| &report.frames)
        .flat_map(|frame| &frame.failed_rest_opportunities)
    {
        if opportunity.kind == FailedRestKind::PreconditionRejected && !opportunity.was_rough {
            precondition += 1;
        } else if opportunity.kind == FailedRestKind::RoughFallbackToKnownRestSite
            && opportunity.was_rough
        {
            rough_fallback += 1;
        } else {
            unrelated += 1;
        }
    }
    (precondition, rough_fallback, unrelated)
}

fn observe_failed_rest_cascade() -> FailedRestCascadeObservation {
    let (mut h, def) = load_harness();
    let shelter = scenario_place_id(&def, "Shelter");
    let aster = named_agent(&h, "Aster");
    let mut extractor = SurvivalForensicExtractor::new(aster);
    let thresholds = h
        .world
        .get_component_drive_thresholds(aster)
        .copied()
        .expect("Aster should carry drive thresholds");

    let mut known_rest_site_candidate_ticks = Vec::new();
    let mut start_failed_ticks = Vec::new();
    let mut rough_sleep_start_ticks = Vec::new();
    let mut fatigue_samples = Vec::new();
    let mut fatigue_critical_ticks = Vec::new();
    let mut hidden_fatigue_refills = 0;
    let mut previous_fatigue = None;

    for tick_num in 0..TICK_BUDGET {
        h.step_once();
        let tick = Tick(u64::from(tick_num));

        if known_rest_site_candidate_emitted(&h, aster, tick, shelter) {
            known_rest_site_candidate_ticks.push(tick);
        }

        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");
        for event in action_sink.events_for_at(aster, tick) {
            if event.action_name != "sleep" {
                continue;
            }
            match &event.kind {
                ActionTraceKind::StartFailed { .. } => start_failed_ticks.push(tick),
                ActionTraceKind::Started { targets } if targets.is_empty() => {
                    rough_sleep_start_ticks.push(tick);
                }
                _ => {}
            }
        }

        let needs = *h
            .world
            .get_component_homeostatic_needs(aster)
            .expect("Aster should carry needs");
        if let Some(previous) = previous_fatigue
            && needs.fatigue < previous
        {
            let sleeping_rough = h
                .world
                .get_component_sleep_episode(aster)
                .is_some_and(|episode| episode.place == shelter);
            if !sleeping_rough {
                hidden_fatigue_refills += 1;
            }
        }
        previous_fatigue = Some(needs.fatigue);
        fatigue_samples.push(needs.fatigue);

        let exposure = h
            .world
            .get_component_deprivation_exposure(aster)
            .expect("Aster should carry deprivation exposure");
        fatigue_critical_ticks.push(exposure.fatigue_critical_ticks);

        observe_critical_windows(&mut extractor, &h, aster, tick, &needs, &thresholds);

        if start_failed_ticks.len() >= MIN_FAILED_REST_OPPORTUNITIES
            && rough_sleep_start_ticks.len() >= MIN_FAILED_REST_OPPORTUNITIES
        {
            break;
        }
    }

    let reports = finalize_reports(extractor);
    let (precondition_failed_rest, rough_fallback_failed_rest, unrelated_failed_rest) =
        failed_rest_counts(&reports);
    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let collapse_abort_count = action_sink
        .events_for(aster)
        .into_iter()
        .filter(|event| {
            matches!(event.kind, ActionTraceKind::Aborted { .. })
                && event.action_name == "sleep"
                && !h.event_log.events_by_tag(EventTag::Death).is_empty()
        })
        .count();

    FailedRestCascadeObservation {
        known_rest_site_candidate_ticks,
        start_failed_ticks,
        rough_sleep_start_ticks,
        failed_rest_opportunities: precondition_failed_rest + rough_fallback_failed_rest,
        rough_fallback_failed_rest,
        unrelated_failed_rest,
        fatigue_samples,
        fatigue_critical_ticks,
        collapse_abort_count,
        actor_dead: h.world.get_component_dead_at(aster).is_some(),
        hidden_fatigue_refills,
        event_log_hash: hash_serializable(&h.event_log).expect("event log should hash"),
    }
}

// Scenario 485: S174 Failed Rest Cascade Feed
// Setup: Aster is critically tired at a one-slot roofed shelter while Sleeper occupies it for a long sleep episode.
// Proves: Aster repeatedly emits the KnownRestSite sleep candidate, records failed-rest opportunities through rough fallback while the rest site remains occupied, and falls back to capped rough sleep while critical fatigue exposure accumulates without collapse.
// Chain: rest capacity belief -> Sleep candidate -> RestOccupancy start gate -> StartFailed trace -> rough Sleep fallback -> FailedRestOpportunity::RoughFallbackToKnownRestSite -> DeprivationExposure fatigue ticks.
// Handoff: This scenario is the feed for S175's exhaustion-collapse golden. S175 will extend the same scenario shape to provoke collapse-via-wound and eventual death.
#[test]
fn scenario_e_failed_rest_feed() {
    let observation = observe_failed_rest_cascade();

    assert!(
        observation.known_rest_site_candidate_ticks.len() >= MIN_FAILED_REST_OPPORTUNITIES,
        "expected repeated known rest-site candidates: {observation:#?}"
    );
    assert_eq!(
        observation.start_failed_ticks.len(),
        1,
        "only the initial race should hit the start gate; repeated known-impossible starts would violate FND-21: {observation:#?}"
    );
    assert!(
        observation.rough_sleep_start_ticks.len() >= MIN_FAILED_REST_OPPORTUNITIES,
        "expected rough-sleep fallback after failed starts: {observation:#?}"
    );
    assert!(
        observation.failed_rest_opportunities >= MIN_FAILED_REST_OPPORTUNITIES,
        "expected failed-rest opportunities in fatigue critical window: {observation:#?}"
    );
    assert!(
        observation.rough_fallback_failed_rest >= MIN_FAILED_REST_OPPORTUNITIES,
        "expected repeated rough-fallback failed-rest records: {observation:#?}"
    );
    assert_eq!(observation.unrelated_failed_rest, 0);
    assert!(
        observation
            .fatigue_critical_ticks
            .windows(2)
            .all(|pair| pair[1] >= pair[0]),
        "fatigue critical exposure should be monotonic: {observation:#?}"
    );
    assert!(
        observation
            .fatigue_critical_ticks
            .last()
            .copied()
            .unwrap_or(0)
            > 0,
        "fatigue should spend time in critical exposure: {observation:#?}"
    );
    assert_eq!(observation.hidden_fatigue_refills, 0);
    assert_eq!(observation.collapse_abort_count, 0);
    assert!(!observation.actor_dead);
}

#[test]
fn scenario_e_failed_rest_feed_replays_deterministically() {
    let first = observe_failed_rest_cascade();
    let second = observe_failed_rest_cascade();

    assert_eq!(first.event_log_hash, second.event_log_hash);
    assert_eq!(first, second);
}
