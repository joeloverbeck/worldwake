//! S174 Scenario A golden coverage for rest-site contention and rough sleep.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_ai::survival_forensics::FailedRestKind;
use worldwake_ai::{
    CriticalWindowReport, DecisionOutcome, GoalKind, OpportunityAnchor, OpportunityKey,
    SurvivalForensicExtractor,
};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{
    DecisionEventPayload, EntityId, EventTag, EventView, GoalKey, HomeostaticNeedId, Permille,
    SleepEpisodeEndedPayload, SleepEpisodeStartedPayload, StateHash, Tick, WakeReason,
    hash_serializable,
};
use worldwake_sim::ActionTraceKind;

const TICK_BUDGET: u32 = 220;

#[derive(Clone, Debug, Eq, PartialEq)]
struct SafeRestObservation {
    known_candidate_agents: BTreeSet<String>,
    known_candidate_tick: Option<Tick>,
    shelter_sleeper: String,
    rough_sleeper: String,
    start_failed_tick: Tick,
    rough_candidate_tick: Tick,
    rough_start_tick: Tick,
    max_rest_occupants: usize,
    shelter_recovery: Permille,
    rough_recovery: Permille,
    shelter_end_reason: WakeReason,
    rough_end_reason: WakeReason,
    failed_rest_count: usize,
    final_rest_occupancy_present: bool,
    event_log_hash: StateHash,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-safe-rest.ron")
}

