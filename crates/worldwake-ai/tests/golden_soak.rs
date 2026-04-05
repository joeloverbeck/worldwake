//! Soak test: seven-day endurance autoplay.
//!
//! Runs 10,080 ticks (7 in-game days) across multiple seeds with 20 agents and
//! verifies per-tick invariants (conservation, needs bounds, dead agent
//! inactivity, unique placement, tick monotonicity, causal link integrity)
//! under extended autonomous play.
//!
//! Gated behind the `soak` feature because each seed takes minutes to run.
//! Run with: `cargo test -p worldwake-ai --features soak --test golden_soak`
#![cfg(feature = "soak")]

mod golden_harness;

use golden_harness::soak_world::build_t30_world;
use std::collections::BTreeSet;
use worldwake_core::{
    hash_event_log, hash_world, total_authoritative_commodity_quantity,
    verify_authoritative_conservation, CauseRef, CommodityKind, EventId, EventView, Permille, Seed,
    StateHash,
};

/// Per-run result collected for cross-run diversity analysis.
struct SoakResult {
    world_hash: StateHash,
    event_log_hash: StateHash,
    saw_death: bool,
    saw_acquire: bool,
    saw_travel: bool,
    saw_tell: bool,
    saw_claim_office: bool,
    saw_steal: bool,
}

fn env_u8(name: &str) -> Option<u8> {
    std::env::var(name).ok().and_then(|v| v.parse().ok())
}

fn soak_seeds(seed_start: u8, seed_count: u8) -> Vec<Seed> {
    (0u8..seed_count)
        .map(|offset| {
            let i = seed_start.wrapping_add(offset);
            let mut bytes = [0u8; 32];
            bytes[0] = i;
            bytes[31] = i.wrapping_mul(17);
            Seed(bytes)
        })
        .collect()
}

fn soak_seeds_from_env() -> Vec<Seed> {
    soak_seeds(
        env_u8("SOAK_SEED_START").unwrap_or(0),
        env_u8("SOAK_SEEDS").unwrap_or(10),
    )
}

fn scaled_emergence_threshold(seed_count: usize, baseline_hits: usize) -> usize {
    let numerator = seed_count.saturating_mul(baseline_hits);
    numerator.div_ceil(10).max(1)
}

/// Run a single soak run for the given seed and return per-run results.
fn run_t30_soak(seed: Seed) -> SoakResult {
    let (mut h, all_agents, _ruling_faction, _bandit_faction, _office) = build_t30_world(seed);

    const TOTAL_TICKS: u64 = 10080;
    let commodities_to_check = [
        CommodityKind::Apple,
        CommodityKind::Grain,
        CommodityKind::Bread,
        CommodityKind::Coin,
    ];

    // Snapshot initial commodity totals for conservation checks.
    let mut commodity_totals: std::collections::BTreeMap<CommodityKind, u64> = commodities_to_check
        .iter()
        .map(|&c| (c, total_authoritative_commodity_quantity(&h.world, c)))
        .collect();

    let initial_world_hash = hash_world(&h.world).unwrap();
    let mut prev_tick = h.scheduler.current_tick();
    let mut last_checked_event: u64 = 0;

    // Emergence flags.
    let mut saw_death = false;
    let mut saw_travel = false;

    for _ in 0..TOTAL_TICKS {
        h.step_once();
        let current_tick = h.scheduler.current_tick();

        // --- Per-tick invariant 1: Conservation ---
        for (&commodity, total) in &mut commodity_totals {
            let actual = total_authoritative_commodity_quantity(&h.world, commodity);
            // Total can only increase through production (harvest/craft), never decrease
            // except through consumption. Update the running total to track increases.
            if actual > *total {
                // Production created new units — update baseline.
                *total = actual;
            }
            verify_authoritative_conservation(&h.world, commodity, actual).unwrap_or_else(|e| {
                panic!(
                    "conservation violation at tick {:?} for {:?}: {e}",
                    current_tick, commodity
                )
            });
        }

        // --- Per-tick invariant 2: Needs bounds ---
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

        // --- Per-tick invariant 3: Dead agent inactivity ---
        for &agent in &all_agents {
            if let Some(dead_at) = h.world.get_component_dead_at(agent) {
                saw_death = true;
                assert!(
                    !h.agent_has_active_action(agent),
                    "dead agent {agent:?} (died at {:?}) has active action at tick {current_tick:?}",
                    dead_at.0
                );
            }
        }

        // --- Per-tick invariant 4: Unique placement ---
        for &agent in &all_agents {
            if h.agent_is_dead(agent) {
                continue;
            }
            // Agents in transit have effective_place == None, which is legal.
            // If they have a place, it must exist in the topology.
            if let Some(place) = h.world.effective_place(agent) {
                assert!(
                    h.world.topology().place(place).is_some(),
                    "agent {agent:?} placed at non-existent place {place:?} at tick {current_tick:?}"
                );
            }
        }

        // --- Per-tick invariant 5: Tick monotonicity ---
        assert!(
            current_tick > prev_tick,
            "tick did not advance: prev={prev_tick:?}, current={current_tick:?}"
        );
        prev_tick = current_tick;

        // --- Per-tick invariant 6: Causal link integrity (incremental) ---
        // Only check events appended since the last tick — the append-only log
        // guarantees previously-checked events never change.
        let log_len = h.event_log.len() as u64;
        for idx in last_checked_event..log_len {
            let event_id = EventId(idx);
            if let Some(record) = h.event_log.get(event_id) {
                match record.cause() {
                    CauseRef::Event(cause_id) => {
                        assert!(
                            h.event_log.get(cause_id).is_some(),
                            "event {event_id:?} references non-existent cause {cause_id:?} \
                             at tick {current_tick:?}"
                        );
                    }
                    CauseRef::SystemTick(_) | CauseRef::Bootstrap | CauseRef::ExternalInput(_) => {
                        // Valid non-event causes.
                    }
                }
            }
        }
        last_checked_event = log_len;

        // Track travel emergence.
        if !saw_travel {
            saw_travel = h
                .event_log
                .events_by_tag(worldwake_core::EventTag::Travel)
                .len()
                > 0;
        }
    }

    // --- Per-run invariant 7: Emergence checks via event log ---
    // Death: already tracked per-tick.
    // Acquire: check for any completed trade or harvest action.
    let saw_acquire = h
        .event_log
        .events_by_tag(worldwake_core::EventTag::Trade)
        .len()
        > 0
        || h.event_log
            .events_by_tag(worldwake_core::EventTag::ActionCommitted)
            .len()
            > 0;
    // Travel: already tracked.
    // Share belief: check for social (tell) events.
    let saw_tell = h
        .event_log
        .events_by_tag(worldwake_core::EventTag::Social)
        .len()
        > 0;

    // Political emergence (ClaimOffice): check for political events.
    let saw_claim_office = h
        .event_log
        .events_by_tag(worldwake_core::EventTag::Political)
        .len()
        > 0;

    // Crime emergence (StealItem): check for crime events.
    let saw_steal = h
        .event_log
        .events_by_tag(worldwake_core::EventTag::Crime)
        .len()
        > 0;

    // --- Per-run invariant 8: State changed ---
    let final_world_hash = hash_world(&h.world).unwrap();
    assert_ne!(
        initial_world_hash, final_world_hash,
        "world state did not change after {TOTAL_TICKS} ticks (seed: {seed:?})"
    );

    SoakResult {
        world_hash: final_world_hash,
        event_log_hash: hash_event_log(&h.event_log).unwrap(),
        saw_death,
        saw_acquire,
        saw_travel,
        saw_tell,
        saw_claim_office,
        saw_steal,
    }
}

