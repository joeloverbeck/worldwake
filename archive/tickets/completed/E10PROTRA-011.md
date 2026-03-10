# E10PROTRA-011: Pick-up + Put-down actions in worldwake-systems

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new action defs/handlers and focused action-framework surface additions
**Deps**: E10PROTRA-003 (`CarryCapacity` component must exist)

## Problem

E10 requires that goods move physically through carriers rather than teleportation. The codebase already has the core transport primitives for that model:

- `CarryCapacity(LoadUnits)` on agents
- possession relations for "who is carrying this now"
- container relations for nested storage
- `effective_place` and transit state so carried goods move with the carrier

What is still missing is the physical action layer that moves a ground item lot into an agent's carried inventory, and back out onto the ground. Without those actions, the transport architecture is incomplete: agents can travel with possessions, but there is no action path to acquire or drop the goods being transported.

## Assumption Reassessment (2026-03-10)

1. `CarryCapacity(LoadUnits)` already exists on agents in `worldwake-core` and has component tests.
2. `LoadUnits` infrastructure already exists in `worldwake-core/src/load.rs`, including per-commodity load, per-entity load, and remaining container capacity helpers.
3. Carrying is modeled through `possessed_by`, not `owned_by`. Ownership and possession are intentionally independent in the current architecture.
4. Agents are not containers. A direct pick-up therefore means assigning possession of the target entity to the actor, not inserting it into an "agent inventory container."
5. Nested carried storage already exists through container entities that are themselves possessed by the actor. Container-to-container transfer is a separate concern from basic pick-up / put-down.
6. Travel already propagates carried goods by moving the actor's direct possessions through transit, and nested contents follow through containment.
7. Existing action framework surface is intentionally narrow. It currently supports `Harvest`, `Craft`, and `None` payloads plus `Travel` local state, so pick-up / put-down may require small, explicit additions there.
8. Existing event tags already cover this feature. `Inventory` and `Transfer` should be reused; new event tag variants are not needed.

## Architecture Decision

Implement pick-up and put-down as physical actions over the current possession model.

- `pick_up`: ground item lot at actor place -> possessed by actor
- `put_down`: possessed item lot -> ground item lot at actor place

This is a better fit than the original ticket framing of "transfer ownership to agent inventory" because:

1. It matches the code that already powers control, travel, and nested carried storage.
2. It avoids inventing a second carrying path parallel to possession.
3. It preserves the separation between ownership and possession, which the current world model already tests explicitly.
4. It keeps the first transport implementation small and robust while leaving room for later container-targeted loading actions if needed.

## Scope

### In Scope

1. Ground-to-actor pick-up for `ItemLot`
2. Actor-to-ground put-down for carried `ItemLot`
3. Carry-capacity enforcement using `CarryCapacity` plus current carried load
4. Partial pick-up by splitting an item lot when the actor can carry only part of it
5. Event emission through existing action/event infrastructure using existing tags
6. Focused action-framework additions required to represent these actions cleanly

### Out of Scope

1. Container-to-container transfer
2. Ground-to-container loading in one action
3. Put-down into a target container
4. Unique-item pick-up / put-down
5. Vehicle/cart loading
6. Trade/exchange actions
7. Theft semantics
8. AI selection of these actions

## What To Change

### 1. Add transport action registration in `worldwake-systems`

Create a focused module for transport actions and export its registration from `crates/worldwake-systems/src/lib.rs`.

Define:

- `register_transport_actions`
- `pick_up` action def/handler
- `put_down` action def/handler

### 2. Model actions against possession, not ownership

Pick-up commit should:

- validate the actor and target are co-located
- validate the target is an `ItemLot`
- validate the lot is not already possessed
- compute current carried load from the actor's direct possessions
- split the lot if only a partial quantity fits
- clear ground placement on the moved lot if needed
- set possessor to the actor

Put-down commit should:

- validate the actor currently controls the target by possession
- reject non-possessed targets
- clear possessor on the target
- set ground location to the actor's current place

Ownership should remain unchanged throughout both actions.

### 3. Keep action-framework additions minimal

Add only the smallest new action-payload / state / validation surface needed for these transport actions. Do not generalize into a broad transfer framework unless the implementation shows unavoidable duplication.

### 4. Reuse existing event tags

Use existing tags such as:

- `EventTag::Inventory`
- `EventTag::Transfer`

Do not add new `EventTag` variants for pick-up / put-down.

## Files Expected To Change

- `crates/worldwake-systems/src/transport_actions.rs` (new)
- `crates/worldwake-systems/src/lib.rs`
- `crates/worldwake-sim/src/action_payload.rs` (if transport payload is needed)
- `crates/worldwake-sim/src/action_state.rs` (only if explicit transport state is needed)
- `crates/worldwake-sim/src/action_validation.rs` (if targeted preconditions/constraints are needed)
- related tests in the touched files

## Acceptance Criteria

1. Pick-up transfers an `ItemLot` from ground at the actor's place into the actor's possessions.
2. Put-down transfers a possessed `ItemLot` from the actor onto ground at the actor's place.
3. Carry capacity is enforced via `CarryCapacity` and current carried load.
4. Partial pick-up splits the source lot deterministically when only part fits.
5. Pick-up fails when the target is not co-located with the actor.
6. Pick-up fails when the actor lacks `CarryCapacity` or has no remaining capacity.
7. Put-down fails when the actor does not currently possess the target.
8. Ownership is unchanged by pick-up / put-down.
9. Travel continues to move picked-up goods through possession without any new transport side channel.
10. Relevant crate and workspace tests pass.

## Invariants

1. Conservation: transfers only move existing goods; they do not create or destroy quantity.
2. Carry capacity: direct carried load never exceeds `CarryCapacity`.
3. No teleportation: transport still flows only through possession plus travel.
4. Ownership/possession separation remains intact.
5. Deterministic behavior only; no floating-point arithmetic.

## Test Plan

### New / Modified Tests

1. `crates/worldwake-systems/src/transport_actions.rs`
   - register defs
   - pick-up happy path
   - put-down happy path
   - co-location failure
   - capacity failure
   - partial pick-up split
   - put-down non-possessed failure
   - ownership preserved
   - event tag emission
2. `crates/worldwake-systems/src/travel_actions.rs`
   - confirm a picked-up lot still follows actor travel through possession if existing coverage is insufficient
3. `crates/worldwake-sim/src/action_payload.rs`
   - roundtrip coverage if a transport payload variant is introduced
4. `crates/worldwake-sim/src/action_validation.rs`
   - targeted validation coverage if new transport-specific preconditions/constraints are introduced

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

Implemented:

1. `pick_up` and `put_down` action defs/handlers in `worldwake-systems`
2. minimal action-surface additions needed to model direct possession and ground-only pickup cleanly
3. carry-capacity enforcement from `CarryCapacity` plus recursively carried load
4. deterministic partial lot splitting for over-capacity pick-up
5. focused affordance/validation coverage so ground lots, directly carried lots, and contained lots are distinguished correctly

Changed from the original plan:

1. carrying remained possession-based; ownership transfer was not introduced
2. no new event-tag variants were added; existing inventory/transfer tags were reused
3. the implementation added small `BeliefView` and action-semantics surface expansions so affordances stay architecturally correct instead of relying on handler-time rejection alone
4. scope stayed on ground <-> direct possession transfer for `ItemLot`; container-targeted loading remains future work
