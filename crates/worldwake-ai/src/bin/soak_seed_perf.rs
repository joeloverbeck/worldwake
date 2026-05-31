use std::collections::BTreeMap;
use std::time::Instant;

#[allow(unused_imports, clippy::wildcard_imports)]
#[path = "../../tests/golden_harness/mod.rs"]
mod golden_harness;

use golden_harness::soak_world::build_t30_world;
use worldwake_ai::perf_telemetry::{
    PlanningTelemetrySession, PlanningTelemetrySummary, PlanningWindowSummary,
    SOAK_SEED_PERF_TELEMETRY_CONFIG,
};
use worldwake_core::{
    CauseRef, CommodityKind, DecisionEventPayload, EventId, EventView, Permille, Seed,
    hash_event_log, hash_world, total_authoritative_commodity_quantity,
    verify_authoritative_conservation,
};

struct SoakRunResult {
    duration_ms: u128,
    planning_summary: PlanningTelemetrySummary,
    world_hash: String,
    event_log_hash: String,
    event_count: usize,
    max_decision_payload_bytes: BTreeMap<&'static str, usize>,
}

const TOTAL_TICKS: u64 = 10080;
const GOAL_COMMITTED_BYTE_CEILING: usize = 2048;
const PLAN_ADOPTED_BYTE_CEILING: usize = 1024;
const BLOCKER_RECORDED_BYTE_CEILING: usize = 4096;
const REPLAN_TRIGGERED_BYTE_CEILING: usize = 4096;
const EXPECTATION_MISMATCH_BYTE_CEILING: usize = 4096;
const SOURCE_EXPECTATION_FAILURE_BYTE_CEILING: usize = 3072;

fn nanos_to_micros(nanos: u128) -> u128 {
    nanos / 1_000
}

fn format_ratio_millis(ratio_millis: Option<u128>) -> String {
    match ratio_millis {
        Some(value) => format!("{}.{:03}", value / 1000, value % 1000),
        None => "NA".to_string(),
    }
}

fn emit_window(label: &str, summary: PlanningWindowSummary) {
    println!("{label}_tick_start={}", summary.window.start_tick);
    println!(
        "{label}_tick_end_exclusive={}",
        summary.window.end_tick_exclusive
    );
    println!("{label}_planning_sample_count={}", summary.sample_count);
    println!(
        "{label}_planning_total_us={}",
        nanos_to_micros(summary.total_duration_nanos)
    );
    println!(
        "{label}_planning_avg_us={}",
        summary.average_duration_nanos().map_or(0, nanos_to_micros)
    );
}

fn decision_payload_tag(payload: &DecisionEventPayload) -> &'static str {
    match payload {
        DecisionEventPayload::GoalOffered(_) => "GoalOffered",
        DecisionEventPayload::GoalSuppressed(_) => "GoalSuppressed",
        DecisionEventPayload::GoalCommitted(_) => "GoalCommitted",
        DecisionEventPayload::PlanAdopted(_) => "PlanAdopted",
        DecisionEventPayload::BlockerRecorded(_) => "BlockerRecorded",
        DecisionEventPayload::GoalAbandoned(_) => "GoalAbandoned",
        DecisionEventPayload::GoalSuspended(_) => "GoalSuspended",
        DecisionEventPayload::PlanInvalidated(_) => "PlanInvalidated",
        DecisionEventPayload::RepairApplied(_) => "RepairApplied",
        DecisionEventPayload::ExpectationMismatch(_) => "ExpectationMismatch",
        DecisionEventPayload::SourceExpectationFailure(_) => "SourceExpectationFailure",
        DecisionEventPayload::ResourceSourceQualityObserved(_) => "ResourceSourceQualityObserved",
        DecisionEventPayload::SleepEpisodeStarted(_) => "SleepEpisodeStarted",
        DecisionEventPayload::SleepEpisodeEnded(_) => "SleepEpisodeEnded",
        DecisionEventPayload::WashFacilityUsed(_) => "WashFacilityUsed",
        DecisionEventPayload::WasteCreated(_) => "WasteCreated",
        DecisionEventPayload::ReplanTriggered(_) => "ReplanTriggered",
        DecisionEventPayload::SurveyRecorded(_) => "SurveyRecorded",
    }
}

