# S107PRODIV-003: GoalBeliefView accessors for diversification state

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new GoalBeliefView trait methods and test-belief-view wiring
**Deps**: archive/tickets/S107PRODIV-001.md

## Problem

The AI crate needs to read `DiversificationProfile` and `LastProactiveExplorationTick` during candidate generation. Per established pattern, this requires GoalBeliefView accessors in worldwake-sim plus corresponding test-belief-view wiring for AI-focused tests.

## Assumption Reassessment (2026-04-17)

1. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:70-417`. Existing accessor pattern: `fn exploration_profile(&self, agent: EntityId) -> Option<ExplorationProfile>` at line 200. `RuntimeBeliefView` at line 864. `PerAgentBeliefView` at `per_agent_belief_view.rs:45`.
2. `AgentBeliefStore.place_visits`, omitted-field serde compatibility, and `BeliefStoreDiff` support already landed in `S107PRODIV-001`; this ticket no longer owns core belief-store shape changes.
3. `agent_belief_store` accessor exists on GoalBeliefView (line 91) returning `Option<&AgentBeliefStore>` — the AI crate can already access `place_visits` through this existing accessor. Dedicated `diversification_profile` and `last_proactive_exploration_tick` accessors are still needed for component access.
4. Live trait fallout is broader than the original draft implied. Multiple AI-side `TestBeliefView`/`MockView` harnesses implement `GoalBeliefView` directly or indirectly through `RuntimeBeliefView`, so compile fallout may touch focused AI test files beyond `candidate_generation.rs` even though the production boundary remains the same.

## Architecture Check

1. Follows established GoalBeliefView accessor pattern (same as ExplorationProfile, PerceptionProfile, etc.). Component access through the belief view layer maintains crate boundaries (AI crate cannot directly access ECS store).
2. No backward-compatibility shims. This ticket now narrows to read-surface wiring only because the shared belief-store field already landed in ticket 001.

## Verification Layers

1. GoalBeliefView accessors return correct values → focused unit test with TestBeliefView
2. AI test-belief-view wiring exposes the new accessors to candidate-generation tests → focused unit test or existing AI test infrastructure proof
3. Single-layer ticket: accessor wiring only, no behavioral logic

## What to Change

### 1. Add GoalBeliefView accessors

In `crates/worldwake-sim/src/belief_view.rs`, add to the appropriate trait(s):
```rust
fn diversification_profile(&self, agent: EntityId) -> Option<DiversificationProfile>;
fn last_proactive_exploration_tick(&self, agent: EntityId) -> Option<Tick>;
```

### 2. Implement in RuntimeBeliefView and PerAgentBeliefView

Add component lookups delegating to the ECS store, following the existing `exploration_profile` pattern.

### 3. Update TestBeliefView

In test infrastructure, add fields and trait impl for the new accessors so AI crate tests can mock these values. Treat `candidate_generation.rs` as the primary harness named by the spec, then fix any additional compile-fallout harnesses that implement the same trait surface.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify) — add 2 trait methods + impls
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify) — implement forwarding
- `crates/worldwake-ai/src/candidate_generation.rs` (modify) — add fields to TestBeliefView if defined there
- `crates/worldwake-ai/src/*` focused test harness files as compile fallout demands — update any `GoalBeliefView` test doubles that must satisfy the new accessor surface

## Out of Scope

- PlaceVisitRecord update logic (ticket 004)
- Proactive candidate emission (ticket 006)
- CLI wiring (ticket 005)
- AgentBeliefStore field shape, serde compatibility, and BeliefStoreDiff support (already delivered by ticket 001)

## Acceptance Criteria

### Tests That Must Pass

1. GoalBeliefView::diversification_profile returns Some when component is set, None otherwise
2. GoalBeliefView::last_proactive_exploration_tick returns correct Option<Tick>
3. Existing suite: `cargo test -p worldwake-sim`
4. Existing suite: `cargo test -p worldwake-ai --lib --no-run`
5. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. GoalBeliefView accessors follow the established component-lookup pattern
2. AI-side access to `place_visits` continues through the existing `agent_belief_store` accessor, not a duplicate dedicated method
3. No behavioral changes — accessors are wiring only
4. Any AI test-harness fallout is trait-surface maintenance, not a scope expansion into proactive behavior

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs` — accessor tests with mock data
2. `crates/worldwake-ai/src/candidate_generation.rs` — TestBeliefView wiring coverage if that harness owns the new accessor surface
3. Additional AI harness files only if compile fallout requires trait-impl updates

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test -p worldwake-ai --lib --no-run`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-17.

- Added `diversification_profile` and `last_proactive_exploration_tick` to both `GoalBeliefView` and `ProfileBeliefView` in [`crates/worldwake-sim/src/belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs), and wired the runtime blanket impl through the existing profile boundary.
- Implemented authoritative forwarding in [`crates/worldwake-sim/src/per_agent_belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs) so the acting agent can read `DiversificationProfile` directly and `LastProactiveExplorationTick` as `Option<Tick>`.
- Updated the primary AI test harness in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) with concrete storage and trait wiring for the new accessor pair.
- Reassessment proved the wider AI fallout stayed compile-only for this slice; no additional harness files required edits after the runtime blanket impl was updated.

## Verification Result

- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-sim per_agent_goal_belief_view_exposes_diversification_components`
- Passed `cargo test -p worldwake-ai test_belief_view_exposes_diversification_accessors`
- Passed `cargo test -p worldwake-sim`
- Passed `cargo test -p worldwake-ai --lib --no-run`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
