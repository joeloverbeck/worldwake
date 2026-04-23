# SURVTRADE-003: Isolate the Substitute Trade Branch in the Row-9 Roadmap Scenario and Golden

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — scenario/golden proof surface for substitute-driven trade
**Deps**: `archive/tickets/completed/SURVTRADE-002-substitute-trade-selection-and-row9-proof.md`; `docs/scenario-roadmap.md` row 9 `survival-trade`

## Problem

`SURVTRADE-002` lands the focused AI substitute-selection seam, but row 9 still is not truthfully landed. The authored `survival-trade.ron` scenario still stages Bread and gives `Buyer Nila` substitute preferences ordered as `[Bread, Apple, Grain]`, so the roadmap golden still truthfully proves the bread-market branch rather than a substitute-isolation branch.

## Assumption Reassessment (2026-04-23)

1. `crates/worldwake-ai/src/candidate_generation.rs` already emits explicit substitute-backed `AcquireCommodity(SelfConsume)` goals when the preferred direct local trade path is missing.
2. `crates/worldwake-ai/src/ranking.rs` now carries planner-visible substitute-order preference and can rank earlier stored substitutes ahead of rival same-category self-consume acquisition candidates.
3. `crates/worldwake-ai/src/goal_model.rs` still proves that the selected `AcquireCommodity` branch builds an explicit `trade` payload override rather than a hidden runtime commodity rewrite.
4. The current roadmap-owned golden at `crates/worldwake-ai/tests/golden_survival_trade.rs` still proves only the authored bread-market branch.
5. The authored scenario at `scenarios/survival-trade.ron` still keeps Bread locally staged and still lists `SubstitutePreferences` as `{ Food: [Bread, Apple, Grain] }`, so substitute pursuit is not isolated at the scenario layer.
6. The exact boundary now under audit is scenario/golden isolation for row 9: can a truthful roadmap-owned authored scenario force the substitute branch to be the proved causal reason, excluding the currently lawful direct Bread market branch?
7. This is no longer a ranking-sensitive implementation ticket. The remaining work is scenario/golden design and truthful roadmap closeout, not additional AI branch preference arithmetic unless reassessment finds a new contradiction.

## Architecture Check

1. The clean path is to isolate substitute-driven trade in authored scenario state and a roadmap-owned golden instead of trying to infer substitute landing from the existing bread-market survival branch.
2. No backwards-compatibility shims or hidden runtime rewrites are needed; the remaining gap is proof ownership at the scenario/golden layer.

## Verification Layers

1. Authored scenario excludes the direct Bread branch and exposes a substitute-only market decision -> scenario file plus focused golden setup assertions
2. AI-selected branch is substitute-driven for the authored reason -> roadmap-owned golden action trace / selected-branch assertions
3. Row-9 roadmap wording is truthful -> updated `docs/scenario-roadmap.md` backed by the passing substitute golden

## What to Change

### 1. Scenario isolation

Author or revise the roadmap-owned trade scenario so the lawful local survival branch actually depends on substitute-driven trade rather than listed Bread availability.

### 2. Golden ownership

Update `golden_survival_trade.rs` or replace it with the truthful row-owned golden surface that proves substitute-driven trade for the authored causal reason.

### 3. Roadmap closeout

Only promote row 9 when the substitute branch is behaviorally proved by the authored scenario/golden pair.

## Files to Touch

- `scenarios/survival-trade.ron` (modify) or a truthful replacement scenario if reassessment proves the current file should stay bread-market owned
- `crates/worldwake-ai/tests/golden_survival_trade.rs` (modify) or the truthful owning golden replacement
- `docs/scenario-roadmap.md` (modify)

## Out of Scope

- Reopening substitute selection arithmetic in `crates/worldwake-ai/src/ranking.rs` unless reassessment finds a new live ordering contradiction
- Commit-time trade payload mutation
- Non-row-9 trade economy expansion such as merchant restock or recipe-input substitute planning

## Acceptance Criteria

### Tests That Must Pass

1. A roadmap-owned golden proves a substitute-driven trade branch, not just the bread-market branch.
2. The golden's selected branch remains an explicit trade payload for the substitute commodity.
3. If row 9 wording changes to landed, the updated roadmap text is backed by that passing substitute golden.

### Invariants

1. Substitute pursuit remains an explicit commodity goal and explicit trade payload.
2. The roadmap row is only marked landed when the substitute branch is behaviorally proved by the authored scenario/golden pair.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_survival_trade.rs` or its truthful replacement — prove substitute-driven trade as the authored row-9 branch
2. `None` — reuse existing lower-layer substitute-selection and trade-payload proofs from `SURVTRADE-002`; this ticket owns the scenario/golden layer

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_trade -- --ignored --exact <truthful substitute scenario test>`
2. `cargo test -p worldwake-ai <focused supporting selector if the scenario needs a new helper-level proof>`
3. `cargo clippy --workspace --all-targets -- -D warnings`
