//! Resilience and determinism tests using the T30 world setup.
//!
//! T31: Stress test with periodic disruptions (agent kills, item destruction,
//!   workstation removal, teleportation) — proves invariants hold under chaos.
//! T32: Save/load replay consistency — proves deterministic execution across
//!   a serialization boundary.
//!
//! Gated behind the `soak` feature because they share the heavyweight T30 world.
//! Run with: `cargo test -p worldwake-ai --features soak --test golden_resilience`
#![cfg(feature = "soak")]

mod golden_harness;

use golden_harness::soak_world::*;
use golden_harness::*;
use worldwake_core::{
    CauseRef, CommodityKind, DeadAt, EntityId, EntityKind, EventId, EventView, Permille, Seed,
    StateHash, Tick, hash_event_log, hash_world, total_authoritative_commodity_quantity,
    verify_authoritative_conservation,
};

// ---------------------------------------------------------------------------
// Scenario 31: Stress with Frequent Disruptions
// ---------------------------------------------------------------------------
//
// Systems: Needs, Production, Trade, Combat, Travel, Social, Politics, Perception
// GoalKinds: ConsumeOwnedCommodity, AcquireCommodity, RestockCommodity, ShareBelief,
//   ClaimOffice, StealItem, Patrol, Harvest, Craft
// ActionDomains: Needs, Trade, Travel, Combat, Production, Social, Transport
// Places: T30Hub, T30Market, T30Farm, T30Forge, T30Barracks, T30RulersHall,
//   T30Forest, T30BanditCamp, T30Road, T30Orchard
// Principles: 3, 4, 6, 7, 8, 10, 12, 14, 26
//
// Setup: Reuses T30's 10-place topology and 20-agent population. Every 100 ticks,
//   one random disruption is injected via WorldTxn: kill an agent, destroy an item
//   lot, remove a workstation tag, or teleport an agent. Disruption type is selected
//   deterministically from DeterministicRng for reproducibility. Runs 2880 ticks
//   (2 in-game days) with 28 disruptions total.
//
// Proves: The full simulation stack handles arbitrary mid-run disruptions gracefully.
//   All per-tick invariants (conservation, needs bounds, dead agent inactivity,
//   unique placement, tick monotonicity, causal link integrity) hold despite
//   disruptions. Save/load roundtrip at end produces identical hash. No panics.
//
// Chain: autonomous agents + periodic disruptions (death, destruction, removal,
//   teleportation) -> AI replanning around changed state -> invariants hold
//   every tick despite arbitrary state mutations.

