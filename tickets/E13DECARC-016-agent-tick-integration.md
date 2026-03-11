# E13DECARC-016: Agent tick integration and decision loop

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — integrates AI into the tick loop
**Deps**: E13DECARC-007, E13DECARC-008, E13DECARC-012, E13DECARC-013, E13DECARC-014, E13DECARC-015

## Problem

All the pieces need to be wired together into the per-tick decision loop for AI-controlled agents. This is the unified agent tick: derive pressures -> update dirty flag -> evaluate interrupts -> generate candidates -> rank -> plan -> revalidate -> emit `InputKind::RequestAction`. Dead agents, human-controlled agents, and idle states must be handled correctly.

## Assumption Reassessment (2026-03-11)

1. `ControlSource::Ai` marks AI agents — confirmed.
2. `InputEvent` and `InputKind::RequestAction` exist — confirmed.
3. `InputQueue` accepts deterministic `(tick, sequence_no)` ordering — confirmed.
4. `Scheduler` runs systems per tick — confirmed.
5. `SystemFn` signature takes `SystemExecutionContext` — confirmed.
6. `ReplanNeeded` is a signal from the action framework — confirmed.
7. All component and AI subsystems from prior tickets.

## Architecture Check

1. Decision loop is event-driven: only replan when dirty flag is set.
2. Dirty flag triggers: plan missing, plan finished, plan invalidated, `ReplanNeeded` received, place changed, inventory changed, wounds changed, threshold band changed, blocked intent cleared/expired.
3. AI reads through `&dyn BeliefView` only — constructs `OmniscientBeliefView` at tick start.
4. AI outputs are `InputKind::RequestAction` — same pipeline as human.
5. Dead agents are skipped entirely.
6. No action running + no valid plan = idle (explicit, lawful).

## What to Change

### 1. Implement the per-agent decision function in `worldwake-ai/src/agent_tick.rs`

```rust
pub fn decide_for_agent(
    view: &dyn BeliefView,
    agent: EntityId,
    runtime: &mut AgentDecisionRuntime,
    blocked_memory: &mut BlockedIntentMemory,
    utility: &UtilityProfile,
    registry: &ActionDefRegistry,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    budget: &PlanningBudget,
    current_tick: Tick,
    replan_signals: &[ReplanNeeded],
) -> Option<InputEvent>
```

Per-agent tick flow:
1. Skip dead agents (return `None`)
2. Derive current pressures (pain, danger)
3. Update dirty flag based on state changes
4. Process any `ReplanNeeded` signals for this agent
5. Clear expired / resolved blocked intents
6. Evaluate interrupts against current action
7. If replanning required:
   a. Generate grounded candidates
   b. Suppress blocked candidates
   c. Rank candidates (priority class + motive score)
   d. Plan only top `max_candidates_to_plan`
   e. Select best valid plan
8. If current valid plan exists:
   a. Revalidate next step by affordance identity
   b. If invalid: trigger failure handling, return `None`
   c. If valid: emit `InputKind::RequestAction` for that step
9. If no valid plan: return `None` (idle)

### 2. Implement dirty flag logic

```rust
fn compute_dirty(
    view: &dyn BeliefView,
    agent: EntityId,
    runtime: &AgentDecisionRuntime,
    replan_signals: &[ReplanNeeded],
    blocked_memory: &BlockedIntentMemory,
    current_tick: Tick,
) -> bool
```

Dirty when:
- Plan missing or finished
- Plan invalidated (next step would fail revalidation)
- `ReplanNeeded` received for this agent
- Place changed since last decision
- Inventory/possessions changed
- Wounds changed
- Relevant threshold band crossed
- Blocked intent cleared or expired

### 3. Wire into the system dispatch

Create a system function that iterates all `ControlSource::Ai` agents and calls `decide_for_agent()`. This must integrate with the existing `SystemDispatch` / scheduler infrastructure.

### 4. Manage `AgentDecisionRuntime` storage

`AgentDecisionRuntime` instances are stored in a `BTreeMap<EntityId, AgentDecisionRuntime>` owned by the AI system, not in the component tables. This map is initialized lazily (on first AI tick for each agent).

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify — was empty stub)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — add dirty computation)
- `crates/worldwake-ai/src/lib.rs` (modify — export system function)
- `crates/worldwake-systems/src/lib.rs` (modify — may need to register AI system in dispatch table, or this is done in worldwake-cli)

## Out of Scope

- Per-agent belief stores (E14) — uses `OmniscientBeliefView` for now
- CLI / human control interface — E21
- Multi-agent coordination / group planning — Phase 3+
- Exploration motivation from ignorance — Phase 3+

## Acceptance Criteria

### Tests That Must Pass

1. Dead agent produces no `InputEvent`
2. Human-controlled agent (`ControlSource::Human`) is skipped
3. Agent with hunger at critical + owned food -> emits RequestAction for consume
4. Agent with no food + seller nearby -> emits RequestAction for travel or trade
5. Agent with valid plan, undirty state -> re-emits next step without replanning
6. Agent receiving `ReplanNeeded` -> sets dirty, replans
7. Agent with invalid next step -> triggers failure handling, records `BlockedIntent`
8. Agent with all candidates blocked -> idles (returns `None`)
9. Agent does not thrash between equal plans (switch margin enforced)
10. Agent does not retry blocked target (BlockedIntentMemory suppresses)
11. `InputEvent` has correct `scheduled_tick` and monotonic `sequence_no`
12. All AI reads go through `&dyn BeliefView` (grep: no `&World` in worldwake-ai)
13. Existing suite: `cargo test --workspace`

### Invariants

1. All AI reads through `&dyn BeliefView` — no `&World` access in `worldwake-ai`
2. AI outputs are `InputKind::RequestAction` — same pipeline as human
3. Dead agents produce nothing
4. Idle is explicit behavior, not an error
5. `AgentDecisionRuntime` is NOT in component tables
6. Decision loop is event-driven (dirty flag gated)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` — integration tests with full mock setup: BeliefView, registry, agents with various states
2. End-to-end test: create world with agents, run several ticks, verify agents produce correct RequestActions

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
