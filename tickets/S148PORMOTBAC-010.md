# S148PORMOTBAC-010: Golden coverage for five-slot portfolio and resume/abandon lifecycle

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: No — test-only ticket; adds new golden suite `crates/worldwake-ai/tests/golden_portfolio_five_slots.rs` covering 8 scenarios per spec D14; audits the existing `crates/worldwake-ai/tests/golden_portfolio_planning.rs` suite after the S148PORMOTBAC-001 enum rename and migrates any remaining expectations to the five-slot taxonomy where appropriate
**Deps**: `archive/tickets/S148PORMOTBAC-001.md`, `archive/tickets/S148PORMOTBAC-002.md`, `archive/tickets/S148PORMOTBAC-003.md`, `archive/tickets/S148PORMOTBAC-004.md`, `archive/tickets/S148PORMOTBAC-005.md`, `archive/tickets/S148PORMOTBAC-006.md`, `archive/tickets/S148PORMOTBAC-007.md`, `archive/tickets/S148PORMOTBAC-008.md`, `archive/tickets/S148PORMOTBAC-009.md`, `specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

S148 expands the portfolio from three to five slots, introduces operating-mode-modulated weight degradation, adds typed resume/abandon condition lifecycle predicates on `IntentionFrame`, and emits `Discrepancy::AbandonConditionFired` when abandonment fires. Without golden coverage, regressions in any of these new surfaces would land silently. Spec D14 names 8 scenarios that prove the contract end-to-end through the real agent decision cycle (candidate generation → ranking → slot assembly → planning → execution); a missing golden suite leaves the new mechanism unverified at the E2E layer. Additionally, the existing `golden_portfolio_planning.rs` suite must be audited after the minimal enum-name fallout handled by `archive/tickets/S148PORMOTBAC-001.md` so it continues to exercise the slot-assembly contract under the five-slot taxonomy where appropriate.

## Assumption Reassessment (2026-05-17)

1. Existing golden suite at `crates/worldwake-ai/tests/golden_portfolio_planning.rs` contains 6 tests on the three-slot model (per portfolio.rs `#[cfg(test)]` block enumeration during reassessment): `survival_slot_picks_highest_motive_survival`, `commitment_slot_picks_committed_opportunity`, `commitment_slot_falls_back_to_highest_obligation`, `self_consume_acquire_populates_survival_slot`, `plausible_slots_by_score_applies_weights`, `survival_slot_prefers_higher_priority_class_over_higher_motive` (test names paraphrased; full list confirmed during ticket reassessment).
2. Spec S148 D14 specifies 8 new scenarios:
   - (a) All five slots populated under `OperatingMode::Normal` (each slot receives a winner derived from the corresponding motive class).
   - (b) `OperatingMode::Emergency` zeroes `EconomicOpportunity` and `SocialMotive`; `NeedSurvival`, `PainCare`, `ObligationDuty` continue to populate.
   - (c) `OperatingMode::Idle` populates all five slots when low-priority candidates exist.
   - (d) `NeedSurvival` winner is planned first under priority-class ordering.
   - (e) `IntentionFrame.motive_refs` matches the committed goal's `motive_source_contributions`.
   - (f) `explicit_claims` invalidate on `ArtifactExistence::Destroyed` and on each non-`Active` `ArtifactLegalEffect` transition (Suspended, Expired, Revoked, Fulfilled).
   - (g) `resume_conditions` resume a suspended intention on `OpportunityVisible`, `LocationReached`, and `BeliefStatusChanged`.
   - (h) `abandon_conditions` cause `Exhausted` transition on `MotiveSourceLost`, `OpportunityForeverGone`, `PatienceExhausted`, `ArtifactDestroyed`, and `ArtifactLegalEffectLost`.
   - Additional scenario: `causal_links` cap enforcement (when 1+ events beyond `causal_links_per_step_cap` are pushed, oldest is evicted).
