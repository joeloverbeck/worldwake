# S112PORPLAN-006: Golden — portfolio planning infeasibility probe

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (golden test ticket)
**Deps**: archive/tickets/S112PORPLAN-005.md

## Problem

S112's core falsification: the portfolio lets an agent commit a feasible lower-motive goal within 2 ticks when the top two candidates are infeasible. Pre-S112, the flat top-N loop would waste ≥5 ticks retrying A and B before reaching C. This ticket adds the golden that proves the post-S112 behavior and guards against regression.

## Assumption Reassessment (2026-04-20)

1. Golden harness for survival-class scenarios lives at `crates/worldwake-ai/tests/golden_survival_baseline.rs`, `golden_survival_contested.rs`, `golden_survival_scattered.rs`, and uses the common `golden_harness/` helpers. The new file `golden_portfolio_planning.rs` follows the same pattern: RON scenario under `scenarios/`, golden assertions over decision history + runtime traces.
2. `docs/generated/golden-e2e-inventory.md` and `docs/generated/golden-scenario-index.md` are the canonical golden test and scenario indexes; they must be regenerated after this ticket lands via `python3 scripts/golden_inventory.py --write --check-docs`.
3. The live branch does not support the ticket's original commodity-only sketch honestly. Local discrepancy/blocker memory suppresses `AcquireCommodity` before portfolio admission, disconnected remote commodity branches do not survive into the ranked set, and the earlier injected-apple commitment path did not surface as a lawful portfolio slot in the authored scenario.
4. The strongest honest three-slot golden on the landed 005 surface is instead:
   - `Sleep` in the survival slot, rejected by the feasibility probe via a scoped blocker-memory entry
   - `ReportMissing` in the commitment slot, seeded from an overdue expectation and rejected by the feasibility probe because the subject is not believed
   - `ProduceCommodity { recipe_id: Bake Bread }` in the economic slot, plausible and committed within 2 ticks
5. Motivating invariant (restated): within 2 ticks of the starting state, the agent commits the feasible lower-motive economic goal; `GoalCommittedPayload::rejected_alternatives` contains the rejected `Sleep` and `ReportMissing` goals tagged `FeasibilityProbeFailed`.
6. Scenario isolation: intentionally designed to exercise only the probe-rejection path. The authored world is a single-place workshop with one lawful bread-production branch and no travel, combat, trade, politics, alternate workstation family, or extra self-care support.
7. Harness boundary: full action registries are required. The test seeds the overdue expectation, violation profile, belief snapshot, and scoped blocker-memory entry through the golden harness because the scenario schema does not directly author those runtime surfaces.

## Assumption Reassessment (2026-04-21)

1. The original three-goal sketch in this ticket (`AcquireCommodity`, `AcquireCommodity`, `ConsumeOwnedCommodity`) is not an honest fit for the landed 005 portfolio surface. On the live branch, all three of those goals would classify into the **survival** slot, so they cannot yield the ticket's claimed `PortfolioTrace` shape of `2 RejectedBeforeSearch + 1 Plausible` across three slots.
2. A first hybrid rewrite that relied on disconnected remote commodity sources was also not honest against the landed planner: those route-unknown self-consume branches never survived into the ranked-goal set on the live branch, so they could not populate `PortfolioTrace` or `rejected_alternatives`.
3. The landed golden uses a **single-place** authored scenario with one lawful economic opportunity (`Bake Bread`) and harness-seeded runtime state for the two rejected slots:
   - scoped blocker memory yields a probe-only rejection for `Sleep`
   - an overdue expectation plus violation profile emits `ReportMissing`, and a preserved committed plan keeps that goal in the commitment slot on the winning tick
4. The feasible selected goal is therefore a lawful `ProduceCommodity { recipe_id: Bake Bread }` economic opportunity. This still proves S112's real invariant: lower-motive feasible work can win the tick once higher-motive infeasible slots are probe-rejected.
5. Ticket 005 already proved the lower-layer integration contract in focused planning tests. This golden remains test-only and owns only the end-to-end authored proof surface plus regenerated golden docs.

## Architecture Check

1. Golden-only ticket: no engine changes, all behavior changes landed in ticket 005. This ticket is purely a regression surface + canonical falsification case for the S112 architecture.
2. Follows `docs/golden-e2e-testing.md` conventions: decision-trace assertion for slot-attempted order, event-log delta assertion for `rejected_alternatives` contents.
3. Scenario isolation is explicit per precision rule 8: named intended branch, named excluded competing affordances, named isolation rationale.

## Verification Layers

1. Agent commits the feasible economic goal within 2 ticks → decision-trace assertion via `DecisionOutcome::Planning`.
2. `rejected_alternatives` contains the rejected `Sleep` and `ReportMissing` slot goals with `FeasibilityProbeFailed` → event-log payload assertion on the `GoalCommittedPayload` emitted at the commit tick.
3. `PortfolioTrace::slots` shows exactly 3 slots on the winning planning tick:
   - survival = `RejectedBeforeSearch`
   - commitment = `RejectedBeforeSearch`
   - economic = `Plausible`
   and `slots_attempted == 1`.

## What to Change

### 1. Create `scenarios/portfolio-planning.ron`

Scenario with:

