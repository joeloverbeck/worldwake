# S137PLACAULIN-010: Golden plan-repair scenarios + Phase 11 gate

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None — net-new test file + scenario fixtures
**Deps**: archive/tickets/S137PLACAULIN-011.md (successful localized repair handlers), archive/tickets/S137PLACAULIN-007.md (revalidation routing), tickets/S137PLACAULIN-008.md (RepairAttemptTrace), tickets/S137PLACAULIN-009.md (observer rendering)

## Problem

S137's Validation section requires five golden scenarios exercising the bounded localized repair path and a Phase 11 gate metric: `survival-baseline.ron` shows >=30% reduction in `EventTag::ReplanTriggered` count compared to a pre-S137 baseline. Without these goldens, the spec's behavioral contract (repair runs before full replan; budget exhaustion falls through; repeat-failed repair surrenders; `DiscrepancyClearing` clearing resolves blockers structurally; merchant-moved breach rebinds to a sibling) is unverified.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Existing golden tests under `crates/worldwake-ai/tests/` include `golden_survival_*`, `golden_ai_decisions`, `golden_planner_pathology`, etc. No `golden_plan_repair.rs` exists — confirmed by `ls crates/worldwake-ai/tests/golden_plan_repair*` returning empty. Scenario fixtures `survival-baseline.ron`, `survival-scattered.ron`, `survival-contested.ron` all exist under `scenarios/`. The Phase 11 gate fires on `survival-baseline.ron` per spec.
2. Spec `specs/S137-plan-causal-links-and-repair.md` Validation section enumerates the five scenarios: (1) merchant-moved breach → `RebindTarget`; (2) stale belief breach → `InsertVerification` (or `NoEpistemicSubstrate` short-circuit without S139); (3) repeat-failed repair → `Abandon`; (4) `DiscrepancyClearing::CommodityAvailabilityChanged` met → blocker cleared structurally; (5) repair budget exhaustion → fall-through to full replan with `RepairOutcome::Failed` recorded.
3. Shared boundary: the golden test framework expects scenario RON + named assertions. Per `docs/golden-e2e-testing.md` (referenced by `tickets/README.md`), goldens map invariants to proof surfaces via action trace, decision trace, event-log delta, and authoritative world state — the same Verification Layers contract as ticket-level work.
4. **Phase 11 gate metric — concrete delta math**: the metric is "≥30% reduction in `EventTag::ReplanTriggered` count on `survival-baseline.ron` over a 1440-tick replay." Pre-S137 baseline must be captured at the commit immediately before this ticket lands. The expected counter relationship: `ReplanTriggered_after / ReplanTriggered_before ≤ 0.7`, with `RepairApplied_after ≈ (ReplanTriggered_before − ReplanTriggered_after) ± 10%`. Implementer captures the baseline once at the start of the ticket (`git stash` the S137 changes, run the existing baseline, record the count, restore).
5. **Live `GoalKind` under test**: scenarios primarily exercise `GoalKind::TravelTo` and `GoalKind::Trade` (merchant-moved breach), plus `GoalKind::ProduceCommodity` (recipe-driven rebind cases). The active operator surface: `PlannerOpKind::Travel`, `PlannerOpKind::Trade`, `PlannerOpKind::CraftRecipe`. All exist in current planner.
6. **Coverage gap**: existing tests `classify_accepted_repair_*` (planning.rs:3354-3461) cover post-hoc classification only — they do not cover the pre-failure repair path. `S137PLACAULIN-006` covers the staged module and `S137PLACAULIN-011` covers successful localized strategy handlers; this ticket adds the golden E2E and runtime trace coverage after those surfaces are live.
7. **Scenario isolation**: each of the 5 scenarios must isolate one repair-kind branch from lawful competing affordances. For scenario 2 (`InsertVerification`), exclude lawful direct-belief-acquisition paths so the planner doesn't bypass the stale-belief breach. For scenario 5 (budget exhaustion), set `repair_budget_fraction` low enough that all attempted kinds fail before any can succeed.
8. **Cumulative arithmetic — Phase 11 gate**: the 30% reduction threshold is the contract. Validate survivability: if the actual reduction is between 20% and 30%, the gate fails — the ticket either tightens the repair search's coverage or refines the metric definition rather than weakening the threshold. If reduction is ≥30%, the gate passes; the absolute counter values are recorded in the golden assertion for reproducibility.

## Architecture Check

1. **Test-only changes**: no production code modified; pure validation surface addition. The 5 scenarios + Phase 11 gate exercise existing tickets 001-009 end-to-end.
2. **Mixed-layer verification per precision rule #5**: each scenario maps invariants to specific proof surfaces (decision trace for `RepairKind` selection rationale, action trace for plan-replacement ordering, event-log delta for `RepairApplied`/`ReplanTriggered` counter math, authoritative world state for blocker-cleared invariant).

## Verification Layers

1. `RebindTarget` rationale → decision-trace `RepairAttemptTrace.chosen_kind == Some(RebindTarget)` + `substitute_target` matches the sibling merchant entity.
2. `InsertVerification` short-circuit (without S139) → decision-trace `RepairAttemptTrace.rejected` contains `(InsertVerification, NoEpistemicSubstrate)`.
3. `Abandon` after repeat failure → decision-trace shows two prior `RepairMemory.repairs[signature].succeeded == false` entries within TTL.
4. `CommodityAvailabilityChanged` blocker clearing → authoritative world state (BlockerMemory diff) + event-log delta showing `EventTag::BlockerCleared` (or equivalent existing tag — verify during reassessment).
5. Budget exhaustion fall-through → action trace + event-log delta showing both `RepairAttemptTrace` with `chosen_kind == None` and subsequent `EventTag::ReplanTriggered`.
6. Phase 11 gate counter math → event-log delta over 1440 ticks asserting `ReplanTriggered` count is at most 70% of baseline.

