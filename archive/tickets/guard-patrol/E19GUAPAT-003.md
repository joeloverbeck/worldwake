# E19GUAPAT-003: Implement patrol action definition, duration contract, and handler

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new patrol action, patrol duration expression, action registry wiring
**Deps**: E19GUAPAT-001 (PatrolRoute, PatrolProfile components), E19GUAPAT-002 (GoalKind::Patrol, PlannerOpKind::Patrol, EventTag::Patrol), [specs/E19-guard-patrol.md](/home/joeloverbeck/projects/worldwake/specs/E19-guard-patrol.md)

## Problem

`GoalKind::Patrol { place }` and `PlannerOpKind::Patrol` already exist, but the action catalog still has no `"patrol"` action for guards to execute. The missing runtime leaf breaks the intended shared boundary between patrol route state (`PatrolRoute.current_index`), planner patrol steps keyed by a waypoint `place`, and authoritative action execution that advances route progress only on commit.

## Assumption Reassessment (2026-03-30)

1. The shared abstraction boundary under audit is: `PatrolRoute { assigned_places, current_index }` in [crates/worldwake-core/src/patrol.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/patrol.rs), `GoalKind::Patrol { place }` in [crates/worldwake-core/src/goal.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs), and the missing `"patrol"` `ActionDef` classification surface in [crates/worldwake-ai/src/planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs). This ticket should complete that boundary, not invent a second route/waypoint identity path.
2. Action registration still follows the live pattern in [crates/worldwake-systems/src/action_registry.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs): each action family exposes `register_*_action()` and `register_all_actions()` wires it into the full catalog.
3. Patrol should follow the existing targeted-action pattern, not a payload-only pattern. The closest live analogue is [crates/worldwake-systems/src/investigate_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/investigate_actions.rs), which uses `TargetSpec::ActorPlace` plus start-time authoritative validation. Because `GoalKind::Patrol { place }` already carries the waypoint as the plan destination, duplicating that waypoint in `ActionPayload` would create aliasing between planner target state and `PatrolRoute.current_index`.
4. `PatrolProfile` already includes `vigilance` in [crates/worldwake-core/src/patrol.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/patrol.rs), so dwell duration can and should remain profile-driven as the spec intends. The original ticket was correct on the presence of the field, but incomplete on the runtime implication: the action framework currently lacks a patrol-specific `DurationExpr` variant, so duration scaling cannot be implemented cleanly without extending the shared duration contract.
5. `ActionPayload` currently has 20 variants in [crates/worldwake-sim/src/action_payload.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs). No patrol payload variant is currently needed if the patrol action is target-based and reads authoritative route state directly.
6. Action duration is resolved before `start_*` runs in [crates/worldwake-sim/src/start_gate.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/start_gate.rs). Therefore patrol duration must be modeled through a new `DurationExpr` branch resolved in both authoritative and belief-estimation paths, not via ad-hoc handler-local timers or a hidden fixed constant.
7. Commit-time patrol event tagging is already framework-owned. [crates/worldwake-sim/src/tick_action.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_action.rs) adds `EventTag::ActionCommitted` plus every tag in `ActionDef.causal_event_tags` when the commit succeeds. The patrol handler should mutate route progress through `WorldTxn`; it does not need a bespoke event-emission path.
8. `GoalKind::Patrol { place }` is already treated as a progress barrier for `PlannerOpKind::Patrol` in [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), and `classify_action_def()` already maps `(ActionDomain::Generic, "patrol")` to `PlannerOpKind::Patrol` in [crates/worldwake-ai/src/planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs). The missing piece is the action definition itself, not planner alias work.
9. Current focused/unit coverage exists for nearby action patterns, but there is no patrol-specific runtime coverage yet. `cargo test -p worldwake-systems -- --list` currently exposes `action_registry::tests::build_full_action_registries_returns_complete_action_catalog`, `investigate_actions::*`, and `travel_actions::*` tests, but no patrol tests. This ticket must add focused patrol action coverage rather than relying on broader crate tests alone.
10. Mismatch + correction: the original ticket proposed a new patrol payload and touching [crates/worldwake-sim/src/action_payload.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs). That duplicates live planner target identity without solving a current runtime gap. Corrected scope: implement patrol as a target-based generic action keyed by the waypoint target and add the missing shared `DurationExpr` support instead of adding a patrol payload alias.
11. No adjacent contradictions currently block this ticket. Route adaptation, patrol candidate generation, and public-order integration remain separate follow-up work exactly because this ticket can complete the patrol execution leaf without taking ownership of those higher-layer behaviors.

