# S33OPPSCOGOAIDE-005: Remove first-per-goal planning admission and search ranked opportunities directly

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — candidate selection boundary changes
**Deps**: S33OPPSCOGOAIDE-002, S33OPPSCOGOAIDE-004

## Problem

`S33OPPSCOGOAIDE-002` removed candidate-generation aliasing, but it intentionally left a temporary planning-time dedup in place: only the first ranked candidate per `GoalKey` is searched. That preserved budget behavior during the transition, but after `S33OPPSCOGOAIDE-004` it now recreates the exact architectural contradiction S33 is trying to remove: a concrete opportunity can still suppress its sibling after ranking, even though generation, ranking, and exhaustion are already opportunity-scoped.

The shared abstraction boundary under audit is:

- ranking output: `Vec<RankedGoal>`
- planning input: the subset of ranked candidates forwarded by `build_candidate_plans()`

Today those layers disagree on the canonical identity. Ranking is opportunity-scoped; planning admission is still desire-scoped.

## Assumption Reassessment (2026-03-28)

1. `GroundedGoal.anchor` and per-opportunity candidate emission already exist in live code from archived `S33OPPSCOGOAIDE-002`. This ticket is not introducing opportunity-scoped candidates from scratch.
2. `S33OPPSCOGOAIDE-004` is now complete and archived. Live code in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) and [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs) already keys exhaustion by `OpportunityKey`, so the remaining limitation is no longer cache identity; it is admission ordering.
3. The current planning pipeline still performs a temporary first-per-`GoalKey` collapse inside `build_candidate_plans()` via the `seen_goals` gate in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs). That is the actual behavior this ticket replaces.
4. Decision traces already carry per-attempt `OpportunityAnchor` provenance from archived `S33OPPSCOGOAIDE-010` and the live traced planning path in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs). Because `build_candidate_plans()` still stops at the first found plan, trace clarity for this ticket depends on preserving ranked attempt order, not on adding a new selected-plan identity field.
5. This ticket is about planning admission and same-pass fallthrough, not planning snapshot isolation. Candidate-local snapshot scope has already landed in archived `S33OPPSCOGOAIDE-010`, so the remaining live contradiction is strictly the first-per-`GoalKey` admission gate.
6. Adjacent contradictions already covered elsewhere should stay split: `PlannedPlan.opportunity` remains owned by `S33OPPSCOGOAIDE-006`, persistence format bump by `S33OPPSCOGOAIDE-008`, and end-to-end autonomous switching proof by `S33OPPSCOGOAIDE-009`.
7. Mismatch + correction: [`specs/S33-opportunity-scoped-goal-identity.md`](/home/joeloverbeck/projects/worldwake/specs/S33-opportunity-scoped-goal-identity.md) still describes a post-rank "select one surviving opportunity per `GoalKey` before search" stage. Live code and this ticket show that stage is now the architectural bug, not the target end state. The ticket should implement direct ranked opportunity admission and note the spec drift rather than preserving the stale dedup narrative in code.
8. `plan_selection.rs` does not currently impose an additional same-goal admission collapse. The live hard gate is in `build_candidate_plans()`. This ticket should not broaden scope into plan selection unless implementation proves a concrete downstream bug rather than a hypothetical one.

## Architecture Check

1. The canonical transport path after this change is opportunity identity flowing from ranking into plan search admission. The duplicate lawful path, desire-level pre-search collapse, must be removed in scope rather than left beside the new path.
2. The cleaner architecture is: rank opportunities, admit them in rank order, skip only exhausted opportunities, and stop when a plannable opportunity succeeds. That preserves explicit competition between concrete sources instead of hiding it behind a temporary dedup heuristic.
3. `GoalKey` still belongs in high-level motive accounting and frame continuity, but not as a hidden pre-search admission key. Reusing it for both layers violates the spec’s desire/opportunity separation.
4. This change is more robust than the current architecture because it removes the last same-pass alias boundary between ranking and search without introducing a second selector or compatibility path. The runtime keeps one lawful meaning for "which opportunity was considered next": the ranked candidate stream itself.
5. No backward compatibility, no aliasing. The old first-per-`GoalKey` gate should be deleted, not kept as a fallback.

## Verification Layers

1. Admission ordering: focused planning/runtime test proving multiple ranked opportunities for one `GoalKey` are attempted in rank order, not collapsed before search.
2. Exhaustion fallthrough: focused planning/runtime test proving an exhausted higher-ranked opportunity does not suppress a lower-ranked alternative with the same `GoalKey`.
3. Decision trace provenance: focused traced-planning assertion showing `planning.attempts` enumerates distinct opportunities in ranked order and preserves iteration provenance.

