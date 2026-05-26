# S173SELCARINT-006: Candidate-emitter occupancy filter (wash + relieve)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — wash + relieve candidate emitters gain occupancy filtering with FND-14A/14B source-class split
**Deps**: `archive/tickets/S173SELCARINT-001.md` (uses `SelfCareOccupancy`), `specs/S173-self-care-interruption-occupancy.md` (D6, distributed D5 emitter-time read)

## Problem

The Wash and Relieve (toilet/latrine) candidate emitters in `crates/worldwake-ai/src/candidate_generation.rs` filter on facility *physical* state today — `wash_access_opportunities` at L4744 checks `facility_wash_basin_state(*workstation).is_some_and(|state| state.clean_water_units > 0)` at L4765-4767 — but they do not consult `SelfCareOccupancy`. As a result, the planner emits wash candidates targeting facilities that may already be occupied by another actor. The first downstream rejection happens at action start (ticket 004's reservation requirement), forcing a tick of wasted candidate emission and a replan. This ticket adds the missing emitter-time occupancy filter, gated by the FND-14A/14B source-class table from spec D5: co-located actors read occupancy directly from world state (FND-14A); for remote candidates, occupancy must be belief-backed (FND-14B), and absence of belief means no candidate is emitted.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Existing wash and relieve emitter tests in `crates/worldwake-ai/src/candidate_generation.rs` (verify at /implement-ticket time):
   - `wash_requires_dirtiness_and_known_clean_basin_state:13161`, `emit_wash_goal_produces_one_candidate_per_basin_at_place:13210`, `emit_wash_goal_produces_zero_candidates_when_no_basins_reachable:13251`, `emit_wash_goal_skips_known_remote_basin_without_state_carrier:13279`, `generate_candidates_explores_for_wash_access_when_only_local_water_is_available:22906`, `dirtiness_emits_water_acquisition_when_no_clean_wash_basin_is_known:12624`
   - `emit_relieve_goal_produces_per_place_latrine_candidates_plus_wilderness:13043`, `emit_relieve_goal_produces_only_wilderness_when_no_latrines_reachable:13096`, `relief_path_actionable_relieve_returns_true_for_wilderness_path:12892`, `fatigue_and_bladder_emit_sleep_and_relieve:12986`
   - All must continue to pass; new filter is additive (existing tests do not configure occupancy state).
2. The live goal kinds under test: `GoalKind::Wash` (`crates/worldwake-core/src/goal.rs:73`) and `GoalKind::Relieve` (L72). Wash candidates are emitted by `emit_wash_goal` (via `wash_access_opportunities`); Relieve candidates by `emit_relieve_goal` (location at line 4607 region). Verify the relieve emitter's function name at implementation time — `emit_relieve_goal` is the canonical surface per the existing test naming. The relieve-candidate per-place latrine enumeration is the integration site for the latrine-side occupancy filter.
3. Shared abstraction boundary: the `GoalBeliefView` accessor surface. `facility_wash_basin_state` (`crates/worldwake-sim/src/belief_view.rs:495`) returns `Option<WashBasinState>` and already handles FND-14A/14B split. Reading `SelfCareOccupancy` from a facility via belief view: **no existing accessor exists** for SelfCareOccupancy. Spec D5 asserts "no new accessor is required" — verify this claim at implementation time. Two options if a new accessor IS needed: (a) add a thin `facility_self_care_occupancy_observed` accessor on `GoalBeliefView` following the `facility_wash_basin_state` pattern (single new method with the FND-14A/14B-split implementation); (b) inline the read at the call site with an explicit FND-14A check (co-location predicate before reading world state directly). Option (a) is the cleaner long-term path because it centralizes the source-class logic.
4. **Path-level discrepancy from spec**: spec D5 claims no new accessor is required; codebase has `facility_wash_basin_state` for `WashBasinState` but nothing for `SelfCareOccupancy`. Resolved at /implement-ticket time by adding the accessor (option (a) above) if the consumer cannot compose without it. This is a path-level correction per Step 2's two-tier classification — the deliverable contract (don't query remote occupancy on behalf of agent) is preserved; the implementation question is which surface holds the FND-14A/14B split. Surfaces here without re-running `/reassess-spec`.
5. Goal infrastructure status (per `worldwake-validation-patterns.md::Goal Infrastructure Validation`): `GoalKind::Wash` and `GoalKind::Relieve` already exist with full integration (`GoalDispatchKey`, `GoalDispatchDeclaration` at `goal_schema.rs:DECL_WASH` L364-374 and `DECL_RELIEVE` L352-363, `GoalKindPlannerExt` methods, ranking, candidate generation). No new GoalKind variant; no infrastructure additions.
6. Authoritative-to-AI Impact Rule (spec's Auth-to-AI section point 2): modifies `generate_candidates` — emitter filter is the integration point. Points 1 (`get_affordances`), 4 (`BestEffort` action start), 5 (`handle_plan_failure`), 6 (payload revalidation) are owned by ticket 004's reservation surface; this ticket's contract is the emitter-side filter only.
7. Distributed deliverable D5: this ticket covers the emitter-time read side (FND-14B belief-backed for remote candidates; FND-14A co-located fallback when the actor is at the basin's place). The start-gate read (FND-14A at action start) is owned by ticket 004.

## Architecture Check

1. The filter is applied where candidate emission already gates on physical state (`clean_water_units > 0`). Adding the occupancy gate at the same site preserves the emit-vs-rank separation (per `worldwake-validation-patterns.md::Candidate Scoring Architecture`): the emitter still decides *whether* to emit; ranking still decides relative priority. The occupancy filter is a gate, not a score.
2. FND-14A/14B source-class split is enforced by the accessor (or by the call-site composition if no new accessor lands). Co-located occupancy: direct world read. Remote occupancy: belief-backed only — no plan composed assuming a remote basin is free unless belief carries that claim. This avoids the FND-14 violation of the planner reading authoritative remote state.
3. The filter is best-effort: an emitter-time pass that becomes stale by action start fails at the reservation gate (ticket 004) and replans next tick. This two-layer defense (emitter filter + start-gate reservation) is intentional and matches the architecture's existing pattern for other contested resources (e.g., harvest, craft).

## Verification Layers

1. Emitter-time co-located occupancy read → focused candidate-generation test: configure a Wash candidate scenario where the basin carries `SelfCareOccupancy` for another actor, assert zero wash candidates emitted for the actor.
2. Emitter-time belief-backed remote occupancy read → focused candidate-generation test: actor has belief that a remote basin is occupied; assert no wash candidate emitted for that basin.
3. Emitter passes when occupancy is absent → focused candidate-generation test: basin has no `SelfCareOccupancy`, candidate is emitted normally.
4. Action-trace ordering vs candidate-emission ordering → not applicable; this ticket changes emission, not action lifecycle.
5. Decision trace surface (FND-29 debuggability) → if a candidate is filtered for occupancy, the decision trace records the filter reason. Verify the existing emit-trace pattern in `candidate_generation.rs` is extended to surface "skipped due to known occupancy" — at minimum, the decision trace should not silently drop the candidate without traceable cause.

## What to Change

### 1. Add a `GoalBeliefView` accessor for SelfCareOccupancy presence (if needed)

If the consumer cannot compose without it, add a new method to `GoalBeliefView` in `crates/worldwake-sim/src/belief_view.rs`:

```rust
/// Returns `Some(true)` if the actor has a belief or co-located observation
/// that the facility/place is occupied by another actor; `Some(false)` if
/// known-not-occupied; `None` if no lawful information source (remote, no
/// belief). Matches the FND-14A/14B source-class table in spec D5.
fn facility_self_care_occupancy_observed(
    &self,
    actor: EntityId,
    facility: EntityId,
) -> Option<bool>;
```

Implement on `RuntimeBeliefView` and any blanket-forward `impl_goal_belief_view!`-style sites if they exist. (Note: per Step 2 spot-check (g), no `impl_goal_belief_view!` macro is present in the codebase; forwarding is manual.)

If the existing `wash_basin_state` accessor (L545, 1047 of `belief_view.rs`) can be extended to carry occupancy info alongside `WashBasinState`, that's an alternative path — but adding a sibling accessor is cleaner and more orthogonal.

### 2. Extend `wash_access_opportunities` to filter on occupancy

In `crates/worldwake-ai/src/candidate_generation.rs` at L4744, alongside the existing `facility_wash_basin_state(*workstation).is_some_and(|state| state.clean_water_units > 0)` check at L4765-4767, add:

```rust
&& view.facility_self_care_occupancy_observed(actor, *workstation) != Some(true)
```

(Or equivalent — the predicate should drop the candidate when occupancy is known to be present.)

### 3. Extend the relieve emitter's latrine enumeration

The relieve emitter (`emit_relieve_goal` per the existing test naming pattern at L13043) enumerates per-place latrines. Add the same occupancy gate to the latrine-target enumeration. Wilderness-relief candidates remain unfiltered (wilderness is location-flexible and does not require occupancy).

### 4. Update existing emitter tests

Existing tests (`emit_wash_goal_produces_one_candidate_per_basin_at_place`, etc.) do not configure `SelfCareOccupancy` in their world setup, so they should pass unchanged (the new filter has no effect when no occupancy is present). Verify at implementation time.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — emitter filter additions in `wash_access_opportunities` and the relieve latrine emitter)
- `crates/worldwake-sim/src/belief_view.rs` (modify — new accessor `facility_self_care_occupancy_observed` if needed; verify at implementation time whether composition is possible without it)

## Out of Scope

- Start-gate reservation enforcement — owned by ticket 004.
- Action-side occupancy write/remove — owned by ticket 004.
- Atomic-action emitter changes (eat, drink, wilderness, sleep) — no occupancy filter applies to those.
- Belief-update mechanics (how an actor learns a remote basin is occupied) — this ticket reads belief, not writes it. Belief writes happen through normal perception/witness propagation per S163.
- Decision-trace enrichment beyond the existing "candidate filtered" surface — if more detail is needed, that becomes a follow-up traceability ticket.

## Acceptance Criteria

### Tests That Must Pass

1. New test: `emit_wash_goal_skips_basin_with_known_self_care_occupancy_by_other_actor` — basin has `SelfCareOccupancy { occupant: other_actor }`, actor's candidate is filtered.
2. New test: `emit_wash_goal_emits_when_actor_is_the_occupant` — if the actor IS the occupant (already washing), candidate emission still works correctly (no self-blocking).
3. New test: `emit_wash_goal_skips_remote_basin_with_belief_of_occupancy` — actor at a different place believes the basin is occupied; no candidate emitted.
4. New test: `emit_relieve_goal_skips_latrine_with_known_occupancy_by_other_actor` — symmetric for the relieve emitter on a latrine-tagged Place.
5. Existing tests pass: `wash_requires_dirtiness_and_known_clean_basin_state`, `emit_wash_goal_produces_one_candidate_per_basin_at_place`, `emit_wash_goal_produces_zero_candidates_when_no_basins_reachable`, `emit_wash_goal_skips_known_remote_basin_without_state_carrier`, `dirtiness_emits_water_acquisition_when_no_clean_wash_basin_is_known`, `emit_relieve_goal_produces_per_place_latrine_candidates_plus_wilderness`, `emit_relieve_goal_produces_only_wilderness_when_no_latrines_reachable` — all unchanged in their existing assertions because the world-setup does not configure occupancy.
6. Existing suite: `cargo test -p worldwake-ai candidate_generation`.

### Invariants

1. A wash or relieve candidate is never emitted for a facility/place whose `SelfCareOccupancy` is known (by belief or co-location) to be held by another actor.
2. Remote occupancy reads are belief-backed only — no FND-14 violation through direct authoritative reads on behalf of remote actors.
3. Wilderness-relief candidates are not filtered (location-flexible, no occupancy substrate).
4. Self-occupied actors (actor IS the occupant) are not self-blocked from emitting a candidate against the same facility.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` inline tests — 4 new tests covering the new filter scenarios.
2. If a new belief-view accessor lands in `belief_view.rs`, add inline accessor-level tests covering the FND-14A/14B source-class split (mirror the existing `facility_wash_basin_state` test pattern around L3181/L3740).

### Commands

1. `cargo test -p worldwake-ai candidate_generation`
2. `cargo test -p worldwake-sim belief_view` (if the new accessor lands)
3. `cargo build --workspace`
4. `./scripts/verify.sh` before commit.
