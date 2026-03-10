# E10PROTRA-011: Pick-up + Put-down actions in worldwake-systems

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new ActionDefs + ActionHandlers
**Deps**: E10PROTRA-003 (CarryCapacity component must exist)

## Problem

Goods move only through carried containment under a moving agent. Pick-up and put-down are the physical actions that transfer goods between a location and an agent's carried inventory. These actions must enforce `CarryCapacity` limits using the existing `LoadUnits` infrastructure. Without these, there is no way for agents to acquire goods for transport.

## Assumption Reassessment (2026-03-10)

1. `CarryCapacity(LoadUnits)` will exist on Agent entities after E10PROTRA-003 — confirmed.
2. `LoadUnits` and the per-unit load system exist in `worldwake-core/src/load.rs` — confirmed.
3. `ItemLot` entities have commodity + quantity. Per-unit loads are defined per `CommodityKind`.
4. The ownership/containment relation system in `relations.rs` tracks who owns/carries what.
5. Pick-up and put-down are physical actions — they take time (even if brief) and are not instantaneous.
6. The spec says "Pick-up / put-down remain physical actions" — confirming they go through the action framework.

## Architecture Check

1. Pick-up transfers an item lot (or partial quantity) from a co-located container/ground to the agent's carried inventory.
2. Put-down transfers from agent's carried inventory to a co-located container/ground.
3. Carry capacity enforcement: before pick-up, compute current carried load (derived from owned item lots) and verify remaining capacity >= item load.
4. These are relatively simple actions — short duration (1-2 ticks or configurable), minimal preconditions beyond co-location and capacity.
5. Partial pick-up: if an agent can only carry 5 of 10 apples, they pick up 5. The lot is split.

## What to Change

### 1. New module `crates/worldwake-systems/src/transport_actions.rs`

Define:
- `pick_up_handler`: ActionHandler
  - **start**: Validate co-location with target item lot, validate carry capacity sufficient, reserve item
  - **tick**: Brief (1 tick typical)
  - **commit**: Transfer ownership of item lot (or split lot) to agent, emit event
  - **abort**: Release reservation, no change
- `put_down_handler`: ActionHandler
  - **start**: Validate agent carries the item
  - **tick**: Brief (1 tick typical)
  - **commit**: Transfer ownership from agent to location/container, emit event
  - **abort**: No change

- `ActionDef` for PickUp:
  - Actor constraints: has `CarryCapacity`
  - Targets: item lot entity
  - Preconditions: co-located with item, remaining capacity >= item load
  - Duration: configurable (1 tick default)

- `ActionDef` for PutDown:
  - Actor constraints: owns/carries the target item
  - Targets: item lot entity
  - Duration: configurable (1 tick default)

### 2. Register and export

## Files to Touch

- `crates/worldwake-systems/src/transport_actions.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify — add module + export registration)
- `crates/worldwake-core/src/event_tag.rs` (modify — add PickUp/PutDown event tags if needed)

## Out of Scope

- Travel actions (E10PROTRA-010)
- Container-to-container transfer (future)
- Vehicle or cart loading (future)
- AI decision to pick up (E13)
- Trade/exchange actions (E11)
- Theft mechanics (E17)

## Acceptance Criteria

### Tests That Must Pass

1. **Carry capacity is enforced via `LoadUnits`** — pick-up fails if remaining capacity < item load.
2. **Pick-up transfers item lot ownership to agent**.
3. **Put-down transfers item lot ownership from agent to location**.
4. **Co-location enforced**: cannot pick up items at a different location.
5. **Partial pick-up**: if capacity allows only partial quantity, lot is split and partial quantity picked up.
6. **Put-down of non-carried item fails**.
7. **Events emitted** for both pick-up and put-down.
8. **No teleportation path moves goods without a carrier** — goods at location A cannot appear at location B without an agent carrying them through travel.
9. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Conservation: pick-up and put-down only transfer ownership — no goods created or destroyed.
2. Carry capacity: total carried load never exceeds `CarryCapacity`.
3. Goods move only through containment — no teleportation side channel.
4. No floating-point arithmetic.
5. Deterministic behavior.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/transport_actions.rs` — pick-up happy path, capacity enforcement, co-location check, partial pick-up (lot split), put-down happy path, put-down non-carried failure, conservation, event emission

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
