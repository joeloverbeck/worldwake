/// Diagnostic binary: runs ticks of a T30 soak world and prints per-interval
/// timing, data structure sizes, and RSS memory usage.
use std::time::Instant;

#[allow(unused_imports, clippy::wildcard_imports)]
#[path = "../../tests/golden_harness/mod.rs"]
mod golden_harness;

use golden_harness::soak_world::build_t30_world;
use worldwake_core::Seed;

const DIAG_TICKS: u64 = 300;
const REPORT_INTERVAL: u64 = 60;

fn seed_from_id(seed_id: u8) -> Seed {
    let mut bytes = [0u8; 32];
    bytes[0] = seed_id;
    bytes[31] = seed_id.wrapping_mul(17);
    Seed(bytes)
}

fn rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().nth(1)?.parse::<u64>().ok())
        .map_or(0.0, |pages| (pages * 4096) as f64 / (1024.0 * 1024.0))
}

fn main() {
    let seed = seed_from_id(0);
    let (mut h, _all_agents, _ruling_faction, _bandit_faction, _office) = build_t30_world(seed);

    let global_start = Instant::now();
    let mut interval_start = Instant::now();

    eprintln!(
        "{:>5} {:>8} {:>10} {:>8} {:>10}",
        "tick", "dt_ms", "events", "rss_mb", "mb/event"
    );

    let mut prev_events: usize = 0;
    let mut prev_rss: f64 = rss_mb();

    for tick_num in 0..DIAG_TICKS {
        h.step_once();

        if (tick_num + 1) % REPORT_INTERVAL == 0 {
            let dt = interval_start.elapsed().as_millis();
            let event_count = h.event_log.len();
            let rss = rss_mb();

            let new_events = event_count - prev_events;
            let rss_delta = rss - prev_rss;
            let mb_per_event = if new_events > 0 {
                rss_delta / new_events as f64
            } else {
                0.0
            };

            eprintln!(
                "{:>5} {:>8} {:>10} {:>8.1} {:>10.3}",
                tick_num + 1,
                dt,
                event_count,
                rss,
                mb_per_event,
            );

            prev_events = event_count;
            prev_rss = rss;
            interval_start = Instant::now();
        }
    }

    eprintln!("\ntotal_elapsed_ms={}", global_start.elapsed().as_millis());
    eprintln!("final_rss_mb={:.1}", rss_mb());
    eprintln!("final_events={}", h.event_log.len());
    eprintln!(
        "avg_kb_per_event={:.1}",
        rss_mb() * 1024.0 / h.event_log.len() as f64
    );
}
