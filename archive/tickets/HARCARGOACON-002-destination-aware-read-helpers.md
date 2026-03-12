# HARCARGOACON-002: Add destination-aware cargo read helpers

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` (`BeliefView`, `OmniscientBeliefView`), `worldwake-ai` (`PlanningState`, test doubles)
**Deps**: HARCARGOACON-001 cargo-goal redesign work already partially landed in code; this ticket now covers only the missing read-model helpers it depends on

## Problem

Cargo continuity hardening now relies on destination-aware cargo queries, but the shared read surface is still global-only:

- `commodity_quantity(holder, kind)` sums controlled stock across all places
- there is no shared helper for "how much of commodity X does actor A control at place P?"
- there is no authoritative helper for "which concrete local lots of commodity X can actor A control at place P?"
- `PlanningState` has no ref-safe equivalent for enumerating local controlled lots when hypotheticals exist

That leaves the architecture in an awkward middle state: cargo goals are already commodity-and-destination based, but the read model still cannot answer destination-aware logistics questions cleanly.

## Assumption Reassessment (2026-03-12)

1. `GoalKind::MoveCargo` is **already** `MoveCargo { commodity, destination }` in [`crates/worldwake-core/src/goal.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs). The original ticket assumption that lot-based goal identity still existed was wrong.
2. `search.rs` still treats `MoveCargo` as unsupported, and `candidate_generation.rs` still excludes it. Those gaps are real, but they belong to later cargo-continuity tickets, not this helper ticket.
3. `BeliefView` still exposes only global `commodity_quantity(holder, kind)` and has no destination-aware cargo helper surface.
4. `PlanningState` can model hypothetical cargo lots, so a helper returning `Vec<EntityId>` is not sufficient for all planning-time cargo enumeration. Planning needs a `PlanningEntityRef`-aware helper in addition to any authoritative `BeliefView` helper.
5. Test-only `BeliefView` impl count was overstated and stale. The current affected test doubles are in:
   - `crates/worldwake-ai/src/pressure.rs`
   - `crates/worldwake-ai/src/failure_handling.rs`
   - `crates/worldwake-ai/src/goal_model.rs`
   - `crates/worldwake-ai/src/planning_state.rs`
   - `crates/worldwake-ai/src/planning_snapshot.rs`
   - `crates/worldwake-ai/src/plan_revalidation.rs`
   - `crates/worldwake-ai/src/planner_ops.rs`
   - `crates/worldwake-ai/src/candidate_generation.rs`
   - `crates/worldwake-ai/src/search.rs`
   - `crates/worldwake-ai/src/ranking.rs`
   - `crates/worldwake-sim/src/trade_valuation.rs`

## Architecture Check

1. Adding explicit destination-aware helpers is better than overloading `commodity_quantity` with optional place parameters. It keeps the global and place-local queries distinct and honest.
2. For authoritative views, returning `Vec<EntityId>` for local controlled lots is acceptable because only authoritative entities exist there.
3. For `PlanningState`, forcing hypothetical cargo lots into `EntityId` would be the wrong architecture. The clean design is:
   - add destination-aware quantity and authoritative lot-list helpers to `BeliefView`
   - add a planning-only `local_controlled_lot_refs_for(...) -> Vec<PlanningEntityRef>` helper on `PlanningState`
4. The helpers should be concrete-state based: filter by effective place and control, not by abstract cargo scores or aliases.
5. Deterministic ordering must remain explicit everywhere through sorted `Vec`s / `BTree*` traversal.

## What to Change

### 1. `BeliefView` trait

Add:

```rust
fn controlled_commodity_quantity_at_place(
    &self,
    agent: EntityId,
    place: EntityId,
    commodity: CommodityKind,
) -> Quantity;

fn local_controlled_lots_for(
    &self,
    agent: EntityId,
    place: EntityId,
    commodity: CommodityKind,
) -> Vec<EntityId>;
```

Semantics:

- only lots whose effective place is `place`
- only lots the actor can concretely control now
- deterministic ordering

### 2. `OmniscientBeliefView`

Implement both methods against authoritative world state.

Recommended semantics:

- `local_controlled_lots_for`: enumerate effective-local item lots of the requested commodity and retain only those `can_control(agent, lot)`
- `controlled_commodity_quantity_at_place`: sum the quantities of the returned lots

This is better than filtering only `direct_possessions(agent)`, because destination-local stock and candidate seeding both need to see concretely controlled local lots, not just directly possessed ones.

