# E20COMBEH-004: Wilderness relief action definition and handler

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-systems (needs_actions.rs), worldwake-core (topology.rs constant)
**Deps**: E20COMBEH-001 (MetabolismProfile.wilderness_relief_dirtiness_penalty, EventTag::WildernessRelief)

## Problem

Agents with critical bladder currently have only one relief option: the `toilet` action at `PlaceTag::Latrine` locations. If no latrine is reachable, the agent has no plan and suffers a deprivation accident. A `relieve_wilderness` action at outdoor places provides a fallback with a dirtiness penalty, enabling the planner to discover relief paths naturally through affordance evaluation.

## Assumption Reassessment (2026-03-30)

1. **`toilet` action** (`crates/worldwake-systems/src/needs_actions.rs:68-76`): Registered with `Precondition::ActorAlive`, `DurationExpr::ActorMetabolism { kind: MetabolismDurationKind::Toilet }`. Handler: `commit_toilet` (lines 327-354) sets bladder to 0, creates `CommodityKind::Waste` at place. No place constraint (currently toilet can be used anywhere — may be an E09 issue, but out of scope here).
2. **`CommodityKind::Waste`** (`crates/worldwake-core/src/items.rs:20`): Exists. Used by `commit_toilet`.
3. **PlaceTag outdoor variants** (`crates/worldwake-core/src/topology.rs:11-26`): `Forest`, `Trail`, `Field`, `Farm`, `Road` are all present. No `Outdoor` meta-tag exists, which is intentional per the spec.
4. **Action registration pattern** (`crates/worldwake-systems/src/needs_actions.rs`): Actions registered via `register_def()` helper. Handlers built via `ActionHandler::new(start, tick, commit, abort)`.
5. **Precondition variants**: `ActorAlive` exists. Need to verify `ActorNotIncapacitated` and `ActorNotInTransit` exist. The spec lists these as actor constraints.
6. **VisibilitySpec::SamePlace**: Confirmed available in `worldwake-core`. Used by other actions.
7. **Interruptibility::InterruptibleWithPenalty**: Confirmed available.
8. **`OUTDOOR_RELIEF_TAGS` constant**: Does not exist yet. Should be defined in `worldwake-core/src/topology.rs` as a public constant for reuse by both this action's place constraint and the planner (E20COMBEH-005).

## Architecture Check

1. The `relieve_wilderness` action follows the exact same pattern as `toilet` — same handler structure, same commit logic (bladder reset, waste creation), with additional dirtiness penalty and place constraint. This is the cleanest approach: one new action def + one commit handler, reusing all existing infrastructure.
2. Defining `OUTDOOR_RELIEF_TAGS` as a constant in `worldwake-core/src/topology.rs` ensures the action constraint and planner (E20COMBEH-005) use the same tag set. No duplication.
3. No backward-compatibility shims. The new action is purely additive.

## Verification Layers

1. Place constraint (outdoor tags accepted, indoor tags rejected) → focused unit tests
2. Commit effects (bladder → 0, dirtiness += penalty, waste created) → focused unit test
3. EventTag tagging → focused unit test (commit event has WildernessRelief tag)
4. Visibility → focused unit test (SamePlace on commit event)
5. Single-layer ticket: worldwake-systems action handler only; perception wiring is existing E14 infrastructure.

## What to Change

### 1. Define `OUTDOOR_RELIEF_TAGS` constant

In `crates/worldwake-core/src/topology.rs`, add:

```rust
/// Place tags that represent outdoor locations where wilderness relief is available.
pub const OUTDOOR_RELIEF_TAGS: &[PlaceTag] = &[
    PlaceTag::Forest,
    PlaceTag::Trail,
    PlaceTag::Field,
    PlaceTag::Farm,
    PlaceTag::Road,
];
```

### 2. Register `relieve_wilderness` action

In `crates/worldwake-systems/src/needs_actions.rs`, register a new action:

- **Name**: `"relieve_wilderness"`
- **Domain**: `ActionDomain::Needs`
- **Actor constraints**: `ActorAlive`, `ActorNotIncapacitated`, `ActorNotInTransit`
- **Place constraint**: Actor at place with any tag in `OUTDOOR_RELIEF_TAGS`
- **Duration**: `DurationExpr::ActorMetabolism { kind: MetabolismDurationKind::Toilet }` (same as toilet — spec says so)
- **Body cost**: `BodyCostPerTick::zero()`
- **Interruptibility**: `InterruptibleWithPenalty`
- **Visibility**: `VisibilitySpec::SamePlace`
- **Causal event tags**: `EventTag::WildernessRelief` (+ standard `ActionCommitted`)
- **Handler**: `start_noop`, `tick_continue`, `commit_relieve_wilderness`, `abort_noop`

### 3. Implement `commit_relieve_wilderness`

