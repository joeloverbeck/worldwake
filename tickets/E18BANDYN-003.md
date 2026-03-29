# E18BANDYN-003: Implement Raid action definition and handler

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-sim (action payload, def registry), worldwake-systems (handler)
**Deps**: E18BANDYN-001 (BanditCamp component), E12 (combat system — completed)

## Problem

Bandits need a Raid action to initiate combat against non-faction agents for the purpose of acquiring goods. The Raid action is semantically distinct from Attack (it drives bandit-specific AI goal generation) but mechanically delegates to the same wound-application logic. This ticket adds the action payload, definition, and handler.

## Assumption Reassessment (2026-03-29)

1. `ActionPayload` enum in `crates/worldwake-sim/src/action_payload.rs` currently has 18 variants. Adding `Raid(RaidActionPayload)` follows the established pattern. Each variant has a typed accessor method.
2. `ActionDomain::Combat` exists in `crates/worldwake-core/src/action_domain.rs`.
3. `CombatActionPayload` in `crates/worldwake-sim/src/action_payload.rs` already stores `{ target, weapon }`. `RaidActionPayload` needs the same fields to delegate combat resolution.
4. Combat resolution logic (wound application, death handling) lives in `crates/worldwake-systems/src/combat.rs`. The raid handler must delegate to this existing logic, not duplicate it.
5. `ActionHandlerId` is assigned sequentially by `ActionHandlerRegistry::register()`. The handler registration order must remain consistent.
6. `EventTag::Combat` and `EventTag::Transfer` exist in `crates/worldwake-core/src/event_tag.rs`.
7. `VisibilitySpec` for `SamePlace` visibility exists — used by existing combat actions.
8. `BlockedIntentMemory` with `BlockingFact::CombatTooRisky` exists for recording failed combat attempts.
9. `Interruptibility::FreelyInterruptible` exists in `crates/worldwake-sim/src/action_semantics.rs`.
10. `Constraint::ActorAlive`, `ActorHasControl`, `ActorNotInTransit` exist as actor constraints.
11. Precondition checking for `MemberOf` relation requires access to relation tables — need to verify how existing preconditions reference relations.

## Architecture Check

1. Raid delegates combat resolution to existing wound-application logic rather than duplicating it. This is cleaner than a flag on Attack because: (a) AI candidate generation needs to distinguish raid from generic combat for goal priority, (b) the action def has different preconditions (requires faction membership), (c) witnesses should be able to distinguish raid events from defensive combat for belief formation.
2. No backwards-compatibility shims. New payload variant, new action def, new handler — all additive.

## Verification Layers

1. Raid action starts with correct preconditions → focused unit test: start-gate validation
2. Combat resolution produces wounds → action trace: Committed event with wound deltas
3. Target death → authoritative world state: `DeadAt` component on target after lethal raid
4. Witness formation → event-log delta: event has `SamePlace` visibility, co-located agents are witnesses
5. Failed raid records blocked intent → focused unit test: `BlockedIntentMemory` contains `CombatTooRisky` after raider flees
6. Raid is interruptible → action trace: `Aborted` event when raider disengages

## What to Change

### 1. Add RaidActionPayload

In `crates/worldwake-sim/src/action_payload.rs`:

```rust
pub struct RaidActionPayload {
    pub target: EntityId,
    pub weapon: Option<EntityId>,
}
```

Add `Raid(RaidActionPayload)` variant to `ActionPayload` enum. Add `as_raid()` accessor.

### 2. Register Raid action definition

In `crates/worldwake-systems/src/combat.rs` (or a new `crates/worldwake-systems/src/raid.rs` if the file is large):

- Define `raid_action_def()` returning `ActionDef` with:
  - Domain: `ActionDomain::Combat`
  - Actor constraints: `ActorAlive`, `ActorHasControl`, `ActorNotInTransit`
  - Preconditions: target alive, target at actor's place, actor has `MemberOf` to a faction with `BanditCamp`, target is NOT in same faction
  - Duration: from actor's `CombatProfile`
  - Interruptibility: `FreelyInterruptible`
  - Visibility: `SamePlace`
  - Event tags: `Combat`, `Transfer`

### 3. Implement raid handler

Handler functions (`on_start`, `on_tick`, `on_commit`, `on_abort`):

- `on_commit`: Delegates to existing combat wound-application logic. On victory: emits combat event, target's possessions become lootable. On defeat: emits combat event, records `CombatTooRisky` blocked intent.
- `on_abort`: Raider disengages, no further wounds applied.

### 4. Register handler in action handler registry

Wire the handler into the action handler registry following the pattern of existing combat action registration.

## Files to Touch

- `crates/worldwake-sim/src/action_payload.rs` (modify — add `RaidActionPayload` struct, `Raid` variant, accessor)
- `crates/worldwake-sim/src/lib.rs` (modify — re-export new payload type)
- `crates/worldwake-systems/src/combat.rs` (modify — add raid action def + handler, or create new `raid.rs`)
- `crates/worldwake-systems/src/lib.rs` (modify — if new module, add `pub mod raid;`)
- Action def registry wiring file (modify — register raid def)
- Action handler registry wiring file (modify — register raid handler)

## Out of Scope

- AI candidate generation for `RaidTarget` goals (E18BANDYN-006)
- Planner ops mapping for raid (E18BANDYN-007)
- EstablishCamp action (E18BANDYN-004)
- `bandit_camp_system()` (E18BANDYN-005)
- Route threat estimation (E18BANDYN-008)
- Golden test T22 (E18BANDYN-009)
- Loot action after successful raid — already exists from E12

## Acceptance Criteria

### Tests That Must Pass

1. Raid action starts successfully when all preconditions are met (actor is faction member, target at same place, target not in same faction)
2. Raid action fails to start when actor has no faction membership
3. Raid action fails to start when target is in same faction
4. Raid commit applies wounds to target via existing combat resolution
5. Raid commit on target death sets `DeadAt` component
6. Raid abort produces no further wounds
7. Raid event has `SamePlace` visibility — co-located agents are witnesses
8. Failed raid records `CombatTooRisky` in attacker's `BlockedIntentMemory`
9. Existing suite: `cargo test -p worldwake-systems`
10. Existing suite: `cargo test -p worldwake-sim`
11. Existing suite: `cargo clippy --workspace`

### Invariants

1. Conservation: raid does not create or destroy items — only combat produces wounds, and loot is a separate action
2. Agent symmetry (FND-17): raid uses the same combat resolution as Attack
3. No teleportation: raid requires co-location (`TargetAtActorPlace`)
4. Raid is interruptible — bandits can disengage and flee

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs` (or `raid.rs`) — focused tests for raid preconditions, commit, abort
2. `crates/worldwake-sim/src/action_payload.rs` — verify `as_raid()` accessor

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace`
4. `cargo build --workspace`
