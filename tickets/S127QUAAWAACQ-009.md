# S127QUAAWAACQ-009: Surface `AcquisitionQuantity` through the decision-trace pipeline

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: `worldwake-ai` (trace carriers + emitter), `worldwake-core` (optional new carrier)
**Deps**: S127QUAAWAACQ-008

## Problem

Spec S127 D11 promises that the existing `AcquireCommodity` decision-trace lines "add `desired_min`, `desired_target`, `horizon_ticks`". The implementation tickets (S127QUAAWAACQ-001..007) added the `quantity: AcquisitionQuantity` field to `GoalKind::AcquireCommodity` and wrote `derive_acquire_commodity_quantity` to compute the per-agent target from need projection + carry capacity. However, both points where the goal becomes observable to the trace layer — `emit_candidate_with_trace` in `crates/worldwake-ai/src/candidate_generation.rs:4794` and `From<GoalKind> for GoalKey` in `crates/worldwake-core/src/goal.rs:200-215` — apply the `GoalKey::from(kind)` normalization that collapses `quantity` to `AcquisitionQuantity::single()`. The collapsed value is what flows into:

- `OpportunityKey.goal_key.kind` (the only goal identity surfaced through the planner pipeline).
- `RankedGoalSummary.opportunity` (decision trace's per-tick ranked goal record).
- `format_goal_key` and `format_goal_kind` (decision-trace summary strings).

As a result, `desired_target` has no observable effect today: the per-agent variation that S126 + S127 are supposed to make visible at the trace layer is silently erased. Golden coverage for this surface (S127QUAAWAACQ-008 Golden 4) was narrowed during reassessment to "the candidate emitter emits AcquireCommodity within horizon and the agent harvests successfully" because the live trace surface cannot prove anything stronger.

## Architecture Check

1. The `quantity` field is intentionally excluded from goal identity (Design Goal 9) — two acquisition goals with the same commodity + purpose share a `GoalKey` so the planner does not double-emit. This is correct.
2. What's missing is a parallel observability carrier that preserves the per-emission `AcquisitionQuantity` for the trace layer without affecting goal identity.
3. The candidate emitter already has the value at `derive_acquire_commodity_quantity`'s return point. A bounded, optional field on `RankedGoalSummary` (or on `CandidateOfferDiagnostic`) would round-trip it through to the decision trace.

## What to Change

1. Add an optional `acquisition_quantity: Option<AcquisitionQuantity>` field to `RankedGoalSummary` (or to a new `RankedGoalDetail` carrier, if `RankedGoalSummary` should stay narrow). Populate it in the ranking-trace builder when the ranked goal is `GoalKind::AcquireCommodity`.
2. Thread the original `GoalKind` (or just the `AcquisitionQuantity` value) from `emit_candidate_with_trace` through to the diagnostics record so the ranking pass can read it without re-deriving.
3. Update `format_goal_kind` / `format_goal_key` callers that print the selected goal to include the quantity tuple when available.
4. Add a focused unit test that proves an agent with a high need projection sees `desired_target > 1` in the recorded `RankedGoalSummary.acquisition_quantity` for the AcquireCommodity goal.

## Out of Scope

- Changing goal identity (`GoalKey`) to include quantity.
- Changing `is_satisfied` semantics (which already uses `desired_min`).
- S131 wait-tick projection observability (separate spec).

## Acceptance Criteria

1. After implementation, `golden_s126_long_horizon_scales_desired_target` (in `golden_quantity_aware_acquisition.rs`) can be widened to assert `desired_target > 1` in the decision trace's ranked-goal record for the AcquireCommodity goal. Update that golden in this ticket.
2. `cargo test -p worldwake-ai` passes.
3. `./scripts/verify.sh` passes.

## Test Plan

1. Focused unit test in `crates/worldwake-ai/src/candidate_generation.rs` (or `decision_trace.rs`) verifying the new field is populated with the live derived value.
2. Widen `golden_s126_long_horizon_scales_desired_target` to assert the decision-trace surface reflects the derived `desired_target`.
3. `python3 scripts/golden_inventory.py --write --check-docs` — refresh inventory if scenario metadata changes.

## References

- S127QUAAWAACQ-008 Outcome §"Follow-up Gaps Identified" item 1.
- `crates/worldwake-core/src/goal.rs:200-215` — `GoalKey::from(GoalKind)` normalization.
- `crates/worldwake-ai/src/candidate_generation.rs:4794` — emission point where quantity is currently collapsed.
- `crates/worldwake-ai/src/candidate_generation.rs:2870` — `derive_acquire_commodity_quantity`.
