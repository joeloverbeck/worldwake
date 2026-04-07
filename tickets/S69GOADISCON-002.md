# S69GOADISCON-002: Add family_policy and progress_barrier_ops fields to GoalDispatchDeclaration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — struct field additions, no runtime behavior change
**Deps**: S69GOADISCON-001

## Problem

Static per-goal-family metadata (family policy, progress barrier op kinds) is scattered across standalone functions (`goal_family_policy()` in `goal_policy.rs`, per-goal-kind op_kind checks in `is_progress_barrier()` in `goal_model.rs`). This ticket adds these as fields on `GoalDispatchDeclaration` and populates all 37 declaration entries, establishing the declaration table as the single source of truth for static goal metadata.

## Assumption Reassessment (2026-04-07)

1. `GoalDispatchDeclaration` is defined in `crates/worldwake-ai/src/goal_dispatch_decl.rs` at line 43 with 5 existing fields: `trace_label`, `provenance_family`, `relevant_ops`, `invalidation_strategy`, `feasibility_strategy`. Confirmed current.
2. `GoalFamilyPolicy` struct in `goal_policy.rs:71-75` has 3 fields: `suppression: SuppressionRule`, `penalty_interrupt: PenaltyInterruptEligibility`, `free_interrupt: FreeInterruptRole`. All types are in `worldwake-ai`.
3. `goal_family_policy()` in `goal_policy.rs:103-212` is an exhaustive match returning `GoalFamilyPolicy` per `GoalKind`. After S69GOADISCON-001, the new `GoalDispatchKey` sub-variants align with the payload discriminations in this function (ShareBelief by CommunicationClass, PostNotice by ThreatWarning).
4. `is_progress_barrier()` in `goal_model.rs:1041-1159` has per-goal-kind direct op_kind barriers at lines 1055-1127 (13 match arms). These map to a `&'static [PlannerOpKind]` slice per dispatch key.
5. `PlannerOpKind` is in `crates/worldwake-ai/src/planner_ops.rs` — same crate, no boundary issue.
6. The struct uses `const` declarations (e.g., `const DECL_CONSUME_OWNED_COMMODITY: GoalDispatchDeclaration = ...`). New fields must be const-constructible. `GoalFamilyPolicy` contains only `Copy` enums — confirmed const-constructible.

## Architecture Check

1. Centralizing static metadata in the declaration table follows the existing pattern (trace_label, provenance_family, relevant_ops, invalidation/feasibility strategies are already there). This is a natural extension, not a new pattern.
2. No backward-compatibility shims. The old functions remain temporarily (consumers still call them) — they are removed in ticket 003.

## Verification Layers

1. Declaration field correctness → new test `test_family_policy_matches_standalone_function` comparing declaration values against `goal_family_policy()` output for every GoalKind
2. Barrier ops correctness → new test `test_progress_barrier_ops_match_goal_model` comparing declaration values against `is_progress_barrier()` direct op_kind logic
3. Declaration completeness → existing `test_declaration_completeness` still passes
4. Single-layer ticket (AI planner internals only) — no cross-system verification needed

## What to Change

### 1. Extend `GoalDispatchDeclaration` struct

Add two fields:

```rust
pub struct GoalDispatchDeclaration {
    // existing fields...
    pub family_policy: GoalFamilyPolicy,
    pub progress_barrier_ops: &'static [PlannerOpKind],
}
```

### 2. Populate all 37 declaration constants

For each `DECL_*` constant, add the two new fields. Values come from:

- **`family_policy`**: Migrated from `goal_family_policy()` in `goal_policy.rs`. Each dispatch key maps to the exact `GoalFamilyPolicy` value that the standalone function returns for the corresponding `GoalKind` variant(s). The new ShareBelief sub-variants get differentiated policies:
  - `ShareBeliefAlarm`: `SuppressionRule::Never`
  - `ShareBeliefTestimony`: `SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::Critical)`
  - `ShareBeliefGossip`: `SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::High)`
  - `PostNoticeWarning`: enterprise policy (no suppression)
  - `PostNoticeOther`: stress-suppressed policy

- **`progress_barrier_ops`**: Migrated from the per-goal-kind direct op_kind barriers in `is_progress_barrier()`. Examples:
  - `ShareBeliefAlarm/Testimony/Gossip`: `&[PlannerOpKind::Tell]`
  - `InvestigateViolation`: `&[PlannerOpKind::Investigate]`
  - `Patrol`: `&[PlannerOpKind::Patrol]`
  - `FulfillBounty`: `&[PlannerOpKind::ClaimBounty]`
  - `SellCommodity`: `&[PlannerOpKind::StaffMarket]`
  - `PunishFine`: `&[PlannerOpKind::Fine]`
  - `PunishExile`: `&[PlannerOpKind::Exile]`
  - `ClaimOffice`/`SupportCandidateForOffice`: `&[PlannerOpKind::DeclareSupport, PlannerOpKind::PressForceClaim]`
  - Goals with no direct op_kind barriers: `&[]`

### 3. Add cross-validation tests

Add tests that iterate over all `GoalKind` variants and verify:
- `GoalDispatchKey::from_goal_kind(kind).declaration().family_policy` equals `goal_family_policy(kind)`
- Each op in `progress_barrier_ops` is actually a barrier for that goal kind (validated against the current `is_progress_barrier()` logic for direct op_kind checks only — not QueueForFacilityUse, MoveCargo, or materialization barrier layers)

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify)
- `crates/worldwake-ai/src/goal_policy.rs` (read-only reference for value migration)
- `crates/worldwake-ai/src/goal_model.rs` (read-only reference for value migration)

## Out of Scope

- Removing `goal_family_policy()` or changing its callers (ticket 003)
- Simplifying `is_progress_barrier()` or changing its callers (ticket 003)
- Modifying `ranking.rs` or `priority_class()` (explicitly excluded from S69 — runtime-dependent)
- Adding new ECS components or system functions

## Acceptance Criteria

### Tests That Must Pass

1. `test_declaration_completeness` — every GoalDispatchKey has a declaration with all fields populated
2. New: `test_family_policy_matches_standalone_function` — declaration family_policy equals goal_family_policy() for every GoalKind
3. New: `test_progress_barrier_ops_match_goal_model` — declaration barrier ops are consistent with is_progress_barrier() direct op_kind checks
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Every `GoalDispatchDeclaration` has a valid `family_policy` matching the standalone function's output
2. `progress_barrier_ops` contains only `PlannerOpKind` values that the current `is_progress_barrier()` treats as direct barriers for that goal kind
3. No runtime behavior changes — old functions and new fields coexist temporarily

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_dispatch_decl.rs::test_family_policy_matches_standalone_function` — proves declaration values match standalone function for all GoalKind variants
2. `crates/worldwake-ai/src/goal_dispatch_decl.rs::test_progress_barrier_ops_match_goal_model` — proves barrier ops are correct for all dispatch keys

### Commands

1. `cargo test -p worldwake-ai -- test_declaration`
2. `cargo test -p worldwake-ai -- test_family_policy`
3. `cargo test -p worldwake-ai -- test_progress_barrier_ops`
4. `cargo clippy --workspace --all-targets -- -D warnings`
