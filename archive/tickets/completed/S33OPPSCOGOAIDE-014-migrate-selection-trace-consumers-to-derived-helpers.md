# S33OPPSCOGOAIDE-014: Migrate selection-trace consumers to derived helper APIs

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` golden-test decision-trace consumer normalization
**Deps**: archive/tickets/S33OPPSCOGOAIDE-013-canonicalize-selection-trace-identity.md, archive/tickets/completed/S33OPPSCOGOAIDE-012-trace-test-query-surface.md

## Problem

The original ticket narrative is stale. `SelectionTrace` is already canonicalized on `selected_opportunity`, and the raw `selection.selected` field is already gone. The remaining inconsistency is smaller: representative golden tests still express exact selected-goal assertions through a mix of helper predicates and ad-hoc `selected_goal()` pattern matching.

That is no longer a data-model problem. It is a proof-surface consistency problem. For exact selected-goal assertions, the cleanest long-term surface is `SelectionTrace::selected_goal_is()`; for concrete branch identity, the cleanest surface is `selected_opportunity`; and for scenarios that truly need payload extraction from the selected goal, `selected_goal()` remains the right derived read.

## Assumption Reassessment (2026-03-28)

1. The exact shared abstraction boundary under audit is the public plan-selection decision-trace consumer surface in `crates/worldwake-ai/src/decision_trace.rs`: `SelectionTrace::selected_goal()`, `SelectionTrace::selected_goal_is()`, and `SelectionTrace::selected_opportunity_is()`, plus the representative golden tests that still mix helper predicates with ad-hoc `selected_goal()` matching.
2. Reassessment against the live codebase on 2026-03-28 confirms the original premise is outdated:
   - `SelectionTrace` already stores only `selected_opportunity`
   - `SelectionTrace::selected_goal()` already derives the desire-level view from that canonical opportunity path
   - `SelectionTrace::selected_goal_is()` already exists and is covered by focused tests in `crates/worldwake-ai/src/decision_trace.rs`
3. There are no remaining live `planning.selection.selected` consumers. A repo-wide search shows the remaining representative selected-goal assertions in the named golden files already go through derived helper APIs, but some still open-code exact-goal checks through `selected_goal().map(...)` or `selected_goal().is_some_and(...)`.
4. The same fact currently has two lawful consumption styles within the canonical helper surface:
   - exact selected-goal assertions expressed through `selected_goal_is(goal_key)`
   - exact selected-goal assertions expressed by matching over `selected_goal()`
   After this ticket, representative exact-goal assertions should prefer `selected_goal_is(goal_key)`. Scenarios that need to inspect the selected goal payload or capture a dynamic identifier may continue to use `selected_goal()`.
5. The exact goal families exercised by the remaining representative assertions are `TreatWounds`, `RestockCommodity`, `ReduceDanger`, `ConsumeOwnedCommodity`, `InvestigateViolation`, and `ClaimOffice`. The asserted invariant is still purely about the decision-trace proof surface, not planner behavior, ranking arithmetic, or action ordering.
6. This remains a single-layer AI/debugging-contract ticket. Action-trace ordering and authoritative world-state mutation are not the proof surface here except where the existing golden already uses them for its own scenario invariant.
7. Coverage gap classification:
   - focused helper coverage already exists in `decision_trace::tests::selected_goal_helper_derives_from_selected_opportunity`
   - the remaining gap is representative golden normalization onto the clearest helper predicate for exact goal identity
   - no new production helper is justified by the live codebase sweep
8. Scope correction: this is not a repo-wide migration and not a request for another helper layer such as `selected_goal_kind()`. Adding more goal-only convenience APIs here would create extra surface area without solving a real architectural gap.
9. Information-path correction: there is now only one stored transport path for selected branch identity (`selected_opportunity`). This ticket changes only the consumer read style over that canonical path.
10. No ranking-sensitive or ordering-sensitive claims are being changed. When a scenario asserts that a goal was selected, the proof remains on the decision-trace selection surface.

## Architecture Check

1. The current architecture is already the right one: `selected_opportunity` is the single stored truth, and desire-level answers are derived. That is cleaner than any alternative that restores a goal-only alias or stores the same fact twice.
2. The remaining beneficial cleanup is modest but real. When a test knows the exact `GoalKey` it expects, `selected_goal_is()` states that contract more clearly than re-matching on `selected_goal()`. That reduces local pattern noise without expanding the data model.
3. Adding another convenience API such as `selected_goal_kind()` would be less clean than the current architecture. It would create another goal-only consumer path with little extra value, while `selected_goal_is()` already handles exact-identity assertions and `selected_goal()` already handles the payload-extraction cases.
4. This aligns with `docs/FOUNDATIONS.md` Principle 25: derived views should remain small, intentional, and non-duplicative. It also aligns with Principle 26: no compatibility alias should be reintroduced for a field path that has already been removed.

## Verification Layers

1. Canonical selected-branch identity still derives correct desire-level answers -> focused `decision_trace` helper tests.
2. Exact selected-goal assertions remain readable and correct across representative goal families -> representative `golden_*` tests.
3. Concrete branch identity assertions remain on `selected_opportunity` where same-goal sibling distinction matters -> representative `golden_supply_chain` evidence/provenance assertions.
4. Additional action-trace or authoritative-world verification is not the contract of this ticket; those layers remain only where the pre-existing golden scenario already uses them.

## What to Change

### 1. Normalize representative exact-goal assertions onto the clearest helper predicate

- In the representative golden tests named below, replace exact selected-goal assertions that currently pattern-match over `selected_goal()` with `selected_goal_is(goal_key)` when the expected `GoalKey` is already known in the scenario.
- Keep `selected_goal()` in place where the test genuinely needs to extract dynamic payload from the selected goal, such as a generated `violation_id`.

### 2. Do not add new helper surface unless migration proves a real gap

- Reassessment did not find a missing production helper. Do not add another goal-only convenience API unless a concrete remaining assertion cannot be expressed cleanly with `selected_goal_is()`, `selected_goal()`, or `selected_opportunity`.
- Do not widen this into a broad trace-style rewrite, candidate helper work, or planner behavior change.

## Files to Touch

- `crates/worldwake-ai/tests/golden_care.rs` (modify)
- `crates/worldwake-ai/tests/golden_combat.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `crates/worldwake-ai/tests/golden_supply_chain.rs` (modify)
- `tickets/S33OPPSCOGOAIDE-014-migrate-selection-trace-consumers-to-derived-helpers.md` (modify, then archive)

