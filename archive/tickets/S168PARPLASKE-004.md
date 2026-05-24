# S168PARPLASKE-004: Validation goldens (reuse + fallback)

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: None — golden tests + replay equivalence checks in the existing `golden_ai` scenario module surface.
**Deps**: `archive/tickets/S168PARPLASKE-001.md` (`revalidate_skeleton_step`); `archive/tickets/S168PARPLASKE-002.md` (budget-exhausted populated `remaining_skeleton`); `archive/tickets/S168PARPLASKE-006.md` (information-barrier segment production); `archive/tickets/S168PARPLASKE-003.md` (`search_plan_seeded` + `PartialPlanResumeTrace`); `archive/specs/S168-partial-plan-skeleton-reuse.md` (Validation and Falsification section).

## Problem

S168's Validation and Falsification section (FND-31) declares four proof obligations the focused unit tests in tickets 001-003 do not satisfy:

1. **Golden (reuse)**: information-barrier suspend → lawful carrier satisfies barrier → skeleton revalidates → same pursuit resumes via seeded search; trace shows reuse.
2. **Golden (fallback)**: assumption goes stale before resume → reuse rejected → full replan; trace shows the invalidation reason.
3. **Replay/save-load equivalence**: both goldens replay identically; the budget-exhaustion save round-trips with the now-populated skeleton.
4. **No-regression**: survival/integration goldens unaffected (resume behavior is equivalent or strictly better-bounded).

The focused tests in 001-003 and 006 prove the function-level contracts: revalidation correctness, population correctness, information-barrier segment production, resume routing, trace emit, seeded-search internal fallback. They do not prove the end-to-end causal chain — "an agent that suspended at an information barrier and saw the barrier satisfied resumes its remembered pursuit via seeded search and the world reaches the same lawful state as full replan would." That requires golden E2E coverage.

This ticket delivers both goldens, the replay/save-load equivalence checks, and a no-regression survival sweep.

## Assumption Reassessment (2026-05-24)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. **Codebase shape (golden harness)**. Corrected during implementation:
   - Golden E2E tests live in the `crates/worldwake-ai/tests/golden_ai.rs` test target, with scenario modules under `crates/worldwake-ai/tests/scenarios/`. The drafted `tests/golden_ai/<scenario>.rs` and RON-scenario-file layout was stale.
   - `crates/worldwake-ai/tests/scenarios/partial_plan_terminals.rs` already owned the S149 partial-plan segment carrier goldens, so the S168 reuse/fallback/save-load coverage landed there as Scenarios 465-467.
   - Public decision-trace assertions use `AgentTickDriver::enable_tracing()` plus `DecisionTraceSink::traces_for(...)`, matching the `PartialPlanResumeTrace` emit point ticket 003 wired.
