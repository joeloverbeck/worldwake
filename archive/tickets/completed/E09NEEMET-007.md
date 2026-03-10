# E09NEEMET-007: Consumption and care action definitions and handlers

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — action semantic extensions, new ActionDefs, ActionHandlers, registration helpers
**Deps**: E09NEEMET-004 (consumable profiles), E09NEEMET-003 (HomeostaticNeeds), E09NEEMET-005 (metabolism system for body cost integration)

## Problem

E09 requires concrete physiology actions, but the current ticket overstates what the codebase already supports. The action framework can execute fixed-duration actions against coarse target specs, and the needs system already applies basal physiology, deprivation wounds, and bladder accidents. What is missing is the clean glue between data-driven physiology state and the action layer: consumable-aware targeting, control-aware access checks, and profile-driven duration resolution where durations legitimately come from actor or commodity data.

The current codebase does **not** yet model beds, latrine facilities, wash basins, shelter quality, or a reusable sleep-site quality read-model. Forcing those concepts into this ticket would add speculative architecture that E10/E13 are better positioned to own.

## Spec References

- `specs/E09-needs-metabolism.md`
- E09 Deliverables: Consumption & Care Actions, Commodity Consumable Profile Extensions, Metabolism Profile, Sleep-Site Quality

## Assumption Reassessment (2026-03-10)

1. `ActionDef` already includes `body_cost_per_tick`; the original ticket omitted that existing field.
2. `ActionHandler` is `on_start`, `on_tick`, `on_commit`, `on_abort` — confirmed.
3. `ActionDefRegistry` and `ActionHandlerRegistry` use sequential ID assignment — confirmed.
4. `Interruptibility` only has `NonInterruptible`, `InterruptibleWithPenalty`, and `FreelyInterruptible` — confirmed. The spec’s `ByDanger` / `ByMajorPain` policy is not representable yet and should remain approximated.
5. `DurationExpr` currently supports only `Fixed(NonZeroU32)` — not enough for commodity-driven eat/drink durations or per-agent toilet/wash durations.
6. `Precondition` and `TargetSpec` are too coarse for this ticket as written. They cannot currently express:
   - actor control over a chosen target lot
   - consumable-effect filtering for edible vs drinkable lots
   - profile-driven duration selection
7. There is **no existing facility/affordance schema** for beds, cots, bunks, latrines, toilet stalls, wash basins, or shelter quality in `worldwake-core`.
8. There is **no existing sleep-site quality read-model**. Bed bonuses and reservation-driven sleep quality are not implementable cleanly in this ticket without inventing unsupported world state.
9. `ActionState` is currently only `Empty`; there is no richer per-instance local state model to lean on for bespoke care-action bookkeeping.
10. The needs system already covers sustained deprivation wounds and involuntary bladder relief; this ticket should add explicit voluntary care actions, not duplicate those consequences.

## Architecture Check

### What Is Beneficial To Add Now

1. Add **generic, reusable action-semantic support** for profile-driven durations:
   - duration from target consumable profile
   - duration from actor metabolism profile
2. Add **control-aware targeting** so care actions can require the actor to actually control the specific item lot they act on.
3. Keep action legality **physical only**: access, control, co-location, commodity data.
4. Keep all physiology effects **data-driven** from `CommodityConsumableProfile` and `MetabolismProfile`.

### What Should Not Be Forced Into This Ticket

1. Bed/shelter bonuses for sleep. There is no underlying facility or sleep-quality architecture yet.
2. Toilet-stall reservations or wash-facility reservations. Those affordances do not exist yet.
3. Facility-aware target selection for E10 structures that have not been implemented.
4. Fake backward-compatible wrappers or alias paths in the action framework.

## Revised Scope

Implement the clean Phase 2 slice that the current architecture can support without speculative facility modeling:

1. `Eat`
2. `Drink`
3. `Sleep` as repeatable ground-sleep / rest action with per-tick fatigue reduction only
4. `Toilet` as self-care action without facility reservations
5. `Wash` using actor-controlled water lots, not world water-source facilities

The ticket explicitly **defers** bed-quality bonuses and facility-aware reservations to later tickets once those world affordances exist concretely.

## What To Change

### 1. Extend action semantics for clean data-driven care actions

Add the smallest reusable semantic surface needed for this ticket:

- control-aware precondition for acting on a bound target the actor actually controls
- consumable-aware precondition for target lots with the required relief effect
- duration expressions that resolve from:
  - target consumable profile
  - actor metabolism profile

These extensions should stay generic enough to support future production/trade/combat actions that also need profile-driven timing or control-aware item interaction.

### 2. Add `needs_actions` module in `worldwake-systems`

Create a dedicated module for E09 care-action defs, handlers, and registration helpers.

Expected public setup helpers:

- register E09 needs action defs into an `ActionDefRegistry`
- register E09 needs action handlers into an `ActionHandlerRegistry`

Do **not** hardwire these actions into registry constructors. Keep registration explicit.

### 3. Eat action

**ActionDef**

- actor constraints: `ActorAlive`
- target: item lot at actor effective place
- preconditions:
  - actor alive
  - target exists
  - target is an item lot
  - actor can control target
  - target has a consumable profile with non-zero hunger relief
- duration: resolve from target consumable profile
- interruptibility: `InterruptibleWithPenalty`
- body cost per tick: zero

**ActionHandler**

- on_commit:
  - consume exactly 1 unit from the target lot
  - reduce `hunger` by commodity profile relief
  - reduce `thirst` by commodity profile relief if present
  - increase `bladder` by commodity profile fill
- on_abort:
  - no consumption
