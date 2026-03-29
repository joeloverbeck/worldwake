# E18BANDYN-004: Implement EstablishCamp action definition and handler

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-sim (action payload, def registry), worldwake-systems (handler)
**Deps**: E18BANDYN-001 (BanditCamp/BanditCampProfile components)

## Problem

Survivors regrouping at a rally point need the `EstablishCamp` action to create a new bandit camp. This action has duration, material cost, minimum member count, and is interruptible. It attaches the `BanditCamp` component to the current Place, creates a supply container, and transfers the actor's supplies into it.

## Assumption Reassessment (2026-03-29)

1. `BanditCamp` and `BanditCampProfile` components are introduced by E18BANDYN-001. This ticket depends on those types existing.
2. `EntityKind::Container` exists. Container creation follows the pattern in `crates/worldwake-core/src/items.rs` — allocate entity, attach `Container` component, set `located_in` relation.
3. `ActionDomain::Generic` exists — appropriate for camp establishment since it's not combat, trade, or production.
4. `Interruptibility::InterruptibleWithPenalty` exists in `crates/worldwake-sim/src/action_semantics.rs`.
5. `PlaceTag::Camp` and `PlaceTag::Forest` exist in `crates/worldwake-core/src/topology.rs` — preconditions check the current place has one of these tags.
6. `BanditCampProfile.establishment_duration_ticks` provides the duration — no magic numbers.
7. `BanditCampProfile.min_regroup_count` provides the minimum member count.
8. `members_of(faction_id)` query on `RelationTables` returns all faction members. Filtering to "living members at same place" requires checking `DeadAt` absence and `located_in` match.
9. `EventTag::WorldMutation` exists for the camp-establishment event.
10. Supply transfer uses existing `PossessedBy`/`ContainedBy` relation mutations — no new transfer logic needed.
11. Commit condition: "if the place already has a `BanditCamp` component, reuse existing camp rather than creating a duplicate" — this is a commit-time check, not a precondition (allows reoccupying abandoned camps).

## Architecture Check

1. EstablishCamp is a duration-bearing, interruptible action that creates authoritative world state through the standard action framework. This is cleaner than a special-case system because: (a) it has preconditions, duration, cost, and occupancy (FND-8), (b) it can be interrupted by attacks (FND-9), (c) progress is lost on interruption (granular outcome). Alternative: a "system auto-creates camp when members gather" would violate FND-8 (no preconditions/duration) and FND-1 (authored trigger).
2. No backwards-compatibility shims. New payload variant, new action def, new handler.

## Verification Layers

1. Precondition enforcement (place tag, faction membership, member count, supplies) → focused unit test: start-gate validation
2. Camp creation on commit → authoritative world state: Place has `BanditCamp` component after commit
3. Container creation → authoritative world state: new Container entity at place with supplies
4. Supply transfer → authoritative world state: actor's food items moved to camp container
5. Interruption loses progress → action trace: `Aborted` event, no `BanditCamp` component created
6. Reoccupation of existing camp → focused unit test: commit on place with existing `BanditCamp` reuses it
7. Conservation → `verify_live_lot_conservation` passes: supplies transferred, not created/destroyed

## What to Change

### 1. Add EstablishCampActionPayload

In `crates/worldwake-sim/src/action_payload.rs`:

```rust
pub struct EstablishCampActionPayload {
    pub faction: EntityId,
}
```

Add `EstablishCamp(EstablishCampActionPayload)` variant. Add `as_establish_camp()` accessor.

### 2. Register EstablishCamp action definition

In a new file `crates/worldwake-systems/src/bandit_camp_actions.rs`:

- Define `establish_camp_action_def()` returning `ActionDef` with:
  - Domain: `ActionDomain::Generic`
  - Actor constraints: `ActorAlive`, `ActorHasControl`, `ActorNotInTransit`
  - Preconditions: actor's place has `PlaceTag::Camp` or `PlaceTag::Forest`; actor has `MemberOf` to a bandit faction; minimum living faction members at same place; actor possesses food commodity
  - Duration: `BanditCampProfile.establishment_duration_ticks`
  - Body cost: moderate (hunger, fatigue increase)
  - Interruptibility: `InterruptibleWithPenalty`
  - Commit conditions: same place requirements still met; minimum members still present
  - Visibility: `SamePlace`
  - Event tags: `WorldMutation`

### 3. Implement handler

- `on_commit`:
  1. Check if place already has `BanditCamp` — if so, reuse (reoccupation)
  2. Otherwise: attach `BanditCamp { faction, supplies }` to current Place
  3. Create Container entity at current place for camp supplies
  4. Transfer actor's carried food supplies into camp container
  5. Emit camp-establishment event
- `on_abort`: No camp created. Actor retains carried supplies. Progress lost.

### 4. Register handler and action def

Wire into action handler registry and action def registry.

## Files to Touch

- `crates/worldwake-sim/src/action_payload.rs` (modify — add `EstablishCampActionPayload`, variant, accessor)
- `crates/worldwake-sim/src/lib.rs` (modify — re-export new payload type)
- `crates/worldwake-systems/src/bandit_camp_actions.rs` (new — action def + handler)
- `crates/worldwake-systems/src/lib.rs` (modify — add `pub mod bandit_camp_actions;`)
- Action def registry wiring file (modify — register establish_camp def)
- Action handler registry wiring file (modify — register establish_camp handler)

## Out of Scope

- Raid action (E18BANDYN-003)
- `bandit_camp_system()` abandonment detection (E18BANDYN-005)
- AI candidate generation for `RegroupWithFaction` that leads to this action (E18BANDYN-006)
- Planner search integration (E18BANDYN-007)
- `BanditCamp` and `BanditCampProfile` component definitions (E18BANDYN-001)
- Route threat estimation (E18BANDYN-008)
- Golden test T22 (E18BANDYN-009)

## Acceptance Criteria

### Tests That Must Pass

1. EstablishCamp starts when all preconditions met (correct place tag, faction member, enough members, has supplies)
2. EstablishCamp fails to start at a place without `PlaceTag::Camp` or `PlaceTag::Forest`
3. EstablishCamp fails to start when below `min_regroup_count` living members at place
4. EstablishCamp fails to start when actor has no food commodity
5. EstablishCamp commit creates `BanditCamp` component on the Place entity
6. EstablishCamp commit creates a Container entity with supplies
7. EstablishCamp commit transfers actor's food to camp container (conservation holds)
8. EstablishCamp on a place with existing `BanditCamp` reuses the camp (no duplicate)
9. EstablishCamp abort does not create camp — actor retains supplies
10. Existing suite: `cargo test -p worldwake-systems`
11. Existing suite: `cargo clippy --workspace`

### Invariants

1. Conservation: supplies are transferred, not created or destroyed. `verify_live_lot_conservation` passes.
2. FND-8: action has preconditions, duration, cost, and is interruptible
3. FND-9: interruption produces partial outcome (progress lost, no camp)
4. Only one `BanditCamp` component per place — commit reuses existing if present
5. No magic numbers — duration and member count from `BanditCampProfile`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/bandit_camp_actions.rs` — focused tests for preconditions, commit, abort, reoccupation
2. Conservation test — verify `verify_live_lot_conservation` after supply transfer

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace`
4. `cargo build --workspace`
