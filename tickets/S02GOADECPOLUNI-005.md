# S02GOADECPOLUNI-005: Wire DecisionContext construction in agent_tick and thread to ranking + interrupts

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — agent_tick.rs wiring
**Deps**: S02GOADECPOLUNI-002, S02GOADECPOLUNI-003

## Problem

After tickets 002 and 003, both `rank_candidates()` and `evaluate_interrupt()` accept a `&DecisionContext`. However, each call site may be constructing it independently or using a placeholder. The spec requires `DecisionContext` to be built **once** per agent decision pass and threaded to both consumers.

## Assumption Reassessment (2026-03-16)

1. `rank_candidates()` is called at `agent_tick.rs:506` inside the agent tick pipeline — confirmed.
2. `evaluate_interrupt()` is called at `agent_tick.rs:604` inside `handle_active_action_phase()` — confirmed.
3. Both calls happen in the same per-agent tick scope, with access to the same `view` (GoalBeliefView), `agent` (EntityId), and belief state — confirmed.
4. `DecisionContext` needs `max_self_care_class` and `danger_class`, both derivable from `HomeostaticNeeds`, `DriveThresholds`, and `derive_danger_pressure()` — confirmed from ranking.rs:74-99.
5. The agent tick function `run_agent_decision_phase()` has access to the belief view and agent ID — confirmed.

## Architecture Check

1. Building `DecisionContext` once in the agent tick's decision phase and passing it down to both `rank_candidates()` and `evaluate_interrupt()` eliminates any possibility of the two computing divergent pressure states.
2. No backwards-compatibility shims. The temporary placeholder constructions from tickets 002/003 are replaced with the proper single-construction site.

## What to Change

### 1. Build `DecisionContext` in agent tick decision phase

In the function that calls `rank_candidates()` (around line 506 in agent_tick.rs), construct:
```rust
let decision_context = DecisionContext {
    max_self_care_class: /* derive from view, agent, needs, thresholds */,
    danger_class: /* derive from view, agent, thresholds, danger_pressure */,
};
```

Use the same derivation logic currently in `RankingContext::max_self_care_class()` and `RankingContext::danger_class()`. Consider extracting a `DecisionContext::from_beliefs(view, agent)` constructor to keep agent_tick clean.

### 2. Pass `&decision_context` to `rank_candidates()`

Update the call site to pass the shared context.

### 3. Pass `&decision_context` to `evaluate_interrupt()`

Update the call in `handle_active_action_phase()` to pass the same shared context. Since `handle_active_action_phase` is called after ranking, the `DecisionContext` must be threaded through or stored in a local variable accessible to both.

### 4. Remove any temporary placeholder constructions

If tickets 002/003 introduced temporary `DecisionContext` construction at call sites, replace them with the single authoritative construction.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify)
- `crates/worldwake-ai/src/goal_policy.rs` (modify — add `DecisionContext::from_beliefs()` constructor if helpful)

## Out of Scope

- Modifying ranking logic beyond call-site parameter passing
- Modifying interrupt logic beyond call-site parameter passing
- Adding new goal families
- Changes to `worldwake-core` or `worldwake-sim`
- Modifying `ranking.rs` or `interrupts.rs` (already done in tickets 002-004)

## Acceptance Criteria

### Tests That Must Pass

1. `DecisionContext` is constructed exactly once per agent decision tick, not separately in ranking and interrupts
2. Both `rank_candidates()` and `evaluate_interrupt()` receive the same `DecisionContext` instance
3. All existing golden tests pass — behavioral equivalence confirmed end-to-end
4. All existing unit tests pass: `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`

### Invariants

1. `DecisionContext` is built from belief-facing reads only (via `GoalBeliefView`), never from authoritative world state
2. Single construction site prevents divergent pressure classification between ranking and interrupts
3. No compatibility wrappers or dual-path policy evaluation

## Test Plan

### New/Modified Tests

1. No new unit tests required (behavioral equivalence verified by existing test suites)
2. Golden tests serve as the integration verification

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace && cargo clippy --workspace`
