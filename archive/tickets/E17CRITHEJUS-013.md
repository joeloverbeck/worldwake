# E17CRITHEJUS-013: Golden test — witnessed theft enables accusation chain

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — accusation had to become office-filed at a concrete `CrimeRegister`, planner goal identity had to name that filing surface explicitly, and punishment planning had to align with lawful locally collectible control rather than generic inventory belief
**Deps**: E17CRITHEJUS-008 (accuse action), E17CRITHEJUS-009 (fine/exile actions), E17CRITHEJUS-011 (justice candidates), E17CRITHEJUS-012 (theft golden as validation baseline), E17CRITHEJUS-015 (typed social-evidence detail), E17CRITHEJUS-016 (relayable social evidence)

## Problem

There is still no golden proving the full witnessed-theft justice chain end to end under the intended E17 architecture:

1. thief steals
2. witness acquires typed theft evidence
3. witness relays that evidence through Tell
4. authority files an accusation at the jurisdictional `CrimeRegister`
5. authority later punishes the accused when co-located

Reassessment shows this is not a "tests only" gap. The live `accuse` production path in `crates/worldwake-systems/src/justice_actions.rs` still requires the accused to be physically co-located with the accuser at accusation time via `TargetSpec::EntityAtActorPlace` plus `validate_accuse_context()`. That collapses accusation filing into the same-place confrontation path and blocks the cleaner office-record architecture described in `specs/E17-crime-theft-justice.md`.

## Assumption Reassessment (2026-03-26)

1. Shared boundary under audit: the crime-case propagation path from `SocialObservationDetail::SuspectedTheft` in agent belief state, through `TellTopic`, into `InstitutionalClaim::Accusation` at a `CrimeRegister`, and then into `InstitutionalClaim::Verdict` plus punishment world-state mutations.
2. `E17CRITHEJUS-015` and `E17CRITHEJUS-016` have already landed. The ticket can no longer claim that typed theft evidence or social-evidence relay are missing. Live code now supports `SocialObservationDetail::SuspectedTheft` in `crates/worldwake-core/src/belief.rs`, relayable social observations in `crates/worldwake-systems/src/tell_actions.rs`, and social-topic Tell candidate generation in `crates/worldwake-ai/src/candidate_generation.rs`.
3. The live Tell architecture already makes the intended information path lawful: witness perception creates a concrete `SocialObservation`; `TellTopic::SocialObservation` relays that observation physically; the listener records reported theft evidence with degraded provenance. The canonical end-state path for this ticket is therefore witness evidence -> Tell -> accusation record -> punishment. No alternate compatibility relay path should be introduced.
4. Existing coverage already proves adjacent lower layers:
   - `crates/worldwake-ai/tests/golden_emergent.rs`: `golden_theft_leads_owner_to_local_suspected_theft_discovery`
   - `crates/worldwake-ai/src/candidate_generation.rs`: `justice_candidates_emit_accuse_from_matching_typed_theft_testimony`
   - `crates/worldwake-ai/src/candidate_generation.rs`: `justice_candidates_emit_fine_punishment_from_consulted_accusation`
   - `crates/worldwake-ai/tests/planner_conformance.rs`: `conformance_accuse`
5. `cargo test -p worldwake-ai -- --list` confirms there is currently no `golden_*` test covering the full witnessed-theft -> Tell -> accuse -> punish chain. The gap is specifically missing golden/E2E coverage, not missing focused candidate-generation coverage.
6. The original ticket assumption that Tell could not yet carry witnessed theft evidence is obsolete. `crates/worldwake-core/src/belief.rs::social_observation_is_relayable()` no longer excludes `SocialObservationDetail::SuspectedTheft`, and `crates/worldwake-systems/src/tell_actions.rs` includes relayable social observations in tell affordances and commit transfer.
7. The more important mismatch is architectural: `crates/worldwake-systems/src/justice_actions.rs::accuse_action_def()` binds `accuse` to `TargetSpec::EntityAtActorPlace { kind: EntityKind::Agent }`, and `validate_accuse_context()` rejects non-colocated suspects. That means the authority cannot lawfully travel to the `CrimeRegister` and file a case against an absent suspect even though the spec and ticket narrative require accusation and punishment to be separate stages.
8. `specs/E17-crime-theft-justice.md` consistently treats accusation as office filing and punishment as later co-located enforcement:
   - accusation: actor at `CrimeRegister`, evidence in belief state, no duplicate case
   - punishment: institutional authority plus co-location with accused
   The current code therefore diverges from the spec on a production contract, not only on test coverage.