3. Shared abstraction under audit: the E2E golden harness. Scenario constructors use the standard golden-harness fixture pattern at `crates/worldwake-ai/tests/golden_harness/` and reference `docs/golden-e2e-testing.md` for proof-surface choice. Per the precision-rules guidance (Rule 5 — Verification Surface Mapping), each golden's invariant maps to the strongest available proof surface: candidate selection / slot assembly → `decision trace`; action lifecycle ordering → `action trace`; authoritative mutation ordering → `event-log delta and/or authoritative world state`.
4. Per the spec's D13 Authoritative-to-AI Impact Analysis: candidate generation (slot assembly) and `handle_plan_failure` (abandon-condition replan) are flagged. The goldens must cover both: scenario (a)/(b)/(c) verify slot assembly; scenario (h)'s `abandon_conditions` cases verify that a fired `AbandonConditionFired` discrepancy routes through `handle_plan_failure` and produces a replan with the abandoned intention's motive correctly removed from the contributing set.
5. Test harness boundary: golden coverage requires full action registries (not local needs-only harness) because the scenarios exercise multi-system action lifecycles (acquire, travel, contention claims, social artifacts). Use the existing full-registry helpers per `crates/worldwake-ai/tests/golden_harness/` precedent.

## Architecture Check

1. The golden suite proves the full agent-decision-cycle contract end-to-end — candidate generation through authoritative outcome — under the real `step_tick` pipeline. Per precision-rule 6 (Decision-Trace Preference), AI reasoning assertions prefer decision-trace surfaces over indirect event-log absence. Each scenario's invariant maps to the strongest available lower-layer proof surface.
2. The migration of the existing 6-test golden suite preserves the original scenarios' intent — what changes is variant naming (Survival → NeedSurvival, Commitment → ObligationDuty, Economic → EconomicOpportunity), the weight read source (now `PortfolioWeightsProfile` via belief view), and where appropriate the addition of the new `PainCare`/`SocialMotive` cases to existing test fixtures.
3. Scenario isolation (precision-rule 8): each new golden documents the lawful competing affordances the test setup intentionally excludes (e.g., the all-five-slots scenario explicitly populates motives across each discriminant class so no slot is empty due to fixture omission rather than logic gap).

## Verification Layers

1. Slot assembly produces correct per-slot winner → **decision trace** asserting per-slot composition matches the expected motive-class mapping
2. Operating-mode weight degradation → **decision trace** asserting `EconomicOpportunity` and `SocialMotive` slots are absent (weight zeroed) under Emergency mode and present under Normal/Idle
3. `IntentionFrame.motive_refs` matches `AgendaEntry.motive_source_contributions` at commit time → **decision trace** + **authoritative world state** (the IntentionFrame is authoritative state attached to the agent)
4. `explicit_claims` invalidation on lifecycle transition → **event-log delta** showing the lifecycle transition + **decision trace** showing the subsequent `AbandonConditionFired` emission
5. `resume_conditions` resume on belief/visibility/location change → **decision trace** showing the resume decision + **action trace** showing the resumed action's start
6. `abandon_conditions` produce `Exhausted` transition → **decision trace** showing the abandon decision + **event-log delta** showing the resulting `Discrepancy::AbandonConditionFired` payload
7. `causal_links` cap enforcement → **focused unit test** (already covered by ticket 007) supplemented by an **action trace** assertion in a golden that pushes >cap events

## What to Change

### 1. Create `golden_portfolio_five_slots.rs`

`crates/worldwake-ai/tests/golden_portfolio_five_slots.rs` (new): 8 golden tests covering scenarios (a)-(h) from Assumption Reassessment item 2. Each test:

- Constructs a fixture scenario with controlled motive sources, agents, places, and artifacts.
- Steps the simulation through enough ticks for the scenario's invariant to manifest.
- Asserts the invariant via the proof surface named in Verification Layers (above).

Use the existing golden-harness scaffolding under `crates/worldwake-ai/tests/golden_harness/` per the established patterns in sibling golden files.

Plus one additional test: `causal_links_cap_evicts_oldest_in_fifo_order_during_resume_abandon_cycle` exercising the cap enforcement through a multi-event abandon-and-resume cycle.

### 2. Audit and migrate `golden_portfolio_planning.rs`

For each of the 6 existing tests:

