# HARCARGOACON-006: Integration verification tests for cargo goal continuity

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-ai (`agent_tick` tests, `search` tests, `enterprise` bug fix)
**Deps**: HARCARGOACON-001 through HARCARGOACON-005 (all prior tickets)

## Problem

The full cargo-goal continuity architecture needs end-to-end verification: candidate emission → plan search → execution with materialization → dirty replan → goal retention → satisfaction at destination. Individual unit tests from prior tickets verify pieces; this ticket verifies the integrated behavior and the key architectural property: goal stability across replanning after materializing cargo steps.

## Assumption Reassessment (2026-03-12)

### Confirmed Facts

1. `agent_tick.rs` does contain cargo-adjacent tests, but the existing `MoveCargo` coverage is limited to materialization-binding mechanics such as `materialized_pickup_binding_survives_intervening_travel_until_put_down_resolution`; it does **not** yet verify the full `agent_tick` runtime loop (candidate refresh -> dirty replan -> same-goal retention/rediscovery).
2. `crates/worldwake-systems/tests/e10_production_transport_integration.rs` already contains scheduler-level conservation and deterministic replay coverage via `scheduler_partial_pickup_travel_put_down_replays_deterministically`.
3. `HARCARGOACON-004` already added candidate-generation tests for:
   - `MoveCargo` emission from local stock plus demand
   - zero-deliverable suppression
   - at-destination suppression
   - locality / remote-stock suppression
4. `HARCARGOACON-005` already added goal-model and search coverage for:
   - destination-local `MoveCargo` satisfaction
   - cargo search support
   - a simple pickup -> travel cargo plan
5. `search.rs` now builds cargo plans with an exact `TransportActionPayload { quantity }` override in the simple search coverage, so the architecture is already beyond the older assumption that transport had no quantity-aware planning path.
6. `agent_tick` dirtiness is still driven by `observation_snapshot_changed(...)`, including commodity-signature changes caused by cargo materialization, so runtime continuity still depends on stable goal identity rather than hidden mutation suppression.

### Discrepancies Corrected From The Original Ticket

1. The original ticket treated candidate emission, search enablement, destination-local satisfaction, zero-deliverable suppression, conservation, and deterministic replay as still-unverified cargo architecture. They are already covered by prior tickets and existing tests.
2. The original ticket proposed adding new replay/conservation tests in `e10_production_transport_integration.rs`, but that file already covers the core partial-pickup -> travel -> put_down replay/conservation path. Duplicating that exact coverage would add noise more than value.
3. The real missing verification is narrower and more architectural: proving that the live `agent_tick` runtime preserves or re-derives the same `MoveCargo { commodity, destination }` intent after a materializing cargo step dirties the runtime.

## Architecture Check

1. Adding runtime continuity tests is more beneficial than expanding more unit-level cargo tests. The unit and scheduler layers are already covered; the unverified boundary is the live runtime loop where those pieces must compose honestly.
2. The current architecture is directionally correct: continuity should come from stable `GoalKind::MoveCargo { commodity, destination }` identity and ordinary dirty replanning, not from runtime exceptions or cargo-specific aliasing.
3. Reusing the existing scheduler replay/conservation test instead of cloning it keeps the test suite cleaner and more extensible. We should add only the missing runtime assertions, not restate already-proven lower-level invariants.
4. If the new runtime test exposes a failure, the correct fix is production code that preserves clean goal continuity. The fix should not introduce backward-compatibility shims, materialization aliases at the goal layer, or dirtiness suppression hacks.

## What to Change

### 1. Agent tick continuity tests (agent_tick.rs)

Add or update tests that verify:

**Scenario: Goal stability across replan after materialization**
- Set up agent with local cargo lot and merchandise profile with home_market demand
- Agent tick produces `MoveCargo { commodity, destination }` goal
- `pick_up` executes and materializes a split-off carried lot
- Observation snapshot changes and marks the runtime dirty
- The next read/plan cycle keeps or re-derives the same `MoveCargo { commodity, destination }` goal key
- The runtime continues toward the destination rather than switching to a different cargo identity or dropping the logistics intent

**Scenario: Satisfaction at destination while carrying**
- Agent arrives at destination carrying the commodity
- `MoveCargo` satisfaction check counts carried-at-destination stock
- Goal is satisfied without requiring explicit `put_down`

### 2. Merchant restock + delivery integration (agent_tick.rs)

