# S35OBSACTSIG-004: Extend belief views with activity query methods

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` belief-view traits and belief-backed runtime implementations; `worldwake-ai` planning snapshot/runtime implementation
**Deps**: `specs/S35-observable-activity-signals.md`

## Problem

The observable-activity spec now relies on belief-mediated activity reads during ranking, but the current belief-view surfaces expose only raw `known_entity_beliefs()` cloning. There is no dedicated query for "what activity do I believe this entity is doing?" or "which believed agents are active here in this domain?" That forces higher layers to either duplicate belief-store filtering logic or stay blind to observed competition.

## Assumption Reassessment (2026-03-29)

1. The live spec is [`specs/S35-observable-activity-signals.md`](/home/joeloverbeck/projects/worldwake/specs/S35-observable-activity-signals.md), not the non-existent `specs/S35-observable-action-signatures.md` / `specs/S35-observable-action-signalization.md` paths referenced in earlier drafts.
2. `BelievedActivity` and `BelievedEntityState.believed_activity` already exist in [`crates/worldwake-core/src/belief.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs), so this ticket is no longer blocked on S35OBSACTSIG-001 and must consume the landed belief shape directly.
3. The perception-side projection already exists in [`crates/worldwake-systems/src/perception.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/perception.rs), including focused tests for set/clear behavior. This ticket remains a read-surface ticket, not a perception ticket.
4. The exact shared abstraction boundary under audit is the `GoalBeliefView` / `RuntimeBeliefView` contract in [`crates/worldwake-sim/src/belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs). Ranking consumes `GoalBeliefView`, but the macro-backed implementation path goes through `RuntimeBeliefView`.
5. `GoalBeliefView` currently has no activity-specific query methods. `RuntimeBeliefView` currently has no activity-specific query methods. The macro `impl_goal_belief_view!` delegates from the former to the latter, so the contract must be extended in one coherent place rather than patched per consumer.
6. `PerAgentBeliefView` in [`crates/worldwake-sim/src/per_agent_belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs) is one belief-backed runtime implementation, but not the only one that matters architecturally. `PlanningState` in [`crates/worldwake-ai/src/planning_state.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs) also implements `RuntimeBeliefView` and snapshots `actor_known_entity_beliefs`, so excluding snapshot/runtime planning views would leave the contract internally inconsistent.
7. There is no live [`crates/worldwake-sim/src/omniscient_belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/omniscient_belief_view.rs) file. The previous ticket version invented a fallback implementation path that does not exist in current code and should be removed from scope.
8. `known_entity_beliefs()` already returns cloned `BelievedEntityState` values. That means `agents_active_at()` can be a derived query over belief snapshots, but `believed_activity_of()` cannot be retrofitted as a borrowed default on `GoalBeliefView` alone without either cloning or widening trait obligations. The clean approach is to add explicit methods to `RuntimeBeliefView`, give them default no-op implementations for non-belief-backed test stubs, and override them in belief-backed runtimes.
9. Coverage gap today: `cargo test -p worldwake-sim -- --list` shows focused `per_agent_belief_view` tests and perception tests, but no test names covering `believed_activity_of` / `agents_active_at` because those methods do not exist yet. There is likewise no focused `PlanningState` test proving snapshot parity for activity beliefs.
10. Adjacent contradiction exposed during reassessment: the old ticket said snapshot-level belief views were out of scope, but the live architecture uses `PlanningState` as a `RuntimeBeliefView` implementation behind the same macro-based `GoalBeliefView` surface. That contradiction is a required consequence of this ticket and is corrected in-scope here, not deferred.

## Architecture Check

1. The clean architecture is to make activity queries first-class belief-view operations rather than re-scanning `known_entity_beliefs()` ad hoc in ranking. That keeps the AI dependent on one stable read contract instead of on the storage layout of belief snapshots.
2. Extending `RuntimeBeliefView` with default method bodies is cleaner than forcing every test stub in the workspace to fabricate activity storage. Belief-backed runtimes (`PerAgentBeliefView`, `PlanningState`) override the methods; lightweight test doubles continue to compile unless a test actually needs activity behavior.
3. `agents_active_at()` is a derived query, not stored state. That preserves concrete state in `BelievedEntityState` while centralizing the filtering rule in one place.
4. No backwards-compatibility aliasing or duplicate access path should be introduced. The new API becomes the canonical way to ask for believed activity; callers should not add parallel helper scans once it exists.

## Verification Layers

1. `PerAgentBeliefView` returns believed activity for known entities and filters competitors by place/domain/target -> focused unit tests in `worldwake-sim`
2. `PlanningState` preserves the same activity query semantics from the captured belief snapshot -> focused unit tests in `worldwake-ai`
3. Trait/macro integration remains compile-correct across existing runtime stubs and macro users -> targeted crate test runs for `worldwake-sim` and `worldwake-ai`
4. This is a belief-query contract ticket, not an action-lifecycle or event-ordering ticket, so action trace / event-log mapping is not the proof surface here

## What to Change

### 1. Extend belief-view traits

In [`crates/worldwake-sim/src/belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs):

