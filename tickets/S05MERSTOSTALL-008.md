# S05MERSTOSTALL-008: Add facility control authorization and theft distinction

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — authorization checks, E17 crime model interaction
**Deps**: S05MERSTOSTALL-005

## Problem

The system must distinguish lawful facility access (by the facility controller) from theft of displayed or stored goods (by unauthorized agents). Without this distinction, any agent could manipulate facility containers without consequence.

## Assumption Reassessment (2026-04-01)

1. Stock actions (003/004) have basic authorization checks — but theft classification via E17 pipeline is not yet wired.
2. E17 crime model pipeline exists — check whether `SuspectedTheft` event type and investigation flow are implemented or specced.
3. Displayed goods in display containers are perceptible to agents at the same place — check perception model for container contents visibility.
4. Facility control derives from an existing mechanism — check `can_exercise_control` or equivalent for facility entities.
5. Candidate generation for theft must include displayed lots as targets — check current theft candidate generation scope.

## Architecture Check

1. Authorization strengthening in stock actions follows the existing control-check pattern. Theft classification reuses the E17 crime pipeline rather than introducing a parallel enforcement mechanism — single path for all property violations.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Authorized controller succeeds at stock actions → action trace (focused test)
2. Unauthorized agent rejected by stock actions → action trace (focused test)
3. Unauthorized container access classified as theft → event-log delta (focused test)
4. Displayed goods appear as theft targets in candidate generation → candidate generation test
5. `SuspectedTheft` produced for unauthorized access → event-log delta (focused test)

## What to Change

### 1. Strengthen authorization in stock actions

In `stock_actions.rs`: ensure all stock actions (store, collect, stage, unstage) verify the agent is the facility controller via `can_exercise_control` or equivalent. Rejection produces a distinct error, not silent failure.

### 2. Classify unauthorized container access as theft

Wire unauthorized container access attempts through the E17 crime pipeline to produce `SuspectedTheft` events.

### 3. Update theft candidate generation

In `candidate_generation.rs`: displayed lots in facility display containers are valid theft targets for non-controller agents.

## Files to Touch

- `crates/worldwake-systems/src/stock_actions.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-systems/src/crime_actions.rs` or equivalent (modify)

## Out of Scope

- Audit hooks for stock inspection (009)
- AI planning for stock actions (007)
- Golden tests (010)

## Acceptance Criteria

### Tests That Must Pass

1. Authorized facility controller succeeds at all stock actions
2. Unauthorized agent rejected by stock actions with distinct error
3. Displayed goods appear as theft targets for non-controllers
4. `SuspectedTheft` event produced for unauthorized container access
5. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Only the facility controller can lawfully manipulate facility containers
2. Unauthorized access follows the E17 crime pipeline — no parallel enforcement
3. System decoupling — crime classification reuses existing pipeline

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/stock_actions.rs` — authorized controller succeeds, unauthorized rejected
2. `crates/worldwake-systems/src/stock_actions.rs` — unauthorized access produces SuspectedTheft
3. `crates/worldwake-ai/src/candidate_generation.rs` — displayed lots as theft targets

### Commands

1. `cargo test -p worldwake-systems -- stock`
2. `cargo test -p worldwake-systems -- theft`
3. `cargo test -p worldwake-ai -- theft`
4. `cargo clippy --workspace --all-targets -- -D warnings`