fn decision_payload_byte_ceiling(payload: &DecisionEventPayload) -> Option<usize> {
    match payload {
        DecisionEventPayload::GoalCommitted(_) => Some(GOAL_COMMITTED_BYTE_CEILING),
        DecisionEventPayload::PlanAdopted(_) => Some(PLAN_ADOPTED_BYTE_CEILING),
        DecisionEventPayload::BlockerRecorded(_) => Some(BLOCKER_RECORDED_BYTE_CEILING),
        DecisionEventPayload::ReplanTriggered(_) => Some(REPLAN_TRIGGERED_BYTE_CEILING),
        DecisionEventPayload::ExpectationMismatch(_) => Some(EXPECTATION_MISMATCH_BYTE_CEILING),
        DecisionEventPayload::SourceExpectationFailure(_) => {
            Some(SOURCE_EXPECTATION_FAILURE_BYTE_CEILING)
        }
        _ => None,
    }
}

fn assert_decision_payload_size(
    payload: &DecisionEventPayload,
    event_id: EventId,
    tick: worldwake_core::Tick,
    max_decision_payload_bytes: &mut BTreeMap<&'static str, usize>,
) {
    let tag = decision_payload_tag(payload);
    let bytes = bincode::serialize(payload)
        .expect("decision payload should serialize during soak size sweep")
        .len();
    max_decision_payload_bytes
        .entry(tag)
        .and_modify(|max| *max = (*max).max(bytes))
        .or_insert(bytes);

    if let Some(ceiling) = decision_payload_byte_ceiling(payload) {
        assert!(
            bytes <= ceiling,
            "{tag} decision payload for event {event_id:?} at tick {tick:?} serialized to {bytes} bytes, exceeding ceiling {ceiling}"
        );
    }
}

fn parse_seed_arg() -> Result<u8, String> {
    let mut args = std::env::args().skip(1);
    let Some(seed_text) = args.next() else {
        return Err(
            "usage: cargo run -p worldwake-ai --bin soak_seed_perf -- <seed-id>".to_string(),
        );
    };
    if args.next().is_some() {
        return Err("expected exactly one positional argument: <seed-id>".to_string());
    }
    seed_text
        .parse::<u8>()
        .map_err(|e| format!("invalid seed-id `{seed_text}`: {e}"))
}

fn seed_from_id(seed_id: u8) -> Seed {
    let mut bytes = [0u8; 32];
    bytes[0] = seed_id;
    bytes[31] = seed_id.wrapping_mul(17);
    Seed(bytes)
}

