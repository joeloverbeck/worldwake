# S129PLADIRFAC-009: Per-basin and per-place-latrine candidate emission

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — refactors `emit_wash_goal` and `emit_relieve_goal` to produce per-facility / per-place anchored candidates instead of single bundled candidates
**Deps**: archive/tickets/S129PLADIRFAC-001.md, archive/tickets/S129PLADIRFAC-004.md

## Problem

Today's `emit_wash_goal` (`crates/worldwake-ai/src/candidate_generation.rs:3313–3390`) bundles all wash basins at one place into a single candidate anchored on `OpportunityAnchor::Place`. Today's `emit_relieve_goal` (lines 3282–3309) emits a single un-anchored candidate with `OpportunityAnchor::None`. Without per-basin and per-place-latrine anchoring, the ranking layer (ticket 010) cannot differentiate basins by `clean_water_units` / `dirtiness_level` or prefer a clean latrine over wilderness — it simply sees one bundled candidate and ranks it as a whole.

This ticket implements the candidate-emission half of S129's D10 (per Q2=(a) and Q3=(a) approved during reassessment): split wash candidates per-basin, split relieve candidates per-place-latrine + wilderness fallback. It does **not** add ranking arithmetic — that's ticket 010. The split makes the differentiating data reachable to ranking; this ticket's job is to produce the right candidate shape.

This ticket exercises Authoritative-to-AI Impact Rule checklist point 2 (`generate_candidates`) for the wash refactor.

## Assumption Reassessment (2026-04-29)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `emit_wash_goal` at `crates/worldwake-ai/src/candidate_generation.rs:3313–3390`. Verified during reassessment: enumerates `wash_access_opportunities()` for places reachable from the agent that host `WorkstationTag::WashBasin` facilities + co-located Water `ResourceSource`. Today emits **one candidate per reachable place** anchored on `OpportunityAnchor::Place(candidate_place)` (line 3351). Since ticket 007 reduces wash to a single basin target (target arity 2→1), `wash_access_opportunities` may need its own contract revision — confirm during implementation whether the helper produces basin entities or place entities today.
2. `emit_relieve_goal` at `crates/worldwake-ai/src/candidate_generation.rs:3282–3309`. Verified: emits **one candidate** with `OpportunityAnchor::None` (line 3301). No place information attached.
3. The shared abstraction boundary under audit is the candidate-emission contract — `GroundedGoal { goal_key, anchor, evidence_sets }` per the `worldwake-validation-patterns.md` Candidate Scoring Architecture rule. Emitters gate (decide *whether* to emit); ranking decides *priority*. This ticket adds emission gates per-basin / per-place-latrine; the per-anchor scoring lives in ticket 010 (`ranking.rs`).
4. Live `OpportunityAnchor` variants are `None`, `Place(EntityId)`, and `Entity(EntityId)` in `worldwake-core/src/goal.rs`. Per Q2=(a), wash candidates anchor on `Entity(basin_id)`; per Q3=(a), latrine candidates anchor on `Place(place_id)` (since Q1=(b) keeps latrines as place-tagged places, not facilities), and a wilderness fallback candidate stays with `None`.
5. The agent's reachability surface (which places/facilities are "known" to the agent for emission purposes) is provided through the existing belief-view layer — emit_wash_goal and emit_relieve_goal already use this. The new accessors from ticket 004 (`place_dirtiness`, `latrine_fullness`, `wash_basin_state`) are read by ticket 010 ranking; emission itself only needs to enumerate the facilities/places that exist, not their state values.
6. Existing focused/unit coverage: grep `candidate_generation.rs`'s test module for tests exercising `emit_wash_goal` and `emit_relieve_goal`. Tests will need updating because the candidate count changes — instead of one wash candidate per reachable place, multiple wash candidates per reachable basin; instead of one un-anchored relieve candidate, one candidate per reachable latrine-tagged place plus one wilderness candidate.
7. Phase distinction (precision-rules §1): this ticket lives in candidate generation. Ranking is ticket 010. Authoritative outcome is ticket 007. The action surface is ticket 003 + 007. Treating these as one merged ticket would muddle the four-phase boundary.
8. Post-007 review correction (2026-05-01): ticket 007 added planning-snapshot `WashBasinState` carriage and `goal_model::places_with_wash_access` now checks basin clean water instead of co-located `ResourceSource`. Implementation confirmed `PlanningState`'s default `WashBasinState` fallback for known wash-basin facilities was an FND-14 dynamic-state leak for remote/merely known basins. This ticket removed that fallback and added `GoalBeliefView::facility_wash_basin_state`, so per-basin emission requires a concrete facility-state carrier with `clean_water_units > 0`.

## Architecture Check

