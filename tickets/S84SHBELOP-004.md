# S84SHBELOP-004: Golden test — ShareBelief succeeds under co-location

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: S84SHBELOP-001

## Problem

No golden E2E test proves that ShareBelief planning succeeds when a co-located listener exists. The existing `golden_social.rs` tests cover tell action payloads and social observations but do not exercise the full planning pipeline for ShareBelief goals — from candidate generation through plan search to tell action commit and listener belief update.

## Assumption Reassessment (2026-04-10)

1. **Golden social test file confirmed**: `crates/worldwake-ai/tests/golden_social.rs` exists, imports `TellProfile`, `TellTopic`, `CommunicationClass`, `TellActionPayload`, `TellCommitResult`. No existing test asserts a full ShareBelief planning → tell execution → listener belief update cycle.
2. **GoalKind::ShareBelief confirmed**: At `goal.rs:95-99` with fields `{ listener, topic, communication_class }`. Three dispatch variants at `goal_dispatch_decl.rs:507-533`.
3. **Shared boundary**: This test exercises the full agent decision pipeline: candidate generation (`social_listeners_at` → `emit_social_candidates`) → planning snapshot construction → search (`PlannerOpKind::Tell`) → action execution → tell commit → listener belief update.
5. **Live GoalKind**: `GoalKind::ShareBelief` with `relevant_ops: [PlannerOpKind::Tell]`. The scenario depends on: (a) candidate generation finding the listener via `social_listeners_at`, (b) snapshot place indexing including the listener (fixed by S84SHBELOP-001), (c) search finding the Tell affordance, (d) tell action committing and updating listener beliefs.
12. **Scenario isolation**: The test needs both agents to have `PerceptionProfile` (required for `effective_place` beliefs) and `TellProfile`. Other goal kinds should be minimized — use a setup where both agents have satisfied needs so ShareBelief is the primary goal.

## Architecture Check

1. A golden E2E test is the appropriate verification surface for the full ShareBelief pipeline — it exercises all layers from candidate generation through action commit without mocking intermediate stages.
2. No backward-compatibility shims. Purely additive test.

## Verification Layers

1. ShareBelief goal generated as candidate -> decision trace
2. Tell plan search succeeds (not frontier-exhausted) -> decision trace `PlanSearchOutcome`
3. Tell action commits -> event-log delta (tell commit event)
4. Listener belief store updated -> authoritative world state (listener's belief store contains the shared belief)
5. Both agents have `PerceptionProfile` -> scenario setup validation

## What to Change

### 1. Add golden test in `golden_social.rs`

Add a test `share_belief_succeeds_with_colocated_listener` (or similar) that:

**Setup**:
- Two agents at the same place
- Both have `PerceptionProfile`, `TellProfile`, `CommunicationProfile`
- Both have satisfied homeostatic needs (to avoid competing need-driven goals)
- Agent A has a belief about a third entity (e.g., entity state at a remote place) that Agent B does not have
- Run enough ticks for perception to establish `effective_place` beliefs

**Assertion** (within bounded tick count):
- Agent A generates a `ShareBelief` candidate targeting Agent B
- Agent A's plan search for the ShareBelief goal succeeds (not `FrontierExhausted`)
- Agent A executes a tell action targeting Agent B
- After tell commits, Agent B's belief store contains the shared belief topic

## Files to Touch

- `crates/worldwake-ai/tests/golden_social.rs` (modify)

## Out of Scope

- Testing ShareBelief when agents are NOT co-located (expected failure case)
- Testing tell action payload structure (already covered by existing golden_social tests)
- Testing social candidate omissions (covered by existing unit tests)
- Testing multiple communication classes — one class (e.g., Testimony) is sufficient

## Acceptance Criteria

### Tests That Must Pass

1. New golden test: ShareBelief plan search succeeds with co-located listener
2. New golden test: tell action commits and updates listener's belief store
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. ShareBelief planning requires co-location — both agents at the same place with mutual `effective_place` beliefs
2. Tell action has 2-tick duration — test must advance enough ticks for commit
3. PerceptionProfile is required for `effective_place` beliefs — agents without it cannot observe co-located entities

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_social.rs` — full ShareBelief pipeline: candidate → plan → tell → listener belief update

### Commands

1. `cargo test -p worldwake-ai golden_social::share_belief_succeeds`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
