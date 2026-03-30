# E19GUAPAT-006: Implement belief-driven patrol route adaptation

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new authoritative patrol system hook plus patrol-route mutation logic
**Deps**: E19GUAPAT-001 (PatrolRoute, PatrolProfile), E19GUAPAT-004 (patrol candidate generation exists)

## Problem

Guards must adapt their patrol routes from belief-local crime information: adding or promoting waypoints where theft has been personally confirmed or socially reported, and deprioritizing waypoints that are no longer recent on those same belief surfaces. Route adaptation must never read global world-state crime truth on the guard's behalf.

## Assumption Reassessment (2026-03-30)

1. `PatrolRoute` and `PatrolProfile` already exist in [`crates/worldwake-core/src/patrol.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/patrol.rs). The mutable authoritative route contract is still only `assigned_places: Vec<EntityId>` plus `current_index: usize`.
2. Patrol candidate generation is already live in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs), and patrol motive ranking is already live in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs). The earlier narrative that E19GUAPAT-004 still needed to replace a placeholder patrol path is stale for this repo state.
3. The shared boundary under audit is: authoritative `PatrolRoute` state in `worldwake-core`, mutated by a world system in `worldwake-systems`, then read by AI candidate generation and planning in `worldwake-ai`. This ticket must preserve that read/write split.
4. `GoalKind::Patrol` already exists in [`crates/worldwake-core/src/goal.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs), and `PlannerOpKind::Patrol` is already wired through the AI planner. This ticket should not broaden into patrol-goal or patrol-op architecture that already shipped.
5. `ViolationMemory` does exist in [`crates/worldwake-core/src/violation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/violation.rs), and `unresolved_records(current_tick)` returns unresolved records. But `RecordedViolation` does not have a shared top-level `place` field, so the lawful patrol place surface is variant-specific: today that is `ViolationKind::SuspectedTheft { theft, .. }` via `theft.expected_place`.
6. The ticket's original assumption that "crime reports" arrive through `ViolationMemory` is incorrect. Cross-agent report propagation currently travels through `AgentBeliefStore.social_observations` and `Tell`, not by appending `ViolationMemory` entries for the listener. The relevant report carrier is `SocialObservationDetail::SuspectedTheft { theft, .. }` in [`crates/worldwake-core/src/belief.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs).
7. Spec Section 6 names `ViolationMemory`, `known_social_observations`, and institutional beliefs as lawful route-adaptation inputs. The live ticket must match that wider belief surface instead of narrowing adaptation to `ViolationMemory` alone.
8. There is no patrol system slot today. The closed scheduler set in [`crates/worldwake-sim/src/system_manifest.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/system_manifest.rs) currently ends with `Perception`, and [`crates/worldwake-systems/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs) has no patrol system handler. This ticket therefore includes adding a dedicated authoritative patrol system hook rather than only editing an existing patrol file.
9. Candidate generation is read-only today, and keeping it read-only is still the cleaner architecture. Route adaptation requires `WorldTxn` mutation and should not be smuggled into `emit_patrol_candidates()`.
10. The minimal route contract is still sufficient for this ticket if adaptation is expressed as stable reordering plus monotonic append of newly relevant waypoints. Do not introduce waypoint side metadata unless focused tests prove `assigned_places + current_index` cannot express the invariant cleanly.
11. The spec calls for recency/staleness behavior, but the current codebase has no patrol-specific hard threshold constant. The clean implementation is to derive a route-reactive recency window from existing retention/freshness data plus `route_adaptation_sensitivity`, not to introduce new magic numbers.
12. Adjacent contradiction exposed during reassessment: patrol motive escalation in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) currently counts active `ViolationMemory` theft records and institutional beliefs, but not socially relayed theft observations. That is adjacent to, but not required for, route mutation itself. Keep this ticket scoped to route adaptation and treat report-driven patrol-motive escalation as follow-up work if it proves necessary.

## Architecture Check

