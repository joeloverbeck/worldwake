# E19GUAPAT-007: Golden tests for patrol lifecycle, belief-driven motive, and route adaptation

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Patrol action affordance legality alignment, patrol-route snapshot propagation through planning/runtime, and new patrol golden coverage
**Deps**: [archive/tickets/guard-patrol/E19GUAPAT-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/guard-patrol/E19GUAPAT-001.md), [archive/tickets/guard-patrol/E19GUAPAT-003.md](/home/joeloverbeck/projects/worldwake/archive/tickets/guard-patrol/E19GUAPAT-003.md), [archive/tickets/guard-patrol/E19GUAPAT-004.md](/home/joeloverbeck/projects/worldwake/archive/tickets/guard-patrol/E19GUAPAT-004.md), [archive/tickets/guard-patrol/E19GUAPAT-005.md](/home/joeloverbeck/projects/worldwake/archive/tickets/guard-patrol/E19GUAPAT-005.md), [archive/tickets/guard-patrol/E19GUAPAT-006.md](/home/joeloverbeck/projects/worldwake/archive/tickets/guard-patrol/E19GUAPAT-006.md), [archive/tickets/guard-patrol/E19GUAPAT-008.md](/home/joeloverbeck/projects/worldwake/archive/tickets/guard-patrol/E19GUAPAT-008.md)

## Problem

Patrol infrastructure is now implemented and has focused coverage at the component, action, ranking, and patrol-system layers. What is still missing is golden proof that these delivered patrol pieces compose correctly through the live AI/runtime boundary:

1. a guard can complete a lawful patrol cycle and wrap its route,
2. an active patrol can be interrupted by higher-priority survival pressure without corrupting `PatrolRoute.current_index`,
3. patrol motive really scales from the guard's local belief substrate,
4. patrol route adaptation really changes downstream AI behavior, and
5. patrol behavior stays belief-local rather than leaking from other agents' reports or authoritative truth.

## Assumption Reassessment (2026-03-30)

1. The live golden harness is [`GoldenHarness`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs), not a hypothetical `GoldenTestHarness`. New patrol goldens should follow the existing `mod golden_harness;` pattern in `crates/worldwake-ai/tests/golden_*.rs`.
2. The shared abstraction boundary under audit is:
   authoritative patrol state in [`crates/worldwake-core/src/patrol.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/patrol.rs) and [`crates/worldwake-systems/src/patrol.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/patrol.rs),
   consumed through the AI belief/runtime surface in [`crates/worldwake-sim/src/belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs),
   then selected and executed through `worldwake-ai` and `worldwake-sim`.
3. Patrol candidate generation and ranking are already live. `emit_patrol_candidates()` exists in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs), and patrol motive arithmetic already lives in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs). The old ticket narrative that E19GUAPAT-004 still left a placeholder path is stale.
4. Patrol route adaptation is already live in [`crates/worldwake-systems/src/patrol.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/patrol.rs). Focused tests there already prove authoritative route mutation from retained `SocialObservationDetail::SuspectedTheft` and `ViolationMemory` inputs. This ticket should prove the cross-layer downstream consequence, not re-litigate the patrol system internals.
5. Patrol lifecycle contracts are already live in [`crates/worldwake-systems/src/patrol_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/patrol_actions.rs). Focused tests already prove duration scaling, commit/wrap, and abort-preserves-index. The golden layer should prove that AI actually drives those authoritative contracts in a realistic chain.
6. The ticket's original "public order feedback loop converges" scenario is not a live architectural contract. `public_order()` now includes guard presence in [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs), but thief deterrence still comes from local witness counting in [`crates/worldwake-ai/src/theft.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/theft.rs), not from `public_order()` or a transmitted guarded-place belief. Forcing a golden about "more patrols -> fewer crimes -> fewer patrols" would overclaim beyond the current code.
7. That mismatch is architectural, not cosmetic. The current system has two distinct safety/deterrence substrates:
   - patrol urgency uses crime memory plus office beliefs,
   - theft deterrence uses locally observed witness count.
   There is no canonical public-order-to-thief decision path yet. This ticket must not paper over that with a speculative golden.
8. The clean scope correction is to keep this ticket on delivered patrol architecture only: patrol cycle, interruption/resume, belief-driven motive, route adaptation, and locality. If the project later wants a true settlement-level negative feedback loop, that should be a separate architectural ticket that first chooses the canonical deterrence substrate instead of mixing `public_order()` and witness-count logic implicitly.
9. The original ticket's assumption that every patrol golden needs `PerceptionProfile` because "all golden test agents must have PerceptionProfile" is too broad. The live repo rule is narrower: set `PerceptionProfile` where the scenario depends on observation retention or post-production/post-event perception. These patrol goldens do depend on belief retention and social observation freshness, so the involved guards should still be given explicit perception profiles.
10. The spec's Canonical Regression Scenario F is still useful as motivation, but the live code does not yet implement its full "route predation feedback" claim through a public-order consumer. The ticket should prove only the parts that actually shipped.
11. Corrected scope: golden coverage for the patrol epic should prove end-to-end patrol behavior across AI, patrol action execution, and authoritative route mutation, while explicitly excluding the not-yet-canonical thief/public-order feedback loop.

