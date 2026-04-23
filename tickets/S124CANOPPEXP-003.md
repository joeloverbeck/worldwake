# S124CANOPPEXP-003: SourceInvalidated outcome in record_assumption_failure reconsideration path

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — AI-layer intention-frame reconsideration routing
**Deps**: `specs/S124-canonical-opportunity-expectation-failure.md`, `archive/tickets/S124CANOPPEXP-002.md`

## Problem

Today the S122-landed entrypoint [`record_assumption_failure(...)`](../crates/worldwake-ai/src/agent_tick/frame.rs) at `frame.rs:496` records discrepancy memory entries with `Discrepancy::BeliefContradicted` or `Discrepancy::PartialExecutionDrift` depending on whether the failed assumption had a concrete target. For source-backed expectation failures (where `S124CANOPPEXP-002` routes one or more incidents through the evolved `apply_source_reliability_failure_observations`), the reconsideration path cannot distinguish "the source was invalidated but the goal kind is still valid" from "the goal itself must be abandoned." Without this distinction, source-learning masquerades as goal rejection when the committed source is the concrete contradiction, and the agent loses the ability to immediately replace the current opportunity with a same-goal sibling source without abandoning the entire goal.

This ticket extends the reconsideration path so that when ticket 002's attribution function decrements source reliability for the agent's currently committed opportunity, the resulting frame reconsideration signals `SourceInvalidated` (not `GoalInvalidated`). Ranking on the next tick then selects a sibling source under the same goal kind if one is viable; if none is viable, the frame clears through the existing failure/discrepancy path.

## Assumption Reassessment (2026-04-23)

1. `record_assumption_failure(...)` exists at [`crates/worldwake-ai/src/agent_tick/frame.rs:496`](../crates/worldwake-ai/src/agent_tick/frame.rs). Current signature: `pub(super) fn record_assumption_failure(frame: &IntentionFrame, agent_place: Option<EntityId>, blocker_target: Option<EntityId>, discrepancy_memory: &mut DiscrepancyMemory, tick: Tick, structural_block_ticks: u32)`. It records a `DiscrepancyEntry` with a `BlockerKey` keyed on `(goal_key, place, target, action_def)` and a clearing condition of either `TtlExpiry` or `CommodityAvailabilityChanged { commodity, place }`. Existing focused coverage: `record_assumption_failure_uses_structural_block_ticks_with_target`, `record_assumption_failure_for_expected_commodity_clears_on_reavailability`, `record_assumption_failure_uses_structural_block_ticks_without_target`, `record_assumption_failure_overwrites_prior_entry_for_same_key` — all at [`frame.rs:1596-1725`](../crates/worldwake-ai/src/agent_tick/frame.rs).
2. The caller chain currently invokes `record_assumption_failure` from [`agent_tick/mod.rs:935`](../crates/worldwake-ai/src/agent_tick/mod.rs) (inside the frame-assumption evaluation path). Ticket `S124CANOPPEXP-002` produces attribution outcome from `apply_source_reliability_failure_observations` that this ticket consumes to route reconsideration.
3. Shared abstraction boundary under audit: the reconsideration decision for source-backed expectation failures. Before this ticket, `record_assumption_failure` cannot distinguish source invalidation from goal invalidation. After this ticket, the entrypoint (or a thin wrapper beside it) accepts an outcome tag or equivalent signal so the frame clear reason / discrepancy shape reflects the distinction.
4. Concrete arithmetic context (precision rule 7): this ticket does not introduce new thresholds, deltas, or cadences. It routes an existing decision through an additional branch. The `structural_block_ticks` TTL at [`frame.rs:500-510`](../crates/worldwake-ai/src/agent_tick/frame.rs) is preserved as-is.
5. Heuristic/filter discipline (precision rule 12): this ticket does NOT remove the existing `Discrepancy::BeliefContradicted` / `Discrepancy::PartialExecutionDrift` branches; it adds a sibling branch (`Discrepancy::SourceInvalidated` or analogous routing via existing `FrameClearReason` variants) for source-backed expectation failures. Existing tests that cover `BeliefContradicted` and `PartialExecutionDrift` must continue to pass unchanged.
6. Mismatch + correction: the spec's D5 text mentions "extend `record_assumption_failure` or its caller chain." The implementation may choose either (a) widen `record_assumption_failure` with an optional outcome tag, or (b) add a sibling entrypoint `record_source_invalidation(...)` called by ticket 002's writer directly. Option (b) is preferred if option (a) would require every existing caller to pass a default tag — that smells like a no-op shim and violates FND-28. Final choice must be made at implementation time based on caller-site ergonomics; if (b) is chosen, the sibling entrypoint must live in `frame.rs` alongside `record_assumption_failure` so both reconsideration paths share the module.