## What to Change

### 1. Create `crates/worldwake-ai/tests/golden_plan_repair.rs`

Five golden scenarios per spec Validation section. Each scenario:

- Loads a scenario file (existing `survival-*.ron` for some, or new fixtures for scenarios needing specific breach conditions).
- Drives the simulation for a bounded tick range.
- Asserts the expected `RepairAttemptTrace`, `RepairAppliedPayload`, event-log counters, and authoritative state per the verification layers above.

Scenario list:

- **`golden_plan_repair::merchant_moved_breach_rebinds_to_sibling`** — exercises `RepairKind::RebindTarget`.
- **`golden_plan_repair::stale_belief_breach_attempts_insert_verification`** — exercises `InsertVerification` + short-circuit when S139 not landed (asserts `NoEpistemicSubstrate` failure entry).
- **`golden_plan_repair::repeat_failed_repair_surrenders_with_abandon`** — exercises TTL-suppressed repair memory.
- **`golden_plan_repair::commodity_availability_changed_clears_blocker_structurally`** — exercises `DiscrepancyClearing` consumption.
- **`golden_plan_repair::repair_budget_exhaustion_falls_through_to_full_replan`** — exercises bounded budget + fall-through to `ReplanTriggered`.

### 2. Phase 11 gate counter assertion

Add `golden_plan_repair::phase_11_gate_baseline_ron_replan_reduction` running `survival-baseline.ron` for 1440 ticks. Capture the pre-S137 baseline once (in a fixture constant — `BASELINE_REPLAN_TRIGGERED_COUNT_PRE_S137`) and assert:

```rust
let replan_count = count_events(&event_log, EventTag::ReplanTriggered);
assert!(replan_count <= (BASELINE_REPLAN_TRIGGERED_COUNT_PRE_S137 * 70) / 100,
    "Phase 11 gate: ReplanTriggered count {} exceeds 70% of pre-S137 baseline {}",
    replan_count, BASELINE_REPLAN_TRIGGERED_COUNT_PRE_S137);
```

The baseline constant is captured during this ticket's implementation by running `survival-baseline.ron` against `main` immediately before merging this ticket, recording the count.

### 3. No-regression assertions for existing survival goldens

Add a sub-test confirming `golden_survival_baseline`, `golden_survival_scattered`, `golden_survival_contested` continue to pass without modification — repair is additive at the agent-tick level.

### 4. Scenario fixtures (if needed)

If any of the 5 scenarios require a tailored RON fixture (e.g., scenario 1 needs a guaranteed-movable merchant), add it under `scenarios/` and reference from the test. Reuse existing `survival-*.ron` where possible.

## Files to Touch

- `crates/worldwake-ai/tests/golden_plan_repair.rs` (new)
- Likely: `scenarios/plan-repair-*.ron` (new — tailored fixtures for scenarios requiring specific breach conditions; reuse existing where possible)

## Out of Scope

- Production code changes — all production work landed in tickets 001-009.
- Modifications to existing `golden_survival_*` tests — those are no-regression coverage and must remain unmodified.
- S139 integration — ticket asserts the `NoEpistemicSubstrate` short-circuit; S139's landing will allow scenario 2 to be re-asserted with successful `InsertVerification`.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_plan_repair` — all 5 scenarios + Phase 11 gate pass.
2. `cargo test -p worldwake-ai --test golden_survival_baseline` — no regression.
3. `cargo test -p worldwake-ai --test golden_survival_scattered` — no regression.
4. `cargo test -p worldwake-ai --test golden_survival_contested` — no regression.
5. `scripts/verify.sh` — full CI gate passes.

### Invariants

1. Phase 11 gate counter math holds: `survival-baseline.ron` 1440-tick replay produces `EventTag::ReplanTriggered` count ≤ 70% of pre-S137 baseline.
2. Each of the 5 scenarios maps every claimed invariant to a single, named verification surface (no collapsing distinct layers into one trace assertion).
3. Repair is additive: existing `golden_survival_*` tests pass without modification.
4. Scenarios are deterministic — same seed produces same `RepairAttemptTrace` contents and same event-log counter values.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_plan_repair.rs` — new file containing the 5 scenarios + Phase 11 gate test + no-regression cross-references.
2. Optional new RON fixtures under `scenarios/plan-repair-*.ron` if reusing `survival-*.ron` is insufficient for scenario isolation per precision rule #8.

### Commands

1. `cargo test -p worldwake-ai --test golden_plan_repair`
2. `cargo test -p worldwake-ai --test golden_survival_baseline`
3. `cargo test -p worldwake-ai --test golden_survival_scattered`
4. `cargo test -p worldwake-ai --test golden_survival_contested`
5. `cargo test --workspace`
6. `scripts/verify.sh`

If `docs/generated/golden-e2e-inventory.md` is regenerated as part of golden additions, run `python3 scripts/golden_inventory.py --write --check-docs` and commit the regenerated files per `tickets/README.md`.
