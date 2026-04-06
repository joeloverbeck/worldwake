# TK-3: Low-Band Self-Care Goals Clear at Medium Threshold

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` self-care goal satisfaction boundaries
**Deps**: archive/tickets/TK-2-consume-owned-terminal.md

## Problem

`GoalKind::Sleep` and `GoalKind::Relieve` are emitted once their drives cross the corresponding `DriveThresholds::* .low()` bands in `crates/worldwake-ai/src/candidate_generation.rs`, but `GoalKind::is_satisfied()` in `crates/worldwake-ai/src/goal_model.rs` currently clears those goals when the drive is still below `medium()`. After TK-2 fixed the owned-consumption loop, the fallback golden showed the same zero-step `GoalSatisfied` pattern for `Relieve` and later `Sleep`: the agent selected those self-care goals, found a 0-step satisfied plan at the root node, and idled instead of dispatching the action.

## Assumption Reassessment (2026-04-06)

1. `golden_fallback_to_addressable_need_when_top_need_unsatisfiable` in `crates/worldwake-ai/tests/golden_ai_decisions.rs` now passes for the TK-2-owned consume path, but its decision trace still records repeated `GoalSatisfied[steps=0]` selections for `Relieve` and `Sleep` after the apples are exhausted.
2. The live symbols under audit are `emit_sleep_goal`, `emit_relieve_goal`, and `GoalKind::{Sleep,Relieve}::is_satisfied` in `crates/worldwake-ai/src/{candidate_generation.rs,goal_model.rs}`.
3. Shared boundary under audit: self-care goal emission thresholds vs. self-care planner terminal thresholds for low-band needs.
4. The invariant is the same one TK-2 corrected for consumption: if a self-care goal is emitted because a low-band need is actionable, the planner must not treat that goal as already satisfied at the root node.
5. This is a separate bug uncovered during TK-2 verification, not a required consequence of the consume fix. It deserves its own ticket because it touches additional goal families (`Sleep`, `Relieve`, and likely `Wash`) with their own proof surface.
6. Ticket says the owned contradiction is only the root-node terminal threshold mismatch. Live `PlannerOpKind::Sleep` handling in `crates/worldwake-ai/src/goal_model.rs` also hardcodes hypothetical relief to `below_medium(thresholds.fatigue.medium())`, while authoritative `sleep` in `crates/worldwake-systems/src/needs_actions.rs` is a one-tick action that reduces fatigue only by `MetabolismProfile::rest_efficiency`. Correction applied: `Sleep` needs both the terminal threshold fix and a matching hypothetical transition so repeated sleep steps can remain planner-visible when one sleep does not clear the low band. Safe because this stays inside the ticket's stated self-care planner-contract boundary.

## Architecture Check

1. Aligning emission and satisfaction thresholds per self-care goal family is cleaner than adding more golden-specific exceptions or selection heuristics; the mismatch lives in the goal contract itself.
2. No backward-compatibility shims are needed. The planner boundary should simply express one consistent low-band actionability contract.

## Verification Layers

1. Low-band `Sleep`/`Relieve` goals do not return zero-step root success -> focused `goal_model` unit/runtime coverage
2. Agents dispatch the first lawful low-band self-care action instead of idling -> golden decision/action trace in `golden_ai_decisions.rs`
3. Existing self-care conformance still passes -> `planner_conformance.rs` plus crate-level `cargo test -p worldwake-ai`

## What to Change

### 1. Align non-consumption self-care terminal checks

Make `GoalKind::{Sleep,Relieve,Wash}` satisfaction boundaries consistent with their current candidate-emission bands, and keep the `Sleep` hypothetical planner transition aligned with the authoritative one-tick `rest_efficiency` contract so repeated sleep steps remain planner-visible when one sleep does not clear the low band.

### 2. Add focused proof for low-band root-node behavior

Add or adjust targeted tests so low-band `Sleep`/`Relieve` goals no longer collapse into `GoalSatisfied[steps=0]` at the root node once selected.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/tests/golden_trade.rs` (modify)

## Out of Scope

- Further changes to `ConsumeOwnedCommodity`; TK-2 already owns that surface
- Merchant selling / `SellCommodity` behavior
- New need-system runtime mechanics outside the planner contract

## Acceptance Criteria

### Tests That Must Pass

1. A focused low-band `Sleep` or `Relieve` proof no longer returns a zero-step satisfied plan at the root node.
2. `golden_fallback_to_addressable_need_when_top_need_unsatisfiable` can continue past the consume phase without idling on low-band `Relieve` or `Sleep`.
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Self-care goals emitted from low-band needs must not be considered satisfied before a lawful action is dispatched or the need actually drops below the matching terminal band.
2. Emission and satisfaction thresholds for each self-care goal family must remain aligned.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — focused low-band self-care satisfaction coverage
2. `crates/worldwake-ai/src/search/tests.rs` — keep flexible-goal search coverage aligned with the corrected low-band sleep contract
3. `crates/worldwake-ai/tests/golden_ai_decisions.rs` — extend fallback golden and recalibrate the triple-need scenario once non-consumption self-care becomes lawfully executable
4. `crates/worldwake-ai/tests/golden_trade.rs` — keep route-knowledge restock coverage isolated from newly lawful self-care travel

### Commands

1. `cargo test -p worldwake-ai --test golden_ai_decisions golden_fallback_to_addressable_need_when_top_need_unsatisfiable -- --exact`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-06.

- Aligned `GoalKind::{Sleep,Relieve,Wash}::is_satisfied()` with the same low-band actionability contract used by candidate generation, so those goals no longer collapse into zero-step root success in the `[low, medium)` band.
- Updated the `Sleep` hypothetical planner transition to subtract `MetabolismProfile::rest_efficiency` per sleep action instead of teleporting fatigue below `medium`, which keeps repeated sleep steps planner-visible when one sleep is not enough.
- Added focused goal-model coverage for low-band self-care satisfaction and cumulative sleep relief, and extended the fallback golden to reject zero-step `GoalSatisfied` selections for `Sleep`/`Relieve`.
- Recalibrated stale search and golden fixtures that had been implicitly relying on the old self-care idling behavior.

## Deviations

- Broader verification surfaced two stale goldens outside the ticket's initial file list: `golden_three_way_need_competition` and `merchant_route_knowledge_alone_does_not_unlock_remote_restock`. Both were fixture-level recalibrations, not new engine changes: the former now seeds lower initial fatigue so it still proves weighted three-way local ordering, and the latter now uses a shorter observation window so route-knowledge coverage is not polluted by newly lawful low-band bladder relief travel.

## Verification Result

- Passed `cargo test -p worldwake-ai goal_model::tests::self_care_goals_remain_unsatisfied_in_low_band --lib -- --exact`
- Passed `cargo test -p worldwake-ai goal_model::tests::sleep_planner_transition_accumulates_rest_efficiency_until_low_band_clears --lib -- --exact`
- Passed `cargo test -p worldwake-ai search::tests::test_binding_flexible_goal_unaffected --lib -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_ai_decisions golden_fallback_to_addressable_need_when_top_need_unsatisfiable -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_ai_decisions golden_three_way_need_competition -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_trade merchant_route_knowledge_alone_does_not_unlock_remote_restock -- --exact`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
