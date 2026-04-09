# S77BELCAPPRI-006: Reconcile scheduler-driven care integration with live affordance resolution

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-sim` self-inventory belief/read surface and `e09` care integration proof
**Deps**: S08ACTSTAABORES-002

## Problem

Broad verification after `S77BELCAPPRI-005` still fails in `crates/worldwake-systems/tests/e09_needs_integration.rs::scheduler_driven_care_actions_apply_effects_and_preserve_conservation`. The test queues strict external requests for `eat`, `drink`, `sleep`, `toilet`, and `wash`, but the run now aborts with `TickStepError::RequestedAffordanceUnavailable` before the care sequence completes. This ticket must determine whether the live care-action affordance surface is wrong under the current scheduler/runtime contract or whether the integration harness assumptions are stale.

## Assumption Reassessment (2026-04-09)

1. `cargo test -p worldwake-systems` fails outside `S77BELCAPPRI-005`'s tell/listener boundary in `scheduler_driven_care_actions_apply_effects_and_preserve_conservation` at `crates/worldwake-systems/tests/e09_needs_integration.rs:277-351`.
2. The failure boundary is authoritative request resolution, not later action execution: `Harness::run_queued_action_to_completion()` at `crates/worldwake-systems/tests/e09_needs_integration.rs:111-120` calls `self.step_once().unwrap()`, and the failing run returns `TickStepError::RequestedAffordanceUnavailable` from the shared tick-step pipeline.
3. This is a mixed-boundary ticket. The exact contract under audit is strict external request resolution in `worldwake-sim::tick_step` versus the live care-action affordance surface registered by `register_needs_actions()` and exercised through the `e09` harness.
4. The motivating scenario is not a golden narrative but an integration proof for scheduler-driven care and conservation. The invariant is that, given the concrete harness setup in `e09_needs_integration`, each queued care request should remain lawfully requestable at the moment it is issued.
5. Existing archived coverage in `archive/tickets/S08ACTSTAABORES-002-heal-first-effect-medicine-conservation.md` identifies this same `e09` test as the cross-crate proof for scheduler-driven care. Any correction here must preserve that role rather than downgrading it into a weaker unit-only assertion.
6. Focused reproduction showed the first failing request is `drink`, not `eat` or a later place-bound care action: after `eat` completes, `step_tick()` rejects strict `drink` with `RequestedAffordanceUnavailable { def_id: ActionDefId(1), targets: [water] }`.
7. The live mismatch is not inventory mutation. After `eat`, `world.possessions_of(actor)` still contains both the remaining bread lot and the water lot, but `get_affordances()` only returns `sleep`.
8. Root cause: `PerAgentBeliefView::direct_possessions()` exposes authoritative self possessions, but `affordance_query` then filters those target IDs through `EntityBeliefView::entity_kind()`. `PerAgentBeliefView::knows_entity()` did not treat self-possessed entities as known, so directly possessed lots vanished from `EntityDirectlyPossessedByActor` target enumeration once ordinary perception had dropped their belief entries.
9. Auto-correction applied: ticket said the likely fix lived in `tick_step` or needs-action affordance exposure; live code has a narrower self-local read-surface contradiction in `crates/worldwake-sim/src/per_agent_belief_view.rs`. Correction applied: retarget the production fix to `PerAgentBeliefView::knows_entity()` plus the `e09` integration proof. Safe because the failing request surface still reproduces through the same strict request-resolution path, and the read-surface mismatch is now directly proved.
10. Adjacent contradiction classification: broader `cargo test -p worldwake-systems` also reaches `e15_information_integration::hidden_event_at_empty_location_remains_isolated_from_remote_agents`, but that assertion failure sits outside the owned care/request-resolution boundary and requires separate triage.

## Architecture Check

1. Fixing the self-local belief/read boundary is cleaner than weakening the integration test or patching `tick_step`. The actor lawfully knows what they are directly carrying, so `EntityDirectlyPossessedByActor` target enumeration should not depend on a stale carried-item belief entry.
2. No backward-compatibility shims should be introduced. The ticket should either restore the lawful scheduler-driven care path or update the integration proof to the current canonical behavior if the prior expectation is no longer architecturally valid.

## Verification Layers

1. Self-possessed lots remain readable as known entity kinds even without a surviving carried-item belief entry -> focused `worldwake-sim` unit test
2. The sequential `eat` then strict `drink` request remains available through `step_tick()` -> focused `e09` integration proof
3. Scheduler-driven care still applies effects and preserves commodity conservation across the full sequence -> existing `scheduler_driven_care_actions_apply_effects_and_preserve_conservation`
4. Mixed-layer ticket: the integration proof still closes through shared request resolution, but the strongest production proof is the `PerAgentBeliefView` self-local read boundary

## What to Change

### 1. Widen self-local knowledge for directly possessed entities

Update `PerAgentBeliefView` so directly possessed entities count as known to the acting agent even when no carried-item belief entry survives in `AgentBeliefStore`.

### 2. Prove the read-surface fix at the owning layer

Add a focused `worldwake-sim` proof showing `entity_kind()` still returns `Some(EntityKind::ItemLot)` for an unknown-but-self-possessed lot.

### 3. Keep the integration proof strong

Preserve the `e09` integration proof and add a focused sequential `eat` then strict `drink` regression so the shared request-resolution path stays covered at the scenario level.

## Files to Touch

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-systems/tests/e09_needs_integration.rs` (modify)

