# S13SURJUS-003: Stop stale `ask_about_person` from blocking `survival-justice` search/report

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planner search/report branch selection, search/report scenario proof
**Deps**: `archive/tickets/S13SURJUS-001.md`, `archive/tickets/S13SURJUS-006.md`, `docs/scenario-roadmap.md` row 13 `survival-justice`

## Problem

The search/report half of `survival-justice` never reaches `search_place` or `report_found`. `Searcher Ivo` commits one `ask_about_person`, then spends the rest of the run retrying stale `ask_about_person` bindings that fail with `PreconditionFailed("ExactIdentityRequired")`, leaving the missing-person expectation overdue for all 1440 ticks.

## Assumption Reassessment (2026-04-24)

1. The owned boundary here is not the accusation pipeline. It is the search/report branch from expectation response through `GoalKind::SearchForMissing`, `search_place`, and `GoalKind::ReportFound`, including any stale-request recovery if the selected branch gets stuck on `ask_about_person`.
2. Focused golden diagnostics on 2026-04-24 showed `Searcher Ivo` committed `ask_about_person` once at ticks 0-1 and then repeatedly failed to start the same action with `ExactIdentityRequired`; no `search_place` or `report_found` commit ever appeared.
3. Final authoritative state at the end of the same run left the expectation store still `Overdue` and last-seen memory unchanged at tick 0, confirming the live failure is earlier than report writing.
4. The first live failure boundary appears to be mixed-layer: either scenario authoring keeps `ask_about_person` alive when it should yield to search, or runtime/AI stale-binding recovery is not dropping a dead `ask_about_person` branch cleanly enough to let `SearchForMissing` win.
5. The relevant live goal families are `GoalKind::SearchForMissing`, `GoalKind::ReportFound`, and the competing `ask_about_person` branch. Reassessment should name the exact operator/prerequisite surface the stale branch is preserving.
6. Because the repeated failures are start-time `ExactIdentityRequired` rejections, this ticket must verify the authoritative start boundary, then confirm `tick_step`/plan-failure recovery actually releases that dead branch instead of reproducing it indefinitely.
7. If a purely authored fix can truthfully remove the stale `ask_about_person` competitor without weakening row 13, prefer that scenario isolation. If stale-request recovery is the real blocker, prove and fix it at the runtime/AI boundary instead.
8. Adjacent contradiction: the accusation/fine retained-case seam remains separate and is explicitly owned by `archive/tickets/S13SURJUS-002.md`.
9. Live implementation disproved the scenario-only hypothesis. Increasing the authored witness-query duration still left `Searcher Ivo` committing `ask_about_person`, then produced 1098 `ExactIdentityRequired` request-resolution rejections and no `search_place` / `report_found` commit in the focused golden.
10. The truthful owner was the planner operator-availability boundary in `crates/worldwake-ai/src/goal_model.rs`: a `GoalKind::SearchForMissing` whose `last_seen` place is the actor's current local place should not keep `PlannerOpKind::AskAboutPerson` available ahead of direct `PlannerOpKind::SearchPlace`.
11. `ask_about_person` remains available for non-local last-seen search goals, preserving the remote inquiry branch instead of deleting the social-query operator family.
12. With the local ask branch unavailable, the retained scenario commits `search_place`, resolves Searcher Ivo's expectation as `FoundSafe`, commits `report_found`, and writes a `MissingPersonStatus::FoundSafe` claim to the local office register.
13. Row 13 can now be marked `Landed` because the prior accusation/fine seams and this search/report seam are all proved in `golden_survival_justice.rs`.

## Architecture Check

1. Fixing the stale-branch owner is cleaner than forcing search/report through a golden-only scripted request because row 13 needs the autonomous searcher branch to be truthful.
2. No fallback shim should preserve parallel search/report proofs. The row should land through one honest retained branch.

## Verification Layers

1. The stale `ask_about_person` branch no longer monopolizes the run after its first failure boundary -> decision trace / runtime failure handling proof
2. `Searcher Ivo` commits `search_place` for the authored missing-person case -> action trace
3. The found outcome produces a truthful `report_found` branch and updates the intended register/listener state -> action trace plus authoritative expectation/register state
4. The searcher still satisfies the authored survival envelope while the search/report branch wins -> `golden_survival_justice.rs`

