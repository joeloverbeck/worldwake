# E16DPOLPLAN-019: BlockedIntent for failed threats (`ThreatenResisted`)

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: E16DPOLPLAN-004

## Problem

This ticket originally assumed resisted `threaten` actions still needed a new blocked-intent memory path to stop repeated futile political plans. That is no longer true in the current architecture.

The later E16d planner work already changed the relevant behavior at the planning seam: `PlannerOpKind::Threaten` now has explicit claim-office semantics, and a resist outcome leaves the planning state unchanged. With current code, the planner already rejects threaten plans against targets whose courage is too high relative to the actor's attack skill.

## Assumption Reassessment (2026-03-18)

1. `PlannerOpKind::Threaten` is already modeled in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) through `apply_threaten_for_office()`, and `apply_planner_step()` routes `GoalKind::ClaimOffice` threaten ops through that helper. Corrected scope: the old "planner lacks threaten semantics" premise is obsolete.
2. `apply_threaten_for_office()` already encodes the resist case as a no-op planning transition when `attack_skill <= courage`. Focused coverage already exists in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs):
   - `goal_model::tests::planner_selects_threaten_plan`
   - `goal_model::tests::planner_rejects_threaten_against_high_courage`
   - `goal_model::tests::threaten_resist_unchanged`
3. Golden political coverage already exercises the same architecture in [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs) via `golden_threaten_with_courage_diversity`, where a low-courage target yields and a high-courage target does not.
4. `commit_threaten()` in [`crates/worldwake-systems/src/office_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs) does not surface resistance as a plan failure. It commits successfully, then either increases loyalty or adds hostility. Corrected scope: `handle_plan_failure()` in [`crates/worldwake-ai/src/failure_handling.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/failure_handling.rs) is the wrong seam for this outcome because it is only reached from actual step failure / replan paths.
5. `BlockedIntentMemory::is_blocked()` in [`crates/worldwake-core/src/blocked_intent.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/blocked_intent.rs) only keys on `GoalKey`, live TTL, and `blocks_goal_generation()`. It does not consult `related_entity`. For `GoalKind::ClaimOffice`, that means a new blocker recorded after one resisted threaten would suppress the whole office-claim goal, including alternative targets and non-threaten claim paths, not just the failed threaten target.
6. The only current target/action-specific blocker path is the exclusive-facility snapshot/search filter in [`crates/worldwake-ai/src/planning_snapshot.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs), which derives blocked `(related_entity, related_action)` pairs for `BlockingFact::ExclusiveFacilityUnavailable`. There is no generic political-target affordance blocker today, and adding a goal-level `ThreatenResisted` blocker would be architecturally broader than intended.
7. Existing blocked-intent coverage already proves the current suppression seam behaves at the goal layer, not the target layer:
   - `candidate_generation::tests::social_candidates_require_tell_profile_and_respect_blocked_memory`
   - `golden_blocked_intent_memory_with_ttl_expiry`

## Architecture Check

1. The proposed change in the original ticket is not better than the current architecture. Recording `ThreatenResisted` in `handle_plan_failure()` is misplaced because resisted threaten is an authoritative successful commit outcome, not a plan failure.
2. A new goal-level `BlockingFact::ThreatenResisted` would be worse than the current design. Because blocked-intent suppression currently happens by `GoalKey`, it would overblock the full `ClaimOffice` goal for that office instead of only suppressing one failed threaten target. That would silently suppress valid alternatives such as bribing another supporter, threatening a different low-courage target, or simply declaring support directly.
3. The clean future architecture, if a real retry loop reappears, is not a new `BlockingFact` plus candidate-generation suppression. It is a target-specific affordance/planner-candidate blocker integrated at the planning snapshot/search seam, analogous to the existing blocked facility-use path that keys on `(related_entity, related_action)` rather than the whole goal.
4. Given the current planner semantics and existing focused plus golden coverage, no engine or test code change is justified today. The better architectural outcome is to retire this ticket as already satisfied by later E16d work.

## Scope Correction

