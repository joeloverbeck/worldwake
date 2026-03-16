# S02GOADECPOLUNI-006: Final verification — remove all legacy policy functions, full golden + clippy pass

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — cleanup and verification only
**Deps**: S02GOADECPOLUNI-002, S02GOADECPOLUNI-003, S02GOADECPOLUNI-004, S02GOADECPOLUNI-005

## Problem

After tickets 001–005, all four legacy functions should already be deleted. This ticket is a verification pass that confirms no legacy policy code survived, no dead imports remain, all golden tests pass, and the spec's acceptance criteria are fully met.

## Assumption Reassessment (2026-03-16)

1. `is_suppressed()` should have been removed in ticket 002 — verify.
2. `is_critical_survival_goal()` should have been removed in ticket 003 — verify.
3. `is_reactive_goal()` should have been removed in ticket 004 — verify.
4. `no_medium_or_above_self_care_or_danger()` should have been removed in ticket 004 — verify.
5. No `GoalKind::LootCorpse` or `GoalKind::BuryCorpse` pattern match should remain in `interrupts.rs` outside of tests — verify.
6. No `GoalKind::LootCorpse` or `GoalKind::BuryCorpse` pattern match should remain in `ranking.rs` outside of tests and `priority_class()`/`motive_score()` — verify.

## Architecture Check

1. This is a pure verification ticket. It makes no architectural changes.
2. If any legacy function survived, this ticket deletes it and verifies the deletion compiles.

## What to Change

### 1. Verify legacy function removal

Grep for:
- `fn is_suppressed` in ranking.rs
- `fn is_critical_survival_goal` in interrupts.rs
- `fn is_reactive_goal` in interrupts.rs
- `fn no_medium_or_above_self_care_or_danger` in interrupts.rs

If any survive, delete them.

### 2. Clean dead imports

After all deletions, remove unused `use` statements in ranking.rs, interrupts.rs, and agent_tick.rs. Run `cargo clippy --workspace` to catch unused imports.

### 3. Verify spec acceptance criteria checklist

Confirm each item from the spec's "Acceptance Criteria" section:
- [ ] The active corpse-opportunism split between `ranking.rs` and `interrupts.rs` is removed
- [ ] One shared goal-family policy declaration surface exists in `goal_policy.rs`
- [ ] Ranking and interrupts both consume that surface
- [ ] `is_suppressed()`, `is_critical_survival_goal()`, `is_reactive_goal()`, `no_medium_or_above_self_care_or_danger()` are completely removed
- [ ] No new world components or compatibility shims are introduced
- [ ] All 16 goal families are migrated (17 GoalKind variants, but AcquireCommodity with different purposes counts as multiple families sharing the same variant)

### 4. Run full test suite

Confirm all tests pass across the entire workspace.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (verify/modify — dead import cleanup only)
- `crates/worldwake-ai/src/interrupts.rs` (verify/modify — dead import cleanup only)
- `crates/worldwake-ai/src/agent_tick.rs` (verify/modify — dead import cleanup only)

## Out of Scope

- Adding new goal families
- Changing policy declarations
- Modifying test logic beyond dead import removal
- Changes to `worldwake-core` or `worldwake-sim`
- Any behavioral changes

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace` — full workspace green
2. `cargo clippy --workspace` — no warnings
3. `cargo build --workspace` — clean build
4. Grep confirmation: `is_suppressed`, `is_critical_survival_goal`, `is_reactive_goal`, `no_medium_or_above_self_care_or_danger` appear zero times in non-test production code in `ranking.rs` and `interrupts.rs`
5. All golden corpse-opportunism and suppression scenarios pass
6. All golden interrupt scenarios pass

### Invariants

1. No goal-family-specific policy branches exist in `ranking.rs` or `interrupts.rs` — all policy is declared in `goal_policy.rs`
2. No compatibility wrappers or dual-path policy evaluation exist
3. Deterministic behavior is unchanged across the full test suite
4. No new world components introduced
5. `goal_family_policy()` is the single authoritative policy declaration point

## Test Plan

### New/Modified Tests

1. No new tests — this is a verification-only ticket

### Commands

1. `cargo test --workspace`
2. `cargo clippy --workspace`
3. `grep -rn 'fn is_suppressed\|fn is_critical_survival_goal\|fn is_reactive_goal\|fn no_medium_or_above_self_care_or_danger' crates/worldwake-ai/src/ranking.rs crates/worldwake-ai/src/interrupts.rs`