1. **Option A**: Dedicated `patrol_route_adaptation_system()` in `worldwake-systems`, added to the closed scheduler after `Perception`. This keeps authoritative route mutation inside the system layer, lets the system consume fresh same-tick belief updates, and leaves AI candidate generation read-only.
2. **Option B**: Piggyback route mutation inside `emit_patrol_candidates()` or ranking helpers. This would mix authoritative mutation into the AI read path and weaken the existing belief-view boundary.
3. **Option C**: Overload `perception_system()` to mutate routes as a side effect of belief projection. This avoids a new system id but couples belief acquisition and patrol-route maintenance into one subsystem.
4. **Recommendation**: Option A. A dedicated patrol system is the cleanest long-term architecture because patrol-route maintenance is authoritative world state, not AI computation and not perception itself.
5. Route adaptation should stay on the minimal route contract for now. If future work needs a clean separation between baseline assignment and transient hotspots, make that a direct `PatrolRoute` model upgrade rather than sidecar metadata or alias paths.
6. No backwards-compatibility shims or alias routes.

## Verification Layers

1. Report-driven waypoint append from social belief memory -> focused patrol-system test asserting authoritative `PatrolRoute.assigned_places`
2. Self-held `ViolationMemory` theft record promotes/adds waypoint -> focused patrol-system test asserting authoritative `PatrolRoute`
3. Sensitivity/freshness gate -> focused patrol-system test asserting stale reports do not change the route
4. Deprioritization of non-recent waypoints -> focused patrol-system test asserting stable route reordering, not removal
5. Information locality / no cross-agent leakage -> focused patrol-system test where another agent holds the report but the guard does not
6. Runtime safety for in-flight actions -> focused patrol-system test proving guards with active actions are not rewritten mid-action
7. Single-layer ticket note: decision and action traces are not the primary proof surface here because the contract under change is authoritative route mutation before the next AI read cycle

## What to Change

### 1. Add a dedicated patrol system module

Create [`crates/worldwake-systems/src/patrol.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/patrol.rs) with a `patrol_route_adaptation_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError>`.

The system should:

```rust
// For each idle agent with PatrolRoute + PatrolProfile:
//   1. Read belief-local patrol signals only:
//      - active suspected-theft records from ViolationMemory
//      - retained suspected-theft social observations from AgentBeliefStore
//   2. Convert those signals into route-relevant places using exact lawful fields
//   3. Apply a sensitivity-scaled freshness window derived from existing retention/TTL data
//   4. Rebuild the route by:
//      - promoting currently relevant places
//      - appending newly relevant places not already assigned
//      - preserving inactive places later in stable order
//      - never removing existing assigned places
//   5. Reset or preserve current_index according to the new ordered route contract
//   6. Commit changed PatrolRoute values through WorldTxn
```

### 2. Register a dedicated patrol system slot

Add `SystemId::Patrol` to [`crates/worldwake-sim/src/system_manifest.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/system_manifest.rs) and wire the handler in [`crates/worldwake-systems/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs).

Ordering requirement:
- Run `Patrol` after `Perception`, so same-tick perception/tell-driven social observations are visible before the next AI input cycle.

### 3. Derive recency from existing freshness data

Do not introduce new patrol-specific magic constants.

Use existing retention windows:
- `RecordedViolation.expires_tick - observed_tick` for `ViolationMemory`
- `PerceptionProfile.memory_retention_ticks` for retained social observations

Then scale the route-reactive window by `PatrolProfile.route_adaptation_sensitivity`.

## Files to Touch

- `crates/worldwake-systems/src/patrol.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify — export and dispatch patrol system)
- `crates/worldwake-sim/src/system_manifest.rs` (modify — add patrol system id and canonical order)
- `crates/worldwake-systems/src/patrol_actions.rs` (modify only if shared patrol helpers or tests belong there)

## Out of Scope

- Patrol action handler (E19GUAPAT-003 — already delivered)
- Patrol candidate generation and baseline patrol motive scoring (already delivered in current code)
- Guard presence factor (E19GUAPAT-005)
- Captain-mediated route reassignment (deferred per spec to future epic)
- Permanent waypoint removal (spec explicitly defers this)
- Golden E2E tests (E19GUAPAT-007)
- Patrol-motive escalation from socially relayed theft observations unless route-adaptation implementation proves that gap must be closed in the same ticket
- Richer patrol route-entry metadata unless a concrete adaptation invariant proves `Vec<EntityId>` is insufficient during implementation

## Acceptance Criteria

### Tests That Must Pass