## What to Change

### 1. Remove temporary first-per-`GoalKey` planning dedup

In `crates/worldwake-ai/src/agent_tick/planning.rs`, delete the transitional logic that keeps only the first ranked candidate for each `GoalKey` before search.

### 2. Make plan admission opportunity-scoped

Iterate ranked candidates in rank order and attempt plan search per concrete opportunity. A higher-ranked failed or exhausted opportunity may be skipped, but it must not suppress later opportunities with the same `GoalKey`.

### 3. Keep desire identity where it is actually canonical

`GoalKey` remains the canonical desire identity for `IntentionFrame` continuity and high-level motive accounting. It should not continue to serve as the canonical planning-admission key once opportunities have already been separated.

### 4. Keep existing trace/debug surfaces authoritative

Use the existing traced planning path in `agent_tick/planning.rs` as the canonical proof surface. Only change trace wiring if removing the gate breaks ranked-attempt provenance; do not add a parallel debug path.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — remove temporary desire-level planning dedup, iterate ranked opportunities directly, and update focused planning tests)
- `crates/worldwake-ai/src/agent_tick/tests.rs` or traced-planning tests in `planning.rs` (modify if an integration-level trace assertion is needed)

## Out of Scope

- Candidate-local planning snapshot isolation (already delivered by archived `S33OPPSCOGOAIDE-010`)
- `PlannedPlan.opportunity` data plumbing (`S33OPPSCOGOAIDE-006`)
- Save/load (`S33OPPSCOGOAIDE-008`)
- New goldens (`S33OPPSCOGOAIDE-009`)
- Spec-document cleanup for the stale post-rank dedup narrative in `specs/S33-opportunity-scoped-goal-identity.md`

## Acceptance Criteria

### Tests That Must Pass

1. Multiple ranked opportunities for the same `GoalKey` are admitted to search in rank order rather than collapsed to the first one.
2. An exhausted higher-ranked opportunity does not block a lower-ranked alternative for the same desire.
3. Traced planning output shows distinct planning attempts for the competing opportunities.
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo clippy --workspace`

### Invariants

1. Ranking stays opportunity-scoped end to end through planning admission.
2. `GoalKey` remains desire identity; it is no longer reused as a hidden planning dedup key.
3. No duplicate admission path remains after the change.

## Test Plan

### New/Modified Tests

1. [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) — `same_goal_ranked_opportunities_are_attempted_in_order`
2. [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) — `exhausted_same_goal_opportunity_does_not_block_later_sibling`
3. [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) or [`crates/worldwake-ai/src/agent_tick/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs) — traced-planning test verifying `planning.attempts` records both opportunities distinctly and in ranked order.

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai agent_tick::planning::tests::same_goal_ranked_opportunities_are_attempted_in_order`
3. `cargo test -p worldwake-ai agent_tick::planning::tests::exhausted_same_goal_opportunity_does_not_block_later_sibling`
4. `cargo test -p worldwake-ai agent_tick::planning::tests::traced_planning_records_same_goal_opportunity_attempt_order`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-28
- Actual changes:
  - removed the temporary `seen_goals` first-per-`GoalKey` planning gate from [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) so ranked sibling opportunities now reach search in order
  - preserved early stop on the first found plan, so search still avoids unnecessary lower-ranked work once a plannable opportunity succeeds
  - tightened selected-plan search provenance in the traced planning path so it follows the actually found search attempt rather than the first same-goal attempt
  - added focused coverage for ranked same-goal attempt order, exhausted-sibling fallthrough, and traced attempt ordering
- Deviations from original plan:
  - no `plan_selection.rs` changes were needed; reassessment was correct that the live hard gate was in `build_candidate_plans()`
  - no `decision_trace.rs` schema change was needed; existing trace structures were sufficient once planning-side provenance lookup was corrected
  - the stale post-rank dedup narrative in [`specs/S33-opportunity-scoped-goal-identity.md`](/home/joeloverbeck/projects/worldwake/specs/S33-opportunity-scoped-goal-identity.md) was noted in the ticket but intentionally not edited here
- Verification results:
  - `cargo test -p worldwake-ai agent_tick::planning::tests::same_goal_ranked_opportunities_are_attempted_in_order` passed
  - `cargo test -p worldwake-ai agent_tick::planning::tests::exhausted_same_goal_opportunity_does_not_block_later_sibling` passed
  - `cargo test -p worldwake-ai agent_tick::planning::tests::traced_planning_records_same_goal_opportunity_attempt_order` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo clippy --workspace` passed
  - `cargo test --workspace` passed
