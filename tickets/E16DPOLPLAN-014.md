# E16DPOLPLAN-014: Golden Scenario 17 — Faction eligibility filters office claim

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: E16DPOLPLAN-007

## Problem

No golden test proves that `EligibilityRule::FactionMember` correctly filters which agents can generate `ClaimOffice` goals.

## Assumption Reassessment (2026-03-18)

1. `EligibilityRule::FactionMember(faction_id)` exists on `OfficeData` — confirmed
2. `candidate_is_eligible()` checks faction membership — confirmed
3. `emit_political_candidates` in candidate_generation.rs uses eligibility check — confirmed
4. `FactionData` component and `member_of` relation exist — confirmed from E16

## Architecture Check

1. Two-agent test: A (faction member, eligible) vs B (non-member, ineligible)
2. Tests negative case: B should NEVER generate ClaimOffice, not just fail at execution

## What to Change

### 1. Add to `golden_offices.rs`

- **Setup**: Vacant office with `EligibilityRule::FactionMember(faction_x)`. Agent A is member of `faction_x` (eligible). Agent B is NOT a member (ineligible). Both at jurisdiction, sated, `enterprise_weight > 0`.
- **Expected**: A generates `ClaimOffice`, B does NOT. A declares and gets installed. B never generates a `ClaimOffice` goal.
- **Assertions**: A is office holder. B never executed a `DeclareSupport` action. Event log shows no ClaimOffice-related events from B.

> **Golden E2E documentation**: Review and update `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` as necessary to reflect the new scenario(s) added by this ticket.

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Other eligibility rules (e.g., AllAgents)
- Faction creation/dissolution dynamics
- Changes to eligibility logic
- Changes to production code

## Acceptance Criteria

### Tests That Must Pass

1. `golden_faction_eligibility_filters_office_claim` — A installed, B never attempted
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Ineligible agents never generate political goals for restricted offices
2. Faction membership is checked at candidate generation, not at action execution
3. No ClaimOffice events from ineligible agent in event log

## Test Plan

### New/Modified Tests

1. `golden_offices.rs::golden_faction_eligibility_filters_office_claim`

### Commands

1. `cargo test -p worldwake-ai golden_offices`
2. `cargo test --workspace`
