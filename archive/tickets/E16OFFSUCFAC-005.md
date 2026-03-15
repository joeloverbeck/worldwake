# E16OFFSUCFAC-005: Add Bribe, Threaten, DeclareSupport Action Payloads

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new ActionPayload variants in worldwake-sim
**Deps**: E16OFFSUCFAC-001

## Problem

E16 introduces three new social actions (Bribe, Threaten, DeclareSupport), each requiring a payload struct and an `ActionPayload` enum variant. These must exist in `worldwake-sim` before the action handlers in `worldwake-systems` can be implemented. This ticket establishes the payload types and enum wiring only — no handler logic.

## Assumption Reassessment (2026-03-15)

1. `ActionPayload` in `crates/worldwake-sim/src/action_payload.rs` currently has 9 variants (None, Tell, Transport, Harvest, Craft, Trade, Combat, Loot, QueueForFacilityUse) — confirmed.
2. Each payload variant has a typed accessor method (e.g., `as_tell()`, `as_trade()`) — confirmed, new payloads need accessors.
3. `ActionPayload` derives `Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize` — confirmed. New payload structs should match the existing payload struct trait set (`Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize`).
4. `CommodityKind` and `Quantity` are available from `worldwake-core` — confirmed, needed for `BribeActionPayload`.
5. `EntityId` is available — confirmed, needed for all three payloads.
6. Existing payload coverage is centralized in `crates/worldwake-sim/src/action_payload.rs` unit tests. The file already has trait, accessor, and per-variant bincode roundtrip tests for the current payload set.
7. `crates/worldwake-sim/src/save_load.rs` currently verifies whole-`SimulationState` roundtrips, but its fixture does not exercise arbitrary `ActionPayload` variants. This ticket should not claim direct save/load coverage for the new payloads unless it also expands that fixture.

## Architecture Check

1. Following the exact pattern of existing payloads (e.g., `TellActionPayload`, `TradeActionPayload`).
2. Payload structs are pure data — no behavior, no handler logic.
3. No backward-compatibility shims needed — purely additive.
4. Extending the central `ActionPayload` enum is still the cleanest fit for the current sim architecture. It keeps payload typing explicit at the action-system boundary and avoids aliasing these new social actions onto unrelated payload shapes.
5. Long-term cleanup opportunity: the accessor match arms are becoming repetitive as `ActionPayload` grows. If more payload families are added after E16/E17, consider a macro or helper pattern that generates the enum accessors and corresponding tests from a single list. That is not justified inside this ticket because it would broaden scope without changing semantics.

## What to Change

### 1. Add payload structs

In `crates/worldwake-sim/src/action_payload.rs`:

```rust
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BribeActionPayload {
    pub target: EntityId,
    pub offered_commodity: CommodityKind,
    pub offered_quantity: Quantity,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ThreatenActionPayload {
    pub target: EntityId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct DeclareSupportActionPayload {
    pub office: EntityId,
    pub candidate: EntityId,
}
```

### 2. Add `ActionPayload` variants

```rust
pub enum ActionPayload {
    // ... existing variants ...
    Bribe(BribeActionPayload),
    Threaten(ThreatenActionPayload),
    DeclareSupport(DeclareSupportActionPayload),
}
```

### 3. Add typed accessors

```rust
pub fn as_bribe(&self) -> Option<&BribeActionPayload> { ... }
pub fn as_threaten(&self) -> Option<&ThreatenActionPayload> { ... }
pub fn as_declare_support(&self) -> Option<&DeclareSupportActionPayload> { ... }
```

### 4. Add re-exports

In `crates/worldwake-sim/src/lib.rs`, re-export the new payload types.

## Files to Touch

- `crates/worldwake-sim/src/action_payload.rs` (modify — add 3 structs, 3 variants, 3 accessors)
- `crates/worldwake-sim/src/lib.rs` (modify — add re-exports)

## Out of Scope

- Action handler registration (E16OFFSUCFAC-006)
- Action definition registration in the action registry (E16OFFSUCFAC-006)
- Start-gate validation, commit semantics, tick behavior (E16OFFSUCFAC-006)
- AI planner integration (E16OFFSUCFAC-009)
- Broad `SimulationState` save/load fixture expansion solely to exercise these variants

## Acceptance Criteria

### Tests That Must Pass

1. `BribeActionPayload` constructs with `target`, `offered_commodity`, `offered_quantity` and roundtrips through bincode.
2. `ThreatenActionPayload` constructs with `target` and roundtrips through bincode.
3. `DeclareSupportActionPayload` constructs with `office`, `candidate` and roundtrips through bincode.
4. `ActionPayload::Bribe(...)`, `ActionPayload::Threaten(...)`, and `ActionPayload::DeclareSupport(...)` wrap and unwrap through typed accessors correctly.
5. The new accessors return `None` for non-matching variants, integrated into the existing centralized typed-accessor test.
6. `crates/worldwake-sim/src/lib.rs` re-exports the new payload structs.
7. `cargo test -p worldwake-sim`
8. `cargo clippy --workspace --all-targets -- -D warnings`
9. `cargo test --workspace`

### Invariants

1. `ActionPayload` remains `Default` with `None`.
2. No existing payload variants change.
3. All new types derive the same trait set as existing payloads.
4. The payload API remains centralized in `action_payload.rs`; new coverage should extend the existing trait/accessor/bincode unit tests rather than introducing a parallel test style.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_payload.rs` — extend the existing trait test to include the three new structs.
2. `crates/worldwake-sim/src/action_payload.rs` — extend the existing typed accessor matrix to cover Bribe, Threaten, and DeclareSupport plus negative cases.
3. `crates/worldwake-sim/src/action_payload.rs` — add one bincode roundtrip test per new payload variant, following the existing per-variant pattern.

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-15
- Actual changes:
  - Added `BribeActionPayload`, `ThreatenActionPayload`, and `DeclareSupportActionPayload` to `crates/worldwake-sim/src/action_payload.rs`
  - Added matching `ActionPayload` enum variants and typed accessors
  - Re-exported the new payload structs from `crates/worldwake-sim/src/lib.rs`
  - Extended the centralized payload unit tests for traits, typed accessors, and bincode roundtrips
- Deviations from original plan:
  - Corrected the ticket before implementation to match the codebase’s real testing architecture
  - Removed the unsupported claim that this ticket should directly prove whole-`SimulationState` save/load coverage for the new variants
  - Refactored the accessor coverage into multiple smaller tests to satisfy workspace clippy instead of suppressing the lint
- Verification results:
  - `cargo test -p worldwake-sim action_payload -- --nocapture` ✅
  - `cargo test -p worldwake-sim` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
  - `cargo test --workspace -q` ✅