fn load_harness() -> (GoldenHarness, ScenarioDef) {
    let def =
        load_scenario_file(&scenario_path()).expect("survival safe rest scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival safe rest scenario should spawn");
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

fn agent_name(agents: &BTreeMap<String, EntityId>, agent: EntityId) -> String {
    agents
        .iter()
        .find_map(|(name, candidate)| (*candidate == agent).then(|| name.clone()))
        .expect("agent id should have scenario name")
}

fn sleep_started_payloads(h: &GoldenHarness, sleeper: EntityId) -> Vec<SleepEpisodeStartedPayload> {
    h.event_log
        .events_by_tag(EventTag::SleepEpisodeStarted)
        .iter()
        .filter_map(|id| h.event_log.get(*id))
        .filter_map(|record| match record.decision_payload()? {
            DecisionEventPayload::SleepEpisodeStarted(payload) if payload.sleeper == sleeper => {
                Some(payload.clone())
            }
            _ => None,
        })
        .collect()
}

fn sleep_ended_payloads(h: &GoldenHarness, sleeper: EntityId) -> Vec<SleepEpisodeEndedPayload> {
    h.event_log
        .events_by_tag(EventTag::SleepEpisodeEnded)
        .iter()
        .filter_map(|id| h.event_log.get(*id))
        .filter_map(|record| match record.decision_payload()? {
            DecisionEventPayload::SleepEpisodeEnded(payload) if payload.sleeper == sleeper => {
                Some(payload.clone())
            }
            _ => None,
        })
        .collect()
}

fn record_known_sleep_candidates(
    h: &GoldenHarness,
    tick: Tick,
    agents: &BTreeMap<String, EntityId>,
    shelter: EntityId,
    known_candidate_agents: &mut BTreeSet<String>,
    known_candidate_tick: &mut Option<Tick>,
) {
    let opportunity = OpportunityKey {
        goal_key: GoalKey::from(GoalKind::Sleep),
        anchor: OpportunityAnchor::Place(shelter),
    };
    for (name, agent) in agents {
        let Some(trace) = h
            .driver
            .trace_sink()
            .and_then(|sink| sink.trace_at(*agent, tick))
        else {
            continue;
        };
        let DecisionOutcome::Planning(planning) = &trace.outcome else {
            continue;
        };
        if planning
            .candidates
            .generated_contains_opportunity(opportunity)
        {
            known_candidate_agents.insert(name.clone());
            known_candidate_tick.get_or_insert(tick);
        }
    }
}

fn rough_sleep_candidate_tick(h: &GoldenHarness, agent: EntityId, tick: Tick) -> Option<Tick> {
    let opportunity = OpportunityKey {
        goal_key: GoalKey::from(GoalKind::Sleep),
        anchor: OpportunityAnchor::None,
    };
    let trace = h
        .driver
        .trace_sink()
        .and_then(|sink| sink.trace_at(agent, tick))?;
    let DecisionOutcome::Planning(planning) = &trace.outcome else {
        return None;
    };
    planning
        .candidates
        .generated_contains_opportunity(opportunity)
        .then_some(tick)
}

fn finalize_reports(
    extractors: BTreeMap<String, SurvivalForensicExtractor>,
) -> BTreeMap<String, Vec<CriticalWindowReport>> {
    extractors
        .into_iter()
        .map(|(name, extractor)| (name, extractor.finalize()))
        .collect()
}

fn failed_rest_count(reports: &[CriticalWindowReport]) -> usize {
    reports
        .iter()
        .filter(|report| report.need == HomeostaticNeedId::Fatigue)
        .flat_map(|report| &report.frames)
        .flat_map(|frame| &frame.failed_rest_opportunities)
        .filter(|opportunity| opportunity.kind == FailedRestKind::PreconditionRejected)
        .count()
}

fn observe_safe_rest() -> SafeRestObservation {
    let (mut h, def) = load_harness();
    let shelter = scenario_place_id(&def, "Shelter North");
    let agents = named_agents(&h);
    assert_eq!(agents.len(), 2);

    let mut extractors = agents
        .iter()
        .map(|(name, agent)| (name.clone(), SurvivalForensicExtractor::new(*agent)))
        .collect::<BTreeMap<_, _>>();
    let thresholds = agents
        .iter()
        .map(|(name, agent)| {
            (
                name.clone(),
                h.world
                    .get_component_drive_thresholds(*agent)
                    .copied()
                    .expect("scenario agents should carry drive thresholds"),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut known_candidate_agents = BTreeSet::new();
    let mut known_candidate_tick = None;
    let mut start_failed: Option<(String, Tick)> = None;
    let mut rough_candidate_tick = None;
    let mut max_rest_occupants = 0usize;

    for tick_num in 0..TICK_BUDGET {
        h.step_once();
        let tick = Tick(u64::from(tick_num));

        record_known_sleep_candidates(
            &h,
            tick,
            &agents,
            shelter,
            &mut known_candidate_agents,
            &mut known_candidate_tick,
        );

        if let Some(occupancy) = h.world.get_component_rest_occupancy(shelter) {
            max_rest_occupants = max_rest_occupants.max(occupancy.occupants.len());
        }

        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");
        for (name, agent) in &agents {
            let needs = h
                .world
                .get_component_homeostatic_needs(*agent)
                .expect("scenario agents should carry needs");
            observe_critical_windows(
                extractors
                    .get_mut(name)
                    .expect("every agent should have a forensic extractor"),
                &h,
                *agent,
                tick,
                needs,
                thresholds
                    .get(name)
                    .expect("every agent should have thresholds"),
            );

            for event in action_sink.events_for_at(*agent, tick) {
                if event.action_name == "sleep"
                    && matches!(event.kind, ActionTraceKind::StartFailed { .. })
                {
                    start_failed = Some((name.clone(), tick));
                }
            }
        }

        if let Some((_, failed_agent)) = start_failed
            .as_ref()
            .and_then(|(name, _)| agents.get(name).map(|agent| (name, *agent)))
            && rough_candidate_tick.is_none()
        {
            rough_candidate_tick = rough_sleep_candidate_tick(&h, failed_agent, tick);
        }

        if agents
            .values()
            .all(|agent| !sleep_ended_payloads(&h, *agent).is_empty())
        {
            break;
        }
    }

    let (rough_sleeper, start_failed_tick) = start_failed.unwrap_or_else(|| {
        let events = h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events()
            .iter()
            .filter(|event| event.action_name == "sleep")
            .map(|event| {
                format!(
                    "tick={:?} actor={} kind={:?} detail={:?}",
                    event.tick, event.actor, event.kind, event.detail
                )
            })
            .collect::<Vec<_>>();
        let rankings = agents
            .iter()
            .filter_map(|(name, agent)| {
                let trace = h
                    .driver
                    .trace_sink()
                    .and_then(|sink| sink.trace_at(*agent, Tick(0)))?;
                let DecisionOutcome::Planning(planning) = &trace.outcome else {
                    return None;
                };
                Some(format!(
                    "{name}: selected={:?} ranked={:?}",
                    planning.selection.selected_opportunity,
                    planning
                        .candidates
                        .ranked_summaries_for_goal(GoalKey::from(GoalKind::Sleep))
                ))
            })
            .collect::<Vec<_>>();
        panic!(
            "one agent should fail the full rest-site precondition; known_candidate_agents={known_candidate_agents:?}, known_candidate_tick={known_candidate_tick:?}, sleep_events={events:#?}, rankings={rankings:#?}"
        );
    });
    let rough_agent = *agents
        .get(&rough_sleeper)
        .expect("rough sleeper should be a scenario agent");
    let shelter_agent = agents
        .values()
        .copied()
        .find(|agent| *agent != rough_agent)
        .expect("other agent should be shelter sleeper");
    let shelter_sleeper = agent_name(&agents, shelter_agent);
    let shelter_start = sleep_started_payloads(&h, shelter_agent)
        .into_iter()
        .next()
        .expect("shelter sleeper should start sleep");
    let rough_start = sleep_started_payloads(&h, rough_agent)
        .into_iter()
        .next()
        .expect("rough sleeper should start rough sleep");
    let shelter_end = sleep_ended_payloads(&h, shelter_agent)
        .into_iter()
        .next()
        .expect("shelter sleep should finish within budget");
    let rough_end = sleep_ended_payloads(&h, rough_agent)
        .into_iter()
        .next()
        .expect("rough sleep should finish within budget");
    let reports = finalize_reports(extractors);
    let failed_rest_count = failed_rest_count(
        reports
            .get(&rough_sleeper)
            .expect("rough sleeper should have forensics reports"),
    );

    assert_eq!(shelter_start.place, shelter);
    assert_eq!(rough_start.place, shelter);
    assert_eq!(shelter_start.recovery_modifier.value(), 1100);
    assert_eq!(rough_start.recovery_modifier.value(), 300);
    let rough_start_tick = h
        .action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for(rough_agent)
        .into_iter()
        .find_map(|event| {
            (event.action_name == "sleep"
                && matches!(
                    event.kind,
                    ActionTraceKind::Started { ref targets } if targets.is_empty()
                ))
            .then_some(event.tick)
        })
        .expect("rough sleeper should start a targetless Sleep action after rejection");

    SafeRestObservation {
        known_candidate_agents,
        known_candidate_tick,
        shelter_sleeper,
        rough_sleeper,
        start_failed_tick,
        rough_candidate_tick: rough_candidate_tick
            .expect("losing agent should emit a targetless rough-sleep candidate"),
        rough_start_tick,
        max_rest_occupants,
        shelter_recovery: shelter_end.accumulated_recovery,
        rough_recovery: rough_end.accumulated_recovery,
        shelter_end_reason: shelter_end.end_reason,
        rough_end_reason: rough_end.end_reason,
        failed_rest_count,
        final_rest_occupancy_present: h.world.get_component_rest_occupancy(shelter).is_some(),
        event_log_hash: hash_serializable(&h.event_log).expect("event log should hash"),
    }
}

// Scenario 483: S174 Safe Rest Contention
// Setup: Two critically tired agents are co-located at a one-slot roofed shelter.
// Proves: both agents see the KnownRestSite candidate, one occupant writes RestOccupancy, the loser records a precondition-rejected failed-rest opportunity, then rough-sleeps under the recovery floor.
// Chain: rest capacity belief -> Sleep candidate -> RestOccupancy start gate -> StartFailed trace -> rough Sleep fallback -> SleepEpisodeEnded recovery delta -> SurvivalForensicExtractor.
#[test]
fn scenario_a_rest_site_contention() {
    let observation = observe_safe_rest();

    assert_eq!(
        observation.known_candidate_agents,
        BTreeSet::from(["Aster".to_string(), "Bram".to_string()])
    );
    assert_eq!(observation.known_candidate_tick, Some(Tick(0)));
    assert_eq!(observation.max_rest_occupants, 1);
    assert!(
        observation.rough_start_tick > observation.start_failed_tick,
        "rough fallback action should start after the rest-site precondition rejection"
    );
    assert!(
        observation.shelter_recovery > observation.rough_recovery,
        "shelter recovery should beat capped rough sleep: {observation:?}"
    );
    assert!(matches!(
        observation.shelter_end_reason,
        WakeReason::IntendedDuration | WakeReason::TargetRecovery
    ));
    assert!(matches!(
        observation.rough_end_reason,
        WakeReason::IntendedDuration | WakeReason::TargetRecovery
    ));
    assert_eq!(observation.failed_rest_count, 1);
    assert!(
        !observation.final_rest_occupancy_present,
        "committed sleep should release RestOccupancy"
    );
}

#[test]
fn scenario_a_rest_site_contention_replays_deterministically() {
    let first = observe_safe_rest();
    let second = observe_safe_rest();

    assert_eq!(first.event_log_hash, second.event_log_hash);
    assert_eq!(first, second);
}
