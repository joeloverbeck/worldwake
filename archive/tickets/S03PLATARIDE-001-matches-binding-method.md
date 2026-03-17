# S03PLATARIDE-001: Add `matches_binding()` to `GoalKindPlannerExt`

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `GoalKindPlannerExt` trait gains a new method; `GoalKind` impl block extended
**Deps**: None (all prerequisite types — `GoalKind`, `GoalKey`, `PlannerOpKind`, `GoalKindPlannerExt` — already exist)

## Problem

The planner can currently accept any affordance from the same broad action family without verifying it targets the specific entity that motivated the goal. For example, `LootCorpse { corpse: X }` could match a Loot affordance targeting corpse Y if both are at the same place. This ticket adds the `matches_binding()` method that distinguishes auxiliary ops (always pass) from terminal ops (must verify exact target identity).

## Assumption Reassessment (2026-03-17)

1. `GoalKindPlannerExt` trait is defined in `crates/worldwake-ai/src/goal_model.rs:38` with methods `goal_kind_tag`, `relevant_op_kinds`, `relevant_observed_commodities`, `build_payload_override`, `apply_planner_step`, `goal_relevant_places`, `is_terminal`.
2. `GoalKind` has exactly 17 variants (goal.rs:15–64) matching the spec's classification table.
3. `GoalKey` already extracts canonical `entity: Option<EntityId>` and `place: Option<EntityId>` from `GoalKind` variant fields (goal.rs:67–97).
4. `PlannerOpKind` has exactly 18 variants (planner_ops.rs:13–33) matching the spec's auxiliary/terminal classification.
5. `SearchCandidate` has `authoritative_targets: Vec<EntityId>` field (search.rs:34).

## Architecture Check

1. Reading canonical target identity directly from `GoalKind` variant fields avoids a parallel `GoalBindingPolicy` enum that would duplicate information and add sync burden.
2. The method signature `matches_binding(&self, authoritative_targets: &[EntityId], op_kind: PlannerOpKind) -> bool` keeps all matching state-derived and deterministic.
3. No backward-compatibility shims or aliases introduced.

## What to Change

### 1. Add `matches_binding` to `GoalKindPlannerExt` trait definition

In `crates/worldwake-ai/src/goal_model.rs`, add to the trait:

```rust
fn matches_binding(
    &self,
    authoritative_targets: &[EntityId],
    op_kind: PlannerOpKind,
) -> bool;
```

### 2. Implement `matches_binding` for `GoalKind`

In the `impl GoalKindPlannerExt for GoalKind` block in `goal_model.rs`, implement the dispatch logic:

- **Empty `authoritative_targets`** → return `true` (planner-only synthetic candidates bypass binding).
- **Auxiliary ops** (`Travel`, `Trade`, `Harvest`, `Craft`, `QueueForFacilityUse`, `MoveCargo`, `Consume`, `Sleep`, `Relieve`, `Wash`, `Defend`, `Bribe`, `Threaten`) → return `true` unconditionally.
- **Terminal ops** (`Attack`, `Loot`, `Heal`, `Tell`, `DeclareSupport`, `Bury`) → dispatch on `GoalKind` variant:
  - Flexible goals (`ConsumeOwnedCommodity`, `AcquireCommodity`, `Sleep`, `Relieve`, `Wash`, `ReduceDanger`, `ProduceCommodity`, `SellCommodity`, `RestockCommodity`) → `true`
  - `EngageHostile { target }` with `Attack` → `authoritative_targets.contains(&target)`
  - `Heal { target }` with `Heal` → `authoritative_targets.contains(&target)`
  - `LootCorpse { corpse }` with `Loot` → `authoritative_targets.contains(&corpse)`
  - `BuryCorpse { corpse, burial_site }` with `Bury` → `authoritative_targets.contains(&corpse)` (burial_site is place-level, checked if authoritative_targets includes it)
  - `ShareBelief { listener, .. }` with `Tell` → `authoritative_targets.contains(&listener)`
  - `ClaimOffice { .. }` / `SupportCandidateForOffice { .. }` with `DeclareSupport` → `true` (empty `bound_targets` handled by empty-targets bypass; if non-empty, these don't need tighter binding per spec)
  - `MoveCargo { destination, .. }` with `MoveCargo` (terminal for this goal) → `authoritative_targets.contains(&destination)`

### 3. Add unit tests for `matches_binding`

Per-variant tests in `goal_model.rs` (or a new `#[cfg(test)] mod matches_binding_tests` block):

- **Match**: `LootCorpse { corpse: X }` + `[X]` + `Loot` → `true`
- **Mismatch**: `LootCorpse { corpse: X }` + `[Y]` + `Loot` → `false`
- **Auxiliary bypass**: `LootCorpse { corpse: X }` + `[destination]` + `Travel` → `true`
- **Empty targets bypass**: `LootCorpse { corpse: X }` + `[]` + `Loot` → `true`
- **Flexible goal**: `Sleep` + any targets + any op → `true`
- **EngageHostile match/mismatch**: target A + `[A]`/`[B]` + `Attack`
- **Heal match/mismatch**: target T + `[T]`/`[U]` + `Heal`
- **ShareBelief match/mismatch**: listener L + `[L]`/`[M]` + `Tell`
- **MoveCargo destination match/mismatch**: destination D + `[D]`/`[E]` + `MoveCargo`
- **ClaimOffice with DeclareSupport**: always true (spec edge case)
- **SupportCandidateForOffice with DeclareSupport**: always true

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — trait definition + impl + tests)

