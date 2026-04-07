# S69: Goal Dispatch Consolidation

**Phase**: 7 — Consequence Carriers
**Status**: Draft
**Priority**: Medium
**Category**: Architecture refinement — AI planner internals + GoalDispatchKey completeness
**Crates**: `worldwake-ai`

## Summary

Consolidate two scattered per-`GoalKind` static property tables — goal family policy and progress barrier ops — into the existing `GoalDispatchDeclaration` struct. As a prerequisite, expand `GoalDispatchKey` with payload-aware variants for `ShareBelief` and `PostNotice` so that all behaviorally distinct goal sub-families have a unique dispatch key. This reduces the number of files that must be updated when adding a new `GoalKind` variant from 5+ to 4 (removing `goal_policy.rs` from the checklist).

## Dependencies

- None. All changes are internal to `worldwake-ai`.

## Design Goals

1. **Single source of truth for static goal metadata**: All per-`GoalKind` properties that are static functions of the variant (no runtime state dependency) live in `GoalDispatchDeclaration`. Runtime-dependent decisions (priority class computation, suppression evaluation) remain in their respective functions.
2. **Reduced new-variant checklist**: Adding a `GoalKind` variant no longer requires updating `goal_policy.rs`. The checklist becomes: `goal.rs` (enum variant), `goal_dispatch_key.rs` (dispatch key + `from_goal_kind`), `goal_dispatch_decl.rs` (declaration entry with family policy and barrier ops), `ranking.rs` (priority class), and `goal_model.rs` (satisfaction, application, residual barrier logic).
3. **Consistent GoalDispatchKey discrimination**: `GoalDispatchKey` already discriminates `AcquireCommodity` by `CommodityPurpose` and `PunishAccused` by `PunishmentKind`. This spec extends that pattern to `ShareBelief` (by `CommunicationClass`) and `PostNotice` (by `NoticeTopic::ThreatWarning` vs other), eliminating an inconsistency.
4. **No behavioral change**: All existing behavior is preserved. This is a structural consolidation, not a design change.

## Non-Goals

- Modifying `GoalKind` itself or moving it between crates.
- Changing how `GoalFamilyPolicy`, `GoalPriorityClass`, or progress barriers work at runtime.
- Consolidating `relevant_observed_commodities()` or `build_payload_override()` — these require runtime inputs (recipe registry, planning state) and cannot be expressed as static struct fields.
- Consolidating `priority_class()` from `ranking.rs` — this function depends on runtime context (agent needs, danger class, pain pressure, recipe output assessment) and cannot be a static field.
- Introducing new goal kinds or changing existing goal behavior.

## FOUNDATIONS Alignment

| Principle | Status | Rationale |
|-----------|--------|-----------|
| P20 — Resource-Bounded Practical Reasoning | Addressed | Reduces the cognitive and mechanical cost of adding new goal variants, making planner extension more tractable |
| P26 — Systems Through State | Satisfied | Goal properties remain static lookup tables, not cross-system calls |
| P27 — Derived Summaries Are Caches | Satisfied | These are not derived summaries — they are authoritative static metadata per goal kind |
| P28 — No Backward Compatibility | Satisfied | The old standalone functions are removed entirely, not wrapped or aliased. GoalDispatchKey expansion replaces single variants with payload-discriminated variants — no compatibility shims |
| Preamble — Architecturally Comprehensive | Satisfied | GoalDispatchKey expansion fixes an existing inconsistency; consolidation cleanly separates static metadata (declaration table) from runtime logic (functions) |

## Deliverables

### 1. Expand `GoalDispatchKey` with payload-aware variants

In `crates/worldwake-ai/src/goal_dispatch_key.rs`, replace the single `ShareBelief` and `PostNotice` variants with payload-discriminated variants:

**Remove:**
- `ShareBelief`
- `PostNotice`

**Add:**
- `ShareBeliefAlarm`
- `ShareBeliefTestimony`
- `ShareBeliefGossip`
- `PostNoticeWarning` (for `NoticeTopic::ThreatWarning`)
- `PostNoticeOther` (for all other `NoticeTopic` variants)

Update `from_goal_kind()` to discriminate:

```rust
GoalKind::ShareBelief { communication_class, .. } => match communication_class {
    CommunicationClass::Alarm => Self::ShareBeliefAlarm,
    CommunicationClass::Testimony => Self::ShareBeliefTestimony,
    CommunicationClass::Gossip => Self::ShareBeliefGossip,
},
GoalKind::PostNotice { topic, .. } => match topic {
    NoticeTopic::ThreatWarning { .. } => Self::PostNoticeWarning,
    _ => Self::PostNoticeOther,
},
```

Update the `ALL` constant and its count accordingly (+3 net variants: 37 total).

