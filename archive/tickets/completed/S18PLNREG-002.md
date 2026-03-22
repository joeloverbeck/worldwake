# S18PLNREG-002: Generalize planner semantics coverage for override-driven actions

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — planner semantics classification audit helper/tests, optional focused invariant helper
**Deps**: `docs/FOUNDATIONS.md`, `archive/tickets/completed/E13DECARC-010-planner-op-kinds-and-semantics-table.md`, `archive/tickets/completed/E16BFORLEGJURCON-009.md`

## Problem

Planner action classification already regressed once for `press_force_claim` and `yield_force_claim` because `build_semantics_table()` matched on placeholder default payload shape instead of stable action identity. The live code still contains the same architectural fragility for several fixed-name planner ops whose semantics are keyed by action identity while runtime payloads are synthesized later. We need to remove that payload-shape coupling and add a focused guard so future registration changes cannot silently drop planner-relevant defs out of semantics.

## Assumption Reassessment (2026-03-22)

1. Current classification lives in `classify_action_def()` and `build_semantics_table()` inside `crates/worldwake-ai/src/planner_ops.rs`.
2. Current focused coverage exists at `planner_ops::tests::build_semantics_table_classifies_registered_planner_action_defs`, but that test proves today's assembled registry classifies. It does not explicitly lock the architectural rule that fixed-name planner families must classify by stable action identity rather than by `ActionDef.payload`.
3. The live fixed-name planner action families still classified with payload-shape coupling in `classify_action_def()` are:
   - `queue_for_facility_use`
   - `tell`
   - `consult_record`
   - `bribe`
   - `threaten`
   - `declare_support`
   Their action defs register with `ActionPayload::None` today, so the regression is latent rather than currently failing.
4. The live goal/planner surfaces already using synthesized payload binding or other payload-sensitive planner behavior include:
   - `GoalKind::ClaimOffice` -> `PlannerOpKind::ConsultRecord` / `DeclareSupport` / `Bribe` / `Threaten` / `PressForceClaim`
   - `GoalKind::SupportCandidateForOffice` -> `PlannerOpKind::ConsultRecord` / `DeclareSupport`
   - `GoalKind::ShareBelief` -> `PlannerOpKind::Tell`
   - exclusive-facility planning -> `PlannerOpKind::QueueForFacilityUse`
5. This is a planner regression/prevention ticket. The intended verification layer is focused planner-ops and goal-model coverage, not golden E2E.
6. There is no timing or ordering contract here. The contract is semantic reachability: if an action family is planner-relevant, its registered action defs must classify regardless of placeholder default payload encoding.
7. No heuristic is being removed. The missing substrate is a stronger invariant connecting action registration and planner classification.
8. The first failure boundary in the motivating bug was semantics classification before payload override construction. The exact symbols checked during reassessment were `classify_action_def()`, `build_semantics_table()`, and `GoalKind::build_payload_override()`.
9. For the motivating political case, the closure boundary was AI-layer operator availability before action execution.
10. No `ControlSource` or runtime-intent behavior is involved.
11. Scenario isolation is intentional: this ticket is not proving every planner path, only the invariant that planner-relevant action defs cannot silently drop out of semantics because of registration payload placeholders.
12. Mismatch corrected: current behavior is fixed for force-law actions, but the codebase still contains broader payload-shape coupling for other fixed-name planner families. The generalized ticket should address that wider structural risk, not only the already-fixed force-law pair.
13. The failure envelope is structural, not arithmetic: one misclassified action family is enough to make an otherwise lawful plan unreachable and convert a semantic omission into misleading `BudgetExhausted` noise.

## Architecture Check

1. The clean approach is to make planner semantics classification depend on stable action identity for fixed-name planner families and to add a focused invariant test for those families. That is more robust than repeatedly patching individual `match` arms after regressions surface.
2. The design should stay local to planner registration/classification. It should not add cross-module alias maps, fallback shims, or duplicated action registration metadata.
3. No backwards-compatibility aliasing or alternate planner-only action names should be introduced.

## Verification Layers

1. Fixed-name planner action defs classify to a semantics entry regardless of default payload placeholder shape -> focused `planner_ops.rs` tests
2. Goal-model payload synthesis for representative override-driven political and social ops still works after classification cleanup -> focused `goal_model.rs` tests
3. Runtime planner selection stays reachable for a force-law political path after semantics cleanup -> existing `agent_tick` focused trace test as downstream sanity check
4. No action-trace or authoritative-world-state layer is primary because this ticket is about planner registration invariants, not action execution
5. Additional layer mapping beyond planner-focused tests is not primary for this single-layer regression-prevention ticket

## What to Change

### 1. Define the planner classification policy for fixed-name planner actions

Make the ticket explicit about the architectural rule: if the planner identifies an action family by stable action identity, `classify_action_def()` must not depend on the default `ActionDef.payload` variant for that family. This includes payload-override families and other fixed-name planner families whose runtime payload handling is resolved later.

