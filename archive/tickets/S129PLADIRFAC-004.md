# S129PLADIRFAC-004: ProfileBeliefView hygiene accessors

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — three new agent/profile-facing accessors on `ProfileBeliefView` in `worldwake-sim`; auto-forwarded to `GoalBeliefView` via existing blanket impl
**Deps**: archive/tickets/S129PLADIRFAC-001.md (provides the three components the accessors read), archive/tickets/S129PLADIRFAC-003.md (provides the runtime-only `FacilityBeliefView::wash_basin_state` precondition read)

## Problem

S129's AI ranking integration (D10, ticket 010) reads `PlaceDirtiness`, `LatrineFullness`, `WashBasinState` from co-located places/facilities to score Sleep, Wash, Relieve, and ExploreLocation candidates. Ticket 003 added only the runtime-only `FacilityBeliefView::wash_basin_state(entity) -> Option<WashBasinState>` accessor needed for precondition/affordance filtering. The agent/profile-facing accessors still do not exist, so the AI crate cannot reach all three hygiene states through the same `GoalBeliefView` surface as `place_sleep_quality_profile` (the precedent at `belief_view.rs:770`). Without this ticket, ticket 009 (candidate emission) and ticket 010 (ranking) cannot read the components without bypassing the belief-view abstraction (FND-26 — systems interact through state, not direct accessor leakage).

## Assumption Reassessment (2026-04-29)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `ProfileBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:758` carries the precedent accessor `place_sleep_quality_profile(&self, agent: EntityId, place: EntityId) -> SleepQualityProfile` at line 770. Live correction: the trait default returns the neutral default because the trait has no `world()` accessor; `PerAgentBeliefView` owns the actor-scoped production override. The three new accessors follow that actual split: neutral trait defaults plus a `PerAgentBeliefView` override for FND-14A co-located authoritative reads.
2. `GoalBeliefView` trait at `belief_view.rs:264` is the consumer surface used by AI ranking and candidate generation. The blanket `impl<T> GoalBeliefView for T where T: ... + ProfileBeliefView` at `belief_view.rs:1359` auto-forwards every `ProfileBeliefView` method into `GoalBeliefView` consumers — no manual macro forwarding required.
3. The forwarding-method block at `belief_view.rs:1763` is the blanket `impl<T> GoalBeliefView for T where T: ... + ProfileBeliefView`, not a `RuntimeBeliefView` impl block. It must forward each new `GoalBeliefView` method to `ProfileBeliefView`.
4. The shared abstraction boundary under audit is the FND-14A surface — co-located agents read place/facility physical properties directly from authoritative state. The production override calls `world.get_component_*(entity).copied().unwrap_or_default()` only when `agent == self.agent` and the place/facility is co-located with the actor. For role-specific components (`LatrineFullness`, `WashBasinState`), the `unwrap_or_default()` is correct because callers should already be filtering by tag (the AI candidate emitter checks `PlaceTag::Latrine` / `WorkstationTag::WashBasin` before invoking the accessor).
5. Existing focused/unit coverage: `belief_view.rs`'s inline test module covers `place_sleep_quality_profile` as belief/local-visibility scoped through `PerAgentBeliefView`; new tests follow the same pattern. No golden-level coverage today — that lands in ticket 012.
6. 2026-04-30 live correction from ticket 003: `FacilityBeliefView::wash_basin_state(entity) -> Option<WashBasinState>` now exists for runtime precondition filtering. This ticket must not duplicate that one-argument facility method; it still owns the three `ProfileBeliefView`/`GoalBeliefView` accessors with the explicit `agent` parameter used by AI ranking/candidate-generation code.

## Architecture Check

1. Three accessors as default-method extensions of `ProfileBeliefView` keep the hygiene domain on the same trait surface as the sleep-quality precedent. Splitting into a new `HygieneBeliefView` trait would force ticket 010's ranking code to require both traits and fragment the belief-view surface without architectural benefit. The trait already covers "place/facility profile reads", so hygiene state fits naturally.
2. Default-method implementation (rather than required) means no breaking change to types that already implement `ProfileBeliefView` — they auto-inherit neutral defaults and override only if they need actor-scoped behavior. No backward-compat shim: net-new methods on a trait that already accommodates this pattern.
3. The existing `FacilityBeliefView::wash_basin_state` is intentionally narrower than this ticket's planned `ProfileBeliefView::wash_basin_state(agent, basin)`: the former is a runtime precondition helper, while the latter is the AI-facing actor-scoped hygiene read. If Rust method-name ambiguity appears during implementation, use explicit trait qualification rather than renaming or removing the ticket-003 method.