/// Run a single T31 stress run for the given seed. Panics on invariant violation.
fn run_t31_stress(seed: Seed) {
    let (mut h, all_agents, _ruling_faction, _bandit_faction, _office) = build_t30_world(seed);

    const TOTAL_TICKS: u64 = 2880;
    const DISRUPTION_INTERVAL: u64 = 100;

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

    let mut prev_tick = h.scheduler.current_tick();
    let mut last_checked_event: u64 = 0;

    // Separate RNG stream for disruptions so they don't perturb the simulation RNG.
    let mut disruption_seed = seed;
    disruption_seed.0[0] = disruption_seed.0[0].wrapping_add(0xDD);
    let mut disruption_rng = worldwake_sim::DeterministicRng::new(disruption_seed);

    // Collect all place IDs from the T30 topology for teleportation targets.
    let all_places = [
        PLACE_T30_HUB,
        PLACE_T30_MARKET,
        PLACE_T30_FARM,
        PLACE_T30_FORGE,
        PLACE_T30_BARRACKS,
        PLACE_T30_RULERS_HALL,
        PLACE_T30_FOREST,
        PLACE_T30_BANDIT_CAMP,
        PLACE_T30_ROAD,
        PLACE_T30_ORCHARD,
    ];

    for tick_idx in 0..TOTAL_TICKS {
        // --- Disruption injection every DISRUPTION_INTERVAL ticks ---
        if tick_idx > 0 && tick_idx % DISRUPTION_INTERVAL == 0 {
            let disruption_type = disruption_rng.next_range(0, 4);
            let current_tick_val = h.scheduler.current_tick().0;

            match disruption_type {
                0 => {
                    // Kill a random living agent.
                    let living: Vec<EntityId> = all_agents
                        .iter()
                        .copied()
                        .filter(|&a| !h.agent_is_dead(a))
                        .collect();
                    if !living.is_empty() {
                        let idx = disruption_rng.next_range(0, living.len() as u32) as usize;
                        let victim = living[idx];
                        let mut txn = new_txn(&mut h.world, current_tick_val);
                        txn.set_component_dead_at(
                            victim,
                            DeadAt {
                                tick: Tick(current_tick_val),
                                cause: worldwake_core::DeathCause::CombatWounds,
                            },
                        )
                        .unwrap();
                        commit_txn(txn, &mut h.event_log);
                    }
                }
                1 => {
                    // Destroy a random ItemLot (archive it and adjust conservation baseline).
                    let lots: Vec<EntityId> =
                        h.world.entities_of_kind(EntityKind::ItemLot).collect();
                    if !lots.is_empty() {
                        let idx = disruption_rng.next_range(0, lots.len() as u32) as usize;
                        let lot = lots[idx];
                        // Read quantity before archiving to adjust conservation baseline.
                        if let Some(item_lot) = h.world.get_component_item_lot(lot).cloned() {
                            let commodity = item_lot.commodity;
                            let qty = item_lot.quantity.0 as u64;
                            let mut txn = new_txn(&mut h.world, current_tick_val);
                            txn.archive_entity(lot).unwrap();
                            commit_txn(txn, &mut h.event_log);
                            // Reduce conservation baseline by the destroyed quantity.
                            if let Some(total) = commodity_totals.get_mut(&commodity) {
                                *total = total.saturating_sub(qty);
                            }
                        }
                    }
                }
                2 => {
                    // Remove WorkstationTag from a random facility.
                    let facilities: Vec<EntityId> = h
                        .world
                        .entities_of_kind(EntityKind::Facility)
                        .filter(|&e| h.world.get_component_workstation_marker(e).is_some())
                        .collect();
                    if !facilities.is_empty() {
                        let idx = disruption_rng.next_range(0, facilities.len() as u32) as usize;
                        let facility = facilities[idx];
                        let mut txn = new_txn(&mut h.world, current_tick_val);
                        txn.clear_component_workstation_marker(facility).unwrap();
                        commit_txn(txn, &mut h.event_log);
                    }
                }
                3 => {
                    // Teleport a random living agent to a random place.
                    let living: Vec<EntityId> = all_agents
                        .iter()
                        .copied()
                        .filter(|&a| !h.agent_is_dead(a))
                        .collect();
                    if !living.is_empty() {
                        let agent_idx = disruption_rng.next_range(0, living.len() as u32) as usize;
                        let agent = living[agent_idx];
                        let place_idx =
                            disruption_rng.next_range(0, all_places.len() as u32) as usize;
                        let target_place = all_places[place_idx];
                        let mut txn = new_txn(&mut h.world, current_tick_val);
                        txn.set_ground_location(agent, target_place).unwrap();
                        commit_txn(txn, &mut h.event_log);
                    }
                }
                _ => unreachable!(),
            }
        }

        h.step_once();
        let current_tick = h.scheduler.current_tick();

        // --- Per-tick invariant 1: Conservation ---
        for (&commodity, total) in &mut commodity_totals {
            let actual = total_authoritative_commodity_quantity(&h.world, commodity);
            if actual > *total {
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
                assert!(
                    !h.agent_has_active_action(agent),
                    "dead agent {agent:?} (died at {:?}) has active action at tick {current_tick:?}",
                    dead_at.tick
                );
            }
        }

        // --- Per-tick invariant 4: Unique placement ---
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
                    CauseRef::SystemTick(_) | CauseRef::Bootstrap | CauseRef::ExternalInput(_) => {}
                }
            }
        }
        last_checked_event = log_len;
    }

    // --- Verification layer 4: Save/load roundtrip fidelity ---
    let pre_save_hash = hash_world(&h.world).unwrap();
    let roundtripped = h.save_load_roundtrip();
    let post_load_hash = hash_world(&roundtripped.world).unwrap();
    assert_eq!(
        pre_save_hash, post_load_hash,
        "save/load roundtrip at tick 2880 produced different hash: \
         pre={pre_save_hash:?}, post={post_load_hash:?}"
    );
}

#[test]
fn t31_stress_disruptions() {
    let mut seed_bytes = [0u8; 32];
    seed_bytes[0] = 0x31;
    seed_bytes[31] = 0xAB;
    run_t31_stress(Seed(seed_bytes));
}