**Scenario: Restock requires delivery to home_market**
- Merchant acquires commodity away from home_market
- Cargo intent remains active because home_market stock is still below remembered demand
- Delivery to home_market dampens cargo pressure at the destination
- The runtime does not treat remote possession alone as destination satisfaction

### 3. Existing lower-level verification to rely on, not duplicate

The following are already covered and should be treated as prerequisites, not reimplemented in this ticket:

- candidate-generation suppression and emission cases from `HARCARGOACON-004`
- destination-local satisfaction and cargo search support from `HARCARGOACON-005`
- scheduler replay/conservation for partial pickup/travel/put_down in `e10_production_transport_integration.rs`

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify — add runtime continuity / home-market delivery tests)
- `crates/worldwake-systems/tests/e10_production_transport_integration.rs` (only if implementation reveals a real missing integration assertion beyond the existing replay/conservation test)

## Out of Scope

- Re-testing candidate-generation cases already covered in `HARCARGOACON-004`
- Re-testing goal-model/search cases already covered in `HARCARGOACON-005`
- Reproducing the existing scheduler replay/conservation test unless a real gap is found
- Adding new action handlers or action definitions unless a failing runtime test proves a production bug
- Modifying replay/save-load infrastructure
- Performance optimization of observation snapshots (separate hardening)
- Generic hauling jobs or non-merchant cargo delivery

## Acceptance Criteria

### Tests That Must Pass

1. `goal_stability_across_cargo_replan_after_materialization` — same `MoveCargo` goal key re-derived after lot-splitting pick_up
2. `cargo_satisfaction_at_destination_while_carrying` — carried stock at destination satisfies `MoveCargo`
3. `merchant_restock_requires_delivery_to_home_market` — acquiring stock remotely does not suppress `MoveCargo` emission for home_market
4. Existing covered tests from `HARCARGOACON-004` and `HARCARGOACON-005` remain passing
5. Existing scheduler replay/conservation coverage in `e10_production_transport_integration.rs` remains passing
6. `cargo test --workspace` and `cargo clippy --workspace` pass

### Invariants

1. Cargo goal identity is stable across lot materialization — replanning finds the same `GoalKind::MoveCargo { commodity, destination }`
2. No cargo-specific runtime suppression or dirtiness hacks exist
3. Enterprise procurement and cargo delivery remain decoupled through shared concrete state
4. Lower-level conservation and deterministic replay guarantees remain covered by the existing scheduler integration test rather than duplicated here

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` — `goal_stability_across_cargo_replan_after_materialization`, `cargo_satisfaction_at_destination_while_carrying`, `merchant_restock_requires_delivery_to_home_market`
2. `crates/worldwake-ai/src/search.rs` — `cargo_search_handles_partial_pickup_split_before_travel`, `authoritative_partial_cargo_pickup_can_reach_goal_satisfaction`
3. `crates/worldwake-ai/src/enterprise.rs` — existing destination-local restock-gap tests now also cover the overflow regression fixed during this ticket
4. `crates/worldwake-systems/tests/e10_production_transport_integration.rs` — no new test added; existing replay/conservation coverage reused as intended

### Commands

1. `cargo test -p worldwake-ai agent_tick`
2. `cargo test -p worldwake-systems --test e10_production_transport_integration`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-12
- What actually changed:
  - corrected the ticket scope before implementation so it reflects the cargo hardening work already completed in `HARCARGOACON-004` and `HARCARGOACON-005`
  - added `agent_tick` runtime coverage for cargo-goal continuity, carried-at-destination satisfaction, and remote-stock delivery to `home_market`
  - added `search` regression coverage for partial split-pickup cargo plans, including an authoritative-world snapshot case
  - fixed a real production bug in `restock_gap_at_destination` and `restock_gap_for_market`, where eager `then_some(...)` evaluation could underflow when stock met or exceeded demand
- Deviations from original plan:
  - no new `e10_production_transport_integration.rs` test was needed because the existing scheduler replay/conservation test already covered that layer
  - the continuity test had to be written against the `agent_tick` runtime planning/replanning boundary rather than a naive “active action must still be running” assumption, because one-tick transport actions make that assertion brittle
  - the implementation exposed an architectural smell: `PlanningBudget.max_plan_depth` currently influences both search depth and planning snapshot travel horizon. This ticket did not redesign that coupling, but the tests now avoid assuming those two concerns are the same
- Verification results:
  - `cargo test -p worldwake-ai agent_tick` passed
  - `cargo test -p worldwake-ai search` passed
  - `cargo test -p worldwake-systems --test e10_production_transport_integration` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