1. Do not add `BlockingFact::ThreatenResisted`.
2. Do not change `handle_plan_failure()`.
3. Do not add new blocked-intent suppression for `ClaimOffice`.
4. Verify the existing planner, blocked-intent, and golden-political tests still pass.
5. Verify full workspace tests and lint still pass.
6. Archive this ticket as already satisfied by the current architecture.

## Files Touched

- `tickets/E16DPOLPLAN-019.md`

## Out of Scope

- Production-code changes in `blocked_intent.rs`
- Production-code changes in `failure_handling.rs`
- New golden scenarios for threaten resistance
- Generalized affordance-target blocker infrastructure

## Acceptance Criteria

### Tests That Must Pass

1. `goal_model::tests::planner_selects_threaten_plan`
2. `goal_model::tests::planner_rejects_threaten_against_high_courage`
3. `goal_model::tests::threaten_resist_unchanged`
4. `golden_threaten_with_courage_diversity`
5. `candidate_generation::tests::social_candidates_require_tell_profile_and_respect_blocked_memory`
6. `golden_blocked_intent_memory_with_ttl_expiry`
7. `cargo test --workspace`
8. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Threaten planning remains outcome-based: yield adds hypothetical support, resist leaves planning state unchanged.
2. Resisted threaten is not misclassified as a plan failure.
3. Blocked-intent suppression remains goal-scoped unless and until a deliberate target-scoped planner blocker is introduced.
4. `ClaimOffice` alternatives are not silently suppressed by a single failed threaten target.

## Tests

### New/Modified Tests

1. None. Existing focused and golden coverage already capture the intended architecture.

### Existing Tests Relied On

1. `goal_model::tests::planner_selects_threaten_plan`
   Rationale: proves the planner can still choose threaten when the actor's pressure can legitimately yield support.
2. `goal_model::tests::planner_rejects_threaten_against_high_courage`
   Rationale: proves the planner already rejects the high-courage resist case that this ticket originally tried to dampen after the fact.
3. `goal_model::tests::threaten_resist_unchanged`
   Rationale: locks the key planning invariant that a resist outcome does not advance `ClaimOffice`.
4. `golden_threaten_with_courage_diversity`
   Rationale: verifies the full political pipeline still produces divergent yield/resist behavior in an end-to-end office scenario.
5. `candidate_generation::tests::social_candidates_require_tell_profile_and_respect_blocked_memory`
   Rationale: confirms blocked-intent suppression still lives at the goal-candidate layer.
6. `golden_blocked_intent_memory_with_ttl_expiry`
   Rationale: confirms blocked-intent memory retains its current TTL behavior without introducing a new political blocker path.

## Test Plan

### Commands

1. `cargo test -p worldwake-ai planner_selects_threaten_plan`
2. `cargo test -p worldwake-ai planner_rejects_threaten_against_high_courage`
3. `cargo test -p worldwake-ai threaten_resist_unchanged`
4. `cargo test -p worldwake-ai golden_threaten_with_courage_diversity`
5. `cargo test -p worldwake-ai social_candidates_require_tell_profile_and_respect_blocked_memory`
6. `cargo test -p worldwake-ai golden_blocked_intent_memory_with_ttl_expiry`
7. `cargo test --workspace`
8. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-18
- What actually changed: corrected the ticket to match the current E16d architecture; no engine or test code changes were necessary because later planner work already removed the original retry-loop premise.
- Deviations from original plan:
  - dropped the proposed `BlockingFact::ThreatenResisted` variant
  - dropped the proposed `failure_handling.rs` change because resisted threaten is not a failure-path event
  - documented the cleaner future architecture if this class of bug ever returns: target-specific affordance suppression at the planning snapshot/search seam, not goal-level blocked-intent suppression
- Verification results:
  - `cargo test -p worldwake-ai planner_selects_threaten_plan` ✅
  - `cargo test -p worldwake-ai planner_rejects_threaten_against_high_courage` ✅
  - `cargo test -p worldwake-ai threaten_resist_unchanged` ✅
  - `cargo test -p worldwake-ai golden_threaten_with_courage_diversity` ✅
  - `cargo test -p worldwake-ai social_candidates_require_tell_profile_and_respect_blocked_memory` ✅
  - `cargo test -p worldwake-ai golden_blocked_intent_memory_with_ttl_expiry` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
