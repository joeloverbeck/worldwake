# S96OBLSAT-005: Satiation-dampened ranking

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — modifies goal ranking logic (RankingContext, post_notice_motive, post_bounty_motive)
**Deps**: archive/tickets/S96OBLSAT-001.md, archive/tickets/S96OBLSAT-002.md

## Problem

Without satiation dampening in goal ranking, obligation-class goals (PostNotice, PostBounty) maintain maximum priority scores indefinitely, preventing survival needs from competing. This is the core fix for the Guard Theron pathology (487 PostNotice executions while starving).

## Assumption Reassessment (2026-04-12)

1. `RankingContext` at `crates/worldwake-ai/src/ranking.rs:341-355` is a private struct with 13 fields. `RankingContext::new` at line 357-381 populates fields from `GoalBeliefView` (e.g., `view.homeostatic_needs(agent)` at line 371, `view.exploration_profile(agent)` at line 373).
2. `post_notice_motive` at line 954 returns `score_product(context.utility.notice_posting_weight, threat_signal)` at line 978. No existing satiation logic.
3. `post_bounty_motive` at line 899 returns `score_product(context.utility.bounty_posting_weight, reward_signal)` at line 948-951. No existing satiation logic.
4. `score_product` at line 1213: `u32::from(weight.value()) * u32::from(pressure.value())`. Returns u32.
5. Existing focused test in `ranking.rs` `#[cfg(test)]` block at line 1685: `test_same_feasibility_falls_through_to_motive` at line 6106.
6. Shared boundary: `RankingContext` is private to `ranking.rs` — changes are contained within a single module.

## Architecture Check

1. The `apply_obligation_satiation` helper is a pure function over `RankingContext` fields — no side effects, no cross-system calls. Satiation multiplier is derived (never stored), consistent with FND-3 and FND-27.
2. Pruning stale tracker entries during `RankingContext::new` keeps the Vec bounded without a separate SystemFn, following the existing pattern where ranking context construction does lightweight preprocessing.
3. No backwards-compatibility shims. Direct modification of existing motive functions.

## Verification Layers

1. Satiation decay below threshold → no effect → focused unit test
2. Satiation decay above threshold → multiplier applied → focused unit test
3. Floor prevents zero score → focused unit test
4. `post_notice_motive` and `post_bounty_motive` both apply satiation → focused unit tests
5. Single-module ticket; cross-system integration verified in golden test (ticket 006).

## What to Change

### 1. Extend `RankingContext`

Add two fields:
```rust
satiation_profile: ObligationSatiationProfile,
obligation_tracker: ObligationExecutionTracker,
```

Populate in `RankingContext::new`:
```rust
satiation_profile: view.obligation_satiation_profile(agent),
obligation_tracker: {
    let mut tracker = view.obligation_execution_tracker(agent);
    let window = u64::from(view.obligation_satiation_profile(agent).window_ticks);
    tracker.completion_ticks.retain(|t| t.0 >= current_tick.0.saturating_sub(window));
    tracker
},
```

### 2. Add `apply_obligation_satiation` helper

Per spec D3 code example. Pure function taking `&RankingContext` and `raw_score: u32`, returning dampened `u32`. Key logic:
- Count recent executions within window
- If at or below threshold, return raw score unchanged
- Compute decay multiplier: `max(floor, 1000 - over_threshold * decay_per_execution)`
- Return `raw_score * multiplier / 1000`

### 3. Apply satiation in `post_notice_motive`

Replace the final `score_product(...)` return with:
```rust
let raw_score = score_product(context.utility.notice_posting_weight, threat_signal);
apply_obligation_satiation(context, raw_score)
```

### 4. Apply satiation in `post_bounty_motive`

Same pattern — wrap the final `score_product(...)` return with `apply_obligation_satiation`.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify)

## Out of Scope

- Changing how `threat_warning_signal_for_place` computes threat intensity
- Modifying PostNotice/PostBounty action mechanics, duration, or preconditions
- Perception throttling or artifact TTL

## Acceptance Criteria

### Tests That Must Pass

1. `apply_obligation_satiation` with 0 recent executions returns raw score unchanged
2. `apply_obligation_satiation` with executions at threshold returns raw score unchanged
3. `apply_obligation_satiation` with executions above threshold returns decayed score
4. Decay respects `satiation_floor` — score never drops below floor percentage
5. Default profile: 7 executions in window reduces 808200 score to ~40410
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Satiation multiplier is derived from stored state, never stored itself (FND-3, FND-27)
2. Agents with zero `notice_posting_weight` are unaffected (motive returns 0 before satiation check)
3. Tracker entries older than `window_ticks` are pruned during context construction

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` (inline `#[cfg(test)]`) — unit tests for `apply_obligation_satiation` covering: no decay, at threshold, above threshold, floor enforcement, default profile arithmetic

### Commands

1. `cargo test -p worldwake-ai -- apply_obligation_satiation`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
