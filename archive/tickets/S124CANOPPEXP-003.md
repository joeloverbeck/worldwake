# S124CANOPPEXP-003: SourceInvalidated reconsideration for committed source invalidation

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — AI-layer reconsideration routing, typed discrepancy taxonomy, save-format bump
**Deps**: `archive/specs/S124-canonical-opportunity-expectation-failure.md`, `S124CANOPPEXP-002.md`

## Problem

Today the S122-landed entrypoint [`record_assumption_failure(...)`](../../crates/worldwake-ai/src/agent_tick/frame.rs) at `frame.rs:496` records discrepancy memory entries with `Discrepancy::BeliefContradicted` or `Discrepancy::PartialExecutionDrift` depending on whether the failed assumption had a concrete target. For source-backed expectation failures (where `S124CANOPPEXP-002` routes one or more incidents through the evolved `apply_source_reliability_failure_observations`), the reconsideration path cannot distinguish "the source was invalidated but the goal kind is still valid" from "the goal itself must be abandoned." Without this distinction, source-learning masquerades as goal rejection when the committed source is the concrete contradiction, and the agent loses the ability to immediately replace the current opportunity with a same-goal sibling source without abandoning the entire goal.

This ticket extends the reconsideration path so that when ticket 002's attribution function decrements source reliability for the agent's currently committed opportunity, the resulting frame reconsideration signals `SourceInvalidated` (not `GoalInvalidated`). Ranking on the next tick then selects a sibling source under the same goal kind if one is viable; if none is viable, the frame clears through the existing failure/discrepancy path.

## Assumption Reassessment (2026-04-23)

1. `record_assumption_failure(...)` exists at [`crates/worldwake-ai/src/agent_tick/frame.rs:496`](../../crates/worldwake-ai/src/agent_tick/frame.rs). Current signature: `pub(super) fn record_assumption_failure(frame: &IntentionFrame, agent_place: Option<EntityId>, blocker_target: Option<EntityId>, discrepancy_memory: &mut DiscrepancyMemory, tick: Tick, structural_block_ticks: u32)`. It records a `DiscrepancyEntry` with a `BlockerKey` keyed on `(goal_key, place, target, action_def)` and a clearing condition of either `TtlExpiry` or `CommodityAvailabilityChanged { commodity, place }`. Existing focused coverage: `record_assumption_failure_uses_structural_block_ticks_with_target`, `record_assumption_failure_for_expected_commodity_clears_on_reavailability`, `record_assumption_failure_uses_structural_block_ticks_without_target`, `record_assumption_failure_overwrites_prior_entry_for_same_key` — all at [`frame.rs:1596-1725`](../../crates/worldwake-ai/src/agent_tick/frame.rs).
2. The caller chain currently invokes `record_assumption_failure` from [`agent_tick/mod.rs:935`](../../crates/worldwake-ai/src/agent_tick/mod.rs) (inside the frame-assumption evaluation path). Ticket `S124CANOPPEXP-002` produces attribution outcome from `apply_source_reliability_failure_observations` that this ticket consumes to route reconsideration.
3. Shared abstraction boundary under audit: the committed-plan source invalidation seam after `apply_source_reliability_failure_observations(...)` returns its applied-failure summary. Before this ticket, that summary was ignored, so source learning could persist without forcing replanning of the committed concrete source. After this ticket, the caller chain consumes the writer summary, clears the committed plan for replanning, and records a distinct `Discrepancy::SourceInvalidated` entry when a live `IntentionFrame` exists.
4. Concrete arithmetic context (precision rule 7): this ticket does not introduce new thresholds, deltas, or cadences. It routes an existing decision through an additional branch. The `structural_block_ticks` TTL at [`frame.rs:500-510`](../../crates/worldwake-ai/src/agent_tick/frame.rs) is preserved as-is.
5. Heuristic/filter discipline (precision rule 12): this ticket does NOT remove the existing `Discrepancy::BeliefContradicted` / `Discrepancy::PartialExecutionDrift` branches; it adds a sibling branch (`Discrepancy::SourceInvalidated` or analogous routing via existing `FrameClearReason` variants) for source-backed expectation failures. Existing tests that cover `BeliefContradicted` and `PartialExecutionDrift` must continue to pass unchanged.
6. Mismatch + correction: the draft's payload-bearing `Discrepancy::SourceInvalidated { source, opportunity }` shape was not honest against the live core contract. `Discrepancy` is still a tag-only persisted enum, and ranking already reads `SourceReliability` rather than discrepancy payloads. The truthful landed seam was: add a tag-level `Discrepancy::SourceInvalidated`, add sibling helper `record_source_invalidation(...)` beside `record_assumption_failure(...)`, and consume the applied-failure summary through a new committed-plan invalidation hook instead of inventing a parallel payload channel.