### 3. `PlanningState`

Implement:

- the `BeliefView` quantity helper: `controlled_commodity_quantity_at_place(...) -> Quantity`
- an inherent planning helper:

```rust
pub fn local_controlled_lot_refs_for(
    &self,
    agent: PlanningEntityRef,
    place: EntityId,
    commodity: CommodityKind,
) -> Vec<PlanningEntityRef>;
```

This helper must include authoritative and hypothetical lots, respect removals/overrides, and return deterministic ordering.

Add a small authoritative wrapper only if still useful:

```rust
fn local_controlled_lots_for(&self, agent: EntityId, place: EntityId, commodity: CommodityKind) -> Vec<EntityId>
```

But do **not** pretend hypothetical lots are authoritative IDs.

### 4. Test doubles

Update every test-only `BeliefView` impl to satisfy the new trait surface.

- stubs that do not exercise cargo locality may return `Quantity(0)` / `Vec::new()`
- stubs used by helper tests should model place, control, and quantity consistently

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs`
- `crates/worldwake-sim/src/omniscient_belief_view.rs`
- `crates/worldwake-ai/src/planning_state.rs`
- test-double files listed above as needed for trait compilation

## Out of Scope

- Emitting `MoveCargo` in candidate generation
- Removing `MoveCargo` from unsupported search goals
- Adding `restock_gap_at_destination`
- Changing `GoalKind`, `GoalKey`, or cargo-goal identity again
- Runtime dirtiness / plan-retention behavior

Those belong to later cargo hardening tickets. This ticket exists to provide the clean read-model surface those later changes should build on.

## Acceptance Criteria

### Tests That Must Pass

1. New unit tests in [`crates/worldwake-sim/src/omniscient_belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/omniscient_belief_view.rs):
   - destination-aware quantity counts only local controlled stock
   - local controlled lot enumeration is deterministic and excludes uncontrolled / wrong-place / wrong-commodity lots
2. New unit tests in [`crates/worldwake-ai/src/planning_state.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs):
   - quantity helper respects place filtering
   - `local_controlled_lot_refs_for` includes hypothetical lots and honors overrides/removals deterministically
3. Narrow crate tests for the touched areas pass
4. `cargo test --workspace` passes
5. `cargo clippy --workspace` passes

### Invariants

1. Place-local quantity is zero when no matching controlled lots exist at that place.
2. Local lot enumeration is deterministic.
3. Quantity equals the sum of matching lot quantities for the same place/commodity/control predicate.
4. `PlanningState` does not erase hypothetical cargo identity by forcing it through `EntityId`.
5. No compatibility aliasing or dual semantics are introduced.

## Test Plan

### New/Modified Tests

1. [`crates/worldwake-sim/src/omniscient_belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/omniscient_belief_view.rs)
   - `controlled_commodity_quantity_at_place_filters_by_place_and_control`
   - `local_controlled_lots_for_returns_deterministic_local_matches`
2. [`crates/worldwake-ai/src/planning_state.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs)
   - `controlled_commodity_quantity_at_place_counts_local_authoritative_and_hypothetical_stock`
   - `local_controlled_lot_refs_for_tracks_hypotheticals_and_removals`

### Commands

1. `cargo test -p worldwake-sim omniscient_belief_view`
2. `cargo test -p worldwake-ai planning_state`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-12
- What actually changed:
  - added `controlled_commodity_quantity_at_place(...)` and `local_controlled_lots_for(...)` to `BeliefView`
  - implemented both helpers in `OmniscientBeliefView`
  - added `PlanningState::local_controlled_lot_refs_for(...)` so hypothetical lots stay representable in planning
  - implemented `PlanningState` destination-aware quantity support plus an authoritative-ID wrapper for `BeliefView`
  - updated all affected test doubles to satisfy the expanded trait surface
  - added new helper-focused tests in `omniscient_belief_view.rs` and `planning_state.rs`
- Deviations from original plan:
  - the ticket was corrected before implementation because `GoalKind::MoveCargo` had already migrated to `{ commodity, destination }`
  - `PlanningState` did not mirror lot enumeration through `Vec<EntityId>` alone; it now exposes a ref-aware helper for hypotheticals, which is the cleaner architecture
  - no candidate-generation, search, or restock-gap behavior was changed here; those remain for later cargo tickets
- Verification results:
  - `cargo test -p worldwake-sim omniscient_belief_view` passed
  - `cargo test -p worldwake-ai planning_state` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
