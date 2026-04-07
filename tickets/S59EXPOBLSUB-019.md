# S59EXPOBLSUB-019: ReportFound as SearchForMissing post-resolution step and golden E2E

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — goal dispatch declaration, goal model, golden E2E test
**Deps**: S59EXPOBLSUB-001 through -016 (all completed and archived)

## Problem

`ReportFound` is a fully implemented action (`crates/worldwake-systems/src/report_actions.rs:674-959`) with affordance enumeration, payload validation, and commit logic that updates institutional records and resolves overdue expectations. However, `PlannerOpKind::ReportFound` is not listed in any goal's `relevant_ops` (confirmed: zero references in goal_dispatch_decl.rs), `build_payload_override()` returns `UnsupportedGoal` (goal_model.rs:645), and `apply_planner_step()` is a no-op (goal_model.rs:1010). The planner can never select ReportFound as a step in any plan.

Per FOUNDATIONS P10 (aftermath — actions create partial outcomes and future hooks) and P17 (violated expectation drives behavior), reporting findings is a natural consequence of search resolution. Per P20, reporting is a tactic (not a world condition), so it belongs as a post-resolution step within SearchForMissing rather than a standalone goal.

## Assumption Reassessment (2026-04-07)

1. **ReportFound is NOT in any relevant_ops** — Confirmed: grep for `PlannerOpKind::ReportFound` in `goal_dispatch_decl.rs` returns zero results. The constant `SEARCH_FOR_MISSING_OPS` at lines 89-93 contains only `[Travel, AskAboutPerson, SearchPlace]`.
2. **No GoalKind::ReportFound exists** — Confirmed: `crates/worldwake-core/src/goal.rs:17-122` has no ReportFound variant. Only `GoalKind::ReportMissing` exists (lines 48-52). This is correct per P20 — ReportFound is a tactic, not a goal.
3. **Action handler is complete** — `start_report_found`, `tick_report_found`, `commit_report_found`, `abort_report_found` all implemented in `report_actions.rs:674-959`.
4. **Affordance enumeration exists** — `enumerate_report_found_payloads` at `report_actions.rs:375-401` filters for found expectations with matching last-seen place.
5. **Payload struct** — `ReportFoundActionPayload { target: EntityId, expectation_id: ExpectationId }` in `worldwake-sim/src/action_payload.rs:387-390`.
6. **Live GoalKind under test**: `GoalKind::SearchForMissing { subject, last_seen }`. Current operator surface: `[Travel, AskAboutPerson, SearchPlace]`. ReportFound is absent from this list and must be added.
7. **commit_report_found world mutations** — Two target types: (a) Agent target: relays last-seen memory with hearsay provenance, resolves target's overdue expectations, resolves target's missing-person violations (`report_actions.rs:892-920`). (b) OfficeRegister target: writes missing-person status claim with found status (`report_actions.rs:921-929`).
8. **Planner semantics** — `PlannerOpKind::ReportFound` has `may_appear_mid_plan=false`, `is_materialization_barrier=false`, `transition_kind=GoalModelFallback` (planner_ops.rs:279).
9. **Golden scenario numbering** — Scenario 124 is now taken by S59EXPOBLSUB-018. Scenario 125 is free.
10. **Scenario isolation consideration** — The golden test must chain search -> find -> report. The scenario must include an office or interested agent to report to. The existing scenario 120 (overdue drives search) establishes the search-then-find pattern; this ticket extends it with a reporting step.
11. **Design choice: which goal's relevant_ops?** — ReportFound logically follows SearchForMissing (you report what you found during a search). Adding it to SEARCH_FOR_MISSING_OPS makes the planner able to chain search -> report in a single plan. This keeps the behavior emergent per P1 — the planner decides whether to report based on co-located targets, not a scripted sequence.

## Architecture Check