2. **Spec/doc references**. S168 Validation and Falsification (`archive/specs/S168-partial-plan-skeleton-reuse.md`). The reuse golden's narrative: "information-barrier suspend → lawful carrier satisfies barrier → skeleton revalidates → same pursuit resumes via seeded search with the trace showing revalidation." The fallback golden's narrative: "assumption goes stale before resume → reuse rejected → full replan, trace shows the invalidation reason."
3. **Cross-system boundary under audit (precision rule 2)**. The reuse golden spans: information-barrier suspension (agenda_manager), companion goal that brings the lawful carrier (existing S114/S149 machinery), barrier satisfaction (perception/belief layer), resume gate (`try_resume_partial_plan`), revalidation (ticket 001), seeded search (ticket 003), action execution. Each layer's contribution must be verifiable from the trace, not inferred from the final world state.
4. **Live `GoalKind` / operator surface under test (precision rule 13)**. The landed scenarios use `GoalKind::AcquireCommodity` on the existing partial-plan carrier harness, with explicit `PlannedSkeletonStep` values (`Sleep` for reusable seed retention, `Trade` with `SellerKnown(Bread)` for invalid fallback). This keeps the proof at the S168 resume/revalidation boundary instead of re-testing the full commodity-acquisition producer chain owned by earlier tickets.
5. **Scenario isolation (precision rule 8)**. The landed harness injects exactly one suspended information-barrier agenda entry and no rival agenda entries. The intended branch is resume via seeded-search preservation; competing autonomous acquisition, witness, ranking, and action-execution branches are intentionally absent because this ticket proves the downstream resume carrier/trace contract, not producer discovery.
6. **Replay/save-load equivalence (precision rule 14, FND-12)**. The save round-trip case the spec calls out: "the budget-exhaustion save round-trips with the now-populated skeleton." After ticket 002, the budget-exhausted segment carries `Some(_)` skeleton; the save/load surface must preserve that across a round-trip. The landed proof saves and reloads the enclosing golden harness simulation state plus `AgentDecisionRuntime`, confirms the suspended segment still carries the skeleton, then compares the original and reloaded next-tick resume traces.
7. **Negative cases (precision rule 12)**. Spec lists three:
   - **no skeleton-as-rail**: a populated skeleton step whose precondition no longer holds → reuse rejected, lawful replan; the trace records the invalidation reason. Provable via the fallback golden's trace assertion (the invalidation reason is named, not just "fallback fired").
   - **no world-truth read**: revalidation reads only belief view. Provable via ticket 001's focused unit test (mock that panics on world accessor); the golden itself doesn't need a separate assertion if the focused-test coverage proves the architectural property.
   - **no skeleton for combat/target-identity steps**: provable via ticket 002's filter tests; the golden doesn't need a separate assertion.
