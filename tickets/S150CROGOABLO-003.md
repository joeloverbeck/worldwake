# S150CROGOABLO-003: Scope-aware BlockerClearingCondition variants

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `BlockerClearingCondition` enum in `worldwake-core`; recording-path clearing logic in `worldwake-ai` and `worldwake-systems`
**Deps**: archive/tickets/S150CROGOABLO-002.md

## Problem

After ticket 002, `BlockerMemory` carries `RouteSegment` and `Counterparty` scoped blockers but the new entries all use `BlockerClearingCondition::TtlOnly` — the only physical dampener is TTL expiry. FND-11 (every positive-feedback loop needs a physical dampener) and FND-22A (learning must have explicit clearing) want an additional clearing path: an agent that safely traverses a previously-blocked segment, or successfully interacts with a previously-blocked counterparty, should be able to clear the blocker through the witnessed contradicting observation rather than waiting out the full TTL. This ticket adds the two new variants and wires the recording-path logic that switches from `TtlOnly` to the typed clearing variants when the blocker's safe-witnessing observation predicate can be expressed concretely.

## Assumption Reassessment (2026-05-17)

1. `BlockerClearingCondition` lives at `crates/worldwake-core/src/blocker_memory.rs:131-155` with 8 existing variants (7 fact-specific clearing predicates + `TtlOnly`). The enum derives `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. Both new variant payloads (`RouteSegment`, `EntityId`) satisfy `Copy`, preserving the derive bounds.
2. Spec source: `specs/S150-cross-goal-blocker-scoping.md` D6 (scope-aware variants) and Concrete Dampeners section.
3. Shared abstraction boundary: `BlockerClearingCondition` is the contract between (a) the recording sites that author the predicate when a blocker is created and (b) the `sweep_cleared` consumer at `blocker_memory.rs:84-86` that applies the predicate against subsequent observations. Both sides must understand the new variants.
4. AI regression layer: the recording-path edits modify `crates/worldwake-ai/src/agent_tick/observation.rs` (route safe-witnessing) and `crates/worldwake-ai/src/agent_tick/execution.rs` (counterparty acceptance), plus `crates/worldwake-systems/src/trade_actions.rs` (trade-commit clearing). All three are exercised by inline `#[cfg(test)]` tests in their host modules and by goldens in `crates/worldwake-ai/tests/`. The harness boundary is focused-unit for the variant additions and recording-condition selection; cross-tick clearing behavior is covered by golden ticket 006.
5. Existing tests in target modules: `crates/worldwake-core/src/blocker_memory.rs::sweep_cleared_removes_matching_entries`, `sweep_cleared_retains_non_matching_entries`, `blocker_memory_roundtrips_through_bincode` — all need to handle the new variants in their fixture or extend coverage. Recording-site test coverage in `crates/worldwake-ai/src/agent_tick/tests.rs` does not currently test clearing-condition selection; new focused tests asserting "after `record(Blocker { clearing_condition: RouteRetraversedSafely(segment), ... })`, subsequent safe-traversal observation triggers sweep" are added.
6. Adjacent contradictions: ticket 002 already added new helpers (`route_segment_blocked`, `counterparty_blocked`) that read from `BlockerMemory`; the clearing path in this ticket is the symmetric write. No adjacent contradictions; this is a required consequence of 002's recording-paths-use-TtlOnly default.

## Architecture Check

1. **FND-11 concrete dampener**: Without scope-aware clearing variants, the only physical mechanism that removes a `RouteSegment` or `Counterparty` blocker is TTL expiry — a clock-driven dampener with no information-content. The new variants tie clearing to a concrete witnessed observation, satisfying FND-11's "physical dampener" requirement at the per-blocker level.
2. **FND-22A explicit invalidating observation**: The new variants make the clearing predicate inspectable in stored state ("this blocker clears when route AB is retraversed safely"), parallel to how the existing fact-specific variants (`PathDiscovered { destination }`, `DangerReduced { place }`) tie clearing to concrete world predicates.
3. **No new substrate**: The clearing-condition machinery (`sweep_cleared` + per-tick observation-matching predicates) already exists. This ticket only extends the variant set; the dispatch logic at the consumer sites (observation-matching code that fires `sweep_cleared` with a closure that matches `RouteRetraversedSafely(segment)` against the just-observed safe traversal) is the only behavioral addition.

