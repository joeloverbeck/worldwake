# AIRANKCOV-001: Focused Ranking Coverage — Promoted Hunger Beats Higher-Motive Wash

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: [archive/tickets/completed/S17WOULIFGOLSUI-002.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S17WOULIFGOLSUI-002.md), [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs)

## Problem

Current focused ranking coverage proves the pieces of the Scenario 30 behavior separately:

- clotted wounds can promote hunger from `High` to `Critical`,
- `Wash` does not receive that promotion,
- motive scoring multiplies utility weight by pressure,
- cross-family ordering prefers higher priority class before motive.

What is still missing is one focused test that composes those facts into the exact contract exposed by Scenario 30: a promoted hunger goal should outrank a `Wash` goal even when `Wash` has the higher motive score inside the unpromoted `High` class.

That focused proof would make future regressions much easier to localize than relying only on the golden.

## Assumption Reassessment (2026-03-21)

1. The relevant ranking behavior lives in [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs), especially:
   - `drive_priority()`
   - `promote_for_clotted_wound_recovery()`
   - `motive_score()`
   - `compare_ranked_goals()`
2. Existing focused tests cover adjacent but not identical behavior:
   - `clotted_wound_boosts_hunger_high_to_critical`
   - `clotted_wound_no_boost_relieve_or_wash`
   - `hunger_candidate_becomes_critical_and_uses_weight_times_pressure`
   - `same_priority_candidates_sort_by_motive_then_kind_then_ids`
   - `critical_self_treat_outranks_claim_office_even_with_lower_motive`
   These prove the ingredients, but not the exact hunger-vs-wash comparison shape that Scenario 30 depends on.
3. Existing golden coverage now proves the runtime chain in [crates/worldwake-ai/tests/golden_combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs) via `golden_recovery_aware_boost_eats_before_wash`, but that is intentionally a mixed-layer scenario. The missing gap is a lower-layer ranking proof that isolates just the ordering contract.
4. This is a focused/unit AI ticket. The intended layer is ranking-unit coverage, not runtime `agent_tick`, action execution, or authoritative state.
5. Ordering is the contract here, but specifically ranking-order ordering inside one ranked-candidate list. The compared branches are intentionally asymmetric in motive score and differ in final priority class because of recovery-aware promotion.
6. No heuristic/filter removal, stale-request boundary, political closure boundary, or `ControlSource` semantics apply.
7. Concrete arithmetic for the intended test should mirror the live Scenario 30 shape:
   - hunger at `pm(760)`
   - dirtiness at `pm(860)`
   - default utility weights `pm(500)`
   - bread motive `380_000`
   - wash motive `430_000`
   - clotted wound present
   This produces the exact "lower motive, higher final class" comparison the ticket should pin.
8. Mismatch corrected: the missing gap is not that ranking behavior is untested in general; it is that the exact promoted-hunger-vs-higher-motive-wash comparison is not yet named and locked down at the focused layer.

## Architecture Check

1. A focused ranking test is the clean complement to the existing golden. It proves the earliest causal boundary directly and keeps future failures diagnosable without replaying a full mixed-layer scenario.
2. This should remain a test-only ticket. The current ranking architecture is correct; the gap is precision of focused coverage.
3. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. A clotted-wound hunger candidate outranks `Wash` despite lower motive score because class is compared before motive -> focused unit test in [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs)
2. Existing Scenario 30 golden remains the downstream runtime proof of the same contract through action execution and recovery -> existing golden coverage in [crates/worldwake-ai/tests/golden_combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs)
3. Single-layer note: no additional runtime or authoritative verification is required because this ticket is intentionally about the earliest ranking boundary

## What to Change

### 1. Add one focused comparison test in `ranking.rs`

Add a ranking-unit test that constructs the live Scenario 30 comparison shape directly:

- clotted wound present
- hunger at `High`
- dirtiness at `High` but with higher raw pressure
- default utility weights
- candidates: `ConsumeOwnedCommodity { Bread }` and `Wash`

The test should assert:

- `Wash` has the higher raw `motive_score`
- `ConsumeOwnedCommodity { Bread }` is promoted to `Critical`
- `Wash` remains `High`
- bread ranks before wash in the final ranked list

### 2. Keep the test isolated and concrete

Do not add runtime harness setup or trace assertions here. This ticket is specifically about ranking-unit coverage.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify)

## Out of Scope

- Any production behavior changes
- Decision-trace payload changes
- Golden-test changes
- Harness changes

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai clotted_wound_boosts_hunger_high_to_critical -- --exact`
2. `cargo test -p worldwake-ai clotted_wound_no_boost_relieve_or_wash -- --exact`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The focused layer explicitly proves that a promoted hunger branch can beat a higher-motive wash branch
2. The test uses live arithmetic rather than narrative assumptions
3. No production code changes are introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` — add one focused test for the promoted-hunger-vs-higher-motive-wash comparison that Scenario 30 depends on

### Commands

1. `cargo test -p worldwake-ai clotted_wound_boosts_hunger_high_to_critical -- --exact`
2. `cargo test -p worldwake-ai clotted_wound_no_boost_relieve_or_wash -- --exact`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
