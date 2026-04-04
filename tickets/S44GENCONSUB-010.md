# S44GENCONSUB-010: Lawful unique-item pickup contention

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — transport action architecture, component registration, contention attachment for ground unique items
**Deps**: S44GENCONSUB-005, S44GENCONSUB-006

## Problem

The active S44 spec still names ground `UniqueItem` pickup as a Phase 1 contention domain, but the live runtime has no lawful unique-item pickup affordance at all. `crates/worldwake-systems/src/transport_actions.rs` registers `pick_up` only for `EntityKind::ItemLot`, so attaching `ContentionQueue` to ground unique items today would be fake substrate with no action path that could consume it. This ticket owns the missing affordance path plus the corresponding race-mode contention integration for unowned ground unique items.

## Assumption Reassessment (2026-04-04)

1. `pick_up` in `crates/worldwake-systems/src/transport_actions.rs` targets `EntityKind::ItemLot` only in both target spec and validation. Confirmed.
2. `UniqueItem` entities already have physical placement and possession state in `worldwake-core`; they can be on the ground and inside containers, but no transport action currently moves them through the human/AI action surface. Confirmed via `world/placement.rs` and `world.rs`.
3. The active spec `specs/S44-generalized-contention-substrate.md` is internally inconsistent: its Phase 1 table includes ground unique-item pickup contention, while its component-registration text still limits contention to `Agent || Facility`. Ticket says unique-item contention is Phase 1; live action surface has no pickup affordance; correction applied: this ticket owns the missing affordance path before contention attachment; why safe: FOUNDATIONS P8/P9/P12/P20 require an explicit lawful affordance before contention state can honestly arbitrate it.
4. `ContentionPolicy { auto_promote: false, max_waiters: Some(0) }` already expresses the intended race-mode contract for unique-item pickup once the action exists. Confirmed from `contention.rs` and the S44 spec.
5. This is a mixed transport/contention boundary ticket. Shared abstraction under audit: physical ground-item pickup semantics in `transport_actions.rs` plus contention-state registration and affordance annotation for `EntityKind::UniqueItem`.

## Architecture Check

1. Creating the lawful unique-item pickup affordance first is cleaner than attaching contention components to entities the runtime cannot yet act upon. It preserves explicit world processes instead of fake placeholder state.
2. No backward-compatibility shims. The transport surface should be widened directly rather than duplicating item-lot and unique-item pickup under parallel action families.

## Verification Layers

1. Ground unique item exposes a lawful pickup affordance -> focused transport affordance/runtime test
2. Unowned ground unique item receives race-mode contention state when it becomes ground-accessible -> authoritative world state check
3. First actor can acquire the race-mode grant and second actor receives structured contention rejection -> authoritative start validation / action trace
4. The widened transport path does not regress existing item-lot pickup semantics -> focused transport regression tests

## What to Change

### 1. Widen the transport pickup architecture to cover ground unique items

Refactor `pick_up` or its transport equivalent so a lawful action surface exists for `EntityKind::UniqueItem` on the ground. Keep the action family unified if the existing transport abstractions can support that without duplication.

### 2. Register contention for unowned ground unique items

Once unique-item pickup is lawful, extend `ContentionQueue` / `ContentionPolicy` registration and lifecycle attachment so unowned ground unique items receive race-mode contention:

- `grant_hold_ticks`: short hold window suitable for pickup races
- `auto_promote: false`
- `max_waiters: Some(0)`

### 3. Integrate contention-aware pickup validation

Make unique-item pickup obey the same generalized contention validation path already used for other contention-managed affordances, with structured rejection when another actor already holds the race-mode grant.

## Files to Touch

- `crates/worldwake-systems/src/transport_actions.rs` (modify — lawful unique-item pickup path)
- `crates/worldwake-core/src/component_schema.rs` (modify — extend contention registration if unique items become a live contention kind)
- `crates/worldwake-systems/src/<appropriate lifecycle boundary>.rs` (modify — attach/remove race-mode contention for ground unique items once the live pickup path is defined)

## Out of Scope

- Corpse/patient contention attachment (`S44GENCONSUB-007`)
- Perception of contention state (`S44GENCONSUB-008`)
- Golden contention proof (`S44GENCONSUB-009`)

## Acceptance Criteria

### Tests That Must Pass

1. Ground unique item exposes a lawful pickup affordance through the real transport action surface
2. Unowned ground unique item carries race-mode contention policy
3. Second claimant receives structured contention rejection while the first holds the grant
4. Existing suite: `cargo test --workspace`

### Invariants

1. Unique-item contention is only attached once a lawful unique-item pickup affordance exists
2. Race-mode pickup still resolves through inspectable grant state, not invisible tick order

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/transport_actions.rs` — lawful unique-item pickup path plus contention-aware rejection
2. `crates/worldwake-core/src/component_schema.rs` or owning lifecycle tests — unique-item contention registration/attachment

### Commands

1. `cargo test -p worldwake-systems transport_actions`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