## Architecture Check

1. A target-based patrol action keeps one lawful identity path for the waypoint: planner goal `place` -> bound action target -> authoritative validation against `PatrolRoute.current_index`. This is cleaner than adding a parallel payload field that could drift from the route or goal target.
2. A dedicated patrol `DurationExpr` is more robust than a handler-local constant because both authoritative start validation and belief-side duration estimation need the same contract. Extending the shared duration surface now prevents future patrol timing logic from fragmenting across runtime layers.
3. Advancing `PatrolRoute.current_index` only on commit preserves the authoritative stored-state invariant described in the spec and avoids backwards-compatibility shims or alias paths.

## Verification Layers

1. Patrol action definition shape and full-catalog registration -> focused unit tests in `patrol_actions.rs` plus `action_registry::tests::build_full_action_registries_returns_complete_action_catalog`
2. Profile-driven patrol duration contract -> focused unit tests for the new `DurationExpr` resolution surface and patrol action definition assertions
3. Patrol start-gate authoritative invariants (`PatrolRoute`, `PatrolProfile`, actor co-located with current waypoint target) -> focused runtime tests using `start_action`
4. Commit/abort route-progress ordering (`current_index` advances only on commit, never on abort) -> focused runtime tests using `tick_action` / `abort_action` plus authoritative world-state assertions
5. Successful patrol commit emits `EventTag::Patrol` through the framework-owned tag path -> focused runtime test asserting event-log tags after commit
6. Shared patrol duration contract remains planner-snapshot compatible -> focused `worldwake-ai` parity/unit coverage plus crate test pass

## What to Change

### 1. Add patrol action implementation in `worldwake-systems`

Create `crates/worldwake-systems/src/patrol_actions.rs` with:
- `register_patrol_action(defs, handlers) -> ActionDefId`
- `patrol_action_def() -> ActionDef`
- `start_patrol()` that authoritatively checks:
  - actor has `PatrolRoute`
  - actor has `PatrolProfile`
  - route is non-empty
  - action target equals `assigned_places[current_index]`
  - actor is effectively at that waypoint
- `tick_patrol()` as standard duration-driven continuation
- `commit_patrol()` that advances `current_index` modulo `assigned_places.len()` via `WorldTxn`
- `abort_patrol()` that leaves route state unchanged

### 2. Extend the shared duration contract in `worldwake-sim`

Add a patrol-specific `DurationExpr` variant and resolve it in both:
- authoritative duration resolution in [crates/worldwake-sim/src/action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs)
- belief-side duration estimation in [crates/worldwake-sim/src/belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs)

The duration must derive from `PatrolProfile` in a deterministic way and avoid introducing patrol-local magic numbers or floats.

### 3. Wire patrol into the public action catalog

Update:
- [crates/worldwake-systems/src/action_registry.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs)
- [crates/worldwake-systems/src/lib.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs)

so the full registry exports and registers the patrol action.

### 4. Keep planner duration inventory and snapshot parity aligned

Because the new patrol duration expression becomes part of the shared non-fixed planner duration surface, update the minimal `worldwake-ai` contracts that enumerate or mirror live duration dependencies so they remain exhaustive and testable.

### 5. Add focused patrol runtime tests

Add tests covering:
- definition shape and duration expression
- registration in the full catalog
- commit advances route index
- wraparound from final waypoint to zero
- abort preserves route index
- start rejects missing `PatrolRoute`
- start rejects missing `PatrolProfile`
- start rejects stale/mismatched waypoint target
- commit produces `EventTag::Patrol`

## Files to Touch

- `crates/worldwake-systems/src/patrol_actions.rs` (new)
- `crates/worldwake-systems/src/action_registry.rs` (modify)
- `crates/worldwake-systems/src/lib.rs` (modify)
- `crates/worldwake-sim/src/action_semantics.rs` (modify)
- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-ai/src/planner_duration_contract.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)

