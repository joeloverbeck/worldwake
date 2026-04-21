# S112PORPLAN-006: Golden — portfolio planning infeasibility probe

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (golden test ticket)
**Deps**: archive/tickets/S112PORPLAN-005.md

## Problem

S112's core falsification: the portfolio lets an agent commit a feasible lower-motive goal within 2 ticks when the top two candidates are infeasible. Pre-S112, the flat top-N loop would waste ≥5 ticks retrying A and B before reaching C. This ticket adds the golden that proves the post-S112 behavior and guards against regression.

## Assumption Reassessment (2026-04-20)

1. Golden harness for survival-class scenarios lives at `crates/worldwake-ai/tests/golden_survival_baseline.rs`, `golden_survival_contested.rs`, `golden_survival_scattered.rs`, and uses the common `golden_harness/` helpers. The new file `golden_portfolio_planning.rs` follows the same pattern: RON scenario under `scenarios/`, golden assertions over decision history + runtime traces.
2. `docs/generated/golden-e2e-inventory.md` and `docs/generated/golden-scenario-index.md` are the canonical golden test and scenario indexes; they must be regenerated after this ticket lands via `python3 scripts/golden_inventory.py --write --check-docs`.
3. Live `GoalKind`s exercised: one infeasible pair (e.g., two `AcquireCommodity` goals whose anchors point to commodities the agent cannot reach — probe rejects with `MissingObservation`) plus one feasible `ConsumeOwnedCommodity { Survival }` goal with lower motive (e.g., mild hunger against owned food). This matches the three-candidate structure S112's Validation test 10 requires.
4. Motivating invariant (restated): within 2 ticks of the starting state, the agent commits the lowest-motive feasible goal; `GoalCommittedPayload::rejected_alternatives` contains the two infeasible candidates tagged `FeasibilityProbeFailed`. Pre-S112 baseline (reproducible with the same scenario against `main` before ticket 005 lands): agent wastes ≥5 ticks on A and B before reaching C.
5. Scenario isolation: intentionally designed to exercise only the probe-rejection path — no concurrent political pressure, no contested affordances, no background economic churn. Competing lawful affordances that would lawfully compete with the three goals (e.g., social observation, travel) are intentionally absent from the scenario setup; the golden's contract is the probe-driven slot ordering, not broader agent behavior.
6. Harness boundary: full action registries are required — the probe's affordance-existence check needs live `ActionDefRegistry` + `RecipeRegistry` content to distinguish infeasible from feasible commodity goals.

## Architecture Check

1. Golden-only ticket: no engine changes, all behavior changes landed in ticket 005. This ticket is purely a regression surface + canonical falsification case for the S112 architecture.
2. Follows `docs/golden-e2e-testing.md` conventions: decision-trace assertion for slot-attempted order, event-log delta assertion for `rejected_alternatives` contents.
3. Scenario isolation is explicit per precision rule 8: named intended branch, named excluded competing affordances, named isolation rationale.

## Verification Layers

1. Agent commits goal C within 2 ticks → decision-trace assertion via `golden_harness` on the `DecisionOutcome::Planning` arm's committed opportunity.
2. `rejected_alternatives` contains A and B with `FeasibilityProbeFailed` → event-log delta assertion on the `GoalCommittedPayload` emitted at the commit tick.
3. `PortfolioTrace::slots` shows A and B as `RejectedBeforeSearch` and C as `Plausible` → decision-trace assertion on the new portfolio field (ticket 004).

## What to Change

### 1. Create `scenarios/portfolio-planning.ron`

Scenario with:

- One agent with `CognitiveProfile { max_candidates_to_plan: 2, slot_weights: default, ..default }`.
- Two high-motive `AcquireCommodity` goals whose target commodities are at places the agent has no believed route to (or whose targets are unknown in belief) — probe rejects both with `RouteUnknown` / `MissingObservation`.
- One low-motive `ConsumeOwnedCommodity { Survival: Food }` goal satisfiable from the agent's own inventory — probe passes with `Plausible`.
- No other lawful competing affordances (no patrol routes, no political offices, no combat pressures).

Cross-reference existing scenarios (`scenarios/survival-baseline.ron`, `scenarios/survival-contested.ron`) for structural conventions; AgentDef fields per `crates/worldwake-cli/src/scenario/types.rs`.

### 2. Create `crates/worldwake-ai/tests/golden_portfolio_planning.rs`

Following the pattern of `golden_survival_baseline.rs`:

- Load `scenarios/portfolio-planning.ron`.
- Advance the world for 2 ticks.
- Assert: agent committed to goal C within 2 ticks.
- Assert: `GoalCommittedPayload::rejected_alternatives` contains the two infeasible goals tagged `GoalRejectionReason::FeasibilityProbeFailed`.
- Assert: `PlanningPipelineTrace::portfolio` on the commit tick shows exactly 3 slots — 2 `RejectedBeforeSearch` and 1 `Plausible`.
- Assert: `slots_attempted == 1` (only the plausible slot reached `search_plan`).

### 3. Regenerate golden-scenario docs

After the test file is in place and runnable, regenerate:

```bash
python3 scripts/golden_inventory.py --write --check-docs
```

This updates `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and `docs/generated/golden-scenario-details/portfolio-planning.md`.

## Files to Touch

- `scenarios/portfolio-planning.ron` (new)
- `crates/worldwake-ai/tests/golden_portfolio_planning.rs` (new)
- `docs/generated/golden-e2e-inventory.md` (regenerated)
- `docs/generated/golden-scenario-index.md` (regenerated)
- `docs/generated/golden-scenario-details/portfolio-planning.md` (new, regenerated)

## Out of Scope

- Engine or runtime changes — all behavior changes landed in 001–005.
- Multi-tick probe-suppression verification (discrepancy-memory TTL behavior over many ticks) — that's S109's coverage contract, not S112's.
- Observer binary rendering of the new trace — deferred to a follow-up observer ticket.
- Extending `survival-baseline` or `survival-contested` with portfolio-specific assertions — those goldens remain regression-check surfaces for default slot-weight behavior (ticket 005 acceptance).

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_portfolio_planning` passes.
2. `python3 scripts/golden_inventory.py --write --check-docs` produces no diff after regeneration (i.e., the regenerated docs match the committed docs).
3. Existing suite: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. The scenario is deterministic — a fixed seed reproduces the same commit sequence every run.
2. Agent commits within 2 planning ticks (upper bound asserted, not lower-bound motive).
3. All three goals appear in the `PortfolioTrace`; only the `Plausible` one produces a search attempt.
4. `rejected_alternatives` entries use the pre-existing `GoalRejectionReason::FeasibilityProbeFailed` variant (no new variants).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_portfolio_planning.rs` — new golden E2E test per the What to Change section.
2. `scenarios/portfolio-planning.ron` — new scenario file backing the golden.

### Commands

1. `cargo test -p worldwake-ai --test golden_portfolio_planning`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
