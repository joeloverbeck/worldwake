# S147HTNMETDEC-004: MethodSchema and supporting type surface

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — introduces the `htn` module in worldwake-ai with `MethodSchema` and 14 supporting types. No runtime behavior yet (consumed by tickets 006, 007).
**Deps**: `archive/tickets/S147HTNMETDEC-001.md` (MethodSchemaId, MotiveSourceDiscriminant, GoalKindDiscriminant), `archive/tickets/S147HTNMETDEC-002.md` (MethodFailureKind)

## Problem

S147 D1 defines the data shape that every first-ship method (ticket 006) and the method selector (ticket 007) consume. The supporting type surface — `MethodPrecondition`, `SubgoalTemplate`, `MotiveBias`, `MethodFailureMode`, plus 10 inline-defined supporting enums (`BeliefPredicate`, `EntityCriterion`, `RoleTag`, `LocationTemplate`, `TopicTemplate`, `PayloadTemplate`, `PayloadValueTemplate`, `ArtifactTemplate`, `ClaimRequirement`, `ExplanationTemplateId`) — was scoped at reassessment to the first-ship method requirements. The inline-definition strategy (vs. one ticket per type) follows the "Supporting type clusters" heuristic in `.claude/skills/reassess-spec/references/spec-writing-rules.md`: tight first-ship-scoped types with a single parent consumer prefer inline definition. This ticket also carries D6's split — the `From<&MethodFailureMode> for MethodFailureKind` impl that bridges the ai-side payload type to the core-side discriminant.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Before this ticket, no `htn` module existed in `crates/worldwake-ai/src/` (`lib.rs:6-36` enumerated the previous modules; no `htn` listed). The landed `htn/` directory and `htn/mod.rs` are net-new. None of the 14 supporting types existed anywhere in the workspace.
2. `PlannerOpKind` exists at `crates/worldwake-ai/src/planner_ops.rs:13` with 32+ variants. Method subgoals reference real variants (verified in ticket 006). `WorkstationTag` lives in `crates/worldwake-core/src/production.rs`. `CommodityKind`, `EntityId`, `Quantity`, `Permille`, and `GoalPlanningBudget` all live in `worldwake-core`.
3. `MethodFailureMode` (this ticket) projects to `MethodFailureKind` (defined by `archive/tickets/S147HTNMETDEC-002.md` at `crates/worldwake-core/src/discrepancy.rs`) via a `From` impl. The five `MethodFailureMode` variants map to the five `MethodFailureKind` variants 1:1 (`PreconditionLost → PreconditionLost`, `SubgoalUnachievable → SubgoalUnachievable`, etc.) — the projection drops the ai-side payload (`BeliefPredicate`, `ArtifactTemplate`, etc.) and preserves only the discriminant.
4. Shared boundary: `MethodSchema` and its supporting types are the data contract between the registry (ticket 006), the method selector (ticket 007), and the planner integration (ticket 008). The contract is owned by this ticket; consumers may read but not modify.
5. The spec's D1 pseudocode referenced `crate::role::OfficeKind` in `EntityCriterion::OfficeOfKind`. Live reassessment found no `crate::role` module in `worldwake-ai` and no core-side `OfficeKind` discriminator, so the landed `EntityCriterion` omits `OfficeOfKind` rather than inventing a new authority surface. The active spec snippet was truth-synced to the landed surface.

## Architecture Check

1. Inline definition of 10 supporting enums within `htn/method_schema.rs` follows the spec-writing-rules.md "Supporting type clusters" heuristic: each type is tightly scoped to first-ship method requirements, consumed only by the method registry and selector, and would not benefit from independent ticket placement. Variant lists are explicitly first-ship-scoped; future methods that need new variants own the addition as part of their own deliverable.
2. The `From<&MethodFailureMode> for MethodFailureKind` impl lives in this file (ai-side) because `MethodFailureMode` is ai-side; `MethodFailureKind` is core-side and `From` is implemented in the ai crate where the conversion is needed. This respects the workspace layering — core cannot reference `MethodFailureMode`, so the impl must live in ai.
3. No backwards-compatibility shims. All types are net-new; no existing types are modified.

## Verified Layers

1. All 15 types compile with the required derives → `cargo build -p worldwake-ai` succeeds.
2. `From<&MethodFailureMode> for MethodFailureKind` covers all five `MethodFailureMode` variants → focused unit test in `method_schema.rs` tests asserts each variant projects to its discriminant counterpart.
3. Single-layer ticket — runtime consumption is verified by tickets 006 (registry), 007 (selector), 008 (planner integration). This ticket verifies only the type surface and derive correctness.

## Landed Changes

### 1. Create the `htn/` module skeleton

New files:
- `crates/worldwake-ai/src/htn/mod.rs` — declares `pub mod method_schema;` (other sub-modules land in tickets 006, 007).

Modify `crates/worldwake-ai/src/lib.rs` to add `pub mod htn;`.

### 2. Define `MethodSchema` and 14 supporting types

New file `crates/worldwake-ai/src/htn/method_schema.rs`:

