# S137PLACAULIN-011: plan_repair successful localized strategy handlers

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes - successful `plan_repair` handlers for local plan replacement
**Deps**: archive/tickets/S137PLACAULIN-006.md

## Problem

`S137PLACAULIN-006` landed the public `plan_repair` module, bounded attempt ordering, repair-memory skip behavior, and causal-link emission, but it deliberately did not synthesize successful localized replacement plans. Before this ticket, `attempt_repair_then_replan` returned deterministic `RepairOutcome::Failed` entries only. Before ticket 007 can route live invalidator breaches through repair, the module needed real handlers for the repair kinds that can succeed without S139.

## Assumption Reassessment (2026-05-13)

1. `crates/worldwake-ai/src/plan_repair.rs` exposes `PlanRepairContext`, `RepairOutcome`, `RepairFailure`, `attempt_order`, and `attempt_repair_then_replan(&PlanRepairContext, &CognitiveProfile, &RepairMemory)`. This ticket changed the handlers from staged failure-only behavior to successful localized construction when the context supplies lawful replacement candidates.
2. `PlanRepairContext` carries the current `OpportunityKey`, breached causal link, breach signature, preserved prefix, reusable suffix, new evidence, discrepancy entry, and explicit `RepairPlanCandidate` inputs. It does not carry an `AgentDecisionRuntime`; ticket 007 owns constructing the candidates at the invalidator seam from lawful local search/belief inputs.
3. Provider fidelity is part of the same boundary. Archive ticket 006 maps `RequiredFact` into `PlanningFact` and a first-pass `CausalProvider`; before successful handlers trust the provider, reassess whether each emitted provider is a lawful supporter of the fact. In particular, `RouteKnown` and `ResourceAccess` may need richer provider evidence than the staged observation/belief mapping in `plan_guard_build.rs`.
4. Shared boundary under audit: the local replacement-plan contract. A successful handler must return a `PlannedPlan` whose prefix/suffix semantics are clear and whose replacement step payload still passes action-handler payload validation once ticket 007 starts it.
5. Live `GoalKind` under test: `TravelTo`, `Trade`, and `ProduceCommodity` are the first goal families that existing S137 post-hoc repair classification covers. This ticket should keep coverage focused on those surfaces unless reassessment proves a smaller first success slice is the only truthful boundary.
6. Ordering contract: successful localized repair remains action-lifecycle-internal. Ticket 007 owns running the repair before full replan in `agent_tick`; this ticket owns only whether `attempt_repair_then_replan` can produce `RepairOutcome::Repaired`.
7. Adjacent contradiction classification: `InsertVerification` still depends on S139 and may continue to return `RepairFailure::NoEpistemicSubstrate`; that is not a blocker for this ticket if the other non-S139 strategies can succeed.

## Architecture Check

1. The clean boundary is successful plan construction inside `plan_repair`, not inside `agent_tick/execution.rs`. Routing should consume `RepairOutcome`; it should not know how to synthesize replacement plans.
2. Replacement strategy code must reuse planner/search abstractions where possible rather than duplicating domain-specific path, trade, harvest, or craft logic.
3. No compatibility aliases: update the `attempt_repair_then_replan` signature if the staged context is insufficient, then update dependent ticket text in the same pass.

## Verified Layers

1. `RebindTarget` success -> focused unit/runtime test proving a sibling target satisfying the same required fact yields `RepairOutcome::Repaired { kind: RebindTarget, new_plan }`.
2. `ReplaceProvider` success -> focused unit/runtime test proving a different provider step can satisfy the same consumer fact and produces a replacement plan.
3. `DowngradeToProgressBarrier` / `Abandon` semantics -> focused test proving the returned plan shape or failure path is deterministic and does not skip preserved-prefix commitments.
4. Payload validity -> waived for this ticket because `plan_repair` does not synthesize domain-specific action payloads; ticket 007 owns candidate construction and payload-validator proof at the live invalidator seam.
5. Causal-provider fidelity -> focused unit/runtime test proving every successful strategy reads a provider that lawfully supports the breached `PlanningFact`, or updates the causal-provider shape before routing uses it.

## What Changed

### 1. Extended `PlanRepairContext`

Added explicit fields for the current opportunity and replacement candidates rather than passing a broad runtime object. The data contract stays small enough for ticket 007 to construct at the invalidator seam.

### 2. Implemented non-S139 successful repair handlers

