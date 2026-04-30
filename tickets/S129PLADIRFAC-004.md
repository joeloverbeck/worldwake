# S129PLADIRFAC-004: ProfileBeliefView hygiene accessors

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — three new agent/profile-facing accessors on `ProfileBeliefView` in `worldwake-sim`; auto-forwarded to `GoalBeliefView` via existing blanket impl
**Deps**: archive/tickets/S129PLADIRFAC-001.md (provides the three components the accessors read), archive/tickets/S129PLADIRFAC-003.md (provides the runtime-only `FacilityBeliefView::wash_basin_state` precondition read)

## Problem

S129's AI ranking integration (D10, ticket 010) reads `PlaceDirtiness`, `LatrineFullness`, `WashBasinState` from co-located places/facilities to score Sleep, Wash, Relieve, and ExploreLocation candidates. Ticket 003 added only the runtime-only `FacilityBeliefView::wash_basin_state(entity) -> Option<WashBasinState>` accessor needed for precondition/affordance filtering. The agent/profile-facing accessors still do not exist, so the AI crate cannot reach all three hygiene states through the same `GoalBeliefView` surface as `place_sleep_quality_profile` (the precedent at `belief_view.rs:770`). Without this ticket, ticket 009 (candidate emission) and ticket 010 (ranking) cannot read the components without bypassing the belief-view abstraction (FND-26 — systems interact through state, not direct accessor leakage).

## Assumption Reassessment (2026-04-29)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `ProfileBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:758` carries the precedent accessor `place_sleep_quality_profile(&self, agent: EntityId, place: EntityId) -> SleepQualityProfile` at line 770. The default-method impl (rather than a required method) was chosen by S128 because every place implicitly carries the profile (universal-on-Place pattern). Three new accessors follow the same pattern.
2. `GoalBeliefView` trait at `belief_view.rs:264` is the consumer surface used by AI ranking and candidate generation. The blanket `impl<T> GoalBeliefView for T where T: ... + ProfileBeliefView` at `belief_view.rs:1359` auto-forwards every `ProfileBeliefView` method into `GoalBeliefView` consumers — no manual macro forwarding required.
3. `RuntimeBeliefView` at `belief_view.rs:1289` is the production-runtime impl; its forwarding-method block at line 1763 (`fn place_sleep_quality_profile(&self, ...) -> SleepQualityProfile { ProfileBeliefView::place_sleep_quality_profile(self, agent, place) }`) is the parallel for each new accessor.
4. The shared abstraction boundary under audit is the FND-14A surface — co-located agents read place/facility properties directly from authoritative state. The accessors call `world.get_component_*(entity).copied().unwrap_or_default()` (mirroring the sleep-quality default-on-missing pattern); for role-specific components (`LatrineFullness`, `WashBasinState`), the `unwrap_or_default()` is correct because callers should already be filtering by tag (the AI candidate emitter checks `PlaceTag::Latrine` / `WorkstationTag::WashBasin` before invoking the accessor).
5. Existing focused/unit coverage: `belief_view.rs`'s inline test module (locate during implementation; likely after the trait definitions) covers `place_sleep_quality_profile` round-trip; new tests follow the same pattern. No golden-level coverage today — that lands in ticket 012.
6. 2026-04-30 live correction from ticket 003: `FacilityBeliefView::wash_basin_state(entity) -> Option<WashBasinState>` now exists for runtime precondition filtering. This ticket must not duplicate that one-argument facility method; it still owns the three `ProfileBeliefView`/`GoalBeliefView` accessors with the explicit `agent` parameter used by AI ranking/candidate-generation code.

## Architecture Check

1. Three accessors as default-method extensions of `ProfileBeliefView` keep the hygiene domain on the same trait surface as the sleep-quality precedent. Splitting into a new `HygieneBeliefView` trait would force ticket 010's ranking code to require both traits and fragment the belief-view surface without architectural benefit. The trait already covers "place/facility profile reads", so hygiene state fits naturally.
2. Default-method implementation (rather than required) means no breaking change to types that already implement `ProfileBeliefView` — they auto-inherit the new methods and override only if they need bespoke behavior. No backward-compat shim: net-new methods on a trait that already accommodates this pattern.
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
    // default impl reads world authoritative state per FND-14A
    self.world().get_component_place_dirtiness(place).copied().unwrap_or_default()
}

fn latrine_fullness(&self, agent: EntityId, place: EntityId) -> LatrineFullness {
    self.world().get_component_latrine_fullness(place).copied().unwrap_or_default()
}

fn wash_basin_state(&self, agent: EntityId, basin: EntityId) -> WashBasinState {
    self.world().get_component_wash_basin_state(basin).copied().unwrap_or_default()
}
```

Adjust the exact body to match the existing precedent's `world()` accessor or equivalent the trait already exposes. Match the precedent's signature shape exactly — the `agent: EntityId` parameter is preserved per FND-14A's "co-located perception by this agent" framing even though the default impl doesn't read agent-specific state.

### 2. `crates/worldwake-sim/src/belief_view.rs` — `RuntimeBeliefView` forwarding block

After line 1768 (the existing `place_sleep_quality_profile` forwarder), add three new forwarder methods:

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

These forwarders may not be strictly necessary if the blanket `impl<T> GoalBeliefView for T where T: ProfileBeliefView` already covers `RuntimeBeliefView` — confirm during implementation by reading lines 1289–1440 to see whether forwarders are needed for runtime types specifically. If not, omit step 2.

### 3. Imports

Add `use crate::PlaceDirtiness;` etc. at the top of `belief_view.rs` (or `use worldwake_core::{PlaceDirtiness, LatrineFullness, WashBasinState};` if the file doesn't already import core types via crate-level alias).

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — trait extensions and runtime forwarders)

## Out of Scope

- Reads from the AI crate (deferred to tickets 009 and 010).
- Belief-store-backed accessors (today's pattern reads authoritative state directly per FND-14A; if S131 or a future spec adds belief-decayed perception for these components, that's a future ticket).
- Off-place propagation via `ShareBelief` (S129 explicitly defers this).

## Acceptance Criteria

### Tests That Must Pass

1. New focused test `place_dirtiness_accessor_returns_authoritative_state` in `belief_view.rs` test module — seeds a Place with `PlaceDirtiness { value: pm(500), ... }`, constructs a `RuntimeBeliefView`, asserts the accessor returns the seeded value.
2. New focused test `latrine_fullness_accessor_returns_default_for_unauthored_place` — non-latrine place returns `LatrineFullness::default()` (zero fill).
3. New focused test `wash_basin_state_accessor_returns_default_for_non_basin_facility` — analogous to the latrine test.
4. New focused test `goal_belief_view_forwards_hygiene_accessors` — proves the blanket `impl<T> GoalBeliefView for T where T: ProfileBeliefView` correctly forwards the three new methods.
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
