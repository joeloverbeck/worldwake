# S114PLASTGUA-010: Golden scenario — merchant departs, purchase plan invalidates via guard

**Status**: REJECTED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — golden test authoring only.
**Deps**: `archive/tickets/S114PLASTGUA-001.md`, `archive/tickets/S114PLASTGUA-002.md`, `archive/tickets/S114PLASTGUA-003.md`, `archive/tickets/S114PLASTGUA-004.md`, `archive/tickets/S114PLASTGUA-005.md`, `archive/tickets/S114PLASTGUA-006.md`, `archive/tickets/S114PLASTGUA-007.md`, `archive/tickets/S114PLASTGUA-008.md`, `archive/tickets/S114PLASTGUA-009.md`, `archive/tickets/S114PLASTGUA-011.md`

## Problem

S114 spec validation test 12 was drafted as an end-to-end golden proving that a buyer plans `Travel -> Trade`, the merchant departs mid-travel, and the trade step is invalidated by the new guard path. Live reassessment disproved that autonomous golden premise on the current branch: remote merchant stock does not carry enough planner-visible custody/detail to keep displayed sale stock distinct from loose cargo, so the buyer does not truthfully hold the authored remote `trade` branch. The missing work is an engine-side remote belief/candidate-generation fix, not just a missing golden.

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
6. Shared boundary under audit: the golden-test narrative contract. The scenario must prove end-to-end: guard breach → classify_revalidation returns Invalidated → action-start route-to-failure → event-log emission → discrepancy record → agent replans. This now spans guard-check + replan preservation (ticket 007), plan-adoption records (ticket 008), overdue emission path (ticket 009), and the separate guard-breach start-failure emission path (ticket 011).
7. Scenario isolation choice: the scenario isolates the guard-breach branch by removing lawful competing affordances. The intended branch is "buyer attempts trade with merchant → guard fires `TargetMoved` → replan." Lawful competing affordances excluded from setup: no alternative merchants at the arrival place (so the planner cannot sidestep the scenario by swapping counterparties); no high-priority needs that would override the trade goal mid-execution.
8. Ordering contract: this test relies on authoritative world-state ordering (merchant moves before buyer arrives), action lifecycle ordering (buyer's `Trade` action starts at arrival tick, revalidation runs at start), and decision-trace ordering (post-breach replan is the next goal decision). All three are distinct proof surfaces per `docs/golden-e2e-testing.md`.

## Assumption Reassessment (2026-04-22)

1. Ticket says a new dedicated file `crates/worldwake-ai/tests/golden_plan_step_guards.rs` should own this scenario. Live golden ownership already has same-domain merchant/trade scaffolding in `crates/worldwake-ai/tests/golden_merchant_selling.rs`, including seller-departure setup and trade-specific helpers. Correction applied: keep the golden in `golden_merchant_selling.rs` instead of creating a duplicate suite.
2. Ticket says an existing target-gone golden should be updated by searching `crates/worldwake-ai/tests`. Live code has no such golden test; the only `FrameClearReason::AssumptionFailed` golden helper is `crates/worldwake-ai/tests/golden_harness/commodity_assumption_falsification.rs`, which is not a merchant/trade target-gone golden and is owned by a different contract. Correction applied: remove that nonexistent golden-update deliverable from this ticket.
3. Ticket says the golden should prove an action-trace start-abort on the breach tick. Live `S114PLASTGUA-011` changed the guard-breach path to emit `ExpectationMismatch` and clear the plan before enqueue in the AI execution seam; the focused proof there explicitly asserts no queued request or active action on the breach tick. Correction applied: the honest golden boundary is decision-trace selected trade branch before departure, then event-log/discrepancy/replan aftermath after the pre-enqueue guard breach. No action-trace start-abort claim remains.
4. Live `trade` registration in `crates/worldwake-systems/src/trade_actions.rs` already carries `guard_template: Some(GuardTemplateSpec { required_facts: [TargetPresent], invalidators: [TargetMoved, BeliefStatusChange], ... })`, and focused proofs for guard classification/emission already landed in `archive/tickets/S114PLASTGUA-007.md` and `archive/tickets/S114PLASTGUA-011.md`. This ticket remains golden-only and should reuse those lower-layer proofs rather than reopening production ownership.
5. The strongest honest golden seam is hybrid, per `docs/golden-e2e-testing.md`: prove the AI selected a travel-then-trade plan against the merchant via decision trace before the move, then move the merchant during travel, and finally prove `ExpectationMismatch` event emission, `Discrepancy::BeliefContradicted`, and replan fallout after arrival. This keeps the golden aligned with the live pre-enqueue rejection architecture instead of chasing an impossible post-enqueue trade-start trace.
6. Survival regressions named in the ticket still exist and remain valid broad proof (`golden_survival_baseline`, `golden_survival_scattered`, `golden_survival_contested`), but `golden_planner_pathology` and `golden_portfolio_planning` are same-crate unaffected regression gates rather than focused owned surfaces. Keep them in broadened verification only.
7. Focused implementation attempt disproved the remaining golden premise. With existing merchant-selling helpers plus remote belief seeding, the buyer selected `AcquireCommodity(SelfConsume)` via `Travel -> MoveCargo`, not `Travel -> Trade`. Tightening the setup to remove the cargo-like branch then produced no remote acquisition candidate at all. The exact live evidence is the failing decision-trace summaries from the attempted focused golden run on 2026-04-22.
8. Root cause from that failed reproduction: remote `BelievedEntityState` does not preserve custody/container/display structure for item lots, while `candidate_generation.rs` treats remote lots with unknown container/possessor as loose cargo. That engine remainder is now explicitly owned by follow-up ticket `tickets/S114PLASTGUA-013.md`.

## Architecture Check

1. The clean outcome is to reject this ticket as written and move the remaining work to an engine follow-up. Forcing a golden around the current `MoveCargo` or no-candidate behavior would publish a false contract instead of the intended S114 trade-guard branch.
2. Focused lower-layer proofs from tickets 007, 009, and 011 remain the truthful coverage for guard classification, mismatch emission, and discrepancy recording on the live branch.

## Verification Layers

1. **Focused rejection evidence** — attempted golden reproduction at the real merchant-selling harness boundary proved the selected remote branch is `Travel -> MoveCargo` or no candidate, not `Travel -> Trade`.
2. **Lower-layer S114 coverage already landed** — guard classification/emission/discrepancy behavior remains proved by the focused tests from tickets 007, 009, and 011.
3. **Remaining work owner** — follow-up ticket `tickets/S114PLASTGUA-013.md` now owns the engine-side remote belief/candidate-generation gap required before any truthful golden can prove the authored scenario.

## What to Change

### 1. Record the rejection and hand off the remaining engine gap

- Keep this ticket as the rejection record for the disproven golden premise.
- Link the remaining engine work to `tickets/S114PLASTGUA-013.md`.
- Do not land a golden that claims the remote trade guard path until that follow-up restores a truthful `Travel -> Trade` planner branch.

## Files to Touch

- `tickets/S114PLASTGUA-010.md` (modify — rejection record and corrected ownership)
- `tickets/S114PLASTGUA-013.md` (new — engine follow-up for remote sale-stock belief/candidate routing)

## Out of Scope

- Any production code changes — the live production seam already landed in tickets 007/009/011.
- New action registrations beyond `trade` — other actions' guard_templates are future work.
- Performance/determinism tuning — replan timing assertion (within 2 ticks) is tested, but no ranking-heuristic changes.

## Acceptance Criteria

### Tests That Must Pass

1. Rejection evidence is recorded factually in this ticket.
2. Follow-up ticket `tickets/S114PLASTGUA-013.md` exists and owns the remaining engine work.

### Invariants

1. This ticket does not overstate a golden contract the live branch cannot satisfy.
2. The remaining work is tracked as engine work, not hidden inside a weakened golden.

## Test Plan

### New/Modified Tests

1. None — reassessment-only rejection record. The failed exploratory golden was not kept.

### Commands

1. `cargo test -p worldwake-ai --test golden_merchant_selling seller_departure_invalidates_trade_plan_via_guard_breach -- --exact --nocapture` (failed during reassessment; remote branch selected `Travel -> MoveCargo` or no candidate instead of `Travel -> Trade`)
2. `cargo test -p worldwake-ai --test golden_merchant_selling -- --list` (passed; compile/list sanity check after backing out the exploratory golden)

## Outcome

Rejected on 2026-04-22.

- Reassessment proved that the drafted autonomous golden premise is false on the live branch. The remote merchant setup does not truthfully reach `Travel -> Trade`; with existing merchant-selling helpers it selects `Travel -> MoveCargo`, and when that cargo-like path is removed it emits no remote acquisition candidate.
- No code or golden was landed from the exploratory reproduction.
- Created follow-up ticket `tickets/S114PLASTGUA-013.md` to own the engine-side remote belief/candidate-generation gap required before this golden can be re-authored honestly.

## Deviations

- The ticket was initially narrowed to the existing merchant-selling suite, but focused reproduction then disproved the remaining golden premise entirely. Per repo rules, the ticket is closed as a rejection record rather than forcing a misleading test.

## Verification Result

- Failed as expected during reassessment: `cargo test -p worldwake-ai --test golden_merchant_selling seller_departure_invalidates_trade_plan_via_guard_breach -- --exact --nocapture`
- Passed compile/list sanity check after removing the exploratory golden: `cargo test -p worldwake-ai --test golden_merchant_selling -- --list`
