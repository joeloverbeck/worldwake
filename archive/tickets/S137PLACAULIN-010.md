# S137PLACAULIN-010: Golden plan-repair substrate scenarios

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None — net-new golden test file + generated golden docs
**Deps**: archive/tickets/S137PLACAULIN-011.md (successful localized repair handlers), archive/tickets/S137PLACAULIN-007.md (revalidation routing), archive/tickets/S137PLACAULIN-008.md (RepairAttemptTrace), archive/tickets/S137PLACAULIN-009.md (observer rendering)

## Problem

S137's Validation section requires golden coverage for the bounded localized repair path. Before this ticket, the live code had focused unit/runtime proof for the `plan_repair` module and revalidation routing, but no generated golden inventory surface dedicated to the S137 repair-search contract. This ticket adds the truthful golden layer for the repair substrate that is live now: sibling rebinding, `InsertVerification` short-circuit while S139 is absent, repair-memory retry suppression, structural commodity clearing, budget-exhaustion fall-through, and the `Abandon` final local outcome.

The drafted Phase 11 gate metric (`survival-baseline.ron` shows >=30% reduction in `EventTag::ReplanTriggered` count compared to a pre-S137 baseline) was reassessed during this ticket. The live 1440-tick replay currently emits `ReplanTriggered=82` and `RepairApplied=0`, so that gate is not a test-only missing-golden slice. Follow-up `tickets/S137PLACAULIN-012.md` owns diagnosing and landing the survival-baseline gate witness.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Existing golden tests under `crates/worldwake-ai/tests/` include `golden_survival_*`, `golden_ai_decisions`, `golden_planner_pathology`, etc. No `golden_plan_repair.rs` existed before this ticket. Scenario fixtures `survival-baseline.ron`, `survival-scattered.ron`, `survival-contested.ron` all exist under `scenarios/`.
2. Spec `specs/S137-plan-causal-links-and-repair.md` Validation section originally enumerated five scenarios plus the Phase 11 counter gate. Live proof narrowed this ticket to six substrate goldens: `RebindTarget`, `InsertVerification` short-circuit, one-kind retry suppression, `CommodityAvailabilityChanged` clearing, repair-budget exhaustion, and `Abandon` plan shape.
3. Shared boundary: the golden test framework expects scenario RON + named assertions. Per `docs/golden-e2e-testing.md` (referenced by `tickets/README.md`), goldens map invariants to proof surfaces via action trace, decision trace, event-log delta, and authoritative world state — the same Verification Layers contract as ticket-level work.
4. **Phase 11 gate metric — concrete delta math**: the metric remains "≥30% reduction in `EventTag::ReplanTriggered` count on `survival-baseline.ron` over a 1440-tick replay," but that metric is deferred to `tickets/S137PLACAULIN-012.md` because the current live replay emits no `RepairApplied` events.
5. **Live `GoalKind` under test**: scenarios primarily exercise `GoalKind::TravelTo` and `GoalKind::Trade` (merchant-moved breach), plus `GoalKind::ProduceCommodity` (recipe-driven rebind cases). The active operator surface: `PlannerOpKind::Travel`, `PlannerOpKind::Trade`, `PlannerOpKind::CraftRecipe`. All exist in current planner.
6. **Coverage gap**: existing tests `classify_accepted_repair_*` (planning.rs:3354-3461) cover post-hoc classification only — they do not cover the pre-failure repair path. `S137PLACAULIN-006` covers the staged module and `S137PLACAULIN-011` covers successful localized strategy handlers; this ticket adds the golden E2E and runtime trace coverage after those surfaces are live.
7. **Scenario isolation**: each substrate golden isolates one repair-kind branch from lawful competing affordances using programmatic `PlanRepairContext` fixtures. For `InsertVerification`, no lawful S139 epistemic substrate is provided, so the assertion is `NoEpistemicSubstrate`. For budget exhaustion, `repair_budget_fraction` and `max_node_expansions` yield zero budget so the first repair kind records `BudgetExhausted`.
8. **Cumulative arithmetic — Phase 11 gate superseded for this ticket**: the 30% reduction threshold remains the Phase 11 gate contract, but it is not implemented in this ticket. Focused live proof from the first `golden_plan_repair` attempt showed `survival-baseline.ron` currently emits `ReplanTriggered=82` and `RepairApplied=0` over 1440 ticks. A golden counter assertion would therefore be false. `tickets/S137PLACAULIN-012.md` owns the survival-baseline gate diagnosis and implementation.
9. **Repeat-failed repair reassessment**: the live `RepairMemory` is keyed as `BTreeMap<BreachSignature, RepairEntry>`, so one breach signature stores one latest `RepairEntry` with one `RepairKind`. The original "two prior failed repair entries within TTL for the same signature cause Abandon" wording overclaimed the live data shape. This ticket proves the live retry-suppression contract (`RecentlyFailed` skips the matching kind without globally suppressing later kinds) and separately proves `Abandon` for the no-preserved-prefix final local outcome. Any stricter all-kinds-repeat-failed surrender semantics belong to `tickets/S137PLACAULIN-012.md` if the Phase 11 gate diagnosis proves they are required.

