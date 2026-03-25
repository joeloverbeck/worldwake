# S25-004: Golden test verification and behavioral audit

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — verification-only ticket
**Deps**: S25-001, S25-002, S25-003

## Problem

After feasibility reordering is wired into the agent pipeline (S25-002), some golden tests may produce different tick counts or action sequences because agents now prefer locally-actionable goals. Every behavioral change must be verified as an improvement (agent acts more sensibly), not a regression.

## Assumption Reassessment (2026-03-25)

1. Golden tests live in `crates/worldwake-ai/tests/golden_*.rs`. As of S24 completion, there are 133+ `golden_*` tests across multiple files.
2. Golden tests assert on specific tick counts, action sequences, and state outcomes. Feasibility reordering may change the tick at which an agent starts eating, crafting, or traveling if a previously-higher-motive but unreachable goal was consuming a planning slot.
3. Any changed behavior should be strictly better: the agent should find food/action sooner, not later. If an agent does something worse, the feasibility check has a false positive (marking reachable as Unlikely) or false negative (marking unreachable as Likely).
4. Decision traces (`h.driver.enable_tracing()`) and action traces (`h.enable_action_tracing()`) are the primary diagnostic tools for understanding behavioral changes.
5. Deterministic replay tests should still pass since the feasibility annotation is deterministic (same beliefs + same blocker memory → same hint).

## Architecture Check

1. This is a verification-only ticket — no code changes except potential golden test assertion updates where behavior improved.
2. No backward-compatibility shims.

## Verification Layers

1. All golden tests pass: `cargo test -p worldwake-ai --test golden_*` (Note: actual pattern depends on test file naming; run `cargo test -p worldwake-ai` to catch all)
2. Any changed assertion is documented with a brief comment explaining why the new behavior is better
3. Deterministic replay tests pass alongside their main golden tests
4. If any test regresses, the root cause is investigated via decision traces before adjusting the test

## What to Change

### 1. Run all golden tests

```bash
cargo test -p worldwake-ai
```

### 2. For each failing golden test

- Enable decision tracing and action tracing to understand the behavioral change
- Determine whether the new behavior is an improvement (agent acts more sensibly due to feasibility reordering)
- If improvement: update the test assertion and add a brief comment noting the feasibility-driven improvement
- If regression: investigate the feasibility check — the bug is in S25-001's dispatch table, not in the golden test. Fix the feasibility check rather than adjusting the test.

### 3. Document changes

For each modified golden test, add a one-line comment above the changed assertion:
```rust
// S25: feasibility reordering — agent now eats local food (Likely) before searching for remote food (Uncertain)
```

## Files to Touch

- `crates/worldwake-ai/tests/golden_*.rs` (modify — only assertion updates where behavior improved; zero changes if all tests pass as-is)

## Out of Scope

- Writing new golden tests for feasibility (that would be a separate spec)
- Modifying `feasibility.rs`, `ranking.rs`, or `agent_tick/mod.rs` (those are S25-001 and S25-002)
- Changing any test to make it pass by weakening assertions — tests must reflect actual behavioral improvements

## Acceptance Criteria

### Tests That Must Pass

1. All `golden_*` tests: `cargo test -p worldwake-ai`
2. All deterministic replay companion tests pass alongside their main tests
3. No golden test has a weakened assertion (e.g., wider tick range, removed check)

### Invariants

1. Every modified golden assertion is accompanied by a comment explaining the improvement
2. No golden test is skipped, disabled, or commented out
3. If a feasibility false-positive/negative is discovered, it is fixed in `feasibility.rs` (backport to S25-001 scope), not papered over in the golden test

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_*.rs` — assertion updates only where behavior changed for the better; None if all pass unchanged

### Commands

1. `cargo test -p worldwake-ai` — full AI crate including all golden tests
2. `cargo clippy -p worldwake-ai` — no new warnings

## Outcome

- **Completion date**: 2026-03-25
- **What actually changed**: No golden test assertions were modified. All 955 worldwake-ai tests (across 13 test binaries) passed with 0 failures after S25-001/S25-002/S25-003 feasibility sketching changes.
- **Deviations from original plan**: None. The ticket anticipated the possibility that all tests pass as-is, and that is what happened. Feasibility reordering didn't change observable outcomes because agents were already finding actionable goals within the planning budget in all existing test scenarios.
- **Verification results**: `cargo test -p worldwake-ai` — 955 passed, 0 failed, 2 ignored. `cargo clippy -p worldwake-ai` — clean, no warnings.
