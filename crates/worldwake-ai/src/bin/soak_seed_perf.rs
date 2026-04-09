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
    CauseRef, CommodityKind, EventId, EventView, Permille, Seed, hash_event_log, hash_world,
    total_authoritative_commodity_quantity, verify_authoritative_conservation,
};

struct SoakRunResult {
    duration_ms: u128,
    planning_summary: PlanningTelemetrySummary,
    world_hash: String,
    event_log_hash: String,
    event_count: usize,
}

const TOTAL_TICKS: u64 = 10080;

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
}
