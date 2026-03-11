# E13DECARC-013: Failure handling, BlockingFact derivation, and BlockedIntent lifecycle

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — AI-layer logic
**Deps**: E13DECARC-003, E13DECARC-005, E13DECARC-009

## Problem

When a plan step fails revalidation or execution, the agent must: drop the remaining plan, derive a concrete `BlockingFact`, write a `BlockedIntent` to memory, and mark itself dirty for replanning. Blocked intents must also clear when the believed blocker resolves or the TTL expires.

## Assumption Reassessment (2026-03-11)

1. `BlockedIntentMemory`, `BlockedIntent`, `BlockingFact` from E13DECARC-003.
2. `ReplanNeeded` struct exists in `worldwake-sim` with `agent`, `failed_action_def`, `failed_instance`, `reason`, `tick` — confirmed.
3. `AbortReason` exists in `worldwake-sim` — confirmed.
4. `PlanningBudget` has `transient_block_ticks` and `structural_block_ticks` from E13DECARC-009.
5. `BeliefView` extensions from E13DECARC-005.

## Architecture Check

1. Full replan on failure — Phase 2 does not splice or salvage remaining tail.
2. `BlockingFact` derivation follows a priority order: inspect beliefs -> inspect affordance family -> use `ReplanNeeded.reason` -> fall back to `Unknown`.
3. TTLs: transient blockers (out of stock, workstation busy, reservation conflict) use short TTL; structural blockers (no seller, missing tool, no path) use long TTL.
4. Blockers clear early when the believed blocker is no longer true.

## What to Change

### 1. Implement failure handler in `worldwake-ai/src/failure_handling.rs`

```rust
pub fn handle_plan_failure(
    view: &dyn BeliefView,
    agent: EntityId,
    failed_step: &PlannedStep,
    replan_signal: Option<&ReplanNeeded>,
    runtime: &mut AgentDecisionRuntime,
    blocked_memory: &mut BlockedIntentMemory,
    budget: &PlanningBudget,
    current_tick: Tick,
)
```

Steps:
1. Drop remaining plan from `runtime.current_plan`
2. Derive `BlockingFact` from beliefs + failed step
3. Determine TTL (transient vs structural)
4. Write `BlockedIntent` to `blocked_memory`
5. Preserve or clear `runtime.current_goal` depending on whether goal is still grounded
6. Set `runtime.dirty = true`

### 2. Implement `BlockingFact` derivation

```rust
fn derive_blocking_fact(
    view: &dyn BeliefView,
    agent: EntityId,
    step: &PlannedStep,
    replan_signal: Option<&ReplanNeeded>,
) -> BlockingFact
```

Priority order:
1. Inspect beliefs against step targets:
   - Seller exists but no stock -> `SellerOutOfStock`
   - Workstation reservation conflict -> `ReservationConflict`
   - Source quantity zero -> `SourceDepleted`
   - Target dead/gone -> `TargetGone`
2. Inspect affordance family existence with different targets:
   - No sellers at all -> `NoKnownSeller`
   - No path -> `NoKnownPath`
3. Use `ReplanNeeded.reason` as hint:
   - Map known abort reasons to blocking facts
4. Fall back to `Unknown`

### 3. Implement TTL classification

```rust
fn blocking_fact_ttl(fact: &BlockingFact, budget: &PlanningBudget) -> u32
```

- Transient: `SellerOutOfStock`, `WorkstationBusy`, `ReservationConflict`, `TargetGone`, `Unknown` -> `budget.transient_block_ticks`
- Structural: `NoKnownPath`, `NoKnownSeller`, `MissingTool`, `MissingInput`, `SourceDepleted`, `TooExpensive`, `DangerTooHigh`, `CombatTooRisky` -> `budget.structural_block_ticks`

### 4. Implement blocker resolution clearing

```rust
pub fn clear_resolved_blockers(
    view: &dyn BeliefView,
    agent: EntityId,
    blocked_memory: &mut BlockedIntentMemory,
    current_tick: Tick,
)
```

Check each non-expired `BlockedIntent`:
- `SellerOutOfStock` + seller entity -> seller has stock again?
- `WorkstationBusy` + workstation -> workstation no longer reserved?
- `SourceDepleted` + source -> source has quantity > 0?
- `NoKnownPath` + place -> path now exists?
- `TargetGone` + entity -> entity reappeared/alive?
- Clear if resolved or expired

## Files to Touch

- `crates/worldwake-ai/src/failure_handling.rs` (modify — was empty stub)
- `crates/worldwake-core/src/blocked_intent.rs` (modify — may need to add resolution check helpers)

## Out of Scope

- Plan revalidation logic (what triggers the failure) — E13DECARC-014
- Reactive interrupts — E13DECARC-015
- Agent tick integration — E13DECARC-016
- WorldTxn mutations for BlockedIntentMemory — those use existing component set APIs

## Acceptance Criteria

### Tests That Must Pass

1. Failure drops remaining plan from runtime
2. Failure sets `runtime.dirty = true`
3. Seller with no stock -> `BlockingFact::SellerOutOfStock`
4. No path available -> `BlockingFact::NoKnownPath`
5. Target dead -> `BlockingFact::TargetGone`
6. Reservation conflict -> `BlockingFact::ReservationConflict`
7. Unknown abort reason -> `BlockingFact::Unknown`
8. Transient blockers use `transient_block_ticks` TTL
9. Structural blockers use `structural_block_ticks` TTL
10. `BlockedIntent` suppresses immediate retry of same goal key
11. Blocker clears when seller restocks
12. Blocker clears when workstation becomes available
13. Blocker clears on TTL expiry
14. `Unknown` uses transient TTL
15. Existing suite: `cargo test --workspace`

### Invariants

1. Full replan on failure — no tail splicing
2. `BlockingFact` is always concrete and inspectable
3. TTL classification is deterministic
4. Blocker clearing uses belief queries, not world state directly
5. `runtime.dirty` is always set after failure

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/failure_handling.rs` — derivation tests, TTL tests, clearing tests

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
