# S23-006: Workspace verification and golden behavioral proof

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — verification only
**Deps**: S23-001, S23-002, S23-003, S23-004, S23-005

## Problem

After S23-001 through S23-005 change the blocker data model, recording, lookup, search pruning, and Unknown TTL, a full workspace verification is needed to confirm no regressions and that the core behavioral goals are met: (1) place-scoped blocking works (failure at Place A does not block Place B), (2) Unknown blockers expire faster, (3) decision traces include diagnostic context.

## Assumption Reassessment (2026-03-24)

1. Existing golden tests that exercise blockers (plan failures, StartFailed outcomes, multi-location resource competition) should pass unchanged — the behavioral change is that agents can now route around blocked places, not that blocking is removed.
2. `cargo test --workspace` is the authoritative verification command.
3. `cargo clippy --workspace` must show no new warnings.
4. No new golden test scenarios are strictly required by the spec. However, if existing golden scenarios do not already exercise multi-location routing, a targeted golden test proving "harvest failure at Place A, agent harvests at Place B" would strengthen confidence.
5. This is a verification-only ticket — no code changes expected unless a regression is found.

## Architecture Check

1. Verification-only — no architectural changes.
2. If regressions are found, they should be fixed in the originating ticket (S23-001–005), not here.

## Verification Layers

1. All focused tests pass → `cargo test --workspace`
2. All clippy checks pass → `cargo clippy --workspace`
3. Place-scoped blocking behavioral proof → existing or new golden test
4. Unknown TTL behavioral proof → existing focused test from S23-005
5. Trace diagnostic proof → existing focused test from S23-005

## What to Change

### 1. Run full workspace verification

```bash
cargo test --workspace
cargo clippy --workspace
```

### 2. Confirm key behavioral properties

- Harvesting failure at Place A no longer blocks harvesting at Place B
- Unknown blockers expire in 5 ticks, not 20
- Decision traces for Unknown blockers include diagnostic context (action_def)
- Search trace shows `PlaceBlocker` filter reasons when candidates are pruned

### 3. (If needed) Fix any regressions found

Attribute fixes to the originating ticket scope.

## Files to Touch

- None expected (verification only)
- If golden test is added: `crates/worldwake-ai/tests/golden_*.rs` (modify)

## Out of Scope

- **No new feature code** — this is verification only
- **No changes to blocker data model** — that is S23-001
- **No changes to search logic** — that is S23-004
- **Do not refactor or optimize any existing code during verification**

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace` — all pass (0 failures)
2. `cargo clippy --workspace` — no new warnings
3. Golden tests where blockers are exercised — pass unchanged
4. Confirm (via focused test output or manual inspection) that:
   - Two blockers for same goal at different places coexist in memory
   - Place-specific blocker does not suppress candidate generation
   - Place-specific blocker prunes search candidates at that place
   - Unknown blocker expires in 5 ticks
   - Unknown blocker diagnostic_context contains action_def

### Invariants

1. No existing test has been deleted, disabled, or weakened
2. No `#[ignore]` annotations added
3. Conservation invariants still hold (`verify_conservation`)
4. Deterministic replay still works (golden replay tests)
5. Save/load round-trip still works (format version 5)

## Test Plan

### New/Modified Tests

1. None — verification-only ticket; all tests were added in S23-001 through S23-005.

### Commands

1. `cargo test --workspace`
2. `cargo clippy --workspace`
3. `cargo test -p worldwake-ai -- golden` (focused golden test run)
