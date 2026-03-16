# S01PROOUTOWNCLA-007: Add ownership check to authoritative pick_up validation

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — action validation, transport action preconditions
**Deps**: S01PROOUTOWNCLA-003 (extended `can_exercise_control()`)

## Problem

`pick_up` is currently a universal "grab anything on the floor" action. Once production outputs have ownership, `pick_up` must become the lawful custody-taking action: actor may pick up an unpossessed lot only if it is unowned or `can_exercise_control(actor, lot)` succeeds.

## Assumption Reassessment (2026-03-15)

1. `validate_pick_up` at `transport_actions.rs:131-178` checks colocality, item lot type, not in container, unpossessed — confirmed
2. `evaluate_precondition_authoritatively()` at `action_validation.rs` validates against world state — confirmed
3. `Precondition::ActorCanControlTarget` exists and calls `can_exercise_control()` — confirmed
4. `pick_up` action def preconditions at `transport_actions.rs:48-56` include `TargetUnpossessed` but no ownership check — confirmed

## Architecture Check

1. Adding `ActorCanControlTarget` or a new `TargetUnownedOrActorControls` precondition to the `pick_up` action def is the cleanest approach
2. Alternative: add ownership check directly in `validate_pick_up()` — this is more explicit and doesn't require a new precondition type
3. Recommendation: use the existing `can_exercise_control()` in `validate_pick_up()` since the check is conditional (unowned lots are freely pickable, so a blanket `ActorCanControlTarget` would reject unowned lots)

## What to Change

### 1. Update `validate_pick_up()` in `transport_actions.rs`

After the "unpossessed" check, add an ownership check:

```rust
// Ownership check: actor can pick up only if unowned or actor can exercise control
if let Some(_owner) = txn.owner_of(target) {
    txn.can_exercise_control(actor, target)
        .map_err(|e| ActionError::PreconditionFailed(format!(
            "actor {actor} cannot lawfully pick up owned entity {target}: {e}"
        )))?;
}
// Unowned lots remain freely pickable (no owner means no restriction)
```

### 2. Update authoritative validation in `action_validation.rs`

If `pick_up` validation goes through `evaluate_precondition_authoritatively()`, ensure the ownership check is also enforced there. This depends on whether `validate_pick_up()` is the sole validation path or if preconditions are also checked independently.

Verify: does `pick_up` use both the `ActionDef` precondition list AND the `validate_pick_up` function, or just one? Adjust accordingly to ensure no bypass.

## Files to Touch

- `crates/worldwake-systems/src/transport_actions.rs` (modify — add ownership check to `validate_pick_up`)
- `crates/worldwake-sim/src/action_validation.rs` (modify — if separate validation path exists)

## Out of Scope

- Belief-based affordance filtering (S01PROOUTOWNCLA-008)
- `put_down` semantics (unchanged per spec — clears possession, preserves ownership)
- Theft implementation (E17)
- Travel possession semantics (unchanged — possessed lots travel with actor)

## Acceptance Criteria

### Tests That Must Pass

1. Lawful `pick_up` succeeds for actor-owned local output
2. Lawful `pick_up` succeeds for unowned output
3. Lawful `pick_up` rejects owned local output when actor lacks control
4. Lawful `pick_up` succeeds for faction member on faction-owned unpossessed lot
5. Lawful `pick_up` succeeds for office holder on office-owned unpossessed lot
6. `put_down` preserves ownership while clearing possession
7. Travel continues to move possessed lots without changing ownership
8. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Unowned lots remain freely pickable by anyone
2. Owned lots require `can_exercise_control()` to succeed
3. Possession overrides ownership (already handled by `can_exercise_control()`)
4. No bypass path exists — all pick_up paths enforce ownership check
5. `put_down` does NOT change ownership (only clears possession)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/transport_actions.rs` test module — lawful pickup of owned/unowned lots, rejection of unauthorized pickup, faction/office delegation, put_down ownership preservation

### Commands

1. `cargo test -p worldwake-systems pick_up`
2. `cargo test -p worldwake-systems transport`
3. `cargo test -p worldwake-systems`
4. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-16
- **What changed**: Added ownership check to `validate_pick_up()` in `crates/worldwake-systems/src/transport_actions.rs`. Owned unpossessed lots now require `can_exercise_control(actor, lot)` to succeed; unowned lots remain freely pickable.
- **Deviations**: None — implemented as specified (option 3: conditional check in `validate_pick_up` rather than a new precondition type).
- **Verification**: `cargo test -p worldwake-systems`, `cargo clippy --workspace` — all pass. Commit `77560e3`.
