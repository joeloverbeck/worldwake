# S143STABELVIE-006: Belief-wall trap golden + `compile_fail` doctest on `DebugWorldView`

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None (test infrastructure only; new golden + doctest)
**Deps**: archive/tickets/S143STABELVIE-002.md, S143STABELVIE-003, S143STABELVIE-004

## Problem

The spec's two type-level guarantees need explicit regression coverage:

1. **FND-14A wall** — "co-location does not tell you who owns the chest". Without a golden that exercises co-located physical observation alongside authority-belief absence, a future refactor could silently re-introduce an authoritative read on a `BelievedAuthorityView` impl, returning a real owner identity for a co-located chest where the agent has no owner-belief. The CI grep lint (ticket 005) doesn't catch this — it catches only `DebugWorldView` imports.
2. **`DebugWorldView` unreachability** — the cfg-gate (ticket 002) and CI grep lint (ticket 005) provide enforcement, but a `compile_fail` doctest on the trait's documentation is the type-level witness that future refactors cannot accidentally widen the trait's reachability. Compile-fail doctests run under `cargo test --doc` and produce a clear failure mode when the boundary is breached.

## Assumption Reassessment (2026-05-13)

1. Compile-fail doctest precedents in worldwake-ai: `crates/worldwake-ai/src/planning_snapshot.rs:402-414` (proves the `DistanceMatrix` field is unreachable from external code; verified during prior reassessment) and `crates/worldwake-ai/src/ranking.rs:7-19` (proves the `compare_ranked_goals` private function and the `OrderedRanked::from_sorted_for_test` test-only accessor are unreachable from external code). The pattern uses `/// ```compile_fail` doctests in the trait/struct doc comment.
2. Adjacent existing goldens that exercise observation and belief boundaries: `crates/worldwake-ai/tests/golden_epistemic_sensing.rs`, `golden_perception_omission.rs`, `golden_perception_exposure.rs` (all confirmed to exist via path batch). The new belief-wall trap golden composes with these — it does not replace them; the scope is distinct (authority-belief absence given physical co-location).
3. Scenario authoring approach: golden tests in `crates/worldwake-ai/tests/` typically construct scenarios programmatically or via RON loading (e.g., `scenarios/*.ron`). The belief-wall trap golden authors a small scenario inline or as a dedicated RON file with: one agent, one chest (item-lot), one place (office building); agent's belief store has co-location observation but no owner-belief, holder-belief, jurisdiction-belief, or office-holder-belief for the chest/building.

## Architecture Check

1. The golden lives in the standard `crates/worldwake-ai/tests/golden_*.rs` location, following existing convention.
2. The `compile_fail` doctest lands on `DebugWorldView`'s trait doc comment in `crates/worldwake-sim/src/belief_view.rs`. Compile-fail doctests are exercised by `cargo test --doc -p worldwake-sim`; they produce a clear failure signal if the cfg-gate is ever relaxed or the trait is moved.
3. The golden's assertions cover both the legal path (physical observation succeeds for co-located entities) and the illegal-by-FND-14A path (authority-belief reads return `BeliefRead::Unknown` despite co-location).

## Verification Layers

1. **Decision-trace assertion (FND-14A wall)**: After the agent observes the chest, no `Steal` candidate appears in the agent's decision trace — verified via `AgentDecisionTrace` inspection in the golden. This is the primary behavioral proof that the wall holds end-to-end.
2. **Authority-belief absence (focused)**: A focused unit test within the golden directly verifies `view.believed_owner_of(chest) == BeliefRead::Unknown` and `view.believed_office_holder(office) == BeliefRead::Unknown` for the test agent at scenario start. This is the lower-layer proof surface for the type-system contract.
3. **Co-located physical observation (legal path)**: A focused unit test verifies `view.colocated_entities(actor).value.contains(&chest)` and `value.contains(&building)` — confirming the legal `LocalPhysicalObservationView` reads succeed.
4. **Compile-time enforcement (`DebugWorldView`)**: `cargo test --doc -p worldwake-sim` exercises the `compile_fail` doctest — its absence of compile failure (i.e., if the doctest unexpectedly compiles) means the trait surface no longer enforces the boundary; the doctest fails the test run.

## What to Change

### 1. New golden test `crates/worldwake-ai/tests/golden_belief_wall_trap.rs`

Per spec D8. Scenario setup:
- One agent, located at one place ("office_building").
- One chest (`ItemLot` entity) at the same place, owned (in authoritative world state) by a second agent the test agent has never met.
- The office building has an institutional record (`OfficeHeld`) indicating a magistrate's jurisdiction.
- The agent's `AgentBeliefStore`:
  - Has co-location observation entries for the chest and building (per perception output).
  - Has **no** owner-belief for the chest (`believed_owner_of(chest) == Unknown`).
  - Has **no** holder-belief, access-right-belief, jurisdiction-belief, or office-holder-belief for the building/office.

Assertions:
- `view.colocated_entities(actor).value` contains the chest and building entities.
- `view.believed_owner_of(chest) == BeliefRead::Unknown`.
- `view.believed_holder_of(chest) == BeliefRead::Unknown`.
- `view.believed_jurisdiction(building) == BeliefRead::Unknown`.
- `view.believed_office_holder(office) == BeliefRead::Unknown`.
- After running the agent's decision tick, the decision trace contains no `Steal` candidate (the legality predicate requires `believed_owner_of` which returns `Unknown`).
- After running the decision tick, no `Steal` action is committed to the event log (regression guard at the authoritative outcome layer).

### 2. `compile_fail` doctest on `DebugWorldView` in `crates/worldwake-sim/src/belief_view.rs`

Add to the trait's doc comment:

```rust
/// `DebugWorldView` is a labeled surface for debug/observer/test access. It is
/// cfg-gated and must not be reachable from release builds of `worldwake-ai`.
///
/// ```compile_fail
/// // This doctest fails to compile because DebugWorldView is gated by
/// // #[cfg(any(debug_assertions, test))] and the doctest harness compiles
/// // doctest code in release-equivalent context for this assertion.
/// fn requires_release_debug() {
///     let _check: fn(&dyn worldwake_sim::DebugWorldView) = |_| {};
/// }
/// ```
#[cfg(any(debug_assertions, test))]
pub trait DebugWorldView {
    // ...existing trait body from ticket 002...
}
```

The exact doctest framing follows the existing precedents (`planning_snapshot.rs:402-414`, `ranking.rs:7-19`); the implementer should mirror that style. If the doctest harness's cfg behavior makes the cfg-gate approach unverifiable via `compile_fail`, fall back to a different witness — e.g., a `compile_fail` doctest exercising a private trait method or a sealed-trait pattern. The implementer documents the chosen witness in the doc comment.

## Files to Touch

- `crates/worldwake-ai/tests/golden_belief_wall_trap.rs` (new)
- `crates/worldwake-sim/src/belief_view.rs` (modify — add `compile_fail` doctest on `DebugWorldView`)

Likely: scenario fixture file for the belief-wall trap setup may be needed if existing golden tests use RON files rather than inline construction. To be confirmed during implementation: inspect adjacent goldens (`golden_epistemic_sensing.rs`, `golden_perception_omission.rs`, `golden_perception_exposure.rs`) and follow whichever fixture convention they use.

## Out of Scope

- Modifying existing goldens (`golden_epistemic_sensing.rs`, etc.) — the new golden composes with them, not replaces.
- The CI grep lint — ticket 005 (already lands enforcement on `DebugWorldView` imports, separately from the doctest).
- Trait migration — tickets 003 and 004.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_belief_wall_trap` — the new golden passes.
2. `cargo test -p worldwake-sim --doc` — the new `compile_fail` doctest on `DebugWorldView` exercises correctly (the harness reports the expected compile failure as a passing doctest).
3. Existing adjacent goldens continue to pass: `cargo test -p worldwake-ai golden_epistemic_sensing`, `golden_perception_omission`, `golden_perception_exposure`.
4. Existing suite: `cargo test --workspace`.

### Invariants

1. The golden encodes the FND-14A "co-location does not tell you who owns the chest" rule as a regression check — future refactors that re-introduce authoritative reads on `BelievedAuthorityView` impls will fail this golden.
2. The `compile_fail` doctest is the type-level witness that `DebugWorldView` imports from non-debug code paths are caught at compile time. A passing doctest means the boundary holds.
3. The golden's decision-trace and event-log assertions are layered per `docs/precision-rules.md` Rule 5 — decision-trace for AI reasoning behavior, event-log for authoritative outcome.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_belief_wall_trap.rs` (new) — full golden per spec D8, with decision-trace, focused belief-read, and event-log layer assertions.
2. `crates/worldwake-sim/src/belief_view.rs` — `compile_fail` doctest on `DebugWorldView` (per existing precedents at `planning_snapshot.rs:402-414`, `ranking.rs:7-19`).

### Commands

1. `cargo test -p worldwake-ai golden_belief_wall_trap`
2. `cargo test -p worldwake-sim --doc`
3. `cargo test -p worldwake-ai golden_epistemic_sensing golden_perception_omission golden_perception_exposure`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `scripts/verify.sh`
