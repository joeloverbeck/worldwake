# S16S09GOLVAL-001: Extract `no_recovery_combat_profile` and `stable_wound_list` to golden harness

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

`no_recovery_combat_profile()` and `stable_wound_list()` are private helpers in `golden_emergent.rs` (lines 87 and 104). Tickets S16S09GOLVAL-002 through S16S09GOLVAL-004 all need them in `golden_combat.rs`. Duplicating the helpers violates DRY; extracting to the shared harness is the clean solution.

## Assumption Reassessment (2026-03-20)

1. `no_recovery_combat_profile()` exists at `crates/worldwake-ai/tests/golden_emergent.rs:87` as a private `fn`. Returns `CombatProfile` with `natural_recovery_rate: pm(0)`, `defend_stance_ticks: nz(10)`. Used by 6 tests in that file.
2. `stable_wound_list()` exists at `crates/worldwake-ai/tests/golden_emergent.rs:104` (inferred from `no_recovery_combat_profile` context). Creates a single-wound `WoundList` with clotted (bleed_rate=0) wound at given severity.
3. No equivalent public helpers exist in `golden_harness/mod.rs` — checked via grep for `no_recovery_combat_profile` and `stable_wound_list` in the harness module.
4. Not an AI regression ticket. No ordering contract. Pure refactor — move helpers, update call sites.
5. Not removing/weakening any heuristic or filter.
6. Not a stale-request or start-failure ticket.
7. Not a political office-claim ticket.
8. No ControlSource manipulation.
9. No golden scenario isolation needed — this is infrastructure.
10. No mismatches found.

## Architecture Check

1. Moving shared helpers to `golden_harness/mod.rs` follows the existing pattern — `default_combat_profile()`, `seed_agent()`, `give_commodity()` etc. are already there. This is strictly cleaner than duplicating the function across test files.
2. No backwards-compatibility shims. The private functions in `golden_emergent.rs` are replaced with calls to the public harness versions.

## Verification Layers

1. Single-layer ticket: pure test infrastructure refactor. No behavioral invariants change.
2. All existing golden tests that use `no_recovery_combat_profile()` must still compile and pass unchanged — verified by running the full `worldwake-ai` test suite.

## What to Change

### 1. Add helpers to `golden_harness/mod.rs`

Add `pub fn no_recovery_combat_profile() -> CombatProfile` and `pub fn stable_wound_list(severity: u16) -> WoundList` to the shared harness, using the exact same implementation currently in `golden_emergent.rs`.

### 2. Update `golden_emergent.rs` call sites

Remove the private `no_recovery_combat_profile()` and `stable_wound_list()` functions. Replace all call sites with imports from the harness module.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add two public helpers)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify — remove private helpers, update imports)

## Out of Scope

- Any engine/production code changes
- Adding new golden test scenarios
- Changing the behavior of any existing test
- Modifying `golden_combat.rs` (that happens in subsequent tickets)
- Changing the function signatures or semantics of the helpers

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_emergent` — all existing emergent golden tests still pass
2. `cargo test -p worldwake-ai` — full AI crate suite passes (no regressions from import changes)

### Invariants

1. `no_recovery_combat_profile()` returns the exact same `CombatProfile` as before (`natural_recovery_rate: pm(0)`, `defend_stance_ticks: nz(10)`)
2. `stable_wound_list()` returns the exact same `WoundList` as before
3. No test behavior changes — only import paths change

## Test Plan

### New/Modified Tests

1. None — infrastructure-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo test -p worldwake-ai golden_emergent`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
