# S31-001: Define Exhaustion Invalidation Data Types

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — AI runtime data model
**Deps**: S30 (completed)

## Problem

The exhaustion cache stores only `ExhaustionEntry { exhausted_at, count }`. S31 requires per-goal invalidation conditions and a baseline snapshot. This ticket introduces the new types and extends `ExhaustionEntry` without changing any runtime behavior.

## Assumption Reassessment (2026-03-27)

1. The shared abstraction boundary under audit is `AgentDecisionRuntime.exhaustion_cache: BTreeMap<GoalKey, ExhaustionEntry>` plus its serialized transport through `AgentTickDriverState` runtime bytes (`crates/worldwake-ai/src/decision_runtime.rs`, `crates/worldwake-ai/src/agent_tick/mod.rs`). This is a runtime-data-model ticket, but it is also a save/load-boundary ticket because the cache persists through raw `bincode`.
2. `ExhaustionEntry` currently derives `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize` in `crates/worldwake-ai/src/decision_runtime.rs`. `Copy` must be removed once the entry owns `Vec`-backed state.
3. `HomeostaticNeedId` already exists in `crates/worldwake-core/src/needs.rs`; `Permille`, `CommodityKind`, `UniqueItemKind`, `EntityId`, and `Quantity` already satisfy the needed serde/value-trait bounds. `HomeostaticNeeds` currently derives `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize` but not ordering, so S31-001 must either add `Ord + PartialOrd` at the core type or weaken the exhaustion value contract. The cleaner option is to make `HomeostaticNeeds` fully ordered because it is a concrete value object.
4. The previous ticket draft incorrectly claimed `ExhaustionEntry` needs `Ord` because it is the value in a `BTreeMap`. `BTreeMap<GoalKey, ExhaustionEntry>` requires `Ord` on `GoalKey`, not on `ExhaustionEntry`. Keeping `Ord` on the entry is acceptable for deterministic test comparisons, but it is not structurally required by the map.
5. The previous ticket draft also incorrectly claimed `#[serde(default)]` alone preserves compatibility with existing format-6 runtime bytes. Live code serializes `AgentTickDriverState` and deserializes it directly with `bincode::deserialize` in `AgentTickDriver::restore_runtime_state`; `bincode` is not self-describing, so changing the encoded `ExhaustionEntry` layout without an explicit format transition is not a safe compatibility story.
6. Because the runtime payload schema changes, this ticket cannot honestly remain "no save-format impact". A clean implementation must either:
   - bump the top-level save format and handle the old runtime payload explicitly, or
   - stop claiming compatibility with pre-change runtime bytes.
   S31-001 should take the explicit-format path rather than relying on accidental `bincode` behavior.
7. `reset_exhausted_goals_if_needed` / TTL invalidation still lives in `crates/worldwake-ai/src/agent_tick/planning.rs`. This ticket should not change invalidation behavior yet; it only prepares the data contract that later tickets will consume.

## Architecture Check

1. Splitting the new types into a dedicated `crates/worldwake-ai/src/exhaustion.rs` module is cleaner than growing `planning.rs` into the owner of both policy and persistence types. The invalidation schema is shared runtime state, not planning-loop glue.
2. Extending `ExhaustionEntry` now, before behavior changes, keeps the later S31 tickets surgical: derivation, change detection, and invalidation logic can build on one stable data contract instead of repeatedly reshaping persisted runtime state.
3. Explicit save-format handling is architecturally cleaner than pretending raw `bincode` layout changes are backward-compatible. If the serialized contract changes, the format boundary must say so.

## Verification Layers

1. Type-level exhaustion contract -> focused unit tests in `decision_runtime.rs` / `exhaustion.rs`
2. Runtime persistence boundary (`AgentTickDriverState` save/load bytes) -> focused unit tests in `agent_tick/tests.rs`
3. Save-format boundary (`worldwake-sim` header versioning) -> focused unit tests in `crates/worldwake-sim/src/save_load.rs`
4. No planner-behavior change in this ticket -> existing planning tests must continue to pass unchanged after literal updates

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

### 5. Make the persistence boundary explicit

- Update runtime/save tests to cover the expanded `ExhaustionEntry`.
- Bump the top-level save format in `crates/worldwake-sim/src/save_load.rs` and keep the version boundary honest for the new runtime payload schema.
- Do not add alias loaders or silent shims inside the AI runtime; the format layer owns format transitions.

