# S15STAFAIEME-004: Update Golden Coverage Dashboard For S15 Start-Failure Emergence

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S15STAFAIEME-001, S15STAFAIEME-002, S15STAFAIEME-003

## Problem

[docs/golden-e2e-coverage.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-coverage.md) still compresses S08 into the care-only regression and still carries stale backlog wording that points at the archived traceability spec rather than the current S10 pricing blocker. Once the new S15 scenarios land, the dashboard will become actively misleading unless it is updated.

## Assumption Reassessment (2026-03-19)

1. The current dashboard in [docs/golden-e2e-coverage.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-coverage.md) says the suite contains 118 `golden_*` tests across 10 files and names only the care pre-start wound disappearance case as explicit S08 start-failure proof.
2. The live inventory was cross-checked today with `cargo test -p worldwake-ai -- --list`, which currently enumerates the existing golden binaries and test names. This command must be rerun after the S15 scenario tickets land so file counts and test totals are updated from actual binary output, not hand-edited guesses.
3. This is a documentation-only ticket. The behavior gap is not missing focused/unit coverage; it is stale documentation that no longer reflects active golden E2E coverage after S15 is implemented.
4. The dashboard must describe mixed-layer chains precisely. When it lists the new scenarios, it should mention the specific cross-system recovery chain and that S08 coverage now exists in production, trade, and politics, not just care.
5. The current backlog section incorrectly says S02c is blocked on `specs/S08-ai-decision-traceability.md`. Per the S15 spec, that blocker wording is stale and should be replaced with the active S10 pricing constraint.
6. Because this ticket is documentation-only, no additional verification-layer mapping beyond inventory and consistency checks is needed.
7. Scope correction: do not adjust scenario semantics here. If inventory, counts, or descriptions disagree with the implemented tests, update this doc to match the code rather than rewriting the tests in this ticket.

## Architecture Check

1. Keeping the dashboard aligned with the live golden inventory is cleaner than leaving historical narrative drift in the planning docs.
2. No backwards-compatibility wording should preserve the obsolete "care-only S08" or "traceability spec blocker" story once the codebase has moved on.

## Verification Layers

1. File counts and test counts match live binaries -> `cargo test -p worldwake-ai -- --list`.
2. New cross-system S15 chains are represented in the dashboard narrative -> doc review against implemented test names from the three scenario tickets.
3. Stale blocker wording is removed and replaced with the current S10 pricing blocker -> doc diff review.
4. Additional action/decision-trace mapping is not applicable because this ticket does not change executable behavior.

## What to Change

### 1. Update the file layout and summary counts

Revise the file-layout counts, total `golden_*` count, and summary statistics in [docs/golden-e2e-coverage.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-coverage.md) using the actual post-S15 `--list` output.

### 2. Add the new S15 cross-system chains

Add the production, trade, and political S15 scenarios to the cross-system interaction section and summary language so the dashboard no longer implies S08 is only represented by the care race.

### 3. Fix stale backlog wording

Replace the outdated S02c blocker note that cites the archived traceability spec with the active S10 pricing blocker named in the S15 spec.

## Files to Touch

- `docs/golden-e2e-coverage.md` (modify)

## Out of Scope

- `docs/golden-e2e-scenarios.md`
- `docs/golden-e2e-testing.md`
- any `crates/` test or runtime code
- changing scenario semantics to make the documentation easier to write

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
2. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
3. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
4. Inventory check: `cargo test -p worldwake-ai -- --list`

### Invariants

1. The dashboard must reflect the codebase that actually exists after S15 lands; counts and names must come from the live test binaries.
2. S08 coverage must no longer be described as care-only once production, trade, and politics have explicit golden proof.
3. Documentation must not reintroduce stale blockers or archived-spec references as if they were current active constraints.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai --test golden_production`
3. `cargo test -p worldwake-ai --test golden_trade`
