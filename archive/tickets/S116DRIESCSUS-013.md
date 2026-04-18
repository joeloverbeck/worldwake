# S116DRIESCSUS-013: Reconcile same-goal acquisition regressions after survival-stability repairs

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: No — reassessment proved the live planner/acquisition boundary was already correct and the failing assertions were stale
**Deps**: tickets/S116DRIESCSUS-011.md, archive/tickets/S116DRIESCSUS-012.md

## Problem

Post-review of `S116DRIESCSUS-012` confirmed that its broader `cargo test -p worldwake-ai --lib` failure was not generic fallout. Four isolated `worldwake-ai` unit tests still fail in the same self-consume acquisition domain:

- `agent_tick::planning::tests::same_goal_planning_trace_records_candidate_cap_stop_reason`
- `agent_tick::planning::tests::same_goal_ranked_opportunities_are_attempted_in_order`
- `agent_tick::planning::tests::traced_planning_records_same_goal_opportunity_attempt_order`
- `agent_tick::tests::unseen_seller_relocation_preserves_stale_acquisition_belief`

These failures sit outside `012`'s authored-threshold and `Sleep` progress-barrier seam, but they are still real and architecture-relevant. The repo needs a bounded owner to determine whether the live same-goal acquisition contract regressed or whether those tests now overstate the post-`011` / post-`012` planner contract.

Any solution must align with [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md): one canonical acquisition information path, belief-backed remote planning plus authoritative local visibility, no workaround aliases, and no fabricated same-goal trace provenance that diverges from the real selected/attempted search path.

## Assumption Reassessment (2026-04-18)

1. The motivating proof surface is concrete and current, not speculative. During post-ticket review for `S116DRIESCSUS-012`, these exact isolated reruns failed on the live branch:
   - `cargo test -p worldwake-ai --lib agent_tick::planning::tests::same_goal_planning_trace_records_candidate_cap_stop_reason -- --exact`
   - `cargo test -p worldwake-ai --lib agent_tick::planning::tests::same_goal_ranked_opportunities_are_attempted_in_order -- --exact`
   - `cargo test -p worldwake-ai --lib agent_tick::planning::tests::traced_planning_records_same_goal_opportunity_attempt_order -- --exact`
   - `cargo test -p worldwake-ai --lib agent_tick::tests::unseen_seller_relocation_preserves_stale_acquisition_belief -- --exact`
2. The first three failures are all in `crates/worldwake-ai/src/agent_tick/planning.rs` and concern the same-goal sibling attempt/trace contract:
   - one test now sees `continuation_trigger: None` where it expected a concrete `OpportunityKey`
   - two tests still expect the first sibling opportunity to be searched/found in order
3. The fourth failure is in `crates/worldwake-ai/src/agent_tick/tests.rs` and shows the ranked goal set no longer contains `GoalKind::AcquireCommodity { commodity: Bread, purpose: SelfConsume }` for the stale unseen-seller relocation fixture.
4. Exact shared boundary under audit is mixed-layer:
   - same-goal sibling opportunity ordering and trace recording in `agent_tick::planning`
   - stale acquisition belief retention and candidate/ranking visibility in `agent_tick` / acquisition planning
   - any relevant production carrier beneath those tests must be named precisely during implementation reassessment before code changes
5. This ticket must not assume the tests are correct or incorrect up front. Reassessment must determine whether:
   - `S116DRIESCSUS-011` and `S116DRIESCSUS-012` exposed a real production regression in same-goal acquisition behavior or traceability, or
   - the tests now over-claim a pre-fix continuation/visibility contract that is no longer lawful on the live branch
6. The same-goal trace failures are in the same planner/search family named in [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md): same-goal sibling planning stop provenance, selected-plan provenance, and candidate-attempt ordering. Reassessment must cite the live planner contract there before changing assertions or production code.
7. The stale seller-belief failure sits adjacent to `S116DRIESCSUS-011`'s “authoritative local, belief-backed remote” repair, so the ticket must explicitly classify whether the missing `AcquireCommodity(Bread)` goal is:
   - a lawful consequence of the repaired information boundary, or
   - an unintended loss of remote stale-belief pursuit that still should exist
8. Reassessment outcome: the four failing tests were stale against the live contract.
   - `docs/planner-contracts.md` already defines `PlanSearchTrace.attempts` as the authoritative admitted-attempt order and `SameGoalPlanningTrace.continuation_trigger` as `None` when no admitted sibling actually finds a plan.
   - the live `summarize_same_goal_planning_trace(...)` implementation in `crates/worldwake-ai/src/agent_tick/planning.rs` already matches that contract.
   - `unseen_seller_relocation_preserves_stale_acquisition_belief` was over-claiming that a stale same-place seller belief should keep a local `AcquireCommodity(SelfConsume Bread)` goal visible after authoritative local state no longer contains that seller; under `S116DRIESCSUS-011`'s repaired boundary, the stale belief may persist until refresh, but the local acquisition goal must not.

## Architecture Check

1. A dedicated follow-up ticket was cleaner than folding these four failures into `012` after the fact, because `012` truthfully owned authored-threshold drift and `Sleep` progress-barrier recovery, not same-goal acquisition continuation or unseen-seller stale-belief semantics.
2. The honest end state is either:
   - production behavior/trace repaired so the same-goal and stale-belief contract remains lawful, or
   - tests narrowed to the live planner contract after reassessment
   In both cases, there must be one canonical trace and acquisition-path story, not dual “runtime truth” and “test-only expectation” semantics.
