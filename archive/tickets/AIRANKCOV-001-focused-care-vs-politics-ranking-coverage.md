# AIRANKCOV-001: Add Focused Coverage For Care-Vs-Politics Ranking Semantics

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: `crates/worldwake-ai/src/ranking.rs`, `crates/worldwake-ai/tests/golden_emergent.rs`, archived `archive/tickets/S13POLEMEGOLSUI-003-wounded-politician-priority-ordering.md`, `docs/FOUNDATIONS.md`

## Problem

The current AI test suite proves the pieces of care-vs-politics ranking separately, but it does not yet make the combined semantics explicit in one focused place. As a result, it is easy to misread the architecture and assume `ClaimOffice` and `TreatWounds` are symmetric weight-driven branches when they are not.

This ticket adds focused coverage that locks down the current architecture: self-treatment is shaped by pain band plus pressure-scaled motive, while office claiming is a fixed `Medium` enterprise goal. The goal is not to change the architecture, but to make the asymmetry explicit and defend it with precise tests.

## Assumption Reassessment (2026-03-19)

1. Existing focused coverage already proves the pieces independently in `crates/worldwake-ai/src/ranking.rs`: `self_treat_wounds_uses_pain_weight_for_motive`, `claim_office_uses_enterprise_weight_and_medium_priority`, `enterprise_does_not_outrank_critical_self_care`, and `medium_priority_enterprise_and_critical_self_care_outrank_share_belief`.
2. Existing golden coverage now proves mixed-layer end-to-end behavior in `crates/worldwake-ai/tests/golden_emergent.rs`: `golden_wounded_politician_pain_first`, `golden_wounded_politician_enterprise_first`, and `golden_wounded_politician_replays_deterministically`.
3. The remaining gap is missing focused/unit coverage for the exact crossover semantics between `GoalKind::TreatWounds { patient: self }` and `GoalKind::ClaimOffice { office }`. The gap is not missing golden coverage; it is missing focused coverage that makes the ranking model legible and maintainable.
4. This is a ranking-layer ticket, not a runtime `agent_tick` or authoritative-system ticket. Local focused/unit coverage in `crates/worldwake-ai/src/ranking.rs` is the intended verification layer; full action registries are not required.
5. The ticket must not weaken or bypass any heuristic/filter. It should codify the current substrate explicitly: pain-driven self-care already stands on concrete wound state and drive thresholds, while `ClaimOffice` currently stands on flat enterprise motive with fixed `Medium` priority. This means only `Critical` self-care has a strict class advantage over `ClaimOffice`; `Medium` self-care shares the same class and must win on motive score.
6. Mismatch + correction: the architecture already supports the observed golden behaviors, but the focused tests do not yet explain why those behaviors happen. The corrected scope is to add explicit unit coverage for the true class-plus-motive crossover, not to imply that all medium-pain self-care automatically outranks politics or to rebalance ranking semantics.

## Architecture Check

1. The clean solution is to add focused tests to `crates/worldwake-ai/src/ranking.rs` rather than encoding more of the explanation into goldens. Ranking semantics belong in the ranking module, where the exact priority-class and motive interactions are visible and stable.
2. This preserves a clean layered test pyramid: focused unit tests prove ranking math and class interactions; goldens prove the mixed-layer end-to-end consequence. No new compatibility paths or special cases are introduced.
3. A broader architectural improvement could eventually replace `ClaimOffice`'s flat enterprise motive with a concrete political opportunity signal, but that would be a production semantics change and belongs in a separate ticket. This ticket should document the current architecture, not smuggle in that redesign.

## Verification Layers

1. self-treatment priority-class and motive interaction vs `ClaimOffice` -> focused unit tests in `crates/worldwake-ai/src/ranking.rs`
2. architectural intent remains aligned with current mixed-layer runtime behavior -> rerun `golden_wounded_politician_*` in `crates/worldwake-ai/tests/golden_emergent.rs`
3. no additional authoritative event-log mapping is required because this ticket targets ranking semantics only

## What to Change

### 1. Add focused crossover tests in ranking

Add unit tests in `crates/worldwake-ai/src/ranking.rs` that prove:
- critical self-treatment outranks `ClaimOffice` even when `ClaimOffice` has the larger motive score
- medium self-treatment can outrank `ClaimOffice` when both share the `Medium` class and self-care wins on motive score
- medium or low self-treatment can rank below `ClaimOffice` when class parity or lower class leaves enterprise ahead

