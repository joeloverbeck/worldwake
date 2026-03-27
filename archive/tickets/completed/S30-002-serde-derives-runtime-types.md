# S30-002: Add serde derives to AI runtime types needed for save/load parity

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — serde derives on AI runtime persistence types plus focused round-trip coverage
**Deps**: `archive/tickets/completed/S30-001-exhaustion-entry-unification.md`, `specs/S30-ai-runtime-save-load-parity.md`

## Problem

`AgentDecisionRuntime`, `MaterializationBindings`, and `ExhaustionEntry` still lack `Serialize`/`Deserialize` derives. That blocks the S30 save/load-parity architecture from treating AI runtime continuity as persistable AI-layer state instead of silently dropping it at the save/load boundary.

## Assumption Reassessment (2026-03-27)

1. The exact abstraction boundary under audit is the AI runtime persistence contract inside [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs): the fields of `AgentDecisionRuntime` that S30 classifies as persistable versus derived.
2. The original ticket text is stale about `ExhaustionEntry`. S30-001 already introduced `ExhaustionEntry` in [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs), and the live shape is `pub exhausted_at: Option<Tick>, pub count: u8`, not the older spec sketch `Tick + count`.
3. `MaterializationBindings` and `AgentDecisionRuntime` still derive only non-serde traits in [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs). The lack of serde derives is the real remaining gap for this ticket.
4. The live persistable fields still match the S30 spec: `current_plan`, `current_step_index`, `step_in_flight`, `last_effective_place`, `last_needs`, `last_wounds`, `last_commodity_signature`, `last_unique_item_signature`, `last_facility_access_signature`, `last_in_transit`, `materialization_bindings`, and `exhaustion_cache`.
5. The live derived-only fields also match the S30 spec and should stay non-persisted: `dirty: DirtySet`, `last_priority_class: Option<GoalPriorityClass>`, and `last_frame_clear_reason: Option<FrameClearReason>`. `DirtySet::default()` is the empty set in [`crates/worldwake-ai/src/dirty_set.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/dirty_set.rs); later S30 work is responsible for setting `DirtySet::all()` after restore.
6. The nested type surface is already largely serde-ready in current code: `PlanningBudget` in [`crates/worldwake-ai/src/budget.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/budget.rs), `PlannedPlan` in [`crates/worldwake-ai/src/planner_ops.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs), `HypotheticalEntityId` in [`crates/worldwake-ai/src/planning_state.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs), `GoalKey` in [`crates/worldwake-core/src/goal.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs), and the snapshot/value types under `worldwake-core` already derive serde.
7. `GoalPriorityClass` and `FrameClearReason` already derive serde in current code, but that does not change this ticket’s boundary. These fields should still be skipped because S30 classifies them as derived/diagnostic state rather than canonical persisted AI runtime.
8. `worldwake-ai/Cargo.toml` already depends on `serde = { version = "1", features = ["derive"] }`. No dependency change is needed unless compilation proves another transitive type is missing serde.
9. The original ticket’s test plan was too vague. `cargo test -p worldwake-ai -- --list` confirms there is no current focused serde test for `decision_runtime`, so this ticket should add explicit round-trip tests in [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs) and run a real narrow command against them.
10. Architecture note: the benefit here is not “serde everywhere.” The clean design is to persist only AI-layer runtime facts that carry decision continuity across save/load, while continuing to treat world state as authoritative and derived runtime caches as reconstructable. That matches Foundational Principles 11, 24, and 25 better than either dropping runtime continuity or promoting the whole runtime indiscriminately.

## Architecture Check

1. Adding serde derives directly to the canonical runtime types is cleaner than introducing parallel save structs or alias wrappers for this layer. The runtime types under audit already define the persisted boundary; duplicating them would create an unnecessary second schema to keep in sync.
2. Using `#[serde(skip)]` on `dirty`, `last_priority_class`, and `last_frame_clear_reason` is the right long-term architecture because those values are derived or diagnostic, not source-of-truth AI commitments. Persisting them would blur the line between canonical runtime continuity and recomputable cache state.
3. Keeping `ExhaustionEntry` serializable is more robust than trying to special-case the exhaustion cache later. The unified entry from S30-001 is already the canonical per-goal record and should remain so across future S31 invalidation work.
4. No backward-compatibility aliasing or shim layer should be added. The types under audit either become the persisted contract now or they do not.

## Verification Layers

1. `MaterializationBindings` value round-trip -> focused unit test in `decision_runtime.rs`
2. `AgentDecisionRuntime` persisted fields round-trip while derived fields reset -> focused unit test in `decision_runtime.rs`
3. New derives do not break existing AI behavior/build surfaces -> `cargo test -p worldwake-ai`
4. Workspace serialization/build/lint surface remains clean -> `cargo test --workspace` and `cargo clippy --workspace`

## What to Change

### 1. Add serde derives to `ExhaustionEntry`

Update the live S30-001 struct in `decision_runtime.rs`:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ExhaustionEntry {
    pub exhausted_at: Option<Tick>,
    pub count: u8,
}
```

### 2. Add serde derives to `MaterializationBindings`

```rust
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MaterializationBindings { ... }
```

### 3. Add serde derives to `AgentDecisionRuntime` and skip derived fields

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

### 4. Add focused serde round-trip tests

Add tests in `decision_runtime.rs` that prove:

- `MaterializationBindings` round-trips through `bincode`
- `AgentDecisionRuntime` round-trips all persisted fields
- skipped fields deserialize to their defaults rather than preserving pre-serialization values

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` (modify — serde derives and focused tests)
- `tickets/S30-002-serde-derives-runtime-types.md` (modify, then archive on completion)

## Out of Scope

- `SaveableRuntime` trait definition and opaque AI payload wiring (later S30 tickets)
- `AgentTickDriver` save/restore implementation
- post-load validation/reinitialization logic such as `DirtySet::all()`
- save format version changes
- golden harness save/load runtime restoration work
- any behavioral change to AI planning logic

## Acceptance Criteria

### Tests That Must Pass

1. New focused serde tests in `decision_runtime.rs`
2. `cargo test -p worldwake-ai decision_runtime`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

### Invariants

1. `AgentDecisionRuntime` can be serialized/deserialized without introducing alias types or persistence wrappers.
2. Persisted runtime fields round-trip exactly through `bincode`.
3. Derived fields (`dirty`, `last_priority_class`, `last_frame_clear_reason`) do not persist and deserialize to defaults.
4. `ExhaustionEntry` remains the single canonical persisted exhaustion record per `GoalKey`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_runtime.rs` — add a `materialization_bindings_bincode_round_trip_preserves_entries` test to prove hypothetical-to-authoritative mappings persist exactly.
2. `crates/worldwake-ai/src/decision_runtime.rs` — add an `agent_decision_runtime_bincode_round_trip_skips_derived_fields` test to prove persisted runtime continuity and derived-field reset semantics.

### Commands

1. `cargo test -p worldwake-ai decision_runtime`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-27
- What actually changed:
  - Corrected the ticket assumptions to match the live S30-001 runtime shape, especially that `ExhaustionEntry` already existed and uses `Option<Tick>` for `exhausted_at`.
  - Added `Serialize`/`Deserialize` derives directly to `MaterializationBindings`, `ExhaustionEntry`, and `AgentDecisionRuntime`.
  - Marked the derived-only runtime fields `dirty`, `last_priority_class`, and `last_frame_clear_reason` with `#[serde(skip)]`.
  - Added focused `bincode` round-trip coverage for `MaterializationBindings` and `AgentDecisionRuntime`.
- Deviations from original plan:
  - No `Cargo.toml` dependency change was required because `serde` was already present with `derive`.
  - The final work stayed intentionally narrow. It did not implement runtime restore wiring, save-format changes, or post-load validation; those remain later S30 scope.
- Verification results:
  - `cargo test -p worldwake-ai decision_runtime` passed.
  - `cargo test -p worldwake-ai` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace` passed.
