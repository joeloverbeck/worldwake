# E17CRITHEJUS-004: Add GoalKind variants and GoalKey extraction

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — GoalKind enum extension in core, cascading match-arm updates across workspace
**Deps**: E17CRITHEJUS-001 (needs `PunishmentKind`), E17CRITHEJUS-002 (needs `ViolationId` — already exists from S27)

## Problem

No goal kinds exist for theft, accusation, or punishment. The AI pipeline cannot generate, rank, plan, or execute crime/justice goals without `GoalKind::StealItem`, `GoalKind::Accuse`, and `GoalKind::PunishAccused`.

## Assumption Reassessment (2026-03-25)

1. `GoalKind` enum in `crates/worldwake-core/src/goal.rs` currently has 19+ variants. `GoalKey` extraction via `From<&GoalKind>` is implemented for each variant. Adding 3 new variants requires updating both.
2. `ViolationId` is already available from S27 in `crates/worldwake-core/src/violation.rs`.
3. `PunishmentKind` will be added by E17CRITHEJUS-001.
4. This ticket is core-only type additions. The AI-side planner integration (GoalKindTag, PlannerOpKind, goal policy, ranking, feasibility) is E17CRITHEJUS-005.
5. N/A.
6. N/A.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. No mismatches found.
12. N/A.

## Architecture Check

1. Adding GoalKind variants follows the established pattern (S27 added `InvestigateViolation` the same way). `GoalKey` extraction gives each variant a canonical entity-based identity for dedup and blocked-intent matching.
2. No backwards-compatibility aliasing introduced. All exhaustive matches enforced by compiler.

## Verification Layers

1. `GoalKind::StealItem` -> `GoalKey` extracts `target_item` entity -> focused unit test
2. `GoalKind::Accuse` -> `GoalKey` extracts `accused` entity -> focused unit test
3. `GoalKind::PunishAccused` -> `GoalKey` extracts `accused` entity -> focused unit test
4. Serde round-trip for each new variant -> focused unit test
5. Workspace-wide compilation confirms all exhaustive matches updated.

## What to Change

### 1. Add 3 variants to `GoalKind` in `goal.rs`

```rust
StealItem { target_item: EntityId },
Accuse { accused: EntityId, violation_id: ViolationId },
PunishAccused { accused: EntityId, punishment: PunishmentKind },
```

### 2. Implement `GoalKey::from(&GoalKind)` for each

- `StealItem { target_item }` -> `GoalKey::Entity(target_item)`
- `Accuse { accused, .. }` -> `GoalKey::Entity(accused)`
- `PunishAccused { accused, .. }` -> `GoalKey::Entity(accused)`

### 3. Update exhaustive matches across workspace

All `match goal_kind { ... }` blocks in worldwake-ai (goal_model, planner_ops, goal_policy, ranking, feasibility, candidate_generation, search, etc.) need placeholder arms. These placeholders will be filled in by E17CRITHEJUS-005.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify)
- Any file with exhaustive match on `GoalKind` (modify — add placeholder arms across workspace)

## Out of Scope

- `GoalKindTag` additions (E17CRITHEJUS-005)
- `PlannerOpKind` additions (E17CRITHEJUS-005)
- Goal policy, ranking, feasibility for new goals (E17CRITHEJUS-005)
- `matches_binding()` implementation (E17CRITHEJUS-005)
- Candidate generation (E17CRITHEJUS-010/011)
- Action definitions (E17CRITHEJUS-006/008/009)

## Acceptance Criteria

### Tests That Must Pass

1. `GoalKind::StealItem` serde round-trip
2. `GoalKind::Accuse` serde round-trip (with `ViolationId`)
3. `GoalKind::PunishAccused` serde round-trip (both `Fine` and `Exile` punishment kinds)
4. `GoalKey` extraction returns correct entity for each new variant
5. Existing suite: `cargo test -p worldwake-core`
6. Existing suite: `cargo build --workspace` (all match arms compile)

### Invariants

1. `GoalKind` remains `Clone + Debug + Eq + PartialEq + Serialize + Deserialize`
2. `GoalKey` extraction is total (no panics, no unreachable arms)
3. No behavioral change to existing goal kinds

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` — serde round-trip and GoalKey extraction for all 3 new variants

### Commands

1. `cargo test -p worldwake-core`
2. `cargo build --workspace`
3. `cargo clippy --workspace`
