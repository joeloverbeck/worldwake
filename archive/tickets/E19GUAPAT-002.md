# E19GUAPAT-002: Add patrol goal, planner op, event tag, and dispatch coverage

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new goal/op/tag identity plus AI dispatch-table coverage in core and AI crates
**Deps**: E19GUAPAT-001, [specs/E19-guard-patrol.md](/home/joeloverbeck/projects/worldwake/specs/E19-guard-patrol.md)

## Problem

The patrol epic needs an explicit duty identity across authoritative goals, planner operators, and event classification. The current ticket understates the implementation boundary as "mechanical enum additions," but the live AI architecture routes every new goal family through dispatch declarations, feasibility coverage, planner-op semantics, failure handling, and exhaustive test surfaces. This ticket should add patrol as a first-class goal/op/tag family without introducing alias paths or bypassing the dispatch system.

## Assumption Reassessment (2026-03-30)

1. `PatrolRoute` and `PatrolProfile` already exist in [crates/worldwake-core/src/patrol.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/patrol.rs). The dependency on E19GUAPAT-001 is already satisfied, so this ticket should not describe those components as future prerequisites.
2. `GoalKind` in [crates/worldwake-core/src/goal.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs) currently has 22 variants, not 27. The earlier count conflated authoritative `GoalKind` with AI-internal `GoalDispatchKey`, which currently has 27 entries in [crates/worldwake-ai/src/goal_dispatch_key.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_key.rs).
3. `PlannerOpKind` in [crates/worldwake-ai/src/planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs) currently has 28 variants, not 40. No `Patrol` variant exists.
4. `EventTag` in [crates/worldwake-core/src/event_tag.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/event_tag.rs) currently has 21 variants. No patrol-specific tag exists.
5. The live AI abstraction boundary under audit is not "goal-to-op mapping in `goal_model.rs`." It is the dispatch pipeline:
   [crates/worldwake-ai/src/goal_dispatch_key.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_key.rs) -> [crates/worldwake-ai/src/goal_dispatch_decl.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs) -> [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) / [crates/worldwake-ai/src/feasibility.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/feasibility.rs) / [crates/worldwake-ai/src/failure_handling.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/failure_handling.rs). The ticket must target that boundary directly.
6. The live patrol spec still calls for `GoalKind::Patrol { place: EntityId }` and `PlannerOpKind::Patrol` with `GoalModelFallback` semantics in [specs/E19-guard-patrol.md](/home/joeloverbeck/projects/worldwake/specs/E19-guard-patrol.md). That shape still fits the architecture because patrol is a durable duty state with a concrete waypoint anchor and its own terminal action, not a disguised travel or investigate alias.
7. `PlannerOpKind` does not expose a standalone `semantics()` method. Semantics are derived through `build_semantics_table()`, `classify_action_def()`, `semantics_for()`, and related tests in [crates/worldwake-ai/src/planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs). The ticket must reference the real mechanism.
8. `GoalKind::Patrol` will require more than compiler-driven match fixes. Current exhaustive coverage includes `GoalDispatchKey::ALL`, dispatch declarations, goal-model exhaustive tests, feasibility dispatch, and failure-handling switches. Those need explicit scope, not an implicit "compiler will flag these" assumption.
9. No adjacent contradiction forces a broader redesign. The current dispatch-table architecture is already the clean extensibility point; the correction is to use it consistently rather than bypassing it.

## Architecture Check

1. Adding an explicit patrol goal/op/tag family is cleaner than overloading existing identities such as `Travel`, `InvestigateViolation`, or a generic world-mutation tag. Patrol is a stable causal duty with its own planner terminal step, its own event surface, and its own candidate family in the spec. Reusing another family would hide intent, weaken traces, and create architectural aliasing.
2. The robust implementation path is to extend the existing dispatch architecture, not to special-case patrol inside `goal_model.rs`. `GoalDispatchKey` + `GoalDispatchDeclaration` is the repo's current extensibility boundary for new goal families, and using it keeps provenance, relevant ops, invalidation, and feasibility definitions in one place.
3. No backwards-compatibility shim or duplicate path should be added. Patrol should become the single lawful identity for patrol duty throughout the planning stack.

## Verification Layers

1. Authoritative patrol goal/tag serialization stability -> focused core unit tests in [crates/worldwake-core/src/goal.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs) and [crates/worldwake-core/src/event_tag.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/event_tag.rs)
2. Planner op classification and semantics for patrol action defs -> focused AI unit tests in [crates/worldwake-ai/src/planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs)
3. Patrol goal dispatch coverage stays coherent with live goal model -> focused AI unit tests in [crates/worldwake-ai/src/goal_dispatch_key.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_key.rs) and [crates/worldwake-ai/src/goal_dispatch_decl.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs)
4. Exhaustive goal/planner integration surfaces remain implemented -> focused AI unit tests in [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), plus compile coverage from targeted crate tests
5. This is not a runtime-behavior ticket yet. Candidate generation and action execution proofs remain in E19GUAPAT-003 and E19GUAPAT-004, so no decision-trace or action-trace verification is required in this ticket.

## What to Change

### 1. Add `GoalKind::Patrol` in `crates/worldwake-core/src/goal.rs`

```rust
Patrol { place: EntityId },
```

Update `GoalKey::from`, exhaustive tests, and any other helper matches that enumerate all `GoalKind` variants.

### 2. Add `EventTag::Patrol` in `crates/worldwake-core/src/event_tag.rs`

```rust
Patrol,
```

### 3. Add `PlannerOpKind::Patrol` in `crates/worldwake-ai/src/planner_ops.rs`

```rust
Patrol,
```