9. This divergence matters architecturally. Requiring suspect presence at accusation time weakens P16/P21/P23 by making the institutional record path depend on immediate physical confrontation. It also makes `Accuse` less distinct from `Fine`/`Exile`, reducing extensibility for future guard/investigation behavior.
10. Clean scope correction: this ticket must include the minimal production fix needed to restore the intended accusation boundary, then add the golden and replay companion against that contract. Leaving `Engine Changes: None` would hide a real production contradiction.
11. Scenario isolation: the intended branch is witnessed theft testimony leading to authority action. Lawful competing branches include owner-local investigation and direct possession observation by the authority. The golden should suppress those unrelated branches by setup so it proves the witness-relay path specifically, not "some lawful theft evidence eventually existed."
12. Timing contract:
   - theft visibility and witness evidence acquisition: authoritative action/event consequence in the theft tick
   - Tell propagation: action lifecycle ordering
   - accusation filing: authoritative `CrimeRegister` mutation ordering
   - punishment: authoritative world-state mutation plus record supersession ordering
   The ticket must not rewrite these as generic "later tick" expectations when action-trace or record-state boundaries are stronger.
13. Further reassessment during implementation exposed additional real contradictions that belong in scope because they sit on the same shared contract:
   - `GoalKind::Accuse` in [`crates/worldwake-core/src/goal.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs) and [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) still treated accusation as suspect-directed rather than register-directed. The live goal now must carry `crime_register`.
   - witnessed theft was not yet fully lawful end to end because [`crates/worldwake-systems/src/perception.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/perception.rs) did not project `Hidden` crime-transfer events into `SocialObservationDetail::SuspectedTheft`, and [`crates/worldwake-systems/src/tell_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/tell_actions.rs) still needed to materialize relayed theft evidence into the listener's `ViolationMemory`.
   - punishment planning in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) initially chose `Fine` from believed inventory quantity alone. That violated P22 because runtime fine validation requires locally accessible controlled commodity. The planner contract had to be tightened.
   - the lower-layer read behind that planner decision was also wrong: [`crates/worldwake-sim/src/per_agent_belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs) conflated local observation with recursive possession quantity, even when the holder could no longer lawfully access the lot from the current place. This ticket therefore owns the read-surface correction too.
   - `accuse` exact-bound affordances needed a clean shared affordance-target enumeration hook in [`crates/worldwake-sim/src/action_handler.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_handler.rs) and [`crates/worldwake-sim/src/affordance_query.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/affordance_query.rs) so the runtime affordance surface stays lawful after moving `accuse` to a dynamic `SpecificEntity` contract.
14. Current office authority remains place-exact. The golden therefore lawfully proves accusation-at-register plus later punishment only after the accused is brought back into the office jurisdiction place. That limitation is explicit in the scenario and should not be misdescribed as universal remote punishment.

## Architecture Check

1. The cleaner architecture is to keep `Accuse` as institutional record filing at a `CrimeRegister`, independent of suspect presence, while keeping `Fine`/`Exile` as the later co-located enforcement step. That preserves the intended separation between evidence, institutional memory, and physical enforcement.
2. This is better than staging the accused at the register just to satisfy the current action contract. That would encode an incidental implementation constraint into the golden and would fail to exercise the architecture the spec actually wants.
3. This is also better than adding a second "remote accuse" alias action beside the current one. No backward-compatibility or parallel accusation path should remain. The existing `accuse` action should be corrected in place.
4. The same reasoning applies one layer lower for affordances and commodity reads. Adding special-case punishment fallbacks or test-only goldens around stale helper semantics would leave the architecture split. The cleaner solution is to repair the shared affordance/observation surfaces so planner, runtime validation, and golden assertions all speak the same world-state language.

## Verification Layers

