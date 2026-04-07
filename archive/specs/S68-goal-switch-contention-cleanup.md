# S68: Goal-Switch Contention Cleanup

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Category**: Architecture fix — AI agent lifecycle

## Problem

When an AI agent switches goals during the planning phase, stale contention queue entries from the abandoned goal are not cleaned up. This causes `DuplicateActor` errors when the new goal's action tries to enqueue on the same contended entity.

**Concrete failure path**:
1. Agent adopts `TreatWounds` for wounded entity W
2. Agent enters W's `ContentionQueue` via `QueueForFacilityUse`
3. New candidate `EscortToSafety` for W arrives with competitive motive
4. Agent switches goals (planning.rs:912-920) — `materialization_bindings.clear()` runs but `ContentionIntents` is NOT cleared
5. `escort_to_safety` commits and calls `enqueue_for_contention(actor, W, ...)` (escort_actions.rs:480)
6. `ContentionQueue::enqueue` calls `has_actor(actor)` which finds the actor still in the queue from step 2 -> `DuplicateActor` error

**Root cause**: The goal-switch path in `agent_tick/planning.rs:912-920` clears `materialization_bindings` but does NOT clear the agent's `ContentionIntents` component. The same gap exists in the "lost plan" path at lines 927-940.

The death-clear path (agent_tick/mod.rs:396-397) correctly resets `current_facility_intents = ContentionIntents::default()`, and the authoritative `prune_invalid_waiters` system in `facility_queue.rs:147-169` then removes the actor from any `ContentionQueue` where their intents no longer match. No equivalent intent-clearing exists on the goal-switch or lost-plan paths.

## Scope

This spec fixes the contention cleanup gap on goal switch and lost plan. It does NOT redesign the contention system or change how goals compete.

## Information-Path Analysis

No new information paths. This is lifecycle cleanup — the agent already knows about the contended entity from perception and beliefs. The fix ensures stale world-state artifacts (queue entries, intent records) are cleaned up when the agent abandons a goal.

## Positive-Feedback Analysis

No positive-feedback loops introduced. Goal switching is already bounded by the planner's search and ranking system.

## Concrete Dampeners

N/A — no amplifying loops.

## Stored State vs. Derived Read-Model

| Artifact | Classification | Change |
|----------|---------------|--------|
| `ContentionQueue` (per-entity) | Authoritative stored state | Stale entries removed by existing `prune_invalid_waiters` after intent clear |
| `ContentionIntents` (per-agent) | Authoritative stored state | Must clear on goal switch and lost plan |

## What to Change

### 1. Thread `facility_intents` into planning functions

The goal-switch code lives inside `plan_and_validate_next_step` (planning.rs:541) and its traced wrapper `plan_and_validate_next_step_traced` (planning.rs:670). Neither function currently takes `&mut ContentionIntents`. Add it to both signatures and update the call sites in `mod.rs` (lines 528, 632, 684).

### 2. Clear ContentionIntents on goal switch

In `crates/worldwake-ai/src/agent_tick/planning.rs`, at the goal-switch path (around line 912), clear the agent's facility intents alongside `materialization_bindings`:

```rust
// Existing:
runtime.materialization_bindings.clear();
// Add:
facility_intents.intents.clear();
```

This mirrors the death-clear path at `agent_tick/mod.rs:397`.

### 3. Clear ContentionIntents on lost plan

In the same file, at the "lost plan" path (around line 932), apply the same cleanup:

```rust
// Existing:
runtime.materialization_bindings.clear();
// Add:
facility_intents.intents.clear();
```

### 4. Actual ContentionQueue dequeue — handled by existing prune system

No direct `ContentionQueue` mutation is needed. Once `ContentionIntents` is cleared, the existing `prune_invalid_waiters` system in `facility_queue.rs:147-169` handles the actual queue removal on the next `contention_system` tick. The prune checks `actor_has_matching_contention_intent` (line 164-169) and removes any waiter whose intents no longer match. This is the same mechanism that handles death cleanup — the death-clear path also only clears `ContentionIntents`, relying on the prune system for queue removal.

### 5. Verify the interrupt path also cleans up

Check `agent_tick/active_action.rs` interrupt path (lines 102-122) — when an active action is interrupted during goal switch, `reconcile_in_flight_state` (observation.rs:287) is called with `facility_intents` and handles intent reconciliation through `reconcile_committed_facility_queue_intents`. Verify this path correctly updates intents when the interrupted action was a `QueueForFacilityUse`.

## Cross-System Interactions

| System | Interaction | Mediation |
|--------|-------------|-----------|
| Contention (S44) | Goal switch clears intents; prune system dequeues stale entries | State — ContentionIntents cleared, `prune_invalid_waiters` reacts |
| Care (E12) | TreatWounds contention must be cleaned when agent switches to EscortToSafety | State — ContentionIntents |
| AI planning | Goal switch and lost-plan paths must coordinate with contention cleanup | Lifecycle — intent clear in planning function |

## FOUNDATIONS Alignment

- **P4 (Persistent Identity)**: Contention queue entries are world state with stable identity — abandoned entries must be explicitly removed, not left as orphans. The prune system provides the explicit removal mechanism.
- **P8 (Preconditions & Duration)**: The contention system's `has_actor` check is correct — the bug is that stale state makes lawful actions appear unlawful
- **P21 (Revisable Commitments)**: Goal switching IS the mechanism for revisable commitments. The fix ensures switching doesn't leave world-state debris from abandoned commitments
- **P26 (Systems Interact Through State)**: The fix follows the state-mediated pattern — planning clears `ContentionIntents`, the contention system's prune step reacts to the state change. No direct cross-system mutation.

## Risks

- **Prune timing**: The `prune_invalid_waiters` system runs on the next `contention_system` tick after intents are cleared. If the new goal's action commits within the same system tick (before the prune runs), the stale queue entry would still cause `DuplicateActor`. In practice, action commits happen in subsequent ticks after the planning phase, so the prune will have run. If a same-tick race is discovered, the `enqueue` call site could add a pre-enqueue `remove_actor` (contention.rs:136) guard, but this should be treated as a separate fix if needed.

## Verification

- **Unit test**: Verify that after a goal switch in the planning function, `facility_intents.intents` is empty. ✅ Delivered by S68GOASWICON-001.
- **Golden test**: `golden_goal_switch_clears_contention_queue_entry` (Scenario 123) proves the full path: agent queued at exclusive workstation via AcquireCommodity, fatigue metabolism drives goal switch, stale queue entry pruned, simulation completes without DuplicateActor. ✅ Delivered by S68GOASWICON-003. (Original narrative used TreatWounds → EscortToSafety, but EscortToSafety ops lack QueueForFacilityUse; production domain was used instead per P1.)
- **Regression**: All existing golden tests pass (`cargo test -p worldwake-ai`). ✅ 1065 unit + 45 golden.
- **Invariant sweep**: Every `materialization_bindings.clear()` in the codebase now has a matching `facility_intents.intents.clear()`. ✅ Delivered by S68GOASWICON-001 + S68GOASWICON-002 + S68GOASWICON-004.
