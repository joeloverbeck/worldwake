# E18BANDYN-002: Add RegroupWithFaction and RaidTarget goal kinds

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-core goal.rs, worldwake-ai goal_model.rs
**Deps**: E13 (decision architecture — completed)

## Problem

E18 introduces two new goal kinds: `RegroupWithFaction { faction: EntityId }` (drives survivors to travel to a rally point after camp destruction) and `RaidTarget { target: EntityId }` (drives bandits to raid non-faction agents for goods). These goal kinds must exist before AI candidate generation (E18BANDYN-006) or planner ops (E18BANDYN-007) can reference them.

## Assumption Reassessment (2026-03-29)

1. `GoalKind` enum in `crates/worldwake-core/src/goal.rs` currently has 21 variants. Adding two more follows the established pattern. Each variant carries the minimum data needed to identify the goal target.
2. `GoalKindTag` in `crates/worldwake-ai/src/goal_model.rs` maps `GoalKind` variants to planner operation kinds. Every `GoalKind` variant must have a corresponding `GoalKindTag` entry and `GoalKindPlannerExt` impl.
3. The spec distinguishes Raid from Attack semantically: "bandits generate `RaidTraveler` goal candidates, not generic `EngageHostile`". This justifies a distinct `RaidTarget` goal kind rather than reusing `EngageHostile`.
4. `RegroupWithFaction` maps to `Travel` planner ops (spec Section 8). Priority: below immediate survival, above enterprise.
5. `RaidTarget` maps to `Combat` planner ops (analogous to `EngageHostile` but with bandit-specific candidate generation).
6. `PlannerOpKind` in `crates/worldwake-ai/src/planner_ops.rs` already has `Travel` and `Combat` variants — no new planner op kinds needed, just new mappings.

## Architecture Check

1. Separate goal kinds for raid and regroup follow the existing pattern where semantically distinct motivations get distinct `GoalKind` variants (e.g., `EngageHostile` vs `LootCorpse` vs `BuryCorpse` — all combat-adjacent but with different AI motivations). This enables distinct priority classes and suppression rules without conditional logic.
2. No backwards-compatibility shims. Net-new enum variants only.

## Verification Layers

1. GoalKind variant existence → structural: enum compiles with new variants
2. GoalKindTag mapping completeness → focused unit test: all GoalKind variants have a tag
3. PlannerExt mapping → focused unit test: new goal kinds map to correct planner op kinds
4. Serialization → existing save/load roundtrip (GoalKind is serialized in ActiveGoal component)

## What to Change

### 1. Add GoalKind variants

In `crates/worldwake-core/src/goal.rs`:

```rust
RegroupWithFaction { faction: EntityId },
RaidTarget { target: EntityId },
```

### 2. Add GoalKindTag entries

In `crates/worldwake-ai/src/goal_model.rs`, add corresponding `GoalKindTag` variants and implement the `GoalKindPlannerExt` trait mapping:

- `RegroupWithFaction` → planner ops: `Travel` (travel to believed rally point)
- `RaidTarget` → planner ops: `Combat` (engage target at same location)

### 3. Update exhaustive matches

Any `match` on `GoalKind` throughout the codebase must be updated to handle the new variants. Use `cargo build --workspace` to find all sites.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify — add 2 enum variants)
- `crates/worldwake-ai/src/goal_model.rs` (modify — add GoalKindTag entries + PlannerExt impls)
- Any files with exhaustive `match` on `GoalKind` (modify — add arms)

## Out of Scope

- AI candidate generation logic that produces these goals (E18BANDYN-006)
- Planner search integration and barrier logic (E18BANDYN-007)
- Raid action definition and handler (E18BANDYN-003)
- Priority class assignment and suppression rules (E18BANDYN-007)
- Route threat estimation (E18BANDYN-008)

## Acceptance Criteria

### Tests That Must Pass

1. `GoalKind::RegroupWithFaction` and `GoalKind::RaidTarget` compile and are pattern-matchable
2. All existing `GoalKind` matches updated — `cargo build --workspace` succeeds with no errors
3. `GoalKindTag` mapping test covers new variants
4. Existing suite: `cargo test -p worldwake-core`
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo clippy --workspace`

### Invariants

1. Every `GoalKind` variant has a corresponding `GoalKindTag` (enforced by exhaustive match)
2. Every `GoalKindTag` maps to at least one `PlannerOpKind` via `GoalKindPlannerExt`
3. No existing goal kind behavior changes — only additive

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` tests — verify new variants map to correct planner ops
2. Existing exhaustive-match compile tests serve as structural verification

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
4. `cargo build --workspace`
