# FND01PHA1FOUALI-002: Ban Zero-Tick Actions via NonZeroU32

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — action_semantics.rs type change, call-site updates across sim crate
**Deps**: None (independent, can parallelize with -001 and -003)

## Problem

`DurationExpr::Fixed(u32)` allows `Fixed(0)`, meaning a world action can complete instantly with no time cost. This violates Principle 6 (Every Action Has Physical Cost): "Actions consume time, materials, energy, or attention. Nothing is free."

## Assumption Reassessment (2026-03-10)

1. `DurationExpr` enum at `action_semantics.rs:36-46` has variant `Fixed(u32)` with `resolve()` returning `u32` — confirmed.
2. `ALL_DURATION_EXPRS` at `action_semantics.rs:107` contains `[DurationExpr::Fixed(0), DurationExpr::Fixed(5)]` — confirmed, `Fixed(0)` must be removed.
3. Test `duration_expr_resolves_fixed_ticks` asserts `Fixed(0).resolve() == 0` — confirmed, must be updated.
4. `DurationExpr::Fixed(...)` appears in `action_def.rs`, `action_def_registry.rs`, `affordance_query.rs`, `tick_action.rs`, `tick_step.rs`, `start_gate.rs`, and `interrupt_abort.rs` — confirmed via grep, but only `action_def.rs` is production-only usage today. The others are test code or test helpers that still matter because they encode construction patterns.
5. Before implementation, the only zero-duration uses were in tests and test helpers, not production action definitions. That still mattered because those tests were codifying invalid behavior, and the `duration: u32` helper seams in `tick_action.rs` and `start_gate.rs` allowed new zero-duration fixtures to slip back in.
6. `worldwake-sim` re-exports `DurationExpr` from `lib.rs`, so the change remains crate-public and will be caught at compile time across dependents without needing compatibility shims.

## Architecture Check

1. `NonZeroU32` is the idiomatic Rust way to encode "must be at least 1" at the type level. Zero-duration actions become unrepresentable, not just invalid.
2. No backward-compatibility shims — the old `Fixed(u32)` variant is replaced directly (Principle 13).
3. Tightening test helper signatures to `NonZeroU32` is more robust than keeping `u32` helpers plus `unwrap()` at the final construction site. It keeps the invariant at the helper boundary and prevents new zero-duration fixtures from creeping back in through tests.

## What to Change

### 1. Change `DurationExpr::Fixed(u32)` to `DurationExpr::Fixed(NonZeroU32)`

In `action_semantics.rs`:
- Add `use std::num::NonZeroU32;`
- Change variant: `Fixed(NonZeroU32)`

### 2. Update `resolve()` method

Change `Self::Fixed(ticks) => ticks` to `Self::Fixed(ticks) => ticks.get()`.

### 3. Update all call sites constructing `DurationExpr::Fixed(...)`

For each file that constructs `DurationExpr::Fixed(n)`:
- Replace `DurationExpr::Fixed(n)` with `DurationExpr::Fixed(NonZeroU32::new(n).unwrap())` where `n` is a known-positive literal.
- Where test helpers currently accept `duration: u32`, change the helper parameter itself to `NonZeroU32` and pass positive `NonZeroU32` values at the call sites. This is cleaner than leaving a raw `u32` seam in helper APIs.
- Files to update:
  - `crates/worldwake-sim/src/action_def.rs`
  - `crates/worldwake-sim/src/action_def_registry.rs`
  - `crates/worldwake-sim/src/affordance_query.rs`
  - `crates/worldwake-sim/src/tick_action.rs`
  - `crates/worldwake-sim/src/tick_step.rs`
  - `crates/worldwake-sim/src/start_gate.rs`
  - `crates/worldwake-sim/src/interrupt_abort.rs`

### 4. Update `ALL_DURATION_EXPRS` test constant

Remove `DurationExpr::Fixed(0)` from the array. Replace with a valid non-zero value (e.g., `Fixed(NonZeroU32::new(1).unwrap())`).

### 5. Update test assertions