## Out of Scope

- Wiring `matches_binding()` into `search_candidates()` — that is S03PLATARIDE-002.
- `BindingRejection` trace struct — that is S03PLATARIDE-003.
- Search integration tests with multiple entities — that is S03PLATARIDE-004.
- Any changes to `worldwake-core` (no new components or `GoalKind` variant changes).
- Any changes to `worldwake-sim` affordance enumeration.
- Any changes to `worldwake-systems` action handlers.
- `BuryCorpse` golden test coverage (no action def exists yet).

## Acceptance Criteria

### Tests That Must Pass

1. `matches_binding_loot_corpse_match` — correct corpse + Loot op → true
2. `matches_binding_loot_corpse_mismatch` — wrong corpse + Loot op → false
3. `matches_binding_auxiliary_bypass` — exact-bound goal + auxiliary op → true
4. `matches_binding_empty_targets_bypass` — exact-bound goal + empty targets → true
5. `matches_binding_flexible_goal` — Sleep/Consume/etc. + any targets + any op → true
6. `matches_binding_engage_hostile_match` and `_mismatch`
7. `matches_binding_heal_match` and `_mismatch`
8. `matches_binding_share_belief_match` and `_mismatch`
9. `matches_binding_move_cargo_destination_match` and `_mismatch`
10. `matches_binding_declare_support_always_passes`
11. Existing suite: `cargo test -p worldwake-ai`
12. `cargo clippy --workspace`

### Invariants

1. All 17 `GoalKind` variants must be covered by `matches_binding` (enforced by exhaustive match).
2. Flexible goals never reject any candidate regardless of targets or op kind.
3. Empty `authoritative_targets` always returns `true` regardless of goal or op kind.
4. Auxiliary ops always return `true` regardless of goal or targets.
5. Planner determinism is preserved — `matches_binding` is a pure function of its inputs.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — new `#[cfg(test)] mod matches_binding_tests` with 12+ unit tests covering match, mismatch, auxiliary bypass, empty bypass, and flexible-goal cases for all exact-bound variants.

### Commands

1. `cargo test -p worldwake-ai -- matches_binding`
2. `cargo test --workspace && cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-17
- **What changed**: Added `matches_binding()` method to `GoalKindPlannerExt` trait and implemented it on `GoalKind` in `crates/worldwake-ai/src/goal_model.rs`. Three-layer dispatch: empty targets bypass, auxiliary ops bypass, terminal ops check goal-specific target identity. Added 22 unit tests covering all acceptance criteria.
- **Deviations**: Clippy required merging `EngageHostile`/`Heal` arms (identical `contains(target)` bodies) and merging `ClaimOffice`/`SupportCandidateForOffice` with the flexible goals arm (all return `true`). Semantically identical to the ticket's specification. Ticket assumption #1 listed `is_terminal` as a trait method but the actual methods are `is_progress_barrier` and `is_satisfied` — no impact on implementation.
- **Verification**: `cargo test --workspace` all pass, `cargo clippy --workspace` clean.