## Out of Scope

- Changing selection behavior, ranking, suppression, or plan search
- Adding a new goal-only helper API without a demonstrated coverage gap
- Candidate-generation helper migrations unrelated to selected-goal assertions
- Save/load changes
- Repo-wide normalization of every trace assertion style

## Acceptance Criteria

### Tests That Must Pass

1. The representative migrated golden tests express exact selected-goal assertions through `selected_goal_is()` when the expected `GoalKey` is known.
2. Tests that need concrete branch identity still use `selected_opportunity` rather than reconstructing it from a goal-only helper.
3. The migrated tests still prove the same scenario invariants as before.
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`

### Invariants

1. Selected-goal consumer queries remain derived from canonical `selected_opportunity` trace state.
2. No new goal-only alias field or helper path is introduced.
3. Dynamic-id scenarios may continue to read `selected_goal()` directly when that is the strongest available surface.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_care.rs` — normalize remote-care exact selected-goal assertions to `selected_goal_is()`.
   Rationale: proves a care/travel scenario can assert exact selected-goal identity without re-matching on the derived goal payload.
2. `crates/worldwake-ai/tests/golden_supply_chain.rs` — normalize exact restock-goal assertions while preserving `selected_opportunity` checks for branch-specific evidence provenance.
   Rationale: proves the architecture keeps exact desire-level assertions and concrete opportunity assertions on separate, explicit surfaces.
3. `crates/worldwake-ai/tests/golden_combat.rs` — normalize exact self-care and `ReduceDanger` selections to `selected_goal_is()`.
   Rationale: proves the helper predicate is sufficient across survival and combat-pressure scenarios.
4. `crates/worldwake-ai/tests/golden_emergent.rs` — normalize exact political/investigation/self-care selections where the expected `GoalKey` is known, while leaving dynamic-id extraction on `selected_goal()`.
   Rationale: proves the current helper surface is already strong enough without adding another API.

### Commands

1. `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker`
2. `cargo test -p worldwake-ai golden_stale_prerequisite_belief_discovery_replan`
3. `cargo test -p worldwake-ai golden_same_place_concurrent_violations_stay_distinct`
4. `cargo test -p worldwake-ai golden_traceability_explains_stale_fine_branch_without_source_diving`
5. `cargo test -p worldwake-ai golden_reduce_danger_defensive_mitigation`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-28
- What actually changed:
  - Reassessed the ticket against the live code and corrected its stale premise: `SelectionTrace` was already canonicalized on `selected_opportunity`, and there were no remaining `selection.selected` consumers to migrate.
  - Kept the architecture as-is rather than adding another helper layer, because the current split is already the clean design: `selected_goal_is()` for exact `GoalKey` assertions, `selected_opportunity` for concrete branch identity, and `selected_goal()` only where tests need to read dynamic payload.
  - Normalized representative golden assertions in `golden_care.rs`, `golden_supply_chain.rs`, `golden_combat.rs`, and `golden_emergent.rs` onto `selected_goal_is()` where the expected goal identity was already known.
- Deviations from original plan:
  - No production code or new helper API was added. Reassessment showed the core architecture from S33OPPSCOGOAIDE-012 and -013 was already delivered and cleaner than any new goal-only convenience surface.
  - Scope narrowed from a supposed field-removal migration to a small representative golden-test normalization pass plus ticket correction.
- Verification results:
  - `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker`
  - `cargo test -p worldwake-ai golden_stale_prerequisite_belief_discovery_replan`
  - `cargo test -p worldwake-ai golden_same_place_concurrent_violations_stay_distinct`
  - `cargo test -p worldwake-ai golden_traceability_explains_stale_fine_branch_without_source_diving`
  - `cargo test -p worldwake-ai golden_reduce_danger_defensive_mitigation`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
