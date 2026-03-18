# E16DPOLPLAN-009: Golden Scenario 12 — Competing claims with loyal supporter

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E16DPOLPLAN-007

## Problem

No golden test covers multi-agent political competition where loyalty-driven support from a third agent determines the outcome.

## Assumption Reassessment (2026-03-18)

1. `GoalKind::SupportCandidateForOffice` exists in candidate_generation — confirmed
2. Loyalty relation drives `SupportCandidateForOffice` goal generation — confirmed
3. Support counting in `succession_system()` compares declarations per candidate — confirmed
4. `social_weight` in `UtilityProfile` enables support candidate generation — confirmed

## Architecture Check

1. Multi-agent test with 3 agents: A (claimant), B (claimant), C (loyal supporter of A)
2. Tests emergent multi-agent coordination through independent AI decisions

## What to Change

### 1. Add to `golden_offices.rs`

- **Setup**: Vacant office. Agents A and B both eligible, both `enterprise_weight > 0`. Agent C has loyalty to A, `social_weight > 0`. All at jurisdiction, all sated.
- **Expected**: A declares for self, B declares for self, C generates `SupportCandidateForOffice(A)` and declares for A. A gets 2 declarations (self + C), B gets 1. Politics system installs A.
- **Assertions**: Office holder == A. C's support_declaration for office == A.

> **Golden E2E documentation**: Review and update `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` as necessary to reflect the new scenario(s) added by this ticket.

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Bribe/Threaten scenarios
- Faction eligibility filtering
- Force succession law
- Changes to production code

## Acceptance Criteria

### Tests That Must Pass

1. `golden_competing_claims_with_loyal_supporter` — A installed, C supported A
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Support counting is deterministic
2. Loyalty-driven support is emergent (C's AI autonomously decides to support A)
3. No agent reads world state directly

## Test Plan

### New/Modified Tests

1. `golden_offices.rs::golden_competing_claims_with_loyal_supporter`

### Commands

1. `cargo test -p worldwake-ai golden_offices`
2. `cargo test --workspace`
