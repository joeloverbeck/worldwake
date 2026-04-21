# S114PLASTGUA-010: Golden scenario — merchant departs, purchase plan invalidates via guard

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — golden test authoring only.
**Deps**: `archive/tickets/S114PLASTGUA-001.md`, `tickets/S114PLASTGUA-002.md`, `tickets/S114PLASTGUA-003.md`, `tickets/S114PLASTGUA-004.md`, `tickets/S114PLASTGUA-005.md`, `tickets/S114PLASTGUA-006.md`, `tickets/S114PLASTGUA-007.md`, `tickets/S114PLASTGUA-008.md`, `tickets/S114PLASTGUA-009.md`

## Problem

S114 spec validation test 12 is the integration proof that guard + expectation infrastructure actually changes agent behavior correctly. It also is the acceptance gate for F1's resolution (AI-side tick step correctness under a live tick pipeline, not just per-unit coverage). Tests 9 and 10 confirm no regression in existing target-gone and survival paths.

## Assumption Reassessment (2026-04-21)

1. The `trade` action's `guard_template` is populated in ticket 006 with `RequiredFactSpec::TargetPresent` + `InvalidatorSpec::TargetMoved` + `InvalidatorSpec::BeliefStatusChange`. That is the only action registration exercised by this test's golden scenario; other action registrations retain `guard_template: None`.
2. Live `GoalKind` under test: whatever the planner routes a "buy commodity X" intent through — likely `AcquireCommodity` or `Trade` family (confirm via `rg 'GoalKind::' crates/worldwake-ai/src/candidate_generation.rs` at implementation time). The scenario authors an agent with a need that drives this goal path; the exact GoalKind selection is emergent from needs + affordances, not forced.
3. S113 identity-bound target-location envelope (accessor `believed_target_location`) exists (S113 landed). Test 12 in S114 spec says it piggybacks on S113 test 12 — verify S113's test name at implementation time via `rg -l 'golden.*target_location' crates/worldwake-ai/tests`.
4. Survival goldens cited in spec test 10:
   - `golden_survival_baseline` — verify exists via `rg -l 'fn golden_survival_baseline' crates/worldwake-ai/tests`
   - `golden_survival_scattered` — same
   - `golden_survival_contested` — same
   All three must stay green — running `cargo test -p worldwake-ai golden_survival` is the gate.
