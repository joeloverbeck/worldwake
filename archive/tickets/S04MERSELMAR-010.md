# S04MERSELMAR-010: DemandMemory ranking integration and blocked-intent dampening

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — ranking boost logic, blocked-intent usage for sell cycles
**Deps**: S04MERSELMAR-007

## Problem

`SellCommodity` must be rankable relative to other goals. Remembered local demand should boost the sell-goal motive without overpowering critical self-care. Repeated unproductive market-presence cycles must be dampened through the existing `BlockedIntentMemory` system rather than a merchant-specific cooldown table.

## Assumption Reassessment (2026-04-01)

1. `DemandMemory` entries contain `DemandObservation { place, commodity, reason, tick }`. `DemandObservationReason::WantedToSellButNoBuyer` exists at `crates/worldwake-core/src/trade.rs:74`. Confirmed.
2. ~~Goal ranking already wired~~: `SellCommodity` uses `enterprise_score` → `opportunity_signal` → `market_signal_for_place` → `relevant_demand_quantity`, which queries demand memory. Ranking boost is already implemented.
3. `BlockedIntentMemory` in `crates/worldwake-core/src/blocked_intent.rs` stores `BlockedIntent`. Generic `filter_blocked_candidates` (candidate_generation.rs:260) already suppresses any goal kind with a matching blocker — no SellCommodity-specific code needed.
4. `handle_plan_failure` creates `BlockedIntent` entries when plans fail, but `commit_staff_market` commits successfully — `handle_plan_failure` never fires for unproductive cycles. **This is the actual gap**: no `BlockedIntent` is created after a successful-but-unproductive market presence.
5. `SellCommodity` candidate generation (ticket 007) emits candidates. Already on main.
6. Enterprise class enforcement already correct: `SellCommodity` returns `GoalPriorityClass::Medium` (ranking.rs:343), survival goals use `High`/`Critical` which always wins.
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

### 1. ~~Add demand memory ranking boost for `SellCommodity`~~ (ALREADY DONE)

`SellCommodity` already uses `enterprise_score` → `opportunity_signal` → `relevant_demand_quantity` which queries demand memory. No work needed.

### 2. Wire `BlockedIntentMemory` for `SellCommodity` failed cycles

The `staff_market` commit handler (ticket 003) records `WantedToSellButNoBuyer`. The failure handling path must create a `BlockedIntent` for `SellCommodity { commodity }` at `home_market` when an unproductive cycle completes.

This can be done either:
- In `commit_staff_market` (ticket 003) directly creating a `BlockedIntent`
- In `handle_plan_failure` when the completed-but-unproductive pattern is detected

The cleaner approach is for `commit_staff_market` to signal an "unproductive completion" that the AI layer interprets. The `WantedToSellButNoBuyer` demand observation is the signal. The agent tick driver or candidate generation can check for recent `WantedToSellButNoBuyer` at `home_market` and suppress re-emission for a blocking period.

### 3. ~~Suppress `SellCommodity` re-emission after unproductive cycle~~ (ALREADY DONE)

Generic `filter_blocked_candidates` (candidate_generation.rs:260) already suppresses any goal kind with a matching blocker. No SellCommodity-specific code needed.

### 4. ~~Ensure enterprise ranking does not overpower self-care~~ (ALREADY DONE)

`SellCommodity` returns `GoalPriorityClass::Medium` (ranking.rs:343). Survival goals use `High`/`Critical` which always wins. No work needed.

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

## Outcome

- **Completion date**: 2026-04-01
- **What changed**:
  - Added `BlockingFact::NoBuyer` variant for seller-side unproductive cycle dampening
  - `commit_staff_market` now creates a `BlockedIntent` for `SellCommodity { commodity }` when no trades occurred, using `market_presence_ticks` from `TradeDispositionProfile` as the blocking period
  - `blocker_resolved` in `failure_handling.rs` handles `NoBuyer` (TTL-only expiry, no early resolution)
  - `blocking_fact_ttl` in `failure_handling.rs` maps `NoBuyer` to `structural_block_ticks`
  - 1 new focused test: `staff_market_unproductive_commit_creates_blocked_intent_for_sell_commodity`
- **Deviations from original plan**:
  - Deliverables 1, 3, 4 were already implemented by prior work (ranking via `enterprise_score`/`opportunity_signal`, generic `filter_blocked_candidates`, enterprise priority class). Only deliverable 2 (blocked intent creation) was needed.
- **Verification**: `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo test --workspace` all tests pass
