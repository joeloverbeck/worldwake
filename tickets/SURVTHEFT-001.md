# SURVTHEFT-001: Unblock survival-theft from impossible AcquireCommodity dominance

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes - worldwake-ai candidate generation, ranking/selection, and survival-theft scenario proof surface
**Deps**: docs/scenario-roadmap.md row 12 (`survival-theft`)

## Problem

The planned `survival-theft` roadmap row cannot land truthfully on the live codebase. In the drafted scenario, a merchant stages visible apple stock at a concealed market, but the hungry thief keeps ranking `AcquireCommodity(SelfConsume)` for apples above other needs while the planner finds no plan for that goal and never selects a committed theft branch. The row therefore overstates theft + concealment as landable behavior under the current AI contract.

## Assumption Reassessment (2026-04-24)

1. Focused live proof failed: `cargo test --release -p worldwake-ai --test golden_survival_theft survival_theft_proves_concealed_staged_lot_branch -- --ignored --test-threads=1 --nocapture` repeatedly showed the thief ranking `AcquireCommodity { commodity: Apple, purpose: SelfConsume }` at `Market Hollow` with `plans_found=0`, and no selected theft branch.
2. The roadmap still marks row 12 as planned in [docs/scenario-roadmap.md](/home/joeloverbeck/projects/worldwake/docs/scenario-roadmap.md:164), so there is no existing landed proof surface to preserve.
3. Shared boundary under audit: the contract between `GoalKind::AcquireCommodity { purpose: SelfConsume }`, `GoalKind::StealItem`, candidate generation in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs:3882), and ranking/selection in [ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs:1091).
4. Intended invariant for the motivating scenario: when a merchant stages visible owned food locally and lawful food acquisition is excluded, the live decision pipeline should still admit and select a theft-capable branch before the agent starves.
5. The live `GoalKind` under test is `AcquireCommodity(SelfConsume)` for apples. The live authored theft seam depends on `emit_theft_candidates`, the `steal` transport action, and the ranking/suppression policy for `StealItem`.
6. This is an AI regression at the full golden E2E layer. Local candidate-generation tests are not sufficient because the failure is in the interaction between candidate emission, suppression, ranking, and selection.
7. Ordering divergence is mixed-layer: `AcquireCommodity(SelfConsume)` wins on priority/motive, but its current operator surface does not realize theft, while `StealItem` is either absent or never selected. The contradiction depends on both candidate filtering/suppression and ranking/selection.
8. Current heuristic/filter under suspicion: theft-family suppression and/or the separation between generic acquisition goals and theft-specific goals. This ticket must name the exact live rule that stands in for missing acquisition fallback today and either repair it or narrow the roadmap row around it.
9. The first failure boundary is planning selection, not authoritative action start. The live symbols already implicated are `emit_theft_candidates`, `assess_theft_deterrence`, `GoalKind::StealItem`, and the search selection outcome for `AcquireCommodity(SelfConsume)`.
10. The drafted scenario intentionally excludes lawful competing branches: the thief knows only `Harvest Water`, has no coin, and the merchant stages apples specifically so the row should isolate staged-lot theft rather than trade or harvest.
11. Reassessment exposed one adjacent but insufficient fix: displayed sale lots can expose a visible seller even without a separate owner belief. A bounded candidate-generation fallback was added locally, but the full scenario still failed, so the blocker is broader than sale-lot ownership visibility alone.
12. Mismatch + correction: the row should not be treated as ready-to-land. It is now a drafting-stage roadmap item blocked on this ticket.

## Architecture Check

1. The clean fix is to make the live AI contract truthful about how impossible acquisition goals degrade into theft-capable planning, rather than forcing the roadmap scenario to fake a theft landing through brittle authored thresholds.
2. No compatibility shims should be introduced. The final change should leave one canonical path for theft-capable acquisition under the live planner contract.

## Verification Layers

1. Visible staged food with no lawful food branch admits a theft-capable decision path -> decision trace in `golden_survival_theft.rs`
2. The selected path commits a real `steal` against merchant-owned displayed stock -> action trace + authoritative world state
3. Concealed-place witness perception still modulates the theft event -> perception trace + social observation state
4. If a lower-layer fix is required first, add or update focused `worldwake-ai` tests that prove whether `StealItem` is emitted, suppressed, or outranked under the exact authored market state.

## What to Change

### 1. Reassess the AI contract for theft-capable acquisition

Determine whether the truthful fix is:
- allowing `AcquireCommodity(SelfConsume)` to realize through theft when lawful branches are absent,
- promoting/admitting `StealItem` as the live alternative when acquisition is impossible,
- or changing the suppression/ranking rule that currently prevents the theft branch from ever reaching selection.

### 2. Restore a truthful roadmap proof surface

Once the live planner contract is fixed, re-author `scenarios/survival-theft.ron`, add `golden_survival_theft.rs`, and only then wire the scenario into `.github/workflows/golden-survival.yml`.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/search/*` or adjacent selection logic (modify, if reassessment proves needed)
- `scenarios/survival-theft.ron` (new, only after planner contract is truthful)
- `crates/worldwake-ai/tests/golden_survival_theft.rs` (new, only after planner contract is truthful)
- `.github/workflows/golden-survival.yml` (modify, only after the scenario passes)
- `docs/scenario-roadmap.md` (modify)

## Out of Scope

- Landing `survival-justice`, `survival-patrol`, or later roadmap rows
- Broad theft/justice redesign unrelated to the acquisition-vs-theft selection contradiction
- Weakening place-concealment or witness-fidelity behavior just to force the row green

## Acceptance Criteria

### Tests That Must Pass

1. A focused `worldwake-ai` test proves the corrected theft-capable acquisition contract at the exact candidate/ranking boundary being changed.
2. `cargo test --release -p worldwake-ai --test golden_survival_theft survival_theft_proves_concealed_staged_lot_branch -- --ignored --test-threads=1`
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The planner must not keep selecting an impossible generic self-consume acquisition branch while a truthful theft-capable local branch exists and is supposed to own the scenario.
2. The row must only be marked landed once the committed theft, post-theft eat, and concealed witness-fidelity seam are all proved in the live golden.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` or adjacent focused tests - prove the precise theft-capable acquisition admission rule under staged visible stock.
2. `crates/worldwake-ai/tests/golden_survival_theft.rs` - prove the full 1440-tick roadmap-owned behavior once the planner contract is fixed.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --release -p worldwake-ai --test golden_survival_theft survival_theft_proves_concealed_staged_lot_branch -- --ignored --test-threads=1`
3. `cargo clippy --workspace --all-targets -- -D warnings`