5. Test 9 in the spec (existing target-gone golden) claims that *with* guards, the `BeliefContradicted` path is taken instead of `AssumptionFailed`. Identify the existing target-gone golden test at implementation time via `rg -l 'AssumptionFailed' crates/worldwake-ai/tests` — the scenario likely already exists but its post-condition assertions must be updated to match the new causal pathway.
6. Shared boundary under audit: the golden-test narrative contract. The scenario must prove end-to-end: guard breach → classify_revalidation returns Invalidated → action-start route-to-failure → event-log emission → discrepancy record → agent replans. This spans guard-check (ticket 007), plan-adoption records (ticket 008), overdue emission path (ticket 009) — all three must integrate correctly under tick pipeline.
7. Scenario isolation choice: the scenario isolates the guard-breach branch by removing lawful competing affordances. The intended branch is "buyer attempts trade with merchant → guard fires `TargetMoved` → replan." Lawful competing affordances excluded from setup: no alternative merchants at the arrival place (so the planner cannot sidestep the scenario by swapping counterparties); no high-priority needs that would override the trade goal mid-execution.
8. Ordering contract: this test relies on authoritative world-state ordering (merchant moves before buyer arrives), action lifecycle ordering (buyer's `Trade` action starts at arrival tick, revalidation runs at start), and decision-trace ordering (post-breach replan is the next goal decision). All three are distinct proof surfaces per `docs/golden-e2e-testing.md`.

## Architecture Check

1. New golden test file isolates the scenario without polluting existing golden suites. Naming pattern `golden_plan_step_guards_*.rs` matches repo conventions for new S-level integration coverage.
2. The test does **not** construct `PlannedStep` or `ExpectationRecord` literals directly — it exercises the full tick pipeline through scenario authoring, ensuring reviewers see emergent behavior, not pre-cooked fixtures.
3. Per `docs/precision-rules.md` Rule 2 (Layer Precision), every assertion cites the exact symbol it verifies: `classify_revalidation` return value → `PlanInvalidationReason::ExpectationMismatch`; event log → `DecisionEventPayload::ExpectationMismatch` with populated `expectation_kind` + `mismatch_detail`; `DiscrepancyMemory` record → `Discrepancy::BeliefContradicted`.

## Verification Layers

1. **Guard-breach classification** — `classify_revalidation` returns `Invalidated { reason: PlanInvalidationReason::ExpectationMismatch { step_index } }` → decision trace on the arrival tick.
2. **Action-start revalidation** — `BestEffort` start aborts the step without entering the action handler → action trace on the arrival tick shows start-abort, not commit.
3. **Event-log emission** — the event log contains exactly one `DecisionEventPayload::ExpectationMismatch` for this step with `expectation_kind: Some(ExpectationKindTag::State)` and `mismatch_detail: Some(GuardInvalidator(InvalidatorTag::TargetMoved))` → event-log delta assertion.
4. **Discrepancy classification** — `DiscrepancyMemory` gains a `Discrepancy::BeliefContradicted` record → authoritative world state post-tick.
5. **Replan timing** — within 2 ticks after the breach, the agent's `current_plan` references a new `goal` — decision trace showing fresh candidate generation and plan adoption.
6. **Existing target-gone golden regression** — where prior code took `BlockingFact::AssumptionFailed`, the new code takes `Discrepancy::BeliefContradicted`. Update the existing test's post-condition assertion to the new causal pathway (test 9 per spec).
7. **Survival stability** — `golden_survival_baseline`, `golden_survival_scattered`, `golden_survival_contested` final tick counts + survival rates unchanged within their existing tolerances (test 10 per spec).

## What to Change

### 1. New golden test file

`crates/worldwake-ai/tests/golden_plan_step_guards.rs` (new):

Scenario outline:
- Topology: two places, `marketplace` and `buyer_home`, connected by a route. `buyer` starts at `buyer_home`, `merchant` at `marketplace` with a commodity lot.
- `buyer` has a consumption need that drives an `AcquireCommodity` goal for whatever the merchant sells. Per scenario isolation (Assumption 7), no alternative merchants are present.
- Buyer plans a `Trade` step at `marketplace` (from ticket 006, this step has a populated `guard_template`).
- Buyer begins `TravelTo(marketplace)`. Mid-travel: `merchant` is moved (via scenario scripted move or explicit handler call) from `marketplace` to a different place.
- Buyer arrives at `marketplace` on tick T. Per S113, the buyer's `believed_target_location` for `merchant` is still `marketplace` (belief is stale) at tick T, but on the start path for the Trade step, `check_guard` reads the current envelope and detects the divergence → `TargetMoved` invalidator fires → revalidation returns `Invalidated`.
- Assertions at tick T:
  - Decision trace contains `classify_revalidation → Invalidated { reason: ExpectationMismatch { step_index: 1 } }` (or whatever index the trade step has).
  - Action trace shows `BestEffort(Trade)` starts and aborts on the same tick.
  - Event log contains one new `ExpectationMismatch` event with `expectation_kind: Some(ExpectationKindTag::State)` and `mismatch_detail: Some(MismatchDetail::GuardInvalidator(InvalidatorTag::TargetMoved))`.
  - `DiscrepancyMemory` for buyer contains a `Discrepancy::BeliefContradicted` record.
- Assertion at tick T+1 or T+2 (within 2 ticks per spec test 12): buyer's `current_plan.goal` differs from its pre-breach goal — a replan has occurred.

### 2. Update existing target-gone golden (test 9 per spec)

Identify via `rg -l 'AssumptionFailed' crates/worldwake-ai/tests` at implementation time. Update the post-breach assertion from the `BlockingFact::AssumptionFailed` pathway to the `Discrepancy::BeliefContradicted` pathway now that guards deliver the richer classification.

### 3. Confirm survival goldens (test 10 per spec)

Run `cargo test -p worldwake-ai golden_survival` after all S114 tickets are merged. No code changes expected — this step is pure regression verification.

## Files to Touch

- `crates/worldwake-ai/tests/golden_plan_step_guards.rs` (new)
- Existing target-gone golden test file — identify at implementation time (modify — update post-condition assertion)

## Out of Scope

- Any production code changes — all S114 production code lands in tickets 001-009.
- New action registrations beyond `trade` — other actions' guard_templates are future work.
- Performance/determinism tuning — replan timing assertion (within 2 ticks) is tested, but no ranking-heuristic changes.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_plan_step_guards_merchant_departs_before_arrival` (new) — full end-to-end assertions per Verification Layers 1-5.
2. Existing target-gone golden test (identified at implementation time) — updated post-condition assertion passes.
3. `golden_survival_baseline`, `golden_survival_scattered`, `golden_survival_contested` — stay green with no tolerance change.
4. `golden_planner_pathology` and `golden_portfolio_planning` — stay green (pre-S114 pathway unaffected; these actions have `guard_template: None`).
5. Full golden suite: `cargo test -p worldwake-ai --tests` with no regressions.

### Invariants

1. The scenario's assertions are derived from independent proof surfaces — decision trace, action trace, event log, authoritative world state — not collapsed into one narrative assertion (per `docs/precision-rules.md` Rule 5).
2. The scenario-isolation choice (no alternative merchants; no override-priority needs) is documented inline in the test file, naming the lawful competing affordances excluded and why.
3. The replan-within-2-ticks assertion uses `(tick, sequence_in_tick)` ordering for comparison where applicable, not narrative "next tick" language (per Rule 14).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_plan_step_guards.rs` (new) — primary golden test per scenario outline above.
2. Existing target-gone golden test (identify at implementation time) — updated post-condition.

### Commands

1. `cargo test -p worldwake-ai golden_plan_step_guards`
2. `cargo test -p worldwake-ai golden_survival` (regression gate)
3. `cargo test -p worldwake-ai` (full AI-crate suite; includes target-gone + portfolio + pathology goldens)
4. `scripts/verify.sh` (pre-PR gate — matches CI invariants exactly)