## Architecture Check

1. **Test-only changes**: no production code modified; pure validation surface addition. The golden scenarios exercise existing tickets 001-009 at the strongest honest substrate boundary currently available.
2. **Mixed-layer verification per precision rule #5**: each scenario maps invariants to specific proof surfaces (decision trace for `RepairKind` selection rationale, action trace for plan-replacement ordering, event-log delta for `RepairApplied`/`ReplanTriggered` counter math, authoritative world state for blocker-cleared invariant).

## Verification Layers

1. `RebindTarget` rationale → decision-trace `RepairAttemptTrace.chosen_kind == Some(RebindTarget)` + `substitute_target` matches the sibling merchant entity.
2. `InsertVerification` short-circuit (without S139) → decision-trace `RepairAttemptTrace.rejected` contains `(InsertVerification, NoEpistemicSubstrate)`.
3. Retry suppression → `RepairMemory.repairs[signature]` with `succeeded == false` for one kind yields `(RepairKind, RepairFailure::RecentlyFailed)` for that kind while later lawful repair kinds still run.
4. `CommodityAvailabilityChanged` blocker clearing → authoritative state (`BlockerMemory::sweep_cleared`) plus `PlanRepairContext` using `DiscrepancyClearing::CommodityAvailabilityChanged`.
5. Budget exhaustion fall-through → direct `RepairOutcome::Failed` proof with `(RepairKind::RebindTarget, RepairFailure::BudgetExhausted)` plus append-only `ReplanTriggered` payload roundtrip.
6. `Abandon` final local outcome → direct `RepairOutcome::Repaired { kind: Abandon }` proof with empty `ProgressBarrier` plan when no prefix can be preserved.
7. Phase 11 gate counter math → deferred to `tickets/S137PLACAULIN-012.md` because the live `survival-baseline.ron` witness currently emits no `RepairApplied` events.

## What Changed

### 1. Created `crates/worldwake-ai/tests/golden_plan_repair.rs`

Six golden scenarios now cover the live S137 repair substrate. Each scenario uses a programmatic fixture at the strongest honest repair-search boundary; no authored RON fixture was added because the current `survival-baseline.ron` replay does not exercise localized repair.

The scenarios assert `RepairOutcome`, `RepairFailure`, `RepairAppliedPayload`, `ReplanTriggeredPayload`, and authoritative `BlockerMemory` behavior per the verification layers above.

Scenario list:

- **`golden_plan_repair::merchant_moved_breach_rebinds_to_sibling`** — exercises `RepairKind::RebindTarget`.
- **`golden_plan_repair::stale_belief_breach_attempts_insert_verification`** — exercises `InsertVerification` + short-circuit when S139 not landed (asserts `NoEpistemicSubstrate` failure entry).
- **`golden_plan_repair::recently_failed_repair_kind_is_skipped_without_global_suppression`** — exercises TTL-suppressed repair memory for the live one-entry-per-signature shape.
- **`golden_plan_repair::commodity_availability_changed_clears_blocker_structurally`** — exercises `DiscrepancyClearing` consumption.
- **`golden_plan_repair::repair_budget_exhaustion_falls_through_to_full_replan`** — exercises bounded budget + fall-through to `ReplanTriggered`.
- **`golden_plan_repair::abandon_returns_empty_progress_barrier_after_prior_strategies_fail`** — exercises `RepairKind::Abandon` plan shape.

### 2. Deferred the Phase 11 gate counter assertion

The counter assertion did not land in this ticket. The live replay currently emits `ReplanTriggered=82` and `RepairApplied=0`, so a 30% reduction assertion would be false. `tickets/S137PLACAULIN-012.md` owns diagnosing whether the gate needs a production-routing fix, a scenario witness change, or a metric/spec correction.

### 3. Verified existing survival golden binaries without modification

The existing `golden_survival_baseline`, `golden_survival_scattered`, and `golden_survival_contested` binaries pass without source modifications. Their long 1440-tick scenario tests remain `#[ignore]` under the existing golden-survival workflow, so this ticket records the non-ignored binary proof and does not claim the ignored cases ran.