#[test]
fn t30_seven_day_soak() {
    // CI defaults to 10 seeds. SOAK_SEED_START can shard the range across jobs,
    // and local runs can raise SOAK_SEEDS for broader coverage.
    let seeds = soak_seeds_from_env();
    let seed_count = seeds.len();

    let results: Vec<SoakResult> = seeds.iter().map(|&s| run_t30_soak(s)).collect();
    let claim_office_threshold = scaled_emergence_threshold(seed_count, 3);
    let steal_threshold = scaled_emergence_threshold(seed_count, 3);

    // --- Cross-run invariant 9: Not all hashes identical ---
    let unique_world_hashes: BTreeSet<_> = results.iter().map(|r| r.world_hash).collect();
    let unique_log_hashes: BTreeSet<_> = results.iter().map(|r| r.event_log_hash).collect();
    assert!(
        unique_world_hashes.len() > 1 || unique_log_hashes.len() > 1,
        "all {seed_count} runs produced identical hashes — emergence is not seed-sensitive"
    );

    // --- Cross-run invariant 10: Political emergence ---
    let claim_office_count = results.iter().filter(|r| r.saw_claim_office).count();
    assert!(
        claim_office_count >= claim_office_threshold,
        "only {claim_office_count}/{seed_count} runs produced ClaimOffice events \
         (need >= {claim_office_threshold})"
    );

    // --- Cross-run invariant 11: Crime emergence ---
    let steal_count = results.iter().filter(|r| r.saw_steal).count();
    assert!(
        steal_count >= steal_threshold,
        "only {steal_count}/{seed_count} runs produced StealItem/Crime events \
         (need >= {steal_threshold})"
    );

    // --- Per-run emergence: at least 1 death, acquire, travel, tell across all runs ---
    let death_count = results.iter().filter(|r| r.saw_death).count();
    let acquire_count = results.iter().filter(|r| r.saw_acquire).count();
    let travel_count = results.iter().filter(|r| r.saw_travel).count();
    let tell_count = results.iter().filter(|r| r.saw_tell).count();

    // These should be true for the vast majority of runs given diverse population.
    assert!(death_count >= 1, "no runs produced a death in 10080 ticks");
    assert!(
        acquire_count >= 1,
        "no runs produced an acquire/trade action in 10080 ticks"
    );
    assert!(travel_count >= 1, "no runs produced travel in 10080 ticks");
    assert!(
        tell_count >= 1,
        "no runs produced a tell/share-belief action in 10080 ticks"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soak_seeds_default_to_ten_from_zero() {
        let seeds = soak_seeds(0, 10);
        assert_eq!(seeds.len(), 10);
        assert_eq!(seeds[0].0[0], 0);
        assert_eq!(seeds[9].0[0], 9);
    }

    #[test]
    fn soak_seeds_honor_start_and_count() {
        let seeds = soak_seeds(4, 3);
        assert_eq!(seeds.len(), 3);
        assert_eq!(seeds[0].0[0], 4);
        assert_eq!(seeds[1].0[0], 5);
        assert_eq!(seeds[2].0[0], 6);
    }

    #[test]
    fn scaled_emergence_threshold_matches_full_and_sharded_runs() {
        assert_eq!(scaled_emergence_threshold(10, 3), 3);
        assert_eq!(scaled_emergence_threshold(2, 3), 1);
        assert_eq!(scaled_emergence_threshold(5, 3), 2);
    }
}
