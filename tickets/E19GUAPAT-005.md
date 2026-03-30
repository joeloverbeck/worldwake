# E19GUAPAT-005: Extend public_order() with guard_presence_factor()

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — extends derived view function in worldwake-systems
**Deps**: E19GUAPAT-001 (PatrolRoute component must exist for guard detection)

## Problem

The `public_order()` derived view in `offices.rs` currently accounts for office vacancy and hostile faction pairs, but not for the presence of patrolling guards. The spec requires a `guard_presence_factor()` contribution so that guard presence at a place increases the derived public order value. This is a designer/CLI diagnostic view — agents never read it (Principle 14).

## Assumption Reassessment (2026-03-30)

1. `public_order()` exists in `crates/worldwake-systems/src/offices.rs` (line 130). Current implementation: starts at `PUBLIC_ORDER_BASELINE`, subtracts `VACANT_OFFICE_PENALTY` per vacant office and `HOSTILE_FACTION_PAIR_PENALTY` per hostile faction pair.
2. The function signature is `pub fn public_order(place: EntityId, world: &World) -> Permille`.
3. `PatrolRoute` component (from E19GUAPAT-001) identifies guards — any agent with a `PatrolRoute` is a guard.
4. The spec defines `guard_presence_factor()` as: count agents at the place who have `PatrolRoute`, multiply by `GUARD_PRESENCE_BONUS`, cap at `MAX_GUARD_ORDER_BONUS`.
5. `public_order()` is exported from `crates/worldwake-systems/src/lib.rs` (line 48).
6. This is a **derived view** (FND-01 H.3) — not authoritative stored state. Agents never query it. It's for CLI display and designer diagnostics only.
7. No adjacent contradictions found.

## Architecture Check

1. Adding a helper `guard_presence_factor()` and integrating it into the existing `public_order()` function is the minimal change. The alternative (a separate function) would fragment the public order calculation unnecessarily.
2. Using `Permille` arithmetic with named constants (`GUARD_PRESENCE_BONUS`, `MAX_GUARD_ORDER_BONUS`) follows existing conventions in `offices.rs` (`PUBLIC_ORDER_BASELINE`, `VACANT_OFFICE_PENALTY`).
3. No backwards-compatibility shims.

## Verification Layers

1. Guard presence increases public_order → focused unit test: place with 0 vs 1 vs 2 guards
2. Cap enforcement → focused unit test: place with many guards doesn't exceed baseline + MAX_GUARD_ORDER_BONUS
3. Derived view correctness → all existing public_order tests still pass (additive change)
4. Single-layer ticket (derived view extension) — no cross-layer mapping needed.

## What to Change

### 1. Add `guard_presence_factor()` in `crates/worldwake-systems/src/offices.rs`

```rust
const GUARD_PRESENCE_BONUS: u16 = /* per-guard bonus */;
const MAX_GUARD_ORDER_BONUS: u16 = /* cap */;

fn guard_presence_factor(place: EntityId, world: &World) -> Permille {
    let patrolling_guards = world.entities_at(place)
        .filter(|e| world.get_component_patrol_route(*e).is_some())
        .count();
    Permille::new((patrolling_guards as u16 * GUARD_PRESENCE_BONUS).min(MAX_GUARD_ORDER_BONUS))
}
```

### 2. Integrate into `public_order()` in same file

Add `guard_presence_factor(place, world)` as an additive term after existing penalty subtractions:

```rust
pub fn public_order(place: EntityId, world: &World) -> Permille {
    let mut order = PUBLIC_ORDER_BASELINE;
    // existing vacancy/hostility penalties...
    order = order.saturating_add(guard_presence_factor(place, world));
    order
}
```

Ensure `Permille::saturating_add` exists or use equivalent arithmetic.

## Files to Touch

- `crates/worldwake-systems/src/offices.rs` (modify — add `guard_presence_factor()`, integrate into `public_order()`)

## Out of Scope

- Patrol action handler or candidate generation (E19GUAPAT-003, E19GUAPAT-004)
- Route adaptation (E19GUAPAT-006)
- Agents reading public_order() — this is a derived view only, never queried by agents
- Thief behavior responding to guard presence (separate system — thieves react to their beliefs about guard presence, not to `public_order()` directly)
- Golden E2E tests (E19GUAPAT-007)

## Acceptance Criteria

### Tests That Must Pass

1. `public_order()` at a place with 0 guards returns same value as before (no regression)
2. `public_order()` at a place with 1 guard with PatrolRoute returns higher value than with 0 guards
3. `public_order()` at a place with many guards is capped at MAX_GUARD_ORDER_BONUS above the base
4. Agent without PatrolRoute at a place does not contribute to guard_presence_factor
5. Existing public_order tests continue to pass
6. Existing suite: `cargo test -p worldwake-systems`
7. `cargo clippy --workspace`

### Invariants

1. `public_order()` remains a derived view — never stored as authoritative state (Principle 3/27)
2. No `f32`/`f64` — all arithmetic uses `Permille` and integer math
3. Function signature unchanged — callers unaffected
4. Guard detection uses `PatrolRoute` presence, not a separate "is_guard" flag

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/offices.rs` or `crates/worldwake-systems/tests/` — focused tests for guard presence factor integration

### Commands

1. `cargo test -p worldwake-systems -- public_order`
2. `cargo test -p worldwake-systems -- offices`
3. `cargo clippy --workspace && cargo test --workspace`