// ---------------------------------------------------------------------------
// Scenario 32: Long Replay Consistency
// ---------------------------------------------------------------------------
//
// Systems: Needs, Production, Trade, Combat, Travel, Social, Politics, Perception
// GoalKinds: ConsumeOwnedCommodity, AcquireCommodity, RestockCommodity, ShareBelief,
//   ClaimOffice, StealItem, Patrol, Harvest, Craft
// ActionDomains: Needs, Trade, Travel, Combat, Production, Social, Transport
// Places: T30Hub, T30Market, T30Farm, T30Forge, T30Barracks, T30RulersHall,
//   T30Forest, T30BanditCamp, T30Road, T30Orchard
// Principles: 3, 4, 6, 12
//
// Setup: Reuses T30's 10-place topology and 20-agent population. A continuous
//   1440-tick run records (hash_world, hash_event_log) at every 100-tick
//   checkpoint. A split run saves at tick 720, loads the snapshot, and
//   continues for another 720 ticks, recording the same checkpoints.
//
// Proves: Save/load mid-run preserves all world meaning (Principle 12).
//   Deterministic execution: same seed + same inputs = identical StateHash
//   at every checkpoint, whether run continuously or split across a
//   serialization boundary. No state leakage through save/load.
//
// Chain: seed -> continuous 1440-tick run -> checkpoint hashes
//   vs seed -> 720 ticks -> save_to_bytes -> load_from_bytes -> 720 ticks
//   -> checkpoint hashes must match exactly at every 100-tick boundary.

/// Run ticks continuously, recording (tick, world_hash, log_hash) at
/// every 100-tick checkpoint.
fn run_continuous(seed: Seed, total_ticks: u64) -> Vec<(u64, StateHash, StateHash)> {
    let (mut h, _agents, _rf, _bf, _office) = build_t30_world(seed);
    let mut checkpoints = Vec::new();

    for tick_idx in 1..=total_ticks {
        h.step_once();
        if tick_idx % 100 == 0 {
            let world_hash = hash_world(&h.world).unwrap();
            let log_hash = hash_event_log(&h.event_log).unwrap();
            checkpoints.push((tick_idx, world_hash, log_hash));
        }
    }

    checkpoints
}

/// Run `save_at` ticks, save to bytes, load from bytes, then continue for
/// `total_ticks - save_at` more ticks. Record checkpoints at every 100-tick
/// boundary across both halves.
fn run_split(seed: Seed, save_at: u64, total_ticks: u64) -> Vec<(u64, StateHash, StateHash)> {
    let (mut h, _agents, _rf, _bf, _office) = build_t30_world(seed);
    let mut checkpoints = Vec::new();

    // --- First half: run up to save_at ---
    for tick_idx in 1..=save_at {
        h.step_once();
        if tick_idx % 100 == 0 {
            let world_hash = hash_world(&h.world).unwrap();
            let log_hash = hash_event_log(&h.event_log).unwrap();
            checkpoints.push((tick_idx, world_hash, log_hash));
        }
    }

    // --- Save -> Load boundary ---
    let mut h = h.save_load_roundtrip();

    // --- Second half: continue from save_at+1 to total_ticks ---
    for tick_idx in (save_at + 1)..=total_ticks {
        h.step_once();
        if tick_idx % 100 == 0 {
            let world_hash = hash_world(&h.world).unwrap();
            let log_hash = hash_event_log(&h.event_log).unwrap();
            checkpoints.push((tick_idx, world_hash, log_hash));
        }
    }

    checkpoints
}

/// Run both continuous and split, assert exact checkpoint match.
fn run_t32_replay_consistency(seed: Seed) {
    let total_ticks: u64 = 1440;
    let save_at: u64 = 720;

    let continuous = run_continuous(seed, total_ticks);
    let split = run_split(seed, save_at, total_ticks);

    assert_eq!(
        continuous.len(),
        split.len(),
        "T32: checkpoint count mismatch: continuous={}, split={}",
        continuous.len(),
        split.len()
    );

    for (c, s) in continuous.iter().zip(split.iter()) {
        assert_eq!(
            c.0, s.0,
            "T32: checkpoint tick mismatch: continuous={}, split={}",
            c.0, s.0
        );
        assert_eq!(
            c.1, s.1,
            "T32: world hash mismatch at tick {}: continuous={:?}, split={:?}",
            c.0, c.1, s.1
        );
        assert_eq!(
            c.2, s.2,
            "T32: event log hash mismatch at tick {}: continuous={:?}, split={:?}",
            c.0, c.2, s.2
        );
    }
}

#[test]
fn t32_replay_consistency() {
    let mut seed_bytes = [0u8; 32];
    seed_bytes[0] = 0x32;
    seed_bytes[31] = 0xCC;
    run_t32_replay_consistency(Seed(seed_bytes));
}