## Files to Touch

- `crates/worldwake-ai/tests/golden_plan_repair.rs` (new)
- No new `scenarios/*.ron` fixtures; the proof seam is programmatic because the live authored survival baseline does not currently exercise localized repair.

## Out of Scope

- Production code changes — all production work landed in tickets 001-009 for the substrate proved here.
- Modifications to existing `golden_survival_*` tests — those are no-regression coverage and must remain unmodified.
- S139 integration — ticket asserts the `NoEpistemicSubstrate` short-circuit; S139's landing will allow scenario 2 to be re-asserted with successful `InsertVerification`.
- Phase 11 `survival-baseline.ron` replan-reduction gate — follow-up `tickets/S137PLACAULIN-012.md`.

## Acceptance Result

### Tests That Passed

1. `cargo test -p worldwake-ai --test golden_plan_repair` — all six substrate scenarios pass.
2. `cargo test -p worldwake-ai --test golden_survival_baseline` — non-ignored tests pass; existing long scenario cases remain ignored by workflow.
3. `cargo test -p worldwake-ai --test golden_survival_scattered` — non-ignored tests pass; existing long scenario cases remain ignored by workflow.
4. `cargo test -p worldwake-ai --test golden_survival_contested` — non-ignored tests pass; existing long scenario cases remain ignored by workflow.
5. `cargo test -p worldwake-ai` — affected crate regression.

### Invariants

1. Each golden scenario maps every claimed invariant to a single, named verification surface (no collapsing distinct layers into one trace assertion).
2. The Phase 11 gate is explicitly not claimed by this ticket; it is handed to `tickets/S137PLACAULIN-012.md`.
3. Repair is additive: existing `golden_survival_*` tests pass without modification.
4. Scenarios are deterministic — same seed produces same `RepairAttemptTrace` contents and same event-log counter values.

## Test Plan Result

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_plan_repair.rs` — new file containing the six live substrate scenarios.
2. No new RON fixtures; programmatic fixtures are the truthful proof seam for this validation slice.

### Commands Run

1. `cargo test -p worldwake-ai --test golden_plan_repair`
2. `cargo test -p worldwake-ai --test golden_survival_baseline`
3. `cargo test -p worldwake-ai --test golden_survival_scattered`
4. `cargo test -p worldwake-ai --test golden_survival_contested`
5. `cargo test -p worldwake-ai`
6. `python3 scripts/golden_inventory.py --write --check-docs`

## Outcome

Completed on 2026-05-13.

- Added `crates/worldwake-ai/tests/golden_plan_repair.rs` with scenarios 408-413 for the live S137 repair substrate.
- Regenerated generated golden docs, adding `docs/generated/golden-scenario-details/plan-repair.md` and updating the inventory, index, and coverage matrix.
- Truth-synced `specs/S137-plan-causal-links-and-repair.md` and `specs/IMPLEMENTATION-ORDER.md` to record that the Phase 11 survival-baseline counter gate remains open.
- Created follow-up `tickets/S137PLACAULIN-012.md` for the Phase 11 gate diagnosis and landing.

## Deviations

- The drafted Phase 11 counter assertion did not land. A live 1440-tick `survival-baseline.ron` replay observed during implementation emitted `ReplanTriggered=82` and `RepairApplied=0`, so the 30% reduction gate is not a truthful test-only assertion yet.
- The drafted repeat-failed `Abandon` scenario narrowed to the live `RepairMemory` shape. The golden now proves one-kind retry suppression and separately proves the `Abandon` final local outcome when no prefix can be preserved.
- No new `scenarios/*.ron` fixtures were added; programmatic fixtures are the strongest honest proof seam for the validation slice that landed.
- Deterministic replay companion tests were not added for scenarios 408-413 because each scenario is a small, deterministic programmatic repair-context fixture that directly asserts the repair substrate or event payload shape; replaying an authored RON scenario is the unresolved Phase 11 gate owned by `tickets/S137PLACAULIN-012.md`.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_plan_repair`
- Passed `cargo test -p worldwake-ai --test golden_survival_baseline` (non-ignored tests; existing long scenario cases remain ignored by workflow)
- Passed `cargo test -p worldwake-ai --test golden_survival_scattered` (non-ignored tests; existing long scenario cases remain ignored by workflow)
- Passed `cargo test -p worldwake-ai --test golden_survival_contested` (non-ignored tests; existing long scenario cases remain ignored by workflow)
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo test -p worldwake-ai`
