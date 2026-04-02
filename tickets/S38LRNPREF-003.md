# S38LRNPREF-003: GoalBeliefView trait extension and PerAgentBeliefView implementation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — GoalBeliefView trait in worldwake-sim
**Deps**: S38LRNPREF-001

## Problem

The AI planner and ranking pipeline have no way to read an agent's experience records or preference profile. The `GoalBeliefView` trait must be extended with accessor methods so that ranking (S38LRNPREF-006, 007) can query experience data without violating the belief-only planning invariant (P14).

## Assumption Reassessment (2026-04-02)

1. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:20` — verified. It has 50+ methods across three categories: subjective reads, self-authoritative reads, public structure reads.
2. `PerAgentBeliefView` at `crates/worldwake-sim/src/per_agent_belief_view.rs` implements `GoalBeliefView`. Route experience and preference profile are self-authoritative reads (agent reads its own data).
3. All existing `GoalBeliefView` implementations must be updated: `PerAgentBeliefView` (production), plus test mock implementations in `ranking.rs`, `affordance_query.rs`, `commodity_opportunity.rs`, `trade_valuation.rs`, and potentially others.
4. `RouteExperience`, `SourceReliability`, `PreferenceProfile` exist after S38LRNPREF-001.
5. New methods return `Option<&T>` matching existing patterns like `fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds>`.

## Architecture Check

1. Adding to `GoalBeliefView` (the narrow AI-facing surface) rather than reading components directly maintains the belief-only planning invariant (P14). The agent's experience is its own belief state — self-authoritative.
2. No backward-compatibility shims. New methods with default `None` return allow existing mock implementations to compile without changes if desired, but explicit implementations are preferred for test clarity.

## Verification Layers

1. Trait methods return correct data for agents with experience → focused unit test
2. Trait methods return `None` for agents without experience → focused unit test
3. All GoalBeliefView implementations compile → compile-time verification
4. Single-layer ticket (worldwake-sim trait + impl); no cross-system verification needed.

## What to Change

### 1. GoalBeliefView trait extension

Add to `GoalBeliefView` in `crates/worldwake-sim/src/belief_view.rs`:

```rust
fn route_experience(&self, agent: EntityId) -> Option<&RouteExperience> { None }
fn source_reliability(&self, agent: EntityId) -> Option<&SourceReliability> { None }
fn preference_profile(&self, agent: EntityId) -> Option<&PreferenceProfile> { None }
```

Default implementations return `None` so mock implementations in tests compile without changes.

### 2. PerAgentBeliefView implementation

Implement the three methods in `PerAgentBeliefView` at `crates/worldwake-sim/src/per_agent_belief_view.rs` by reading from the world's component store. These are self-authoritative reads — the agent reads its own components.

### 3. Import new types

Add imports for `RouteExperience`, `SourceReliability`, `PreferenceProfile` in `belief_view.rs` and `per_agent_belief_view.rs`.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — 3 new trait methods with defaults)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — 3 method implementations)

## Out of Scope

- Ranking adjustments that use these methods (S38LRNPREF-006, 007)
- Action handler experience recording (S38LRNPREF-004, 005)
- Updating test mock implementations beyond what's needed for compilation

## Acceptance Criteria

### Tests That Must Pass

1. `PerAgentBeliefView::route_experience` returns `Some` for agent with `RouteExperience` component
2. `PerAgentBeliefView::route_experience` returns `None` for agent without `RouteExperience` component
3. Same for `source_reliability` and `preference_profile`
4. All existing GoalBeliefView mock implementations compile (default `None` return)
5. Existing suite: `cargo test --workspace`

### Invariants

1. `GoalBeliefView` methods are self-authoritative reads — agent reads only its own experience
2. Default trait implementations return `None` (agents without experience are unaffected)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify test module or new focused test) — verify the three new methods return correct data

### Commands

1. `cargo test -p worldwake-sim per_agent_belief_view`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
