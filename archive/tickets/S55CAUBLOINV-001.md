# S55CAUBLOINV-001: Core types and BlockedIntent extension

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` blocked_intent types, save format version bump
**Deps**: S55 spec (reassessed 2026-04-06)

## Problem

`BlockedIntent` carries no data about what evidence would clear it. Clearing logic is implicit in a code-driven `blocker_resolved` match in `failure_handling.rs`. This ticket adds the type infrastructure so clearing conditions become explicit, inspectable data on each blocker — a prerequisite for data-driven evaluation (ticket 003) and condition-aware construction (ticket 002).

## Assumption Reassessment (2026-04-06)

1. `BlockedIntent` is defined at `crates/worldwake-core/src/blocked_intent.rs:124` with 5 fields: `blocker_key`, `blocking_fact`, `diagnostic_context`, `observed_tick`, `expires_tick`. Derives `Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize`. New types must satisfy all these bounds.
2. `BlockedIntentMemory` implements `Component` at `blocked_intent.rs:121`. It stores `BTreeMap<BlockerKey, BlockedIntent>`. The `intents` field is `pub` — existing code accesses it directly (e.g., `failure_handling.rs:88-90` calls `.intents.retain()`).
3. `BlockingFact` has 17 variants at `blocked_intent.rs:142-164`, including parameterized `MissingTool(UniqueItemKind)` and `MissingInput(CommodityKind)`.
4. `CommodityKind` at `crates/worldwake-core/src/items.rs:10` — `Copy` enum with 10 variants.
5. `UniqueItemKind` — `Copy` enum in `worldwake-core`.
6. `Quantity` at `crates/worldwake-core/src/numerics.rs:73` — `Copy` newtype over `u32`.
7. `Permille` at `crates/worldwake-core/src/numerics.rs:25` — `Copy` newtype over `u16`.
8. `SAVE_FORMAT_VERSION` is currently `27` at `crates/worldwake-sim/src/save_load.rs:6`. Adding fields to `BlockedIntent` changes the serialized component format.
9. `BlockedIntent` construction sites are broader than the initial draft listed. Live direct literals also exist in `worldwake-ai/src/agent_tick/{candidates,frame,observation}.rs`, `worldwake-ai/src/feasibility.rs`, `worldwake-ai/src/candidate_generation.rs`, `worldwake-ai/src/search/tests.rs`, `worldwake-ai/src/agent_tick/tests.rs`, `worldwake-ai/tests/golden_care.rs`, and `worldwake-systems/src/trade_actions.rs`. All direct literals must gain the new fields.
10. Single-layer ticket (core types only). No cross-system boundary under audit.

## Architecture Check

1. Storing clearing conditions as explicit data on `BlockedIntent` is cleaner than the current implicit `blocker_resolved` match because: conditions become inspectable for debugging (P29), the mapping from `BlockingFact` to clearing condition is declared at construction time rather than duplicated at evaluation time, and baselines enable detecting *any* change rather than only threshold conditions.
2. No backward-compatibility shims. Old `blocker_resolved` continues to work unchanged alongside the new fields until ticket 003 replaces it.
3. `sweep_cleared` encapsulates the `intents.retain()` pattern in core rather than having AI code reach into `.intents` directly — prepares for ticket 003 without changing behavior now.

## Verification Layers

1. `BlockedIntent` satisfies `Copy + Clone + Eq + PartialEq + Serialize + Deserialize` → focused unit test (`blocked_intent_types_satisfy_required_bounds`)
2. `BlockerClearingCondition` and `ClearingBaseline` satisfy same bounds → new focused unit test
3. Bincode round-trip for `BlockedIntentMemory` with new fields → existing `blocked_intent_memory_roundtrips_through_bincode` test (extended)
4. `sweep_cleared` removes matching entries and retains non-matching → new focused unit test
5. All existing blocker tests pass with TtlOnly/None defaults → `cargo test -p worldwake-core` + `cargo test -p worldwake-ai`
6. Single-layer ticket — additional layer mapping not applicable

## What to Change

### 1. New enums in `crates/worldwake-core/src/blocked_intent.rs`

Add before `BlockedIntent` struct:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BlockerClearingCondition {
    CommodityAvailabilityChanged { commodity: CommodityKind, place: EntityId },
    InventoryChanged { commodity: CommodityKind },
    UniqueItemAcquired { kind: UniqueItemKind },
    PathDiscovered { destination: EntityId },
    EntityReappeared { entity: EntityId },
    DangerReduced { place: EntityId },
    ContentionChanged { facility: EntityId },
    TtlOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ClearingBaseline {
    CommodityQuantity { quantity: Quantity },
    InventoryQuantity { quantity: Quantity },
    UniqueItemCount(u32),
    PathKnown(bool),
    EntityBelieved(bool),
    DangerLevel(Permille),
    ContentionPosition(Option<u32>),
}
```