1. Witness theft evidence can be relayed to an authority through the live social Tell path -> decision trace plus authoritative listener belief-state assertions in the new golden
2. `Accuse` can file against a non-colocated suspect while the accuser is at the `CrimeRegister` -> focused authoritative runtime coverage in `crates/worldwake-systems/src/justice_actions.rs`
3. `get_affordances` / candidate generation still expose the expected accusation and punishment branches after the accusation contract change -> focused `justice_actions` affordance coverage for dynamic exact-bound accuse targets, focused `candidate_generation` coverage for lawful fine-vs-exile selection, plus the golden's decision-trace assertions
4. Planner/operator surface still binds and executes the accusation step lawfully -> `crates/worldwake-ai/tests/planner_conformance.rs` update/addition if needed, plus golden execution
5. `CrimeRegister` append/supersede behavior proves accusation then verdict ordering -> authoritative record assertions in the new golden
6. Punishment durable consequences prove the final branch without overfitting to scheduler timing -> authoritative commodity/faction-hostility assertions in the new golden

## What to Change

### 1. Correct the `accuse` production contract

Update `crates/worldwake-systems/src/justice_actions.rs` so accusation filing requires:

- actor at the same place as a colocated `CrimeRegister`
- subjective theft evidence against the accused
- accused believed alive
- no duplicate unresolved case

It must no longer require the accused to be co-located at accusation time. Keep punishment actions unchanged: `fine` and `exile` still require institutional authority plus co-location with the accused.

### 2. Add focused regression coverage for the corrected accusation boundary

Add or update focused tests in `crates/worldwake-systems/src/justice_actions.rs` and/or `crates/worldwake-ai/tests/planner_conformance.rs` so the repo explicitly proves:

- accusation succeeds when the suspect is absent but the accuser is at the `CrimeRegister`
- accusation still rejects without subjective evidence
- punishment still requires the accused to be co-located

### 3. Add the golden witnessed-theft justice chain

Add a new scenario in `crates/worldwake-ai/tests/golden_emergent.rs`:

**Setup**
- theft place separate from authority seat to force physical evidence travel
- thief with `TheftDispositionProfile`
- witness with `PerceptionProfile` and `TellProfile`
- authority with `JusticeDispositionProfile`, office authority, and a colocated `CrimeRegister`
- scenario isolation that prevents the authority from learning the case by direct observation or owner-local discovery

**Execution**
- thief steals
- witness acquires typed theft evidence
- witness travels or is placed so Tell can lawfully reach the authority
- authority receives relayed social evidence
- authority files accusation at the `CrimeRegister` without requiring suspect co-location
- authority later reaches the suspect and executes `Fine` or `Exile`

**Assertions**
- Tell commit precedes accusation availability
- listener belief store contains relayed theft evidence with reported provenance
- `CrimeRegister` receives `InstitutionalClaim::Accusation`
- later `InstitutionalClaim::Verdict` supersedes that accusation
- durable punishment consequences hold

### 4. Add deterministic replay companion

Standard replay companion for the new golden.

## Files to Touch

