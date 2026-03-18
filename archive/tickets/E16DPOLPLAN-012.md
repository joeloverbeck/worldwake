# E16DPOLPLAN-012: Golden Scenario 15 — Travel to distant jurisdiction for office claim

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — planner locality guards for political actions (see below)
**Deps**: E16DPOLPLAN-007

## Problem

No golden test covers multi-hop travel as a political planning step: agent at remote location plans Travel + DeclareSupport to claim a distant office.

## Assumption Reassessment (2026-03-18)

1. `PlannerOpKind::Travel` in `CLAIM_OFFICE_OPS` enables travel as political plan step — confirmed
2. Multi-hop travel planning already proven in golden_production.rs (Scenario 3b) — confirmed as mechanism, but not for political goals
3. `build_prototype_world` topology has BanditCamp 3 hops from VillageSquare (BanditCamp → ForestPath [5t] → NorthCrossroads [4t] → VillageSquare [3t] = 12 travel ticks) — corrected from original "4 hops" claim
4. Agents need beliefs about the vacant office to generate `ClaimOffice` — confirmed

## Architecture Check

1. Reuses proven travel planning mechanism for a new domain (politics)
2. Tests cross-system interaction: political goal -> travel planning -> sequential execution -> DeclareSupport

## What to Change

### 1. Add to `golden_offices.rs`

- **Setup**: Vacant office at VillageSquare. Eligible agent starts at BanditCamp (3 hops / 12 travel ticks). Agent has beliefs about the vacant office. Sated, `enterprise_weight=pm(800)`.
- **Expected**: Agent generates `ClaimOffice` -> plans multi-hop Travel + DeclareSupport -> traverses route -> arrives -> declares support -> installed after succession period.
- **Assertions**: Agent ends at VillageSquare. Office holder == agent.

> **Golden E2E documentation**: Review and update `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` as necessary to reflect the new scenario(s) added by this ticket.

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Discovered Defect — Planner Locality Guards (Principle 7)

Implementing this test revealed that `apply_planner_step` in `goal_model.rs` did not enforce co-location for any political social action (`DeclareSupport`, `Bribe`, `Threaten`). The planner would simulate these actions succeeding from any distance, producing 1-step plans that failed on execution. This violated Principle 7 (Locality) and Principle 8 (Preconditions).

**Fix applied**:
1. Added `jurisdiction: Option<EntityId>` to `SnapshotEntity` in `planning_snapshot.rs`, populated from `OfficeData.jurisdiction` via the belief view.
2. Added `actor_at_jurisdiction()` helper in `goal_model.rs` — checks if actor's planned position matches the office's jurisdiction.
3. Guarded all three political action arms (`DeclareSupport`, `Bribe`, `Threaten`) in `apply_planner_step` with `actor_at_jurisdiction()`. If the actor is not at the jurisdiction, the step returns state unchanged and the planner must find a Travel step first.
4. Updated all affected unit tests to provide `OfficeData` with jurisdiction.

**Files touched (production)**:
- `crates/worldwake-ai/src/goal_model.rs` — locality guards + `actor_at_jurisdiction()` helper
- `crates/worldwake-ai/src/planning_snapshot.rs` — `jurisdiction` field on `SnapshotEntity` + accessor

## Out of Scope

- Bribe/Threaten scenarios
- Multi-agent competition at destination
- Changes to travel planning logic
- ~~Changes to production code~~ (overridden: planner locality defect required production fix)

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

## Outcome

**Completion date**: 2026-03-18

**What changed**:
- Added `golden_travel_to_distant_jurisdiction_for_claim` test (Scenario 15) to `golden_offices.rs`
- Fixed planner Principle 7 (Locality) defect: added `jurisdiction` field to `SnapshotEntity`, `actor_at_jurisdiction()` guard to all three political action arms in `apply_planner_step`
- Updated `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md`

**Deviations from original plan**:
- Ticket originally specified "Engine Changes: None" and "Out of Scope: Changes to production code". Implementation revealed a planner defect (Principle 7 violation) that required production fixes to `goal_model.rs` and `planning_snapshot.rs`. The defect affected all political social actions, not just the travel scenario.
- Corrected "4 hops" assumption to "3 hops" (shortest path via NorthCrossroads direct edge).

**Verification**: `cargo test --workspace` — all tests pass (28 suites). `cargo clippy --workspace` — clean.
