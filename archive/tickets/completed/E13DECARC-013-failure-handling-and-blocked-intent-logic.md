# E13DECARC-013: Failure handling, BlockingFact derivation, and BlockedIntent lifecycle

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — AI-layer logic
**Deps**: `specs/E13-decision-architecture.md`, `archive/tickets/completed/E13DECARC-003-blocked-intent-memory-component.md`, `archive/tickets/completed/E13DECARC-005-belief-view-extensions.md`, `archive/tickets/completed/E13DECARC-009-planning-budget-and-decision-runtime.md`

## Problem

When a plan step fails revalidation or execution, the agent must: drop the remaining plan, derive a concrete `BlockingFact`, write a `BlockedIntent` to memory, and mark itself dirty for replanning. Blocked intents must also clear when the believed blocker resolves or the TTL expires.

## Assumption Reassessment (2026-03-11)

1. `BlockedIntentMemory`, `BlockedIntent`, and `BlockingFact` already exist in [`crates/worldwake-core/src/blocked_intent.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/blocked_intent.rs), and they already provide the generic storage helpers this ticket needs: `is_blocked()`, `record()`, `expire()`, and `clear_for()`.
2. [`crates/worldwake-ai/src/failure_handling.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/failure_handling.rs) does not exist yet. This ticket must create it and wire it through [`crates/worldwake-ai/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/lib.rs).
3. `ReplanNeeded` exists in [`crates/worldwake-sim/src/replan_needed.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/replan_needed.rs) with `agent`, `failed_action_def`, `failed_instance`, `reason`, and `tick` — confirmed.
4. `AbortReason` in [`crates/worldwake-sim/src/action_handler.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_handler.rs) is now structured and machine-readable. Common commit, interrupt, external, and handler-requested failure causes are modeled semantically; free-form detail remains only under explicit `Other`-style escape hatches.
5. `PlanningBudget` already has `transient_block_ticks` and `structural_block_ticks` in [`crates/worldwake-ai/src/budget.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/budget.rs).
6. `BeliefView` already includes the E13 extension surface in [`crates/worldwake-sim/src/belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs), including seller, reservation, source, wound, corpse, and duration queries.
7. The current planner/operator surface differs from the original ticket sketch:
   - `PlannerOpKind` uses `Trade`, not separate `TradeAcquire` / `TradeSell`.
   - `PlannerOpKind` includes `Defend`.
   - `PlanTerminalKind` already includes `CombatCommitment` in addition to `GoalSatisfied` and `ProgressBarrier`.
8. `cargo test -p worldwake-ai` is green before this work. This ticket adds missing failure-handling infrastructure rather than repairing an already failing suite.

## Architecture Check

1. Full replan on failure remains correct. Phase 2 should still drop the remaining plan instead of doing tail surgery.
2. `BlockingFact` derivation should stay in `worldwake-ai`, not `worldwake-core`. It depends on `BeliefView`, `PlannedStep`, planner operator semantics, and AI runtime policy; moving that logic into core would blur the authoritative-state boundary.
3. `BlockedIntentMemory` in core should remain a dumb authoritative memory store with generic helpers only. Resolution policy belongs in the AI layer.
4. The failure handler should not run candidate generation or ranking just to decide whether to clear `runtime.current_goal`. That grounding decision belongs to the later decision-loop integration ticket. For this ticket, the failure handler should drop the current plan, record the blocker, and mark the runtime dirty.
5. `BlockingFact` derivation should prefer concrete belief inspection first, then planner-step/operator context, then `ReplanNeeded.reason` as a weak fallback, then `Unknown`.
6. TTL classification remains sound as an AI budget policy: short for transient blockers, long for structural blockers, deterministic from `BlockingFact`.
7. The current `BlockedIntent` schema does not store blocker-specific details such as a desired reservation window. That means some resolution checks must be conservative and based only on currently available belief queries. This is acceptable for Phase 2, but it is a real architectural limit to keep in mind during later agent-tick integration.

## Scope Correction

In scope:

1. Create [`crates/worldwake-ai/src/failure_handling.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/failure_handling.rs).
2. Implement AI-layer helpers to:
   - drop the current plan on failure
   - derive a concrete `BlockingFact`
   - compute blocker TTL from `PlanningBudget`
   - record/update `BlockedIntentMemory`
   - clear expired or belief-resolved blockers
   - mark the agent runtime dirty
