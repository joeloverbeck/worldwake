# S137PLACAULIN-011: plan_repair successful localized strategy handlers

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes - successful `plan_repair` handlers for local plan replacement
**Deps**: archive/tickets/S137PLACAULIN-006.md

## Problem

`S137PLACAULIN-006` landed the public `plan_repair` module, bounded attempt ordering, repair-memory skip behavior, and causal-link emission, but it deliberately does not synthesize successful localized replacement plans. `attempt_repair_then_replan` currently returns deterministic `RepairOutcome::Failed` entries only. Before ticket 007 can route live invalidator breaches through repair, the module needs real handlers for the repair kinds that can succeed without S139.

## Assumption Reassessment (2026-05-13)

1. `crates/worldwake-ai/src/plan_repair.rs` exposes `PlanRepairContext`, `RepairOutcome`, `RepairFailure`, `attempt_order`, and `attempt_repair_then_replan(&PlanRepairContext, &CognitiveProfile, &RepairMemory)`. Its current handlers are staged failure handlers.
2. `PlanRepairContext` carries the breached causal link, breach signature, preserved prefix, reusable suffix, new evidence, and discrepancy entry. It does not carry an `AgentDecisionRuntime`, so successful handlers must either build replacement plans from this context plus explicit search inputs added in this ticket, or extend the public function signature truthfully before ticket 007 routes it.
3. Provider fidelity is part of the same boundary. Archive ticket 006 maps `RequiredFact` into `PlanningFact` and a first-pass `CausalProvider`; before successful handlers trust the provider, reassess whether each emitted provider is a lawful supporter of the fact. In particular, `RouteKnown` and `ResourceAccess` may need richer provider evidence than the staged observation/belief mapping in `plan_guard_build.rs`.
4. Shared boundary under audit: the local replacement-plan contract. A successful handler must return a `PlannedPlan` whose prefix/suffix semantics are clear and whose replacement step payload still passes action-handler payload validation once ticket 007 starts it.
5. Live `GoalKind` under test: `TravelTo`, `Trade`, and `ProduceCommodity` are the first goal families that existing S137 post-hoc repair classification covers. This ticket should keep coverage focused on those surfaces unless reassessment proves a smaller first success slice is the only truthful boundary.
6. Ordering contract: successful localized repair remains action-lifecycle-internal. Ticket 007 owns running the repair before full replan in `agent_tick`; this ticket owns only whether `attempt_repair_then_replan` can produce `RepairOutcome::Repaired`.
7. Adjacent contradiction classification: `InsertVerification` still depends on S139 and may continue to return `RepairFailure::NoEpistemicSubstrate`; that is not a blocker for this ticket if the other non-S139 strategies can succeed.

## Architecture Check

1. The clean boundary is successful plan construction inside `plan_repair`, not inside `agent_tick/execution.rs`. Routing should consume `RepairOutcome`; it should not know how to synthesize replacement plans.
2. Replacement strategy code must reuse planner/search abstractions where possible rather than duplicating domain-specific path, trade, harvest, or craft logic.
3. No compatibility aliases: update the `attempt_repair_then_replan` signature if the staged context is insufficient, then update dependent ticket text in the same pass.

## Verification Layers

1. `RebindTarget` success -> focused unit/runtime test proving a sibling target satisfying the same required fact yields `RepairOutcome::Repaired { kind: RebindTarget, new_plan }`.
2. `ReplaceProvider` success -> focused unit/runtime test proving a different provider step can satisfy the same consumer fact and produces a replacement plan.
3. `DowngradeToProgressBarrier` / `Abandon` semantics -> focused test proving the returned plan shape or failure path is deterministic and does not skip preserved-prefix commitments.
4. Payload validity -> focused validator-level test for any synthesized payload accepted by the relevant action handler registration.
5. Causal-provider fidelity -> focused unit/runtime test proving every successful strategy reads a provider that lawfully supports the breached `PlanningFact`, or updates the causal-provider shape before routing uses it.

## What to Change

### 1. Extend `PlanRepairContext` only if needed

If successful handlers need search state that the current context lacks, add explicit fields rather than passing a broad runtime object. Keep the data contract small enough for ticket 007 to construct at the invalidator seam.

### 2. Implement non-S139 successful repair handlers

Implement successful handlers for:

- `RebindTarget`
- `ReplaceProvider`
- `DowngradeToProgressBarrier`
- `Abandon`

`InsertVerification` may keep returning `RepairFailure::NoEpistemicSubstrate` until S139 lands.

### 3. Audit causal-provider fidelity before successful use

Before returning `RepairOutcome::Repaired`, verify that `PlanRepairContext.broken_link.provider` is a lawful supporter of `broken_link.fact`. If `RouteKnown`, `ResourceAccess`, or another fact cannot be repaired from the staged provider mapping, update the provider shape or guard emission in this ticket and truth-sync dependent tickets before routing goes live.

### 4. Keep deterministic budget and memory behavior

Successful handlers must still respect the budget and skip recently failed `(BreachSignature, RepairKind)` pairs.

## Files to Touch

- `crates/worldwake-ai/src/plan_repair.rs` (modify)
- Likely: `crates/worldwake-ai/src/search/` (modify if successful handlers need bounded local search helpers)
- Likely: action-handler registration tests if rebound payload validation needs focused proof
- `tickets/S137PLACAULIN-007.md` (modify only if this ticket changes the `attempt_repair_then_replan` API again)

## Out of Scope

- Revalidation routing in `agent_tick/execution.rs` - ticket 007.
- `RepairAttemptTrace` emission - ticket 008.
- Observer rendering - ticket 009.
- Golden scenarios and Phase 11 gate - ticket 010.
- S139 epistemic sensing implementation.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai plan_repair`
2. Focused payload-validator tests for any newly synthesized payload shape.
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `RepairOutcome::Repaired` is reachable for at least one non-S139 strategy under a focused test.
2. The replacement plan preserves the committed prefix and makes suffix reuse or discard explicit in the test assertion.
3. Budget, attempt order, and repair-memory skip behavior from ticket 006 remain unchanged.
4. No repair success bypasses action payload validation.
5. No repair success relies on a `CausalProvider` that cannot lawfully support the breached `PlanningFact`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/plan_repair.rs` - successful strategy tests for the implemented non-S139 repair kinds.
2. Validator-level tests if new rebound payload shapes are synthesized.

### Commands

1. `cargo test -p worldwake-ai plan_repair`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
