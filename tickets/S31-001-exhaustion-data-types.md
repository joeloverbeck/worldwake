# S31-001: Define Exhaustion Invalidation Data Types

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — AI runtime data model
**Deps**: S30 (completed)

## Problem

The exhaustion cache stores only `ExhaustionEntry { exhausted_at, count }`. S31 requires per-goal invalidation conditions and a baseline snapshot. This ticket introduces the new types and extends `ExhaustionEntry` without changing any runtime behavior.

## Assumption Reassessment (2026-03-27)

1. `ExhaustionEntry` currently derives `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize` at `decision_runtime.rs:55`. The `Copy` derive must be removed because new fields contain `Vec`.
2. `HomeostaticNeedId` exists at `crates/worldwake-core/src/needs.rs:18-25` with variants `Hunger, Thirst, Fatigue, Bladder, Dirtiness`. No new enum needed.
3. `HomeostaticNeeds` at `crates/worldwake-core/src/needs.rs` already derives `Serialize, Deserialize, Clone, Eq, PartialEq`.
4. `Permille` at `crates/worldwake-core/src/numerics.rs` already derives `Serialize, Deserialize, Copy, Clone, Eq, PartialEq, Ord, PartialOrd`.
5. `CommodityKind`, `UniqueItemKind`, `EntityId`, `Quantity` all derive the necessary serde + Eq + Ord traits.
6. `ExhaustionEntry` is used in `BTreeMap<GoalKey, ExhaustionEntry>` which requires `Ord`. The new struct must derive `Ord, PartialOrd`.
7. `#[serde(default)]` on new fields ensures backward compatibility with format version 6 saves.
8. No `SAVE_FORMAT_VERSION` bump needed per spec Section "Save/Load Compatibility".
9. Ticket is pure data-type introduction — no runtime behavior changes, no function signature changes.

## Architecture Check

1. Pure additive: new types + field extensions. No behavioral changes. Cleanest possible first step.
2. No backward-compatibility shims. `#[serde(default)]` is the standard serde mechanism, not a shim.

## Verification Layers

1. `ExhaustionEntry` compiles without `Copy` -> compilation success
2. New types derive required traits -> compilation success
3. `#[serde(default)]` backward compat -> unit test (deserialize old format)
4. Single-layer ticket (data model only). No runtime behavior to verify beyond compilation.

## What to Change

### 1. Create `crates/worldwake-ai/src/exhaustion.rs` module

Define `ExhaustionInvalidationCondition` enum with variants:
- `PositionChanged`
- `CommodityChanged(CommodityKind)`
- `UniqueItemChanged(UniqueItemKind)`
- `WoundsChanged`
- `FacilitiesChanged`
- `BlockerExpired`
- `HostilesChanged`
- `NeedCrossedThreshold { need: HomeostaticNeedId, threshold_delta: Permille }`
- `TargetDead(EntityId)`

Define `ExhaustionBaseline` struct with fields:
- `position: Option<EntityId>`
- `needs: Option<HomeostaticNeeds>`
- `commodity_quantities: Vec<(CommodityKind, Quantity)>`
- `unique_item_counts: Vec<(UniqueItemKind, u32)>`
- `wound_count: usize`
- `hostile_count: usize`

Both derive `Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize`. `ExhaustionBaseline` also derives `Default`.

### 2. Extend `ExhaustionEntry` in `decision_runtime.rs`

- Remove `Copy` from derives (new fields contain `Vec`).
- Add `pub invalidation_conditions: Vec<ExhaustionInvalidationCondition>` with `#[serde(default)]`.
- Add `pub baseline: ExhaustionBaseline` with `#[serde(default)]`.

### 3. Fix all `Copy` usage of `ExhaustionEntry`

Grep for any code that relies on `ExhaustionEntry: Copy` (e.g., implicit copies in match arms, assignments). Switch to `.clone()` where needed.

### 4. Register module in `crates/worldwake-ai/src/lib.rs`

Add `pub mod exhaustion;` and re-export the new types.

## Files to Touch

- `crates/worldwake-ai/src/exhaustion.rs` (new)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify)
- `crates/worldwake-ai/src/lib.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — fix any `Copy` usage of `ExhaustionEntry`)

## Out of Scope

- `derive_invalidation_conditions` implementation (S31-002)
- `condition_changed` implementation (S31-003)
- `invalidate_exhausted_goals` implementation (S31-004)
- Any changes to `record_exhausted_goals` (S31-005)
- Removing `EXHAUSTION_SKIP_TTL` (S31-006)
- Golden tests (S31-007)
- `SAVE_FORMAT_VERSION` bump (not needed per spec)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo build -p worldwake-ai` compiles cleanly (no `Copy` errors)
2. Unit test: `ExhaustionEntry` with empty `invalidation_conditions` deserializes from old-format bytes (backward compat)
3. Unit test: `ExhaustionBaseline::default()` produces all-None/empty/zero fields
4. Existing suite: `cargo test --workspace`

### Invariants

1. `ExhaustionEntry` no longer derives `Copy` — enforced by compiler (Vec fields are not Copy)
2. `ExhaustionBaseline::default()` is the zero-value for all fields
3. All new types are `Serialize + Deserialize + Clone + Eq + Ord`
4. No runtime behavior changes — existing tests pass unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/exhaustion.rs` (inline `#[cfg(test)]`) — serde backward compat round-trip
2. `crates/worldwake-ai/src/exhaustion.rs` (inline `#[cfg(test)]`) — `ExhaustionBaseline::default()` field assertions

### Commands

1. `cargo test -p worldwake-ai exhaustion`
2. `cargo clippy --workspace && cargo test --workspace`