- **Audit variant references** after `archive/tickets/S148PORMOTBAC-001.md`'s migration (`Survival` → `NeedSurvival`, `Commitment` → `ObligationDuty`, `Economic` → `EconomicOpportunity`) and update any remaining golden assertions that still encode the three-slot taxonomy.
- **Update weight reads** — tests that construct explicit weight fixtures use the new `PortfolioWeightsProfile` shape (5 weights + 3 plan caps) per ticket 002.
- **Update planning cap reads** — tests that previously asserted on `max_candidates_to_plan == 2` re-assert on the new default `max_plans_normal == 5` (or pin a fixture `max_plans_normal` value matching the original test's intent) per ticket 008.
- **Preserve test intent** — the original scenarios are not invalidated by the migration; the test name and assertion structure stay aligned with the original invariant.

Decision per-test (made during implementation; default is in-place rename):

| Existing test | Likely action |
|---|---|
| `survival_slot_picks_highest_motive_survival` | Rename to `need_survival_slot_picks_highest_motive_need_pressure`; semantics preserved |
| `commitment_slot_picks_committed_opportunity` | Rename to `obligation_duty_slot_picks_committed_opportunity`; semantics preserved |
| `commitment_slot_falls_back_to_highest_obligation` | Rename to `obligation_duty_slot_falls_back_to_highest_obligation`; semantics preserved |
| `self_consume_acquire_populates_survival_slot` | Rename to `self_consume_acquire_populates_need_survival_slot`; semantics preserved |
| `plausible_slots_by_score_applies_weights` | Keep name (mechanism name unchanged); update fixture to new 5-weight `PortfolioWeightsProfile` |
| `survival_slot_prefers_higher_priority_class_over_higher_motive` | Rename to `need_survival_slot_prefers_higher_priority_class_over_higher_motive`; semantics preserved |

## Files to Touch

- `crates/worldwake-ai/tests/golden_portfolio_five_slots.rs` (new)
- `crates/worldwake-ai/tests/golden_portfolio_planning.rs` (modify — variant rename, fixture migration, assertion update)
- Likely: shared golden-harness helpers under `crates/worldwake-ai/tests/golden_harness/` if scenario construction needs new factory functions for five-slot or condition-driven fixtures (verify during implementation; prefer extending existing helpers over creating new modules)

## Out of Scope

- Modifications to engine code — this ticket is test-only; all production behavior is established by tickets 001-008
- Regenerating `docs/generated/golden-e2e-inventory.md` and sibling docs (run `python3 scripts/golden_inventory.py --write --check-docs` as part of pre-PR verification per `tickets/README.md` — but the doc updates are a mechanical regen, not new content)
- Performance regression guards — S148 is not a performance-optimization spec; the Performance-optimization-specs guideline in the spec-to-tickets skill does not apply
- New observer rendering or trace surface additions (ticket 009 already covers observer-side rendering of the new state)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_portfolio_five_slots` — all 9 new goldens pass (8 D14 scenarios + 1 causal_links cap)
2. `cargo test -p worldwake-ai --test golden_portfolio_planning` — all 6 migrated goldens pass under the renamed variants
3. Existing suite: `cargo test --workspace`
4. Lint: `cargo clippy --workspace --all-targets -- -D warnings`
5. Doc regen check: `python3 scripts/golden_inventory.py --write --check-docs` succeeds (or, if the script reports drift, commit the regenerated docs alongside the goldens)

### Invariants

1. Each new golden's invariant maps to the strongest available proof surface per Verification Layers; no scenario relies solely on downstream event-log absence when a decision-trace assertion exists.
2. Each migrated golden preserves the original scenario's intent — the rename does not change the asserted behavior, only the symbol names.
3. The new goldens cover every `IntentionResumeCondition` variant (5 variants → 5 cases collected across scenarios (g) and the assembled invariant coverage) and every `IntentionAbandonCondition` variant (6 variants → at least one case per variant across scenario (h)).
4. `cargo test -p worldwake-ai --test golden_portfolio_five_slots` exercises the full agent-decision-cycle pipeline (decision-trace + action-trace + event-log delta), not just `assemble_portfolio` in isolation.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_portfolio_five_slots.rs` — 9 new tests per Verification Layers and Assumption Reassessment item 2
2. `crates/worldwake-ai/tests/golden_portfolio_planning.rs` — 6 migrated tests per the per-test table above
3. Likely: `crates/worldwake-ai/tests/golden_harness/` — extend factory helpers as needed for five-slot and condition-driven fixture construction

### Commands

1. `cargo test -p worldwake-ai --test golden_portfolio_five_slots`
2. `cargo test -p worldwake-ai --test golden_portfolio_planning`
3. `cargo test -p worldwake-ai -- --list | grep -E "golden_portfolio"` — verify test discovery
4. `python3 scripts/golden_inventory.py --write --check-docs` — regenerate golden inventory docs
5. `./scripts/verify.sh`
