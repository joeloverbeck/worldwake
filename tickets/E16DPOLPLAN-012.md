# E16DPOLPLAN-012: Golden Scenario 15 — Travel to distant jurisdiction for office claim

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: E16DPOLPLAN-007

## Problem

No golden test covers multi-hop travel as a political planning step: agent at remote location plans Travel + DeclareSupport to claim a distant office.

## Assumption Reassessment (2026-03-18)

1. `PlannerOpKind::Travel` in `CLAIM_OFFICE_OPS` enables travel as political plan step — confirmed
2. Multi-hop travel planning already proven in golden_production.rs (Scenario 3b) — confirmed as mechanism, but not for political goals
3. `build_prototype_world` topology has BanditCamp 4 hops from VillageSquare — confirmed
4. Agents need beliefs about the vacant office to generate `ClaimOffice` — confirmed

## Architecture Check

1. Reuses proven travel planning mechanism for a new domain (politics)
2. Tests cross-system interaction: political goal -> travel planning -> sequential execution -> DeclareSupport

## What to Change

### 1. Add to `golden_offices.rs`

- **Setup**: Vacant office at VillageSquare. Eligible agent starts at BanditCamp (4 hops). Agent has beliefs about the vacant office. Sated, `enterprise_weight=pm(800)`.
- **Expected**: Agent generates `ClaimOffice` -> plans multi-hop Travel + DeclareSupport -> traverses route -> arrives -> declares support -> installed after succession period.
- **Assertions**: Agent ends at VillageSquare. Office holder == agent.

> **Golden E2E documentation**: Review and update `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` as necessary to reflect the new scenario(s) added by this ticket.

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Bribe/Threaten scenarios
- Multi-agent competition at destination
- Changes to travel planning logic
- Changes to production code

## Acceptance Criteria

### Tests That Must Pass

1. `golden_travel_to_distant_jurisdiction_for_claim` — agent at VillageSquare, installed as holder
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Agent traverses correct multi-hop route
2. DeclareSupport only after arrival at jurisdiction
3. Belief-only planning (Principle 10)

## Test Plan

### New/Modified Tests

1. `golden_offices.rs::golden_travel_to_distant_jurisdiction_for_claim`

### Commands

1. `cargo test -p worldwake-ai golden_offices`
2. `cargo test --workspace`
