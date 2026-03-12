# HARHYPENTIDE-007: Exact cargo planning tests and golden scenario

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — test-only ticket
**Deps**: HARHYPENTIDE-001 through HARHYPENTIDE-006 (all preceding tickets)
**Spec Reference**: HARDENING-hypothetical-entity-identity.md, Section E.5, Tests section

## Problem

The hypothetical entity identity architecture is now implemented, but the remaining confidence gap is narrower than originally written: we still need hard coverage that a materializing cargo step can bind a hypothetical lot at runtime and that the resulting authoritative cargo sequence remains deterministic under replay where replay infrastructure is actually applicable.

## Assumption Reassessment (2026-03-12, corrected)

1. `crates/worldwake-ai/tests/golden_e2e.rs` exists with deterministic scenarios, but none currently cover cargo materialization/binding flow — confirmed.
2. The ticket's original "missing tests" assumptions were stale:
   - `crates/worldwake-ai/src/planner_ops.rs` already tests exact partial pickup, zero-fit rejection, hypothetical split-off creation, hypothetical put-down semantics, and synthetic put-down candidate generation.
   - `crates/worldwake-ai/src/search.rs` already tests that search successor construction preserves hypothetical pickup metadata and synthesizes put-down candidates for hypothetical possessions.
   - `crates/worldwake-systems/src/transport_actions.rs` already tests authoritative split pickup, carried-lot travel behavior, and put-down.
   - `crates/worldwake-ai/src/agent_tick.rs` already tests binding application helpers and bound-target resolution.
3. The remaining missing coverage is the seam between those unit tests:
   - a real agent runtime executing a prebuilt plan across `pick_up` materialization, binding, `travel`, and `put_down`
   - deterministic replay of the authoritative fixed-input cargo sequence
4. `replay_and_verify()` exists in `worldwake-sim`, but it explicitly rejects stateful input producers. That means it cannot directly replay a live `AgentTickDriver` AI scenario. The original ticket's "planner golden replay" assumption was incorrect.
5. `GoalKind::MoveCargo` is present in the goal model, but current candidate generation defers it and `GoalKind::MoveCargo::is_satisfied(...)` is still `false`. A fully autonomous golden scenario where the AI naturally chooses `pick_up -> travel -> put_down` is therefore not a valid promise for this ticket without additional planner/goal architecture work.
6. Conservation verification helpers exist in `worldwake-core` and should be used at explicit checkpoints — confirmed.

## Architecture Check

1. This should remain test-only. The core architecture for hypothetical IDs, planner refs, materialization bindings, and exact partial pickup already exists.
2. The right missing test surface is runtime integration, not duplicated unit coverage.
3. Forcing a new autonomous AI cargo-delivery architecture into this ticket would be scope creep. If we want the AI to naturally originate `MoveCargo` plans, that should be a separate architecture ticket rather than disguised as "just more tests."
4. Replay coverage must respect the actual replay boundary:
   - runtime binding coverage belongs in `worldwake-ai`
   - deterministic replay coverage belongs on a fixed-input scheduler scenario, not on a live controller-driven scenario
5. No compatibility aliases or fallback paths should be introduced in tests. Tests should exercise the actual `PlanningEntityRef` and `CommitOutcome` architecture directly.

## What to Change

### 1. Add runtime integration coverage for a bound hypothetical cargo plan

In `crates/worldwake-ai/src/agent_tick.rs` tests:

- Seed a real world with an actor, a ground water lot that must split on pickup, and an adjacent destination.
- Preload `AgentDecisionRuntime` with a concrete three-step plan:
  1. `pick_up` authoritative ground lot
  2. `travel` to destination
  3. `put_down` the hypothetical split-off lot
- Drive the real tick loop through `AgentTickDriver` and assert:
  - partial pickup materializes a new authoritative lot
  - the runtime binds the expected hypothetical ID to that authoritative lot
  - the later `travel` and `put_down` steps resolve and complete against the bound entity
  - the final world state places the split lot on the ground at the destination