fn run_one_seed(seed: Seed) -> SoakRunResult {
    let start = Instant::now();
    let planning_session = PlanningTelemetrySession::start(SOAK_SEED_PERF_TELEMETRY_CONFIG);
    let (mut h, all_agents, _ruling_faction, _bandit_faction, _office) = build_t30_world(seed);
    let commodities_to_check = [
        CommodityKind::Apple,
        CommodityKind::Grain,
        CommodityKind::Bread,
        CommodityKind::Coin,
    ];

    let mut commodity_totals: BTreeMap<CommodityKind, u64> = commodities_to_check
        .iter()
        .map(|&commodity| {
            (
                commodity,
                total_authoritative_commodity_quantity(&h.world, commodity),
            )
        })
        .collect();

    let initial_world_hash = hash_world(&h.world).expect("T30 soak world should hash");
    let mut prev_tick = h.scheduler.current_tick();
    let mut last_checked_event = 0u64;
    let mut max_decision_payload_bytes = BTreeMap::new();

    for _ in 0..TOTAL_TICKS {
        h.step_once();
        let current_tick = h.scheduler.current_tick();

        for (&commodity, total) in &mut commodity_totals {
            let actual = total_authoritative_commodity_quantity(&h.world, commodity);
            if actual > *total {
                *total = actual;
            }
            verify_authoritative_conservation(&h.world, commodity, actual).unwrap_or_else(|e| {
                panic!("conservation violation at tick {current_tick:?} for {commodity:?}: {e}")
            });
        }

        for &agent in &all_agents {
            if h.agent_is_dead(agent) {
                continue;
            }
            if let Some(needs) = h.world.get_component_homeostatic_needs(agent) {
                let max = Permille::new(1000).unwrap();
                assert!(
                    needs.hunger <= max
                        && needs.thirst <= max
                        && needs.fatigue <= max
                        && needs.bladder <= max
                        && needs.dirtiness <= max,
                    "needs out of bounds for agent {agent:?} at tick {current_tick:?}: {needs:?}"
                );
            }
        }

        for &agent in &all_agents {
            if let Some(dead_at) = h.world.get_component_dead_at(agent) {
                assert!(
                    !h.agent_has_active_action(agent),
                    "dead agent {agent:?} (died at {:?}) has active action at tick {current_tick:?}",
                    dead_at.tick
                );
            }
        }

        for &agent in &all_agents {
            if h.agent_is_dead(agent) {
                continue;
            }
            if let Some(place) = h.world.effective_place(agent) {
                assert!(
                    h.world.topology().place(place).is_some(),
                    "agent {agent:?} placed at non-existent place {place:?} at tick {current_tick:?}"
                );
            }
        }

        assert!(
            current_tick > prev_tick,
            "tick did not advance: prev={prev_tick:?}, current={current_tick:?}"
        );
        prev_tick = current_tick;

        let log_len = h.event_log.len() as u64;
        for idx in last_checked_event..log_len {
            let event_id = EventId(idx);
            if let Some(record) = h.event_log.get(event_id) {
                match record.cause() {
                    CauseRef::Event(cause_id) => {
                        assert!(
                            h.event_log.get(cause_id).is_some(),
                            "event {event_id:?} references non-existent cause {cause_id:?} at tick {current_tick:?}"
                        );
                    }
                    CauseRef::SystemTick(_) | CauseRef::Bootstrap | CauseRef::ExternalInput(_) => {}
                }
                if let Some(payload) = record.decision_payload() {
                    assert_decision_payload_size(
                        payload,
                        event_id,
                        current_tick,
                        &mut max_decision_payload_bytes,
                    );
                }
            }
        }
        last_checked_event = log_len;
    }

    let final_world_hash = hash_world(&h.world).expect("post-soak world should hash");
    assert_ne!(
        initial_world_hash, final_world_hash,
        "world state did not change after {TOTAL_TICKS} ticks (seed: {seed:?})"
    );
    let event_log_hash = hash_event_log(&h.event_log).expect("post-soak event log should hash");
    let planning_summary = planning_session.finish();

    SoakRunResult {
        duration_ms: start.elapsed().as_millis(),
        planning_summary,
        world_hash: format!("{final_world_hash:?}"),
        event_log_hash: format!("{event_log_hash:?}"),
        event_count: h.event_log.len(),
        max_decision_payload_bytes,
    }
}

fn main() {
    let seed_id = parse_seed_arg().unwrap_or_else(|message| {
        eprintln!("{message}");
        std::process::exit(2);
    });
    let result = run_one_seed(seed_from_id(seed_id));

    println!("seed_id={seed_id}");
    println!("duration_ms={}", result.duration_ms);
    println!("event_count={}", result.event_count);
    println!("planning_metric=plan_and_validate_next_step");
    emit_window("early", result.planning_summary.early);
    emit_window("late", result.planning_summary.late);
    println!(
        "late_to_early_planning_avg_ratio={}",
        format_ratio_millis(result.planning_summary.late_to_early_average_ratio_millis())
    );
    println!("world_hash={}", result.world_hash);
    println!("event_log_hash={}", result.event_log_hash);
    for (tag, bytes) in result.max_decision_payload_bytes {
        println!("max_decision_payload_bytes_{tag}={bytes}");
    }
}