- add `believed_activity_of(&self, entity: EntityId) -> Option<&BelievedActivity>`
- add `agents_active_at(&self, place: EntityId, domain: ActionDomain, target: Option<EntityId>) -> Vec<EntityId>`
- add the same methods to `RuntimeBeliefView`
- provide `RuntimeBeliefView` defaults of `None` / `Vec::new()` so existing test stubs do not need fake activity support unless a test relies on it
- update `impl_goal_belief_view!` to delegate both methods

### 2. Implement belief-backed runtime behavior

Implement the new methods in:

- [`crates/worldwake-sim/src/per_agent_belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs)
- [`crates/worldwake-ai/src/planning_state.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs)

Rules:

- `believed_activity_of()` reads the stored `BelievedEntityState.believed_activity` for known non-self entities
- `agents_active_at()` derives a fresh `Vec<EntityId>` by filtering believed entities whose `last_known_place`, `believed_activity.action_domain`, and optional `believed_activity.target` match
- results must be deterministic and deduplicated
- self should not be synthesized into subjective activity belief queries unless already present in the underlying belief snapshot

### 3. Add focused tests before/with implementation

Add focused coverage for:

- activity lookup present / absent in `PerAgentBeliefView`
- place/domain/target filtering in `PerAgentBeliefView`
- snapshot parity for the same queries in `PlanningState`

## Files to Touch

- [`crates/worldwake-sim/src/belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) (modify)
- [`crates/worldwake-sim/src/per_agent_belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs) (modify)
- [`crates/worldwake-ai/src/planning_state.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs) (modify)

## Out of Scope

- Perception logic that populates `BelievedActivity`
- Ranking arithmetic and `CompetitionDiscount` behavior
- `UtilityProfile.activity_awareness_weight`
- Save/load changes for `BelievedActivity`
- Any broader refactor of how belief snapshots are stored internally

## Acceptance Criteria

### Tests That Must Pass

1. `PerAgentBeliefView::believed_activity_of()` returns the stored activity for a known observed entity and `None` when absent or unknown.
2. `PerAgentBeliefView::agents_active_at()` filters believed entities by place, `ActionDomain`, and optional target.
3. `PlanningState` exposes the same activity-query results from its captured belief snapshot.
4. Existing suites: `cargo test -p worldwake-sim` and `cargo test -p worldwake-ai`

### Invariants

1. Activity queries are pure belief reads and never mutate belief state.
2. `agents_active_at()` remains derived from belief snapshots; no duplicate stored summary is introduced.
3. The canonical AI-facing activity query path is the belief-view contract, not ad hoc scans in downstream ranking code.

## Test Plan

### New/Modified Tests

1. [`crates/worldwake-sim/src/per_agent_belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs) — prove belief-backed runtime queries return activity correctly and filter competitors deterministically.
2. [`crates/worldwake-ai/src/planning_state.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs) — prove planning snapshots preserve the same activity-query contract as runtime belief views.

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-29
- Actual changes:
  - Extended `GoalBeliefView` and `RuntimeBeliefView` with `believed_activity_of()` and `agents_active_at()`
  - Added default no-op trait implementations so existing runtime test doubles did not need fake activity storage
  - Implemented belief-backed activity queries in `PerAgentBeliefView`
  - Implemented the same query surface in `PlanningState` so planning snapshots stay consistent with runtime belief views
  - Added focused tests in `worldwake-sim` and `worldwake-ai` for lookup and filtering behavior
- Deviations from original plan:
  - Replaced the stale “omniscient belief view” assumption with the live `PlanningState` runtime implementation
  - Corrected the ticket’s dependency/spec assumptions because `BelievedActivity` and perception-side projection had already landed
- Verification results:
  - `cargo test -p worldwake-sim` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
