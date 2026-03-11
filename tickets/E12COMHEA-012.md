# E12COMHEA-012: Loot action definition + handler

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — worldwake-sim (action def) + worldwake-systems (handler)
**Deps**: E12COMHEA-004 (LootActionPayload), E12COMHEA-002 (DeadAt), E12COMHEA-005 (TargetDead precondition), E12COMHEA-006 (wound helpers)

## Problem

The Loot action allows a living agent to transfer items from a dead agent's inventory to their own. This is the only corpse interaction supported in Phase 2. It requires co-location, the target having `DeadAt`, and the looter being alive and not incapacitated.

## Assumption Reassessment (2026-03-11)

1. Relation APIs for ownership transfer exist: `transfer_ownership()` or equivalent in worldwake-core relations — need to verify exact method.
2. Carry capacity / load checking exists from E04 — item transfer must respect it.
3. `TargetDead(u8)` precondition from E12COMHEA-005 validates target has `DeadAt`.
4. `LootActionPayload` from E12COMHEA-004 carries the target EntityId.
5. Item lots owned by an agent are queryable via relation APIs.

## Architecture Check

1. Loot transfers specific items (or all accessible items) from corpse to looter.
2. Must respect carry capacity — transfer only what the looter can carry.
3. Duration derives from item weight (heavier = longer to loot).
4. Emits public event at Place with all co-located agents as witnesses.
5. Conservation invariant: items transfer, never created or destroyed.

## What to Change

### 1. Define Loot ActionDef

- Constraints: `ActorAlive`, `ActorNotDead`, `ActorNotIncapacitated`, `ActorNotInTransit`, `ActorHasControl`
- Targets: one target Agent at actor's place
- Preconditions: `TargetAtActorPlace(0)`, `TargetDead(0)`, `TargetIsAgent(0)`
- Duration: derived from item weight (or `DurationExpr::Fixed` with reasonable default)
- Interruptibility: `FreelyInterruptible`
- Payload: `ActionPayload::Loot(LootActionPayload { target })`

### 2. Register Loot in ActionDefRegistry

### 3. Implement Loot handler

- Query items owned by the dead target
- Transfer items to the looter, subject to carry capacity
- Emit loot event (public at Place)
- If carry capacity is exceeded, transfer only what fits

### 4. Register handler in ActionHandlerRegistry

## Files to Touch

- `crates/worldwake-sim/src/action_def_registry.rs` (modify — register Loot def)
- `crates/worldwake-systems/src/combat.rs` (modify — Loot handler)
- `crates/worldwake-sim/src/action_handler_registry.rs` (modify — register handler)

## Out of Scope

- Bury action (explicitly deferred per spec)
- Discover/inspect corpse (explicitly deferred per spec)
- Corpse auto-cleanup (explicitly deferred per spec)
- Attack/Defend/Heal actions (separate tickets)
- AI deciding when to loot (E13)
- Selective looting (loot specific items) — Phase 2 transfers all accessible items

## Acceptance Criteria

### Tests That Must Pass

1. Loot action transfers items from dead agent to looter
2. Cannot loot alive agents (precondition `TargetDead`)
3. Cannot loot if dead (constraint `ActorNotDead`)
4. Cannot loot if incapacitated (constraint `ActorNotIncapacitated`)
5. Must be co-located with target (precondition `TargetAtActorPlace`)
6. Target must be an Agent (precondition `TargetIsAgent`)
7. Carry capacity is respected — partial transfer if overloaded
8. Corpse retains inventory that wasn't transferred
9. Conservation holds: sum of all items before = sum after
10. Loot event is emitted and visible at Place
11. Existing suite: `cargo test --workspace`

### Invariants

1. 9.5: Conservation — items transfer, never created or destroyed
2. Dead agent body persists — not archived by looting
3. Relation system tracks ownership correctly after transfer

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs` — Loot handler tests with conservation checks

### Commands

1. `cargo test -p worldwake-systems -- combat`
2. `cargo test --workspace && cargo clippy --workspace`