## Out of Scope

- Tell/listener belief-capacity behavior from `S77BELCAPPRI-005`
- Broad planner/golden expansion unless reassessment shows the care request drift is AI-visible
- Unrelated request-resolution paths outside the scheduler-driven care sequence

## Acceptance Criteria

### Tests That Must Pass

1. Focused: self-possessed unknown lots still expose `entity_kind == Some(ItemLot)` through `PerAgentBeliefView`
2. Existing integration: `cargo test -p worldwake-systems --test e09_needs_integration scheduler_driven_care_actions_apply_effects_and_preserve_conservation`
3. Focused integration: `cargo test -p worldwake-systems --test e09_needs_integration scheduler_sequential_eat_then_drink_requests_remain_available`
4. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. The `e09` scheduler-driven care sequence remains lawful under the live strict request-resolution contract.
2. Care-action integration coverage still proves both effect application and commodity conservation rather than relying only on lower-layer unit tests.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — `entity_kind_returns_item_lot_for_self_possessed_unknown_item` — prove self-local item knowledge survives without a carried-item belief entry
2. `crates/worldwake-systems/tests/e09_needs_integration.rs` — `scheduler_sequential_eat_then_drink_requests_remain_available` — prove the previously unavailable strict `drink` request remains requestable after `eat`
3. `crates/worldwake-systems/tests/e09_needs_integration.rs` — keep `scheduler_driven_care_actions_apply_effects_and_preserve_conservation` as the end-to-end care/conservation proof

### Commands

1. `cargo test -p worldwake-sim per_agent_belief_view::tests::entity_kind_returns_item_lot_for_self_possessed_unknown_item`
2. `cargo test -p worldwake-systems --test e09_needs_integration scheduler_sequential_eat_then_drink_requests_remain_available`
3. `cargo test -p worldwake-systems --test e09_needs_integration scheduler_driven_care_actions_apply_effects_and_preserve_conservation`
4. `cargo test -p worldwake-systems`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-09.

- Fixed the real production boundary in `crates/worldwake-sim/src/per_agent_belief_view.rs`: directly possessed entities now count as known to the acting agent, so `entity_kind()` and `EntityDirectlyPossessedByActor` affordance enumeration remain lawful even after carried-item belief entries have been dropped by ordinary perception updates.
- Added the focused `worldwake-sim` proof `entity_kind_returns_item_lot_for_self_possessed_unknown_item`.
- Added the `e09` regression `scheduler_sequential_eat_then_drink_requests_remain_available` and restored `scheduler_driven_care_actions_apply_effects_and_preserve_conservation`.
- Deviation from original plan: reassessment showed the bug was not in `tick_step` or needs-action preconditions. The strict request-resolution failure was only exposing a narrower self-local read-surface contradiction in `PerAgentBeliefView`.

## Verification Result

- Passed `cargo test -p worldwake-sim per_agent_belief_view::tests::entity_kind_returns_item_lot_for_self_possessed_unknown_item`
- Passed `cargo test -p worldwake-systems --test e09_needs_integration scheduler_sequential_eat_then_drink_requests_remain_available`
- Passed `cargo test -p worldwake-systems --test e09_needs_integration scheduler_driven_care_actions_apply_effects_and_preserve_conservation`
- `cargo test -p worldwake-systems` still fails outside this ticket's owned care/request-resolution boundary in `e15_information_integration::hidden_event_at_empty_location_remains_isolated_from_remote_agents`; follow-up captured in `tickets/S77BELCAPPRI-007.md`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
