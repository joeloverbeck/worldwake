# S151TESRELROU-004: GoalBeliefView accessors for new universal profiles

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `GoalBeliefView` trait extension in `worldwake-sim`
**Deps**: archive/tickets/S151TESRELROU-003.md

## Problem

S151's ranking damping (ticket 007) and travel-cost integration (ticket 008) read the new `TestimonyTrustProfile` and `RoutePreferenceProfile` universal components during AI planning. Per the "New Component Read by AI Crate" pattern, AI-crate consumers MUST read universal-profile components through the `GoalBeliefView` trait surface (not via direct ECS reads), so the trait gets two new accessor methods backed by `RuntimeBeliefView` and forwarded by the existing `impl_goal_belief_view!` macro.

## Assumption Reassessment (2026-05-17)

1. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:317` extends `BelievedAuthorityView + LocalPhysicalObservationView`. The trait carries the AI-side belief-and-profile read surface; `MetabolismProfile`, `CognitiveProfile`, and similar universal profiles are accessed through it.
2. Both S151 components (`TestimonyTrustProfile`, `RoutePreferenceProfile`) are registered as universal on `EntityKind::Agent` by ticket 003, with default impls and bootstrap seeding in `World::create_agent()`.
3. Shared boundary under audit: this ticket bridges the core-side component registration (ticket 003) and the AI-crate consumers (tickets 007, 008). The trait shape determines what those consumers see.
4. Live profile accessors in `GoalBeliefView` / `ProfileBeliefView` return `Option<T>` by value rather than borrowed references. The S151 accessors follow that established pattern instead of introducing one-off `&Profile` methods with `expect()` semantics.

## Architecture Check

1. Per FND-26: state-mediated cross-system reads. AI never directly queries the world for these profiles; the trait abstracts the access path so ai-crate code remains testable against mock belief views.
2. `Option<T>` matches the existing profile read contract and keeps test doubles lawful while still letting `PerAgentBeliefView` expose the seeded universal components for the active actor.
3. Trait extension is additive — no existing accessor changes shape, no existing consumer breaks.

## Verified Layers

1. Trait method existence and signature → unit test in `worldwake-sim` that constructs a `RuntimeBeliefView` and calls each new accessor.
2. Macro forwarding correctness → existing AI-side consumers (mocked or shimmed in tests) compile against the trait without changes to call sites.
3. Single-layer ticket — this is a pure surface-extension; downstream behavior lives in tickets 007 and 008.

## Landed Changes

### 1. Extended `GoalBeliefView` and `ProfileBeliefView`

Added two new methods alongside the existing universal-profile accessors:

```rust
fn testimony_trust_profile(&self, agent: EntityId) -> Option<TestimonyTrustProfile>;
fn route_preference_profile(&self, agent: EntityId) -> Option<RoutePreferenceProfile>;
```

### 2. Backed `PerAgentBeliefView` reads

Implemented both methods on `PerAgentBeliefView` through the existing macro-generated component accessors:

```rust
fn testimony_trust_profile(&self, agent: EntityId) -> Option<TestimonyTrustProfile> {
    (agent == self.agent)
        .then(|| self.world.get_component_testimony_trust_profile(agent).cloned())
        .flatten()
}
```

The implementation mirrors the existing self-scoped profile accessors.

### 3. Forwarded through the blanket `GoalBeliefView` impl

Added forwarding entries for both methods so downstream blanket impls and wrapper belief views inherit the S151 profile reads automatically.

### 4. Added focused proof

Added `runtime_belief_view_s151_profile_accessors_return_seeded_defaults` in `crates/worldwake-sim/src/belief_view.rs`.

## Landed Files

- `crates/worldwake-sim/src/belief_view.rs` (modify — trait + impl + macro)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — runtime backing reads)

## Out of Scope

- AI-side consumer reads (ranking damping in ticket 007, travel cost in ticket 008)
- Profile registration and bootstrap (ticket 003)
- Runtime store reads (`TestimonyReliability`/`RoutePreference` live on `AgentDecisionRuntime`, accessed directly from the ai-crate runtime structure, not via `GoalBeliefView`)

## Acceptance Result

### Proved Acceptance Criteria

1. `GoalBeliefView::testimony_trust_profile(&view, agent_id)` returns the `TestimonyTrustProfile::default()` seeded by `World::create_agent()`.
2. `GoalBeliefView::route_preference_profile(&view, agent_id)` returns the `RoutePreferenceProfile::default()` seeded by `World::create_agent()`.
3. The live accessor contract returns `Some(default)` for the actor seeded by `World::create_agent()`; this supersedes the drafted `expect()` wording.
4. Existing suite passed via `cargo test --workspace --quiet`.

### Invariants

1. `GoalBeliefView` accessors remain pure read functions — no mutation, no side effects.
2. Every agent created by `World::create_agent()` receives both S151 universal profile components, and `PerAgentBeliefView` exposes them for the actor through the standard `Option<T>` profile-read shape.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs#[cfg(test)]` — added `runtime_belief_view_s151_profile_accessors_return_seeded_defaults`.

## Outcome

Completed on 2026-05-17.

- Added S151 universal profile accessors to `GoalBeliefView` and `ProfileBeliefView`.
- Backed those accessors from `PerAgentBeliefView` using the seeded `TestimonyTrustProfile` and `RoutePreferenceProfile` components from ticket 003.
- Added focused `worldwake-sim` coverage proving a freshly created agent exposes both default profile components through `GoalBeliefView`.
- Deviated from the draft by using the established `Option<T>` profile-accessor shape instead of borrowed references with `expect()`; downstream callers can unwrap or default in the consuming ticket that owns behavior.

## Verification Result

- Passed `cargo fmt --all`.
- Passed `cargo test -p worldwake-sim --lib belief_view::tests::runtime_belief_view_s151_profile_accessors_return_seeded_defaults -- --exact`.
- Passed `cargo test -p worldwake-sim belief_view`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo test --workspace --quiet`.
- Passed `cargo fmt --all -- --check`.
- Passed `bash scripts/check_active_goal_removed.sh`.
- Passed `bash scripts/check_no_artifact_state.sh`.
- Passed `bash scripts/check_no_debug_view_in_ai.sh`.
- Passed `cargo clippy --workspace`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
- Waived direct `./scripts/verify.sh` invocation because every live `scripts/verify.sh` gate was run individually after inspecting the wrapper.