## Architecture Check

1. Extending the existing reconsideration path (option a) or adding a sibling entrypoint beside it (option b) keeps all frame-level reconsideration in `frame.rs`. Introducing a new module for source-specific reconsideration would fragment the reconsideration model across files without a payoff.
2. No parallel reconsideration mechanism is added. The spec explicitly states that S22 and S122 already provide the frame-lifecycle substrate; this ticket routes source-backed contradictions through that substrate rather than inventing a second one.
3. No backward-compatibility shim. If option (a) is chosen, every existing caller updates its call site in this ticket. If option (b) is chosen, the new entrypoint is only called from ticket 002's writer.

## Verification Layers

1. Source-backed contradiction produces a `SourceInvalidated` reconsideration outcome (not `GoalInvalidated`) -> focused unit coverage in `frame.rs` tests module asserting the distinct discrepancy shape or frame-clear reason for source-backed inputs.
2. Same-goal sibling source is selected on the next tick after `SourceInvalidated` (integration-level proof) -> existing `survival_preferences_keeps_proactive_diversification_alive_under_survival` golden should continue to exhibit this behavior; if the golden's current pass trajectory depended on `BeliefContradicted`-style clearing, adjust the golden assertions to match the new reconsideration shape.
3. Existing non-source assumption failures still route through `BeliefContradicted` / `PartialExecutionDrift` unchanged -> existing `record_assumption_failure_uses_structural_block_ticks_with_target` and siblings at `frame.rs:1596-1725`.
4. Single-layer ticket beyond those three surfaces — action trace / event-log delta coverage is N/A because this ticket mutates AI-layer reasoning state (`DiscrepancyMemory`), not authoritative world state.

## What to Change

### 1. Consume attribution outcome from ticket 002's writer

Ticket 002 leaves `apply_source_reliability_failure_observations` returning a summary of source-invalidation decisions (e.g., `Vec<(SourceKey, ExpectationFailureCause)>`). At the three call sites (`agent_tick/mod.rs:1045`, `planning.rs:1608`, `planning.rs:1966`), after the writer returns, check whether the agent's currently committed opportunity had its source decremented. If yes, route through the new reconsideration path in Change 2.

Grep for how `mod.rs:935` currently calls `record_assumption_failure` to identify the correct integration point — the source-invalidation routing lives at the same call depth, after reliability persistence and before the existing S122 frame-evaluation pass.

### 2. Extend the reconsideration entrypoint (option (a) or (b))

**Option (a)**: Add a parameter to `record_assumption_failure` — e.g., `outcome: AssumptionFailureOutcome { Generic, SourceInvalidated { source: SourceKey, opportunity: OpportunityKey } }` — and branch inside the function so `SourceInvalidated` routes to a new `Discrepancy::SourceInvalidated { source, opportunity }` variant (or a new `DiscrepancyClearing` variant tied to source-reliability recovery). Update all existing callers to pass `Generic`.

**Option (b)**: Define a new sibling entrypoint `record_source_invalidation(frame: &IntentionFrame, source: SourceKey, opportunity: OpportunityKey, discrepancy_memory: &mut DiscrepancyMemory, tick: Tick)` that records the source-specific discrepancy shape directly. Only the new caller from ticket 002's writer invokes this; existing `record_assumption_failure` callers are untouched.

Pick option (a) if existing callers naturally supply outcome context; pick option (b) if adding a `Generic` parameter everywhere would read as a no-op shim.

