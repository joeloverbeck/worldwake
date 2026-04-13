# S101ACTBASBEL-002: BelievedEntityState ring buffer migration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — BelievedEntityState field replacement across all crates
**Deps**: None

## Problem

`BelievedEntityState` currently stores a single `observed_tick: Tick` field. The activation-based decay system needs a ring buffer of presentation ticks to compute frequency-weighted activation. This ticket migrates the data structure and all ~30 construction sites across 4 crates. The existing `enforce_capacity` function is temporarily adapted to use the `last_observed_tick()` accessor until ticket 003 replaces it entirely.

## Assumption Reassessment (2026-04-13)

1. `BelievedEntityState` at `crates/worldwake-core/src/belief.rs:1322-1341` has `observed_tick: Tick` at line 1339. Grep confirms ~30 struct literal construction sites across worldwake-core, worldwake-ai, worldwake-systems, worldwake-cli.
2. `enforce_capacity` at `crates/worldwake-core/src/belief.rs:178` reads `state.observed_tick` at lines 204 and 218. After migration, these must use `state.last_observed_tick().unwrap_or(Tick(0))` to keep compiling until ticket 003 removes enforce_capacity.
3. `record_entity_snapshot_claims` at `crates/worldwake-core/src/belief.rs:81` does not directly mutate `observed_tick`; instead it records claims and re-derives a `BelievedEntityState` summary via `derive_entity_summary()`. The ring-buffer push therefore has to happen on the stored summary after refresh, while `derive_entity_summary()` still needs a lawful single-tick summary for the temporary ticket-002 boundary.
4. The canonical projection builders are `ObservedEntitySnapshot::to_believed_entity_state()` at `crates/worldwake-core/src/belief.rs:1245` and `build_believed_entity_state()` at `crates/worldwake-core/src/belief.rs:1783`. These are the right shared boundaries for a single-observation constructor helper.
5. The ticket's original file list under-claimed the fallout surface. Live `BelievedEntityState` literals also exist in `crates/worldwake-core/src/communication.rs`, `crates/worldwake-sim/src/save_load.rs`, `crates/worldwake-sim/src/per_agent_belief_view.rs`, `crates/worldwake-sim/src/social_relay.rs`, `crates/worldwake-ai/src/search/tests.rs`, `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-ai/src/ranking.rs`, `crates/worldwake-ai/src/plan_revalidation.rs`, `crates/worldwake-ai/src/search/strategic.rs`, and golden test files. These are lawful current-ticket constructor fallout because they instantiate the migrated shared type directly.
6. `tickets/S101ACTBASBEL-002.md` is tracked in git on this branch. `archive/tickets/S101ACTBASBEL-001.md` is now the completed prior slice.
7. `BelievedEntityState` is part of the persisted simulation shape: `crates/worldwake-sim/src/save_load.rs` serializes/deserializes it inside `SimulationState`, and `SAVE_FORMAT_VERSION` is currently `29`. Because Worldwake does not keep backward-compatible save loaders by default, this ticket must update the current save format version alongside the struct migration so older saves fail fast at the version gate instead of deep-deserializing with the wrong shape.

## Architecture Check

1. Ring buffer as `[Tick; 8]` + `u8` count avoids adding SmallVec dependency. Max buffer capacity is 8 (per PerceptionProfile.observation_buffer_capacity). Fixed array is simpler and keeps dependency graph lean.
2. The `last_observed_tick()` accessor provides backward compatibility for code that only needs the most recent observation tick, avoiding a mass API change in code that doesn't need the full buffer.
3. No backward-compatibility shims — `observed_tick` is removed, not aliased (FND-28).

## Verification Layers

1. Ring buffer FIFO semantics → focused unit test (push beyond capacity, verify oldest evicted)
2. `last_observed_tick()` returns most recent → focused unit test
3. All construction sites compile → `cargo build --workspace`
4. `enforce_capacity` still functions → existing belief.rs tests pass
5. Single-layer ticket (worldwake-core data structure) — cross-crate impact is mechanical (construction site updates).

## What to Change

### 1. Modify BelievedEntityState struct

In `crates/worldwake-core/src/belief.rs`, replace `observed_tick: Tick` with:

```rust
pub presentation_ticks: [Tick; 8],
pub presentation_tick_count: u8,
```

Add derived accessor:

```rust
impl BelievedEntityState {
    pub fn last_observed_tick(&self) -> Option<Tick> {
        if self.presentation_tick_count == 0 {
            None
        } else {
            Some(self.presentation_ticks[(self.presentation_tick_count - 1) as usize])
        }
    }
}
```

Add ring buffer push helper (private or pub(crate)):

```rust
pub(crate) fn push_presentation_tick(&mut self, tick: Tick, buffer_capacity: u8) {
    let cap = buffer_capacity.min(8) as usize;
    if cap == 0 { return; }
    if (self.presentation_tick_count as usize) < cap {
        self.presentation_ticks[self.presentation_tick_count as usize] = tick;
        self.presentation_tick_count += 1;
    } else {
        // Shift left (evict oldest), append new
        for i in 1..cap {
            self.presentation_ticks[i - 1] = self.presentation_ticks[i];
        }
        self.presentation_ticks[cap - 1] = tick;
    }
}
```