## Verification Layers

1. The three accessors compile and forward correctly through `RuntimeBeliefView` → `cargo build --workspace` plus a focused unit test in `belief_view.rs` test module that constructs a `RuntimeBeliefView` over a seeded world and asserts each accessor returns the expected component value.
2. Default-on-missing semantics are correct → focused test seeding a place that does NOT carry `LatrineFullness` (i.e., a non-latrine-tagged place) and asserting `latrine_fullness(agent, that_place)` returns `LatrineFullness::default()`. Same for `wash_basin_state` on a non-WashBasin facility.
3. Auto-forwarding to `GoalBeliefView` works → focused test that takes a `T: GoalBeliefView` (e.g., a `RuntimeBeliefView`) and calls `place_dirtiness(...)` through the `GoalBeliefView` trait — proves the blanket impl forwards correctly.

## What to Change

### 1. `crates/worldwake-sim/src/belief_view.rs` — `ProfileBeliefView` trait additions

Inside the `ProfileBeliefView` trait at line 758, add three new default-method accessors after `place_sleep_quality_profile`:

```rust
fn place_dirtiness(&self, agent: EntityId, place: EntityId) -> PlaceDirtiness {
    let _ = (agent, place);
    PlaceDirtiness::default()
}

fn latrine_fullness(&self, agent: EntityId, place: EntityId) -> LatrineFullness {
    let _ = (agent, place);
    LatrineFullness::default()
}

fn wash_basin_state(&self, agent: EntityId, basin: EntityId) -> WashBasinState {
    let _ = (agent, basin);
    WashBasinState::default()
}
```

Match the precedent's signature shape exactly — the `agent: EntityId` parameter is preserved per FND-14A's "co-located perception by this agent" framing even though the default impl doesn't read agent-specific state.

### 2. `crates/worldwake-sim/src/belief_view.rs` — `GoalBeliefView` forwarding block

After the existing `place_sleep_quality_profile` forwarder in the blanket `GoalBeliefView` impl, add three new forwarder methods:

```rust
fn place_dirtiness(&self, agent: EntityId, place: EntityId) -> PlaceDirtiness {
    ProfileBeliefView::place_dirtiness(self, agent, place)
}

fn latrine_fullness(&self, agent: EntityId, place: EntityId) -> LatrineFullness {
    ProfileBeliefView::latrine_fullness(self, agent, place)
}

fn wash_basin_state(&self, agent: EntityId, basin: EntityId) -> WashBasinState {
    ProfileBeliefView::wash_basin_state(self, agent, basin)
}
```

The `GoalBeliefView` trait itself also needs matching default method declarations so downstream code can call the methods through the consumer-facing trait.

### 3. `crates/worldwake-sim/src/per_agent_belief_view.rs` — production actor-scoped overrides

Add `PerAgentBeliefView` overrides for S129 dynamic hygiene state: return default when `agent != self.agent` or the target is not co-located with the actor; otherwise read the corresponding authoritative component and default missing role-specific components. Unlike stable authored sleep-quality profiles, these dynamic hygiene reads must not reveal current remote state solely because the place is known.

### 4. Imports

Add `use crate::PlaceDirtiness;` etc. at the top of `belief_view.rs` (or `use worldwake_core::{PlaceDirtiness, LatrineFullness, WashBasinState};` if the file doesn't already import core types via crate-level alias).

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — trait extensions, blanket `GoalBeliefView` forwarders, focused tests)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — actor-scoped production overrides)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — explicit `FacilityBeliefView::wash_basin_state` qualification after the new actor-scoped method made the one-argument runtime precondition read ambiguous)

## Out of Scope

