# S111SCEHOMLIN-001: PlanningSnapshot accessor-only doctest

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

`PlanningSnapshot` (`crates/worldwake-ai/src/planning_snapshot.rs:395`) holds `shortest_travel_ticks: DistanceMatrix` (line 427) — the all-pairs Floyd-Warshall distance matrix that is the canonical authoritative-only travel data inside the AI crate. Today the field has no visibility qualifier (module-private) and `DistanceMatrix` is also private, so cross-crate readers cannot reach the matrix and must use `pub fn min_travel_ticks(...)` / `pub fn min_travel_ticks_to_any(...)` accessor methods (lines 713, 724). FND-7 (Locality) depends on this surface staying narrow: planner-adjacent code outside `worldwake-ai` must not learn global authoritative distances, only what the accessor methods compute.

There is no compile-time regression guard against a future PR that promotes either symbol to `pub` (or `pub(crate) → pub`). S111 spec D3 calls this the travel-fence audit (PR-1.11 in the assessment).

## Assumption Reassessment (2026-04-19)

1. **Existing tests on PlanningSnapshot**: `cargo test --doc -p worldwake-ai` runs zero doctests on `planning_snapshot.rs` today (the module has no `///` example blocks). The module is exercised indirectly through `crates/worldwake-ai/tests/planner_conformance.rs:210` (`assert_type_is_available::<PlanningSnapshot>()`), `tests/golden_*.rs` golden runs, and unit tests in `src/search/tests.rs` — none of these exercise field visibility, only behavior.
2. **Spec/docs reference**: `specs/S111-scenario-homogeneity-lints.md` D3 (current revision after `/reassess-spec` 2026-04-19). The spec references `crates/worldwake-ai/src/lib.rs:96` for the `PlanningSnapshot` re-export which makes `use worldwake_ai::PlanningSnapshot;` work in doctests.
3. **Shared abstraction boundary**: the audit boundary is the `PlanningSnapshot` public surface — what reaches outside `worldwake-ai`. The contract is "accessor methods only; no `pub` fields whose type is `DistanceMatrix` or any other authoritative-matrix-allowlist type". This ticket asserts that boundary at compile time.
4. **Acceptance-criteria correction before implementation**: the drafted manual regression note overstated the failure trigger for the field visibility case. A doctest compiled as an external crate should fail only if `shortest_travel_ticks` becomes `pub` (or if `DistanceMatrix` becomes `pub`); `pub(crate)` remains crate-private and should continue to fail. The ticket closes against the corrected external-visibility boundary instead of preserving the stale `pub(crate)` wording.

## Architecture Check