- must preserve conservation when consuming from lots with quantity `1` or `>1`

### 4. Drink action

**ActionDef**

- same overall shape as `Eat`
- precondition requires non-zero thirst relief

**ActionHandler**

- on_commit:
  - consume exactly 1 unit from the target lot
  - reduce `thirst`
  - increase `bladder`
  - apply any commodity-defined hunger relief if the drink provides it

### 5. Sleep action

**ActionDef**

- actor constraints: `ActorAlive`
- no external target required
- preconditions: `ActorAlive`
- duration: fixed 1 tick
- interruptibility: `InterruptibleWithPenalty`

**ActionHandler**

- on_tick:
  - reduce `fatigue` by `MetabolismProfile.rest_efficiency`
- on_commit:
  - no extra effect

**Note**

This is intentionally a repeatable rest action, not a bed-aware long-running sleep-site action. That keeps the implementation honest with current architecture and still satisfies the core E09 requirement that sleep without a bed is physically possible.

### 6. Toilet action

**ActionDef**

- actor constraints: `ActorAlive`
- no target required
- preconditions: `ActorAlive`
- duration: resolve from actor metabolism profile (`toilet_ticks`)
- interruptibility: `InterruptibleWithPenalty`

**ActionHandler**

- on_commit:
  - reduce `bladder` substantially
  - create `CommodityKind::Waste` at actor location

**Note**

This ticket does not add latrine/toilet-stall reservations. Voluntary toileting exists first; facility-aware quality or cleanliness effects come later once those facilities exist.

### 7. Wash action

**ActionDef**

- actor constraints: `ActorAlive`
- target: item lot at actor effective place
- preconditions:
  - actor alive
  - target exists
  - target is an item lot
  - actor can control target
  - target commodity is `Water`
- duration: resolve from actor metabolism profile (`wash_ticks`)
- interruptibility: `InterruptibleWithPenalty`

**ActionHandler**

- on_commit:
  - reduce `dirtiness` substantially
  - consume exactly 1 unit of water from the controlled target lot

### 8. Registration helpers

Provide explicit registration helpers in `worldwake-systems` so tests and future runtime composition can register all five E09 care actions without duplicating setup logic.

## Files To Touch

- `crates/worldwake-sim/src/action_semantics.rs`
- `crates/worldwake-sim/src/action_validation.rs`
- `crates/worldwake-sim/src/affordance_query.rs`
- `crates/worldwake-sim/src/start_gate.rs`
- `crates/worldwake-sim/src/omniscient_belief_view.rs`
- `crates/worldwake-systems/src/needs_actions.rs` (new)
- `crates/worldwake-systems/src/lib.rs`

## Out Of Scope

- Bed / cot / bunk reservation support
- Sleep-site quality bonuses
- Shelter-aware sleep modifiers
- Toilet-stall or wash-facility reservations
- World water-source affordances
- AI choice logic for when to eat/drink/sleep/wash/toilet (E13)
- E10/E11/E12 action defs
- Interruptibility refinement to spec-grade condition-specific policies

## Acceptance Criteria

### Tests That Must Pass

1. Eating consumes exactly 1 food unit and applies commodity-defined hunger/thirst/bladder effects.
2. Drinking consumes exactly 1 drink unit and applies commodity-defined effects.
3. Aborted eat/drink does not consume the target lot.
4. Sleep reduces fatigue by `rest_efficiency` with no bed required.
5. Toilet reduces bladder and creates waste at actor location.
6. Wash reduces dirtiness and consumes 1 unit of controlled water.
7. All targeted care actions require actor control over the specific target lot.
8. Profile-driven durations resolve correctly for:
   - consumable-driven eat/drink
   - metabolism-driven toilet/wash
9. All actions keep `HomeostaticNeeds` in valid `Permille` range.
10. Existing relevant suites and `cargo test --workspace` pass.

### Invariants

1. Conservation is preserved for all item-consuming actions.
2. No motivational preconditions are introduced.
3. Consumable effects remain data-driven from `CommodityConsumableProfile`.
4. Sleep remains possible without any bed/facility schema.
5. No speculative facility or sleep-quality model is introduced just to satisfy this ticket.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs`
   - eat/drink/wash/toilet/sleep handler tests
   - abort behavior tests
   - conservation tests
   - control/precondition tests
2. `crates/worldwake-sim/src/action_semantics.rs`
   - new duration/precondition variants roundtrip and resolve correctly
3. `crates/worldwake-sim/src/start_gate.rs`
   - profile-driven durations resolve correctly at action start
4. `crates/worldwake-sim/src/affordance_query.rs`
   - affordance filtering respects new control/consumable preconditions

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-sim`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-10
- What actually changed:
  - Added reusable action semantics for control-aware targets and profile-driven durations.
  - Added E09 needs action registration plus concrete `Eat`, `Drink`, `Sleep`, `Toilet`, and `Wash` handlers in `crates/worldwake-systems/src/needs_actions.rs`.
  - Extended control checks so items inside actor-controlled containers are valid action targets.
  - Added end-to-end tests for the new care actions and supporting semantic validation tests.
- Deviations from original plan:
  - Did not invent bed/shelter bonuses, toilet-stall reservations, wash facilities, or a sleep-site quality model because those world affordances do not exist yet.
  - `Sleep` landed as a repeatable one-tick rest action instead of a bed-aware long-running sleep-site action.
  - Voluntary `Toilet` now resets bladder and creates waste, but does not fabricate a dirtiness formula without a concrete hygiene model.
- Verification results:
  - `cargo test -p worldwake-sim` passed.
  - `cargo test -p worldwake-systems` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