This is the real end-to-end gap in `worldwake-ai`.

### 2. Add deterministic replay coverage on the authoritative cargo sequence

In `crates/worldwake-systems/tests/e10_production_transport_integration.rs`:

- Build a fixed-input scenario for partial `pick_up` -> `travel` -> `put_down`
- Record replay checkpoints and scheduler inputs
- Verify replay reproduces the same final hash
- Verify live and authoritative conservation for `Water` at explicit checkpoints

This keeps replay on the side of the architecture that actually supports replay today.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify tests)
- `crates/worldwake-systems/tests/e10_production_transport_integration.rs` (modify tests)

## Out of Scope

- Production code changes to planner, goal generation, or transport/travel handlers
- Making `MoveCargo` an emitted autonomous goal
- Changing `GoalKind::MoveCargo` satisfaction semantics
- Replaying stateful AI-controller input production through `replay_and_verify()`
- New action families, carry-capacity rule changes, or replay-infrastructure redesign

## Acceptance Criteria

### Tests That Must Pass

1. A real `AgentTickDriver` runtime can execute a preloaded plan whose first step materializes a split-off lot and whose later step targets that lot via `PlanningEntityRef::Hypothetical(...)`.
2. The runtime binds the hypothetical ID exactly once and reuses that binding for later step resolution.
3. The final runtime test state shows the split-off lot grounded at the destination with correct quantities preserved.
4. A fixed-input scheduler replay of partial `pick_up` -> `travel` -> `put_down` reproduces identical final hashes.
5. Conservation invariants hold at explicit checkpoints in the replay-backed cargo scenario.
6. Existing focused AI and transport suites still pass.
7. `cargo test --workspace`
8. `cargo clippy --workspace`

### Invariants

1. No hypothetical entity leaks into authoritative world state.
2. Runtime binding remains explicit: hypothetical step targets resolve only through `MaterializationBindings`.
3. Conservation invariants hold at each explicit checkpoint used by the tests.
4. Replay determinism is validated only at a boundary the current replay architecture supports.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` — runtime execution of preloaded partial-pickup/travel/put-down plan with hypothetical binding.
2. `crates/worldwake-systems/tests/e10_production_transport_integration.rs` — deterministic replay coverage for authoritative partial-pickup/travel/put-down sequence.

### Commands

1. `cargo test -p worldwake-ai agent_tick`
2. `cargo test -p worldwake-systems --test e10_production_transport_integration`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-12
- What actually changed:
  - Added `agent_tick::tests::materialized_pickup_binding_survives_intervening_travel_until_put_down_resolution` to verify that a split-off hypothetical cargo lot is bound once, survives an intervening travel step in runtime state, resolves correctly for a later `put_down`, and is cleared at plan completion.
  - Added `scheduler_partial_pickup_travel_put_down_replays_deterministically` to `crates/worldwake-systems/tests/e10_production_transport_integration.rs` to verify deterministic replay plus live/authoritative water conservation for the authoritative fixed-input cargo sequence.
  - Corrected the ticket assumptions and scope to match the codebase that already had planner/search/transport/binding unit coverage.
- Deviations from original plan:
  - No `golden_e2e` AI scenario was added. The original ticket assumed the current architecture could naturally express and replay a live AI `pick_up -> travel -> put_down` scenario, but that is not true today.
  - The runtime test was narrowed to the binding/resolution contract in `agent_tick` instead of a full controller-driven cargo-delivery loop. During implementation it became clear that observation-snapshot dirtiness triggers replanning after cargo state changes, and `MoveCargo` is still a deferred/non-satisfied goal family; changing that architecture is a separate concern.
  - Replay verification was added at the fixed-input scheduler boundary, which is the boundary `replay_and_verify()` actually supports.
- Verification results:
  - `cargo test -p worldwake-ai agent_tick`
  - `cargo test -p worldwake-systems --test e10_production_transport_integration`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
