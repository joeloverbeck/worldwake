# E12COMHEA-004: ActionPayload Combat + Loot variants

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-sim action_payload module
**Deps**: E07 (ActionPayload enum), E12COMHEA-001 (CombatWeaponRef)

## Problem

The `ActionPayload` enum needs `Combat(CombatActionPayload)` and `Loot(LootActionPayload)` variants so that combat and looting actions can carry their target/weapon data.

## Assumption Reassessment (2026-03-11)

1. `ActionPayload` exists in `crates/worldwake-sim/src/action_payload.rs` with `None`, `Harvest`, `Craft`, `Trade` — confirmed.
2. Each variant has a typed accessor (`as_harvest()`, `as_craft()`, `as_trade()`) — confirmed, pattern must be followed.
3. `ActionPayload` derives `Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize` — confirmed.
4. `CombatWeaponRef` already exists today and is publicly re-exported from `worldwake-core` after E12COMHEA-001; this ticket must use the current type, not treat it as future work.
5. `EntityId` is already publicly re-exported from `worldwake-core` and is the right payload-level identity carrier here.
6. The original ticket understated test impact. `crates/worldwake-sim/src/action_payload.rs` already contains trait/accessor/bincode unit tests that must be extended, and `crates/worldwake-sim/src/lib.rs` re-exports payload types publicly, so this ticket also needs to keep the crate surface coherent.
7. There are currently no production consumers for `Combat` or `Loot` payloads yet; those arrive in E12COMHEA-010 and E12COMHEA-012. The immediate compile/test impact is therefore localized to the payload module and its public exports.
8. `E12COMHEA-013` still carries an unresolved `HealActionPayload` question. This ticket should not pretend Combat and Loot are the only payload shapes this epic may ever need; it should only add the two payloads already mandated by the spec and required by downstream tickets.

## Architecture Check

### What Is Beneficial To Add Now

1. Adding explicit `CombatActionPayload` and `LootActionPayload` variants is better than overloading existing payloads or passing raw IDs through untyped side channels. The action framework stays explicit, serializable, and exhaustively matchable.
2. Keeping domain-specific payload structs is the cleaner architecture here. `Combat` needs both `target` and `weapon`; `Loot` only needs `target`. Introducing a generic catch-all payload or aliasing `Loot` onto a future `Heal` payload would weaken type meaning for little gain.
3. Updating the typed accessor pattern is still the right design because action handlers can reject wrong payload shapes with clear, local contracts instead of downcasting or ad hoc field inspection.

### What This Ticket Should Not Pretend To Solve

1. This ticket should not pre-empt the unresolved `HealActionPayload` decision from E12COMHEA-013. If healing also becomes a target-only payload later, that should be decided on its own merits rather than folded into this ticket through premature abstraction.
2. The original scope overstates blast radius. Today, the only exhaustive `ActionPayload` matches are the accessor methods in `action_payload.rs`; there is not yet a wider set of production `match` sites to update.

## Revised Scope

Implement the two spec-mandated payload shapes cleanly and keep the public sim API/tests aligned:

1. Add `CombatActionPayload`.
2. Add `LootActionPayload`.
3. Extend `ActionPayload` with `Combat` and `Loot`.
4. Add `as_combat()` and `as_loot()`.
5. Update `worldwake-sim` public re-exports and the existing `action_payload.rs` unit tests.

## What to Change

### 1. Add `CombatActionPayload` struct

```rust
pub struct CombatActionPayload {
    pub target: EntityId,
    pub weapon: CombatWeaponRef,
}
```

### 2. Add `LootActionPayload` struct

```rust
pub struct LootActionPayload {
    pub target: EntityId,
}
```

### 3. Add variants to `ActionPayload`

```rust
Combat(CombatActionPayload),
Loot(LootActionPayload),
```

### 4. Add typed accessors

```rust
pub const fn as_combat(&self) -> Option<&CombatActionPayload> { ... }
pub const fn as_loot(&self) -> Option<&LootActionPayload> { ... }
```

### 5. Update all exhaustive matches on ActionPayload

All existing `as_*` methods in `crates/worldwake-sim/src/action_payload.rs` must include the new variants in their non-matching arms.

## Files to Touch

- `crates/worldwake-sim/src/action_payload.rs` (modify)
- `crates/worldwake-sim/src/lib.rs` (modify — re-export the new payload types)

## Out of Scope

- `HealActionPayload` or a `Heal` enum variant — E12COMHEA-013 still owns that design decision
- CombatWeaponRef definition (E12COMHEA-001)
- Action definitions or handlers (E12COMHEA-010/011/012/013)
- Hit resolution logic

## Acceptance Criteria

### Tests That Must Pass

1. `CombatActionPayload` satisfies `Clone + Debug + Eq + Ord + Serialize + Deserialize`
2. `LootActionPayload` satisfies `Clone + Debug + Eq + Ord + Serialize + Deserialize`
3. `ActionPayload::Combat(...)` round-trips through bincode
4. `ActionPayload::Loot(...)` round-trips through bincode
5. `as_combat()` returns `Some` for Combat variant, `None` for all others
6. `as_loot()` returns `Some` for Loot variant, `None` for all others
7. Existing accessor tests still pass (updated for new variants in non-matching arms)
8. `ActionPayload::default()` remains `ActionPayload::None`
9. `worldwake-sim` continues to publicly export the new payload structs
10. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. All payload types derive the same trait set as existing payloads
2. No `f32`/`f64` in payload structs
3. All `ActionPayload` matches remain exhaustive

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_payload.rs` — trait bounds, bincode roundtrip, accessor coverage for new variants and updated non-matching arms

### Commands

1. `cargo test -p worldwake-sim -- action_payload`
2. `cargo test --workspace && cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - corrected the ticket assumptions and scope before implementation so it matched the real `ActionPayload` module, its existing tests, and the current `CombatWeaponRef` state
  - added `CombatActionPayload` and `LootActionPayload`, plus `ActionPayload::Combat` and `ActionPayload::Loot`
  - added `as_combat()` and `as_loot()`, and updated the existing `as_harvest()`, `as_craft()`, and `as_trade()` non-matching arms to stay exhaustive
  - updated `worldwake-sim` public re-exports so downstream combat/loot tickets can import the new payload types directly
  - strengthened `action_payload.rs` tests to cover trait bounds, bincode round-trips, and accessor behavior for the new variants
- Deviations from original plan:
  - narrowed the claimed blast radius: there were no broader production `ActionPayload` match sites to update yet beyond the payload module itself and the crate re-exports
  - kept the unresolved `HealActionPayload` question explicitly out of scope for E12COMHEA-013 instead of introducing a premature shared target-payload abstraction
- Verification results:
  - `cargo test -p worldwake-sim -- action_payload` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