- One AI agent at a single local workshop place with:
  - `known_recipes: ["Bake Bread"]`
  - local `Firewood`
  - a local `Mill`
  - no alternate workstation family or second recipe
  - no local sleep support beyond the seeded blocker-memory proof seam
  - need/profile tuning that keeps `Sleep`, `ReportMissing`, and local bread production simultaneously rankable on the landed branch
- No other lawful competing affordances (no travel graph complexity, combat, patrol, politics, trade churn, or extra self-care branches).

The scenario itself does **not** author a persisted belief store, blocker memory, expectation store, or prior commitment. The golden harness seeds the initial local beliefs plus the runtime inputs needed to exercise the live commitment-slot and probe contracts because the scenario schema does not currently expose all of those surfaces.

Cross-reference existing scenarios (`scenarios/survival-baseline.ron`, `scenarios/survival-contested.ron`) for structural conventions; AgentDef fields per `crates/worldwake-cli/src/scenario/types.rs`.

### 2. Create `crates/worldwake-ai/tests/golden_portfolio_planning.rs`

Following the pattern of `golden_survival_baseline.rs`:

- Load `scenarios/portfolio-planning.ron`.
- Seed local beliefs for the agent.
- Seed a scoped blocker-memory entry for `Sleep` so the candidate survives into portfolio admission but is rejected by the feasibility probe before search.
- Seed an overdue expectation plus violation profile for a subject missing from beliefs, then inject the matching committed `ReportMissing` opportunity into `AgentTickDriver` runtime and `ActiveGoal`, so the live commitment slot is populated honestly on the winning tick.
- Advance the world for at most 2 ticks.
- Assert: the agent commits the feasible `ProduceCommodity { recipe_id: Bake Bread }` goal within 2 ticks.
- Assert: `GoalCommittedPayload::rejected_alternatives` contains:
  - rejected survival-slot `Sleep`
  - rejected commitment-slot `ReportMissing { subject, expectation_id }`
  each tagged `GoalRejectionReason::FeasibilityProbeFailed`.
- Assert: `PlanningPipelineTrace::portfolio` on the winning planning tick shows exactly 3 populated slots with the landed feasibility shape and `slots_attempted == 1`.

### 3. Regenerate golden-scenario docs

After the test file is in place and runnable, regenerate:

```bash
python3 scripts/golden_inventory.py --write --check-docs
```

This updates the generated golden inventory set, including `docs/generated/golden-coverage-matrix.md`, `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and `docs/generated/golden-scenario-details/portfolio-planning.md`.

## Files to Touch

- `scenarios/portfolio-planning.ron` (new)
- `crates/worldwake-ai/tests/golden_portfolio_planning.rs` (new)
- `docs/generated/golden-coverage-matrix.md` (regenerated)
- `docs/generated/golden-e2e-inventory.md` (regenerated)
- `docs/generated/golden-scenario-index.md` (regenerated)
- `docs/generated/golden-scenario-details/*.md` (regenerated, including new `portfolio-planning.md`)

## Out of Scope

- Engine or runtime changes — all behavior changes landed in 001–005.
- Multi-tick probe-suppression verification (discrepancy-memory TTL behavior over many ticks) — that's S109's coverage contract, not S112's.
- Observer binary rendering of the new trace — deferred to a follow-up observer ticket.
- Extending `survival-baseline` or `survival-contested` with portfolio-specific assertions — those goldens remain regression-check surfaces for default slot-weight behavior (ticket 005 acceptance).
- Making scenario files author `AgentBeliefStore`, `ActiveGoal`, or `AgentDecisionRuntime` directly. This ticket uses explicit harness seeding instead.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_portfolio_planning` passes.
2. `python3 scripts/golden_inventory.py --write --check-docs` produces no diff after regeneration (i.e., the regenerated docs match the committed docs).
3. Existing suite: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. The scenario is deterministic — a fixed seed reproduces the same commit sequence every run.
2. Agent commits within 2 planning ticks (upper bound asserted, not lower-bound motive).
3. The winning planning tick records exactly three populated portfolio slots:
   - rejected survival slot
   - rejected commitment slot
   - plausible economic slot
   and only the plausible slot produces a search attempt.
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

## Outcome

Completed: 2026-04-21

Implemented a new `golden_portfolio_planning` authored scenario plus end-to-end golden that proves the landed S112 portfolio behavior on the live branch:

- `Sleep` is admitted into the survival slot and rejected by the feasibility probe via scoped blocker memory.
- A preserved committed `ReportMissing` opportunity is admitted into the commitment slot and rejected by the feasibility probe because the subject is not believed.
- Local `ProduceCommodity { recipe_id: Bake Bread }` remains plausible in the economic slot and is committed within 2 ticks.

The golden also proves that the winning planning tick records exactly three populated portfolio slots, `slots_attempted == 1`, and `GoalCommittedPayload::rejected_alternatives` carries both rejected slot goals tagged `FeasibilityProbeFailed`.

Deviations from original plan:

- The draft commodity-only three-goal seam was not reachable on the landed 005 branch. Reassessment narrowed the golden to the strongest honest live seam: `Sleep` + `ReportMissing` rejections ahead of plausible `Bake Bread` production.
- The generated golden inventory pass updated broader existing `docs/generated/*` inventory/detail surfaces in addition to the new `portfolio-planning` scenario page.

## Verification Result

Passed:

1. `cargo test -p worldwake-ai --test golden_portfolio_planning`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
