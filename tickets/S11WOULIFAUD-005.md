# S11WOULIFAUD-005: Golden test verification and hash recapture

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: S11WOULIFAUD-003, S11WOULIFAUD-004 (changes that may shift deterministic hashes)

## Problem

S11WOULIFAUD-003 changes deprivation wound accumulation behavior (worsening instead of creating duplicates). This alters the authoritative `WoundList` component state, which feeds into canonical state hashing. S11WOULIFAUD-004 changes AI ranking, which may alter agent decisions and thus the deterministic execution path. Any golden test that exercises deprivation or involves agents with clotted wounds may produce different hashes.

## Assumption Reassessment (2026-03-21)

1. Golden tests live in `crates/worldwake-ai/` (test files with `golden` in the name or scenarios that assert on deterministic hashes/event sequences). They use `h.step_once()` loops and assert on world state, event log contents, and/or canonical hashes.
2. The spec explicitly calls out: "Recapture hashes for any affected golden tests."
3. Not an AI regression ticket — this is a planned hash recapture after intentional behavior changes.
4. No ordering dependency within this ticket.
5. N/A — no heuristic removal.
6. N/A.
7. N/A.
8. N/A.
9. N/A.
10. No mismatch. Hash drift is expected and planned.

## Architecture Check

1. Recapturing golden test hashes after intentional behavior changes is the standard procedure. The alternative (freezing behavior to preserve hashes) would prevent the improvement.
2. No backwards-compatibility shims.

## Verification Layers

1. All golden tests pass after recapture → `cargo test -p worldwake-ai`
2. All workspace tests pass → `cargo test --workspace`
3. Clippy clean → `cargo clippy --workspace`
4. Single-layer ticket (test maintenance). If a golden test fails for a reason unrelated to the S11 changes, that is a separate issue to investigate — do not silently fix unrelated failures.

## What to Change

### 1. Run all golden tests and identify failures

```bash
cargo test -p worldwake-ai
```

Identify which tests fail due to changed wound accumulation behavior or changed AI ranking decisions.

### 2. For each failing golden test

- **Verify the failure is caused by S11 changes**: The new behavior (wound worsening, priority boost) should be visible in the test's scenario. If a test fails for an unrelated reason, stop and investigate.
- **Recapture hashes/assertions**: Update expected hash values, event counts, or state assertions to match the new correct behavior.
- **Do NOT change test logic or scenario setup** to work around the new behavior. The scenarios are correct; only the expected outputs change.

### 3. Run full workspace verification

```bash
cargo test --workspace
cargo clippy --workspace
```

## Files to Touch

- `crates/worldwake-ai/tests/*.rs` (modify — golden test files, exact files TBD based on which fail)
- Possibly `crates/worldwake-ai/src/*.rs` test modules if golden tests are inline

## Out of Scope

- Changing wound worsening logic (done in S11WOULIFAUD-003)
- Changing AI ranking logic (done in S11WOULIFAUD-004)
- Changing wound progression or pruning (done in S11WOULIFAUD-002)
- Adding new golden tests or new golden scenarios
- Fixing golden test failures unrelated to S11 changes (file separate ticket)
- Changing test scenario setup or harness configuration

## Acceptance Criteria

### Tests That Must Pass

1. All golden tests in `cargo test -p worldwake-ai` pass
2. All tests in `cargo test --workspace` pass
3. `cargo clippy --workspace` clean

### Invariants

1. Golden test scenarios (setup, tick count, agent configuration) are unchanged — only expected outputs are updated
2. Any hash change is traceable to wound worsening (S11WOULIFAUD-003) or AI priority boost (S11WOULIFAUD-004)
3. No test logic is adapted to work around bugs (TDD bugfixing rule)

## Test Plan

### New/Modified Tests

1. Golden test files TBD — depends on which tests are affected by the S11 behavior changes
2. No new test files created

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
