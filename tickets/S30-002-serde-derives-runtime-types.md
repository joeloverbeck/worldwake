# S30-002: Add serde derives to AgentDecisionRuntime and supporting types

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — serde derives on AI runtime types
**Deps**: S30-001 (ExhaustionEntry must exist before deriving Serialize on AgentDecisionRuntime)

## Problem

`AgentDecisionRuntime` and `MaterializationBindings` lack `Serialize, Deserialize` derives, which blocks serializing AI runtime state across save/load boundaries. Some fields are derived caches that must be skipped during serialization (reconstructed after load).

## Assumption Reassessment (2026-03-27)

1. `AgentDecisionRuntime` currently derives `Clone, Debug, Default, Eq, PartialEq` (`decision_runtime.rs:54`). No serde.
2. `MaterializationBindings` currently derives `Clone, Debug, Default, Eq, PartialEq` (`decision_runtime.rs:29`). No serde.
3. Fields classified as **Derive** (skip during serde): `dirty: DirtySet`, `last_priority_class: Option<GoalPriorityClass>`, `last_frame_clear_reason: Option<FrameClearReason>`.
4. All **Serialize** field types already have serde derives: `PlannedPlan` (`planner_ops.rs:781`), `GoalKey` (`goal.rs:86`), `PlanningBudget` (`budget.rs:4`), `Tick`, `EntityId`, `HomeostaticNeeds`, `Wound`, `CommodityKind`, `Quantity`, `UniqueItemKind`, `ActionDefId`, `HypotheticalEntityId`. Verified via grep.
5. `DirtySet` is a `u16` newtype — does NOT need serde (skipped and reconstructed to `all()` after load).
6. `GoalPriorityClass` — check if it has serde. If not, it doesn't matter because the field is skipped.
7. `FrameClearReason` — same: skipped, no serde needed.
8. After S30-001, `ExhaustionEntry` will exist. This ticket adds `Serialize, Deserialize` to it.

## Architecture Check

1. Using `#[serde(skip)]` on derived fields is the standard Rust pattern for partial serialization. After deserialization, `Default::default()` values are used for skipped fields — `DirtySet::default()` must equal the empty set (post-load code in S30-005 sets it to `all()`).
2. No backward-compatibility shims. Types either have serde or they don't.

## Verification Layers

1. Serde round-trip correctness → focused unit test: serialize `AgentDecisionRuntime` → deserialize → assert serialized fields match, derived fields are default
2. Single-layer ticket (type derives) — no cross-layer mapping needed.

## What to Change

### 1. Add `Serialize, Deserialize` to `ExhaustionEntry`

In `decision_runtime.rs`, update the derive to:
```rust
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ExhaustionEntry { ... }
```

### 2. Add `Serialize, Deserialize` to `MaterializationBindings`

```rust
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MaterializationBindings { ... }
```

### 3. Add `Serialize, Deserialize` to `AgentDecisionRuntime` with `#[serde(skip)]`

```rust
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentDecisionRuntime {
    pub current_plan: Option<PlannedPlan>,
    pub current_step_index: usize,
    #[serde(skip)]
    pub last_frame_clear_reason: Option<FrameClearReason>,
    pub step_in_flight: bool,
    #[serde(skip)]
    pub dirty: DirtySet,
    #[serde(skip)]
    pub last_priority_class: Option<GoalPriorityClass>,
    pub last_effective_place: Option<EntityId>,
    pub last_needs: Option<HomeostaticNeeds>,
    pub last_wounds: Vec<Wound>,
    pub last_commodity_signature: Vec<(CommodityKind, Quantity)>,
    pub last_unique_item_signature: Vec<(UniqueItemKind, u32)>,
    pub last_facility_access_signature: Vec<(EntityId, bool, Option<ActionDefId>)>,
    pub last_in_transit: bool,
    pub materialization_bindings: MaterializationBindings,
    pub exhaustion_cache: BTreeMap<GoalKey, ExhaustionEntry>,
}
```

### 4. Add serde imports

Ensure `use serde::{Serialize, Deserialize};` is present in `decision_runtime.rs`. Check that `worldwake-ai/Cargo.toml` already depends on `serde` with `derive` feature (it should, given `PlanningBudget` and `PlannedPlan` already derive serde in this crate).

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` (modify — add serde derives and skip annotations)
- `crates/worldwake-ai/Cargo.toml` (modify — only if serde dependency is missing, unlikely)

## Out of Scope

- `SaveableRuntime` trait definition (S30-003)
- `AgentTickDriver` serialization implementation (S30-004)
- Post-load validation logic (S30-005)
- Save format changes (S30-003)
- Any behavioral changes to AI decision logic

## Acceptance Criteria

### Tests That Must Pass

1. New focused unit test: bincode round-trip of `AgentDecisionRuntime` with populated serialized fields and default derived fields
2. New focused unit test: bincode round-trip of `MaterializationBindings` with entries
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace && cargo test --workspace`

### Invariants

1. Skipped fields (`dirty`, `last_priority_class`, `last_frame_clear_reason`) deserialize to `Default::default()`, not to the pre-serialization value
2. All serialized fields round-trip identically through bincode
3. Serialization is deterministic (`BTreeMap` ordering preserved, no floats)
4. No new ECS components

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_runtime.rs` (or test module) — `test_runtime_serde_round_trip`: construct a runtime with plan, exhaustion entries, materialization bindings, snapshot anchors; serialize via bincode; deserialize; assert serialized fields match and skipped fields are default.

### Commands

1. `cargo test -p worldwake-ai decision_runtime`
2. `cargo clippy --workspace && cargo test --workspace`