Implemented successful handlers for:

- `RebindTarget`
- `ReplaceProvider`
- `DowngradeToProgressBarrier`
- `Abandon`

`InsertVerification` still returns `RepairFailure::NoEpistemicSubstrate` until S139 lands.

### 3. Audited causal-provider fidelity before successful use

Before returning `RepairOutcome::Repaired`, successful handlers verify that the candidate provider is a lawful supporter of `broken_link.fact`. The implemented support checks cover `TargetPresent`, `CommodityAvailable`, `RouteKnown`, and `ResourceAccess`.

### 4. Kept deterministic budget and memory behavior

Successful handlers still respect the budget and skip recently failed `(BreachSignature, RepairKind)` pairs.

## Files Touched

- `crates/worldwake-ai/src/plan_repair.rs` (modify)
- `crates/worldwake-ai/src/lib.rs` (modify)
- `archive/specs/S137-plan-causal-links-and-repair.md` (modify)
- `archive/tickets/S137PLACAULIN-007.md` (modify)
- `archive/tickets/S137PLACAULIN-011.md` (modify)

## Out of Scope

- Revalidation routing in `agent_tick/execution.rs` - ticket 007.
- `RepairAttemptTrace` emission - ticket 008.
- Observer rendering - ticket 009.
- Golden scenarios and Phase 11 gate - ticket 010.
- S139 epistemic sensing implementation.

## Acceptance Criteria

### Test Results

1. `cargo test -p worldwake-ai plan_repair`
2. Waived: focused payload-validator tests for newly synthesized payload shapes were not applicable because `plan_repair` now consumes explicit `RepairPlanCandidate.step` inputs and does not synthesize domain-specific action payloads itself. Ticket 007 owns constructing those candidate steps at the live invalidator seam and must validate that boundary.
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `RepairOutcome::Repaired` is reachable for at least one non-S139 strategy under a focused test.
2. The replacement plan preserves the committed prefix and makes suffix reuse or discard explicit in the test assertion.
3. Budget, attempt order, and repair-memory skip behavior from ticket 006 remain unchanged.
4. No repair success bypasses action payload validation.
5. No repair success relies on a `CausalProvider` that cannot lawfully support the breached `PlanningFact`.

## Tests Run

### New/Modified Tests

1. `crates/worldwake-ai/src/plan_repair.rs` - successful strategy tests for the implemented non-S139 repair kinds.
2. Validator-level tests were not added because no new rebound payload shapes are synthesized in `plan_repair`.

### Commands

1. `cargo test -p worldwake-ai plan_repair`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Implemented successful localized repair construction in `crates/worldwake-ai/src/plan_repair.rs`.

- `PlanRepairContext` now carries the current opportunity and explicit `replacement_candidates`.
- `RepairPlanCandidate` is the narrow input seam for candidate kind, provider, fact, and step; no broad `AgentDecisionRuntime` is passed into `plan_repair`.
- `RebindTarget` and `ReplaceProvider` can now return `RepairOutcome::Repaired` when a candidate matches the requested `RepairKind`, supports the breached `PlanningFact`, and passes the causal-provider fidelity check.
- `DowngradeToProgressBarrier` returns a progress-barrier plan that preserves only the committed prefix when discrepancy clearing is repair-search-visible.
- `Abandon` returns an empty progress-barrier plan after earlier strategies fail or are skipped.
- `InsertVerification` still returns `RepairFailure::NoEpistemicSubstrate` until S139.

Truth-synced `archive/specs/S137-plan-causal-links-and-repair.md` and `archive/tickets/S137PLACAULIN-007.md` for the finalized `RepairPlanCandidate` API and candidate-construction responsibility.

## Deviations

No `search/` helpers or action-handler registration tests were added. The final design keeps payload synthesis out of `plan_repair`: replacement steps arrive as explicit candidates, and ticket 007 owns producing those candidates from the live invalidator seam.

## Verification Result

1. Passed `cargo test -p worldwake-ai plan_repair`
2. Waived focused payload-validator tests for newly synthesized payload shapes because no new payload shape is synthesized in `plan_repair`.
3. Passed `cargo test -p worldwake-ai`
4. Passed `cargo test --workspace`
5. Passed `cargo clippy --workspace --all-targets -- -D warnings`

Workspace verification emitted the existing non-failing future-incompatibility warning for `ashpd v0.8.1`.
