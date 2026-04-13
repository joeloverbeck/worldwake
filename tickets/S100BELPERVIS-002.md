# S100BELPERVIS-002: Tiered retention in enforce_capacity and enforce_entity_claim_capacity

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — belief retention logic in `AgentBeliefStore`
**Deps**: archive/tickets/S100BELPERVIS-001.md

## Problem

After `archive/tickets/S100BELPERVIS-001.md` adds `infrastructure_retention_ticks` to `PerceptionProfile`, the field remains unused by retention enforcement. Time-based decay in `enforce_capacity()` and `enforce_entity_claim_capacity()` still uses `memory_retention_ticks` uniformly for all beliefs. Infrastructure entities (places, facilities, resource sources, living agents) decay at the same rate as transient observations (item lots, ground items), causing agents to forget where resources exist after 48 ticks. This ticket makes both enforcement functions branch on the existing S77 tier classification to select the appropriate retention window.

## Assumption Reassessment (2026-04-13)

1. `enforce_capacity()` at `belief.rs:178` — the `known_entities.retain` block at lines 195-204 uses `profile.memory_retention_ticks` uniformly. `entity_eviction_tier()` at line 1972 returns `1` for infrastructure (Places, Facilities, alive Agents, resource sources) and `0` for transient. Confirmed via read.
2. `enforce_entity_claim_capacity()` at `belief.rs:226` — the `claims.retain` block at lines 250-256 uses `profile.memory_retention_ticks` uniformly. `claim_eviction_tier()` at line 1956 returns `0` for infrastructure claims (ResourceAvailable, WorkstationPresent, Location for Places/Facilities, Alive for Agents) and `1` for transient. `believed_kind` is already in scope from line 242 of the enclosing loop. Confirmed via read.
3. Existing focused tests exercising these functions (all in `belief.rs` `#[cfg(test)]`):
   - `enforce_capacity_removes_stale_entities_and_social_observations` (line 3232) — tests time-based eviction
   - `enforce_capacity_preserves_infrastructure_entities` (line 3285) — tests S77 capacity eviction tiers
   - `enforce_entity_claim_capacity_evicts_claims_beyond_retention_ticks` (line 3377) — tests time-based claim eviction
   - `enforce_entity_claim_capacity_preserves_infrastructure_claims` (line 3550) — tests S77 capacity eviction claim tiers

## Architecture Check

1. Reuses existing `entity_eviction_tier()` and `claim_eviction_tier()` functions from S77 — no new tier system, no new classification logic. The retention branching mirrors the capacity eviction branching, ensuring the same entities that survive capacity eviction also survive longer under time-based decay.
2. No backward-compatibility shims. The old uniform `memory_retention_ticks` path is replaced by tier-conditional selection. Setting `infrastructure_retention_ticks == memory_retention_ticks` produces identical behavior (graceful fallback, not a shim).

## Verification Layers

1. Infrastructure entity survives past `memory_retention_ticks` but before `infrastructure_retention_ticks` → focused unit test on `enforce_capacity`
2. Infrastructure claim survives past `memory_retention_ticks` but before `infrastructure_retention_ticks` → focused unit test on `enforce_entity_claim_capacity`
3. Both entity types decay past `infrastructure_retention_ticks` → focused unit test (no permanent memory)
4. Social observations unaffected → focused unit test
5. Equal retention parameters produce identical behavior → focused unit test (regression guard)
6. Single-layer ticket (belief store retention logic within worldwake-core). No cross-system concerns.

## What to Change

### 1. Modify `enforce_capacity()` known_entities.retain

In `crates/worldwake-core/src/belief.rs`, replace the `known_entities.retain` block (lines 195-204) to select retention window based on `entity_eviction_tier(state)`:

```rust
self.known_entities.retain(|entity, state| {
    if self.entity_claims.contains_key(entity) {
        return true;
    }
    let retention = if entity_eviction_tier(state) > 0 {
        profile.infrastructure_retention_ticks
    } else {
        profile.memory_retention_ticks
    };
    within_retention_window(state.observed_tick, current_tick, retention)
});
```

### 2. Modify `enforce_entity_claim_capacity()` claims.retain

