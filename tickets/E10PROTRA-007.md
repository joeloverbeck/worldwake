# E10PROTRA-007: ResourceSource regeneration system in worldwake-systems

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new system function in systems crate
**Deps**: E10PROTRA-002 (ResourceSource component must exist)

## Problem

Resource sources (orchards, farms, forests) need to regenerate over time according to their `regeneration_ticks_per_unit` parameter. Without a tick-driven regeneration system, harvested sources never replenish, making production unsustainable.

## Assumption Reassessment (2026-03-10)

1. `ResourceSource` will exist as an authoritative component after E10PROTRA-002 — confirmed dependency.
2. The systems crate already has a pattern for tick systems: `needs_system` in `needs.rs` runs each tick.
3. `SystemManifest` in `worldwake-sim` registers which systems run each tick.
4. `SystemDispatch` routes tick execution to registered systems.
5. Regeneration is simple: if `regeneration_ticks_per_unit` is `Some(n)`, increment `available_quantity` by 1 every `n` ticks, capped at `max_quantity`.

## Architecture Check

1. Regeneration is a world-state mutation driven by the tick loop — it belongs in `worldwake-systems`.
2. The system reads `ResourceSource` components, checks regeneration eligibility, and writes updated `available_quantity` through `WorldTxn`.
3. Regeneration must emit events (for causal tracing and witness tracking).
4. The system must respect the conservation invariant: regeneration creates new quantity only up to `max_quantity`. This is a legitimate "creation" action (natural growth), not a violation — but it must be explicit and event-logged.

## What to Change

### 1. New module `crates/worldwake-systems/src/production.rs`

```rust
/// Regeneration system: replenishes ResourceSource entities each tick.
pub fn resource_regeneration_system(world: &mut WorldTxn, tick: Tick) {
    // For each entity with ResourceSource:
    //   If regeneration_ticks_per_unit is Some(n):
    //     If (tick - last_regen_tick) >= n AND available < max:
    //       Increment available_quantity by 1
    //       Emit regeneration event
}
```

The system needs a way to track "ticks since last regeneration." Options:
- Add a `last_regeneration_tick: Option<Tick>` field to `ResourceSource` (simplest, authoritative).
- Derive from event log (expensive, unnecessary).

Recommend adding `last_regeneration_tick` to `ResourceSource` in this ticket if not already present, or coordinating with E10PROTRA-002 to include it.

### 2. Register in system dispatch

Add `resource_regeneration_system` to the `SystemManifest` / `SystemDispatch`.

### 3. Export from `lib.rs`

## Files to Touch

- `crates/worldwake-systems/src/production.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify — add module + exports + dispatch registration)
- `crates/worldwake-core/src/production.rs` (modify — add `last_regeneration_tick: Option<Tick>` to `ResourceSource` if not already present)
- `crates/worldwake-core/src/event_tag.rs` (modify — add regeneration event tag if needed)

## Out of Scope

- Harvest action logic (E10PROTRA-008)
- Craft action logic (E10PROTRA-009)
- Depletion mechanics (handled by harvest action)
- AI decision to harvest (E13)
- Seasonal or weather-based regeneration modifiers (future)

## Acceptance Criteria

### Tests That Must Pass

1. A `ResourceSource` with `regeneration_ticks_per_unit: Some(NonZeroU32::new(5))` gains 1 unit every 5 ticks.
2. Regeneration stops at `max_quantity` — no overflow past cap.
3. A `ResourceSource` with `regeneration_ticks_per_unit: None` never regenerates.
4. A `ResourceSource` already at `max_quantity` does not emit spurious regeneration events.
5. Regeneration emits an event with causal linkage.
6. Multiple `ResourceSource` entities regenerate independently.
7. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. `available_quantity` never exceeds `max_quantity`.
2. Regeneration is explicit and event-logged — not silent state mutation.
3. No floating-point arithmetic in regeneration calculation.
4. Deterministic: same seed + same inputs = same regeneration sequence.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/production.rs` — regeneration rate, cap enforcement, no-regen case, event emission

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