```rust
use worldwake_core::{
    CommodityKind, EntityId, GoalKindDiscriminant, GoalPlanningBudget,
    MethodFailureKind, MethodSchemaId, MotiveSourceDiscriminant, Permille, Quantity, WorkstationTag,
};
use crate::planner_ops::PlannerOpKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MethodSchema {
    pub id: MethodSchemaId,
    pub goal_kind: GoalKindDiscriminant,
    pub preconditions: Vec<MethodPrecondition>,
    pub subgoals: Vec<SubgoalTemplate>,
    pub expected_artifacts: Vec<ArtifactTemplate>,
    pub required_claims: Vec<ClaimRequirement>,
    pub failure_modes: Vec<MethodFailureMode>,
    pub explanation_template: ExplanationTemplateId,
    pub motive_bias: Vec<MotiveBias>,
    pub planning_budget_hint: Option<GoalPlanningBudget>,
}

// MethodPrecondition, SubgoalTemplate, MotiveBias, MethodFailureMode + 10
// supporting enums per spec D1. See ../../archive/specs/S147-htn-method-decomposition.md D1
// for the full variant lists.
```

Define every type per the spec's D1 section. Apply derives per the spec's note: `Clone, Debug, Eq, PartialEq` for the outer types; `Copy` on small enums where they serve as keys (e.g., `RoleTag`, `ExplanationTemplateId`).

### 3. `From<&MethodFailureMode> for MethodFailureKind` impl

In `method_schema.rs`:

```rust
impl From<&MethodFailureMode> for MethodFailureKind {
    fn from(mode: &MethodFailureMode) -> Self {
        match mode {
            MethodFailureMode::PreconditionLost(_)     => MethodFailureKind::PreconditionLost,
            MethodFailureMode::SubgoalUnachievable(_)  => MethodFailureKind::SubgoalUnachievable,
            MethodFailureMode::ArtifactNotProduced(_)  => MethodFailureKind::ArtifactNotProduced,
            MethodFailureMode::ClaimDenied(_)          => MethodFailureKind::ClaimDenied,
            MethodFailureMode::Timeout(_)              => MethodFailureKind::Timeout,
        }
    }
}
```

### 4. Resolved `crate::role::OfficeKind` reference

Live implementation found no `crate::role` module or equivalent core-side office-kind discriminator. `EntityCriterion::OfficeOfKind` was dropped for this ticket so S147 does not introduce a placeholder authority category ahead of a real office-kind concept.

### 5. Type-surface tests

New focused tests in `method_schema.rs`:
- `method_failure_mode_to_kind_projection_covers_all_variants` — every `MethodFailureMode` variant projects to its `MethodFailureKind` counterpart.
- `method_schema_constructs_and_clones` — sanity check that the outer type derives work end-to-end.

## Landed Files

- `crates/worldwake-ai/src/htn/mod.rs` (new)
- `crates/worldwake-ai/src/htn/method_schema.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — add `pub mod htn;`)
- `archive/specs/S147-htn-method-decomposition.md` (modified — truth-sync D1 snippet to the landed `GoalPlanningBudget` import and no office-kind discriminator)

## Out of Scope

- The 13 first-ship method definitions themselves (ticket 006).
- The `MethodRegistry` type and `build_method_registry()` function (ticket 006).
- `MethodSelector` and `select_method()` (ticket 007).
- `MethodPlanAttemptTrace` and `PlanAttemptTrace.method_trace` extension (ticket 009) — note that `MethodFailureMode` (this ticket) is consumed by the trace, but the trace itself is ticket 009.

## Acceptance Result

### Tests Passed

1. `htn::method_schema::tests::method_failure_mode_to_kind_projection_covers_all_variants` — every `MethodFailureMode` variant projects correctly.
2. `htn::method_schema::tests::method_schema_constructs_and_clones` — outer type derives compile and round-trip.
3. Existing suite: `cargo test -p worldwake-ai` passed.
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings` passed.
5. `./scripts/verify.sh` passed.

### Invariants

1. Every type derives at least `Clone, Debug, Eq, PartialEq` (the bound the outer `MethodSchema` requires).
2. No type references symbols outside `worldwake-core` and `crate::planner_ops` — respects the workspace layering.
3. `MethodFailureMode → MethodFailureKind` projection is exhaustive (one arm per variant, no `_ =>` catch-all).

## Test Plan Result

### Added Tests

1. `crates/worldwake-ai/src/htn/method_schema.rs` inline tests — projection + construction sanity.

### Commands Run

1. `cargo test -p worldwake-ai --lib htn`
2. `cargo build -p worldwake-ai`
3. `cargo test -p worldwake-ai`
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
5. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-17.

- Added the `worldwake-ai::htn` module and exported the staged `method_schema` surface.
- Added `MethodSchema`, `MethodPrecondition`, `SubgoalTemplate`, `MotiveBias`, `MethodFailureMode`, and the first-ship supporting template enums.
- Added the exhaustive `From<&MethodFailureMode> for MethodFailureKind` projection and focused inline tests for the projection and clone/equality surface.
- Truth-synced the active S147 D1 snippet to use the live `worldwake_core::GoalPlanningBudget` export and to omit the draft-only `OfficeOfKind(crate::role::OfficeKind)` variant.

## Deviations

- The drafted `OfficeOfKind` variant did not land because the live codebase has no `OfficeKind` discriminator to wrap or reference. Creating a new placeholder discriminator would have invented an authority category instead of modeling an existing office-kind concept.
- `GoalPlanningBudget` was imported from `worldwake_core`, where the live type is defined and re-exported, rather than from a non-existent `worldwake-ai::goal_planning_budget` module.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib htn`.
- Passed `cargo build -p worldwake-ai`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
- Passed `./scripts/verify.sh`.