3. The landed result is the cleaner second path: no new planner alias or acquisition shim was introduced, and the tests now assert the one already-shipped planner/acquisition contract.

## Verification Layers

1. Same-goal sibling continuation/stop provenance -> focused `worldwake-ai` unit tests in `agent_tick::planning` aligned to `docs/planner-contracts.md`
2. Ranked acquisition candidate visibility under unseen-seller relocation -> focused `worldwake-ai` unit test in `agent_tick::tests`
3. Lower production symbol changes -> not needed; reassessment showed the live planner/acquisition boundary was already correct
4. Broad crate confirmation -> `cargo test -p worldwake-ai --lib`

## What to Change

### 1. Reassess the live same-goal planning contract

Audit the exact same-goal sibling ordering / continuation-trigger / selected-attempt contract in `crates/worldwake-ai/src/agent_tick/planning.rs` against `docs/planner-contracts.md` and the current production code.

### 2. Reassess stale unseen-seller acquisition visibility

Audit whether the unseen-seller relocation fixture still lawfully owns a stale remote `AcquireCommodity(SelfConsume Bread)` candidate after the `011` local-authority repair, or whether the test now overstates the live belief/acquisition contract.

### 3. Repair the truthful boundary

If production behavior regressed, fix it at the canonical planner/acquisition boundary. If the tests are stale, rewrite them so they prove the live lawful contract without fabricating obsolete provenance or stale-goal visibility.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify focused same-goal trace assertions)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify focused stale-seller acquisition assertion)
- `crates/worldwake-ai/src/...` exact lower-layer symbols under reassessed ownership (modify only if the tests expose a real deeper regression)
- `docs/planner-contracts.md` (modify only if reassessment proves factual drift in the active planner contract text)

## Out of Scope

- Reopening `S116DRIESCSUS-012`'s authored-threshold or `Sleep` progress-barrier work
- Reopening `S116DRIESCSUS-011`'s already-fixed stale local `TargetAtActorPlace(0)` loop unless reassessment proves the same root cause still survives
- Golden scenario or scenario-authoring changes in `scenarios/`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::same_goal_planning_trace_records_candidate_cap_stop_reason -- --exact`
2. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::same_goal_ranked_opportunities_are_attempted_in_order -- --exact`
3. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::traced_planning_records_same_goal_opportunity_attempt_order -- --exact`
4. `cargo test -p worldwake-ai --lib agent_tick::tests::unseen_seller_relocation_preserves_stale_acquisition_belief -- --exact`
5. `cargo test -p worldwake-ai --lib`

### Invariants

1. Same-goal trace/ordering assertions must match the live planner contract named in `docs/planner-contracts.md`
2. Acquisition visibility must preserve the canonical information boundary: authoritative local, belief-backed remote, no duplicate stale-local alias path

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` — same-goal sibling ordering / stop-provenance expectations aligned to the live contract
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — unseen-seller stale-acquisition expectation aligned to the live contract
3. Additional lower-layer focused tests only if reassessment proves the regression boundary is deeper than the existing failing tests

### Commands

1. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::same_goal_planning_trace_records_candidate_cap_stop_reason -- --exact`
2. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::same_goal_ranked_opportunities_are_attempted_in_order -- --exact`
3. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::traced_planning_records_same_goal_opportunity_attempt_order -- --exact`
4. `cargo test -p worldwake-ai --lib agent_tick::tests::unseen_seller_relocation_preserves_stale_acquisition_belief -- --exact`
5. `cargo test -p worldwake-ai --lib`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completion date: 2026-04-18

Reassessment showed that `S116DRIESCSUS-013` did not uncover a new production regression. The live planner/search contract already matched `docs/planner-contracts.md`: admitted same-goal attempts are recorded in `PlanSearchTrace.attempts`, and `SameGoalPlanningTrace.continuation_trigger` stays `None` when no admitted sibling actually finds a plan. The three `agent_tick::planning` failures were stale assertions against that already-shipped contract.

The fourth failing test was also stale after `S116DRIESCSUS-011`. A stale unseen seller belief may survive until perception refresh, but once authoritative local state no longer contains that seller, a local `AcquireCommodity(SelfConsume Bread)` goal must not remain visible. The ticket therefore landed as a focused test-contract correction in `crates/worldwake-ai/src/agent_tick/planning.rs` and `crates/worldwake-ai/src/agent_tick/tests.rs`, with no production code change beneath those surfaces.

## Deviations

1. The draft ticket was intentionally written as “production fix or stale-test correction depending on reassessment.” Reassessment resolved decisively to the stale-test path, so `Engine Changes` is now `No`.
2. `docs/planner-contracts.md` did not need edits. The live planner contract text was already correct; only the focused assertions had drifted away from it.

## Verification Result

1. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::same_goal_planning_trace_records_candidate_cap_stop_reason -- --exact`
2. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::same_goal_ranked_opportunities_are_attempted_in_order -- --exact`
3. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::traced_planning_records_same_goal_opportunity_attempt_order -- --exact`
4. `cargo test -p worldwake-ai --lib agent_tick::tests::unseen_seller_relocation_preserves_stale_acquisition_belief -- --exact`
5. `cargo test -p worldwake-ai --lib`
6. `cargo clippy --workspace --all-targets -- -D warnings`
