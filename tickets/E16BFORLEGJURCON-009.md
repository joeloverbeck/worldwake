# E16BFORLEGJURCON-009: Force legitimacy golden E2E tests

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None — test-only ticket
**Deps**: E16BFORLEGJURCON-001 through E16BFORLEGJURCON-008 (all prior tickets)

## Problem

The full force-legitimacy lifecycle — from claim to contest to control to installation, with belief propagation, departure penalties, and AI-driven claiming — needs golden E2E coverage to prove the system works end-to-end and to catch regressions.

## Assumption Reassessment (2026-03-22)

1. Existing golden political tests in `crates/worldwake-ai/tests/golden_political.rs` (or similar) cover support-law offices. No golden tests cover force-law offices.
2. The force-control lifecycle state machine, action handlers, institutional beliefs, affordances, and planner ops from tickets -001 through -008 must all be integrated before these goldens can run.
3. Golden test harness patterns are well-established: `GoldenTestHarness` with `step_once()`, decision tracing, action tracing, and deterministic replay.
4. N/A — AI regression layer: golden E2E coverage.
5. N/A — ordering is tick-level system execution.
6. N/A — no heuristic removal.
7. N/A — not a start-failure ticket.
8. Closure boundaries tested: (a) `PressForceClaim` commit → `contests_office` mutation, (b) control system per-tick → `office_controller` mutation, (c) installation gate → `office_holder` mutation. All three authoritative boundaries are covered.
9. N/A — no ControlSource manipulation beyond standard AI control.
10. Golden scenarios isolate force-law behavior: use `SuccessionLaw::Force` offices exclusively, with eligible agents positioned for claim/contest/control scenarios.
11. No mismatches found.
12. Installation arithmetic: `control_since + uncontested_hold_ticks <= current_tick`. Scenarios set `uncontested_hold_ticks` to small values (e.g., 3-5 ticks) for test tractability.

## Architecture Check

1. Golden tests prove the full emergence chain: AI candidate generation → plan search → action execution → state transition → belief propagation → downstream response. This is the highest-value coverage for an integrated political system.
2. No backward-compatibility shims.

## Verification Layers

1. Claim → contest → control → installation lifecycle → golden E2E (world state + event log)
2. Belief propagation of force-control state → golden E2E (witness beliefs after events)
3. Remote agent ignorance of coup without Tell → golden E2E (belief isolation)
4. AI-driven force claiming → golden E2E (decision trace shows `ClaimOffice` candidate → `PressForceClaim` plan)
5. Departure penalty (control clock reset) → golden E2E (control_since resets after return)
6. Contested office blocks installation → golden E2E (multiple claimants → no installation)
7. Deterministic replay companion for each scenario

## What to Change

### 1. Add golden test scenarios

Minimum scenarios from the spec's test list:

**Scenario A: Uncontested force installation**
- Single eligible agent at force office jurisdiction
- Agent presses force claim (AI-driven or manual input)
- Agent remains sole controller for `uncontested_hold_ticks`
- Agent is installed as `office_holder`
- Verify installation event, belief projection, register entry

**Scenario B: Contested force office**
- Two eligible agents at force office jurisdiction
- Both press force claims
- Office becomes contested, no controller, no installation
- One yields → other becomes controller
- After hold period → installation

**Scenario C: Departure penalty**
- Agent presses claim, becomes controller
- Agent leaves jurisdiction (travel action)
- Control is immediately cleared
- Agent returns → control clock restarts from zero

**Scenario D: Hostility aftermath**
- Agent presses force claim against incumbent holder
- Verify `hostile_to(claimant, holder)` relation created
- Verify hostility persists after resolution

**Scenario E: Belief propagation**
- Agent A presses claim at jurisdiction
- Agent B at same place witnesses event
- Verify B has `ForceControllerOf` institutional belief
- Agent C at remote location does NOT have the belief
- Agent B tells C → C acquires belief through Tell

**Scenario F: AI-driven force claim**
- AI agent believes a force-succession office is vacant and they are eligible
- Decision trace shows `ClaimOffice` candidate → `PressForceClaim` plan
- Agent travels to jurisdiction and executes force claim

### 2. Deterministic replay companions

Each golden scenario must have a deterministic replay companion that reproduces the same outcome from the same seed.

## Files to Touch

- `crates/worldwake-ai/tests/golden_political.rs` (modify — add force-law golden scenarios)
- Possibly a new `golden_force_legitimacy.rs` if the file would become too large

## Out of Scope

- Implementation of any force-legitimacy feature (all must be done in -001 through -008)
- Guard responses to coups — deferred to E19
- Public order impact — deferred to E19
- Support-law golden tests (already covered by E16d)

## Acceptance Criteria

### Tests That Must Pass

1. Golden: uncontested force installation lifecycle (claim → control → hold → install)
2. Golden: contested office blocks installation until contest resolves
3. Golden: departure immediately clears control, return restarts clock
4. Golden: pressing claim against incumbent creates hostility
5. Golden: force-control events project into witness institutional beliefs
6. Golden: remote agents do not learn contest outcomes without Tell
7. Golden: AI agent generates ClaimOffice candidate for eligible vacant force office
8. Deterministic replay companions for all scenarios
9. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. All golden scenarios use force-succession offices exclusively (no support-law confusion)
2. All agent knowledge acquisition follows Principle 7 (locality)
3. All state transitions follow the lifecycle state machine from the spec
4. Deterministic replay produces identical outcomes
5. No existing golden tests break

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_political.rs` or `golden_force_legitimacy.rs` — 6+ golden scenarios with replay companions

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
