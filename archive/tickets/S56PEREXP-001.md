# S56PEREXP-001: Add `Permille::ZERO` constant

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

Multiple upcoming S56 tickets and existing codebase sites use `Permille::new_unchecked(0)` or `Permille(0)` to represent zero. A named constant improves readability and consistency.

## Assumption Reassessment (2026-04-06)

1. `Permille` is defined at `crates/worldwake-core/src/numerics.rs:25` as `pub struct Permille(u16)` with `new()`, `new_unchecked()`, `value()`, `saturating_add()`, `saturating_sub()`. No existing `ZERO` constant.
2. `Permille(0)` appears in test code across archived specs and golden tests. No `Default` impl exists.
3. Single-layer ticket in `worldwake-core` only — no cross-system boundary.

## Architecture Check

1. Named constants for common sentinel values (ZERO) are standard Rust convention (cf. `Duration::ZERO`, `NonZeroU32`). Cleaner than scattered `new_unchecked(0)`.
2. No backwards-compatibility shims — additive change only.

## Verification Layers

1. `Permille::ZERO.value() == 0` -> focused unit test
2. `Permille::ZERO` usable in const context -> compilation proof
3. Single-layer ticket — no additional layer mapping applicable.

## What to Change

### 1. Add `ZERO` constant to `Permille`

In `crates/worldwake-core/src/numerics.rs`, add inside the existing `impl Permille` block:

```rust
pub const ZERO: Permille = Permille(0);
```

### 2. Add unit test

Add a test verifying `Permille::ZERO.value() == 0` and that it equals `Permille::new_unchecked(0)`.

## Files to Touch

- `crates/worldwake-core/src/numerics.rs` (modify)

## Out of Scope

- Replacing existing `Permille::new_unchecked(0)` or `Permille(0)` usage across the codebase — that's mechanical cleanup, not this ticket
- Adding other constants (`ONE`, `MAX`, etc.)

## Acceptance Criteria

### Tests That Must Pass

1. `Permille::ZERO.value() == 0`
2. `Permille::ZERO == Permille::new_unchecked(0)`
3. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `Permille::ZERO` is usable in `const` contexts
2. All existing tests continue to pass — additive change only

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/numerics.rs` (inline test) — verify ZERO constant value and equality

### Commands

1. `cargo test -p worldwake-core -- permille`
2. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-06.

- Added `Permille::ZERO` to `crates/worldwake-core/src/numerics.rs` as a const-friendly named zero value.
- Added an inline unit test proving `Permille::ZERO.value() == 0` and `Permille::ZERO == Permille::new_unchecked(0)`.

## Verification Result

- Passed `cargo test -p worldwake-core -- permille_zero_constant`
- Passed `cargo test -p worldwake-core`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