## Verification Layers

1. New variant trait-bound regression — focused unit test (extending `blocker_clearing_condition_and_baseline_satisfy_required_bounds` at `blocker_memory.rs:259`) proving both new variants satisfy `Copy, Eq, Serialize, Deserialize`.
2. New variant serialization roundtrip — focused unit test extending `blocker_memory_roundtrips_through_bincode` (line 484) to include a blocker with `RouteRetraversedSafely(...)` and one with `CounterpartyAccepted(...)`.
3. Sweep predicate behavior — focused unit test in `blocker_memory.rs` `#[cfg(test)]`: insert a Blocker with `clearing_condition: RouteRetraversedSafely(segment_AB)`, call `sweep_cleared(|b| matches!(b.clearing_condition, BlockerClearingCondition::RouteRetraversedSafely(seg) if seg == segment_AB))`, assert the blocker is removed.
4. Recording-site selection — focused unit test in `agent_tick/observation.rs` `#[cfg(test)]`: simulate the observation path for a witnessed dangerous traversal, assert the recorded `Blocker.clearing_condition == RouteRetraversedSafely(...)` (not `TtlOnly`).
5. Cross-tick clearing in golden coverage — deferred to ticket 006's `golden_cross_goal_blocker_scoping.rs` (the `RouteRetraversedSafely` and `CounterpartyAccepted` clearing-fires scenarios are explicit in D10).

## What to Change

### 1. Add two new variants to `BlockerClearingCondition`

In `crates/worldwake-core/src/blocker_memory.rs:131-155`:

```rust
pub enum BlockerClearingCondition {
    // ... existing 7 fact-specific variants + TtlOnly preserved unchanged
    RouteRetraversedSafely(RouteSegment),
    CounterpartyAccepted(EntityId),
}
```

### 2. Update recording-path clearing-condition selection

In `crates/worldwake-ai/src/agent_tick/observation.rs:626` (the perception-driven recording site from ticket 002): when the recorded blocker is `BlockerScope::RouteSegment(segment)` AND the blocking fact is danger-related (`DangerTooHigh`, `CombatTooRisky`), set `clearing_condition = BlockerClearingCondition::RouteRetraversedSafely(segment)` instead of `TtlOnly`. When the scope is `BlockerScope::Counterparty(other)` AND the blocking fact is interaction-related (`PatienceExhausted`, `NoBuyer`), set `clearing_condition = BlockerClearingCondition::CounterpartyAccepted(other)`.

In `crates/worldwake-ai/src/agent_tick/execution.rs:1341` and `crates/worldwake-ai/src/failure_handling.rs:224` (the action-execution and plan-failure recording sites from ticket 002): apply the same selection logic — scope-typed recording paths use the scope-typed clearing variants by default; the goal-keyed `BlockerScope::Exact(...)` recording paths continue to use existing fact-specific clearing variants (e.g., `CommodityAvailabilityChanged`).

In `crates/worldwake-systems/src/trade_actions.rs:1920-1930` (the `NoBuyer` recording from ticket 002): use `CounterpartyAccepted(counterparty_id)` as the clearing condition when the recorded scope is `BlockerScope::Counterparty(...)`.

### 3. Add sweep-trigger logic at the observation sites

Wherever the AI layer processes an observation that satisfies a typed clearing predicate, call `blocker_memory.sweep_cleared(|b| /* matches the new variant against this observation */)` to clear the relevant blocker(s). The natural placement:

- Safe-traversal observation (`TravelTo` action commit, no danger-event interruption): sweep blockers with `RouteRetraversedSafely(seg)` for the segment that was just traversed.
- Successful counterparty interaction (trade commit, accepted Tell): sweep blockers with `CounterpartyAccepted(other)` for the counterparty just interacted with.

These hook into existing post-action observation pipelines; no new SystemFn is introduced.

