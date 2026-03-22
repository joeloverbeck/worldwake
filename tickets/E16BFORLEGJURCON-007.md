# E16BFORLEGJURCON-007: Add force-claim affordance enumeration

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — affordance query in worldwake-sim or worldwake-systems
**Deps**: E16BFORLEGJURCON-003, E16BFORLEGJURCON-004, E16BFORLEGJURCON-006

## Problem

AI agents need to discover `PressForceClaim` and `YieldForceClaim` as available actions through the affordance system. Without affordance enumeration, the planner cannot include force-claim actions in plan search.

## Assumption Reassessment (2026-03-22)

1. Affordance enumeration functions exist for other political actions (e.g., `enumerate_declare_support_payloads`, `enumerate_bribe_payloads`). No force-claim enumeration functions exist.
2. `get_affordances()` in `affordance_query.rs` aggregates payloads from all enumeration functions. It must include the new force-claim enumerators.
3. `RuntimeBeliefView` provides belief-level checks for eligibility and succession law. `believed_force_controller()` will exist from ticket -006.
4. `contests_office` relation (from ticket -002) provides the authoritative check for whether an agent already contests an office.
5. N/A — not an AI regression, ordering, or heuristic ticket.
6. N/A — no heuristic removal.
7. N/A — not a start-failure ticket.
8. N/A — not a political closure ticket.
9. N/A — no ControlSource manipulation.
10. N/A — no golden scenario.
11. No mismatches found.
12. N/A — no cumulative arithmetic.

## Architecture Check

1. Follows the existing affordance enumeration pattern exactly: function takes `(actor, world, belief_view)` and returns `Vec<ActionPayload>`. Uses `RuntimeBeliefView` for belief-level checks (succession law, eligibility) and authoritative state for `contests_office` membership.
2. No backward-compatibility shims.

## Verification Layers

1. `enumerate_press_force_claim_payloads` returns payload at correct location → focused test
2. Returns empty when not at jurisdiction → focused test
3. Returns empty when not eligible → focused test
4. Returns empty when already contesting → focused test
5. `enumerate_yield_force_claim_payloads` returns payload for active claims → focused test
6. Returns empty when no active claims → focused test
7. Single-layer ticket (affordance enumeration). Planner ops are ticket -008.

## What to Change

### 1. Add `enumerate_press_force_claim_payloads`

Returns `ActionPayload::PressForceClaim` for each force-succession office the actor is eligible for at their current location and does not already contest. Uses `RuntimeBeliefView` to check believed succession law and eligibility.

### 2. Add `enumerate_yield_force_claim_payloads`

Returns `ActionPayload::YieldForceClaim` for each office the actor currently contests. Uses authoritative `contests_office` check.

### 3. Wire into `get_affordances()`

Add calls to both enumeration functions in the affordance aggregation path.

## Files to Touch

- `crates/worldwake-sim/src/affordance_query.rs` or `crates/worldwake-systems/src/office_actions.rs` (modify — add enumeration functions, depending on where existing political affordance enumerators live)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — wire into `get_affordances()`)

## Out of Scope

- Planner op semantics — E16BFORLEGJURCON-008
- Candidate generation changes — E16BFORLEGJURCON-008
- Golden tests — E16BFORLEGJURCON-009
- Force control system — E16BFORLEGJURCON-005
- Belief query methods — E16BFORLEGJURCON-006

## Acceptance Criteria

### Tests That Must Pass

1. Eligible agent at force-office jurisdiction sees `PressForceClaim` in affordances
2. Agent not at jurisdiction does NOT see `PressForceClaim`
3. Ineligible agent does NOT see `PressForceClaim`
4. Agent already contesting does NOT see `PressForceClaim`
5. Agent contesting an office sees `YieldForceClaim` in affordances
6. Agent not contesting any office does NOT see `YieldForceClaim`
7. Existing suite: `cargo test -p worldwake-sim` (or `-p worldwake-systems` depending on location)

### Invariants

1. Affordance enumeration uses `RuntimeBeliefView` for belief-level checks (not omniscient reads)
2. `contests_office` check is authoritative (agent's own claim membership is factual, not belief-based)
3. No existing tests break

## Test Plan

### New/Modified Tests

1. Affordance enumeration test module — focused tests for both `enumerate_press_force_claim_payloads` and `enumerate_yield_force_claim_payloads`

### Commands

1. `cargo test -p worldwake-sim` (or `cargo test -p worldwake-systems`)
2. `cargo clippy --workspace`
3. `cargo test --workspace`