### 2. Update record_entity_snapshot_claims

In the same file, update `record_entity_snapshot_claims` to push `current_tick` onto the stored summary after `refresh_entity_summary_from_claims()`. The `buffer_capacity` parameter comes from the caller in the final design (`PerceptionProfile.observation_buffer_capacity`), but ticket 003 has not landed yet, so ticket 002 should use the temporary constant `5u8` at this boundary and let ticket 003 replace that constant with the profile field.

### 3. Temporarily adapt enforce_capacity

Replace `state.observed_tick` reads in `enforce_capacity` (lines 204, 218) with `state.last_observed_tick().unwrap_or(Tick(0))`.

### 4. Update all construction sites

Every `BelievedEntityState { ... observed_tick: <expr>, ... }` becomes `BelievedEntityState { ... presentation_ticks: { let mut ticks = [Tick(0); 8]; ticks[0] = <expr>; ticks }, presentation_tick_count: 1, ... }`.

Consider adding a constructor helper `BelievedEntityState::with_single_observation(tick: Tick, ...)` to reduce boilerplate at the ~30 construction sites.

Files with construction sites (from grep):
- `crates/worldwake-core/src/world.rs:713`
- `crates/worldwake-core/src/delta.rs:460`
- `crates/worldwake-core/src/component_tables.rs:214`
- `crates/worldwake-core/src/communication.rs`
- `crates/worldwake-core/src/world_txn.rs:4630`
- `crates/worldwake-sim/src/save_load.rs`
- `crates/worldwake-sim/src/per_agent_belief_view.rs`
- `crates/worldwake-sim/src/social_relay.rs`
- `crates/worldwake-ai/src/route_threat.rs` (3 sites)
- `crates/worldwake-ai/src/exhaustion.rs` (4 sites)
- `crates/worldwake-ai/src/planning_state.rs` (5 sites)
- `crates/worldwake-ai/src/planning_snapshot.rs` (2 sites)
- `crates/worldwake-ai/src/pursuit_belief.rs` (2 sites)
- `crates/worldwake-ai/src/goal_model.rs` (1 site)
- `crates/worldwake-ai/src/search/tests.rs`
- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-ai/src/ranking.rs`
- `crates/worldwake-ai/src/plan_revalidation.rs`
- `crates/worldwake-ai/src/search/strategic.rs`
- `crates/worldwake-ai/tests/golden_*.rs`
- `crates/worldwake-cli/src/bin/observer.rs` (1 site)
- `crates/worldwake-systems/src/artifact_actions.rs` (1 site)
- `crates/worldwake-systems/src/perception.rs`
- `crates/worldwake-systems/src/tell_actions.rs`
- `crates/worldwake-systems/src/office_actions.rs`
- `crates/worldwake-systems/src/justice_actions.rs`
- `crates/worldwake-systems/src/investigate_actions.rs`

### 5. Unit tests

- `test_ring_buffer_evicts_oldest_on_overflow` — push 6 ticks into capacity-5 buffer, verify oldest evicted
- `test_last_observed_tick_accessor` — returns most recent tick, returns None when empty
- `test_push_presentation_tick_respects_capacity` — capacity 3 buffer stays at 3 entries
- Existing save/load version tests in `crates/worldwake-sim/src/save_load.rs` — updated current-format version continues to round-trip and reject old versions at the header gate

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify) — struct change, accessor, push helper, enforce_capacity adaptation, tests
- `crates/worldwake-core/src/world.rs` (modify) — construction site
- `crates/worldwake-core/src/delta.rs` (modify) — construction site
- `crates/worldwake-core/src/component_tables.rs` (modify) — construction site
- `crates/worldwake-core/src/communication.rs` (modify) — construction sites
- `crates/worldwake-core/src/world_txn.rs` (modify) — construction site
- `crates/worldwake-sim/src/save_load.rs` (modify) — construction sites + `SAVE_FORMAT_VERSION`
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify) — construction sites
- `crates/worldwake-sim/src/social_relay.rs` (modify) — construction sites
- `crates/worldwake-ai/src/route_threat.rs` (modify) — construction sites
- `crates/worldwake-ai/src/exhaustion.rs` (modify) — construction sites
- `crates/worldwake-ai/src/planning_state.rs` (modify) — construction sites
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify) — construction sites
- `crates/worldwake-ai/src/pursuit_belief.rs` (modify) — construction sites
- `crates/worldwake-ai/src/goal_model.rs` (modify) — construction site
- `crates/worldwake-ai/src/search/tests.rs` (modify) — construction sites
- `crates/worldwake-ai/src/candidate_generation.rs` (modify) — construction sites
- `crates/worldwake-ai/src/ranking.rs` (modify) — construction sites
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify) — construction sites
- `crates/worldwake-ai/src/search/strategic.rs` (modify) — construction sites
- `crates/worldwake-ai/tests/golden_expectation.rs` (modify) — construction sites
- `crates/worldwake-ai/tests/golden_integration.rs` (modify) — construction sites
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify) — construction sites
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify) — construction sites
- `crates/worldwake-cli/src/bin/observer.rs` (modify) — construction site
- `crates/worldwake-systems/src/artifact_actions.rs` (modify) — construction site
- `crates/worldwake-systems/src/epistemic_actions.rs` (modify) — observation freshness assertions and helper fallout
- `crates/worldwake-systems/src/perception.rs` (modify) — construction sites
- `crates/worldwake-systems/src/tell_actions.rs` (modify) — construction sites
- `crates/worldwake-systems/src/office_actions.rs` (modify) — construction sites
- `crates/worldwake-systems/src/justice_actions.rs` (modify) — construction sites
- `crates/worldwake-systems/src/investigate_actions.rs` (modify) — construction sites
- `crates/worldwake-systems/tests/e15_information_integration.rs` (modify) — last-observed accessor assertion fallout

## Out of Scope

- PerceptionProfile field changes (ticket 003)
- Replacing enforce_capacity with prune_decayed_beliefs (ticket 003)
- Call site updates in perception.rs/epistemic_actions.rs/tell_actions.rs (ticket 003 — these call enforce_capacity, not construct BelievedEntityState)
- Golden tests (ticket 004)
- Scenario RON file migration (ticket 003 — scenarios specify PerceptionProfile fields, not BelievedEntityState fields)

## Acceptance Criteria

### Tests That Must Pass

1. `test_ring_buffer_evicts_oldest_on_overflow` — FIFO eviction correct
2. `test_last_observed_tick_accessor` — returns most recent, None when empty
3. `test_push_presentation_tick_respects_capacity` — capacity limit honored
4. Existing suite: `cargo test --workspace` — all existing tests pass after construction site migration
5. `cargo clippy --workspace --all-targets -- -D warnings` — no new warnings
6. Existing save/load header contract remains honest for the new shape — current saves round-trip under the bumped version, older versions reject at the version gate

### Invariants

1. `presentation_tick_count` never exceeds 8 (array bound)
2. `last_observed_tick()` always returns the most recently pushed tick
3. Ring buffer is FIFO — oldest entry evicted first when at capacity
4. All existing behavior preserved — enforce_capacity produces same results via last_observed_tick() accessor

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — `test_ring_buffer_evicts_oldest_on_overflow`, `test_last_observed_tick_accessor`, `test_push_presentation_tick_respects_capacity`
2. `crates/worldwake-sim/src/save_load.rs` — existing save/load version tests updated for the new current format version

### Commands

1. `cargo test -p worldwake-core -- test_ring_buffer`
2. `cargo test -p worldwake-core -- test_last_observed_tick`
3. `cargo test -p worldwake-core -- test_push_presentation_tick`
4. `cargo test --workspace --no-run`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test --workspace`