In `crates/worldwake-core/src/belief.rs`, replace the `claims.retain` block (lines 250-256) to select retention window based on `claim_eviction_tier(claim.aspect, believed_kind)`:

```rust
claims.retain(|claim| {
    let retention = if claim_eviction_tier(claim.aspect, believed_kind) == 0 {
        profile.infrastructure_retention_ticks
    } else {
        profile.memory_retention_ticks
    };
    within_retention_window(claim.acquired_tick, current_tick, retention)
});
```

Note: `believed_kind` is already in scope from line 242. `claim_eviction_tier` returns 0 for infrastructure claims — the inverted tier semantics vs `entity_eviction_tier` (which returns 1 for infrastructure) match S77's existing design.

### 3. Add unit tests

Add 5 new tests to the `#[cfg(test)]` module in `belief.rs`:

1. `infrastructure_retention_entities_survive_longer` — Place entity at tick 0, ItemLot at tick 0, advance to tick 100 (between 48 and 480). Place survives, ItemLot evicted.
2. `infrastructure_retention_claims_survive_longer` — ResourceAvailable claim and Inventory claim on same entity, advance past 48 ticks. ResourceAvailable survives, Inventory evicted.
3. `infrastructure_retention_equal_parameters_no_regression` — Set both retention values to 48. Verify identical behavior to current system.
4. `infrastructure_retention_eventually_decays` — Advance past 480 ticks. Infrastructure beliefs ARE evicted.
5. `infrastructure_retention_social_observations_unaffected` — Social observations still use `memory_retention_ticks`.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — two retain blocks + new tests)

## Out of Scope

- Modifying capacity-based eviction logic (S77, already complete)
- Changing the tier classification functions (`entity_eviction_tier`, `claim_eviction_tier`)
- Updating scenario RON files (ticket 003)
- Adding new belief types, perception changes, or planner modifications
- Planner budget wall issues (separate concern)

## Acceptance Criteria

### Tests That Must Pass

1. `infrastructure_retention_entities_survive_longer` — Place survives at tick 100, ItemLot evicted
2. `infrastructure_retention_claims_survive_longer` — ResourceAvailable claim survives past 48 ticks, Inventory claim evicted
3. `infrastructure_retention_equal_parameters_no_regression` — identical behavior when both values equal
4. `infrastructure_retention_eventually_decays` — infrastructure beliefs evicted past 480 ticks
5. `infrastructure_retention_social_observations_unaffected` — social observations use `memory_retention_ticks`
6. All existing `enforce_capacity_*` and `enforce_entity_claim_capacity_*` tests pass
7. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Infrastructure entities (Places, Facilities, alive Agents, resource sources) use `infrastructure_retention_ticks` for time-based decay
2. Transient entities (ItemLots, dead Agents, unknown kinds) continue using `memory_retention_ticks`
3. Infrastructure claims (ResourceAvailable, WorkstationPresent, Location for Places/Facilities, Alive for Agents) use `infrastructure_retention_ticks`
4. Social observations always use `memory_retention_ticks` regardless of tier
5. All beliefs eventually decay — no permanent memory

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs::infrastructure_retention_entities_survive_longer` — verifies D2 tiered entity retention
2. `crates/worldwake-core/src/belief.rs::infrastructure_retention_claims_survive_longer` — verifies D3 tiered claim retention
3. `crates/worldwake-core/src/belief.rs::infrastructure_retention_equal_parameters_no_regression` — regression guard
4. `crates/worldwake-core/src/belief.rs::infrastructure_retention_eventually_decays` — guards against accidental permanent memory
5. `crates/worldwake-core/src/belief.rs::infrastructure_retention_social_observations_unaffected` — verifies D5 (social observations unchanged)

### Commands

1. `cargo test -p worldwake-core -- infrastructure_retention` — run new tests
2. `cargo test -p worldwake-core -- enforce_capacity` — run all enforce_capacity tests
3. `cargo test -p worldwake-core` — full core crate
4. `cargo test --workspace` — full workspace
5. `cargo clippy --workspace --all-targets -- -D warnings` — lint clean
