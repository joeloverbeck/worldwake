# E10PROTRA-007: ResourceSource regeneration system in worldwake-systems

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new system function in systems crate
**Deps**: E10PROTRA-002 (ResourceSource component must exist)

## Problem

Resource sources (orchards, farms, forests) need to regenerate over time according to their `regeneration_ticks_per_unit` parameter. Without a tick-driven regeneration system, harvested sources never replenish, making production unsustainable.

## Assumption Reassessment (2026-03-10)

1. `ResourceSource` will exist as an authoritative component after E10PROTRA-002 — confirmed dependency.
2. `ResourceSource` already includes `last_regeneration_tick: Option<Tick>` in `worldwake-core`; this ticket does not need to add it.
3. The systems crate already has the tick-system pattern to follow: `needs_system` in `needs.rs` mutates authoritative state through `WorldTxn` and commits an event-log record.
4. `SystemManifest` already includes the closed `Production` system slot. This ticket should not modify `SystemManifest`; it should fill the existing `Production` handler slot in `worldwake_systems::dispatch_table()`.
5. `SystemDispatchTable` is a fixed six-entry table indexed by `SystemId`; there is no separate dynamic registration layer to add here.
6. `EventTag` already has the generic tags needed for this work. Regeneration should use the existing `System` + `WorldMutation` tags and component deltas instead of introducing a bespoke regeneration tag.
7. Regeneration remains simple at this stage: if `regeneration_ticks_per_unit` is `Some(n)`, increment `available_quantity` by 1 every `n` ticks, capped at `max_quantity`.

## Architecture Reassessment

1. Implementing regeneration in the `Production` system slot is more beneficial than pushing it into `worldwake-core` or into ad hoc action code. Regeneration is ongoing world evolution, not a one-off action, so the tick loop is the clean architectural home.
2. Reusing the existing `WorldTxn` + event-log delta model is better than adding a special event type. The event log already captures causal provenance, tags, targets, and component deltas; adding a parallel mechanism would make the architecture less coherent.
3. Adding more registry or alias layers here would be architecture debt. The current fixed `SystemId` + `SystemDispatchTable` model is explicit and sufficient; this ticket should extend it directly.
4. The only architectural gap worth closing in this ticket is the empty `Production` system path in `worldwake-systems`. Filling that gap improves extensibility for later E10 work without introducing compatibility shims.

## Architecture Check

1. Regeneration is a world-state mutation driven by the tick loop — it belongs in `worldwake-systems`.
2. The system reads `ResourceSource` components, checks regeneration eligibility, and writes updated `available_quantity` through `WorldTxn`.
3. Regeneration must commit through `WorldTxn` so the change becomes an append-only event-log record with `CauseRef::SystemTick(tick)` and the usual system/world-mutation tags.
4. The system must respect the conservation invariant: regeneration creates new quantity only up to `max_quantity`. This is a legitimate natural-growth mutation, but it must be explicit and event-logged.
5. Sources with `last_regeneration_tick: None` need a deterministic baseline. The cleanest behavior is to initialize the baseline in authoritative state and wait a full interval before the first regenerated unit, rather than deriving from global tick modulus or replaying old events.

## What to Change

### 1. New module `crates/worldwake-systems/src/production.rs`

```rust
/// Regeneration system: replenishes ResourceSource entities each tick.
pub fn resource_regeneration_system(world: &mut WorldTxn, tick: Tick) {
    // For each entity with ResourceSource:
    //   If regeneration_ticks_per_unit is Some(n):
    //     If (tick - last_regen_tick) >= n AND available < max:
    //       Increment available_quantity by 1
    //       Commit a world-mutation event through WorldTxn
}
```

Use the already-existing `last_regeneration_tick` field on `ResourceSource` to track the authoritative regeneration baseline.

### 2. Wire into the existing systems dispatch table

Set the `Production` slot in `worldwake_systems::dispatch_table()` to `resource_regeneration_system` while leaving the other existing slots unchanged.

### 3. Export from `lib.rs`

## Files to Touch

- `crates/worldwake-systems/src/production.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify — add module + export production dispatch wiring)

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
5. Regeneration commits an event-log record with `CauseRef::SystemTick`, `EventTag::System`, and `EventTag::WorldMutation`.
6. Multiple `ResourceSource` entities regenerate independently.
7. A partially depleted `ResourceSource` with `last_regeneration_tick: None` establishes a deterministic baseline and waits a full interval before the first regenerated unit.
8. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. `available_quantity` never exceeds `max_quantity`.
2. Regeneration is explicit and event-logged — not silent state mutation.
3. No floating-point arithmetic in regeneration calculation.
4. Deterministic: same seed + same inputs = same regeneration sequence.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/production.rs` — regeneration rate, cap enforcement, no-regen case, event emission, deterministic baseline initialization

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completed: 2026-03-10
- What actually changed:
  - Added `crates/worldwake-systems/src/production.rs` with `resource_regeneration_system`.
  - Wired the `Production` slot in `worldwake_systems::dispatch_table()` to the new system.
  - Moved shared dispatch-table assembly out of `needs.rs` and into `crates/worldwake-systems/src/lib.rs`, which is the cleaner architectural owner for crate-level system composition.
  - Added targeted regeneration tests covering cadence, cap behavior, disabled regeneration, deterministic baseline initialization for `last_regeneration_tick: None`, multi-source independence, and dispatch wiring.
- Deviations from original plan:
  - No `worldwake-core` changes were needed because `ResourceSource.last_regeneration_tick` already existed.
  - No `EventTag` changes were needed because the existing `System` and `WorldMutation` tags already model this mutation cleanly.
  - No `SystemManifest` changes were needed because the `Production` system slot already existed; only the systems-crate dispatch table needed wiring.
- Verification results:
  - `cargo test -p worldwake-systems` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
