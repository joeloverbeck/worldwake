# S38LRNPREF-003: Learned-experience belief-surface extension

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — belief trait pair + forwarding in worldwake-sim
**Deps**: S38LRNPREF-001

## Problem

The AI planner and ranking pipeline have no way to read an agent's experience records or preference profile. The canonical AI-facing surface is `GoalBeliefView`, but in live code `PerAgentBeliefView` materializes that surface through the paired `RuntimeBeliefView` trait plus `impl_goal_belief_view!`. The learned-experience accessors must be added across that real forwarding boundary so ranking (S38LRNPREF-006, 007) can query experience data without violating the belief-only planning invariant (P14).

## Assumption Reassessment (2026-04-02)

1. `GoalBeliefView` and `RuntimeBeliefView` both live in `crates/worldwake-sim/src/belief_view.rs` — verified. The live architecture mirrors narrow self-authoritative reads across both traits, then forwards `GoalBeliefView` through `impl_goal_belief_view!`.
2. `PerAgentBeliefView` at `crates/worldwake-sim/src/per_agent_belief_view.rs` does not implement `GoalBeliefView` directly. It implements `RuntimeBeliefView`, and `crate::impl_goal_belief_view!(PerAgentBeliefView<'_>)` materializes the canonical `GoalBeliefView` surface.
3. Existing precedent is `commodity_valuation_profile`: default `None` on both traits, concrete implementation on `PerAgentBeliefView` under `RuntimeBeliefView`, and focused tests proving both the runtime and goal-facing calls.
4. `RouteExperience`, `SourceReliability`, and `PreferenceProfile` exist after S38LRNPREF-001.
5. These are self-authoritative reads: the actor may read only its own learned-experience components.

## Architecture Check

1. The canonical consumer boundary remains `GoalBeliefView`, but the live implementation boundary also includes `RuntimeBeliefView` and the `impl_goal_belief_view!` forwarding macro. The ticket must land on that full boundary, not just the narrow trait surface.
2. Adding these reads to the belief traits rather than reading components directly in AI code maintains the belief-only planning invariant (P14). The agent's experience is self-authoritative state.
3. No backward-compatibility shims. Default `None` methods on both traits preserve compileability for mocks that do not care about this surface, while `PerAgentBeliefView` provides the production implementation.

## Verification Layers

1. `RuntimeBeliefView::{route_experience, source_reliability, preference_profile}` return correct actor-local data when present → focused unit test
2. The same runtime methods return `None` when components are absent → focused unit test
3. `GoalBeliefView` resolves the same values through the forwarding macro → focused unit test
4. Existing mock implementations continue to compile because both traits provide default `None`
5. Single-crate ticket (`worldwake-sim` trait/macro/runtime impl); no cross-system verification needed.

## What to Change

### 1. Belief trait-pair extension

Add the three learned-experience accessors to both `GoalBeliefView` and `RuntimeBeliefView` in `crates/worldwake-sim/src/belief_view.rs`, following the existing `commodity_valuation_profile` pattern:

```rust
fn route_experience(&self, agent: EntityId) -> Option<RouteExperience> { None }
fn source_reliability(&self, agent: EntityId) -> Option<SourceReliability> { None }
fn preference_profile(&self, agent: EntityId) -> Option<PreferenceProfile> { None }
```

Also add the corresponding forwarding arms to `impl_goal_belief_view!`.

### 2. PerAgentBeliefView implementation

Implement the three methods in `PerAgentBeliefView` under `impl RuntimeBeliefView for PerAgentBeliefView<'_>` by reading from the world's component store. These are self-authoritative reads, so they must only return values for `agent == self.agent`.

### 3. Import new types

Add imports for `RouteExperience`, `SourceReliability`, and `PreferenceProfile` in `belief_view.rs` and `per_agent_belief_view.rs`.

### 4. Focused forwarding proof

Extend the focused tests in `crates/worldwake-sim/src/per_agent_belief_view.rs` to prove both:
- the direct `RuntimeBeliefView` path returns the expected values
- the canonical `GoalBeliefView` path forwards the same values

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — trait-pair defaults + forwarding macro)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — runtime implementations + focused tests)

## Out of Scope

- Ranking adjustments that use these methods (S38LRNPREF-006, 007)
- Action handler experience recording (S38LRNPREF-004, 005)
- Updating test mock implementations beyond what compileability requires

## Acceptance Criteria

### Tests That Must Pass

1. `RuntimeBeliefView::route_experience` returns `Some` for the actor with a `RouteExperience` component
2. `RuntimeBeliefView::route_experience` returns `None` when the component is absent
3. Same for `source_reliability` and `preference_profile`
4. `GoalBeliefView::{route_experience, source_reliability, preference_profile}` return the same values through the forwarding path
5. Existing mock implementations compile via the default `None` methods
6. Existing suite: `cargo test --workspace`

### Invariants

1. These are self-authoritative reads: the actor reads only its own learned-experience components
2. `GoalBeliefView` remains the canonical AI-facing surface
3. Default trait implementations return `None` (agents without experience or mocks without overrides are unaffected)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify focused tests) — verify present/absent cases for all three methods across both `RuntimeBeliefView` and `GoalBeliefView`

### Commands

1. `cargo test -p worldwake-sim experience`
2. `cargo test -p worldwake-sim per_agent_belief_view`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completed: 2026-04-02
- What changed:
  - Extended both `GoalBeliefView` and `RuntimeBeliefView` with `route_experience`, `source_reliability`, and `preference_profile`
  - Added the corresponding forwarding arms to `impl_goal_belief_view!`
  - Implemented the new self-authoritative reads on `PerAgentBeliefView` via the live `RuntimeBeliefView` boundary
  - Added focused present/absent tests proving both the runtime path and the canonical goal-facing forwarding path
- Deviations from original plan:
  - Corrected the ticket before implementation to match the live trait-pair and macro-forwarding architecture instead of the stale direct `PerAgentBeliefView -> GoalBeliefView` assumption
- Verification results:
  - `cargo test -p worldwake-sim experience -- --nocapture`
  - `cargo test -p worldwake-sim per_agent_belief_view -- --nocapture`
  - `cargo test -p worldwake-sim`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