Either way, the discrepancy shape for source-backed invalidation must carry enough information for ranking to identify the invalidated source on the next tick — at minimum `SourceKey` and `OpportunityKey`. If the existing `Discrepancy` enum in core needs extension, add a `SourceInvalidated { source: SourceKey, opportunity: OpportunityKey }` variant with matching updates in `BlockerMemory` consumers; verify via grep that no exhaustive match is missed.

### 3. Preserve existing reconsideration semantics

Existing tests at `frame.rs:1596-1725` must all pass unchanged. The new branch is additive: it does not alter the `structural_block_ticks` TTL path, the `TargetAlive`/`RouteExists`/`NoCriticalThreat`/`CommodityAvailableAt` assumption kinds, or the `CommodityAvailabilityChanged` clearing condition.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — entrypoint extension or sibling function)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — route attribution outcome into reconsideration)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — same integration pattern at `planning.rs:1608` and `planning.rs:1966` if applicable)
- `crates/worldwake-core/src/discrepancy.rs` (modify — new `Discrepancy::SourceInvalidated` variant IF the implementation opts to extend the enum; verify existing exhaustive match sites across the workspace via grep)
- `crates/worldwake-core/src/belief.rs` or wherever `BlockerKey`/`DiscrepancyClearing` live (modify — only if the clearing condition needs a new variant)

If the implementation routes through existing `Discrepancy` variants without extending the enum, the two core file touches are not required. Decide at implementation time.

## Out of Scope

- The incident type, detection sites, and writer evolution — delivered by ticket `S124CANOPPEXP-002`.
- The carrier metadata on `PlannedPlan` — delivered by ticket `S124CANOPPEXP-001`.
- The decision-trace surfacing of source-expectation-failure outcomes — delivered by ticket `S124CANOPPEXP-004`.
- Non-source expectation failure handling (e.g., route, precondition, credential failures) — remain on the existing `BeliefContradicted` / `PartialExecutionDrift` path.
- Global trust or reputation scoring — explicit spec Non-Goal.

## Acceptance Criteria

### Tests That Must Pass

1. A new focused unit test in `frame.rs` (tests module) proves that a source-backed contradiction recorded through the new entrypoint produces a `SourceInvalidated`-flavored discrepancy entry (or the equivalent chosen-option routing) with the concrete `SourceKey` and `OpportunityKey` accessible to downstream ranking readers.
2. A new focused unit test proves that the reconsideration outcome is `SourceInvalidated` (not `GoalInvalidated`) when only the committed source is contradicted; the goal kind remains valid for next-tick ranking.
3. Existing suite (unchanged behavior for non-source assumption failures): `cargo test -p worldwake-ai --lib agent_tick::frame::tests -- --exact`
4. Existing regression: `cargo test -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --exact --test-threads=1`
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Existing `Discrepancy::BeliefContradicted` / `Discrepancy::PartialExecutionDrift` routing for non-source assumption failures is unchanged.
2. A committed source-backed opportunity whose source is decremented by ticket 002's writer triggers `SourceInvalidated` reconsideration, not goal abandonment.
3. If no same-goal sibling source is viable, the frame still clears through the existing failure/discrepancy path — `SourceInvalidated` is a reconsideration signal, not a parallel failure mode.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/frame.rs` (tests module) — focused coverage for the new reconsideration entrypoint or branch.
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — runtime-layer test that exercises `apply_source_reliability_failure_observations` → reconsideration routing end-to-end on the AI tick, asserting the frame/discrepancy state after the writer commits.
3. `crates/worldwake-ai/tests/golden_survival_preferences.rs` — if the golden's assertions currently rely on a specific discrepancy shape, update them to match the new `SourceInvalidated` routing; otherwise leave untouched.

### Commands

1. `cargo test -p worldwake-ai --lib agent_tick::frame::tests -- --exact`
2. `cargo test -p worldwake-ai --lib agent_tick::tests -- --exact`
3. `cargo test -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --exact --test-threads=1`
4. `cargo test -p worldwake-ai`
5. `scripts/verify.sh`
