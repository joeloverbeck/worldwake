# S35OBSACTSIG-003: Implement `observe_active_actions()` perception helper

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-systems perception
**Deps**: S35OBSACTSIG-001 (BelievedActivity type, ActionDomain in core)

## Problem

`perception_system()` receives `active_actions` and `action_defs` via `SystemExecutionContext` but ignores both (destructured as `_active_actions`, `_action_defs`). Agents cannot observe what co-located agents are doing. This ticket adds the `observe_active_actions()` helper that populates `BelievedActivity` on `BelievedEntityState`.

## Assumption Reassessment (2026-03-29)

Shared abstraction boundary under audit: authoritative scheduler activity (`ActionInstance` + `ActionDef.domain`) -> perception-owned `AgentBeliefStore.known_entities[*].believed_activity`.

1. `perception_system()` at `crates/worldwake-systems/src/perception.rs:19` still receives `active_actions: &BTreeMap<ActionInstanceId, ActionInstance>` and `action_defs: &ActionDefRegistry`, and still ignores both by destructuring them as `_active_actions` / `_action_defs`.
2. `observe_passive_local_entities()` at `crates/worldwake-systems/src/perception.rs:176` only snapshots colocated entity state through `build_believed_entity_state(...)`. Because `ObservedEntitySnapshot::to_believed_entity_state(...)` hardcodes `believed_activity: None` in `crates/worldwake-core/src/belief.rs:637`, passive observation alone cannot carry activity facts.
3. `BelievedActivity` and `BelievedEntityState.believed_activity` already exist in live code at `crates/worldwake-core/src/belief.rs:655` and `crates/worldwake-core/src/belief.rs:670`. The original ticket assumption that this field lands "after S35OBSACTSIG-001" is stale; this ticket must not claim ownership of that schema work.
4. `UtilityProfile.activity_awareness_weight` also already exists in live code at `crates/worldwake-core/src/utility_profile.rs:18`. That weighting work is outside this ticket.
5. `ActionInstance` at `crates/worldwake-sim/src/action_instance.rs:6` exposes the exact scheduler facts this ticket needs: `actor`, `def_id`, and `targets`.
6. `ActionDefRegistry::get(...)` in `crates/worldwake-sim/src/action_def_registry.rs:24` returns `ActionDef`, and `ActionDef` carries `domain: ActionDomain` at `crates/worldwake-sim/src/action_def.rs:10`.
7. `PerceptionProfile.observation_fidelity` and the existing `passes_observation_check(...)` gate in `crates/worldwake-systems/src/perception.rs:454` are the live observation contract and should be reused rather than inventing a second activity-specific fidelity path.
8. There is no existing active-action helper or activity-focused perception test in `crates/worldwake-systems/src/perception.rs`; this ticket still represents real missing behavior, not duplicate scope.
9. Current code has one lawful path for this fact after the change: scheduler activity -> perception helper -> belief store. There is no competing lawful path today, and this ticket should not introduce one.
10. Adjacent gaps in `GoalBeliefView`, ranking discounts, and decision traces remain separate follow-up tickets (`S35OBSACTSIG-004` through `-007`) and are not required consequences of this perception-layer change.

## Architecture Check

1. The clean architecture is still a dedicated `observe_active_actions()` helper, not a new special case inside `build_believed_entity_state()` or `ObservedEntitySnapshot`. Activity is scheduler state, not passive snapshot state, so folding it into passive snapshot construction would incorrectly couple core belief snapshots to sim runtime internals.
2. The helper should run after `observe_passive_local_entities()`, because passive observation remains the canonical way to seed or refresh colocated entity beliefs. Activity is then layered onto an already valid colocated belief record.
3. The helper should mutate only `BelievedEntityState.believed_activity`; it should not rewrite unrelated belief fields or create a second belief transport path.
4. The read surface remains minimal and robust: `ActionInstance.actor`, `ActionInstance.targets.first()`, and `ActionDef.domain`. Reaching into `ActionPayload` variants would create brittle cross-module coupling and is not justified by the spec.
5. Clearing activity when the observer can directly see the subject idle or no longer colocated is architecturally better than letting stale activity linger. It preserves the "activity is only valid under current colocated observation" contract without adding TTL heuristics or hidden cleanup passes.
6. No compatibility aliases or fallback fields are warranted. The belief field already exists; this ticket should wire it correctly or fail loudly.

## Verification Layers

1. Scheduler fact projection (`ActionInstance` + `ActionDef.domain` -> `BelievedActivity`) -> focused `worldwake-systems` perception test asserting exact belief contents.
2. Observation gating (`observation_fidelity`) -> focused `worldwake-systems` perception test with `Permille(0)` proving activity does not appear.
3. Stale activity clearing for direct local observation (`idle` / `departed`) -> focused `worldwake-systems` perception tests asserting `believed_activity` becomes `None` while other belief state remains intact.
4. Locality boundary (no cross-place observation, no self-observation) -> focused `worldwake-systems` perception tests.
5. Broader regression surface for this ticket's crate boundary -> `cargo test -p worldwake-systems`.
6. Workspace regression check required by ticket finalization -> `cargo test --workspace` and `cargo clippy --workspace`.