- Reads from the AI crate (deferred to tickets 009 and 010).
- Belief-store-backed accessors (today's pattern reads authoritative state directly per FND-14A; if S131 or a future spec adds belief-decayed perception for these components, that's a future ticket).
- Off-place propagation via `ShareBelief` (S129 explicitly defers this).

## Acceptance Criteria

### Tests That Must Pass

1. New focused test `place_dirtiness_accessor_returns_authoritative_state` in `belief_view.rs` test module — seeds a Place with `PlaceDirtiness { value: pm(500), ... }`, constructs a `PerAgentBeliefView`, asserts the accessor returns the seeded value through `GoalBeliefView`.
2. New focused test `place_hygiene_accessors_do_not_reveal_known_remote_dynamic_state` — a known but remote place with non-default hygiene components still returns defaults, preserving S129's co-located dynamic-state boundary.
3. New focused test `latrine_fullness_accessor_returns_default_for_unauthored_place` — non-latrine place returns `LatrineFullness::default()` (zero fill).
4. New focused test `wash_basin_state_accessor_returns_default_for_non_basin_facility` — analogous to the latrine test.
5. New focused test `goal_belief_view_forwards_hygiene_accessors` — proves the blanket `impl<T> GoalBeliefView for T where T: ProfileBeliefView` correctly forwards the three new methods.
5. Existing suite: `cargo test -p worldwake-sim` (all preexisting belief-view tests must continue to pass).

### Invariants

1. The three accessors are default-method on `ProfileBeliefView`; no required-method break for existing `ProfileBeliefView` implementors.
2. The accessors return `T::default()` on missing components — matches the sleep-quality precedent and aligns with the universal-on-Place / role-specific-tag-conditional registration pattern.
3. `GoalBeliefView` consumers (the AI crate) reach the new state through the auto-forwarded methods, never by direct world reads.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs` — four new focused tests in the inline test block.

### Commands

1. `cargo test -p worldwake-sim belief_view`
2. `cargo build --workspace`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-30.

- Added `place_dirtiness`, `latrine_fullness`, and actor-scoped `wash_basin_state` methods to `GoalBeliefView` and `ProfileBeliefView`.
- Implemented the production `PerAgentBeliefView` overrides using the S129 FND-14A co-location boundary: neutral defaults for wrong actor, remote target, or missing role-specific component; authoritative reads for co-located place/facility targets.
- Added focused inline tests for seeded `PlaceDirtiness`, default missing `LatrineFullness`, default missing non-basin `WashBasinState`, and `GoalBeliefView` forwarding.
- Added a focused remote-known regression proving dynamic place hygiene defaults rather than leaking current remote authoritative state.
- Qualified the existing runtime precondition read in `affordance_query.rs` as `FacilityBeliefView::wash_basin_state(...)` so ticket 003's one-argument runtime accessor remains distinct from this ticket's actor-scoped accessor.

## Deviations

- Reassessment corrected the drafted `world()`-based default implementation: `ProfileBeliefView` has no world accessor, so neutral trait defaults plus `PerAgentBeliefView` overrides are the honest live seam.
- The drafted "RuntimeBeliefView forwarding block" is actually the blanket `GoalBeliefView` impl; the landed forwarding lives there.

## Verification Result

- Passed `cargo test -p worldwake-sim --lib belief_view::tests::place_dirtiness_accessor_returns_authoritative_state -- --exact`
- Passed `cargo test -p worldwake-sim --lib belief_view::tests::place_hygiene_accessors_do_not_reveal_known_remote_dynamic_state -- --exact`
- Passed `cargo test -p worldwake-sim --lib belief_view::tests::latrine_fullness_accessor_returns_default_for_unauthored_place -- --exact`
- Passed `cargo test -p worldwake-sim --lib belief_view::tests::wash_basin_state_accessor_returns_default_for_non_basin_facility -- --exact`
- Passed `cargo test -p worldwake-sim --lib belief_view::tests::goal_belief_view_forwards_hygiene_accessors -- --exact`
- Passed `cargo test -p worldwake-sim belief_view`
- Passed `cargo build --workspace`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Post-clippy cleanup rerun: passed `cargo test -p worldwake-sim` and `cargo clippy --workspace --all-targets -- -D warnings`
