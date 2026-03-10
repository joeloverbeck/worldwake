# E09NEEMET-007: Consumption and care action definitions and handlers (Eat, Drink, Sleep, Toilet, Wash)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new ActionDefs, ActionHandlers, action registration
**Deps**: E09NEEMET-004 (consumable profiles), E09NEEMET-003 (HomeostaticNeeds), E09NEEMET-005 (metabolism system for body cost integration)

## Problem

Agents need concrete actions to satisfy their physiological needs: eating, drinking, sleeping, toileting, and washing. Each action must follow the E07 action framework, have physical (not motivational) preconditions, and produce deterministic effects through commodity data and component updates.

## Assumption Reassessment (2026-03-10)

1. `ActionDef` requires: id, name, actor_constraints, targets, preconditions, reservation_requirements, duration, interruptibility, commit_conditions, visibility, causal_event_tags, handler — confirmed.
2. `ActionHandler` requires: on_start, on_tick, on_commit, on_abort — confirmed.
3. `ActionDefRegistry` and `ActionHandlerRegistry` use sequential ID assignment — confirmed.
4. `Interruptibility` has variants: `NonInterruptible`, `InterruptibleWithPenalty`, `FreelyInterruptible` — confirmed.
5. The spec says Eat/Drink are interruptible `ByDanger, ByMajorPain`. Current `Interruptibility` enum doesn't have condition-specific variants. May need to use `InterruptibleWithPenalty` or extend the enum.
6. Sleep duration is derived from current fatigue / rest_efficiency — not a fixed `DurationExpr::Fixed`. May need `DurationExpr` extension or dynamic calculation in the handler.

## Architecture Check

1. Action defs and handlers are registered in `worldwake-sim` registries. The handlers implement logic that modifies `World` state via `WorldTxn`.
2. All five actions have physical preconditions only — "can access food" not "is hungry enough."
3. Sleep is allowed without a bed (ground sleeping). Beds improve recovery rate via sleep-site quality, which is a derived read-model.
4. Eat/Drink consume item quantities — must respect conservation invariant.

## What to Change

### 1. Eat action

**ActionDef**:
- actor_constraints: `ActorAlive`
- targets: edible `ItemLot` accessible to actor (at same place or in possession)
- preconditions: `ActorAlive`, `TargetExists`, target is food commodity
- duration: `consumption_ticks_per_unit` from consumable profile
- interruptibility: `InterruptibleWithPenalty` (approximation of ByDanger/ByMajorPain)

**ActionHandler**:
- on_commit: reduce item lot quantity by 1, decrease `hunger` by `hunger_relief_per_unit`, optionally decrease `thirst` by `thirst_relief_per_unit`, increase `bladder` by `bladder_fill_per_unit`. All from `CommodityConsumableProfile`.
- on_abort: no item consumed (action not completed)

### 2. Drink action

**ActionDef**: Similar to Eat but targets drinkable commodities (Water).

**ActionHandler**:
- on_commit: reduce item quantity, decrease `thirst`, increase `bladder` per profile.

### 3. Sleep action

**ActionDef**:
- actor_constraints: `ActorAlive`
- preconditions: `ActorAlive` (no bed required — ground sleeping allowed)
- reservation: optional bed/sleep spot entity if present
- duration: dynamic based on fatigue level and rest_efficiency
- interruptibility: `InterruptibleWithPenalty`

**ActionHandler**:
- on_tick: decrease `fatigue` by `rest_efficiency` per tick, modified by sleep-site quality
- on_commit: no additional effect (fatigue reduction happened per-tick)

### 4. Toilet action

**ActionDef**:
- preconditions: `ActorAlive`, actor at location with latrine OR in wilderness
- reservation: toilet stall if facility used
- duration: `MetabolismProfile.toilet_ticks`

**ActionHandler**:
- on_commit: decrease `bladder` substantially (to 0 or near-0), create `CommodityKind::Waste` item lot at location, wilderness toileting increases `dirtiness`

### 5. Wash action

**ActionDef**:
- preconditions: `ActorAlive`, actor can access water source / wash facility / carried water
- duration: `MetabolismProfile.wash_ticks`

**ActionHandler**:
- on_commit: decrease `dirtiness` substantially, consume wash water if from carried supply

### 6. Register all actions

Register all 5 `ActionDef` + `ActionHandler` pairs in the respective registries. Define module for action registration setup.

## Files to Touch

- `crates/worldwake-sim/src/action_def_registry.rs` (modify — register new defs, or provide setup fn)
- `crates/worldwake-sim/src/action_handler_registry.rs` (modify — register new handlers)
- `crates/worldwake-systems/src/needs_actions.rs` (new — action defs and handlers for all 5 care actions)
- `crates/worldwake-systems/src/lib.rs` (modify — export needs_actions module)
- `crates/worldwake-sim/src/action_semantics.rs` (modify — if new `Constraint` or `Precondition` variants needed for commodity-type checks)

## Out of Scope

- AI decision to eat/drink/sleep (E13)
- Sleep-site quality affecting AI choice of where to sleep (E13 planning)
- Production actions (E10)
- Trade actions (E11)
- Combat actions (E12)
- Interruptibility condition refinement (ByDanger/ByMajorPain specifics — Phase 2 can use existing variants)

## Acceptance Criteria

### Tests That Must Pass

1. Eating consumes food item lot (quantity decreases by 1) and applies commodity-defined hunger relief.
2. Eating optionally applies thirst relief and bladder fill from consumable profile.
3. Drinking consumes water and applies thirst relief + bladder fill.
4. Sleep reduces fatigue each tick by `rest_efficiency` — even without a bed.
5. Sleep with bed reservation improves recovery rate beyond ground sleeping.
6. Toilet reduces bladder and creates waste entity at location.
7. Wilderness toileting increases dirtiness.
8. Wash reduces dirtiness and consumes water when from carried supply.
9. All actions respect conservation — items consumed are actually removed.
10. All actions have physical preconditions only — no "must be hungry" gate.
11. Aborted eat/drink does not consume the item (no partial consumption on abort).
12. Existing suite: `cargo test --workspace`

### Invariants

1. Conservation: items consumed via eat/drink are properly deducted from lot quantities.
2. No motivational preconditions — action legality is purely physical (Principle 5 from spec).
3. Consumable effects are data-driven from `CommodityConsumableProfile`, not hardcoded.
4. Sleep is possible without bed — beds improve quality, not gate access (spec requirement).
5. All `Permille` values stay in valid range after action effects.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` (tests) — per-action: precondition checks, effect application, conservation, abort behavior
2. Integration tests combining action execution with metabolism system tick

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-sim`
3. `cargo test --workspace`
4. `cargo clippy --workspace`