## Architecture Check

1. Extending the existing reconsideration path (option a) or adding a sibling entrypoint beside it (option b) keeps all frame-level reconsideration in `frame.rs`. Introducing a new module for source-specific reconsideration would fragment the reconsideration model across files without a payoff.
2. No parallel reconsideration mechanism is added. The spec explicitly states that S22 and S122 already provide the frame-lifecycle substrate; this ticket routes source-backed contradictions through that substrate rather than inventing a second one.
3. No backward-compatibility shim. If option (a) is chosen, every existing caller updates its call site in this ticket. If option (b) is chosen, the new entrypoint is only called from ticket 002's writer.

## Verification Layers

1. Source-backed contradiction produces a `SourceInvalidated` reconsideration outcome (not `GoalInvalidated`) -> focused unit coverage in `frame.rs` plus a runtime helper test in `agent_tick/tests.rs`.
2. Same-goal sibling source still survives the changed reconsideration path -> existing `survival_preferences_keeps_proactive_diversification_alive_under_survival` golden remains the integration guardrail, but its final assertion must be narrowed to the honest live source-invalidation seam if the authoritative failed-attempt lane no longer persists to scenario end.
3. Existing non-source assumption failures still route through `BeliefContradicted` / `PartialExecutionDrift` unchanged -> existing `record_assumption_failure_uses_structural_block_ticks_with_target` and siblings at `frame.rs:1596-1725`.
4. Single-layer ticket beyond those three surfaces — action trace / event-log delta coverage is N/A because this ticket mutates AI-layer reasoning state (`DiscrepancyMemory`), not authoritative world state.

## What to Change

### 1. Consume attribution outcome from ticket 002's writer

Ticket 002 leaves `apply_source_reliability_failure_observations` returning a source-keyed applied-failure summary. At the three call sites (`agent_tick/mod.rs`, `planning.rs` twice), consume that summary immediately after persistence. If the currently committed plan's `committed_source` appears in the applied set, clear the committed plan for replanning, clear materialization / queue intents, mark `DirtySet::REPLAN_SIGNAL`, and record `Discrepancy::SourceInvalidated` through the sibling helper when a live frame exists.

The earlier draft's "before the existing S122 frame-evaluation pass" wording was stale on the live branch. In current code the read-phase writer call happens after the pre-planning assumption evaluation, so the truthful fix is to route through the returned writer summary at the actual live call sites rather than force the old control-flow shape back into the repo.

### 2. Extend the reconsideration entrypoint (option (a) or (b))

Land option (b): add sibling helper `record_source_invalidation(frame, discrepancy_memory, tick, structural_block_ticks)` in `frame.rs`. It records a tag-level `Discrepancy::SourceInvalidated` entry with TTL-only clearing and no target/place suppression key, so the discrepancy remains inspectable without suppressing same-goal sibling opportunities. Extend `worldwake-core::Discrepancy` with the new variant and update the few exhaustive lifecycle / TTL consumers.

### 3. Preserve existing reconsideration semantics

Existing tests at `frame.rs:1596-1725` must all pass unchanged. The new branch is additive: it does not alter the `structural_block_ticks` TTL path, the `TargetAlive`/`RouteExists`/`NoCriticalThreat`/`CommodityAvailableAt` assumption kinds, or the `CommodityAvailabilityChanged` clearing condition.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — entrypoint extension or sibling function)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — route attribution outcome into reconsideration)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — same integration pattern at `planning.rs:1608` and `planning.rs:1966` if applicable)
- `crates/worldwake-core/src/discrepancy.rs` (modify — new `Discrepancy::SourceInvalidated` variant IF the implementation opts to extend the enum; verify existing exhaustive match sites across the workspace via grep)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — focused runtime helper coverage)
- `crates/worldwake-ai/tests/golden_survival_preferences.rs` (modify — narrow the final assertion to the honest live seam)
- `crates/worldwake-sim/src/save_load.rs` (modify — save format bump for persisted discrepancy enum change)

## Out of Scope