### 2. Extend `BlockedIntent` struct

Add two fields:

```rust
pub struct BlockedIntent {
    pub blocker_key: BlockerKey,
    pub blocking_fact: BlockingFact,
    pub diagnostic_context: Option<BlockerDiagnostic>,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
    pub clearing_condition: BlockerClearingCondition,      // NEW
    pub baseline_snapshot: Option<ClearingBaseline>,        // NEW
}
```

### 3. Add `sweep_cleared` method on `BlockedIntentMemory`

```rust
pub fn sweep_cleared(&mut self, mut is_cleared: impl FnMut(&BlockedIntent) -> bool) {
    self.intents.retain(|_, intent| !is_cleared(intent));
}
```

### 4. Update all `BlockedIntent` construction sites

Every `BlockedIntent { ... }` literal gains:
```rust
clearing_condition: BlockerClearingCondition::TtlOnly,
baseline_snapshot: None,
```

Sites: `failure_handling.rs:71`, `test_utils.rs` (`sample_blocked_intent`), all `make_intent` / direct construction in `blocked_intent.rs` tests, and every direct `BlockedIntent` literal in `worldwake-ai` production and test code.

### 5. Re-export new types from `crates/worldwake-core/src/lib.rs`

Add `BlockerClearingCondition` and `ClearingBaseline` to the pub use from `blocked_intent`.

### 6. Bump `SAVE_FORMAT_VERSION`

In `crates/worldwake-sim/src/save_load.rs:6`: increment from `27` to `28`.

## Files to Touch

- `crates/worldwake-core/src/blocked_intent.rs` (modify — new enums, struct extension, sweep_cleared method, test updates)
- `crates/worldwake-core/src/lib.rs` (modify — re-export new types)
- `crates/worldwake-core/src/test_utils.rs` (modify — update sample_blocked_intent)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — add new fields to construction site, use `sweep_cleared`, and update test construction sites)
- `crates/worldwake-ai/src/agent_tick/candidates.rs` (modify — update direct construction site)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — update direct construction sites)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — update direct construction site)
- `crates/worldwake-ai/src/feasibility.rs` (modify — update test helper construction site)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — update direct test construction sites)
- `crates/worldwake-ai/src/search/tests.rs` (modify — update direct construction sites)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — update direct construction sites)
- `crates/worldwake-ai/tests/golden_care.rs` (modify — update direct construction sites)
- `crates/worldwake-systems/src/trade_actions.rs` (modify — update direct construction site for `NoBuyer` blocker recording)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump SAVE_FORMAT_VERSION)

## Out of Scope

- Populating real clearing conditions (ticket 002)
- Replacing `blocker_resolved` with condition-based evaluation (ticket 003)
- New AI-side clearing predicate logic
- Golden test changes

## Acceptance Criteria

### Tests That Must Pass

1. New: `blocker_clearing_condition_and_baseline_satisfy_required_bounds` — `Copy + Clone + Eq + Debug + Serialize + Deserialize`
2. New: `sweep_cleared_removes_matching_entries` — predicate returns true → entry removed
3. New: `sweep_cleared_retains_non_matching_entries` — predicate returns false → entry kept
4. Extended: `blocked_intent_memory_roundtrips_through_bincode` — round-trip with non-TtlOnly clearing condition
5. Existing suite: `cargo test -p worldwake-core`
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `BlockedIntent` remains `Copy` — all new field types must be `Copy`
2. `BlockedIntentMemory` bincode serialization round-trips with new fields
3. All existing blocker behavior unchanged — TtlOnly + None defaults produce identical clearing semantics
4. `SAVE_FORMAT_VERSION` bumped exactly once

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/blocked_intent.rs` (tests module) — new bound-check test for `BlockerClearingCondition` and `ClearingBaseline`; new `sweep_cleared` tests; extended bincode round-trip test
2. `worldwake-ai` direct-literal sites — update all production/test `BlockedIntent` literals with the new default fields; no behavior changes in this ticket

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo build --workspace`

## Outcome

Completion date: 2026-04-06

- Added `BlockerClearingCondition` and `ClearingBaseline` in `worldwake-core`, extended `BlockedIntent`, and added `BlockedIntentMemory::sweep_cleared`.
- Updated all live `BlockedIntent` construction sites to provide `TtlOnly` / `None` defaults, including the extra reassessment fallout in `worldwake-systems/src/trade_actions.rs`.
- Re-exported the new core types and bumped `SAVE_FORMAT_VERSION` from `27` to `28`.
- Deviations from original plan: reassessment widened the constructor fallout surface beyond the initial ticket draft, so additional mechanical default-field updates were applied in `worldwake-ai` production/test sites and `worldwake-systems/src/trade_actions.rs`.

## Verification Result

- Passed `cargo test -p worldwake-core blocked_intent -- --nocapture`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo build --workspace`