## Out of Scope

- Patrol candidate generation / patrol motive ranking (separate ticket)
- Route adaptation / waypoint reordering
- Public-order `guard_presence_factor()` derived view integration
- Golden E2E guard patrol scenarios
- Captain-issued patrol orders or richer route-entry metadata
- Any patrol payload alias path in `ActionPayload`

## Acceptance Criteria

### Tests That Must Pass

1. Patrol action is registered and retrievable from the full action registry by name `"patrol"`
2. Patrol duration resolves from `PatrolProfile` through the shared `DurationExpr` surface
3. Patrol commit advances `current_index` from 0 to 1 on a multi-waypoint route
4. Patrol commit wraps `current_index` from the final waypoint back to 0
5. Patrol abort leaves `current_index` unchanged
6. Patrol start rejects actors missing `PatrolRoute`
7. Patrol start rejects actors missing `PatrolProfile`
8. Patrol start rejects a waypoint target that does not match `assigned_places[current_index]`
9. Patrol commit records an event tagged with `EventTag::Patrol`
10. Existing suite: `cargo test -p worldwake-systems`
11. Existing suite: `cargo test -p worldwake-sim`
12. Existing suite: `cargo test -p worldwake-ai`
13. `cargo clippy --workspace`

### Invariants

1. The waypoint identity path remains single-source: planner/affordance target and authoritative `PatrolRoute.current_index` must agree; no patrol payload alias is introduced
2. `PatrolRoute.current_index` mutates only through `WorldTxn` on successful commit
3. Patrol remains `ActionDomain::Generic` and `VisibilitySpec::SamePlace`
4. No `HashMap`, `f32`, or `f64` are introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/patrol_actions.rs` — verifies patrol action registration, start-gate validation, commit/abort route-progress semantics, and patrol event tagging at the action runtime layer
2. `crates/worldwake-systems/src/action_registry.rs` — extends the full-catalog assertion so patrol is proven present in the shared registry
3. `crates/worldwake-sim/src/action_semantics.rs` and/or `crates/worldwake-sim/src/belief_view.rs` — verifies the new patrol duration expression resolves consistently across authoritative and belief-estimation surfaces
4. `crates/worldwake-ai/src/planning_state.rs` and `crates/worldwake-ai/src/planner_duration_contract.rs` — keeps planner duration dependency inventory and snapshot/runtime duration parity exhaustive after adding patrol as a non-fixed duration source

### Commands

1. `cargo test -p worldwake-systems patrol_actions::`
2. `cargo test -p worldwake-systems action_registry::tests::build_full_action_registries_returns_complete_action_catalog`
3. `cargo test -p worldwake-sim patrol`
4. `cargo test -p worldwake-ai`
5. `cargo test -p worldwake-systems`
6. `cargo test -p worldwake-sim`
7. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-30
- Actual changes:
  - added `patrol_actions.rs` with a target-based `"patrol"` action that validates the current waypoint, advances `PatrolRoute.current_index` only on commit, and relies on framework-owned `EventTag::Patrol` commit tagging
  - added `DurationExpr::ActorPatrolProfile` and resolved it in both authoritative and belief-estimation paths using the live `PatrolProfile` contract
  - registered/exported the patrol action in the systems action catalog
  - updated `worldwake-ai` planner duration inventory and planning snapshot/runtime parity surfaces to include the new patrol duration dependency
- Deviations from original plan:
  - no patrol payload or `ActionPayload` variant was added; the waypoint remains single-sourced through the planner/action target plus authoritative `PatrolRoute.current_index`
  - AI snapshot/inventory files needed updates as a required consequence of extending the shared duration contract, even though patrol candidate generation itself stayed out of scope
- Verification results:
  - `cargo test -p worldwake-systems patrol_actions::` passed
  - `cargo test -p worldwake-systems action_registry::tests::build_full_action_registries_returns_complete_action_catalog` passed
  - `cargo test -p worldwake-sim patrol` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test -p worldwake-systems` passed
  - `cargo test -p worldwake-sim` passed
  - `cargo clippy --workspace` passed
