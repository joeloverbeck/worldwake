# S31-004: Replace coarse exhausted-goal reset with goal-aware invalidation

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — AI planning pipeline
**Deps**: S31-001, S31-002, S31-003, [specs/S31-goal-aware-exhaustion-invalidation.md](/home/joeloverbeck/projects/worldwake/specs/S31-goal-aware-exhaustion-invalidation.md)

## Problem

The live planner still clears exhausted-goal TTL state through `reset_exhausted_goals_if_needed`, a coarse dirty-mask path in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs). That architecture ignores the per-goal invalidation model already implemented in [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs), so exhausted goals are recorded with empty condition/baseline data and then globally re-enabled on unrelated world changes.

## Assumption Reassessment (2026-03-27)

1. The shared boundary under audit is `AgentDecisionRuntime.exhaustion_cache: BTreeMap<GoalKey, ExhaustionEntry>` between planner runtime flow in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) and goal-aware invalidation helpers in [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs).
2. The ticket’s original assumption that S31-004 needed to introduce `ExhaustionInvalidationCondition`, `ExhaustionBaseline`, and the extended `ExhaustionEntry` was stale. Those types already exist in [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs) and [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs), with focused helper coverage already present.
3. The live integration gap is twofold: `record_exhausted_goals` in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) still inserts exhausted entries with `invalidation_conditions: Vec::new()` and `baseline: ExhaustionBaseline::default()`, and `plan_and_validate_next_step` still calls `reset_exhausted_goals_if_needed` instead of per-entry invalidation.
4. The live helper `derive_invalidation_conditions` is currently unused in production flow. The clean fix is to make exhausted-goal recording populate the cache with goal-derived conditions and baseline snapshots at the moment the planner observes exhaustion, then let a single invalidation pass remove only entries whose recorded conditions fired.
5. The live `condition_changed` helper already takes runtime-derived `facilities_changed` and `blocker_expired` booleans. That means S31-004 should keep facility/blocker dirtiness as an explicit planner input rather than re-deriving facility signatures or blocker cleanup semantics inside exhaustion logic.
6. The live planner contract for transit-sensitive invalidation is still the same as the old reset path: `PositionChanged` must not fire mid-transit. This remains an `agent_tick` runtime contract, not a generic helper-only concern.
7. Existing focused coverage already proves helper semantics such as threshold deltas, target death, hostile count, facility/blocker flags, and deterministic baseline derivation in [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs). What is missing is planner integration coverage proving that runtime invalidation actually uses those helpers and preserves backoff/count semantics correctly.
8. Current targeted runtime tests such as [`irrelevant_commodity_change_does_not_trigger_replan_for_sleep_goal`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs#L3055) and [`relevant_commodity_change_triggers_replan_for_consume_goal`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs#L3108) only verify dirty-set observation. They do not prove exhausted-goal cache invalidation behavior.
9. The live test inventory confirms the exact focused surfaces available for this ticket: `agent_tick::planning::tests::*`, `exhaustion::tests::*`, and the targeted `agent_tick` runtime tests listed by `cargo test -p worldwake-ai -- --list`.
10. Empty `invalidation_conditions` entries are still a lawful persisted runtime state today because live production code writes them. S31-004 should not assume S31-005 has already removed that state. This ticket’s scope is to stop producing fresh empty entries in normal planning flow; any migration or post-load tightening remains separate.
11. The live `GoalKind` surface for this ticket is not one single planner family. The architecture change must work for every goal family already covered by `derive_invalidation_conditions`, while focused regression tests should prove representative branches such as `ConsumeOwnedCommodity`, `Sleep`, and position-sensitive invalidation.
12. Reassessment changed the scope: this ticket is not “build the invalidation model,” it is “wire the already-built invalidation model into authoritative planner runtime flow and delete the coarse reset path.”

## Architecture Check

1. The cleaner architecture is one source of truth for exhaustion semantics in `exhaustion.rs`: derive invalidation conditions when recording exhaustion, and evaluate them when reconsidering cached entries. That removes the current split-brain design where the helper layer knows per-goal conditions but the planner ignores them.
2. Keeping facility/blocker dirtiness as planner-provided booleans preserves a clean boundary. Observation/runtime owns dirty-domain detection; exhaustion logic consumes those facts. This avoids duplicating facility-signature comparison or blocker-expiry policy in a second subsystem.
3. Deleting `reset_exhausted_goals_if_needed` is strictly better than wrapping it. A compatibility layer would preserve the stale “clear everything on broad dirtiness” architecture that S31 is explicitly replacing.

## Verification Layers

1. Per-goal invalidation removes only entries whose recorded conditions fired -> focused unit tests in [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs) and [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs)
2. Planner runtime records lawful conditions/baselines at exhaustion time instead of empty placeholders -> focused unit test in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs)
3. Transit-sensitive `PositionChanged` behavior remains “no invalidation mid-transit” -> focused unit test in [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs) and planner integration test in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs)
4. Existing `agent_tick` runtime behavior remains stable outside exhaustion invalidation -> targeted `cargo test -p worldwake-ai agent_tick`
5. AI crate regression safety after the planner integration -> `cargo test -p worldwake-ai`
6. This is an `agent_tick` runtime ticket. Additional action-trace or event-log mapping is not applicable because the invariant under change is planner cache invalidation before action start.

