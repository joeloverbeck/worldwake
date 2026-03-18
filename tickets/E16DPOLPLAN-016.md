# E16DPOLPLAN-016: Update coverage report with E16d political scenarios

**Status**: PENDING
**Priority**: LOW
**Effort**: Medium
**Engine Changes**: None
**Deps**: E16DPOLPLAN-008 through E16DPOLPLAN-015, E16DPOLPLAN-020, E16DPOLPLAN-021

## Problem

The golden E2E coverage report does not document any political scenarios. After E16d implementation, 12 new scenarios need to be tracked.

## Assumption Reassessment (2026-03-18)

1. Coverage report is at `docs/golden-e2e-coverage.md` — confirmed (note: spec says `reports/golden-e2e-coverage-analysis.md` but actual file is at `docs/golden-e2e-coverage.md`)
2. Scenarios file is at `docs/golden-e2e-scenarios.md` — confirmed
3. Current coverage: 17/18 GoalKinds, 87 tests, 48 cross-system chains — confirmed
4. `ClaimOffice` and `SupportCandidateForOffice` are not yet tracked — confirmed

## Architecture Check

1. Pure documentation update — no code changes
2. Follows existing format and conventions in both files

## What to Change

### 1. `docs/golden-e2e-coverage.md`

- **GoalKind table**: Add `ClaimOffice` (Yes, scenarios 11-17, 19-20) and `SupportCandidateForOffice` (Yes, scenario 12). Coverage becomes 19/20 (`SellCommodity` still untested).
- **ActionDomain table**: Note new Social sub-actions (Bribe, Threaten, DeclareSupport) exercised in scenarios 13, 14, 11-12.
- **Cross-system chains**: Add ~10 new proven interactions (political goal generation, succession resolution, bribe commodity transfer, courage-based threaten, survival suppression, faction eligibility, travel-to-jurisdiction, force succession, incumbent defense, information locality).
- **File Layout**: Add `golden_offices.rs` entry.
- **Summary statistics**: Update proven test count, cross-system chain count.

### 2. `docs/golden-e2e-scenarios.md`

- Add full documentation for scenarios 11-20 following existing format (file, test name, systems exercised, setup, emergent behavior proven, cross-system chain).

## Files to Touch

- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Any code changes
- Test changes
- Coverage for `SellCommodity` (still requires new system code)

## Acceptance Criteria

### Tests That Must Pass

1. No tests — documentation only
2. Existing suite: `cargo test --workspace` (verify no regressions)

### Invariants

1. All 12 new scenarios documented with consistent format
2. GoalKind coverage correctly updated to 19/20
3. Cross-system chain count accurately reflects new interactions
4. No scenario described that isn't implemented

## Test Plan

### New/Modified Tests

1. None — documentation only

### Commands

1. `cargo test --workspace` (sanity check)