## Architecture Check

1. The clean architecture is to prove the patrol feature at the earliest strong mixed-layer boundaries that match the live code:
   decision traces for patrol goal generation/selection,
   action traces for patrol lifecycle and interruption ordering,
   authoritative `PatrolRoute` state for route progress and adaptation.
2. This is better than trying to force one giant scenario that also proves an unimplemented public-order feedback loop. A giant speculative golden would be brittle and would misdescribe the current architecture.
3. It is also better than duplicating focused internals that already have good coverage in `patrol.rs`, `patrol_actions.rs`, and `ranking.rs`. Golden tests should prove the composition boundary those focused suites do not.
4. If the project later wants the ideal architecture for a true negative patrol/crime feedback loop, the cleaner long-term move is a dedicated follow-up that chooses one canonical criminal-deterrence substrate and propagates it lawfully. The current code does not yet have that single canonical path.
5. No backwards-compatibility shims or alias routes.

## Verification Layers

1. Patrol cycle completion and route wrap -> action trace plus authoritative `PatrolRoute.current_index`
2. Interruption/resume preserves waypoint progress -> action trace ordering plus authoritative `PatrolRoute.current_index`
3. Belief-driven patrol urgency -> decision trace `CandidateTrace.ranked` / patrol `motive_score`
4. Route adaptation changes downstream patrol target -> authoritative `PatrolRoute.assigned_places` followed by decision trace selected patrol goal / selected plan
5. Information locality / no cross-agent leakage -> decision trace patrol motive remains baseline and authoritative route remains unchanged without a guard-local report
6. `public_order()` guard bonus is already proven at the focused derived-view layer in [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs) and is not the primary proof surface for this ticket

## What to Change

### 1. New golden test file: `crates/worldwake-ai/tests/golden_patrol.rs`

Add a dedicated patrol golden suite with scenario metadata blocks and `mod golden_harness;`.

### 2. Prove the delivered patrol contracts, not the speculative feedback loop

Add goldens for these scenarios:

#### a. `golden_patrol_cycle_wraps_route`
- Setup: one guard with a two-waypoint patrol route and explicit `PatrolProfile`
- Run: enough ticks for patrol dwell, travel, next dwell, and wrap
- Assert:
  - decision trace selects `GoalKind::Patrol` for the current waypoint,
  - action trace records patrol commits,
  - authoritative `PatrolRoute.current_index` wraps after the second patrol leg

#### b. `golden_patrol_interruption_preserves_waypoint_until_resume`
- Setup: one guard already patrolling at a waypoint, with carried food available for recovery
- Run:
  - let patrol start,
  - raise hunger to a critical band,
  - observe patrol interruption,
  - observe eat/consume resolution,
  - observe later patrol resumption
- Assert:
  - patrol action is interrupted by the higher-priority survival branch,
  - `PatrolRoute.current_index` does not advance during the interruption,
  - later patrol resumes from that same waypoint and only then advances

#### c. `golden_patrol_belief_urgency_scales_from_local_crime_and_vacancy`
- Setup: comparable guards with the same patrol route/profile, but only one has guard-local patrol beliefs (for example: unresolved theft memory plus believed vacancy on the route jurisdiction)
- Run: one planning tick with decision tracing enabled
- Assert:
  - both generate/select the patrol goal,
  - the informed guard's patrol ranked summary has a higher `motive_score` than the baseline guard,
  - no authoritative world-state-only shortcut is needed

#### d. `golden_patrol_route_adaptation_retargets_after_local_report`
- Setup: one guard with a baseline patrol route and a guard-local social/theft report about a new place
- Run: enough ticks for the patrol adaptation system to mutate the route and for AI to replan
- Assert:
  - authoritative `PatrolRoute.assigned_places` gains/promotes the reported place,
  - the next selected patrol goal / plan targets the adapted route place

#### e. `golden_patrol_locality_requires_guard_local_report`
- Setup: another agent holds the theft report or relevant social observation, but the guard does not
- Run: planning/adaptation ticks
- Assert:
  - guard patrol motive stays at baseline,
  - guard route does not adapt,
  - no omniscient leak from another agent's memory or from authoritative truth occurs

### 3. Deterministic replay companion

