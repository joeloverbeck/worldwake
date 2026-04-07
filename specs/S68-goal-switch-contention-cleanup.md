# S68: Goal-Switch Contention Cleanup

**Status**: DRAFT
**Priority**: HIGH
**Category**: Architecture fix — AI agent lifecycle

## Problem

When an AI agent switches goals during the planning phase, stale contention queue entries from the abandoned goal are not cleaned up. This causes `DuplicateActor` errors when the new goal's action tries to enqueue on the same contended entity.

**Concrete failure path**:
1. Agent adopts `TreatWounds` for wounded entity W
2. Agent enters W's `ContentionQueue` via `QueueForFacilityUse`
3. New candidate `EscortToSafety` for W arrives with competitive motive
4. Agent switches goals (planning.rs:912-920) — `materialization_bindings.clear()` runs but `ContentionIntents` and the actual `ContentionQueue` entry are NOT cleared
5. `escort_to_safety` commits and calls `enqueue_for_contention(actor, W, ...)` (escort_actions.rs:480)
6. `ContentionQueue::enqueue` calls `has_actor(actor)` which finds the actor still in the queue from step 2 -> `DuplicateActor` error

**Root cause**: The goal-switch path in `agent_tick/planning.rs:912-920` clears `materialization_bindings` but does NOT:
- Remove the agent from any `ContentionQueue` they joined under the old goal
- Clear the agent's `ContentionIntents` component

The death-clear path (agent_tick/mod.rs:396-397) correctly resets `current_facility_intents = ContentionIntents::default()`, but no equivalent reset exists on the goal-switch path.

## Scope

This spec fixes the contention cleanup gap on goal switch. It does NOT redesign the contention system or change how goals compete.

## Information-Path Analysis

No new information paths. This is lifecycle cleanup — the agent already knows about the contended entity from perception and beliefs. The fix ensures stale world-state artifacts (queue entries, intent records) are cleaned up when the agent abandons a goal.

## Positive-Feedback Analysis

No positive-feedback loops introduced. Goal switching is already bounded by the planner's search and ranking system.

## Concrete Dampeners

N/A — no amplifying loops.

## Stored State vs. Derived Read-Model

| Artifact | Classification | Change |
|----------|---------------|--------|
| `ContentionQueue` (per-entity) | Authoritative stored state | Must remove actor entry on goal switch |
| `ContentionIntents` (per-agent) | Authoritative stored state | Must clear on goal switch |

## What to Change

### 1. Clear ContentionIntents on goal switch

In `crates/worldwake-ai/src/agent_tick/planning.rs`, at the goal-switch path (around line 912), clear the agent's facility intents alongside `materialization_bindings`:

```rust
// Existing:
runtime.materialization_bindings.clear();
// Add:
facility_intents.intents.clear();
```

This mirrors the death-clear path at `agent_tick/mod.rs:397`.

### 2. Dequeue agent from stale ContentionQueues

When `ContentionIntents` is cleared on goal switch, the agent must also be removed from any `ContentionQueue` they were waiting in. For each cleared intent, issue a `WorldTxn` mutation to dequeue the actor from the target entity's `ContentionQueue`.

This requires the planning function to have write access to the world transaction (or to record dequeue commands for the execution phase to process).

### 3. Verify the interrupt path also cleans up

Check `agent_tick/active_action.rs` interrupt path (lines 102-122) — when an active action is interrupted during goal switch, verify it also dequeues from contention. The `abort_*` handlers should already handle this, but verify.

## Cross-System Interactions

| System | Interaction | Mediation |
|--------|-------------|-----------|
| Contention (S44) | Goal switch must dequeue stale entries | Direct mutation |
| Care (E12) | TreatWounds contention must be cleaned when agent switches to EscortToSafety | State — ContentionQueue |
| AI planning | Goal switch path must coordinate with contention cleanup | Lifecycle |

## FOUNDATIONS Alignment

- **P4 (Persistent Identity)**: Contention queue entries are world state with stable identity — abandoned entries must be explicitly removed, not left as orphans
- **P8 (Preconditions & Duration)**: The contention system's `has_actor` check is correct — the bug is that stale state makes lawful actions appear unlawful
- **P21 (Revisable Commitments)**: Goal switching IS the mechanism for revisable commitments. The fix ensures switching doesn't leave world-state debris from abandoned commitments

## Risks

- **Queue removal ordering**: If the agent is the granted holder (not just waiting), dequeuing must trigger grant promotion for the next waiter. The existing `dequeue` / `remove` API must handle this correctly.
- **Transaction scope**: If the planning phase doesn't have write access to world state, the dequeue must be deferred to the execution phase via a command queue.
