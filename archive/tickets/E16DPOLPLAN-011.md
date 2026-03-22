# E16DPOLPLAN-011: Golden Scenario 14 — Threaten with courage diversity

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None (but depends on E16DPOLPLAN-028 which has engine changes)
**Deps**: E16DPOLPLAN-004, E16DPOLPLAN-007, E16DPOLPLAN-022, E16DPOLPLAN-023, E16DPOLPLAN-024, E16DPOLPLAN-025, E16DPOLPLAN-028

## Problem

No golden test covers the threaten political path with diverse courage values producing different outcomes per target (Principle 20 — agent diversity).

## Assumption Reassessment (2026-03-18)

1. Threaten planning arm (E16DPOLPLAN-004) compares attack_skill vs courage — confirmed dependency
2. `commit_threaten` does loyalty increase on yield, hostility on resist — confirmed
3. `CombatProfile.attack_skill` accessible via snapshot — confirmed
4. `UtilityProfile.courage` varies per agent — confirmed
5. **BLOCKER DISCOVERED**: `PerAgentBeliefView::courage()` only returns courage for self (the planning agent). For other agents it returns `None`, which defaults to `pm(1000)` in `apply_threaten_for_office`, making the planner always evaluate Threaten as "resist." The belief pipeline (`ObservedEntitySnapshot` / `BelievedEntityState`) does not capture courage. **Fix**: E16DPOLPLAN-028 adds courage to the belief observation pipeline, following the same pattern as wounds.

## Architecture Check

1. Tests Principle 20 (agent diversity): same action produces different outcomes based on per-agent parameters
2. Two targets with different courage values demonstrate divergent behavioral outcomes

## What to Change

### 1. Add to `golden_offices.rs`

- **Setup**: Vacant office. Agent A (high `attack_skill=pm(800)`, `enterprise_weight=pm(900)`). Agent B (`courage=pm(200)`, should yield). Agent C (`courage=pm(900)`, should resist). **Agent D (competitor)** at ORCHARD_FARM (not co-located — prevents planner from targeting D with Threaten since D's default courage=pm(500) < 800 would make D a viable target, diverting from the intended B/C courage diversity test). D has already self-declared support for own office claim. A, B, C at jurisdiction, sated. The competitor ensures DeclareSupport alone from A would produce a tie, motivating the planner to select Threaten to build a winning coalition.
- **Expected**: A generates `ClaimOffice`. Planner finds `Threaten(B)` viable (800 > 200) but not `Threaten(C)` (800 < 900) because DeclareSupport alone ties with competitor D. A threatens B -> B yields -> loyalty increase. A declares for self. B may support A. C does not. A's coalition exceeds D's.
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

## Dependency Chain Note

This ticket depends on the coalition-aware planner changes from E16DPOLPLAN-022 through E16DPOLPLAN-025. The competitor agent setup is required because the coalition-aware planner (E16DPOLPLAN-024) now produces `GoalSatisfied` for uncontested DeclareSupport. Without a competitor, the planner would never select Threaten — it would just DeclareSupport and succeed immediately. The competitor creates the contested scenario where Threaten is the rational choice for building a winning coalition.

## Outcome

**Completion date**: 2026-03-18

**What changed**:
- `crates/worldwake-ai/tests/golden_offices.rs`: Removed `#[ignore]` from `golden_threaten_with_courage_diversity`. Moved Agent D from `VILLAGE_SQUARE` to `ORCHARD_FARM`.
- `docs/golden-e2e-coverage.md`: Added Scenario 14 to file layout, cross-system chains, and summary statistics (91→92 tests, 51→52 chains).
- `docs/golden-e2e-scenarios.md`: Added Scenario 14 catalog entry (92 tests).

**Deviations from original plan**: Agent D moved from `VILLAGE_SQUARE` to `ORCHARD_FARM` (same pattern as Scenario 13's bribe test). D's default `courage=pm(500)` was below A's `attack_skill=pm(800)`, making D a viable and preferred threaten target over B. Placing D at a different location prevents co-location-based Threaten targeting while D's self-support declaration still counts for succession (relation-based, not positional).

**Verification**: `cargo test -p worldwake-ai` all pass, `cargo clippy --workspace` clean, `cargo test --workspace` all pass.
