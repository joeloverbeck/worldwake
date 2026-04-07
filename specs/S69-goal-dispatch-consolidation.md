# S69: Goal Dispatch Consolidation

**Phase**: 7 — Consequence Carriers
**Status**: Draft
**Priority**: Medium
**Category**: Architecture refinement — AI planner internals
**Crates**: `worldwake-ai`

## Summary

Consolidate three scattered per-`GoalKind` static property tables — goal family policy, base priority class, and progress barrier ops — into the existing `GoalDispatchDeclaration` struct. This reduces the number of files that must be updated when adding a new `GoalKind` variant from 4-5 to 2 (`goal.rs` for the variant, `goal_dispatch_decl.rs` for all static metadata).

## Dependencies

- None. This spec is a pure refactoring of existing AI internals with no new behavior.

## Design Goals

1. **Single source of truth for static goal metadata**: All per-`GoalKind` properties that are static functions of the variant (no runtime state dependency) live in one struct.
2. **Reduced new-variant checklist**: Adding a `GoalKind` variant requires changes in exactly 2 files: `crates/worldwake-core/src/goal.rs` (the enum variant) and `crates/worldwake-ai/src/goal_dispatch_decl.rs` (the declaration entry).
3. **No behavioral change**: All existing behavior is preserved. This is a structural refactoring, not a design change.

## Non-Goals

- Modifying `GoalKind` itself or moving it between crates.
- Changing how `GoalFamilyPolicy`, `GoalPriorityClass`, or progress barriers work at runtime.
- Consolidating `relevant_observed_commodities()` or `build_payload_override()` — these require runtime inputs (recipe registry, planning state) and cannot be expressed as static struct fields.
- Introducing new goal kinds or changing existing goal behavior.

## FOUNDATIONS Alignment

| Principle | Status | Rationale |
|-----------|--------|-----------|
| P20 — Resource-Bounded Practical Reasoning | Addressed | Reduces the cognitive and mechanical cost of adding new goal variants, making planner extension more tractable |
| P26 — Systems Through State | Satisfied | Goal properties remain static lookup tables, not cross-system calls |
| P27 — Derived Summaries Are Caches | Satisfied | These are not derived summaries — they are authoritative static metadata per goal kind |
| P28 — No Backward Compatibility | Satisfied | The old standalone functions are removed entirely, not wrapped or aliased |

## Deliverables

### 1. Extended `GoalDispatchDeclaration`

In `crates/worldwake-ai/src/goal_dispatch_decl.rs`, add three fields to the existing struct:

```rust
pub struct GoalDispatchDeclaration {
    // --- existing fields ---
    pub trace_label: &'static str,
    pub provenance_family: Option<RankedGoalProvenanceFamily>,
    pub relevant_ops: &'static [PlannerOpKind],
    pub invalidation_strategy: InvalidationStrategy,
    pub feasibility_strategy: FeasibilityStrategy,
    // --- new fields ---
    pub family_policy: GoalFamilyPolicy,
    pub base_priority_class: GoalPriorityClass,
    /// PlannerOpKind values that constitute a progress barrier for this goal.
    /// An empty slice means no progress barriers beyond the default
    /// QueueForFacilityUse check (which applies to all goals that use it).
    pub progress_barrier_ops: &'static [PlannerOpKind],
}
```

### 2. Populate declaration table

Extend each `GoalDispatchKey` variant's declaration entry in the `declaration()` match to include the three new fields. Values are migrated directly from the existing exhaustive matches in `goal_policy.rs`, `ranking.rs`, and `goal_model.rs`.

### 3. Replace standalone consumers

| Current call site | Current function | Replacement |
|-------------------|------------------|-------------|
| `goal_policy.rs` callers | `goal_family_policy(kind)` | `GoalDispatchKey::from_goal_kind(kind).declaration().family_policy` |
| `ranking.rs` callers | inline exhaustive match on `GoalKind` for base priority | `GoalDispatchKey::from_goal_kind(kind).declaration().base_priority_class` |
| `goal_model.rs` callers | `GoalKind::is_progress_barrier(&self, step)` | Check `declaration().progress_barrier_ops.contains(&step.op_kind)` plus the existing `QueueForFacilityUse` default logic |

### 4. Remove emptied code

- Remove `pub fn goal_family_policy()` from `goal_policy.rs` (the struct `GoalFamilyPolicy` and its supporting types remain — they are the field type).
- Remove the base priority exhaustive match from `ranking.rs`.
- Simplify `is_progress_barrier()` in `goal_model.rs` to delegate to the declaration's `progress_barrier_ops` field.

### 5. Preserve `evaluate_suppression()`

`evaluate_suppression()` in `goal_policy.rs` takes runtime `DecisionContext` and cannot be a static field. It stays as-is but reads `family_policy` from the declaration instead of calling the removed `goal_family_policy()`.

## Information-Path Analysis

No new information paths. This is internal planner refactoring — no world state, beliefs, or perception involved.

## Positive-Feedback Analysis

No feedback loops introduced or modified.

## Stored State vs. Derived Read-Model

No new stored state. `GoalDispatchDeclaration` is a compile-time constant table, not ECS state.

## SystemFn Integration

No new system functions. No tick-order changes.

## Component Registration

No new ECS components.

## Cross-System Interactions

None. All changes are internal to `worldwake-ai`. The `GoalFamilyPolicy` and `GoalPriorityClass` types remain in `worldwake-ai` (not cross-crate). `GoalKind` remains in `worldwake-core` unchanged.

## Verification

1. `cargo test -p worldwake-ai` — all existing tests pass with identical behavior
2. `cargo clippy --workspace --all-targets -- -D warnings` — clean
3. Golden soak test (`cargo test -p worldwake-ai --features soak --test golden_soak`) — identical emergence patterns
4. Manual check: adding a hypothetical new `GoalKind` variant triggers exactly 2 compiler errors (one in `goal.rs` exhaustive match, one in `goal_dispatch_decl.rs` declaration table) — no errors in `goal_policy.rs`, `ranking.rs`, or `goal_model.rs`