### 4. New focused tests in `blocker_memory.rs`

- Extend `blocker_clearing_condition_and_baseline_satisfy_required_bounds` to cover both new variants.
- Extend `blocker_memory_roundtrips_through_bincode` to roundtrip a blocker with each new variant.
- New test `sweep_cleared_removes_route_retraversed_safely_blockers`: insert blocker with `RouteRetraversedSafely(seg_AB)`, call sweep predicate, assert removal.
- New test `sweep_cleared_removes_counterparty_accepted_blockers`: same pattern for `CounterpartyAccepted`.

## Files to Touch

- `crates/worldwake-core/src/blocker_memory.rs` (modify) — enum additions + new tests
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify) — clearing-condition selection at recording + sweep-trigger on safe-traversal observation
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify) — clearing-condition selection at recording
- `crates/worldwake-ai/src/failure_handling.rs` (modify) — clearing-condition selection at recording
- `crates/worldwake-systems/src/trade_actions.rs` (modify) — `CounterpartyAccepted` clearing-condition at NoBuyer recording + sweep-trigger on successful trade commit
- Likely: `crates/worldwake-ai/src/agent_tick/tests.rs` (modify) — add focused tests for clearing-condition selection at recording sites (path confirmed during 002 implementation; tests live alongside the recording-site host modules they exercise)

## Out of Scope

- **`BlockerScope` enum, `RouteSegment` newtype, scope-keyed memory** — landed in ticket 002.
- **Observer Section 3b rendering of new clearing variants** — the clearing condition is part of the live blocker state, not the `BlockerRecorded` event payload. Observer rendering already includes `clearing_condition` in the debug format; no new observer wiring is needed for the new variants.
- **Golden cross-tick clearing scenarios** — `RouteRetraversedSafely fires on safe traversal observation` and `CounterpartyAccepted fires on successful interaction` scenarios live in ticket 006's `golden_cross_goal_blocker_scoping.rs`.
- **Modifying existing fact-specific clearing variants** (`CommodityAvailabilityChanged`, `PathDiscovered`, etc.) — preserved unchanged.

## Acceptance Criteria

### Tests That Must Pass

1. `blocker_clearing_condition_and_baseline_satisfy_required_bounds` (extended) — proves both new variants satisfy `Copy, Eq, Serialize, Deserialize`.
2. `blocker_memory_roundtrips_through_bincode` (extended) — roundtrips blockers with each new variant.
3. `sweep_cleared_removes_route_retraversed_safely_blockers` (new) — proves sweep predicate matches the new variant.
4. `sweep_cleared_removes_counterparty_accepted_blockers` (new) — same for Counterparty.
5. Focused tests in agent_tick/observation.rs and agent_tick/execution.rs proving clearing-condition selection at recording.
6. Existing suite: `cargo test -p worldwake-core --lib blocker_memory` clean; `cargo test -p worldwake-ai --lib agent_tick` clean.
7. Workspace: `cargo clippy --workspace --all-targets -- -D warnings` clean.

### Invariants

1. Recording-site clearing-condition selection is deterministic: same `(scope, blocking_fact)` input always produces the same `clearing_condition` choice.
2. `sweep_cleared` with a `RouteRetraversedSafely(seg)` predicate removes exactly the blockers whose `clearing_condition` matches that segment, leaving siblings untouched.
3. `BlockerClearingCondition` continues to satisfy `Copy + Eq + Serialize + Deserialize`.
4. Existing fact-specific clearing variants and `TtlOnly` semantics are preserved unchanged.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/blocker_memory.rs` — variant additions, new focused tests for sweep predicate behavior.
2. `crates/worldwake-ai/src/agent_tick/tests.rs` (and/or alongside recording-site host modules) — new tests asserting clearing-condition selection at recording. Path confirmed during 002 implementation.

### Commands

1. `cargo test -p worldwake-core --lib blocker_memory`
2. `cargo test -p worldwake-ai --lib agent_tick`
3. `cargo test -p worldwake-systems --lib trade_actions`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `./scripts/verify.sh` for the full pre-PR gate.
