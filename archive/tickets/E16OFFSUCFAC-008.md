# E16OFFSUCFAC-008: Implement Public Order Derived Function

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new derived query function in worldwake-systems
**Deps**: E16OFFSUCFAC-002, E16OFFSUCFAC-007

## Problem

E16 requires a `public_order(place, world) -> Permille` function that computes public order at a given place as a derived value. This is NEVER stored (Principle 3) — it is recomputed each time it is queried. E16 contributes two factors: office vacancy (destabilizes) and hostile faction pairs (destabilize). Future epics (E17 crime, E19 guards) will extend this function with additional factors.

## Assumption Reassessment (2026-03-15)

1. `Permille` exists in `crates/worldwake-core/src/numerics.rs` with `new_unchecked`, `value`, `saturating_add`, and `saturating_sub` — confirmed. It does **not** provide tuple-struct construction outside the module, `clamp`, or `saturating_mul`.
2. `offices_with_jurisdiction()` and `office_is_vacant()` helpers from E16OFFSUCFAC-007 will be available — dependency.
3. `hostile_to` relation exists and is directional — confirmed. The derived public-order helper must define clearly whether place-level faction conflict is triggered by one-way hostility or only by mutual hostility.
4. `member_of`/`members_of` relations exist — confirmed, used to determine faction presence at a place.
5. `World` does **not** expose `entities_at(place)`. The authoritative placement API is `entities_effectively_at(place)`, which already includes physically present contained entities and filters dead entities.
6. No `public_order` function exists yet — confirmed.

## Architecture Check

1. `public_order()` is a pure derived function — correct per Principle 3 (no stored derived state).
2. Placed in `worldwake-systems::offices` for now because the current factors are office/faction political state already centered there. Do **not** add storage or a compatibility wrapper.
3. The implementation should use module-local named `Permille` constants and small helper functions so future E17/E19 extensions compose cleanly without scattering hardcoded arithmetic across callers.
4. Place-level faction conflict should count each unordered present-faction pair at most once. A single hostility edge in either direction is sufficient to indicate active political disorder at that place; requiring mutual hostility would couple derived order to symmetric relation upkeep that the model does not guarantee.
4. No backward-compatibility shims.

## What to Change

### 1. Add `public_order()` function

In `crates/worldwake-systems/src/offices.rs` (alongside succession system):

```rust
const PUBLIC_ORDER_BASELINE: Permille = Permille::new_unchecked(750);
const VACANT_OFFICE_PENALTY: Permille = Permille::new_unchecked(200);
const HOSTILE_FACTION_PAIR_PENALTY: Permille = Permille::new_unchecked(100);

/// Computes public order at a place as a derived Permille value.
/// NEVER stored — recomputed on each query (Principle 3).
///
/// E16 factors: office vacancy, hostile faction pairs.
/// Extension points: E17 crime factor, E19 guard factor.
pub fn public_order(place: EntityId, world: &World) -> Permille {
    let mut order = PUBLIC_ORDER_BASELINE;

    // E16 factor: office vacancy
    for office in offices_with_jurisdiction(place, world) {
        if office_is_vacant(office, world) {
            order = order.saturating_sub(VACANT_OFFICE_PENALTY);
        }
    }

    // E16 factor: faction conflict
    for _ in 0..count_present_hostile_faction_pairs_at(place, world) {
        order = order.saturating_sub(HOSTILE_FACTION_PAIR_PENALTY);
    }

    order
}
```

### 2. Add `count_present_hostile_faction_pairs_at()` helper

Count unordered pairs of factions present at a place where at least one hostility edge exists between the two factions. "Present" means at least one living member of the faction is physically present according to `entities_effectively_at(place)`.

### 3. Export from lib.rs

Add `public_order` to the public API of `worldwake-systems`.

## Files to Touch

- `crates/worldwake-systems/src/offices.rs` (modify — add `public_order()` and `count_hostile_faction_pairs_at()`)
- `crates/worldwake-systems/src/lib.rs` (modify — add re-export if needed)

## Out of Scope

- E17 crime factor — documented as extension point comment only
- E19 guard factor — documented as extension point comment only
- Storing public order as a component or relation (MUST NOT do this — Principle 3)
- AI decision-making based on public order (future work)
- Any changes to the succession system (E16OFFSUCFAC-007)
- Introducing a new politics/public-order module split. If the political query surface grows substantially in later tickets, that refactor should happen as a separate architectural change.

## Acceptance Criteria

### Tests That Must Pass

1. `public_order()` returns `Permille(750)` for a place with no offices and no hostile factions (baseline).
2. `public_order()` returns `Permille(550)` for a place with one vacant office (750 - 200).
3. `public_order()` returns `Permille(350)` for a place with two vacant offices (750 - 200 - 200).
4. `public_order()` returns `Permille(650)` for one unordered hostile faction pair present at the place, even if hostility is only declared in one direction.
5. Symmetric hostility for the same two factions is counted once, not twice.
6. Multiple members from the same faction at the same place do not multiply the hostility penalty.
7. `public_order()` returns `Permille(450)` for one vacant office and one hostile faction pair (750 - 200 - 100).
8. `public_order()` saturates at `Permille(0)` — never goes negative.
9. `public_order()` is a pure function — same inputs always produce same output.
10. `public_order()` uses `Permille`, not `f32`/`f64`.
11. `cargo clippy --workspace --all-targets -- -D warnings`
12. `cargo test --workspace`

### Invariants

1. `public_order()` is NEVER stored as a component or relation (Principle 3).
2. Uses `Permille` — no floating point.
3. Deterministic: same world state always produces same result.
4. Does NOT depend on E17 crime rate or E19 guard presence — those are documented extension points only.
5. Filled offices do not reduce order (only vacant ones do).
6. Present faction conflict is counted by unique faction pairs, not by agent count.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/offices.rs` — unit tests for `public_order()` covering baseline, vacancy penalties, one-way hostility, deduplicated symmetric hostility, duplicate faction-member presence, combined penalties, and saturation at zero.

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-15
- What actually changed:
  - Corrected the ticket assumptions before implementation to match the current codebase: `Permille` API shape, placement API (`entities_effectively_at`), and directional hostility semantics.
  - Implemented `public_order(place, world) -> Permille` in `crates/worldwake-systems/src/offices.rs`.
  - Implemented `count_present_hostile_faction_pairs_at(place, world) -> usize` using unique present-faction pairs and one-way-or-two-way hostility detection without double-counting.
  - Exported the new derived query helpers from `crates/worldwake-systems/src/lib.rs`.
  - Added unit coverage for baseline order, vacancy penalties, one-way hostility, symmetric-hostility deduplication, duplicate-member deduplication, combined penalties, and saturation at zero.
- Deviations from original plan:
  - The original sketch used non-existent `Permille` APIs (`Permille(…)`, `clamp`, `saturating_mul`) and a non-existent `World::entities_at`; implementation uses named `Permille::new_unchecked` constants, repeated `saturating_sub`, and `entities_effectively_at`.
  - Hostility is treated as an unordered present-faction pair when either direction is hostile. This is more robust than requiring mutual hostility because the authoritative relation is directional.
  - No separate politics/public-order module split was introduced; the query remains in `offices.rs` because the current political query surface is still small.
- Verification results:
  - `cargo test -p worldwake-systems offices -- --nocapture` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
  - `cargo test --workspace` ✅