3. Export the new helpers from [`crates/worldwake-ai/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/lib.rs).
4. Add focused unit tests covering derivation, TTL selection, blocked-intent recording, and blocker clearing.

Out of scope:

1. Agent tick integration and dirty-trigger orchestration — E13DECARC-016.
2. Affordance revalidation itself — E13DECARC-014.
3. Interrupt policy — E13DECARC-015.
4. Adding `BeliefView`-dependent helper logic to `worldwake-core`.
5. Recomputing the full grounded candidate set just to preserve/clear `runtime.current_goal`.

## What to Change

### 1. Implement failure handler in `worldwake-ai/src/failure_handling.rs`

```rust
pub struct PlanFailureContext<'a> {
    pub view: &'a dyn BeliefView,
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub failed_step: &'a PlannedStep,
    pub replan_signal: Option<&'a ReplanNeeded>,
    pub current_tick: Tick,
}

pub fn handle_plan_failure(
    context: &PlanFailureContext<'_>,
    runtime: &mut AgentDecisionRuntime,
    blocked_memory: &mut BlockedIntentMemory,
    budget: &PlanningBudget,
)
```

Steps:
1. Drop remaining plan from `runtime.current_plan`
2. Derive `BlockingFact` from beliefs + failed step
3. Determine TTL (transient vs structural)
4. Write `BlockedIntent` to `blocked_memory`
5. Leave `runtime.current_goal` unchanged
6. Set `runtime.dirty = true`

### 2. Implement `BlockingFact` derivation

```rust
fn derive_blocking_fact(
    view: &dyn BeliefView,
    agent: EntityId,
    goal_key: &GoalKey,
    step: &PlannedStep,
    replan_signal: Option<&ReplanNeeded>,
) -> BlockingFact
```

Priority order:
1. Inspect beliefs against the failed step and goal key:
   - missing / dead targeted entity -> `TargetGone`
   - trade counterparty exists but requested stock is gone -> `SellerOutOfStock`
   - trade counterparty no longer sells the commodity locally -> `NoKnownSeller`
   - travel target place is no longer adjacent / reachable from current believed place -> `NoKnownPath`
   - workstation has a production job -> `WorkstationBusy`
   - reservable target still has active reservation ranges -> `ReservationConflict`
   - source exists but quantity is zero -> `SourceDepleted`
   - required tool or treatment input is no longer present on the actor -> `MissingTool` / `MissingInput`
   - local attackers / hostiles still make the plan non-viable -> `DangerTooHigh` / `CombatTooRisky`
2. Use `ReplanNeeded.reason` as a weak fallback hint for cases beliefs cannot disambiguate cleanly.
3. Fall back to `Unknown`

### 3. Implement TTL classification

```rust
fn blocking_fact_ttl(fact: BlockingFact, budget: &PlanningBudget) -> u32
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
- `SellerOutOfStock` + seller entity -> seller has stock again for the goal commodity
- `WorkstationBusy` + workstation -> workstation no longer has a production job
- `ReservationConflict` + reservable entity -> reservation ranges are now empty
- `SourceDepleted` + source -> source has quantity > 0 for the goal commodity
- `NoKnownSeller` + local place + goal commodity -> some other seller is now available
- `NoKnownPath` + place -> place is now reachable from the actor’s current believed place
- `TargetGone` + entity -> target is alive again and locally valid
- Clear if resolved or expired

## Files to Touch