1. A `compile_fail` doctest co-located on `PlanningSnapshot` is cleaner than a `tests/*.rs` source-text scan: it fails at compile time (no test-runner overhead), travels with the type definition (so future reorganizations can't separate the test from the symbol), and uses Rust's own visibility checker as the oracle rather than a brittle text matcher. The positive-case doctest paired beside it prevents the negative test from silently always-passing if `PlanningSnapshot` itself stops compiling.
2. No backwards-compatibility shims introduced. The doctest only reads the existing public surface; it adds no new API.

## Verification Layers

1. `shortest_travel_ticks` field is unreachable from outside `worldwake-ai` -> dedicated `compile_fail` doctest (cargo test --doc compile failure).
2. `DistanceMatrix` type is unreachable from outside `worldwake-ai` -> dedicated `compile_fail` doctest (cargo test --doc compile failure).
3. Public accessor methods (`min_travel_ticks`, `min_travel_ticks_to_any`) remain reachable from outside `worldwake-ai` -> positive-case doctest (cargo test --doc compile + execution success).
4. Single-layer ticket: this is a compile-time visibility assertion only — no runtime behavior changes, so no decision-trace, action-trace, or event-log layer applies.

## What to Change

### 1. Add negative-case `compile_fail` doctests to `PlanningSnapshot`

Above the `pub struct PlanningSnapshot { ... }` declaration in `crates/worldwake-ai/src/planning_snapshot.rs` (currently line 395), add documentation blocks whose `compile_fail` examples separately prove the two privacy boundaries from outside the crate:

- one snippet attempts to read `shortest_travel_ticks` from `PlanningSnapshot`
- one snippet mentions `worldwake_ai::planning_snapshot::DistanceMatrix`

Today both fail compilation because `shortest_travel_ticks` is module-private and `DistanceMatrix` is module-private. Splitting the proof is necessary because a single snippet that names both boundaries would still fail if only one symbol became public.

### 2. Add positive-case doctest

Immediately after the `compile_fail` block, add a runnable doctest that reads via the accessor (`s.min_travel_ticks(from, to)`). This compiles and runs successfully today and ensures the regression test cannot silently always-pass if the public surface itself breaks (e.g., if `PlanningSnapshot` is removed from `lib.rs:96`).

The positive doctest needs `EntityId` from `worldwake-core` and a `PlanningSnapshot` value. Constructing a real `PlanningSnapshot` from a doctest is heavy (it requires a `RuntimeBeliefView`); the simpler path is a function-signature-only doctest that exercises the symbol surface without instantiation:

```rust
/// ```
/// use worldwake_ai::PlanningSnapshot;
/// use worldwake_core::EntityId;
/// fn read_via_accessor(s: &PlanningSnapshot, from: EntityId, to: EntityId) -> Option<u32> {
///     s.min_travel_ticks(from, to)
/// }
/// ```
```

This proves the accessor signature is reachable. If `min_travel_ticks` is renamed or removed, the positive doctest fails — making the regression complete.

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — add doc block above `pub struct PlanningSnapshot` at line 395)

## Out of Scope

- Any change to `PlanningSnapshot` field visibility, field types, or accessor method signatures.
- Any change to `DistanceMatrix` definition or visibility.
- The `scenario::lints` module (covered by S111SCEHOMLIN-002).
- Wiring the lint module into scenario load (covered by S111SCEHOMLIN-003).

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --doc -p worldwake-ai` passes (all new doctests succeed: two `compile_fail` blocks fail to compile, the positive block compiles and runs).
2. Manually confirmed regression boundary: temporarily marking `pub struct DistanceMatrix` causes the type-mention `compile_fail` doctest to fail, and temporarily marking both `pub struct DistanceMatrix` plus `pub shortest_travel_ticks` causes the field-access `compile_fail` doctest to fail. Revert before commit.
3. Existing suite: `cargo test -p worldwake-ai` passes unchanged.
4. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings` passes (no new lints from doc comments).

### Invariants

1. `shortest_travel_ticks` field of `PlanningSnapshot` is unreachable from outside `crates/worldwake-ai`.
2. `DistanceMatrix` type is unreachable from outside `crates/worldwake-ai`.
3. `PlanningSnapshot::min_travel_ticks` and `min_travel_ticks_to_any` remain part of the public accessor surface.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs` — three new doctests added above `pub struct PlanningSnapshot` (field-privacy `compile_fail`, type-privacy `compile_fail`, positive runnable). Rationale: enforces FND-7 boundary at compile time without adding a `tests/` file.

### Commands

1. `cargo test --doc -p worldwake-ai` (targeted — proves all doctests behave as designed)
2. `cargo test -p worldwake-ai` (regression)
3. `cargo clippy --workspace --all-targets -- -D warnings` (lint parity with CI)

## Outcome

Completed on 2026-04-19.

Added co-located `PlanningSnapshot` doctests in `crates/worldwake-ai/src/planning_snapshot.rs` to lock the public boundary at compile time. One `compile_fail` snippet attempts to read `shortest_travel_ticks`; a second `compile_fail` snippet mentions `worldwake_ai::planning_snapshot::DistanceMatrix`; together they cover the real public export path for authoritative travel data. The positive doctest proves the accessor-only surface remains reachable by compiling calls to both `min_travel_ticks` and `min_travel_ticks_to_any`.

## Deviations

Corrected three drafted mismatches before closeout: `pub(crate)` on `shortest_travel_ticks` is not an external visibility leak; the externally reachable type path is `worldwake_ai::planning_snapshot::DistanceMatrix`, not the crate root; and a single snippet that names both the field and `DistanceMatrix` does not fail open when only one symbol becomes public. The landed proof uses two `compile_fail` doctests so the type leak is asserted directly and the field leak is asserted at the strongest honest seam.

## Verification Result

Passed on 2026-04-19:

1. `cargo test --doc -p worldwake-ai`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

Manual regression boundary also confirmed locally by temporarily changing `DistanceMatrix` to `pub`, observing the type-mention `compile_fail` doctest fail, then temporarily changing both `DistanceMatrix` and `shortest_travel_ticks` to `pub`, observing both `compile_fail` doctests fail, and finally reverting those changes before final verification.
