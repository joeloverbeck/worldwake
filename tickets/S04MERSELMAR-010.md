# S04MERSELMAR-010: DemandMemory ranking integration and blocked-intent dampening

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — ranking boost logic, blocked-intent usage for sell cycles
**Deps**: S04MERSELMAR-007

## Problem

`SellCommodity` must be rankable relative to other goals. Remembered local demand should boost the sell-goal motive without overpowering critical self-care. Repeated unproductive market-presence cycles must be dampened through the existing `BlockedIntentMemory` system rather than a merchant-specific cooldown table.

## Assumption Reassessment (2026-03-31)

1. `DemandMemory` entries contain `DemandObservation { place, commodity, reason, tick }`. `DemandObservationReason::WantedToSellButNoBuyer` exists at `crates/worldwake-core/src/trade.rs:61`. Confirmed.
2. Goal ranking in `crates/worldwake-ai/src/ranking.rs` uses `RankedGoal` with priority class and motive value. Enterprise goals like `RestockCommodity` already have ranking logic. Confirmed.
3. `BlockedIntentMemory` in `crates/worldwake-core/src/blocked_intent.rs` stores `BlockedIntent { goal, place, expires_at, barrier }`. This is used by the planner to suppress recently-failed goals. Confirmed.
4. `handle_plan_failure` in `crates/worldwake-ai/src/failure_handling.rs` creates `BlockedIntent` entries when plans fail. The `staff_market` commit handler (ticket 003) records `WantedToSellButNoBuyer` on unproductive cycles.
5. `SellCommodity` candidate generation (ticket 007) emits candidates. This ticket adds the ranking signal.
6. The spec (Section 13) says demand memory boosts ranking of `SellCommodity` and `RestockCommodity`, and helps valuation. Ranking boost is this ticket's scope.
7. No adjacent contradictions found.

## Architecture Check

1. Using `DemandMemory` as a ranking signal follows the existing pattern — `RestockCommodity` already uses demand memory for evidence and ranking. `SellCommodity` mirrors this.
2. Using `BlockedIntentMemory` for dampening follows the existing pattern — all goal kinds use the same blocked-intent system for temporary suppression. No merchant-specific cooldown table needed.
3. The ranking boost must not overpower critical self-care (hunger, thirst, safety). Enterprise goals have a lower priority class than survival goals, so the motive boost only competes within the enterprise priority class.
4. No backwards-compatibility shims.

## Verification Layers

1. Demand memory boosts SellCommodity motive -> focused unit test in ranking.rs
2. No demand memory = baseline motive (not zero, not blocked) -> focused unit test
3. Blocked-intent suppresses SellCommodity after unproductive cycle -> focused unit test
4. SellCommodity ranking never exceeds critical self-care -> focused ranking comparison test
5. RestockCommodity ranking also benefits from demand memory -> existing behavior preserved

## What to Change

### 1. Add demand memory ranking boost for `SellCommodity` in `ranking.rs`

When scoring `SellCommodity { commodity }` motive:
- Query `demand_memory` for the agent
- Count recent observations matching the commodity at `home_market` (or any place)
- Use observation count/recency to compute a motive boost
- The boost adds to the enterprise-class motive value, not to the priority class

### 2. Wire `BlockedIntentMemory` for `SellCommodity` failed cycles

The `staff_market` commit handler (ticket 003) records `WantedToSellButNoBuyer`. The failure handling path must create a `BlockedIntent` for `SellCommodity { commodity }` at `home_market` when an unproductive cycle completes.

This can be done either:
- In `commit_staff_market` (ticket 003) directly creating a `BlockedIntent`
- In `handle_plan_failure` when the completed-but-unproductive pattern is detected

The cleaner approach is for `commit_staff_market` to signal an "unproductive completion" that the AI layer interprets. The `WantedToSellButNoBuyer` demand observation is the signal. The agent tick driver or candidate generation can check for recent `WantedToSellButNoBuyer` at `home_market` and suppress re-emission for a blocking period.

### 3. Suppress `SellCommodity` re-emission after unproductive cycle

In candidate generation (from ticket 007), check `BlockedIntentMemory` for `SellCommodity { commodity }` at `home_market`. If blocked, do not emit the candidate. This follows the same suppression pattern used for all other goals.

### 4. Ensure enterprise ranking does not overpower self-care

Verify that `SellCommodity` uses the enterprise priority class (not survival/safety). The motive boost from demand memory should be bounded so it cannot exceed the enterprise class ceiling.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify — add demand memory motive boost for SellCommodity)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — check blocked-intent suppression for SellCommodity)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — if failure handling creates blocked intent for unproductive sell cycles)
- `crates/worldwake-systems/src/trade_actions.rs` (modify — if commit_staff_market directly creates blocked intent)

## Out of Scope

- `SellCommodity` candidate emission conditions (ticket 007 — this ticket adds ranking/dampening on top)
- Valuation changes for commodity opportunity (S06 spec scope)
- Demand memory creation or aging (existing E11 behavior)
- `RestockCommodity` ranking changes (existing behavior, not modified)

## Acceptance Criteria

### Tests That Must Pass

1. Recent demand memory for a commodity at `home_market` boosts `SellCommodity` motive
2. No demand memory results in baseline (non-zero) motive for `SellCommodity`
3. `SellCommodity` is suppressed by `BlockedIntentMemory` after an unproductive `staff_market` cycle
4. Suppressed `SellCommodity` resumes after the blocking period expires
5. `SellCommodity` ranking never exceeds critical self-care goals (hunger at critical, thirst at critical)
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Demand memory is a ranking signal, not a gating condition — merchants can sell without demand memory
2. Dampening uses generic `BlockedIntentMemory`, not a merchant-specific cooldown
3. Enterprise goals never overpower survival-class goals
4. No magic numbers — boost magnitudes come from `UtilityProfile` or similar per-agent parameters

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` — focused test: demand memory boosts SellCommodity motive
2. `crates/worldwake-ai/src/ranking.rs` — focused test: no demand memory = baseline motive
3. `crates/worldwake-ai/src/candidate_generation.rs` — focused test: blocked intent suppresses SellCommodity
4. `crates/worldwake-ai/src/ranking.rs` — focused test: SellCommodity vs critical self-care comparison

### Commands

1. `cargo test -p worldwake-ai -- ranking`
2. `cargo test -p worldwake-ai -- candidate_generation`
3. `cargo clippy --workspace && cargo test --workspace`