### 2. Strengthen focused classification coverage

Add or refactor tests so they explicitly guard the fixed-name planner action families whose semantics should not depend on placeholder payload encoding, including at minimum:

- `queue_for_facility_use`
- `tell`
- `consult_record`
- `bribe`
- `threaten`
- `declare_support`
- `press_force_claim`
- `yield_force_claim`

Prefer a small audit helper or table-driven assertion over one-off assertions if that keeps the rule centralized and extensible.

### 3. Keep representative payload-binding assertions honest

Retain focused `goal_model.rs` assertions for:

- force-law `ClaimOffice` using `PressForceClaim` rather than `DeclareSupport`
- `ShareBelief` using `Tell`

Only extend goal-model coverage if needed to keep the registration/classification rule tied to a real payload-synthesis surface.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify only if a focused binding assertion needs tightening)

## Out of Scope

- Reworking action registration in `worldwake-systems`
- Adding new planner action families
- Golden or workspace-wide behavior changes unrelated to semantics classification

## Acceptance Criteria

### Tests That Must Pass

1. Focused planner-ops test explicitly proves the fixed-name planner action defs above classify successfully without relying on default payload shape
2. Existing force-law and social goal-model payload override tests continue to pass
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Planner classification for fixed-name planner actions must depend on stable action identity, not placeholder payload encoding
2. No planner-relevant registered fixed-name action may silently fall out of the semantics table because of `ActionDef.payload` default shape alone

## Tests

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — add a fixed-name planner-family audit for payload-shape independence
Rationale: proves the real architectural invariant directly at the classification seam instead of only rechecking today's registry snapshot.
2. `crates/worldwake-ai/src/goal_model.rs` — keep or extend focused `PressForceClaim` / `DeclareSupport` / `Tell` payload-override assertions
Rationale: proves representative synthesized-payload planner families still bind lawfully after classification cleanup.

### Commands

1. `cargo test -p worldwake-ai --lib planner_ops::tests::build_semantics_table_classifies_registered_planner_action_defs -- --exact`
2. `cargo test -p worldwake-ai --lib planner_ops::tests::classify_action_def_fixed_name_families_ignore_placeholder_payload_shape -- --exact`
3. `cargo test -p worldwake-ai --lib goal_model::tests::claim_office_force_law_builds_press_force_claim_payload_override -- --exact`
4. `cargo test -p worldwake-ai --lib goal_model::tests::claim_office_force_law_rejects_declare_support_payload_override -- --exact`
5. `cargo test -p worldwake-ai --lib goal_model::tests::share_belief_goal_builds_tell_payload_override -- --exact`
6. `cargo test -p worldwake-ai --lib agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning -- --exact`
7. `cargo test -p worldwake-ai`

## Outcome

Completion date: 2026-03-22

What actually changed:
- Reassessed the ticket against the live code and corrected its scope before implementation. The remaining fragility was broader than the already-fixed force-claim pair: `queue_for_facility_use`, `tell`, `consult_record`, `bribe`, `threaten`, and `declare_support` were still classified with `ActionDef.payload` coupling in `crates/worldwake-ai/src/planner_ops.rs`.
- Simplified `classify_action_def()` so all fixed-name planner families classify by stable action identity. Only the recipe-shaped `harvest:*` and `craft:*` families still depend on payload shape, because their payload families are part of their executable identity.
- Added focused regression coverage in `crates/worldwake-ai/src/planner_ops.rs` proving those fixed-name families still classify when their payload is changed away from the registered placeholder `ActionPayload::None`.

Deviations from original plan:
- No `goal_model.rs` production change was needed. The existing `ClaimOffice` and `ShareBelief` payload-override tests were sufficient once classification coupling was removed in `planner_ops.rs`.
- The ticket broadened from “generalize force-law override coverage” to “remove payload-shape coupling for all fixed-name planner families,” because that is the cleaner and more durable architecture seam exposed by reassessment.

Verification results:
- `cargo test -p worldwake-ai --lib planner_ops::tests::build_semantics_table_classifies_registered_planner_action_defs -- --exact`
- `cargo test -p worldwake-ai --lib planner_ops::tests::classify_action_def_fixed_name_families_ignore_placeholder_payload_shape -- --exact`
- `cargo test -p worldwake-ai --lib goal_model::tests::claim_office_force_law_builds_press_force_claim_payload_override -- --exact`
- `cargo test -p worldwake-ai --lib goal_model::tests::claim_office_force_law_rejects_declare_support_payload_override -- --exact`
- `cargo test -p worldwake-ai --lib goal_model::tests::share_belief_goal_builds_tell_payload_override -- --exact`
- `cargo test -p worldwake-ai --lib agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning -- --exact`
- `cargo test -p worldwake-ai`
- `cargo test --workspace`
- `cargo clippy --workspace`
