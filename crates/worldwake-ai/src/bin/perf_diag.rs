/// Diagnostic binary: runs 500 ticks of a T30 soak world and prints per-tick
/// timing and growth statistics to identify the performance regression.
use std::time::Instant;

#[allow(unused_imports, clippy::wildcard_imports)]
#[path = "../../tests/golden_harness/mod.rs"]
mod golden_harness;

use golden_harness::soak_world::build_t30_world;
use worldwake_core::Seed;

const DIAG_TICKS: u64 = 2880;

fn seed_from_id(seed_id: u8) -> Seed {
    let mut bytes = [0u8; 32];
    bytes[0] = seed_id;
    bytes[31] = seed_id.wrapping_mul(17);
    Seed(bytes)
}

fn main() {
    let seed = seed_from_id(0);
    let (mut h, all_agents, _ruling_faction, _bandit_faction, _office) = build_t30_world(seed);

    let global_start = Instant::now();
    let mut prev_event_count: usize = 0;
    let mut slowest_tick: u64 = 0;
    let mut slowest_tick_ms: u128 = 0;

    eprintln!(
        "{:>5} {:>8} {:>10} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "tick", "tick_ms", "evts_tot", "soc_obs", "bel_ent", "told", "heard", "inst_bel"
    );

    for tick_num in 0..DIAG_TICKS {
        let tick_start = Instant::now();
        h.step_once();
        let tick_ms = tick_start.elapsed().as_millis();

        let event_count = h.event_log.len();
        let _events_this_tick = event_count - prev_event_count;
        prev_event_count = event_count;

        if tick_ms > slowest_tick_ms {
            slowest_tick_ms = tick_ms;
            slowest_tick = tick_num + 1;
        }

        let should_print = tick_num < 10
            || (tick_num < 100 && tick_num % 20 == 19)
            || tick_num % 50 == 49;

        if should_print {
            let mut total_social_obs: usize = 0;
            let mut total_belief_entities: usize = 0;
            let mut total_told: usize = 0;
            let mut total_heard: usize = 0;
            let mut total_institutional: usize = 0;
            for &agent in &all_agents {
                if let Some(beliefs) = h.world.get_component_agent_belief_store(agent) {
                    total_belief_entities += beliefs.known_entities.len();
                    total_social_obs += beliefs.social_observations.len();
                    total_told += beliefs.told_beliefs.len();
                    total_heard += beliefs.heard_beliefs.len();
                    total_institutional += beliefs
                        .institutional_beliefs
                        .values()
                        .map(|v| v.len())
                        .sum::<usize>();
                }
            }

            eprintln!(
                "{:>5} {:>8} {:>10} {:>8} {:>8} {:>8} {:>8} {:>8}",
                tick_num + 1,
                tick_ms,
                event_count,
                total_social_obs,
                total_belief_entities,
                total_told,
                total_heard,
                total_institutional,
            );
        }
    }

    eprintln!("\nslowest_tick={slowest_tick} at {slowest_tick_ms}ms");
    eprintln!("total_elapsed_ms={}", global_start.elapsed().as_millis());
}