1. Guard with a retained `SocialObservationDetail::SuspectedTheft` for a new place gains that place on `assigned_places`
2. Guard with an active `ViolationKind::SuspectedTheft` record for a new place gains or promotes that place on the route
3. Low-sensitivity guard ignores a report that falls outside its sensitivity-scaled freshness window
4. Route rebuild moves currently relevant patrol places ahead of stale ones without removing any existing assigned waypoint
5. Guard does not adapt from another agent's report or from unrelated world state it has not perceived
6. Guard with an active action is not route-mutated mid-action
7. Existing suite: `cargo test -p worldwake-systems`
8. `cargo test -p worldwake-ai -- --list`
9. `cargo clippy --workspace`

### Invariants

1. Route adaptation reads only from the guard's belief-local patrol surfaces: `ViolationMemory`, retained `AgentBeliefStore.social_observations`, and any already-allowed institutional beliefs. It does not query authoritative crime truth on behalf of the guard.
2. No waypoint already present in `assigned_places` is removed by route adaptation
3. `assigned_places` remains the sole authoritative ordered route contract in scope
4. No `HashMap` or `f32`/`f64` introduced
5. System order remains deterministic
6. If `Vec<EntityId>` proves insufficient, the ticket must be corrected first to justify a direct route-model upgrade rather than sidecar route metadata or alias paths

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/patrol.rs` — `social_report_adds_new_patrol_waypoint`: proves retained theft social observations append a new route place and make it the next patrol target.
2. `crates/worldwake-systems/src/patrol.rs` — `violation_memory_record_adds_new_patrol_waypoint`: proves active `ViolationMemory` theft records also append/promote a patrol place.
3. `crates/worldwake-systems/src/patrol.rs` — `low_sensitivity_ignores_stale_report`: proves the freshness gate is derived from sensitivity and existing memory retention rather than unconditional route mutation.
4. `crates/worldwake-systems/src/patrol.rs` — `active_places_are_promoted_without_removing_existing_waypoints`: proves route adaptation reorders and preserves existing waypoints instead of shrinking the route.
5. `crates/worldwake-systems/src/patrol.rs` — `guard_does_not_adapt_from_other_agents_reports`: proves per-guard information locality; another agent's report does not mutate this guard's route.
6. `crates/worldwake-systems/src/patrol.rs` — `active_action_skips_route_adaptation`: protects in-flight action/runtime safety by proving idle-only route mutation.
7. `crates/worldwake-systems/src/patrol.rs` — `dispatch_table_uses_patrol_system_for_patrol_slot`: proves the new scheduler slot is wired to the patrol system.

### Commands

1. `cargo test -p worldwake-systems patrol`
2. `cargo test -p worldwake-systems`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`
5. `cargo test --workspace`

## Outcome

Completed: 2026-03-30

What actually changed:
- Added a dedicated `Patrol` system slot to the closed scheduler and implemented `patrol_route_adaptation_system` in [`crates/worldwake-systems/src/patrol.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/patrol.rs).
- Route adaptation now reads the live lawful patrol signals that exist in this repo: unresolved `ViolationMemory` suspected-theft records plus retained `AgentBeliefStore.social_observations` suspected-theft reports.
- Freshness is derived from existing retention windows and `route_adaptation_sensitivity`; no new patrol-specific magic-number thresholds were added.
- Route rebuild is monotonic append plus stable reordering; existing assigned waypoints are never removed.
- Idle-only mutation was added as an explicit runtime safety rule so active actions are not invalidated by same-tick route rewrites.

Deviations from original plan:
- The original ticket assumed route adaptation could rely on `ViolationMemory` alone. That was incorrect for the live code because socially reported crime knowledge travels through `AgentBeliefStore.social_observations`.
- The original ticket treated E19GUAPAT-004 patrol candidate/ranking work as still pending. In the live code, that architecture was already delivered, so this ticket stayed scoped to authoritative route mutation.
- The implementation did not broaden into patrol-motive escalation from socially relayed theft observations. That adjacent ranking gap remains separate from route mutation.

Verification results:
- `cargo test -p worldwake-systems patrol`
- `cargo test -p worldwake-systems`
- `cargo test -p worldwake-ai`
- `cargo clippy --workspace`
- `cargo test --workspace`