## What to Change

### 1. Reassess the searcher branch owner

Determine whether the fix belongs in scenario isolation, `ask_about_person` admission/retention, or AI stale-branch recovery after `ExactIdentityRequired` start failures.

### 2. Land truthful search/report proof

Once the retained branch is honest, expand `crates/worldwake-ai/tests/golden_survival_justice.rs` to prove `search_place` and `report_found` for the authored missing-person case.

## Files to Touch

- `crates/worldwake-ai/tests/golden_survival_justice.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `docs/scenario-roadmap.md` (modify)
- `docs/generated/golden-*.md` (regenerate)

## Out of Scope

- Accusation/fine retained-case work
- Golden-only scripted search/report requests that bypass the autonomous branch

## Acceptance Criteria

### Tests That Must Pass

1. A row-13 golden in `crates/worldwake-ai/tests/golden_survival_justice.rs` that proves `search_place` commits for the authored missing-person case
2. The same golden proves the follow-on `report_found` commit and the intended found-status write
3. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`

### Invariants

1. A dead `ask_about_person` branch must not indefinitely suppress the truthful search/report branch after repeated exact-identity start failures.
2. Row 13 is marked `Landed` only after search/report is proven at the scenario level.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_survival_justice.rs` — add truthful search/report coverage after the stale-branch owner is fixed
2. `crates/worldwake-ai/src/goal_model.rs` — focused proof that local last-seen search suppresses `ask_about_person` while remote last-seen search still permits it

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_justice -- --list`
2. `cargo test -p worldwake-ai --lib goal_model::tests::search_for_missing_marks_ask_about_person_unavailable_when_last_seen_is_local -- --exact`
3. `cargo test -p worldwake-ai --lib goal_model::tests::search_for_missing_rejects_ask_about_person_payload_when_last_seen_is_local -- --exact`
4. `cargo test -p worldwake-ai --lib goal_model::tests::search_for_missing_keeps_ask_about_person_payload_when_last_seen_is_remote -- --exact`
5. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_search_and_report_found -- --ignored --exact --test-threads=1`
6. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`
7. `python3 scripts/golden_inventory.py --write --check-docs`

## Outcome

Completed on 2026-04-24.

- Fixed the planner-owned local search branch in `GoalKindPlannerExt`: when a missing subject's last-seen place is the actor's current place, `PlannerOpKind::AskAboutPerson` is unavailable and the direct `search_place` branch remains available.
- Preserved the remote inquiry branch by keeping `ask_about_person` available for non-local last-seen search goals.
- Expanded `golden_survival_justice.rs` with Scenario 178, proving `Searcher Ivo` commits `search_place`, resolves the expectation as found safe, commits `report_found`, writes the local office-register found-status claim, and records zero `ExactIdentityRequired` request-resolution rejections.
- Regenerated golden inventory/docs and updated `docs/scenario-roadmap.md` so row 13 is now `Landed`.

## Deviations

- Reassessment first considered scenario isolation and runtime stale-request recovery. A temporary authored epistemic-profile change was tested and discarded because it still left 1098 exact-identity rejections in the focused golden; the landed fix is in planner operator availability instead.
- No `scenarios/survival-justice.ron`, `agent_tick`, `ask_about_person_actions.rs`, or `search_actions.rs` edit was needed in the final patch.

## Verification Result

- Passed `cargo fmt --all`
- Passed `cargo test -p worldwake-ai --lib search_for_missing -- --test-threads=1`
- Passed `cargo test -p worldwake-ai --lib goal_model::tests::search_for_missing_marks_ask_about_person_unavailable_when_last_seen_is_local -- --exact`
- Passed `cargo test -p worldwake-ai --lib goal_model::tests::search_for_missing_keeps_ask_about_person_payload_when_last_seen_is_remote -- --exact`
- Passed `cargo test -p worldwake-ai --lib goal_model::tests::search_for_missing_rejects_ask_about_person_payload_when_last_seen_is_local -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_survival_justice -- --list`
- Passed `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_accusation_substrate -- --ignored --exact --test-threads=1`
- Passed `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_fine_punishment_for_same_theft_case -- --ignored --exact --test-threads=1`
- Passed `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_search_and_report_found -- --ignored --exact --test-threads=1`
- Passed `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