- `crates/worldwake-ai/src/failure_handling.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — export failure-handling helpers)

## Out of Scope

- Plan revalidation logic (what triggers the failure) — E13DECARC-014
- Reactive interrupts — E13DECARC-015
- Agent tick integration — E13DECARC-016
- WorldTxn mutations for `BlockedIntentMemory` — those use existing component set APIs
- `worldwake-core` blocked-intent schema changes unless implementation proves them strictly necessary

## Acceptance Criteria

### Tests That Must Pass

1. Failure drops remaining plan from runtime
2. Failure sets `runtime.dirty = true`
3. Seller with no stock -> `BlockingFact::SellerOutOfStock`
4. No path available -> `BlockingFact::NoKnownPath`
5. Target dead -> `BlockingFact::TargetGone`
6. Busy workstation -> `BlockingFact::WorkstationBusy`
7. Reservation conflict -> `BlockingFact::ReservationConflict`
8. Transient blockers use `transient_block_ticks` TTL
9. Structural blockers use `structural_block_ticks` TTL
10. `BlockedIntent` suppresses immediate retry of same goal key
11. Blocker clears when seller restocks
12. Blocker clears when workstation becomes available
13. Blocker clears on TTL expiry
14. `Unknown` uses transient TTL
15. Trade failure with no remaining local seller -> `BlockingFact::NoKnownSeller`
16. Existing `cargo test -p worldwake-ai`
17. Existing suite: `cargo test --workspace`
18. Existing suite: `cargo clippy --workspace`

### Invariants

1. Full replan on failure — no tail splicing
2. `BlockingFact` is always concrete and inspectable
3. TTL classification is deterministic
4. Blocker clearing uses belief queries, not world state directly
5. `runtime.dirty` is always set after failure
6. Failure-handling policy stays in `worldwake-ai`; authoritative blocked-intent state stays in `worldwake-core`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/failure_handling.rs` — derivation tests, TTL tests, clearing tests, and runtime mutation tests
   Rationale: this module owns the failure policy and should prove that belief-driven classification stays deterministic and local.
2. `crates/worldwake-ai/src/lib.rs` — export smoke test only if needed
   Rationale: later E13 tickets consume these helpers through the crate boundary.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- Outcome amended: 2026-03-11
- Completion date: 2026-03-11
- What actually changed:
  - Added [`crates/worldwake-ai/src/failure_handling.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/failure_handling.rs) with `PlanFailureContext`, `handle_plan_failure(...)`, and `clear_resolved_blockers(...)`.
  - Implemented belief-driven `BlockingFact` derivation for current Phase 2 planner operators, deterministic TTL classification, and blocked-intent recording that drops the runtime plan and marks the runtime dirty.
  - Exported the failure-handling API from [`crates/worldwake-ai/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/lib.rs).
  - Added focused unit coverage for runtime mutation, blocker derivation, TTL classification, seller/workstation clearing, and expiry clearing.
  - Follow-up refinement: replaced stringly `worldwake-sim::AbortReason` payloads with structured abort categories plus structured handler-request reasons, so AI failure handling and future systems can consume stable machine-readable causes instead of parsing ad hoc text.
- Deviations from original plan:
  - The final API uses a borrowed `PlanFailureContext` instead of a wide positional parameter list. This satisfied the workspace lint bar and is a cleaner long-term integration surface for the later agent-tick ticket.
  - No `worldwake-core` changes were needed. The existing `BlockedIntentMemory` schema and helpers were sufficient, and keeping policy out of core preserved the intended authority boundary.
  - `runtime.current_goal` is intentionally left unchanged. Re-grounding or clearing the top-level goal belongs in the later decision-loop integration ticket, not in this lower-level failure-policy helper.
  - Resolution checks are conservative where the stored blocker data is intentionally minimal. In particular, reservation conflicts clear when reservation ranges disappear, because the current blocked-intent schema does not persist the original desired reservation window.
  - `ReplanNeeded.reason` is now structurally meaningful across common engine-level and handler-requested abort causes. Free-form text remains only as optional detail on explicit `Other`-style escape hatches for uncategorized failures.
- Verification results:
  - `cargo test -p worldwake-ai` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace` passed.