## What to Change

### 1. Add planner-facing invalidation helper in `crates/worldwake-ai/src/exhaustion.rs`

Introduce `invalidate_exhausted_goals(...)` alongside the existing helper functions. It should iterate the exhaustion cache and remove only entries whose recorded invalidation conditions fired via `condition_changed(...)`.

Inputs should include:

- `&mut BTreeMap<GoalKey, ExhaustionEntry>`
- `&dyn GoalBeliefView`
- `agent: EntityId`
- `currently_in_transit: bool`
- `facilities_changed: bool`
- `blocker_expired: bool`

This stays a pure invalidation pass. It should not recompute facility signatures or mutate unrelated runtime state.

### 2. Replace coarse reset usage in `crates/worldwake-ai/src/agent_tick/planning.rs`

Delete `reset_exhausted_goals_if_needed` and replace its call site in `plan_and_validate_next_step` with the new goal-aware invalidation helper.

The planner should pass runtime-derived booleans for:

- `runtime.dirty.contains(DirtySet::FACILITIES)`
- `runtime.dirty.contains(DirtySet::BLOCKER_CLEANUP)`

and preserve the existing transit guard using `view.in_transit_state(agent).is_some()`.

### 3. Record goal-derived conditions and baselines when goals exhaust

Update `record_exhausted_goals` so newly exhausted entries populate `invalidation_conditions` and `baseline` from `derive_invalidation_conditions(...)` instead of default placeholders.

This function should continue to preserve `count` for already-known exhausted goals and refresh only `exhausted_at` when the same goal exhausts again. The new derived fields should represent the current exhausted world state, not stale historical data.

### 4. Replace stale reset tests with integration-focused exhaustion tests

Remove the old `reset_exhausted_goals_if_needed`-specific test and replace it with focused coverage that proves:

- only matching exhausted entries are invalidated
- unaffected entries retain `count`
- newly exhausted goals record non-empty condition/baseline data for representative goal kinds
- transit-sensitive invalidation stays suppressed mid-travel

## Files to Touch

- `crates/worldwake-ai/src/exhaustion.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)

## Out of Scope

- Save-format or post-load migration policy for legacy empty invalidation entries
- Changes to `EXHAUSTION_SKIP_TTL` or `exhaustion_skip_active`
- Golden scenario additions beyond existing AI-crate regression coverage
- Re-deriving facility-access or blocker-expiry facts inside exhaustion logic

## Acceptance Criteria

### Tests That Must Pass

1. Focused test: invalidation removes only the exhausted entry whose recorded condition fired
2. Focused test: `record_exhausted_goals` records goal-derived invalidation data for exhausted goals
3. Focused test: `PositionChanged` invalidation remains suppressed while the agent is still in transit
4. Existing focused suite: `cargo test -p worldwake-ai agent_tick`
5. Existing focused suite: `cargo test -p worldwake-ai exhaustion`
6. Existing crate suite: `cargo test -p worldwake-ai`

### Invariants

1. `reset_exhausted_goals_if_needed` no longer exists in the codebase
2. Freshly recorded exhausted goals carry lawful invalidation conditions and baseline state
3. Exhaustion invalidation is per goal, not per coarse dirty-mask domain
4. Facility and blocker invalidation remain driven by observation/runtime facts, not duplicated helper-side policy

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/exhaustion.rs` — add cache-level invalidation tests to prove mixed entries are removed selectively and transit-sensitive conditions stay stable
2. `crates/worldwake-ai/src/agent_tick/planning.rs` — replace the stale coarse-reset test with planner integration tests for condition recording and per-goal invalidation retention

### Commands

1. `cargo test -p worldwake-ai exhaustion`
2. `cargo test -p worldwake-ai agent_tick`
3. `cargo test -p worldwake-ai`

## Outcome

- Completion date: 2026-03-27
- What actually changed: `plan_and_validate_next_step` now invalidates exhausted-goal cache entries through goal-aware conditions from `exhaustion.rs`; `record_exhausted_goals` now records derived invalidation conditions and baseline snapshots instead of empty placeholders; the old coarse `reset_exhausted_goals_if_needed` path was removed.
- Deviations from original plan: the ticket was corrected before implementation because the invalidation model and helper coverage already existed. The delivered work focused on planner/runtime integration instead of rebuilding the data model. Legacy empty-condition entries remain conservatively invalidated on the next dirty reevaluation rather than introducing a separate migration path here.
- Verification results: `cargo test -p worldwake-ai exhaustion`, `cargo test -p worldwake-ai agent_tick::planning::tests`, `cargo test -p worldwake-ai`, `cargo clippy --workspace`, and `cargo test --workspace` all passed.