- `crates/worldwake-systems/src/justice_actions.rs`
- `crates/worldwake-ai/tests/planner_conformance.rs`
- `crates/worldwake-ai/tests/golden_emergent.rs`
- `crates/worldwake-sim/src/action_handler.rs`
- `crates/worldwake-sim/src/affordance_query.rs`
- `crates/worldwake-sim/src/per_agent_belief_view.rs`
- `crates/worldwake-systems/src/perception.rs`
- `crates/worldwake-systems/src/tell_actions.rs`
- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-ai/src/goal_model.rs`
- `crates/worldwake-core/src/goal.rs`
- `tickets/E17CRITHEJUS-013.md`

## Out of Scope

- New evidence types or Tell payload refactors already delivered by `E17CRITHEJUS-015` / `E17CRITHEJUS-016`
- Multiple concurrent crime cases
- Wrong accusation or contradictory testimony goldens
- Appeal/reversal mechanics
- Guard-patrol follow-through from E19

## Acceptance Criteria

### Tests That Must Pass

1. Focused accusation test proving filing at a `CrimeRegister` does not require suspect co-location
2. `golden_witnessed_theft_accusation_chain` (or equivalent): witnessed theft -> Tell -> accuse -> punish
3. `golden_witnessed_theft_accusation_chain_replay` deterministic replay companion
4. `CrimeRegister` contains the accusation followed by a superseding verdict for the same case
5. If `Fine` is the branch taken: conservation and transfer assertions pass
6. If `Exile` is the branch taken: faction membership removal and hostility assertions pass
7. Existing relevant suites remain green

### Invariants

1. P1: the full chain emerges from theft, perception, Tell, institutional records, and punishment actions without authored quest logic
2. P7/P13: crime knowledge travels physically through witness observation and Tell, not through omniscient authority access
3. P16/P23: accusation and verdict exist as institutional record entries, not hidden controller state
4. P21: punishment requires office authority
5. Accusation and punishment remain separate legal/action boundaries
6. No compatibility alias or duplicate accusation path remains

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/justice_actions.rs` — updated focused accusation runtime and affordance regressions, including remote suspect filing and dynamic exact-target accuse affordances
Rationale: proves the authoritative accusation contract and the shared affordance surface after moving `accuse` off `EntityAtActorPlace`.
2. `crates/worldwake-ai/tests/planner_conformance.rs` — updated `conformance_accuse`
Rationale: proves the planner/operator path still binds and commits a remote-filed accusation with the new `crime_register` goal identity.
3. `crates/worldwake-ai/src/candidate_generation.rs` — updated fine/exile candidate tests and added `justice_candidates_fall_back_to_exile_when_fine_is_not_locally_collectible`
Rationale: locks the new punishment invariant that `Fine` requires locally collectible control, not generic remote inventory belief.
4. `crates/worldwake-sim/src/per_agent_belief_view.rs` — added `locally_observed_commodity_quantity_excludes_remote_possessions`
Rationale: proves the lower-layer belief read now matches runtime lawful-access semantics for punishment planning.
5. `crates/worldwake-systems/src/perception.rs` — added `crime_transfer_item_event_records_suspected_theft`
Rationale: proves witnessed hidden crime-transfer events materialize the typed theft evidence the golden depends on.
6. `crates/worldwake-ai/src/plan_revalidation.rs` — added `specific_entity_payload_override_revalidates_with_concrete_step_target`
Rationale: exact-bound `SpecificEntity` steps would otherwise fail revalidation after the goal/action contract change.
7. `crates/worldwake-ai/tests/golden_emergent.rs` — added `golden_witnessed_theft_accusation_chain` and `golden_witnessed_theft_accusation_chain_replays_deterministically`
Rationale: proves the full witness -> Tell -> accuse -> punish chain and its deterministic replay contract.

### Commands

1. `cargo test -p worldwake-systems accuse_affordance_emits_violation_bound_payload_for_matching_suspect_observation -- --nocapture`
2. `cargo test -p worldwake-systems accuse_affordance_emits_payload_for_known_remote_suspect_observation -- --nocapture`
3. `cargo test -p worldwake-systems accusation_can_file_against_remote_known_suspect -- --nocapture`
4. `cargo test -p worldwake-systems crime_transfer_item_event_records_suspected_theft -- --nocapture`
5. `cargo test -p worldwake-sim locally_observed_commodity_quantity_excludes_remote_possessions -- --nocapture`
6. `cargo test -p worldwake-ai justice_candidates_emit_fine_punishment_from_consulted_accusation -- --nocapture`
7. `cargo test -p worldwake-ai justice_candidates_fall_back_to_exile_when_fine_is_not_locally_collectible -- --nocapture`
8. `cargo test -p worldwake-ai conformance_accuse -- --nocapture`
9. `cargo test -p worldwake-ai --test golden_emergent golden_witnessed_theft_accusation_chain -- --nocapture`
10. `cargo test --workspace`
11. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-26
- What actually changed:
  - `Accuse` was refactored into an explicit office-filing goal/action anchored to a concrete `crime_register`.
  - witnessed theft now propagates lawfully through perception and Tell into `ViolationMemory`, then into `CrimeRegister` accusation records, then into punishment.
  - punishment planning now selects `Fine` only from locally collectible control, and the belief-view helper behind that decision was corrected to exclude remote inaccessible possessions.
  - the shared affordance system gained dynamic exact-target support so `accuse` still appears lawfully in `get_affordances` after moving to `SpecificEntity`.
  - the golden and replay companion now prove the full witness-report justice chain.
- Deviations from original plan:
  - scope expanded beyond the initial accusation runtime fix because the same boundary exposed planner goal identity, exact-target affordance enumeration, witnessed theft perception, Tell-to-violation-materialization, and local-control read contradictions.
  - the final golden keeps punishment inside the office jurisdiction place because office authority is still place-exact; this ticket does not broaden punishment jurisdiction beyond the current architecture.
- Verification results:
  - focused runtime, planner, candidate-generation, and belief-view regressions passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
