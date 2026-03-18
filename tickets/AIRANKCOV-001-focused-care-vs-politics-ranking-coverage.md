# AIRANKCOV-001: Add Focused Coverage For Care-Vs-Politics Ranking Semantics

**Status**: PENDING
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
5. The ticket must not weaken or bypass any heuristic/filter. It should codify the current substrate explicitly: pain-driven self-care already stands on concrete wound state and drive thresholds, while `ClaimOffice` currently stands on flat enterprise motive with fixed `Medium` priority.
6. Mismatch + correction: the architecture already supports the observed golden behaviors, but the focused tests do not yet explain why those behaviors happen. The corrected scope is to add explicit unit coverage, not to rebalance ranking semantics.

## Architecture Check

1. The clean solution is to add focused tests to `crates/worldwake-ai/src/ranking.rs` rather than encoding more of the explanation into goldens. Ranking semantics belong in the ranking module, where the exact priority-class and motive interactions are visible and stable.
2. This preserves a clean layered test pyramid: focused unit tests prove ranking math and class interactions; goldens prove the mixed-layer end-to-end consequence. No new compatibility paths or special cases are introduced.

## Verification Layers

1. self-treatment priority and motive interaction vs `ClaimOffice` -> focused unit tests in `crates/worldwake-ai/src/ranking.rs`
2. architectural intent remains aligned with current mixed-layer runtime behavior -> rerun `golden_wounded_politician_*` in `crates/worldwake-ai/tests/golden_emergent.rs`
3. no additional authoritative event-log mapping is required because this ticket targets ranking semantics only

## What to Change

### 1. Add focused crossover tests in ranking

Add unit tests in `crates/worldwake-ai/src/ranking.rs` that prove:
- medium or critical self-treatment outranks `ClaimOffice` even against strong enterprise weight
- low-pain self-treatment can rank below `ClaimOffice`
- the crossover is driven by priority class plus motive, not by motive alone

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

1. A focused ranking test proves medium/critical self-treatment outranks `ClaimOffice`.
2. A focused ranking test proves low-pain self-treatment can rank below `ClaimOffice`.
3. A focused ranking test proves the crossover depends on both priority class and motive, not motive alone.
4. Existing suite: `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_pain_first`
5. Existing suite: `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_enterprise_first`

### Invariants

1. The new focused tests must describe the current architecture accurately without forcing a hidden ranking-policy change.
2. No new test should imply that politics is already driven by a concrete pressure/opportunity signal comparable to pain.

## Test Plan

### New/Modified Tests

1. `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs` — add focused crossover tests for `TreatWounds { self }` vs `ClaimOffice` so the asymmetry is explicit at the ranking layer.
2. `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs` — rerun only as regression coverage to confirm the focused interpretation matches end-to-end behavior.

### Commands

1. `cargo test -p worldwake-ai ranking::tests::self_treat_wounds_uses_pain_weight_for_motive`
2. `cargo test -p worldwake-ai ranking::tests::claim_office_uses_enterprise_weight_and_medium_priority`
3. `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_pain_first`
4. `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_enterprise_first`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `scripts/verify.sh`