```rust
fn commit_relieve_wilderness(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let actor = instance.actor;
    let mut needs = txn.homeostatic_needs(actor)?;
    let profile = txn.metabolism_profile(actor)?;

    needs.bladder = Permille(0);
    needs.dirtiness = needs.dirtiness.saturating_add(profile.wilderness_relief_dirtiness_penalty);
    txn.set_homeostatic_needs(actor, needs)?;

    let place = txn.current_place(actor)?;
    let waste = txn.create_item_lot(CommodityKind::Waste, Quantity(1))?;
    txn.set_ground_location(waste, place)?;

    Ok(CommitOutcome::default())
}
```

### 4. Implement place constraint check

The action's precondition must verify the actor is at a place with at least one outdoor tag. This may be a new `Precondition` variant (e.g., `ActorAtPlaceWithAnyTag(Vec<PlaceTag>)`) or a custom constraint check in the action's start validation. Follow the existing precondition pattern.

## Files to Touch

- `crates/worldwake-core/src/topology.rs` (modify — add `OUTDOOR_RELIEF_TAGS` constant)
- `crates/worldwake-systems/src/needs_actions.rs` (modify — register action, add commit handler)
- `crates/worldwake-sim/src/action_semantics.rs` (modify — if new `Precondition` variant needed for place tag check)
- `crates/worldwake-sim/src/action_validation.rs` (modify — if new precondition validation needed)

## Out of Scope

- Planner integration for the new action (E20COMBEH-005)
- Travel body cost changes (E20COMBEH-002, E20COMBEH-003)
- Golden E2E tests (E20COMBEH-006 through E20COMBEH-008)
- Adding place constraints to the existing `toilet` action (separate concern)
- Perception system changes (E14 handles SamePlace visibility already)
- New social or reputation systems (spec explicitly says none)

## Acceptance Criteria

### Tests That Must Pass

1. `relieve_wilderness` constraint accepts places tagged `Forest`, `Trail`, `Field`, `Farm`, `Road`
2. `relieve_wilderness` constraint rejects places tagged `Inn`, `Hall`, `Barracks`, `Store`, `Latrine`
3. `relieve_wilderness` commit: bladder → `Permille(0)`
4. `relieve_wilderness` commit: dirtiness increases by `wilderness_relief_dirtiness_penalty`
5. `relieve_wilderness` commit: `CommodityKind::Waste` entity created at actor's place
6. `relieve_wilderness` has `VisibilitySpec::SamePlace`
7. `relieve_wilderness` has `EventTag::WildernessRelief`
8. Existing suite: `cargo test -p worldwake-systems`
9. Existing suite: `cargo test --workspace`

### Invariants

1. Waste conservation: every `relieve_wilderness` commit creates exactly one Waste unit
2. Bladder always goes to exactly `Permille(0)` — no partial relief
3. Dirtiness penalty is additive via `saturating_add` — never overflows
4. Agent symmetry: action is available to any agent with `HomeostaticNeeds` at an outdoor place

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` — `relieve_wilderness_accepts_outdoor_places` — constraint check for each outdoor tag
2. `crates/worldwake-systems/src/needs_actions.rs` — `relieve_wilderness_rejects_indoor_places` — constraint check for indoor tags
3. `crates/worldwake-systems/src/needs_actions.rs` — `relieve_wilderness_commit_effects` — bladder, dirtiness, waste creation
4. `crates/worldwake-systems/src/needs_actions.rs` — `relieve_wilderness_visibility_is_same_place` — action def check
5. `crates/worldwake-core/src/topology.rs` — `outdoor_relief_tags_contains_expected` — constant validation

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-core`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

**Completion date**: 2026-03-30

**What changed**:
- Added `PlaceTagSet` bitmask type (`u16`) and `OUTDOOR_RELIEF_TAGS` constant in `worldwake-core/src/topology.rs`
- Added `Constraint::ActorAtPlaceWithAnyTag(PlaceTagSet)` variant in `worldwake-sim/src/action_semantics.rs` with validation in `action_validation.rs` and `affordance_query.rs`
- Added `World::place_has_any_tag_in()` and `RuntimeBeliefView::place_has_any_tag_in()` default method
- Registered `relieve_wilderness` action def and `commit_relieve_wilderness` handler in `worldwake-systems/src/needs_actions.rs`
- Mapped `"relieve_wilderness"` to `PlannerOpKind::Relieve` in `worldwake-ai/src/planner_ops.rs`
- Added 6 focused unit tests + 1 constant validation test

**Deviations from original plan**:
- Ticket suggested a `Vec<PlaceTag>` or new `Precondition` variant for the place constraint. Instead used a `PlaceTagSet` bitmask (`u16`) because `Constraint` derives `Copy`, making `Vec` impossible. The bitmask is more efficient and `Copy`-compatible.
- Registered the action directly instead of through the `register_def` helper, because the helper hardcodes `VisibilitySpec::ParticipantsOnly` and `EventTag::WorldMutation`, while this action needs `SamePlace` and `WildernessRelief`.
- Also added the planner ops mapping (`relieve_wilderness` → `PlannerOpKind::Relieve`) which was listed as out-of-scope (E20COMBEH-005) but was required by the AI crate's exhaustiveness test.

**Verification**: `cargo test --workspace` all pass, `cargo clippy --workspace` clean.
