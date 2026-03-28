# S33OPPSCOGOAIDE-005: Replace temporary first-per-goal planning dedup with post-rank opportunity selection

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — candidate selection boundary changes
**Deps**: S33OPPSCOGOAIDE-002, S33OPPSCOGOAIDE-004

## Problem

`S33OPPSCOGOAIDE-002` removed candidate-generation aliasing, but it intentionally left a temporary planning-time dedup in place: only the first ranked candidate per `GoalKey` is searched. That preserves old behavior at the cost of reintroducing desire-level suppression after ranking. Once exhaustion is re-keyed by `OpportunityKey`, selection must operate on ranked opportunities directly instead of collapsing them back to one candidate per desire before search.

The shared abstraction boundary under audit is:

- ranking output: `Vec<RankedGoal>`
- planning input: the subset of ranked candidates forwarded by `build_candidate_plans()`

Today those layers disagree on the canonical identity. Ranking is opportunity-scoped; planning admission is still desire-scoped.

## Assumption Reassessment (2026-03-28)

1. `GroundedGoal.anchor` and per-opportunity candidate emission already exist in live code from archived `S33OPPSCOGOAIDE-002`. This ticket is not introducing opportunity-scoped candidates from scratch.
2. `S33OPPSCOGOAIDE-004` is now complete and archived. Live code in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) and [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs) already keys exhaustion by `OpportunityKey`, so the remaining limitation is no longer cache identity; it is admission ordering.
3. The current planning pipeline still performs a temporary first-per-`GoalKey` collapse inside `build_candidate_plans()` via the `seen_goals` gate in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs). That is the actual behavior this ticket replaces.
4. Decision traces already carry per-attempt `OpportunityAnchor` provenance from archived `S33OPPSCOGOAIDE-010` and the live traced planning path in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs). The remaining trace question is whether iteration/fallthrough remains explicit enough once multiple same-desire opportunities are admitted in one pass.
5. This ticket is about admission/selection policy, not planning snapshot isolation. Candidate-local snapshot scope has already landed in archived `S33OPPSCOGOAIDE-010`, so the remaining live contradiction is strictly the first-per-`GoalKey` admission gate.
6. Adjacent contradictions already covered elsewhere should stay split: `PlannedPlan.opportunity` remains owned by `S33OPPSCOGOAIDE-006`, persistence format bump by `S33OPPSCOGOAIDE-008`, and end-to-end autonomous switching proof by `S33OPPSCOGOAIDE-009`.

## Architecture Check

1. The canonical transport path after this change is opportunity identity flowing from ranking into plan search admission. The duplicate lawful path, desire-level pre-search collapse, must be removed in scope rather than left beside the new path.
2. The cleaner architecture is: rank opportunities, admit them in rank order, and stop when a plannable non-exhausted opportunity succeeds. That preserves explicit competition between concrete sources instead of hiding it behind a temporary dedup heuristic.
3. `GoalKey` still belongs in high-level motive accounting and plan selection, but not as a hidden pre-search admission key. Reusing it for both layers violates the spec’s desire/opportunity separation.
4. No backward compatibility, no aliasing. The old first-per-`GoalKey` gate should be deleted, not kept as a fallback.

## Verification Layers

1. Admission ordering: focused planning/runtime test proving multiple ranked opportunities for one `GoalKey` are attempted in rank order, not collapsed before search.
2. Exhaustion fallthrough: focused planning/runtime test proving an exhausted higher-ranked opportunity does not suppress a lower-ranked alternative with the same `GoalKey`.
3. Decision trace provenance: focused trace assertion showing planning attempts enumerate distinct opportunities in ranked order and preserve iteration provenance.

## What to Change

### 1. Remove temporary first-per-`GoalKey` planning dedup

In `crates/worldwake-ai/src/agent_tick/planning.rs`, delete the transitional logic that keeps only the first ranked candidate for each `GoalKey` before search.

### 2. Make plan admission opportunity-scoped

Iterate ranked candidates in rank order and attempt plan search per concrete opportunity. A higher-ranked failed or exhausted opportunity may be skipped, but it must not suppress later opportunities with the same `GoalKey`.

### 3. Keep desire identity where it is actually canonical

`GoalKey` remains the canonical desire identity for `IntentionFrame` continuity and high-level motive accounting. It should not continue to serve as the canonical planning-admission key once opportunities have already been separated.

### 4. Strengthen planning traces if needed

If the current `PlanningPipelineTrace` does not clearly expose which ranked opportunity was attempted and why iteration continued, extend it enough to make the new selection contract debuggable. Do not add a second ad hoc debug path if the existing trace surface can be made authoritative with a small schema change.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — remove temporary desire-level planning dedup, iterate ranked opportunities directly)
- `crates/worldwake-ai/src/plan_selection.rs` (modify if selection assumptions still expect one plan attempt per `GoalKey`)
- `crates/worldwake-ai/src/decision_trace.rs` (modify if planning-attempt trace data needs stronger opportunity identity)

## Out of Scope

- Candidate-local planning snapshot isolation (already delivered by archived `S33OPPSCOGOAIDE-010`)
- `PlannedPlan.opportunity` data plumbing (`S33OPPSCOGOAIDE-006`)
- Save/load (`S33OPPSCOGOAIDE-008`)
- New goldens (`S33OPPSCOGOAIDE-009`)

## Acceptance Criteria

### Tests That Must Pass

1. Multiple ranked opportunities for the same `GoalKey` are admitted to search in rank order rather than collapsed to the first one.
2. An exhausted higher-ranked opportunity does not block a lower-ranked alternative for the same desire.
3. Decision traces show distinct planning attempts for the competing opportunities.
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo clippy --workspace`

### Invariants

1. Ranking stays opportunity-scoped end to end through planning admission.
2. `GoalKey` remains desire identity; it is no longer reused as a hidden planning dedup key.
3. No duplicate admission path remains after the change.

## Test Plan

### New/Modified Tests

1. [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) — `multiple_ranked_opportunities_are_attempted_in_rank_order`
2. [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) — `exhausted_higher_ranked_opportunity_falls_through_to_alternative`
3. [`crates/worldwake-ai/src/agent_tick/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs) or [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) — trace-focused test verifying `planning.attempts` records both opportunities distinctly and in ranked order.

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai agent_tick::planning::tests::`
3. `cargo test -p worldwake-ai decision_trace::tests::`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`