## Files to Touch

- `crates/worldwake-ai/src/exhaustion.rs` (new)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify)
- `crates/worldwake-ai/src/lib.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — update literals / fix any `Copy` usage of `ExhaustionEntry`)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)
- `crates/worldwake-core/src/needs.rs` (modify)
- `crates/worldwake-sim/src/save_load.rs` (modify)

## Out of Scope

- `derive_invalidation_conditions` implementation (S31-002)
- `condition_changed` implementation (S31-003)
- `invalidate_exhausted_goals` implementation (S31-004)
- Any changes to `record_exhausted_goals` (S31-005)
- Removing `EXHAUSTION_SKIP_TTL` (S31-006)
- Golden tests (S31-007)
- Goal-aware invalidation behavior itself
- Any runtime migration more complex than the explicit save-format transition needed by this schema change

## Acceptance Criteria

### Tests That Must Pass

1. `cargo build -p worldwake-ai` compiles cleanly (no `Copy` errors)
2. Unit test: `ExhaustionBaseline::default()` produces all-None/empty/zero fields
3. Unit test: `AgentDecisionRuntime` / `AgentTickDriverState` bincode round-trips preserve the new exhaustion fields
4. Focused save/load test coverage passes with the explicit save-format update
5. Existing suite: `cargo test --workspace`

### Invariants

1. `ExhaustionEntry` no longer derives `Copy` — enforced by compiler (Vec fields are not Copy)
2. `ExhaustionBaseline::default()` is the zero-value for all fields
3. All new types are `Serialize + Deserialize + Clone + Eq + Ord`
4. Runtime save bytes and top-level save version no longer rely on accidental compatibility with the pre-change `ExhaustionEntry` layout
5. No invalidation-policy behavior changes land in this ticket

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/exhaustion.rs` (new inline `#[cfg(test)]`) — `ExhaustionBaseline::default()` field assertions
2. `crates/worldwake-ai/src/decision_runtime.rs` — runtime bincode round-trip preserves `invalidation_conditions` and `baseline`
3. `crates/worldwake-ai/src/agent_tick/tests.rs` — saved driver state preserves expanded exhaustion entries
4. `crates/worldwake-sim/src/save_load.rs` — save header version reflects the runtime schema change

### Commands

1. `cargo test -p worldwake-ai exhaustion::tests::exhaustion_baseline_default_is_zero_value -- --exact`
2. `cargo test -p worldwake-ai decision_runtime::tests::agent_decision_runtime_bincode_round_trip_skips_derived_fields -- --exact`
3. `cargo test -p worldwake-ai agent_tick::tests::save_runtime_state_serializes_persisted_driver_state -- --exact`
4. `cargo test -p worldwake-ai agent_tick::tests::from_saved_runtime_restores_and_validates_driver_state -- --exact`
5. `cargo test -p worldwake-sim save_load::tests::save_to_bytes_writes_current_format_version -- --exact`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace`
8. `cargo test --workspace`

## Outcome

Completed: 2026-03-27

What actually changed:
- Added `crates/worldwake-ai/src/exhaustion.rs` with `ExhaustionInvalidationCondition` and `ExhaustionBaseline`.
- Extended `ExhaustionEntry` with `invalidation_conditions` and `baseline`, removed `Copy`, and re-exported the new types from `worldwake-ai`.
- Updated runtime/save tests and all affected exhaustion-entry literals.
- Added `Ord + PartialOrd` to `HomeostaticNeeds` so `ExhaustionBaseline` can remain a deterministic value object.
- Bumped `SAVE_FORMAT_VERSION` from `6` to `7` and added focused coverage asserting the current header version.

Deviations from original plan:
- The original ticket claimed no save-format impact and no core-type changes. Live code disproved both assumptions, so the implementation explicitly versioned the save boundary and updated `HomeostaticNeeds` instead of weakening the exhaustion types.
- No `ExhaustionEntry: Copy` call sites needed behavioral rewrites; the practical fallout was literal/test updates rather than runtime `.clone()` surgery.

Verification results:
- Focused tests passed for the new exhaustion baseline, runtime bincode round-trips, driver-state persistence, post-load validation, and save-format versioning.
- `cargo test -p worldwake-ai` passed.
- `cargo clippy --workspace` passed.
- `cargo test --workspace` passed.
