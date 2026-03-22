# E16DPOLPLAN-014: Golden Scenario 17 — Faction eligibility filters office claim

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: E16DPOLPLAN-007

## Problem

No golden E2E test proves that `EligibilityRule::FactionMember` correctly filters which agents can generate `ClaimOffice` goals through the full AI pipeline. Focused candidate-generation coverage exists, but the full belief -> candidate generation -> planning -> action -> succession path still lacks Scenario 17 coverage from [specs/E16d-political-planning-and-golden-coverage.md](/home/joeloverbeck/projects/worldwake/specs/E16d-political-planning-and-golden-coverage.md).

## Assumption Reassessment (2026-03-18)

1. `EligibilityRule::FactionMember(faction_id)` exists on `OfficeData` — confirmed
2. Candidate generation does check faction membership, but it uses the AI-local `candidate_is_eligible()` in `crates/worldwake-ai/src/candidate_generation.rs`, not the authoritative helper in `crates/worldwake-systems/src/offices.rs` — confirmed
3. Authoritative validation still re-checks eligibility in `crates/worldwake-systems/src/office_actions.rs` and `crates/worldwake-systems/src/offices.rs` — confirmed
4. `FactionData` component and `member_of` relation exist, and the golden harness already provides `seed_faction()` and `add_faction_membership()` helpers — confirmed
5. Focused unit coverage already exists for the positive political candidate path in `crates/worldwake-ai/src/candidate_generation.rs`; the missing coverage is specifically a golden E2E negative case for an ineligible rival — confirmed
6. `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` currently document office scenarios only through Scenario 16, so both docs do need Scenario 17 updates if this ticket is completed — confirmed

## Architecture Check

1. Two-agent test remains the right shape: A (faction member, eligible) vs B (non-member, ineligible)
2. The strongest assertion surface is decision tracing plus action tracing: prove B never generates `ClaimOffice`, not merely that B fails later at execution
3. No production-code change is justified here unless the new golden test exposes a real mismatch between candidate generation and authoritative validation
4. Current architecture is sounder than introducing aliases or shared compatibility shims: AI keeps a belief-view-level eligibility check for planning, while authoritative systems independently validate live world legality. That duplication is acceptable because it enforces the belief/authority boundary and keeps the simulation robust against stale plans

## What to Change

### 1. Add to `golden_offices.rs`

- **Setup**: Vacant office with `EligibilityRule::FactionMember(faction_x)`. Agent A is member of `faction_x` (eligible). Agent B is NOT a member (ineligible). Both at jurisdiction, sated, `enterprise_weight > 0`.
- **Expected**: A generates `ClaimOffice`, B does NOT. A declares and gets installed. B never generates a `ClaimOffice` goal.
- **Assertions**: A is office holder. Decision traces show A generates `ClaimOffice` and B never does across the observed window. Action traces show B never commits `declare_support`.

> **Golden E2E documentation**: Review and update `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` as necessary to reflect the new scenario(s) added by this ticket.

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Other eligibility rules (e.g., AllAgents)
- Faction creation/dissolution dynamics
- Changes to eligibility logic
- Refactoring AI and authoritative eligibility helpers into a shared abstraction unless the golden test proves a real divergence
- Changes to production code unless the golden test exposes a defect

## Acceptance Criteria

### Tests That Must Pass

1. `golden_faction_eligibility_filters_office_claim` — A installed, B never attempted
2. Relevant trace assertions prove B never generated `ClaimOffice` during the scenario
3. Existing focused coverage still passes
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Ineligible agents never generate political goals for restricted offices
2. Faction membership is checked during candidate generation and revalidated authoritatively at action execution / succession resolution
3. The golden test must prove the candidate-generation filter, not only downstream rejection

## Test Plan

### New/Modified Tests

1. `golden_offices.rs::golden_faction_eligibility_filters_office_claim`

### Commands

1. `cargo test -p worldwake-ai golden_offices`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-18
- **What actually changed**:
  - Added `golden_faction_eligibility_filters_office_claim` to `crates/worldwake-ai/tests/golden_offices.rs`
  - Strengthened the scenario to assert the real architectural invariant with decision traces (`ClaimOffice` generated for the eligible agent, never generated for the ineligible agent) plus action traces (`declare_support` never committed by the ineligible agent)
  - Updated `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` to include Scenario 17
  - Corrected this ticket's assumptions and scope before implementation so it reflects the current split between AI-side belief filtering and authoritative validation
- **Deviations from original plan**:
  - No production code changes were needed; the current architecture already cleanly separates belief-side candidate filtering from authoritative execution-time validation
  - The final assertions use traces instead of relying on event-log absence, because traces prove the candidate-generation-stage invariant more directly and robustly
- **Verification results**:
  - `cargo test -p worldwake-ai golden_offices` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
