# E16BFORLEGJURCON-008: Add PressForceClaim and YieldForceClaim planner ops + candidate generation wiring

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — planner ops (ai), candidate generation (ai)
**Deps**: E16BFORLEGJURCON-007, E16BFORLEGJURCON-006

## Problem

AI agents need planner op semantics for `PressForceClaim` and `YieldForceClaim` so the GOAP planner can include these actions in plan search for `GoalKind::ClaimOffice`. Additionally, candidate generation must ensure `ClaimOffice` candidates are emitted for force-succession offices when the agent believes the office is vacant or held by an enemy.

## Assumption Reassessment (2026-03-22)

1. `PlannerOpKind` enum in `planner_ops.rs` contains 17 variants including `DeclareSupport`, `Bribe`, `Threaten`. `PressForceClaim` and `YieldForceClaim` do not exist.
2. `PlannerOpSemantics` entries define goal relevance, terminal conditions, barriers, and mid-plan viability for each op. These must be added for both new ops.
3. `GoalKind::ClaimOffice` and `GoalKindTag::ClaimOffice` already exist. No new goal kind is needed.
4. `candidate_generation.rs` already generates `ClaimOffice` candidates for political scenarios. The spec says to ensure force-succession offices also trigger `ClaimOffice` candidates when appropriate (vacant or enemy-held force office where agent is eligible).
5. The planner discovers `PressForceClaim` as a valid action through affordance binding (ticket -007). The planner op semantics tell the search engine how to treat this action.
6. N/A — not a heuristic removal ticket.
7. N/A — not a start-failure ticket.
8. Closure boundary: `PressForceClaim` planner op maps to `GoalKindTag::ClaimOffice`. Terminal condition is "actor has pressed claim and is sole controller" (belief-level check via `believed_force_controller`). Barrier is "not at jurisdiction" (triggers travel prerequisite). The planner op is AI-layer only.
9. N/A — no ControlSource manipulation.
10. N/A — no golden scenario (that's -009).
11. No mismatches found. Existing `ClaimOffice` candidate generation may need minor extension to also emit for force-law offices, depending on current filtering.
12. N/A — no cumulative arithmetic.

## Architecture Check

1. Follows the existing planner op pattern exactly. `PressForceClaim` maps to `ClaimOffice` goal tag, `YieldForceClaim` has no direct goal relevance (used in retreat/replan). Terminal condition uses `believed_force_controller()` from ticket -006.
2. No backward-compatibility shims. No new goal kind needed.

## Verification Layers

1. `PlannerOpKind::PressForceClaim` maps to `GoalKindTag::ClaimOffice` → focused test on semantics
2. Terminal condition checks belief-level force controller status → focused test
3. Barrier detects "not at jurisdiction" → focused test on barrier logic
4. Mid-plan viability checks agent still believes force succession → focused test
5. `PlannerOpKind::YieldForceClaim` terminal: agent no longer contests → focused test
6. `ClaimOffice` candidates emit for force-succession offices → focused candidate generation test

## What to Change

### 1. Add `PlannerOpKind::PressForceClaim`

Semantics:
- **Goal relevance**: `GoalKindTag::ClaimOffice`
- **Terminal condition**: actor has pressed claim and `believed_force_controller()` returns `(Some(actor), false)` for the target office
- **Barriers**: not at jurisdiction (triggers travel prerequisite via existing place-guidance)
- **Mid-plan viability**: agent still believes the office uses force succession

### 2. Add `PlannerOpKind::YieldForceClaim`

Semantics:
- **Goal relevance**: none directly (retreat/replan utility)
- **Terminal condition**: agent no longer contests the office (authoritative check)

### 3. Ensure candidate generation covers force offices

Verify/extend `candidate_generation.rs` so that when an agent believes a force-succession office is vacant or held by an enemy, and the agent is eligible, a `ClaimOffice { office }` candidate is generated. The planner then discovers `PressForceClaim` through affordance binding.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — add 2 variants + semantics)
- `crates/worldwake-ai/src/goal_model.rs` (modify — map `PressForceClaim` to `ClaimOffice` tag)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — ensure force-office `ClaimOffice` candidates)
- `crates/worldwake-ai/src/search.rs` (verify — no changes needed if affordance binding works correctly)

## Out of Scope

- Affordance enumeration — E16BFORLEGJURCON-007 (must be done first)
- Institutional belief queries — E16BFORLEGJURCON-006
- Force control system — E16BFORLEGJURCON-005
- Golden E2E tests — E16BFORLEGJURCON-009
- Action handlers — E16BFORLEGJURCON-004

## Acceptance Criteria

### Tests That Must Pass

1. `PlannerOpKind::PressForceClaim` semantics return `GoalKindTag::ClaimOffice` as relevant goal
2. Terminal condition: agent believed to be sole uncontested controller returns true
3. Barrier: agent not at jurisdiction triggers travel prerequisite
4. Mid-plan viability: agent believes office no longer uses force succession → invalid
5. `PlannerOpKind::YieldForceClaim` terminal: agent no longer contests returns true
6. `ClaimOffice` candidate generated for eligible agent at vacant force office
7. `ClaimOffice` candidate generated for eligible agent at enemy-held force office
8. No `ClaimOffice` candidate for ineligible agent or non-force office
9. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `PressForceClaim` planner op uses belief-level checks only (Principle 12)
2. No new `GoalKind` variant introduced — `ClaimOffice` covers both succession laws
3. Candidate generation uses `GoalBeliefView` for office belief checks
4. No existing tests break

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` test module — semantics tests for both new ops
2. `crates/worldwake-ai/src/candidate_generation.rs` test module — force-office candidate generation tests

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
