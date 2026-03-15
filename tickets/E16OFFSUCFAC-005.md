# E16OFFSUCFAC-005: Add Bribe, Threaten, DeclareSupport Action Payloads

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new ActionPayload variants in worldwake-sim
**Deps**: E16OFFSUCFAC-001

## Problem

E16 introduces three new social actions (Bribe, Threaten, DeclareSupport), each requiring a payload struct and an `ActionPayload` enum variant. These must exist in `worldwake-sim` before the action handlers in `worldwake-systems` can be implemented. This ticket establishes the payload types and enum wiring only — no handler logic.

## Assumption Reassessment (2026-03-15)

1. `ActionPayload` in `crates/worldwake-sim/src/action_payload.rs` currently has 9 variants (None, Tell, Transport, Harvest, Craft, Trade, Combat, Loot, QueueForFacilityUse) — confirmed.
2. Each payload variant has a typed accessor method (e.g., `as_tell()`, `as_trade()`) — confirmed, new payloads need accessors.
3. `ActionPayload` derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize` — confirmed.
4. `CommodityKind` and `Quantity` are available from `worldwake-core` — confirmed, needed for `BribeActionPayload`.
5. `EntityId` is available — confirmed, needed for all three payloads.

## Architecture Check

1. Following the exact pattern of existing payloads (e.g., `TellActionPayload`, `TradeActionPayload`).
2. Payload structs are pure data — no behavior, no handler logic.
3. No backward-compatibility shims needed — purely additive.

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

## Acceptance Criteria

### Tests That Must Pass

1. `BribeActionPayload` constructs with `target`, `offered_commodity`, `offered_quantity` and roundtrips through bincode.
2. `ThreatenActionPayload` constructs with `target` and roundtrips through bincode.
3. `DeclareSupportActionPayload` constructs with `office`, `candidate` and roundtrips through bincode.
4. `ActionPayload::Bribe(...)` wraps and `as_bribe()` unwraps correctly.
5. `ActionPayload::Threaten(...)` wraps and `as_threaten()` unwraps correctly.
6. `ActionPayload::DeclareSupport(...)` wraps and `as_declare_support()` unwraps correctly.
7. `as_bribe()` returns `None` for non-Bribe payloads (and similarly for other accessors).
8. `cargo clippy --workspace --all-targets -- -D warnings`
9. `cargo test --workspace`

### Invariants

1. `ActionPayload` remains `Default` with `None`.
2. No existing payload variants change.
3. All new types derive the same trait set as existing payloads.
4. Save/load roundtrip preserves new payloads.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_payload.rs` — add serde roundtrip and accessor tests for all three payloads, following the existing Tell payload test pattern.

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