## Outcome

Completed on 2026-04-13.

- Replaced `BelievedEntityState.observed_tick` with a concrete presentation-history buffer: `presentation_ticks: [Tick; 8]` plus `presentation_tick_count: u8`.
- Added `BelievedEntityState::single_observation_defaults()`, `last_observed_tick()`, and `push_presentation_tick()` so single-observation callers stay concise while shared logic derives freshness from retained subjective observation history.
- Updated `record_entity_snapshot_claims()` to push the current presentation tick after refreshing the stored summary, and temporarily adapted `enforce_capacity()` to rank recency through `last_observed_tick().unwrap_or(Tick(0))` until ticket 003 replaces that path.
- Migrated all direct `BelievedEntityState` construction and freshness assertions across `worldwake-core`, `worldwake-sim`, `worldwake-systems`, `worldwake-ai`, CLI observer output, and AI golden helpers/tests to the new buffer-backed shape.
- Bumped `SAVE_FORMAT_VERSION` to `30` so persisted state fails fast at the version gate instead of trying to deserialize the old scalar belief shape into the new retained-history representation.

## Deviations

- The original ticket described the cross-crate fallout as mostly mechanical constructor migration across four crates, but the honest live scope also included `worldwake-systems/src/epistemic_actions.rs`, `worldwake-systems/tests/e15_information_integration.rs`, and several AI golden helpers because they asserted or manipulated freshness directly. Those files were absorbed as lawful current-ticket fallout.
- The temporary presentation-buffer push inside `record_entity_snapshot_claims()` uses `BelievedEntityState::MAX_PRESENTATION_TICKS` at this boundary. Ticket 003 still owns replacing that temporary boundary choice with profile-driven pruning/retention behavior.

## Verification Result

- Passed `cargo test -p worldwake-core --lib belief`
- Passed `cargo test -p worldwake-core --lib homeostatic_needs_max_value`
- Passed `cargo test --workspace --no-run`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
