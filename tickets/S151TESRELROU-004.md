# S151TESRELROU-004: GoalBeliefView accessors for new universal profiles

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `GoalBeliefView` trait extension in `worldwake-sim`
**Deps**: archive/tickets/S151TESRELROU-003.md

## Problem

S151's ranking damping (ticket 007) and travel-cost integration (ticket 008) read the new `TestimonyTrustProfile` and `RoutePreferenceProfile` universal components during AI planning. Per the "New Component Read by AI Crate" pattern, AI-crate consumers MUST read universal-profile components through the `GoalBeliefView` trait surface (not via direct ECS reads), so the trait gets two new accessor methods backed by `RuntimeBeliefView` and forwarded by the existing `impl_goal_belief_view!` macro.

## Assumption Reassessment (2026-05-17)

1. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:317` extends `BelievedAuthorityView + LocalPhysicalObservationView`. The trait carries the AI-side belief-and-profile read surface; `MetabolismProfile`, `CognitiveProfile`, and similar universal profiles are accessed through it.
2. Both new components (`TestimonyTrustProfile`, `RoutePreferenceProfile`) are registered as universal on `EntityKind::Agent` by ticket 003, with default impls and bootstrap seeding in `World::create_agent()`. Runtime reads on known agents may safely use `expect()` (per Section 5 universal-profile contract).
3. Shared boundary under audit: this ticket bridges the core-side component registration (ticket 003) and the AI-crate consumers (tickets 007, 008). The trait shape determines what those consumers see.

## Architecture Check

1. Per FND-26: state-mediated cross-system reads. AI never directly queries the world for these profiles; the trait abstracts the access path so ai-crate code remains testable against mock belief views.
2. `expect()` over `Option<&Component>` is correct per the Section 5 universal-profile contract — every known agent has both profiles seeded by `World::create_agent()` (ticket 003).
3. Trait extension is additive — no existing accessor changes shape, no existing consumer breaks.

## Verification Layers

1. Trait method existence and signature → unit test in `worldwake-sim` that constructs a `RuntimeBeliefView` and calls each new accessor.
2. Macro forwarding correctness → existing AI-side consumers (mocked or shimmed in tests) compile against the trait without changes to call sites.
3. Single-layer ticket — this is a pure surface-extension; downstream behavior lives in tickets 007 and 008.

## What to Change

### 1. Extend `GoalBeliefView` trait (`crates/worldwake-sim/src/belief_view.rs`)

Add two new methods on the trait alongside the existing universal-profile accessors:

```rust
fn testimony_trust_profile(&self, agent: EntityId) -> &TestimonyTrustProfile;
fn route_preference_profile(&self, agent: EntityId) -> &RoutePreferenceProfile;
```

### 2. Backing `RuntimeBeliefView` impl

Implement both methods to read via the existing `get_component_*` macro-generated accessors and `expect()` on known agents:

```rust
fn testimony_trust_profile(&self, agent: EntityId) -> &TestimonyTrustProfile {
    self.world
        .get_component_testimony_trust_profile(agent)
        .expect("agent should have testimony_trust_profile (universal per S151)")
}
```

Mirror the existing `metabolism_profile` / `cognitive_profile` impl style.

### 3. Forward via `impl_goal_belief_view!` macro

Add forwarding entries for both new methods so downstream blanket impls (and any wrapper belief views in tests) inherit them automatically. Find the macro at the canonical existing universal-profile forwarding site in `belief_view.rs` and extend its arm list.

### 4. Re-imports

Ensure `TestimonyTrustProfile` and `RoutePreferenceProfile` are imported in `belief_view.rs`. The `crate::worldwake_core::*` (or equivalent) star-import will likely pick them up after ticket 003's re-exports land; verify during implementation.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — trait + impl + macro)

## Out of Scope

- AI-side consumer reads (ranking damping in ticket 007, travel cost in ticket 008)
- Profile registration and bootstrap (ticket 003)
- Runtime store reads (`TestimonyReliability`/`RoutePreference` live on `AgentDecisionRuntime`, accessed directly from the ai-crate runtime structure, not via `GoalBeliefView`)

## Acceptance Criteria

### Tests That Must Pass

1. `RuntimeBeliefView::testimony_trust_profile(agent_id)` returns the same `TestimonyTrustProfile` that `create_agent` seeded with default values.
2. `RuntimeBeliefView::route_preference_profile(agent_id)` returns the seeded default `RoutePreferenceProfile`.
3. Both accessors `expect()` succeed on every agent created by `World::create_agent()`.
4. Existing suite: `cargo test --workspace`.

### Invariants

1. `GoalBeliefView` accessors remain pure read functions — no mutation, no side effects.
2. Every agent created by `World::create_agent()` satisfies both accessor `expect()` calls.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs#[cfg(test)]` — unit tests asserting the two new accessors return the expected `Default`-seeded profile for a freshly-created agent.

### Commands

1. `cargo test -p worldwake-sim belief_view`
2. `cargo test -p worldwake-ai` (ensure ai-crate consumers compile cleanly after trait extension; they may not use the methods yet)
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `./scripts/verify.sh`
