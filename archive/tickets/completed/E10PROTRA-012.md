# E10PROTRA-012: E10 integration tests — conservation, no-teleportation, no-infinite-harvest

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None expected — tests only unless the integration harness exposes a missing public seam
**Deps**: E10PROTRA-007 through E10PROTRA-011 (all action tickets must be complete)

## Problem

The E10 spec defines several cross-cutting invariants that no single unit test can verify. These integration tests validate that the production and transport systems work together correctly and that no combination of actions violates the foundational invariants.

## Assumption Reassessment (2026-03-10)

1. All required E10 production, transport, and travel components/actions already exist after tickets 001-011 — confirmed.
2. The lot-only conservation helper in `worldwake-core/src/conservation.rs` only sums live `ItemLot` quantities. It does **not** count `ResourceSource.available_quantity`, so harvest/regeneration conservation needs an authoritative material-accounting helper rather than a lot-only one.
3. The replay system exists in `worldwake-sim`, but replay is already covered generically there. This ticket should only add E10-specific replay assertions if an integration gap remains after scheduler-driven coverage is added.
4. `build_prototype_world()` exists, but custom deterministic topologies are a better fit for E10 integration tests because they make route structure, workstation placement, and stock accounting explicit.

## Architecture Check

1. There is currently no E10 integration test file under `crates/worldwake-systems/tests/`; only E09 scheduler integration coverage exists there.
2. Most E10 action-level invariants are already covered in focused unit tests:
   - harvest depletion, workstation matching, and reservation blocking in `production_actions.rs`
   - craft WIP persistence and interrupted craft behavior in `production_actions.rs`
   - pick-up / put-down capacity and possession behavior in `transport_actions.rs`
   - in-transit occupancy and carried-item travel propagation in `travel_actions.rs`
3. The real gap is **scheduler-driven cross-action integration**, not more duplication of existing handler-level checks.
4. Integration assertions must match the current architecture:
   - transport is `pick_up` + `travel` + `put_down`, not a direct place-to-place delivery action
   - only the traveling actor carries `InTransitOnEdge`; carried goods inherit transit through possession / containment and therefore have `effective_place == None` while in transit
   - harvest/regeneration conservation must use authoritative material-accounting helpers that include `ResourceSource.available_quantity`, not just live-lot accounting

## What to Change

### 1. Integration test module

Create `crates/worldwake-systems/tests/e10_production_transport_integration.rs`.

### 2. Scheduler-driven scenarios

Add only the scenarios that cover missing system boundaries:

1. `harvest -> pick_up -> travel -> travel -> put_down` through the scheduler, with assertions after each phase for:
   - no teleportation
   - actor-only `InTransitOnEdge`
   - carried goods following the actor through possession/containment
   - explicit apple stock accounting (`ResourceSource + live apple lots`)
2. harvest-until-empty followed by regeneration ticks and a second harvest, with assertions that:
   - harvest affordance disappears when stock is depleted
   - regeneration restores affordance only after enough production ticks
   - explicit stock accounting stays constant across harvest/regeneration transitions
3. scheduler-driven craft with exact recipe accounting, validating:
   - staged inputs remain counted while WIP is active
   - final per-commodity totals match recipe deltas exactly

### 3. Keep the suite architecture-focused

Do not duplicate handler-level tests already covered elsewhere unless the scheduler path changes the behavior.

## Files to Touch

- `crates/worldwake-systems/tests/e10_production_transport_integration.rs` (new)

## Out of Scope

- Re-testing existing unit-level invariants already covered in `production_actions.rs`, `transport_actions.rs`, `travel_actions.rs`, or `production.rs`
- AI-driven decision selection (E13)
- Trade/restock logic (E11)
- Combat on routes (E12)
- Broad replay-engine testing already covered in `worldwake-sim`
- Performance or soak coverage (E22)

## Acceptance Criteria

### Tests That Must Pass

1. **Scheduler-driven multi-step transport**: harvest at source -> pick up -> travel edge 1 -> travel edge 2 -> put down at destination. Verify explicit source+lot accounting and no teleportation throughout.
2. **Depletion and regeneration**: harvest until stock is empty, verify the harvest affordance disappears, advance regeneration ticks, verify the affordance returns only when stock has concretely regenerated, then harvest again.
3. **Scheduler-driven craft accounting**: craft through the scheduler and verify staged inputs remain materially present during WIP and final per-commodity totals match the recipe exactly.
4. Existing focused unit coverage remains green:
   - interrupted craft WIP persistence
   - concurrent workstation reservation blocking
   - carried items following travel
   - route occupancy on the traveling actor
5. Existing suite: `cargo test --workspace`

### Invariants

1. Harvest/regeneration accounting is explicit: `ResourceSource.available_quantity + live lot quantity` stays constant unless regeneration adds stock.
2. Craft accounting is explicit and recipe-shaped: staged inputs remain materially present until commit, and commit applies the exact declared recipe delta.
3. No entity exists at a place while also being in transit.
4. Only the traveling actor carries `InTransitOnEdge`; carried items follow transit through possession/containment.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/tests/e10_production_transport_integration.rs`
   - scheduler-driven multi-step harvest/transport
   - depletion/regeneration/re-harvest coverage
   - scheduler-driven craft accounting coverage

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

Outcome amended: 2026-03-10

- Completion date: 2026-03-10
- What actually changed:
  - corrected the ticket assumptions and narrowed scope to the real missing coverage: scheduler-driven E10 integration tests
  - added `crates/worldwake-systems/tests/e10_production_transport_integration.rs`
  - refined `worldwake-core` conservation helpers to split lot-only accounting from authoritative commodity accounting, then updated the E10 integration tests to use the authoritative helper for source-backed materials
  - implemented three integration tests:
    - scheduler multi-step `harvest -> pick_up -> travel -> travel -> put_down`
    - harvest depletion -> regeneration -> re-harvest affordance gating
    - scheduler-driven craft accounting with staged WIP assertions
- Deviations from original plan:
  - removed redundant scenarios already covered in focused unit tests (`interrupted craft`, `concurrent workstation reservation`, `carried items travel with carrier`, `route occupancy` at the handler level)
  - removed the assumption that the old lot-only conservation helper covers `ResourceSource` stock; the codebase now exposes separate lot-only and authoritative conservation helpers
  - did not add E10-specific replay tests because the replay engine already has dedicated coverage in `worldwake-sim`, and the actual E10 gap was scheduler/action/system integration
  - observed and documented current scheduler behavior where regeneration may occur in the same tick as a harvest commit if the interval is due; tests now assert that concrete behavior instead of an incorrect stronger invariant
- Verification results:
  - `cargo test -p worldwake-systems` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
  - `cargo test --workspace` ✅