- Remove `assert_eq!(DurationExpr::Fixed(0).resolve(), 0)` (no longer representable).
- Update `duration_expr_resolves_fixed_ticks` to test with non-zero values.
- Update `duration_expr_bincode_roundtrip_covers_every_variant` for the new type.

### 6. Add enforcement test

Add a test: `NonZeroU32::new(0)` returns `None` — documenting the invariant that zero-duration is unrepresentable.

## Files to Touch

- `crates/worldwake-sim/src/action_semantics.rs` (modify — type change + tests)
- `crates/worldwake-sim/src/action_def.rs` (modify — constructor call sites in tests)
- `crates/worldwake-sim/src/action_def_registry.rs` (modify — constructor call sites in tests)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — constructor call sites in tests)
- `crates/worldwake-sim/src/tick_action.rs` (modify — constructor call sites and helper signature in tests)
- `crates/worldwake-sim/src/tick_step.rs` (modify — constructor call sites in tests)
- `crates/worldwake-sim/src/start_gate.rs` (modify — constructor call sites and helper signature in tests)
- `crates/worldwake-sim/src/interrupt_abort.rs` (modify — constructor call sites in tests)
- `crates/worldwake-sim/src/lib.rs` (verify re-export remains correct; code change only if required by compilation)

## Out of Scope

- Do NOT change `DurationExpr` to add new variants (e.g., `Variable`, `Scaled`).
- Do NOT modify action execution logic beyond updating the type.
- Do NOT change the `Scheduler`, `ActionInstance`, or `ActionState` types.
- Do NOT touch worldwake-core crate.

## Acceptance Criteria

### Tests That Must Pass

1. `DurationExpr::Fixed` wraps `NonZeroU32` (compile-time enforcement).
2. No `Fixed(0)` exists anywhere in the codebase (verified by grep).
3. Enforcement test: `assert!(NonZeroU32::new(0).is_none())` documents the zero-duration ban.
4. Existing suite: `cargo test -p worldwake-sim`
5. Full suite: `cargo test --workspace`
6. `cargo clippy --workspace` clean.

### Invariants

1. All action definitions that previously used `Fixed(n)` where `n > 0` continue to resolve to the same tick count.
2. `resolve()` return type remains `u32`.
3. Serde round-trip for `DurationExpr` continues to work (bincode serializes `NonZeroU32` as `u32`).
4. Test utilities and fixtures no longer accept raw `u32` for fixed durations where a zero value would be invalid.

## Test Plan

### New/Modified Tests

1. `action_semantics.rs::zero_duration_is_unrepresentable` — new test asserting `NonZeroU32::new(0).is_none()`.
2. `action_semantics.rs::duration_expr_resolves_fixed_ticks` — updated to use non-zero values.
3. `action_semantics.rs::duration_expr_bincode_roundtrip_covers_every_variant` — updated for new type.
4. Existing sim tests that build `ActionDef` fixtures continue to compile after helper signatures are tightened to `NonZeroU32`, proving the invariant propagates through test construction paths.

### Commands

1. `cargo test -p worldwake-sim -- action_semantics`
2. `cargo test --workspace && cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-10
- What actually changed:
  - `DurationExpr::Fixed` now stores `NonZeroU32`, so zero-tick actions are unrepresentable at the type level.
  - `resolve()` still returns `u32`, preserving downstream arithmetic while enforcing the invariant at construction time.
  - Sim test fixtures that construct `ActionDef` values were updated to use `NonZeroU32`, and the test helper seams in `start_gate.rs` and `tick_action.rs` were narrowed from `u32` to `NonZeroU32`.
  - Added `action_semantics.rs::zero_duration_is_unrepresentable`.
  - Removed obsolete zero-duration behavior tests from `start_gate.rs` and `tick_action.rs` because they were validating the bug rather than protecting a real invariant.
- Deviations from original plan:
  - `crates/worldwake-sim/src/lib.rs` did not need modification after verification.
  - The effective blast radius was narrower in production code than the original ticket implied; most touched files were test-only.
- Verification results:
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-sim duration_expr -- --nocapture`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