## What to Change

### 1. Remove `_` prefix from `active_actions` and `action_defs` in `perception_system()`

In `crates/worldwake-systems/src/perception.rs`, the destructuring of `SystemExecutionContext` currently uses `_active_actions` and `_action_defs`. Remove the underscore prefixes.

### 2. Add `observe_active_actions()` function

```rust
fn observe_active_actions(
    world: &World,
    tick: Tick,
    rng: &mut DeterministicRng,
    active_actions: &BTreeMap<ActionInstanceId, ActionInstance>,
    action_defs: &ActionDefRegistry,
    updated_stores: &mut BTreeMap<EntityId, AgentBeliefStore>,
)
```

Logic per spec Section 3:
- For each agent with `PerceptionProfile` and a belief store in `updated_stores`:
  - Determine agent's place.
  - Build set of co-located actors with active actions.
  - For each co-located actor (not self): fidelity check -> set `believed_activity`.
  - For co-located actors with NO active action: set `believed_activity = None`.

Correction to original scope:
- Do not recreate `BelievedEntityState` from scratch just to set activity. Reuse the colocated belief snapshots already written by passive observation and update only `believed_activity`.
- If a belief store already contains a colocated subject from earlier observation, the helper may clear stale activity there even when no event-driven update occurred this tick.
- Missing or unknown `ActionDef` entries should be ignored rather than inventing placeholder domains.

### 3. Call from `perception_system()`

Insert call to `observe_active_actions()` after `observe_passive_local_entities()` returns.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify — add function, integrate into pipeline)

## Out of Scope

- `GoalBeliefView` extensions (S35OBSACTSIG-004)
- Ranking discount (S35OBSACTSIG-006)
- `BelievedActivity` type definition and `BelievedEntityState` schema work (already landed; not owned by this ticket)
- `UtilityProfile.activity_awareness_weight` changes (already landed; not owned by this ticket)
- Golden tests (S35OBSACTSIG-007)
- Extending `GoalBeliefView` or AI ranking to consume activity beliefs
- Reworking passive snapshot construction in core to understand runtime scheduler state

## Acceptance Criteria

### Tests That Must Pass

1. `BelievedActivity` set when co-located agent has active action and fidelity check passes.
2. `BelievedActivity` not set when fidelity check fails (fidelity = `Permille(0)`).
3. `BelievedActivity` cleared when observed agent has no active action.
4. `BelievedActivity` cleared when observed agent departs (no longer co-located).
5. Activity observation does not cross place boundaries.
6. Observer does not observe their own activity.
7. Existing suite: `cargo test --workspace`

### Invariants

1. `observe_active_actions()` only reads `ActionDef.domain` and `ActionInstance.targets.first()` — no coupling to `ActionPayload` (P24).
2. Observation is gated by `observation_fidelity` (P20).
3. Only co-located agents are observable (P7 — locality).
4. `believed_activity` is only modified through perception, never directly by other systems.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` — `co_located_active_action_sets_believed_activity`
Rationale: proves the main scheduler-to-belief projection with exact domain / target / observed tick values.
2. `crates/worldwake-systems/src/perception.rs` — `active_action_respects_observation_fidelity_gate`
Rationale: proves activity uses the same perception fidelity contract as other direct observation.
3. `crates/worldwake-systems/src/perception.rs` — `idle_colocated_subject_clears_believed_activity`
Rationale: proves stale visible activity does not persist after direct observation of idleness.
4. `crates/worldwake-systems/src/perception.rs` — `departed_subject_clears_believed_activity_when_no_longer_colocated`
Rationale: proves locality-bound activity is cleared when the observer no longer has colocated line of observation.
5. `crates/worldwake-systems/src/perception.rs` — `active_action_does_not_cross_place_boundaries_or_self_observe`
Rationale: proves the locality and self-observation invariants in the same focused surface.

### Commands

1. `cargo test -p worldwake-systems perception::tests::`
2. `cargo test -p worldwake-systems`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-29
- What actually changed:
  - Added `observe_active_actions()` in `crates/worldwake-systems/src/perception.rs`.
  - Wired `perception_system()` to consume `active_actions` and `action_defs` instead of ignoring them.
  - Projected colocated active scheduler state into `BelievedEntityState.believed_activity` using only `ActionDef.domain` and `ActionInstance.targets.first()`.
  - Cleared stale activity when the observer directly re-observed a colocated subject idle, and when a previously colocated subject was directly noticed as departed.
  - Added focused perception tests covering activity projection, fidelity gating, idle clearing, departure clearing, and locality/self-observation invariants.
- Deviations from original plan:
  - No schema work was needed in `worldwake-core`; `BelievedActivity`, `BelievedEntityState.believed_activity`, and `UtilityProfile.activity_awareness_weight` were already present.
  - The implementation stayed scoped to perception and did not touch `GoalBeliefView`, ranking, traces, or golden behavior-selection tests.
  - The helper uses passive same-tick direct observation as the canonical seed for colocated belief entries instead of reconstructing entity belief snapshots from scheduler data.
- Verification results:
  - `cargo test -p worldwake-systems perception::tests::` passed.
  - `cargo test -p worldwake-systems` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace` passed.
