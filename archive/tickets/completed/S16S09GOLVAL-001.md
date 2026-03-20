# S16S09GOLVAL-001: Extract `no_recovery_combat_profile` and `stable_wound_list` to golden harness

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

`no_recovery_combat_profile()` and `stable_wound_list()` are private helpers in `golden_emergent.rs` (lines 87 and 104). Tickets S16S09GOLVAL-002 through S16S09GOLVAL-004 all need them in `golden_combat.rs`. Duplicating the helpers violates DRY; extracting to the shared harness is the clean solution.

## Assumption Reassessment (2026-03-20)

1. `no_recovery_combat_profile()` exists at `crates/worldwake-ai/tests/golden_emergent.rs:87` as a private `fn`. It returns a `CombatProfile` with `natural_recovery_rate: pm(0)` and `defend_stance_ticks: nz(10)`. Current call sites are in `run_wound_vs_hunger`, `run_cooperative_care_delivery`, `run_late_arrival_healer_blocked_by_prehealed_patient`, `run_scavenger_loot_vs_self_care`, and `run_political_vacancy_after_combat_death`.
2. `stable_wound_list()` exists at `crates/worldwake-ai/tests/golden_emergent.rs:104` as a private `fn`. It creates a one-entry `WoundList` with a clotted starvation wound (`bleed_rate_per_tick: pm(0)`) at the requested severity.
3. No equivalent public helpers exist in `golden_harness/mod.rs` — checked via grep for `no_recovery_combat_profile` and `stable_wound_list` in the harness module.
4. `specs/S16-s09-golden-validation.md` says "Existing `golden_harness` infrastructure — no new utilities needed", but the active follow-up tickets `tickets/S16S09GOLVAL-002.md` through `tickets/S16S09GOLVAL-004.md` all currently depend on these helpers being shared. The spec-level assumption is therefore stale; the ticket scope is corrected to add the minimal shared harness utilities needed by those combat goldens.
5. Not an AI regression ticket. No ordering contract. Pure test-infrastructure refactor: move helper ownership into the shared harness and update existing call sites.
6. Verification commands are real for the current binary layout. `cargo test -p worldwake-ai -- --list` shows the `golden_emergent` and `golden_combat` test binaries that this ticket affects.
7. Not removing or weakening any heuristic or filter.
8. Not a stale-request or start-failure ticket.
9. Not a political office-claim ticket.
10. No ControlSource manipulation.
11. No golden scenario isolation needed — this ticket changes reusable setup helpers only, not scenario setup or assertions.
12. Mismatch corrected: the original ticket understated the current dependency graph by treating helper extraction as only future-facing. In the current planning set, it is a concrete prerequisite for the pending S16 combat goldens.

## Architecture Check

1. Moving shared helpers to `golden_harness/mod.rs` follows the existing pattern — `default_combat_profile()`, `seed_agent()`, `give_commodity()` etc. are already there. This is strictly cleaner than duplicating the function across test files.
2. No backwards-compatibility shims. The private functions in `golden_emergent.rs` are replaced with calls to the public harness versions.

## Verification Layers

1. Helper semantics stay identical -> direct source equivalence in `golden_harness/mod.rs` and unchanged assertions in existing goldens
2. Existing emergent care / combat-adjacent scenarios still execute unchanged -> targeted `golden_emergent` test binary
3. Shared harness extraction does not regress the wider AI golden suite -> `cargo test -p worldwake-ai`
4. Workspace lint remains clean after the harness API change -> `cargo clippy --workspace`

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

## Outcome

- Completion date: 2026-03-20
- What actually changed: moved `no_recovery_combat_profile()` and `stable_wound_list()` into `crates/worldwake-ai/tests/golden_harness/mod.rs`, removed the private duplicates from `crates/worldwake-ai/tests/golden_emergent.rs`, and corrected the ticket assumptions to reflect the current S16 dependency graph.
- Deviations from original plan: verification expanded beyond the original ticket commands to include `cargo test --workspace`, because the user requested full test and lint confirmation before archival. No new tests were added because this ticket only changed shared golden test infrastructure, not behavior or assertions.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_emergent` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
