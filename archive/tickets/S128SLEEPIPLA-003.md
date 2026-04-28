# S128SLEEPIPLA-003: Sim-layer DurationExpr and belief-view accessor

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `DurationExpr::Variable { min, max }` variant in `worldwake-sim` action semantics, new `GoalBeliefView::place_sleep_quality_profile` trait method with `RuntimeBeliefView` impl and `impl_goal_belief_view!` macro forwarding.
**Deps**: archive/tickets/S128SLEEPIPLA-001.md

## Problem

S128SLEEPIPLA-004 needs to register the refactored sleep action with a duration expression that admits early termination (recovery curve completes before the upper bound). The current `DurationExpr` enum (`crates/worldwake-sim/src/action_semantics.rs:105-129`) has `Fixed`, `ConsultRecord`, `TargetConsumable`, `TravelToTarget`, `ActorMetabolism`, `ActorTradeDisposition`, `ActorMarketPresence`, `ActorPatrolProfile`, `ActorTheftDisposition`, `ActorInvestigationDisposition`, `ActorWitnessQueryDisposition`, `BanditCampEstablishmentProfile`, `ActorDefendStance`, `CombatWeapon`, `TargetTreatment`, and several others — none model "variable duration with min/max bounds." S128SLEEPIPLA-005 needs `GoalBeliefView::place_sleep_quality_profile(agent: EntityId, place: EntityId) -> SleepQualityProfile` to read each candidate place's sleep quality through the agent's belief surface (so the AI doesn't read authoritative state). Both are small sim-layer additions sharing a single ticket boundary.

## Assumption Reassessment (2026-04-27)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `crates/worldwake-sim/src/action_semantics.rs:105-129` defines `DurationExpr` with the variants listed above; no `Variable { min, max }` exists today. The live `fixed_ticks(self) -> Option<u32>` method returns `Some(n)` only for `Fixed`-style variants — the new `Variable` variant returns `None`. The test sweep `ALL_DURATION_EXPRS` (around line 485) enumerates every variant; the new one must be added.
2. Exhaustive matches on `DurationExpr` exist where durations are resolved to concrete tick counts: `DurationExpr::resolve_for`, `estimate_duration_from_beliefs`, `PlannerDurationDependency::from_duration_expr`, and CLI duration formatting. There is no scheduler-local `DurationExpr` match to edit on the live branch; scheduler code receives the already-resolved `ActionDuration`.
3. `crates/worldwake-sim/src/belief_view.rs:264-813` defines `GoalBeliefView`, with `ProfileBeliefView` sub-trait (lines 754-810) holding accessors like `metabolism_profile()`, `homeostatic_needs()`. Place-level accessors like `place_visibility_profile` either exist on a sibling trait or are absent — the new `place_sleep_quality_profile` accessor follows the closest established place-component accessor pattern. `RuntimeBeliefView` impl backs the live read; `impl_goal_belief_view!` macro at the trait's blanket impl (around line 1351-1364) forwards trait methods.
4. Shared boundary under audit: the planner-facing belief surface. Reassessment corrected the drafted signature to `fn place_sleep_quality_profile(&self, agent: EntityId, place: EntityId) -> SleepQualityProfile`; the `agent` parameter is required to decide whether a place is known to that agent. Returns by value (not `Option<&_>`) because `SleepQualityProfile` is a defaultable place component after archive/tickets/S128SLEEPIPLA-001.md, and S128SLEEPIPLA-006 will make scenario-spawned places carry it universally. Until that seeding lands, absent or unknown place profiles resolve to `SleepQualityProfile::default()` so the AI cannot construct site preference for places it has never observed (FND-7 locality).
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
- Update `fixed_ticks(self) -> Option<u32>` to return `None` for `Variable` (consistent with other non-fixed variants like `ActorMetabolism`).
- Update the `ALL_DURATION_EXPRS` test sweep (around line 485) to include `DurationExpr::Variable { min: NonZeroU32::new(1).unwrap(), max: NonZeroU32::new(64).unwrap() }`.
- Update the `fixed_ticks_returns_none_for_dynamic_variants` test (around line 557+) to include the new variant in the assertion list.

### 2. Cascade exhaustive matches

Grep `worldwake-sim` for `match.*DurationExpr\b` and `DurationExpr::` patterns. For each exhaustive match site, add a `DurationExpr::Variable { min, max } => ...` arm. Live reassessment found no scheduler-local `DurationExpr` match; the concrete duration is resolved before the scheduler receives an `ActionDuration`. The required arms are:

- `DurationExpr::resolve_for` — resolve `Variable` to `max` so the action is scheduled for the upper bound.
- `estimate_duration_from_beliefs` — resolve `Variable` to `max` for belief-side duration estimation.
- `PlannerDurationDependency::from_duration_expr` — classify `Variable` as dependency-free, like `Fixed`, because its bound is embedded in the expression.
- CLI action formatting — render the min/max range instead of falling through to “varies.”

### 3. Add `GoalBeliefView::place_sleep_quality_profile` accessor

In `crates/worldwake-sim/src/belief_view.rs`:

- Add to the appropriate sub-trait (`ProfileBeliefView` if place accessors live there, or directly on `GoalBeliefView` if no place sub-trait exists yet — confirm during reassessment) the method:

  ```rust
  fn place_sleep_quality_profile(&self, agent: EntityId, place: EntityId) -> SleepQualityProfile;
  ```

- Implement on the runtime belief view: read the place's `SleepQualityProfile` via `world.get_component_sleep_quality_profile(place)` only when the named agent knows the place or is currently there. If the agent's belief store has no entry for the place (i.e., the agent has never observed this place), return `SleepQualityProfile::default()`. The exact "agent has belief of place" predicate uses the existing belief-store accessor pattern from sibling place accessors in `RuntimeBeliefView`.
- Forward the method through `impl_goal_belief_view!` macro (around line 1351-1364) so any blanket `impl<T: ProfileBeliefView + ...> GoalBeliefView for T` picks it up.

## Files to Touch

- `crates/worldwake-sim/src/action_semantics.rs` (modify — add variant, update `fixed_ticks`, extend test sweep)
- `crates/worldwake-sim/src/belief_view.rs` (modify — add trait method, `RuntimeBeliefView` impl, macro forwarding)
- `crates/worldwake-ai/src/planner_duration_contract.rs` (modify — classify `Variable` as dependency-free)
- `crates/worldwake-cli/src/handlers/actions.rs` (modify — display `Variable` as a min/max range)

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
2. `place_sleep_quality_profile(agent, unknown_place) == SleepQualityProfile::default()` — FND-7 locality preserved (no AI-layer access to authoritative state for unknown places).
3. `place_sleep_quality_profile(agent, known_place)` returns the authoritative `SleepQualityProfile` of that place (since universal-on-Place guarantees presence; no `Option` wrapper).
4. No new `DurationExpr` exhaustive-match site is left unhandled — `cargo test --workspace --no-run` succeeds.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_semantics.rs` (modify — extend `ALL_DURATION_EXPRS` sweep; assert `Variable` round-trips via bincode; assert `fixed_ticks() == None`).
2. `crates/worldwake-sim/src/belief_view.rs` (modify — add a focused unit test in the existing `#[cfg(test)]` module: seed a world with one place carrying an authored `SleepQualityProfile` and one place without belief; assert the accessor returns the authored profile for the first and the default for the second).

### Commands

1. `cargo test -p worldwake-sim --lib action_semantics::tests`
2. `cargo test -p worldwake-sim --lib belief_view::tests`
3. `cargo test --workspace --no-run` (catches any unhandled `DurationExpr` match arm)
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-28.

- Added `DurationExpr::Variable { min, max }` in `worldwake-sim`, including bincode sweep coverage, `fixed_ticks() == None`, authoritative resolution to the `max` upper bound, and belief-side duration estimation to the same upper bound.
- Added the belief-mediated sleep-quality accessor as `place_sleep_quality_profile(agent, place)` on the `GoalBeliefView`/`ProfileBeliefView` surface and implemented it for `PerAgentBeliefView`. Known or co-located places return their authored `SleepQualityProfile`; unknown places return `SleepQualityProfile::default()`.
- Updated exhaustive downstream consumers: planner duration dependency classification treats `Variable` as dependency-free, and CLI action formatting renders the min/max range.
- Synced the active S128 spec and dependent S128 tickets to the corrected accessor signature and the live `Permille` range for `recovery_modifier`.

## Deviations

- The drafted accessor signature omitted the acting `agent`, but the stated locality contract depends on whether that agent knows the place. The landed signature is `place_sleep_quality_profile(agent, place)`.
- The active S128 family used example `recovery_modifier` values above 1000 (`1100`, `1300`), which are invalid for `Permille`. The active spec and dependent tickets now use bounded values: Hillside Shelter `1000`, Riverside Camp `900`, Forest Clearing `800`, Fertile Fields `700`.
- No scheduler file changed. Live duration resolution happens before the scheduler receives `ActionDuration`, so the required `Variable` scheduling behavior is implemented in `DurationExpr::resolve_for` and `estimate_duration_from_beliefs`.

## Verification Result

- Passed `cargo test -p worldwake-sim --lib -- --list`
- Passed `cargo test -p worldwake-sim --lib action_semantics::tests::variable_duration_expr_resolves_to_upper_bound -- --exact`
- Passed `cargo test -p worldwake-sim --lib belief_view::tests::goal_belief_view_place_sleep_quality_profile_is_belief_scoped -- --exact`
- Passed `cargo test -p worldwake-sim --lib action_semantics::tests`
- Passed `cargo test -p worldwake-sim --lib belief_view::tests`
- Passed `cargo test --workspace --no-run`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