1. Adding ReportFound to SEARCH_FOR_MISSING_OPS follows the established pattern where SearchForMissing already includes AskAboutPerson as a progress-barrier terminal (implemented in S59EXPOBLSUB-018). ReportFound extends this with a post-resolution social step. The planner decides when reporting is possible (co-located office/agent) and worth doing. Note: ReportFound has `may_appear_mid_plan=false`, so it must be wired as a progress barrier (same as AskAboutPerson) to be reachable. Additionally, `enumerate_report_found_payloads` depends on `expectation_store()` which is now available on PlanningState thanks to S59EXPOBLSUB-018's PlanningSnapshot widening.
2. No backwards-compatibility shims — the UnsupportedGoal stub and no-op will be replaced with real logic.
3. No new GoalKind needed — ReportFound remains a PlannerOpKind used as a step within SearchForMissing goals, consistent with P20.

## Verification Layers

1. ReportFound appears as post-search step in SearchForMissing plan -> planning trace in golden E2E
2. ReportFound payload synthesized correctly (target = co-located office/agent) -> decision trace showing payload
3. report_found action commits and updates institutional record -> action trace in golden E2E
4. Office register receives missing-person found status -> authoritative world state assertions
5. Target agent's overdue expectations resolved -> authoritative world state assertions

## What to Change

### 1. Goal dispatch — add ReportFound to SEARCH_FOR_MISSING_OPS

In `crates/worldwake-ai/src/goal_dispatch_decl.rs`, add `PlannerOpKind::ReportFound` to `SEARCH_FOR_MISSING_OPS`:
```rust
const SEARCH_FOR_MISSING_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::AskAboutPerson,
    PlannerOpKind::SearchPlace,
    PlannerOpKind::ReportFound,  // NEW
];
```

### 2. Goal model — build_payload_override for ReportFound within SearchForMissing

In `crates/worldwake-ai/src/goal_model.rs`, replace the `UnsupportedGoal` return for `PlannerOpKind::ReportFound` when the active goal is `SearchForMissing`:
- Synthesize `ReportFoundActionPayload` with `target` = a co-located agent or office from beliefs, and `expectation_id` from the resolved expectation
- If no co-located report target exists, return `UnsupportedGoal` (planner skips)

### 3. Goal model — apply_planner_step for ReportFound within SearchForMissing

Replace the no-op for `PlannerOpKind::ReportFound` with state transition modeling:
- After ReportFound step, the planning state should reflect that the institutional record has been updated and the search obligation is discharged

### 4. Golden E2E test — scenario 124

Add `golden_report_found_after_search` (+ deterministic replay variant) to `crates/worldwake-ai/tests/golden_expectation.rs`. Scenario setup:
- Agent A at Place1 with overdue expectation for entity S
- Entity S actually at Place2
- Office O with OfficeRegister at Place1 (or reachable)
- Expected trace: SearchForMissing candidate -> plan includes travel + search_place + travel-back + report_found -> search finds S -> report_found commits -> office register updated with found status

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — add ReportFound to SEARCH_FOR_MISSING_OPS)
- `crates/worldwake-ai/src/goal_model.rs` (modify — ReportFound payload override and planner step)
- `crates/worldwake-ai/tests/golden_expectation.rs` (modify — add scenario 124)
- `crates/worldwake-ai/tests/scenarios/` (new — scenario 124 RON file if scenario-driven)

## Out of Scope

- ReportFound as a standalone goal (per FOUNDATIONS P20, it's a tactic)
- Reporting to non-co-located targets (would require travel + report chaining, already handled by the planner's Travel step)
- Corpse handling cascade after finding a dead entity (existing system scope)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_report_found_after_search` — ReportFound appears as post-search step, updates institutional record
2. `golden_report_found_after_search_replays_deterministically` — determinism invariant
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. ReportFound only appears in plans when a co-located report target (agent or office) exists in beliefs
2. Institutional record receives found status with correct subject identity
3. Overdue expectations on report target are resolved when target is an agent
4. Deterministic replay produces identical traces

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_expectation.rs::golden_report_found_after_search` — validates ReportFound as post-search step within SearchForMissing goal
2. `crates/worldwake-ai/tests/golden_expectation.rs::golden_report_found_after_search_replays_deterministically` — determinism invariant

### Commands

1. `cargo test -p worldwake-ai golden_report_found`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
