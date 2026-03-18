# E16DPOLPLAN-011: Golden Scenario 14 — Threaten with courage diversity

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None
**Deps**: E16DPOLPLAN-004, E16DPOLPLAN-007

## Problem

No golden test covers the threaten political path with diverse courage values producing different outcomes per target (Principle 20 — agent diversity).

## Assumption Reassessment (2026-03-18)

1. Threaten planning arm (E16DPOLPLAN-004) compares attack_skill vs courage — confirmed dependency
2. `commit_threaten` does loyalty increase on yield, hostility on resist — confirmed
3. `CombatProfile.attack_skill` accessible via snapshot — confirmed
4. `UtilityProfile.courage` varies per agent — confirmed

## Architecture Check

1. Tests Principle 20 (agent diversity): same action produces different outcomes based on per-agent parameters
2. Two targets with different courage values demonstrate divergent behavioral outcomes

## What to Change

### 1. Add to `golden_offices.rs`

- **Setup**: Vacant office. Agent A (high `attack_skill=pm(800)`, `enterprise_weight=pm(900)`). Agent B (`courage=pm(200)`, should yield). Agent C (`courage=pm(900)`, should resist). All at jurisdiction, sated.
- **Expected**: A generates `ClaimOffice`. Planner finds `Threaten(B)` viable (800 > 200) but not `Threaten(C)` (800 < 900). A threatens B -> B yields -> loyalty increase. A declares for self. B may support A. C does not.
- **Assertions**: B has increased loyalty to A. C has hostility or is unaffected. A becomes holder if sufficient support.

> **Golden E2E documentation**: Review and update `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` as necessary to reflect the new scenario(s) added by this ticket.

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Bribe scenarios
- BlockedIntent for failed threats (E16DPOLPLAN-019)
- Changes to production code

## Acceptance Criteria

### Tests That Must Pass

1. `golden_threaten_with_courage_diversity` — divergent outcomes for B vs C
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Agent diversity (Principle 20): same action, different courage -> different outcomes
2. Conservative threat: missing courage defaults to resist
3. Belief-only planning (Principle 10)

## Test Plan

### New/Modified Tests

1. `golden_offices.rs::golden_threaten_with_courage_diversity`

### Commands

1. `cargo test -p worldwake-ai golden_offices`
2. `cargo test --workspace`
