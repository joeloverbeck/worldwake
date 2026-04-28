# S128SLEEPIPLA-003: Sim-layer DurationExpr and belief-view accessor

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `DurationExpr::Variable { min, max }` variant in `worldwake-sim` action semantics, new `GoalBeliefView::place_sleep_quality_profile` trait method with `RuntimeBeliefView` impl and `impl_goal_belief_view!` macro forwarding.
**Deps**: archive/tickets/S128SLEEPIPLA-001.md

## Problem

S128SLEEPIPLA-004 needs to register the refactored sleep action with a duration expression that admits early termination (recovery curve completes before the upper bound). The current `DurationExpr` enum (`crates/worldwake-sim/src/action_semantics.rs:105-129`) has `Fixed`, `ConsultRecord`, `TargetConsumable`, `TravelToTarget`, `ActorMetabolism`, `ActorTradeDisposition`, `ActorMarketPresence`, `ActorPatrolProfile`, `ActorTheftDisposition`, `ActorInvestigationDisposition`, `ActorWitnessQueryDisposition`, `BanditCampEstablishmentProfile`, `ActorDefendStance`, `CombatWeapon`, `TargetTreatment`, and several others — none model "variable duration with min/max bounds." S128SLEEPIPLA-005 needs `GoalBeliefView::place_sleep_quality_profile(place: EntityId) -> SleepQualityProfile` to read each candidate place's sleep quality through the agent's belief surface (so the AI doesn't read authoritative state). Both are small sim-layer additions sharing a single ticket boundary.

## Assumption Reassessment (2026-04-27)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `crates/worldwake-sim/src/action_semantics.rs:105-129` defines `DurationExpr` with the variants listed above; no `Variable { min, max }` exists today. The `fixed_ticks(&self) -> Option<NonZeroU32>` method (around line 533+) returns `Some(n)` only for `Fixed`-style variants — the new `Variable` variant returns `None`. The test sweep `ALL_DURATION_EXPRS` (around line 485) enumerates every variant; the new one must be added.
2. Exhaustive matches on `DurationExpr` exist in the scheduler / action framework — primarily where duration is resolved to concrete tick counts. `crates/worldwake-sim/src/action_semantics.rs:533+ fixed_ticks` is one such match. Other consumers: `Scheduler` resolution code (likely in `scheduler.rs`). Each exhaustive match needs a new arm. Action handlers themselves do not match on `DurationExpr` — they receive resolved tick counts.
3. `crates/worldwake-sim/src/belief_view.rs:264-813` defines `GoalBeliefView`, with `ProfileBeliefView` sub-trait (lines 754-810) holding accessors like `metabolism_profile()`, `homeostatic_needs()`. Place-level accessors like `place_visibility_profile` either exist on a sibling trait or are absent — the new `place_sleep_quality_profile` accessor follows the closest established place-component accessor pattern. `RuntimeBeliefView` impl backs the live read; `impl_goal_belief_view!` macro at the trait's blanket impl (around line 1351-1364) forwards trait methods.
4. Shared boundary under audit: the planner-facing belief surface. The new accessor signature is `fn place_sleep_quality_profile(&self, place: EntityId) -> SleepQualityProfile`. Returns by value (not `Option<&_>`) because `SleepQualityProfile` is a defaultable place component after archive/tickets/S128SLEEPIPLA-001.md, and S128SLEEPIPLA-006 will make scenario-spawned places carry it universally. Until that seeding lands, absent or unknown place profiles resolve to `SleepQualityProfile::default()` so the AI cannot construct site preference for places it has never observed (FND-7 locality).
5. `DurationExpr::Variable { min: NonZeroU32, max: NonZeroU32 }` derives match the existing enum (`Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize` per the existing `ALL_DURATION_EXPRS` test sweep including `bincode` round-trip). The new variant adds two `NonZeroU32` fields — both `Copy` — so the existing derives propagate cleanly.
6. Information-path refactor: `place_sleep_quality_profile` is a new accessor (no prior path). It is the canonical and only path for AI-layer reads of place sleep quality. Authoritative reads at action-handler time go through the world transaction directly (S128SLEEPIPLA-004), not this accessor.

## Architecture Check

1. `DurationExpr::Variable { min, max }` is the natural fit for "this action takes between `min` and `max` ticks; tick-handler-driven early termination commits before `max`." It mirrors the existing `Fixed(NonZeroU32)` pattern but admits a range, keeping the duration semantics in `DurationExpr` rather than smuggling per-action range fields onto every action handler.
2. The accessor returns `SleepQualityProfile` by value (not `Option<...>`) because the universal-on-Place precedent (S128SLEEPIPLA-006) guarantees every place has the component. Belief-mediated reads remain at full fidelity for places the agent knows about; for unknown places, the deterministic `Default` fallback respects FND-7 locality without forcing the AI to handle absence specially.
3. Both deliverables sit on the same architectural slice (sim layer) and unblock independent downstream tickets (D5 → S128SLEEPIPLA-004, D9 → S128SLEEPIPLA-005). Bundling avoids two micro-tickets in the same crate while keeping the diff small.

## Verification Layers

1. `DurationExpr::Variable` round-trips through bincode and reports `fixed_ticks() == None` → focused unit tests in `action_semantics.rs` test module (existing `ALL_DURATION_EXPRS` sweep extended).
2. `GoalBeliefView::place_sleep_quality_profile` returns the authoritative profile for places the agent has belief of, and `SleepQualityProfile::default()` for unknown places → focused unit tests in `belief_view.rs` test module.
3. Single-layer ticket per deliverable: D5 is action-semantics-internal; D9 is belief-view-internal. Both verify at the focused unit / runtime test layer. No decision-trace or action-trace assertions are appropriate yet — those layers exercise the consumers (-004 and -005), not the additions themselves.

## What to Change

### 1. Add `DurationExpr::Variable { min, max }` variant

In `crates/worldwake-sim/src/action_semantics.rs`:

- Add `Variable { min: NonZeroU32, max: NonZeroU32 }` to the `DurationExpr` enum (around line 105-129).
- Update `fixed_ticks(&self) -> Option<NonZeroU32>` to return `None` for `Variable` (consistent with other non-fixed variants like `ActorMetabolism`).
- Update the `ALL_DURATION_EXPRS` test sweep (around line 485) to include `DurationExpr::Variable { min: NonZeroU32::new(1).unwrap(), max: NonZeroU32::new(64).unwrap() }`.
- Update the `fixed_ticks_returns_none_for_dynamic_variants` test (around line 557+) to include the new variant in the assertion list.

### 2. Cascade exhaustive matches

Grep `worldwake-sim` for `match.*DurationExpr\b` and `DurationExpr::` patterns. For each exhaustive match site, add a `DurationExpr::Variable { min, max } => ...` arm. Expected sites:

- Scheduler duration resolution (most likely in `crates/worldwake-sim/src/scheduler.rs`) — for `Variable`, schedule the action to run for up to `max` ticks; tick-handler-driven `ActionProgress::StopAndCommit` (or equivalent) terminates early.
- Any other `DurationExpr` exhaustive match in the sim or systems crate (run grep workspace-wide before editing).

If the scheduler currently uses `fixed_ticks()` exclusively to derive scheduling bounds, returning `None` for `Variable` may already be sufficient — the scheduler would treat it as a non-fixed-duration action and rely on tick handler progress signaling. Confirm this during reassessment; if true, no scheduler change is needed.

### 3. Add `GoalBeliefView::place_sleep_quality_profile` accessor

In `crates/worldwake-sim/src/belief_view.rs`:

- Add to the appropriate sub-trait (`ProfileBeliefView` if place accessors live there, or directly on `GoalBeliefView` if no place sub-trait exists yet — confirm during reassessment) the method:

  ```rust
  fn place_sleep_quality_profile(&self, place: EntityId) -> SleepQualityProfile;
  ```

- Implement on `RuntimeBeliefView`: read the place's `SleepQualityProfile` via `world.get_component_sleep_quality_profile(place)`. If the agent's belief store has no entry for the place (i.e., the agent has never observed this place), return `SleepQualityProfile::default()`. The exact "agent has belief of place" predicate uses the existing belief-store accessor pattern from sibling place accessors in `RuntimeBeliefView`.
- Forward the method through `impl_goal_belief_view!` macro (around line 1351-1364) so any blanket `impl<T: ProfileBeliefView + ...> GoalBeliefView for T` picks it up.

## Files to Touch

- `crates/worldwake-sim/src/action_semantics.rs` (modify — add variant, update `fixed_ticks`, extend test sweep)
- `crates/worldwake-sim/src/belief_view.rs` (modify — add trait method, `RuntimeBeliefView` impl, macro forwarding)
- `Likely: crates/worldwake-sim/src/scheduler.rs` (modify — add `Variable` arm if exhaustive match exists; grep `match.*DurationExpr\b` to confirm path before editing)

## Out of Scope

- Sleep action registration with `DurationExpr::Variable` — handled by S128SLEEPIPLA-004
- Per-place sleep candidate emission consuming `place_sleep_quality_profile` — handled by S128SLEEPIPLA-005
- Other actions adopting `DurationExpr::Variable` — Sleep is the only consumer in this spec; other actions remain on their current variants
- `SleepEpisode`/`SleepQualityProfile` definitions — landed in archive/tickets/S128SLEEPIPLA-001.md

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim action_semantics` — `ALL_DURATION_EXPRS` sweep includes `Variable`; `fixed_ticks_returns_none_for_dynamic_variants` includes `Variable`; bincode round-trip succeeds for `Variable`.
2. `cargo test -p worldwake-sim belief_view` — focused unit test confirms `place_sleep_quality_profile` returns the authored profile for a known place and `SleepQualityProfile::default()` for an unknown place.
3. Existing suite: `cargo test --workspace`.

### Invariants

1. `DurationExpr::Variable { min, max }.fixed_ticks() == None`.
2. `place_sleep_quality_profile(unknown_place) == SleepQualityProfile::default()` — FND-7 locality preserved (no AI-layer access to authoritative state for unknown places).
3. `place_sleep_quality_profile(known_place)` returns the authoritative `SleepQualityProfile` of that place (since universal-on-Place guarantees presence; no `Option` wrapper).
4. No new `DurationExpr` exhaustive-match site is left unhandled — `cargo build --workspace` succeeds.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_semantics.rs` (modify — extend `ALL_DURATION_EXPRS` sweep; assert `Variable` round-trips via bincode; assert `fixed_ticks() == None`).
2. `crates/worldwake-sim/src/belief_view.rs` (modify — add a focused unit test in the existing `#[cfg(test)]` module: seed a world with one place carrying an authored `SleepQualityProfile` and one place without belief; assert the accessor returns the authored profile for the first and the default for the second).

### Commands

1. `cargo test -p worldwake-sim action_semantics belief_view`
2. `cargo build --workspace` (catches any unhandled `DurationExpr` match arm)
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `./scripts/verify.sh`