With semantics:
```rust
PlannerOpKind::Patrol => PlannerOpSemantics {
    may_appear_mid_plan: false,
    is_materialization_barrier: false,
    transition_kind: TransitionKind::GoalModelFallback,
},
```

Implement it through the live planner-op machinery:
- `classify_action_def()` must classify the future `"patrol"` generic action name as `PlannerOpKind::Patrol`
- `semantics_for()` / related tests must cover patrol semantics
- failure-handling exhaustive matches must include the new op where needed without inventing patrol-specific blocker logic prematurely

### 4. Add patrol to the AI dispatch layer

Add a patrol dispatch identity and declaration using the existing architecture:
- extend [crates/worldwake-ai/src/goal_dispatch_key.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_key.rs) with a dedicated patrol key
- extend [crates/worldwake-ai/src/goal_dispatch_decl.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs) with patrol trace label, relevant ops, invalidation strategy, and feasibility strategy
- keep `GoalKindPlannerExt::relevant_op_kinds()` driven by dispatch declarations rather than adding an ad hoc goal-to-op mapping path

### 5. Handle new variants in exhaustive matches

Any `match` on `GoalKind`, `PlannerOpKind`, `EventTag`, `GoalDispatchKey`, `FeasibilityStrategy`, or goal-model exhaustive test vectors across the touched crates must include the new patrol family. Update the smallest lawful set of surfaces; do not broaden into runtime patrol behavior.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify — add variant)
- `crates/worldwake-core/src/event_tag.rs` (modify — add variant)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — add variant, classification, semantics, tests)
- `crates/worldwake-ai/src/goal_dispatch_key.rs` (modify — add patrol dispatch identity)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — add patrol dispatch declaration and tests)
- `crates/worldwake-ai/src/goal_model.rs` (modify — exhaustive goal coverage only where patrol must participate)
- `crates/worldwake-ai/src/feasibility.rs` (modify — add patrol feasibility dispatch coverage)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — exhaustive planner-op coverage)

## Out of Scope

- Patrol action handler (E19GUAPAT-003)
- Patrol candidate generation logic (E19GUAPAT-004)
- Route adaptation (E19GUAPAT-006)
- ActionPayload::Patrol (E19GUAPAT-003)
- Any patrol runtime behavior beyond the identity/dispatch surfaces needed to compile and test this new family

## Acceptance Criteria

### Tests That Must Pass

1. `GoalKind::Patrol { place }` round-trips through serde
2. `GoalKey::from(GoalKind::Patrol { place })` preserves `place` as the canonical place anchor
3. `EventTag::Patrol` is included in the ordered tag inventory and round-trips through serde
4. `PlannerOpKind::Patrol` classification and semantics match the spec (`may_appear_mid_plan: false`, `is_materialization_barrier: false`, `GoalModelFallback`)
5. Patrol has a dedicated `GoalDispatchKey` and dispatch declaration with lawful relevant ops instead of a one-off goal-model alias path
6. Targeted crate suites for touched code pass
7. `cargo clippy --workspace`

### Invariants

1. Patrol has exactly one goal/op/tag identity path through the planner stack; no aliasing onto existing goal families
2. All exhaustive matches on `GoalKind`, `PlannerOpKind`, `EventTag`, and `GoalDispatchKey` include the new patrol family
3. Patrol remains place-anchored at the authoritative goal layer (`GoalKind::Patrol { place }`)
4. No `HashMap` or `f32`/`f64` introduced
5. Serialization format remains deterministic

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` — add patrol goal round-trip and canonical-place extraction coverage because the authoritative goal identity is new
2. `crates/worldwake-core/src/event_tag.rs` — extend ordered tag inventory and round-trip coverage because `EventTag` uses a stable declaration-order contract
3. `crates/worldwake-ai/src/planner_ops.rs` — add patrol op classification/semantics assertions because planner-op behavior is table-driven, not hardcoded elsewhere
4. `crates/worldwake-ai/src/goal_dispatch_key.rs` — add patrol dispatch-key coverage because new goal families must participate in the payload-aware dispatch layer
5. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — add patrol declaration coverage because relevant ops / invalidation / feasibility must stay centralized
6. `crates/worldwake-ai/src/goal_model.rs` and/or `crates/worldwake-ai/src/feasibility.rs` — extend exhaustive coverage tests only where patrol must now be enumerated

### Commands

1. `cargo test -p worldwake-core patrol_goal_roundtrips_through_bincode`
2. `cargo test -p worldwake-core event_tag_bincode_roundtrip_covers_every_variant`
3. `cargo test -p worldwake-ai patrol`
4. `cargo test -p worldwake-ai planner_op_kind_covers_exactly_current_phase_two_families`
5. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-30
- What changed:
  - Added `GoalKind::Patrol { place }` and `EventTag::Patrol` in core.
  - Added `PlannerOpKind::Patrol` plus patrol action classification/semantics in planner ops.
  - Extended the live AI dispatch architecture with a dedicated patrol dispatch key and declaration instead of adding a direct goal-to-op shortcut in `goal_model.rs`.
  - Updated the required exhaustive AI surfaces: goal model, feasibility, failure handling, goal policy, ranking, exhaustion, and agent-tick observation.
  - Added focused patrol tests for goal serialization, dispatch mapping, declaration shape, feasibility, and planner-op semantics.
- Deviations from original plan:
  - The original ticket proposed a direct goal-to-op mapping change in `goal_model.rs`. The actual implementation used `GoalDispatchKey` and `goal_dispatch_decl.rs`, which is the current cleaner extensibility boundary.
  - Patrol motive scoring in ranking is only an identity placeholder (`1`) in this ticket. Real patrol-profile-driven motive weighting remains correctly deferred to E19GUAPAT-004, where patrol candidates are actually emitted.
- Verification results:
  - `cargo test -p worldwake-core` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy --workspace` ✅
