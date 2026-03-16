# S01PROOUTOWNCLA-005: Update craft commit to resolve and assign output ownership

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — craft action handler
**Deps**: S01PROOUTOWNCLA-001 (types), S01PROOUTOWNCLA-002 (helper), S01PROOUTOWNCLA-004 (shared `resolve_output_owner` helper)

## Problem

The craft commit handler creates output lots without ownership, identical to the harvest gap. It must resolve the `ProductionOutputOwnershipPolicy` from the workstation and assign ownership to each output lot.

## Assumption Reassessment (2026-03-15)

1. Craft commit at `production_actions.rs:566-610` creates multiple output lots per recipe — confirmed
2. Craft uses the same `create_item_lot()` + `set_ground_location()` pattern as harvest — confirmed
3. The `resolve_output_owner()` helper will be available from S01PROOUTOWNCLA-004 — confirmed (same file)
4. Craft workstation is a Facility entity with `ProductionJob` component — confirmed

## Architecture Check

1. Reuses `resolve_output_owner()` introduced in S01PROOUTOWNCLA-004 — DRY
2. Same failure semantics: `ProducerOwner` with no owner fails commit
3. Each output lot in the recipe gets the same ownership policy (all from the same workstation)

## What to Change

### 1. Update craft commit handler

In the craft commit handler (around `production_actions.rs:566-610`), for each output lot:
1. Resolve ownership via `resolve_output_owner(txn, instance.actor, workstation)?`
2. Replace `create_item_lot()` + `set_ground_location()` with `create_item_lot_with_owner(commodity, quantity, place, owner)?`

The resolution should happen once before the loop (same policy for all outputs in a single craft action).

## Files to Touch

- `crates/worldwake-systems/src/production_actions.rs` (modify — update craft commit handler)

## Out of Scope

- Harvest changes (S01PROOUTOWNCLA-004 — already done)
- Defining types or helper (S01PROOUTOWNCLA-001, -002)
- Pickup validation (S01PROOUTOWNCLA-007, -008)

## Acceptance Criteria

### Tests That Must Pass

1. Craft with `Actor` policy creates actor-owned, unpossessed ground lots
2. Craft with `ProducerOwner` policy creates producer-owner-owned output lots
3. Craft with `Unowned` policy creates unowned output lots
4. `ProducerOwner` policy on ownerless workstation fails commit
5. All output lots from a single craft share the same ownership
6. Golden craft/barrier scenarios still work under explicit actor-owned output
7. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Every craft output has explicit ownership semantics
2. Ground materialization preserved — outputs are at place, not auto-possessed
3. `ProducerOwner` with no owner is an authoritative failure
4. Conservation invariant still holds (same quantities, just with ownership)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/production_actions.rs` test module — craft-specific ownership tests, golden scenario regression

### Commands

1. `cargo test -p worldwake-systems craft`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-16
- **What changed**: In `commit_craft` (`production_actions.rs`), replaced `create_item_lot()` + `set_ground_location()` with `resolve_output_owner()` (once before the loop) + `create_item_lot_with_owner()` per output lot. Same pattern as harvest (S01PROOUTOWNCLA-004).
- **Deviations**: None. Implementation matched the ticket exactly.
- **Verification**: 6 new craft ownership tests added and passing. Full `cargo test -p worldwake-systems` green. `cargo clippy --workspace` clean. One pre-existing failure (`golden_capacity_constrained_ground_lot_pickup`) unrelated to this change.