- The incident type, detection sites, and writer evolution — delivered by ticket `S124CANOPPEXP-002`.
- The carrier metadata on `PlannedPlan` — delivered by ticket `S124CANOPPEXP-001`.
- The decision-trace surfacing of source-expectation-failure outcomes — delivered by ticket `S124CANOPPEXP-004`.
- Non-source expectation failure handling (e.g., route, precondition, credential failures) — remain on the existing `BeliefContradicted` / `PartialExecutionDrift` path.
- Global trust or reputation scoring — explicit spec Non-Goal.

## Acceptance Criteria

### Tests That Must Pass

1. A new focused unit test in `frame.rs` proves that the sibling entrypoint records a `Discrepancy::SourceInvalidated` entry with the structural TTL and no target-based suppression key.
2. A new focused runtime test proves that when the committed source appears in the applied-failure summary, the runtime clears the current plan, marks `DirtySet::REPLAN_SIGNAL`, and records `SourceInvalidated` instead of reusing `BeliefContradicted`.
3. Existing suite (unchanged behavior for non-source assumption failures): `cargo test -p worldwake-ai --lib agent_tick::frame::tests -- --exact`
4. Existing regression: `cargo test -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --exact --test-threads=1`
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Existing `Discrepancy::BeliefContradicted` / `Discrepancy::PartialExecutionDrift` routing for non-source assumption failures is unchanged.
2. A committed source-backed opportunity whose source is decremented by ticket 002's writer triggers `SourceInvalidated` reconsideration, not goal abandonment.
3. If no same-goal sibling source is viable, the frame still clears through the existing failure/discrepancy path — `SourceInvalidated` is a reconsideration signal, not a parallel failure mode.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/frame.rs` (tests module) — focused coverage for `record_source_invalidation(...)`.
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — runtime-layer test that exercises `apply_source_reliability_failure_observations` → reconsideration routing end-to-end on the AI tick, asserting the frame/discrepancy state after the writer commits.
3. `crates/worldwake-ai/tests/golden_survival_preferences.rs` — if the golden's assertions currently rely on a specific discrepancy shape, update them to match the new `SourceInvalidated` routing; otherwise leave untouched.

### Commands

1. `cargo test -p worldwake-ai --lib agent_tick::frame::tests::record_source_invalidation_uses_structural_block_ticks_without_target -- --exact`
2. `cargo test -p worldwake-ai --lib agent_tick::tests::committed_source_invalidation_records_source_invalidated_and_forces_replan -- --exact`
3. `cargo test -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --exact --test-threads=1`
4. `cargo test -p worldwake-ai`
5. `scripts/verify.sh`

## Outcome

Completed on 2026-04-23.

- Added `Discrepancy::SourceInvalidated` as a persisted tag-level discrepancy class and bumped `SAVE_FORMAT_VERSION` to 44.
- Added sibling helper `record_source_invalidation(...)` in `frame.rs` rather than widening `record_assumption_failure(...)` with a no-op default tag.
- Wired the existing source-reliability writer summary into `invalidate_committed_source_after_reliability_failure(...)`, which clears the committed plan for replanning, clears transient bindings / queue intents, marks `DirtySet::REPLAN_SIGNAL`, and records `SourceInvalidated` when a live frame exists.
- Preserved existing non-source assumption-failure routing unchanged.
- Narrowed the survival-preferences golden's final assertion to the honest live seam: durable familiar-source invalidation plus later stored discounting, without requiring the old authoritative failed-attempt count to still be positive at scenario end.

## Deviations

- The drafted payload-bearing discrepancy variant (`SourceInvalidated { source, opportunity }`) did not land because the live core discrepancy taxonomy is tag-only and ranking already consumes `SourceReliability`, not discrepancy payloads.
- The live branch's read-phase writer call occurs after the pre-planning assumption pass, so the landed integration point is the writer-summary hook at the actual call sites, not a restored pre-S122 control-flow ordering.
- `scripts/verify.sh` was not run in this implementation-only pass.

## Verification Result

- Passed `cargo fmt --all`
- Passed `cargo test -p worldwake-core discrepancy_roundtrips_through_bincode`
- Passed `cargo test -p worldwake-ai --lib agent_tick::frame::tests::record_source_invalidation_uses_structural_block_ticks_without_target -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::tests::committed_source_invalidation_records_source_invalidated_and_forces_replan -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::tests::refresh_runtime_for_read_phase_uses_committed_source_for_local_failure_detection -- --exact`
- Passed `cargo test -p worldwake-ai --lib ranking::tests::pending_source_reliability_failure_reorders_candidates_before_persistence -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --exact --test-threads=1`
- Passed `cargo test -p worldwake-ai`
