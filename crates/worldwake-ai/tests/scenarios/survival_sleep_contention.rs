//! S174 Scenario B golden coverage for multi-slot rest contention and queue promotion.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_ai::{
    CriticalWindowReport, DecisionOutcome, GoalKind, OpportunityAnchor, OpportunityKey,
    SurvivalForensicExtractor,
};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{
    ClaimantOutcome, DecisionEventPayload, EntityId, EventTag, EventView, GoalKey,
    HomeostaticNeedId, StateHash, Tick, hash_serializable,
};
use worldwake_sim::ActionTraceKind;

const TICK_BUDGET: u32 = 260;

#[derive(Clone, Debug, Eq, PartialEq)]
struct SleepContentionObservation {
    known_candidate_agents: BTreeSet<String>,
    known_candidate_tick: Option<Tick>,
    immediate_sleepers: BTreeSet<String>,
    queued_agent: String,
    start_failed_tick: Tick,
    queue_join_tick: Tick,
    grant_tick: Tick,
    promoted_sleep_tick: Tick,
    max_rest_occupants: usize,
    sleep_end_count: usize,
    fatigue_frame_counts: BTreeMap<String, usize>,
    event_log_hash: StateHash,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-sleep-contention.ron")
}

fn load_harness() -> (GoldenHarness, ScenarioDef) {
    let def = load_scenario_file(&scenario_path())
        .expect("survival sleep contention scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival sleep contention scenario should spawn");
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

fn record_known_sleep_candidates(
    h: &GoldenHarness,
    tick: Tick,
    agents: &BTreeMap<String, EntityId>,
    barracks: EntityId,
    known_candidate_agents: &mut BTreeSet<String>,
    known_candidate_tick: &mut Option<Tick>,
) {
    let opportunity = OpportunityKey {
        goal_key: GoalKey::from(GoalKind::Sleep),
        anchor: OpportunityAnchor::Place(barracks),
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

fn sleep_end_count(h: &GoldenHarness) -> usize {
    h.event_log
        .events_by_tag(EventTag::SleepEpisodeEnded)
        .iter()
        .filter_map(|id| h.event_log.get(*id))
        .filter(|record| {
            matches!(
                record.decision_payload(),
                Some(DecisionEventPayload::SleepEpisodeEnded(_))
            )
        })
        .count()
}

fn queue_event_tick(h: &GoldenHarness, tag: EventTag, barracks: EntityId) -> Option<Tick> {
    h.event_log
        .events_by_tag(tag)
        .iter()
        .filter_map(|id| h.event_log.get(*id))
        .find_map(|record| {
            let payload = record.contention_event_payload()?;
            (payload.contested_affordance.facility == barracks).then_some(record.tick())
        })
}

fn queue_action_commit_tick(h: &GoldenHarness) -> Option<Tick> {
    h.action_trace_sink()?.events().iter().find_map(|event| {
        (event.action_name == "queue_for_facility_use"
            && matches!(event.kind, ActionTraceKind::Committed { .. }))
        .then_some(event.tick)
    })
}

fn granted_agent(h: &GoldenHarness, barracks: EntityId) -> Option<EntityId> {
    h.event_log
        .events_by_tag(EventTag::QueueGrantPromoted)
        .iter()
        .filter_map(|id| h.event_log.get(*id))
        .find_map(|record| {
            let payload = record.contention_event_payload()?;
            (payload.contested_affordance.facility == barracks)
                .then_some(payload.winner)
                .flatten()
        })
}

fn queued_behind_agent(h: &GoldenHarness, barracks: EntityId) -> Option<EntityId> {
    h.event_log
        .events_by_tag(EventTag::ContentionResolved)
        .iter()
        .filter_map(|id| h.event_log.get(*id))
        .find_map(|record| {
            let payload = record.contention_event_payload()?;
            (payload.contested_affordance.facility == barracks).then(|| {
                payload.claimants.iter().find_map(|claimant| {
                    (claimant.outcome == ClaimantOutcome::QueuedBehind).then_some(claimant.agent)
                })
            })?
        })
}

fn finalize_reports(
    extractors: BTreeMap<String, SurvivalForensicExtractor>,
) -> BTreeMap<String, Vec<CriticalWindowReport>> {
    extractors
        .into_iter()
        .map(|(name, extractor)| (name, extractor.finalize()))
        .collect()
}

fn observe_sleep_contention() -> SleepContentionObservation {
    let (mut h, def) = load_harness();
    let barracks = scenario_place_id(&def, "Barracks");
    let agents = named_agents(&h);
    assert_eq!(agents.len(), 3);

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
    let mut immediate_sleepers = BTreeSet::new();
    let mut start_failed: Option<(String, Tick)> = None;
    let mut promoted_sleep_tick = None;
    let mut max_rest_occupants = 0usize;

    for tick_num in 0..TICK_BUDGET {
        h.step_once();
        let tick = Tick(u64::from(tick_num));

        record_known_sleep_candidates(
            &h,
            tick,
            &agents,
            barracks,
            &mut known_candidate_agents,
            &mut known_candidate_tick,
        );

        if let Some(occupancy) = h.world.get_component_rest_occupancy(barracks) {
            max_rest_occupants = max_rest_occupants.max(occupancy.occupants.len());
        }

        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");
        let current_granted = granted_agent(&h, barracks);
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
                    && matches!(event.kind, ActionTraceKind::Started { ref targets } if targets == &[barracks])
                {
                    if current_granted == Some(*agent) {
                        promoted_sleep_tick.get_or_insert(tick);
                    } else {
                        immediate_sleepers.insert(name.clone());
                    }
                }
                if event.action_name == "sleep"
                    && matches!(event.kind, ActionTraceKind::StartFailed { .. })
                {
                    start_failed = Some((name.clone(), tick));
                }
            }
        }

        if sleep_end_count(&h) >= 3 {
            break;
        }
    }

    let queued_agent = queued_behind_agent(&h, barracks)
        .or_else(|| granted_agent(&h, barracks))
        .or_else(|| {
            start_failed
                .as_ref()
                .and_then(|(name, _)| agents.get(name).copied())
        })
        .expect("one actor should queue after the full rest-site rejection");
    let reports = finalize_reports(extractors);
    let fatigue_frame_counts = reports
        .iter()
        .map(|(name, reports)| {
            (
                name.clone(),
                reports
                    .iter()
                    .filter(|report| report.need == HomeostaticNeedId::Fatigue)
                    .flat_map(|report| &report.frames)
                    .count(),
            )
        })
        .collect();
    let (_, start_failed_tick) = start_failed
        .clone()
        .expect("third actor should fail the full rest-site precondition");

    let queue_join_tick =
        queue_action_commit_tick(&h).expect("queue admission should commit queue action");

    SleepContentionObservation {
        known_candidate_agents,
        known_candidate_tick,
        immediate_sleepers,
        queued_agent: agent_name(&agents, queued_agent),
        start_failed_tick,
        queue_join_tick,
        grant_tick: queue_event_tick(&h, EventTag::QueueGrantPromoted, barracks).unwrap_or_else(
            || {
                let action_events = h
                    .action_trace_sink()
                    .expect("action tracing should be enabled")
                    .events()
                    .iter()
                    .filter(|event| {
                        event.action_name == "sleep" || event.action_name.contains("queue")
                    })
                    .map(|event| {
                        format!(
                            "tick={:?} actor={} action={} kind={:?} detail={:?}",
                            event.tick, event.actor, event.action_name, event.kind, event.detail
                        )
                    })
                    .collect::<Vec<_>>();
                panic!(
                    "queue promotion should emit QueueGrantPromoted; action_events={action_events:#?}"
                );
            },
        ),
        promoted_sleep_tick: promoted_sleep_tick
            .expect("promoted actor should start targeted sleep after grant"),
        max_rest_occupants,
        sleep_end_count: sleep_end_count(&h),
        fatigue_frame_counts,
        event_log_hash: hash_serializable(&h.event_log).expect("event log should hash"),
    }
}

// Scenario 482: S174 Multi-Slot Rest Contention
// Setup: Three critically tired agents are co-located at a two-slot barracks.
// Proves: two agents occupy the barracks, the loser queues, promotion fires after release, and the promoted agent sleeps.
// Chain: rest capacity belief -> Sleep candidate -> RestOccupancy capacity gate -> queue join -> QueueGrantPromoted -> promoted targeted Sleep.
#[test]
fn scenario_b_multi_slot_contention() {
    let observation = observe_sleep_contention();

    assert_eq!(
        observation.known_candidate_agents,
        BTreeSet::from(["Aster".to_string(), "Bram".to_string(), "Cleo".to_string()])
    );
    assert_eq!(observation.known_candidate_tick, Some(Tick(0)));
    assert_eq!(observation.immediate_sleepers.len(), 2);
    assert_eq!(observation.max_rest_occupants, 2);
    assert!(observation.queue_join_tick >= observation.start_failed_tick);
    assert!(observation.grant_tick > observation.queue_join_tick);
    assert!(observation.promoted_sleep_tick >= observation.grant_tick);
    assert_eq!(observation.sleep_end_count, 3);
    assert!(
        observation
            .fatigue_frame_counts
            .values()
            .all(|frame_count| *frame_count <= 20),
        "elevated-fatigue windows should remain bounded by queue wait: {observation:?}"
    );
    assert!(
        !observation
            .immediate_sleepers
            .contains(&observation.queued_agent),
        "queued agent should be the initial loser, not one of the immediate sleepers"
    );
}

#[test]
fn scenario_b_multi_slot_contention_replays_deterministically() {
    let first = observe_sleep_contention();
    let second = observe_sleep_contention();

    assert_eq!(first.event_log_hash, second.event_log_hash);
    assert_eq!(first, second);
}
