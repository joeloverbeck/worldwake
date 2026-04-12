# S96OBLSAT-002: BeliefView accessors for satiation components

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new trait methods on GoalBeliefView and ProfileBeliefView, plus runtime override and focused tests on PerAgentBeliefView
**Deps**: archive/tickets/S96OBLSAT-001.md

## Problem

The AI crate's ranking logic reads agent components through the `GoalBeliefView` trait abstraction. Without accessor methods for `ObligationSatiationProfile` and `ObligationExecutionTracker`, the ranking system cannot access satiation state.

## Assumption Reassessment (2026-04-12)

1. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:68+` has existing owned/default-returning accessors for some reads, while profile accessors like `exploration_profile` still delegate through `ProfileBeliefView` and default to `None`.
2. `ProfileBeliefView` at `crates/worldwake-sim/src/belief_view.rs:433+` is implemented by `PerAgentBeliefView`, `PlanningState`, and many AI-side test doubles. Making new methods required here would widen the owned fallout into planner snapshot parity and broad mock cleanup without this ticket yet needing that behavior.
3. The live runtime production implementor that actually reads authoritative world state is `PerAgentBeliefView` in `crates/worldwake-sim/src/per_agent_belief_view.rs:456+`; `RuntimeBeliefView` itself is only a supertrait marker with no method bodies.
4. `PlanningState` implements `ProfileBeliefView` in `crates/worldwake-ai/src/planning_state.rs:1182+`, but `PlanningSnapshot` currently carries only `homeostatic_needs`, `drive_thresholds`, `metabolism_profile`, and `disposal_profile` in `SnapshotProfiles` (`crates/worldwake-ai/src/planning_snapshot.rs:183-188`). This ticket should not widen into snapshot-carriage work.
5. Shared boundary under audit: `GoalBeliefView` blanket forwarding through `ProfileBeliefView`, with `PerAgentBeliefView` as the canonical runtime read surface.

## Architecture Check

1. The cleanest live contract is additive default-returning methods on both `GoalBeliefView` and `ProfileBeliefView`, with only `PerAgentBeliefView` overriding them to read world state. That preserves the existing blanket-forwarding pattern without forcing unrelated planning snapshot or test-double work into this ticket.
2. No backwards-compatibility shims. New methods are additive and preserve existing implementors via default behavior.

## Verification Layers

1. `GoalBeliefView` forwards the new accessors through `ProfileBeliefView` → focused runtime-belief-view test
2. `PerAgentBeliefView` reads correct component data for the actor → focused test
3. Absent runtime tracker falls back to empty default without widening snapshot/mock fallout → focused test
4. Single-layer ticket (belief/read surface only); ranking consumption remains ticket 005.

## What to Change

### 1. Add methods to `GoalBeliefView` trait

Add two methods with default impls:
- `obligation_satiation_profile(&self, agent: EntityId) -> ObligationSatiationProfile` — default returns `ObligationSatiationProfile::default()`
- `obligation_execution_tracker(&self, agent: EntityId) -> ObligationExecutionTracker` — default returns `ObligationExecutionTracker::default()`

Note: these return owned values (not `Option`), matching the universal component pattern where every agent has the component.

### 2. Add methods to `ProfileBeliefView` trait

Add the same two methods to `ProfileBeliefView` with default impls returning
`Default::default()`. This keeps `PlanningState` and existing test doubles
compiling until a later ticket actually needs planner-visible satiation state.

### 3. Implement forwarding in blanket GoalBeliefView impl

In the impl block that delegates GoalBeliefView to ProfileBeliefView (~line 1230 area), add forwarding for both methods.

### 4. Implement on `PerAgentBeliefView`

Read from world store using `get_obligation_satiation_profile` /
`get_obligation_execution_tracker` in
`crates/worldwake-sim/src/per_agent_belief_view.rs`. Use
`unwrap_or_default()` for both owned-value accessors so the universal profile
and runtime-generated tracker expose the correct runtime defaults.

### 5. Keep default-only planning and stub surfaces compiling

No special stub override is required if the `ProfileBeliefView` defaults stay
authoritative, but keep any explicit stub implementation aligned if the local
test surface needs it.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)

## Out of Scope

- Ranking logic that consumes these accessors (ticket 005)
- Component type definitions (ticket 001)

## Acceptance Criteria

### Tests That Must Pass

1. `PerAgentBeliefView` returns the actor's `ObligationSatiationProfile`
2. `PerAgentBeliefView` returns default empty `ObligationExecutionTracker` when absent
3. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. `GoalBeliefView` / `ProfileBeliefView` accessors remain additive and non-breaking for existing implementors
2. `PerAgentBeliefView` is the canonical runtime read surface for both components
3. Planner snapshot carriage remains out of scope for this ticket and therefore still defaults outside runtime reads

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — focused tests for profile/tracker runtime reads and `GoalBeliefView` forwarding

### Commands

1. `cargo test -p worldwake-sim obligation_`
2. `cargo build --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-12.

- Added `obligation_satiation_profile` and `obligation_execution_tracker` accessors to `GoalBeliefView` and `ProfileBeliefView` in `crates/worldwake-sim/src/belief_view.rs`, both returning owned values with additive defaults.
- Forwarded the new accessors through the blanket `GoalBeliefView` impl so existing consumers read them through the same canonical trait path as other profile reads.
- Implemented the runtime override in `crates/worldwake-sim/src/per_agent_belief_view.rs` so the actor reads authoritative `ObligationSatiationProfile` / `ObligationExecutionTracker` values from the world store, with default fallback for missing runtime tracker state.
- Added focused `per_agent_belief_view` tests proving actor profile reads, empty-tracker fallback, and `GoalBeliefView` forwarding.

## Deviations

- Reassessment narrowed the live boundary relative to the original ticket draft: `RuntimeBeliefView` itself is only a marker supertrait, so the real runtime implementation landed on `PerAgentBeliefView`.
- Reassessment also kept `PlanningState` and existing AI-side `ProfileBeliefView` test doubles on inherited defaults instead of widening this ticket into planning-snapshot carriage work.
- The proof surface is stronger than the original compile-only draft: focused runtime-belief-view tests were added because `crates/worldwake-sim/src/per_agent_belief_view.rs` already had the correct harness pattern.

## Verification Result

- Passed `cargo test -p worldwake-sim obligation_`
- Passed `cargo test -p worldwake-sim`
- Passed `cargo build --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Ticket file status: archived as untracked file (`archive/tickets/S96OBLSAT-002.md`); original active path removed
