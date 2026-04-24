# S13SURJUS-003: Stop stale `ask_about_person` from blocking `survival-justice` search/report

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — search/report scenario proof, ask/search runtime recovery or scenario isolation
**Deps**: `archive/tickets/S13SURJUS-001.md`, `docs/scenario-roadmap.md` row 13 `survival-justice`

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
8. Adjacent contradiction: the accusation/fine retained-case seam remains separate and is explicitly owned by `tickets/S13SURJUS-002.md`.

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

- `scenarios/survival-justice.ron` (modify if scenario isolation is the truthful fix)
- `crates/worldwake-ai/tests/golden_survival_justice.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/*` (modify if stale-request recovery is the live blocker)
- `crates/worldwake-systems/src/ask_about_person_actions.rs` (modify only if start validation or payload binding is the live blocker)
- `crates/worldwake-systems/src/search_actions.rs` (modify only if search/report admission remains false after stale-branch recovery)

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
2. Row 13 remains `In Progress` until search/report is proven at the scenario level.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_survival_justice.rs` — add truthful search/report coverage after the stale-branch owner is fixed
2. `<runtime or action test path decided by reassessment>` — focused proof for the stale-branch boundary if the fix is not scenario-only

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_justice -- --list`
2. `cargo test -p worldwake-ai --test golden_survival_justice <exact search/report test> -- --ignored --exact --test-threads=1`
3. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_replays_deterministically -- --ignored --exact --test-threads=1`
