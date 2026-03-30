# E19GUAPAT-002: Add GoalKind::Patrol, PlannerOpKind::Patrol, and EventTag::Patrol

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new enum variants in core and AI crates
**Deps**: E19GUAPAT-001 (PatrolRoute/PatrolProfile types must exist)

## Problem

The GOAP planning pipeline needs a `GoalKind::Patrol` variant for candidate generation, a `PlannerOpKind::Patrol` variant for plan search, and an `EventTag::Patrol` variant for event classification. These are mechanical enum additions that unblock the action handler and candidate generation tickets.

## Assumption Reassessment (2026-03-30)

1. `GoalKind` in `crates/worldwake-core/src/goal.rs` currently has 27 variants. No `Patrol` variant exists.
2. `PlannerOpKind` in `crates/worldwake-ai/src/planner_ops.rs` currently has 40 variants. No `Patrol` variant exists.
3. `EventTag` in `crates/worldwake-core/src/event_tag.rs` currently has 21 variants. No `Patrol`-related tag exists.
4. Spec says `GoalKind::Patrol { place: EntityId }` — the `place` field identifies the next waypoint.
5. Spec says `PlannerOpKind::Patrol` with semantics: `may_appear_mid_plan: false`, `is_materialization_barrier: false`, `transition_kind: GoalModelFallback`.
6. Existing `PlannerOpKind` variants each have a `semantics()` method returning `PlannerOpSemantics`. Need to follow that pattern.
7. `GoalKind` derives `Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize` — new variant must be compatible.
8. No adjacent contradictions found.

## Architecture Check

1. Adding enum variants is the established pattern for extending the planning pipeline. Each new goal/op pair follows the same structure. No alternative needed.
2. No backwards-compatibility shims. Pure additions.

## Verification Layers

1. GoalKind::Patrol variant compiles and serializes → focused unit test
2. PlannerOpKind::Patrol semantics are correct → focused unit test checking `semantics()` return
3. EventTag::Patrol is included in match arms → compilation (exhaustive matches enforce this)
4. Single-layer ticket (enum variant additions) — no cross-layer mapping needed.

## What to Change

### 1. Add `GoalKind::Patrol` in `crates/worldwake-core/src/goal.rs`

```rust
Patrol { place: EntityId },
```

Add to any `Display`, `match`, or helper impls that enumerate all variants.

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
    ..Default::default()
},
```

### 4. Update goal-to-op mapping in `crates/worldwake-ai/src/goal_model.rs`

Map `GoalKind::Patrol { .. }` to the appropriate planner op sequence. Follow existing patterns like `GoalKind::InvestigateViolation` mapping.

### 5. Handle new variants in exhaustive matches

Any `match` on `GoalKind`, `PlannerOpKind`, or `EventTag` across the workspace must include the new variants. The compiler will flag these.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify — add variant)
- `crates/worldwake-core/src/event_tag.rs` (modify — add variant)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — add variant + semantics)
- `crates/worldwake-ai/src/goal_model.rs` (modify — add goal-to-op mapping)
- Any files with exhaustive matches on these enums (compiler-guided)

## Out of Scope

- Patrol action handler (E19GUAPAT-003)
- Patrol candidate generation logic (E19GUAPAT-004)
- Route adaptation (E19GUAPAT-006)
- ActionPayload::Patrol (E19GUAPAT-003)
- Any runtime behavior — this ticket adds types only

## Acceptance Criteria

### Tests That Must Pass

1. `GoalKind::Patrol { place }` round-trips through serde
2. `PlannerOpKind::Patrol` semantics returns expected values (`may_appear_mid_plan: false`, `is_materialization_barrier: false`)
3. `EventTag::Patrol` is a valid tag variant
4. Workspace compiles: `cargo build --workspace` (exhaustive match enforcement)
5. Existing suite: `cargo test --workspace`
6. `cargo clippy --workspace`

### Invariants

1. All exhaustive matches on `GoalKind`, `PlannerOpKind`, `EventTag` include new variants
2. No `HashMap` or `f32`/`f64` introduced
3. Serialization format is deterministic

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` — serde round-trip test for `GoalKind::Patrol`
2. `crates/worldwake-ai/src/planner_ops.rs` — semantics assertion for `PlannerOpKind::Patrol`

### Commands

1. `cargo test -p worldwake-core -- goal`
2. `cargo test -p worldwake-ai -- planner_op`
3. `cargo clippy --workspace && cargo test --workspace`
