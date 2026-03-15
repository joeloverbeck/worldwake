# E16OFFSUCFAC-008: Implement Public Order Derived Function

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new derived query function in worldwake-systems
**Deps**: E16OFFSUCFAC-002, E16OFFSUCFAC-007

## Problem

E16 requires a `public_order(place, world) -> Permille` function that computes public order at a given place as a derived value. This is NEVER stored (Principle 3) — it is recomputed each time it is queried. E16 contributes two factors: office vacancy (destabilizes) and hostile faction pairs (destabilize). Future epics (E17 crime, E19 guards) will extend this function with additional factors.

## Assumption Reassessment (2026-03-15)

1. `Permille` exists in `crates/worldwake-core/src/numerics.rs` with `saturating_sub` and `clamp` — confirmed.
2. `offices_with_jurisdiction()` and `office_is_vacant()` helpers from E16OFFSUCFAC-007 will be available — dependency.
3. `hostile_to` relation exists — confirmed, used to count hostile faction pairs at a place.
4. `member_of`/`members_of` relations exist — confirmed, used to determine faction presence at a place.
5. `entities_at(place)` exists — confirmed, used to find agents/factions at a place.
6. No `public_order` function exists yet — confirmed.

## Architecture Check

1. `public_order()` is a pure derived function — correct per Principle 3 (no stored derived state).
2. Placed in `worldwake-systems` because it reads from `World` state and depends on domain knowledge (offices, factions).
3. Extensible design: future E17/E19 factors add terms to the same function. Extension points are documented as comments but NOT implemented.
4. No backward-compatibility shims.

## What to Change

### 1. Add `public_order()` function

In `crates/worldwake-systems/src/offices.rs` (alongside succession system):

```rust
/// Computes public order at a place as a derived Permille value.
/// NEVER stored — recomputed on each query (Principle 3).
///
/// E16 factors: office vacancy, hostile faction pairs.
/// Extension points: E17 crime factor, E19 guard factor.
pub fn public_order(place: EntityId, world: &World) -> Permille {
    let mut order = Permille(750); // Baseline: moderate order

    // E16 factor: office vacancy
    for office in offices_with_jurisdiction(place, world) {
        if office_is_vacant(office, world) {
            order = order.saturating_sub(Permille(200));
        }
    }

    // E16 factor: faction conflict
    let hostile_faction_pairs = count_hostile_faction_pairs_at(place, world);
    order = order.saturating_sub(Permille(100).saturating_mul(hostile_faction_pairs));

    order.clamp(Permille(0), Permille(1000))
}
```

### 2. Add `count_hostile_faction_pairs_at()` helper

Count pairs of factions present at a place that have mutual `hostile_to` relations. "Present" means at least one member of the faction is located at the place.

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

## Acceptance Criteria

### Tests That Must Pass

1. `public_order()` returns `Permille(750)` for a place with no offices and no hostile factions (baseline).
2. `public_order()` returns `Permille(550)` for a place with one vacant office (750 - 200).
3. `public_order()` returns `Permille(350)` for a place with two vacant offices (750 - 200 - 200).
4. `public_order()` returns `Permille(650)` for a place with one hostile faction pair (750 - 100).
5. `public_order()` returns `Permille(450)` for a place with one vacant office and one hostile faction pair (750 - 200 - 100).
6. `public_order()` clamps to `Permille(0)` — never negative.
7. `public_order()` clamps to `Permille(1000)` — never exceeds maximum.
8. `public_order()` is a pure function — same inputs always produce same output.
9. `public_order()` uses `Permille`, not `f32`/`f64`.
10. `cargo clippy --workspace --all-targets -- -D warnings`
11. `cargo test --workspace`

### Invariants

1. `public_order()` is NEVER stored as a component or relation (Principle 3).
2. Uses `Permille` — no floating point.
3. Deterministic: same world state always produces same result.
4. Does NOT depend on E17 crime rate or E19 guard presence — those are documented extension points only.
5. Filled offices do not reduce order (only vacant ones do).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/offices.rs` — unit tests for `public_order()` covering baseline, vacancy, hostile factions, combination, and clamping scenarios.

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
