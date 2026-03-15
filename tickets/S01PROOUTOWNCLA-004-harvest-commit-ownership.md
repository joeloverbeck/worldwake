# S01PROOUTOWNCLA-004: Update harvest commit to resolve and assign output ownership

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — harvest action handler
**Deps**: S01PROOUTOWNCLA-001 (types), S01PROOUTOWNCLA-002 (helper)

## Problem

The harvest commit handler (`commit_harvest` in `production_actions.rs:511-564`) creates output lots without ownership. It must resolve the `ProductionOutputOwnershipPolicy` from the workstation/source entity and assign ownership using `create_item_lot_with_owner()`.

## Assumption Reassessment (2026-03-15)

1. `commit_harvest` at `production_actions.rs:511-564` creates lots via `txn.create_item_lot()` + `txn.set_ground_location()` — confirmed
2. The workstation/source is `instance.targets[0]` — confirmed
3. `WorldTxn` provides read access to components via `txn.get_component::<T>(entity)` or similar — to verify exact API
4. `owner_of()` is accessible from within `WorldTxn` — confirmed
5. `ProductionOutputOwnershipPolicy` will be available on Facility and Place entities after S01PROOUTOWNCLA-001

## Architecture Check

1. Policy resolution happens at commit time, not at action start — this preserves the correct ownership at the moment of production, not at the moment of action initiation
2. `ProducerOwner` with ownerless producer MUST fail the commit (no silent degradation)
3. Uses `create_item_lot_with_owner()` for atomic creation — no forgotten ownership

## What to Change

### 1. Add ownership resolution helper

Add a helper function (private to the module) that resolves `ProductionOutputOwnershipPolicy` to `Option<EntityId>`:

```rust
fn resolve_output_owner(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    producer: EntityId,
) -> Result<Option<EntityId>, ActionError> {
    let policy = txn.get_component::<ProductionOutputOwnershipPolicy>(producer);
    match policy {
        Some(p) => match p.output_owner {
            ProductionOutputOwner::Actor => Ok(Some(actor)),
            ProductionOutputOwner::ProducerOwner => {
                let owner = txn.owner_of(producer)
                    .ok_or_else(|| ActionError::PreconditionFailed(format!(
                        "producer {producer} has ProducerOwner policy but no owner"
                    )))?;
                Ok(Some(owner))
            }
            ProductionOutputOwner::Unowned => Ok(None),
        },
        None => Err(ActionError::PreconditionFailed(format!(
            "producer {producer} has no ProductionOutputOwnershipPolicy"
        ))),
    }
}
```

### 2. Update `commit_harvest`

Replace the `create_item_lot()` + `set_ground_location()` calls with:
1. `let owner = resolve_output_owner(txn, instance.actor, workstation)?;`
2. `let lot = txn.create_item_lot_with_owner(commodity, quantity, place, owner)?;`

## Files to Touch

- `crates/worldwake-systems/src/production_actions.rs` (modify — update `commit_harvest`, add `resolve_output_owner`)

## Out of Scope

- Craft commit changes (S01PROOUTOWNCLA-005 — same pattern, separate ticket for reviewability)
- Defining the policy types (S01PROOUTOWNCLA-001)
- Creating the helper method (S01PROOUTOWNCLA-002)
- Pickup validation (S01PROOUTOWNCLA-007, -008)
- Test fixture migration (S01PROOUTOWNCLA-009)

## Acceptance Criteria

### Tests That Must Pass

1. Harvest with `Actor` policy creates actor-owned, unpossessed ground lot
2. Harvest with `ProducerOwner` policy creates producer-owner-owned output
3. Harvest with `Unowned` policy creates unowned output
4. `ProducerOwner` policy on ownerless producer fails commit rather than degrading silently
5. Missing policy on producer fails commit
6. Ownership assignment produces `RelationDelta::OwnerSet` in committed event deltas
7. Output lot is at the workstation's place (not in actor inventory)
8. Output lot is unpossessed
9. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Every harvest output has explicit ownership semantics — no silent unowned creation
2. Ground materialization preserved — output is at place, not auto-possessed
3. `ProducerOwner` with no owner is an authoritative failure, never silently degraded
4. Ownership is recorded in event deltas for traceability

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/production_actions.rs` test module — Actor policy, ProducerOwner policy, Unowned policy, ownerless ProducerOwner failure, missing policy failure

### Commands

1. `cargo test -p worldwake-systems harvest`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace`
