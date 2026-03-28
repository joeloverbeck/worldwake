# S33OPPSCOGOAIDE-003: Two-pass candidate generation with per-opportunity blocker filtering

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — candidate generation pipeline restructured
**Deps**: S33OPPSCOGOAIDE-002

## Problem

Currently `emit_candidate()` checks `is_blocked(&key, None, None, None, current_tick)` during emission — a global blocker check that suppresses ALL opportunities for a `GoalKey` if any blocker matches. After S33OPPSCOGOAIDE-002, candidates are emitted per-opportunity, but blocker filtering must also become per-opportunity. The spec requires a two-pass approach: Pass 1 emits all candidates without blocker checks, Pass 2 filters per-opportunity using anchor-scoped blocker queries.

## Assumption Reassessment (2026-03-28)

1. After S33OPPSCOGOAIDE-002, `emit_candidate()` no longer contains the `is_blocked(&key, None, None, None, current_tick)` call (it was removed as part of the emission rewrite). This ticket adds the per-opportunity filter as a separate pass.
2. `BlockedIntentMemory::is_blocked()` at `crates/worldwake-core/src/blocked_intent.rs:26` accepts `(goal_key, Option<place>, Option<target>, Option<action_def>, tick)`. The place and target parameters already support per-opportunity scoping.
3. `BlockerKey` at `blocked_intent.rs:8-13` has fields `{ goal_key, place, target, action_def }` — place-scoped blockers already exist in the data model.
4. The generation pipeline flows: `generate_candidates_for_agent()` → collection → ranking. The two-pass filter slots between generation and ranking.
5. This is a candidate-generation-layer ticket. The shared boundary is the `BlockedIntentMemory::is_blocked()` API and the `OpportunityAnchor` → blocker query parameter mapping.
6. No adjacent contradictions.

## Architecture Check

1. Two-pass is cleaner than inline filtering because: (a) the full opportunity set is known before any blocking decisions, enabling desire-level escalation diagnostics; (b) it separates emission logic from filter logic (SRP); (c) it makes testing straightforward — pass 1 output is independently verifiable.
2. No backward-compatibility shims — the old global `is_blocked` call in `emit_candidate` was already removed in S33OPPSCOGOAIDE-002.

## Verification Layers

1. Pass 1 emits all candidates regardless of blockers → focused unit test: blocked opportunity still appears in pass 1 output.
2. Pass 2 filters per-opportunity → focused unit test: blocked orchard opportunity is removed, unblocked market opportunity survives.
3. Desire-level escalation diagnostic → decision trace assertion: when all opportunities for a GoalKey are blocked, trace records it.

## What to Change

### 1. Add per-opportunity blocker filter function

Create a function (e.g., `filter_blocked_opportunities()`) in `candidate_generation.rs` that:
- Takes `Vec<GroundedGoal>`, `&BlockedIntentMemory`, `Tick`
- For each `GroundedGoal`, maps `anchor` to blocker query parameters:
  - `OpportunityAnchor::Place(id)` → `is_blocked(&key, Some(id), None, None, tick)`
  - `OpportunityAnchor::Entity(id)` → `is_blocked(&key, None, Some(id), None, tick)`
  - `OpportunityAnchor::None` → `is_blocked(&key, None, None, None, tick)`
- Returns `(Vec<GroundedGoal>, Vec<FilteredOpportunity>)` where the second element captures filtered-out candidates for tracing.

### 2. Integrate filter into generation pipeline

Call `filter_blocked_opportunities()` after `generate_candidates_for_agent()` returns and before candidates are passed to ranking.

### 3. Remove any residual global blocker checks

Ensure no path in candidate emission still calls `is_blocked` with `(None, None, None)` parameters (global check). All blocker checks go through the two-pass filter.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add `filter_blocked_opportunities()`, remove any residual global blocker checks)
- `crates/worldwake-ai/src/agent_tick/candidates.rs` (modify — integrate two-pass filter into pipeline between generation and ranking)

## Out of Scope

- Decision trace `DesireFullyBlocked` variant (S33OPPSCOGOAIDE-007) — this ticket returns the filtered data but the trace recording is a separate ticket.
- Exhaustion changes (S33OPPSCOGOAIDE-004)
- Post-rank dedup (S33OPPSCOGOAIDE-005)
- Changes to `BlockedIntentMemory` internals or `BlockerKey` struct
- Changes to how blockers are recorded (only how they are queried during candidate generation)

## Acceptance Criteria

### Tests That Must Pass

1. Blocking `Place(orchard)` for `AcquireCommodity(Apple)` does NOT suppress `Place(market)` for the same `GoalKey`.
2. Pass 1 output contains both blocked and unblocked opportunities.
3. Pass 2 output contains only unblocked opportunities.
4. `OpportunityAnchor::None` goals (self-care) are correctly filtered against global blockers.
5. When all opportunities for a `GoalKey` are filtered, the GoalKey has zero surviving candidates.
6. Existing suite: `cargo test -p worldwake-ai`
7. Existing suite: `cargo clippy --workspace`

### Invariants

1. No opportunity is suppressed by a blocker that targets a different anchor (opportunity isolation).
2. Pass 1 is blocker-agnostic — no blocker checks during emission.
3. Pass 2 uses anchor-scoped blocker queries, not global queries.
4. Candidate ordering is deterministic after filtering.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — `test_two_pass_blocker_isolation` — blocked orchard does not suppress market.
2. `crates/worldwake-ai/src/candidate_generation.rs` — `test_pass1_includes_blocked` — pass 1 output includes blocked candidates.
3. `crates/worldwake-ai/src/candidate_generation.rs` — `test_anchor_none_blocker_filtering` — self-care goals filter correctly.
4. `crates/worldwake-ai/src/candidate_generation.rs` — `test_all_opportunities_blocked` — GoalKey with all opportunities blocked yields zero survivors.

### Commands

1. `cargo test -p worldwake-ai -- candidate_generation`
2. `cargo test -p worldwake-ai -- two_pass`
3. `cargo clippy --workspace && cargo test --workspace`