8. **No-regression scope**. The spec says "survival/integration goldens unaffected (resume behavior is equivalent or strictly better-bounded)." This is satisfied by running the existing survival/integration suite and confirming no failures, plus a focused check on any existing test that exercises `try_resume_partial_plan` (enumerated in ticket 003's Assumption Reassessment).
9. **Existing test coverage extended**. `partial_plan_terminals.rs` now proves the observable downstream contract in the live harness: reusable information-barrier skeleton trace + retained seed, invalid skeleton trace + seed clearing, and save/load preservation before resume. The exact tactical action lifecycle and full autonomous commodity branch remain covered by the mechanism-owned planner/search tests from archived tickets 001-003/006 plus the full `golden_ai` and `worldwake-ai` no-regression runs.
10. **Adjacent contradictions**. The only mismatch was harness layout and proof placement. No new follow-up was required.

## Architecture Check

1. **Goldens prove the cross-system causal chain that focused tests cannot.** Per FND-31, "passing a local golden end state is not evidence by itself" — but local focused tests are also insufficient when the contract spans information-barrier suspension → companion spawn → barrier satisfaction → resume → seeded search → action execution. The trace-assertion discipline (precision rule 6: prefer decision-trace assertions over weaker indirect evidence) anchors the proof at the strongest available causal layer.
2. **Replay/save-load equivalence is a hard FND-12 obligation.** A populated skeleton that survives save/load proves the change to the field's content respects the save-format contract (no version bump needed because the field already serialized; but the *content* round-trip must be verified).
3. **No-regression sweep guards against unintended ranking shifts.** The seeded path is a strict optimization in the spec's framing, but any change to the search-control bias could subtly affect plan ordering for adjacent scenarios. Running the existing survival/integration goldens catches that class of regression at the cross-system layer.

## Verified Layers

1. **Reuse causal chain** -> `golden_s168_information_barrier_resume_reuses_skeleton` asserts `PartialPlanResumeTrace { decision: ReusedSeededSearch, per_step_verdicts: [Reusable], seeded_ops: Some([Sleep]) }` and confirms the pending/committed agenda entry retains the skeleton for the seeded planning pass.
2. **Fallback causal chain** -> `golden_s168_information_barrier_resume_falls_back_when_skeleton_invalid` asserts `FallbackToReplanInvalid(BeliefUnknown)`, the invalid per-step verdict, no seeded ops, and cleared `remaining_skeleton` before the ordinary pending replan path continues.
3. **Save/load equivalence** -> `golden_s168_populated_skeleton_survives_save_load_before_resume` saves and reloads the enclosing harness state/runtime, confirms the populated skeleton is still present in the suspended runtime, and confirms the original and reloaded next-tick resume traces match.
4. **No-regression** -> `cargo test -p worldwake-ai --test golden_ai` and `cargo test -p worldwake-ai` both passed after adding the scenarios and regenerated docs.

## Landed Changes

1. Added three S168 golden scenario blocks and tests to `crates/worldwake-ai/tests/scenarios/partial_plan_terminals.rs`:
   - Scenario 465: reusable information-barrier skeleton emits `ReusedSeededSearch` and retains the seed.
   - Scenario 466: invalid information-barrier skeleton emits `FallbackToReplanInvalid(BeliefUnknown)` and clears the seed.
   - Scenario 467: populated skeleton survives simulation/runtime save-load before resume and emits an equivalent next-tick reuse trace after reload.
2. Regenerated the golden inventory, scenario index, coverage matrix, and `partial-plan-terminals` detail page.

## Landed Files

- `crates/worldwake-ai/tests/scenarios/partial_plan_terminals.rs` (modified)
- `docs/generated/golden-e2e-inventory.md` (regenerated)
- `docs/generated/golden-scenario-index.md` (regenerated)
- `docs/generated/golden-scenario-details/partial-plan-terminals.md` (regenerated)
- `docs/generated/golden-coverage-matrix.md` (regenerated)

## Out of Scope

- Budget-exhausted skeleton population — ticket 002.
- Information-barrier segment production — ticket 006.
- Revalidation function — ticket 001.
- Seeded search + resume integration + trace struct — ticket 003.
- Resource/jurisdiction/coordination barrier goldens — spec Non-Goals.
- Combat plan / target-identity-bound skeleton goldens — spec Non-Goals.
- Performance regression guards (the spec is an optimization but explicitly the *lowest-benefit* one; profiling is a stretch goal mentioned only in Risks). Not required for this ticket per S168 Risks section.

## Acceptance Result

1. Reuse trace, fallback trace, and save-load skeleton preservation are covered by the three landed `golden_s168_*` tests.
2. The full `golden_ai` target passed after the new scenario metadata and regenerated docs.
3. The full `worldwake-ai` crate suite passed.
4. Full pre-PR `scripts/verify.sh` is waived at per-ticket closeout because the `implement-spec-tickets` final branch phase owns that gate before push.

## Added Tests

1. `crates/worldwake-ai/tests/scenarios/partial_plan_terminals.rs::golden_s168_information_barrier_resume_reuses_skeleton`
2. `crates/worldwake-ai/tests/scenarios/partial_plan_terminals.rs::golden_s168_information_barrier_resume_falls_back_when_skeleton_invalid`
3. `crates/worldwake-ai/tests/scenarios/partial_plan_terminals.rs::golden_s168_populated_skeleton_survives_save_load_before_resume`

## Outcome

Completed on 2026-05-24.

- Landed S168 validation coverage in the live post-S154 golden harness by extending `partial_plan_terminals.rs`.
- Proved reusable information-barrier skeleton resume, invalid-skeleton fallback, and enclosing simulation/runtime save-load preservation.
- Regenerated golden inventory/index/detail/matrix docs for the three new scenario blocks.

## Deviations

- The drafted per-scenario Rust/RON file layout was stale. The truthful landed location is the existing `golden_ai.rs` target with tests under `crates/worldwake-ai/tests/scenarios/partial_plan_terminals.rs`.
- The landed reuse/fallback tests assert the agenda/decision-trace contract at the resume boundary rather than authoring full autonomous commodity-purchase RON scenarios. This matches the live public harness and keeps the proof at the strongest S168-owned seam while existing focused tests from tickets 001-003/006 continue to own tactical search and producer details.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_ai golden_s168_ -- --nocapture`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo test -p worldwake-ai --test golden_ai`
- Passed `cargo test -p worldwake-ai`
- Waived `scripts/verify.sh` for this ticket closeout because the harness final branch phase runs the full pre-PR gate before push.