1. Splitting per-basin / per-place-latrine candidates moves the "which target am I committing to" decision into the candidate shape itself, so ranking just compares scores rather than disambiguating sub-targets at score time. Per FND-21 (intent is not entitlement), the anchored candidate is the agent's expressed intent — the resolution into actual contention is still owned by the action's affordance/start path, not by candidate selection.
2. Per FND-7 (locality), candidate emission is bounded by the agent's reachable belief — both refactors continue to use the existing reachability helpers; no global "all basins" enumeration.
3. No backward-compat shim: the existing emission shape is replaced, not aliased. Old single-candidate emission is gone; the new per-basin / per-place-latrine emission is the sole producer.

## Verification Layers

1. `emit_wash_goal` produces one candidate per reachable basin → focused unit test seeding two basins at one place. Run `emit_wash_goal`. Assert two candidates, each with `OpportunityAnchor::Entity(basin_id)` for distinct basin entities.
2. `emit_relieve_goal` produces per-place-latrine candidates plus wilderness fallback → focused unit test seeding three reachable places, of which two carry `PlaceTag::Latrine`. Assert three candidates: two with `OpportunityAnchor::Place(latrine_place_id)` for distinct latrines, one with `OpportunityAnchor::None`.
3. `emit_relieve_goal` produces only wilderness when no latrines reachable → focused unit test seeding zero latrines. Assert one candidate with `OpportunityAnchor::None`.
4. `emit_wash_goal` produces no candidates when no basins reachable → focused unit test seeding zero basins. Assert empty candidate set.
5. `emit_wash_goal` filters basins gated by ticket 003's precondition (basin has `clean_water_units > 0`) → focused candidate-generation tests seed explicit `WashBasinState` carriers and prove empty or unknown-state basins do not produce clean-water wash candidates.
6. Remote/merely known basin state does not overclaim clean-water availability → focused planner/candidate-generation test for a known remote wash basin that has a workstation-tag belief but no explicit `WashBasinState` carrier. The assertion should match the reassessed contract: either no clean-water-gated wash candidate/relevant place is emitted, or the ticket adds an explicit belief carrier and proves it is populated before emission.

## What to Change

### 1. Refactor `emit_wash_goal` at `candidate_generation.rs:3313–3390`

Replace the place-anchored bundling with a per-basin enumeration. Pseudocode:

```rust
fn emit_wash_goal(...) -> Vec<GroundedGoal> {
    let mut candidates = Vec::new();
    for opportunity in wash_access_opportunities(context) {
        for basin_id in matching_workstations_at(world, opportunity.place, WorkstationTag::WashBasin) {
            let anchor = OpportunityAnchor::Entity(basin_id);
            candidates.push(emit_candidate_with_trace(
                GoalKey::Wash,
                anchor,
                evidence_for_basin(basin_id, opportunity.place),
                ...
            ));
        }
    }
    candidates
}
```

Verify the helper APIs (`matching_workstations_at`, `wash_access_opportunities`, `emit_candidate_with_trace`) during implementation. The exact name `wash_access_opportunities` was cited at line 3318 of the validation report; if its return shape needs widening to expose basin entity ids per place, that helper change is in-scope.

### 2. Refactor `emit_relieve_goal` at `candidate_generation.rs:3282–3309`

Replace the single un-anchored candidate with per-place-latrine + wilderness emission. Pseudocode:

```rust
fn emit_relieve_goal(...) -> Vec<GroundedGoal> {
    let mut candidates = Vec::new();
    // Per-latrine-place candidates
    for place_id in reachable_places_with_tag(context, PlaceTag::Latrine) {
        candidates.push(emit_candidate_with_trace(
            GoalKey::Relieve,
            OpportunityAnchor::Place(place_id),
            evidence_for_latrine_place(place_id),
            ...
        ));
    }
    // Wilderness fallback (always present so ranking can fall through)
    candidates.push(emit_candidate_with_trace(
        GoalKey::Relieve,
        OpportunityAnchor::None,
        evidence_for_wilderness(),
        ...
    ));
    candidates
}
```

Verify `reachable_places_with_tag` exists or implement an equivalent helper; the existing wilderness-relief affordance already filters places by tag for its precondition `ActorAtPlaceTag(Latrine)`, so a similar helper likely exists.

### 3. Update existing tests

If `candidate_generation.rs`'s test module has tests asserting the bundled wash-candidate shape or the un-anchored relieve-candidate shape, rewrite them to assert the new per-anchor shapes.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — `emit_wash_goal` rewrite at 3313, `emit_relieve_goal` rewrite at 3282; helper additions for reachable latrine places and per-basin opportunities)
- `crates/worldwake-ai/src/goal_model.rs` / `crates/worldwake-ai/src/planning_state.rs` (modify — remove the post-007 clean-water relevant-place fallback that overclaimed unknown remote basin state)
- `crates/worldwake-sim/src/belief_view.rs` (modify — add `GoalBeliefView::facility_wash_basin_state` and `GoalBeliefView::place_has_tag` forwarding so candidate generation can read the exact live carriers)
- Likely: `crates/worldwake-ai/src/affordance_query.rs` (modify — only if `wash_access_opportunities` needs to expose per-basin ids; confirm during implementation)

