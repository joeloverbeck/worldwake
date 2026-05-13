# S137PLACAULIN-012: Phase 11 plan-repair baseline gate witness

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — likely AI repair-routing or scenario-witness correction after diagnosis
**Deps**: archive/tickets/S137PLACAULIN-010.md (golden repair substrate scenarios), archive/tickets/S137PLACAULIN-007.md (revalidation routing), archive/tickets/S137PLACAULIN-011.md (successful localized repair handlers)

## Problem

S137's Phase 11 gate requires `survival-baseline.ron` or another explicitly approved gate witness to show at least a 30% reduction in `EventTag::ReplanTriggered` compared to the pre-S137 baseline, with localized repair replacing full replans. During `S137PLACAULIN-010`, the direct substrate goldens passed, but a live 1440-tick `survival-baseline.ron` replay emitted `ReplanTriggered=82` and `RepairApplied=0`. That means the Phase 11 gate is not yet a truthful golden assertion.

This ticket owns diagnosing and landing the gate witness. The first task is to determine whether the missing `RepairApplied` events are caused by production repair-routing gaps, the authored survival-baseline scenario not exercising causal-link invalidation, or a stale metric definition.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `crates/worldwake-ai/tests/golden_plan_repair.rs` from `S137PLACAULIN-010` proves the live `plan_repair` substrate directly: `RebindTarget`, `InsertVerification` short-circuit without S139, one-kind `RepairMemory` retry suppression, `CommodityAvailabilityChanged` clearing, budget exhaustion, and `Abandon` plan shape.
2. The same ticket attempted the drafted Phase 11 replay and observed `survival-baseline.ron` counters over 1440 ticks: `ReplanTriggered=82`, `RepairApplied=0`. A 30% reduction assertion would therefore be false on the current branch.
3. Shared boundary under audit: the live autonomous AI repair-routing path from `PlanInvalidationReason::ExpectationMismatch` through `attempt_local_repair_for_invalidated_step`, `RepairAttemptTrace`, `RepairAppliedPayload`, and the append-only event log.
4. Live `GoalKind` under test must be determined from the replay before implementation. Do not assume the drafted `TravelTo`/`Trade`/`ProduceCommodity` branches are the actual gate owners until decision traces and event payloads show which full replans dominate the baseline.
5. The repeat-failed `Abandon` premise from the original ticket is not currently represented as "multiple failed kinds for one signature" because `RepairMemory.repairs` stores one `RepairEntry` per `BreachSignature`. If the gate needs all-kinds repeat-failed surrender, this ticket must first change that data contract or explicitly choose a different gate invariant.

## Architecture Check

1. The correct fix must preserve FND-1, FND-3, FND-20, FND-21, and FND-29: repair events must arise from real breached causal links, not from a counter-forcing shim or scenario stage-management.
2. If `survival-baseline.ron` is not a lawful witness for localized repair, update the spec and implementation order to use a scenario that naturally exercises repair, rather than weakening the metric or fabricating events.
3. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Dominant full-replan causes -> decision/event-log trace over the gate witness, grouped by `ReplanReason`, `GoalKind`, and action/start/revalidation boundary.
2. Repair-routing eligibility -> focused lower-layer proof that breached plans carry causal links and reach `attempt_local_repair_for_invalidated_step` before `handle_current_step_failure`.
3. Repair event replacement -> event-log delta showing `EventTag::RepairApplied` rises while `EventTag::ReplanTriggered` falls for the same causal class.
4. Phase 11 gate counter math -> 1440-tick replay asserting post-fix `ReplanTriggered_after / ReplanTriggered_before <= 0.7`, with absolute before/after counters recorded in the test.
5. No-regression survival contract -> existing `golden_survival_baseline`, `golden_survival_scattered`, and `golden_survival_contested` suites continue to pass.

## What to Change

### 1. Diagnose the live gate witness

Add or use a cleanup-safe diagnostic surface to count `ReplanTriggered` and `RepairApplied` events over `survival-baseline.ron`, grouped by agent, goal, reason, and earliest available trace boundary. Remove temporary probes before closeout unless they become the shipped proof surface.

### 2. Land the truthful gate owner

Depending on diagnosis:

- If production repair routing is missing a lawful branch, fix the earliest concrete AI/runtime seam and add focused proof there.
- If `survival-baseline.ron` is not a repair witness, update the S137 validation text and use a scenario that naturally creates breached causal links.
- If the metric definition is stale, update `specs/S137-plan-causal-links-and-repair.md` and `specs/IMPLEMENTATION-ORDER.md` before writing the golden assertion.

### 3. Add the Phase 11 golden gate

Add the final gate assertion only after the chosen witness emits localized repair for a real causal class. Record the absolute baseline and post-fix counter values in the test.

## Files to Touch

- `crates/worldwake-ai/tests/golden_plan_repair.rs` or a narrower dedicated golden file (modify)
- Likely: `crates/worldwake-ai/src/agent_tick/execution.rs` or adjacent AI repair-routing code (modify, if diagnosis proves production ownership)
- Possibly: `scenarios/*.ron` (modify/add, only if scenario witness ownership is proven)
- `specs/S137-plan-causal-links-and-repair.md` (modify)
- `specs/IMPLEMENTATION-ORDER.md` (modify, if gate status or witness changes)

## Out of Scope

- Re-proving the direct `plan_repair` substrate already covered by `S137PLACAULIN-010`.
- Adding fake `RepairApplied` events or relaxing the 30% threshold without a spec-level correction grounded in live evidence.
- S139 epistemic sensing implementation unless the chosen gate witness specifically owns `InsertVerification` success.

## Acceptance Criteria

### Tests That Must Pass

1. Focused proof for the diagnosed earliest owner, if production changes land.
2. Phase 11 gate golden: command and test name to be finalized after witness diagnosis.
3. `cargo test -p worldwake-ai --test golden_survival_baseline`
4. `cargo test -p worldwake-ai --test golden_survival_scattered`
5. `cargo test -p worldwake-ai --test golden_survival_contested`
6. `cargo test -p worldwake-ai`

### Invariants

1. The final gate witness proves repair replaced full replan for a real causal class, not only that the event count changed.
2. Counter math records absolute before/after values and satisfies `ReplanTriggered_after / ReplanTriggered_before <= 0.7`.
3. Existing survival goldens remain unmodified unless diagnosis proves their authored contract is stale.

## Test Plan

### New/Modified Tests

1. To be finalized after diagnosis; likely a Phase 11 gate assertion in `crates/worldwake-ai/tests/golden_plan_repair.rs`.

### Commands

1. `cargo test -p worldwake-ai --test golden_plan_repair`
2. `cargo test -p worldwake-ai --test golden_survival_baseline`
3. `cargo test -p worldwake-ai --test golden_survival_scattered`
4. `cargo test -p worldwake-ai --test golden_survival_contested`
5. `cargo test -p worldwake-ai`
6. `python3 scripts/golden_inventory.py --write --check-docs` if golden metadata changes