Use scenarios that compare the exact same `GoalKind::TreatWounds { patient: agent }` and `GoalKind::ClaimOffice { office }` candidates, so the tests make the current ranking contract explicit:
- `Critical` self-care wins by priority class even with a weaker motive
- `Medium` self-care and `ClaimOffice` tie on class, so motive score breaks the tie
- `Low` self-care can lose because it starts below `ClaimOffice` on class
Use the existing ranking test harness and exact `GoalKind::TreatWounds { patient: agent }` / `GoalKind::ClaimOffice { office }` comparisons rather than indirect helpers.

### 2. Optionally tighten explanatory comments around the tested behavior

If needed, add short comments near the new tests or nearby ranking helpers clarifying that politics currently has fixed `Medium` priority while self-treatment derives class from pain thresholds. Do not add verbose commentary or policy prose inside production code.

## Files to Touch

- `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs` (modify)
- `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs` (modify only if a rerun reveals a needed naming/coverage alignment)

## Out of Scope

- Any change to `rank_candidates`, `goal_policy`, or `ClaimOffice` semantics
- Introducing a new political opportunity model
- Reworking golden scenarios beyond minimal alignment if test names or comments need tightening

## Acceptance Criteria

### Tests That Must Pass

1. A focused ranking test proves critical self-treatment outranks `ClaimOffice` even when `ClaimOffice` has the larger motive score.
2. A focused ranking test proves medium self-treatment can outrank `ClaimOffice` when both share the `Medium` class and self-care wins on motive score.
3. A focused ranking test proves low-pain self-treatment can rank below `ClaimOffice`.
4. Existing suite: `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_pain_first`
5. Existing suite: `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_enterprise_first`

### Invariants

1. The new focused tests must describe the current architecture accurately without forcing a hidden ranking-policy change.
2. No new test should imply that medium-pain self-care automatically outranks `ClaimOffice`; only critical self-care has a strict class advantage in the current design.
3. No new test should imply that politics is already driven by a concrete pressure/opportunity signal comparable to pain.

## Test Plan

### New/Modified Tests

1. `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs` — add focused crossover tests for `TreatWounds { self }` vs `ClaimOffice` that separately prove critical class dominance, medium-class motive tie-breaking, and low-class loss cases.
2. `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs` — rerun only as regression coverage to confirm the focused interpretation matches end-to-end behavior.

### Commands

1. `cargo test -p worldwake-ai ranking::tests::claim_office_uses_enterprise_weight_and_medium_priority -- --exact`
2. `cargo test -p worldwake-ai ranking::tests::self_treat_wounds_uses_pain_weight_for_motive -- --exact`
3. `cargo test -p worldwake-ai ranking::tests::critical_self_treat_outranks_claim_office_even_with_lower_motive -- --exact`
4. `cargo test -p worldwake-ai ranking::tests::medium_self_treat_and_claim_office_tie_break_on_motive -- --exact`
5. `cargo test -p worldwake-ai ranking::tests::low_self_treat_ranks_below_claim_office -- --exact`
6. `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_pain_first`
7. `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_enterprise_first`
8. `cargo test -p worldwake-ai`
9. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- **Completion date**: 2026-03-19
- **What actually changed**:
  - Added three focused ranking tests in `crates/worldwake-ai/src/ranking.rs`:
    - `critical_self_treat_outranks_claim_office_even_with_lower_motive`
    - `medium_self_treat_and_claim_office_tie_break_on_motive`
    - `low_self_treat_ranks_below_claim_office`
  - Corrected the ticket assumptions before implementation to match the actual ranking contract:
    - `ClaimOffice` is always `Medium` with flat `enterprise_weight` motive.
    - `TreatWounds { self }` derives priority class from pain thresholds and motive from `pain_weight * pain_pressure`.
    - `Medium` self-care does not automatically outrank politics; only `Critical` self-care has a strict class advantage.
  - Corrected the verification commands to use real Cargo test invocations.
- **Deviations from original plan**:
  - No golden or production-code changes were needed. The architecture gap was documentation plus focused ranking coverage, not runtime behavior.
  - The ticket originally implied that medium self-treatment should generally outrank `ClaimOffice`. The current architecture does not implement that; the final scope was narrowed to proving the real class-plus-motive crossover semantics.
- **Verification results**:
  - Passed focused tests:
    - `cargo test -p worldwake-ai ranking::tests::critical_self_treat_outranks_claim_office_even_with_lower_motive -- --exact`
    - `cargo test -p worldwake-ai ranking::tests::medium_self_treat_and_claim_office_tie_break_on_motive -- --exact`
    - `cargo test -p worldwake-ai ranking::tests::low_self_treat_ranks_below_claim_office -- --exact`
  - Passed relevant regression tests:
    - `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_pain_first`
    - `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_enterprise_first`
    - `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_replays_deterministically`
  - Passed broader verification:
    - `cargo test -p worldwake-ai`
    - `cargo test --workspace`
    - `cargo clippy --workspace --all-targets -- -D warnings`