Add at least one deterministic replay companion for the main patrol-cycle scenario, unless implementation shows a stronger shared replay helper makes that redundant.

## Files to Touch

- `crates/worldwake-ai/tests/golden_patrol.rs` (new)

## Out of Scope

- Any patrol engine change in `worldwake-core`, `worldwake-systems`, `worldwake-sim`, or `worldwake-ai` unless a new golden exposes a real bug in shipped patrol behavior
- Replacing the current theft deterrence architecture with a `public_order()`-driven consumer
- A speculative "crime decreases because patrols raise public order" convergence golden
- Captain-mediated patrol reassignment
- Richer patrol route-entry metadata than `assigned_places + current_index`

## Acceptance Criteria

### Tests That Must Pass

1. `golden_patrol_cycle_wraps_route`
2. `golden_patrol_interruption_preserves_waypoint_until_resume`
3. `golden_patrol_belief_urgency_scales_from_local_crime_and_vacancy`
4. `golden_patrol_route_adaptation_retargets_after_local_report`
5. `golden_patrol_locality_requires_guard_local_report`
6. At least one deterministic replay companion for the patrol suite
7. `cargo test -p worldwake-ai golden_patrol`
8. `cargo test -p worldwake-ai`
9. `cargo test --workspace`
10. `cargo clippy --workspace`

### Invariants

1. Goldens use the strongest live proof surface per invariant: decision trace for motive/locality, action trace for patrol lifecycle, authoritative route state for patrol progress/adaptation
2. Tests assert only the current authoritative patrol contract (`assigned_places`, `current_index`, explicit dwell timing), not speculative route metadata or hidden cadence semantics
3. No golden claims a public-order feedback loop that the current thief architecture does not consume
4. All arithmetic and verification remain deterministic

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_patrol.rs` — `golden_patrol_cycle_wraps_route`
   Rationale: proves the full AI -> patrol action -> route-progress loop, including authoritative wrap, at the golden layer.
2. `crates/worldwake-ai/tests/golden_patrol.rs` — `golden_patrol_cycle_wraps_route_replays_deterministically`
   Rationale: gives the patrol suite the standard replay guarantee for a multi-tick mixed-layer scenario.
3. `crates/worldwake-ai/tests/golden_patrol.rs` — `golden_patrol_interruption_preserves_waypoint_until_resume`
   Rationale: proves the important cross-layer invariant that patrol can yield to survival pressure without corrupting persistent route progress.
4. `crates/worldwake-ai/tests/golden_patrol.rs` — `golden_patrol_belief_urgency_scales_from_local_crime_and_vacancy`
   Rationale: proves the live patrol motive really depends on guard-local beliefs rather than authoritative global truth.
5. `crates/worldwake-ai/tests/golden_patrol.rs` — `golden_patrol_route_adaptation_retargets_after_local_report`
   Rationale: proves authoritative route adaptation changes later AI patrol selection, which focused unit tests do not cover end to end.
6. `crates/worldwake-ai/tests/golden_patrol.rs` — `golden_patrol_locality_requires_guard_local_report`
   Rationale: guards against omniscient leakage by proving another agent's report or remote truth does not mutate this guard's patrol behavior.

### Commands

1. `cargo test -p worldwake-ai golden_patrol -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-30
- What actually changed:
  - Added [`crates/worldwake-ai/tests/golden_patrol.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_patrol.rs) covering patrol cycle/wrap, deterministic replay, interruption/resume, belief-driven urgency scaling, route-adaptation retargeting, and locality.
  - Added patrol-route invalidation coverage in [`crates/worldwake-ai/src/agent_tick/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs).
  - Promoted patrol-route state into the AI runtime/planning snapshot surface so patrol replans use live waypoint state.
  - Tightened snapshot continuation to require the same opportunity anchor, not just the same `GoalKey`.
  - Fixed a real architecture bug in [`crates/worldwake-systems/src/patrol_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/patrol_actions.rs): patrol affordances are now emitted only when the actor is actually at the current waypoint, so the planner no longer sees an action branch that authoritative start logic would reject.
- Deviations from original plan:
  - The ticket no longer remained test-only. The new goldens exposed two real mixed-layer defects, so the final implementation necessarily included engine/runtime fixes.
  - The patrol-cycle golden was corrected to assert the durable contract that guards continue alternating across wrapped routes without stale patrol start failures, rather than incorrectly asserting that patrol stops after one two-leg cycle.
  - The originally-described public-order/thief convergence loop remains out of scope. The shipped architecture still has no canonical `public_order()` consumer in thief decision-making, so the archived completion record does not claim that loop.
- Verification results:
  - `cargo test -p worldwake-systems patrol_actions -- --nocapture`
  - `cargo test -p worldwake-ai golden_patrol -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