### 2. Extended `GoalDispatchDeclaration`

In `crates/worldwake-ai/src/goal_dispatch_decl.rs`, add two fields to the existing struct:

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
    /// PlannerOpKind values that constitute a direct progress barrier for this goal.
    /// These are the per-goal-kind op_kind barriers (e.g., Tell for ShareBelief,
    /// Investigate for InvestigateViolation). The QueueForFacilityUse check,
    /// ConsumeOwnedCommodity/MoveCargo special case, and is_materialization_barrier
    /// logic remain in `is_progress_barrier()` as they depend on step properties
    /// or goal-family membership, not just a flat op_kind list.
    pub progress_barrier_ops: &'static [PlannerOpKind],
}
```

### 3. Populate declaration table

Extend each `GoalDispatchKey` variant's declaration entry in the `declaration()` match to include the two new fields. Values are migrated directly from:
- `goal_family_policy()` in `goal_policy.rs` for `family_policy`
- The per-goal-kind direct op_kind barriers in `is_progress_barrier()` from `goal_model.rs` for `progress_barrier_ops`

The new `ShareBelief*` and `PostNotice*` variants each get their own declaration entry with the correct family policy (e.g., `ShareBeliefAlarm` gets `SuppressionRule::Never`, `ShareBeliefGossip` gets `SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::High)`).

### 4. Replace standalone consumers

| Current call site | Current function | Replacement |
|-------------------|------------------|-------------|
| `interrupts.rs` callers (lines 77, 98, 127) | `goal_family_policy(kind)` | `GoalDispatchKey::from_goal_kind(kind).declaration().family_policy` |
| `goal_policy.rs:220` (`evaluate_suppression`) | `goal_family_policy(kind)` | `GoalDispatchKey::from_goal_kind(kind).declaration().family_policy` |
| `goal_model.rs` per-goal direct barriers | inline `if matches!(self, GoalKind::X) && step.op_kind == PlannerOpKind::Y` checks | Check `declaration().progress_barrier_ops.contains(&step.op_kind)` |

### 5. Remove emptied code

- Remove `pub fn goal_family_policy()` from `goal_policy.rs` (the struct `GoalFamilyPolicy` and its supporting types remain — they are the field type).
- Remove the per-goal-kind direct op_kind barrier checks from `is_progress_barrier()` in `goal_model.rs` (lines 1055-1127 approximately). Replace with a single `progress_barrier_ops.contains()` call. The following logic remains in `is_progress_barrier()`:
  - The `QueueForFacilityUse` check for 7 specific goal families (lines 1042-1053)
  - The `ConsumeOwnedCommodity`/`MoveCargo` special case (lines 1129-1137)
  - The `is_materialization_barrier` flag-based check (lines 1139-1159)

### 6. Preserve `evaluate_suppression()`

`evaluate_suppression()` in `goal_policy.rs` takes runtime `DecisionContext` and cannot be a static field. It stays as-is but reads `family_policy` from the declaration instead of calling the removed `goal_family_policy()`.

### 7. Preserve `priority_class()`

`priority_class()` in `ranking.rs` depends on runtime context (`RankingContext` with agent needs, danger class, pain pressure, recipe output assessment) and cannot be expressed as a static struct field. It stays as-is with no changes.

## Information-Path Analysis

No new information paths. This is internal planner restructuring — no world state, beliefs, or perception involved.

## Positive-Feedback Analysis

No feedback loops introduced or modified.

## Stored State vs. Derived Read-Model

No new stored state. `GoalDispatchDeclaration` is a compile-time constant table, not ECS state.

## SystemFn Integration

No new system functions. No tick-order changes.

## Component Registration

No new ECS components.

## Cross-System Interactions

None. All changes are internal to `worldwake-ai`. The `GoalFamilyPolicy` and `GoalPriorityClass` types remain in `worldwake-ai` (not cross-crate). `GoalKind` remains in `worldwake-core` unchanged. `CommunicationClass` and `NoticeTopic` are read from `GoalKind` payloads in `from_goal_kind()` — they are already accessible in `worldwake-ai` via the `worldwake-core` dependency.

## Verification

1. `cargo test -p worldwake-ai` — all existing tests pass with identical behavior
2. `cargo clippy --workspace --all-targets -- -D warnings` — clean
3. Golden soak test (`cargo test -p worldwake-ai --features soak --test golden_soak`) — identical emergence patterns
4. Manual check: `GoalDispatchKey::ALL` count matches the number of `declaration()` match arms — no missing entries
5. `goal_policy.rs` no longer contains `goal_family_policy()` — grep confirms removal
6. `is_progress_barrier()` retains QueueForFacilityUse, MoveCargo, and materialization barrier logic — verify these code paths are not accidentally removed