## Out of Scope

- Ranking arithmetic that uses `WashBasinState.clean_water_units` / `dirtiness_level` and `LatrineFullness.fill` — deferred to ticket 010.
- Wash action precondition swap — landed in tickets 003 and 007.
- AI-side reads of `PlaceDirtiness` for sleep/explore ranking — deferred to ticket 010.
- Golden coverage — deferred to ticket 012.

## Acceptance Criteria

### Tests That Must Pass

1. New focused test `emit_wash_goal_produces_one_candidate_per_basin_at_place` — multiple basins at one place produce multiple anchored candidates.
2. New focused test `emit_wash_goal_produces_zero_candidates_when_no_basins_reachable` — emission is bounded by reachability.
3. New focused test `emit_relieve_goal_produces_per_place_latrine_candidates_plus_wilderness` — multiple latrine-tagged places + wilderness fallback.
4. New focused test `emit_relieve_goal_produces_only_wilderness_when_no_latrines_reachable` — wilderness fallback is always present.
5. New or updated focused test covering the post-007 remote/unknown basin-state boundary for wash candidate or relevant-place emission.
6. Updated existing tests in `candidate_generation.rs` (rename to reflect per-anchor expectations).
7. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. `emit_wash_goal` produces exactly one `GroundedGoal` per reachable `WorkstationTag::WashBasin` facility with concrete known `WashBasinState.clean_water_units > 0`, anchored on `OpportunityAnchor::Entity(basin_id)`.
2. `emit_relieve_goal` always produces at least one `GroundedGoal`: the wilderness fallback with `OpportunityAnchor::None`. If reachable latrine-tagged places exist, one additional candidate per such place anchored on `OpportunityAnchor::Place(place_id)`.
3. Candidate enumeration is bounded by the agent's reachable belief surface — no global "all basins in world" enumeration, no global "all latrines" enumeration.
4. Per Candidate Scoring Architecture rule (`worldwake-validation-patterns.md`): no `score` field is attached to the candidate; ranking happens in ticket 010 separately.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` / `goal_model.rs` test modules — five focused tests; existing wash/relieve emission tests rewritten.

### Commands

1. `cargo test -p worldwake-ai candidate_generation`
2. `cargo test -p worldwake-ai`
3. `cargo build --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-01. `emit_relieve_goal` now emits one `OpportunityAnchor::Place(place_id)` candidate per reachable latrine-tagged place plus the wilderness fallback with `OpportunityAnchor::None`. `emit_wash_goal` now emits one `OpportunityAnchor::Entity(basin_id)` candidate per reachable wash basin with explicit known clean-water state.

Implementation found one live-contract correction from the draft: `OpportunityAnchor::Facility` does not exist; the correct basin anchor is `OpportunityAnchor::Entity`. The implementation also removed the `PlanningState` fallback that synthesized default `WashBasinState` from a mere wash-basin workstation tag. That fallback overclaimed dynamic remote facility state, so candidate generation now requires a concrete facility-state carrier before it treats a basin as a clean-water wash opportunity.

## Verification Result

Passed:

1. `cargo test -p worldwake-ai --lib candidate_generation::tests::emit_wash_goal_produces_one_candidate_per_basin_at_place -- --exact`
2. `cargo test -p worldwake-ai --lib candidate_generation::tests::emit_wash_goal_produces_zero_candidates_when_no_basins_reachable -- --exact`
3. `cargo test -p worldwake-ai --lib candidate_generation::tests::emit_wash_goal_skips_known_remote_basin_without_state_carrier -- --exact`
4. `cargo test -p worldwake-ai --lib candidate_generation::tests::emit_relieve_goal_produces_per_place_latrine_candidates_plus_wilderness -- --exact`
5. `cargo test -p worldwake-ai --lib candidate_generation::tests::emit_relieve_goal_produces_only_wilderness_when_no_latrines_reachable -- --exact`
6. `cargo test -p worldwake-ai --lib goal_model::tests::wash_`
7. `cargo test -p worldwake-ai --lib candidate_generation`
8. `cargo test -p worldwake-ai --lib search::tests::search_wash_finds_direct_plan_at_current_clean_basin -- --exact`
9. `cargo test -p worldwake-ai --lib search::tests::search_local_wash_candidates_require_clean_basin -- --exact`
10. `cargo fmt --all`
11. `cargo test -p worldwake-ai`
12. `cargo build --workspace`
13. `cargo clippy --workspace --all-targets -- -D warnings`
