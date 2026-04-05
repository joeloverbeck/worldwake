# S44GENCONSUB-010: Lawful unique-item pickup contention

**Status**: COMPLETED
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
5. This is a mixed transport/sim/contention boundary ticket. Shared abstraction under audit: the transport pickup action surface for ground entities, the `TargetSpec`/affordance binding layer that exposes pickup candidates, and the authoritative contention-state attachment path for unowned ground `UniqueItem` entities.

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

Refactor `pick_up` so a lawful action surface exists for both ground `ItemLot` and ground `UniqueItem` targets. The clean live path is to widen the shared target-binding surface in `worldwake-sim` rather than duplicating pickup under a second action family.

### 2. Register contention for unowned ground unique items

Once unique-item pickup is lawful, extend `ContentionQueue` / `ContentionPolicy` registration and the authoritative contention-system maintenance path so unowned ground unique items receive race-mode contention:

- `grant_hold_ticks`: short hold window suitable for pickup races
- `auto_promote: false`
- `max_waiters: Some(0)`

### 3. Integrate contention-aware pickup validation

Make unique-item pickup obey a lawful race-mode grant path: first claimant acquires the short-lived grant at action start, later claimants receive structured contention rejection while that grant is active, and commit/abort clears the matching pickup grant cleanly.

## Files to Touch

- `crates/worldwake-systems/src/transport_actions.rs` (modify — unified lawful pickup path plus race-mode grant claim/release)
- `crates/worldwake-systems/src/facility_queue.rs` (modify — attach/remove race-mode contention for eligible ground unique items in the authoritative contention sweep)
- `crates/worldwake-core/src/component_schema.rs` (modify — allow contention components on `EntityKind::UniqueItem`)
- `crates/worldwake-sim/src/action_semantics.rs` (modify — widen shared pickup target binding without duplicating the action family)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — enumerate the widened pickup targets through the live affordance path)

## Out of Scope

- Corpse/patient contention attachment (`S44GENCONSUB-007`)
- Perception of contention state (`S44GENCONSUB-008`)
- Golden contention proof (`S44GENCONSUB-009`)

## Acceptance Criteria

### Tests That Must Pass

1. Ground unique item exposes a lawful pickup affordance through the real transport action surface
2. Unowned ground unique item receives race-mode contention state through the authoritative contention maintenance path
3. Second claimant receives structured contention rejection while the first holds the unique-item pickup grant
4. Existing suite: `cargo test --workspace`

### Invariants

1. Unique-item contention is only attached once a lawful unique-item pickup affordance exists
2. Race-mode pickup still resolves through inspectable grant state, not invisible tick order

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/transport_actions.rs` — lawful unique-item pickup path plus grant/rejection cleanup
2. `crates/worldwake-systems/src/facility_queue.rs` — unique-item contention attachment/removal
3. `crates/worldwake-sim/src/affordance_query.rs` — widened pickup target enumeration

### Commands

1. `cargo test -p worldwake-systems transport_actions`
2. `cargo test -p worldwake-systems facility_queue`
3. `cargo test -p worldwake-sim affordance_query`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`

## Outcome

- **Completed**: 2026-04-04
- **What changed**:
  - widened the unified `pick_up` / `put_down` transport surface so ground and directly possessed `UniqueItem` entities use the same action family as `ItemLot`s
  - widened the shared action-target binding surface in `worldwake-sim` with `TargetSpec` support for multi-kind actor-place and direct-possession targets
  - allowed `ContentionPolicy` and `ContentionQueue` on `EntityKind::UniqueItem`
  - attached and removed race-mode contention state for eligible unowned ground unique items in the authoritative contention-system sweep
  - made unique-item pickup claim a short-lived race-mode grant at action start, reject later claimants with structured `contention_rejected`, and clear grant state on commit or abort
  - added focused proof for unique-item pickup grant races, unique-item put-down reattachment, background contention attachment/removal, and widened affordance target enumeration
- **Deviations from original plan**:
  - the live action schema could not keep `pick_up` unified without a small shared `TargetSpec` / affordance-layer widening, so the ticket was corrected to include `crates/worldwake-sim/src/action_semantics.rs` and `crates/worldwake-sim/src/affordance_query.rs`
  - unique-item contention attachment landed partly in the contention maintenance sweep rather than only inside transport handlers so ground eligibility stays authoritative even when state changes outside direct pickup/put-down actions
- **Verification**:
  - `cargo test -p worldwake-systems transport_actions`
  - `cargo test -p worldwake-systems facility_queue`
  - `cargo test -p worldwake-sim affordance_query`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
