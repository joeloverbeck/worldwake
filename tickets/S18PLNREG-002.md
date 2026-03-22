# S18PLNREG-002: Generalize planner semantics coverage for override-driven actions

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — planner semantics classification audit helper/tests, optional focused invariant helper
**Deps**: `docs/FOUNDATIONS.md`, `archive/tickets/completed/E13DECARC-010-planner-op-kinds-and-semantics-table.md`, `archive/tickets/completed/E16BFORLEGJURCON-009.md`

## Problem

Planner action classification recently failed for `press_force_claim` and `yield_force_claim` because `build_semantics_table()` matched on placeholder default payload shape instead of stable action identity. That class of bug is not unique to force-law actions: any action registered with `ActionPayload::None` and specialized later by payload override can silently disappear from planner semantics. We need a generalized guard against that failure mode.

## Assumption Reassessment (2026-03-22)

1. Current classification lives in `classify_action_def()` and `build_semantics_table()` inside `crates/worldwake-ai/src/planner_ops.rs`.
2. Current focused coverage exists at `planner_ops::tests::build_semantics_table_classifies_registered_planner_action_defs`, but that test only checks a finite list of names and does not explicitly define a policy for override-driven actions beyond the currently known set.
3. The live goal surfaces already using override-driven action binding include:
   - `GoalKind::ClaimOffice` -> `PlannerOpKind::DeclareSupport` / `PlannerOpKind::PressForceClaim`
   - `GoalKind::ShareBelief` -> `PlannerOpKind::Tell`
   - queue/facility planner behavior through payload override in search, though queue is already handled as a classified action family
4. This is a planner regression/prevention ticket. The intended verification layer is focused planner-ops and goal-model coverage, not golden E2E.
5. There is no timing or ordering contract here. The contract is semantic reachability: if an action family is planner-relevant, its registered action defs must classify regardless of placeholder default payload encoding.
6. No heuristic is being removed. The missing substrate is a stronger invariant connecting action registration and planner classification.
7. The first failure boundary in the motivating bug was semantics classification before payload override construction. The exact symbols checked during reassessment were `classify_action_def()`, `build_semantics_table()`, and `GoalKind::build_payload_override()`.
8. For the motivating political case, the closure boundary was AI-layer operator availability before action execution.
9. No `ControlSource` or runtime-intent behavior is involved.
10. Scenario isolation is intentional: this ticket is not proving every planner path, only the invariant that planner-relevant action defs cannot silently drop out of semantics because of registration payload placeholders.
11. Mismatch corrected: current behavior is fixed for force-law actions, but the codebase still lacks a generalized specification and guard for override-driven action families.
12. The failure envelope is structural, not arithmetic: one misclassified action family is enough to make an otherwise lawful plan unreachable and convert a semantic omission into misleading `BudgetExhausted` noise.

## Architecture Check

1. The clean approach is to make planner semantics classification depend on stable action identity and to add a focused invariant test for planner-relevant registered action defs. That is more robust than repeatedly patching individual `match` arms after regressions surface.
2. The design should stay local to planner registration/classification. It should not add cross-module alias maps, fallback shims, or duplicated action registration metadata.
3. No backwards-compatibility aliasing or alternate planner-only action names should be introduced.

## Verification Layers

1. Registered planner-relevant action defs classify to a semantics entry -> focused `planner_ops.rs` tests
2. Override-driven goal payload binding still works for classified actions -> existing focused `goal_model.rs` tests
3. Runtime planner selection stays reachable for a force-law political path -> existing `agent_tick` focused trace test as downstream sanity check
4. No action-trace or authoritative-world-state layer is primary because this ticket is about planner registration invariants
5. Additional layer mapping beyond planner-focused tests is not primary for this single-layer regression-prevention ticket

## What to Change

### 1. Define the planner classification policy for override-driven actions

Make the ticket explicit about the architectural rule: if the planner identifies an action family by stable action identity and later synthesizes payloads through `build_payload_override()`, then `classify_action_def()` must not depend on the default `ActionDef.payload` variant for that family.

### 2. Strengthen focused classification coverage

Add or refactor tests so they explicitly guard override-driven planner action families, including at minimum:

- `press_force_claim`
- `yield_force_claim`
- any currently registered planner-relevant action family whose runtime payload is supplied or specialized later

Prefer a small audit helper over one-off assertions if that keeps the rule centralized and extensible.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify only if a focused binding assertion needs tightening)

## Out of Scope

- Reworking action registration in `worldwake-systems`
- Adding new planner action families
- Golden or workspace-wide behavior changes unrelated to semantics classification

## Acceptance Criteria

### Tests That Must Pass

1. Focused planner-ops test explicitly proves override-driven planner action defs classify successfully
2. Existing force-law goal-model payload override tests continue to pass
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Planner classification for override-driven actions must depend on stable action identity, not placeholder payload encoding
2. No planner-relevant registered action may silently fall out of the semantics table because of `ActionDef.payload` default shape alone

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — strengthen semantics-table coverage for override-driven action families
2. `crates/worldwake-ai/src/goal_model.rs` — keep or extend focused payload-override assertions where they prove the binding contract

### Commands

1. `cargo test -p worldwake-ai --lib planner_ops::tests::build_semantics_table_classifies_registered_planner_action_defs -- --exact`
2. `cargo test -p worldwake-ai --lib goal_model::tests::claim_office_force_law_builds_press_force_claim_payload_override -- --exact`
3. `cargo test -p worldwake-ai --lib goal_model::tests::claim_office_force_law_rejects_declare_support_payload_override -- --exact`
4. `cargo test -p worldwake-ai`
