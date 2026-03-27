# AITRACE-002: Planner/runtime legality provenance in traces

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — justice punishment candidate provenance, action start-failure provenance for punishment legality mismatches
**Deps**: [archive/tickets/E17CRITHEJUS-013.md](/home/joeloverbeck/projects/worldwake/archive/tickets/E17CRITHEJUS-013.md), [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md), [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md), [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md)

## Problem

Mixed-layer AI failures are still harder to explain than they should be. The current trace surfaces are strong at showing:

1. whether a goal was generated or selected
2. whether an action started, committed, or aborted

But they are weaker at showing the concrete shared fact that made the planner believe a branch was lawful when runtime later rejected it at authoritative start. That gap violates the debug surface expected by `FOUNDATIONS.md` Principle 27 and weakens the separation between belief-state reasoning and authoritative legality from Principles 12, 24, and 25.

The E17 justice chain exposed the exact missing surface: `candidate_generation` selected `GoalKind::PunishAccused { punishment: Fine { .. } }` from a planner-local `locally_observed_commodity_quantity()` read, but only lower-layer code inspection revealed why runtime `ensure_accessible_quantity()` later rejected the same punishment at start. The product should make that contradiction inspectable from trace data.

## Assumption Reassessment (2026-03-27)

1. Current trace surfaces already cover outcome-level reasoning well:
   - decision traces in [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)
   - action traces in [`crates/worldwake-sim/src/action_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs)
   - search root diagnostics in [`crates/worldwake-ai/src/search/candidates.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs)
2. The missing surface is not “was the branch present?” but “which concrete planner-visible read selected this punishment, and which authoritative fact later contradicted it?” That is a different traceability boundary than candidate omission or generic start/abort lifecycle.
3. Shared abstraction boundary under audit: punishment selection in [`candidate_punishment_for_case()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) versus authoritative punishment start validation in [`validate_fine_start()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/justice_actions.rs) and [`ensure_accessible_quantity()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/justice_actions.rs).
4. Intended invariant: if planner-side punishment selection and runtime punishment start disagree, the causal reason for the disagreement must be reconstructable from trace/state inspection without ad hoc debug output.
5. Live `GoalKind` / operator surface under test:
   - `GoalKind::PunishAccused` currently exposes `PlannerOpKind::Travel` plus `PlannerOpKind::Fine` or `PlannerOpKind::Exile` in [`GoalKindPlannerExt::relevant_op_kinds()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) and exact root synthesis for punishment targets in [`GroundedGoal::synthesized_root_candidate_targets()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs).
   - Mismatch correction: [`docs/planner-contracts.md`](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md) still says `PunishAccused` is deferred, but the live code and focused search tests prove the punishment operator family is already surfaced.
6. The live scenario does not require a generic repo-wide legality-provenance framework. The concrete missing gap is punishment provenance for the E17 justice path; broadening to unrelated trade/resource branches in this ticket would dilute the boundary under audit and expand the change surface without current evidence.
7. This is a mixed-layer AI ticket. The intended layer spans candidate generation, authoritative start, and trace diagnostics. Full action registries and one golden-style regression are required.
8. The current docs already acknowledge a weaker version of this gap:
   - [`docs/golden-e2e-testing.md`](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) says to drop to lower-layer tests when traces do not expose enough provenance
   - [`docs/planner-contracts.md`](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md) documents root omission and duration-skip diagnostics, but not planner/runtime legality mismatches after a branch is surfaced
9. This ticket is not asking for debug-only prints or compatibility shims. Any new provenance surface must become part of the live clean trace model, not an auxiliary workaround.
10. Adjacent contradiction classification:
   - required consequence in scope: missing trace data for planner/runtime punishment legality disagreements
   - separate docs contradiction discovered during reassessment: stale deferred-operator text in [`docs/planner-contracts.md`](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md)
   - out of scope: changing substantive punishment legality rules unless a missing provenance field cannot be emitted without a small supporting refactor
11. The relevant arithmetic/data contract is concrete and local, not abstract. This ticket should expose exact quantities, target identities, consulted accusation identity, and authoritative mismatch facts, not fuzzy “confidence” summaries. That is required by Principles 3 and 25.

## Architecture Check

1. The clean architecture is to add one bounded structured provenance contract for justice punishment selection/start, rather than inventing a generic stringly “legality debug” layer. The E17 path is the live contradiction, and its shared facts are concrete enough to model directly.
2. This is cleaner than sprinkling `eprintln!` diagnostics through tests or action handlers because the provenance remains structured, deterministic, and queryable.
3. This is cleaner than a broad repo-wide legality-provenance abstraction because current evidence only justifies the punishment boundary, and forcing unrelated domains into one abstraction now would create speculative architecture.
4. No backward-compatibility aliasing or duplicate trace pipelines should be introduced. Existing decision/action traces should gain the missing lawful data in place.

## Verification Layers

1. Planner candidate legality records the concrete read used to emit `PunishAccused(Fine)` -> focused decision-trace/unit coverage in `worldwake-ai`
2. Runtime start failure records the authoritative fact that contradicted the planner-side fine assumption -> focused action/start-failure trace coverage in `worldwake-sim` plus justice-focused runtime coverage
3. Cross-layer contradiction is inspectable without ad hoc logging -> focused mixed-layer regression plus one golden/debugging-oriented proof surface
4. The ticket does not use durable downstream world-state as a proxy for legality disagreement; the contract is the earlier candidate/start boundary itself
5. If traces still cannot explain the punishment mismatch after the new payloads land, the strongest lower-layer proof surface remains the relevant belief-view or justice validator unit test; broader multi-domain provenance should become a follow-up ticket rather than being folded into this one

## What to Change

### 1. Extend planner-side trace payloads for punishment candidate formation

Update decision-trace or candidate-trace structures so `GoalKind::PunishAccused` can record the concrete planner-visible facts used to choose `Fine` versus `Exile`.

Required facts:
- consulted accusation entry / theft facts
- actor place and accused place as seen by the planner
- exact locally observed quantity used for `Fine`
- required fine amount
- chosen punishment kind

The payload must stay concrete and bounded. Do not add arbitrary free-form strings.

### 2. Extend runtime start-failure trace payloads for punishment contradiction facts

When a punishment request reaches authoritative start and fails for a legality reason that contradicts the planner-side fine assumption, emit structured contradiction data naming:

- the punishment branch
- the specific authoritative fact that failed
- the concrete entity/place/quantity involved when relevant

This should be queryable from existing trace sinks rather than requiring test-local logging.

### 3. Add focused cross-layer regression coverage

Add focused tests that prove:

- `PunishAccused(Fine)` records the planner-side fact it used
- the same punishment request records the runtime-side contradiction when start fails
- the two traces together explain the mismatch without source inspection

Use the justice fine path where quantity/control legality is already known to be subtle.

### 4. Add one golden-oriented traceability proof

Add or update one golden or golden-harness regression that demonstrates the improved punishment trace surface is sufficient to debug a mixed-layer disagreement without custom output.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/search/candidates.rs` (modify only if root/selection provenance needs extension)
- `crates/worldwake-sim/src/action_trace.rs` (modify)
- `crates/worldwake-sim/src/request_resolution_trace.rs` (modify only if the contradiction boundary belongs there)
- `crates/worldwake-sim/src/tick_step.rs` (modify if trace wiring is needed)
- `crates/worldwake-systems/src/justice_actions.rs` (modify if shared structured punishment provenance is best emitted from the validation boundary)
- `crates/worldwake-ai/tests/` (modify/add focused + golden-facing coverage)
- `crates/worldwake-sim/src/` tests (modify/add focused trace coverage)
- `crates/worldwake-systems/src/` tests (modify/add focused justice coverage if the authoritative contradiction proof belongs there)

## Out of Scope

- Changing justice legality rules themselves unless required to expose trace data cleanly
- Broad trace provenance work for trade, cargo, or unrelated goal families
- Adding generic stringly debug logs
- Rewriting existing trace sinks wholesale
- Broad UI work for trace presentation

## Acceptance Criteria

### Tests That Must Pass

1. Focused test proving decision trace records concrete punishment-selection provenance for a selected `PunishAccused(Fine)` candidate
2. Focused test proving runtime trace records the authoritative contradiction fact for the same punishment branch
3. Focused mixed-layer regression proving the punishment mismatch is explainable from trace data alone
4. Existing relevant AI and sim trace suites remain green
5. Existing suite: `cargo test --workspace`

### Invariants

1. Traceability remains concrete-state-based: entities, places, quantities, accusation identities, and named lawful facts, not abstract “scores” or human-only prose
2. The new provenance surface does not create a parallel authority path; it only explains existing lawful planner/runtime behavior

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` and/or `crates/worldwake-ai/src/decision_trace.rs` focused tests — prove planner-side punishment provenance is recorded for a selected `Fine` branch
2. `crates/worldwake-sim/src/action_trace.rs` and/or justice-focused runtime tests — prove runtime contradiction provenance is recorded structurally for punishment start failure
3. `crates/worldwake-ai/tests/golden_*.rs` — one regression proving the new traces make the punishment mismatch inspectable without ad hoc output

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-sim`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-27
- What actually changed:
  - Added structured planner-side punishment provenance for `PunishAccused(Fine)` so candidate traces record the consulted accusation entry, theft facts, planner-visible places, required fine amount, and locally observed quantity used to select `Fine`.
  - Added structured runtime start-failure legality provenance for fine punishment start failures so action traces record the authoritative contradiction instead of only a string summary.
  - Threaded the new legality payloads through existing decision/action trace surfaces and added a focused golden proving the planner/runtime mismatch is explainable from trace data alone.
  - Added a shared authoritative helper for counting locally controlled commodity quantity at a place and reused it in the per-agent belief/runtime boundary to keep the punishment-selection read grounded in one place.
- Deviations from original plan:
  - Narrowed the ticket from a generic planner/runtime legality-provenance framework to the concrete E17 justice punishment boundary that is actually missing in the live code.
  - `docs/planner-contracts.md` was found to contain stale deferred-operator text for `PunishAccused`; that docs contradiction was recorded in reassessment but not changed in this ticket.
  - No generic multi-domain legality-debug abstraction was added; the delivered design keeps the change bounded to the proven justice contradiction.
- Verification results:
  - `cargo test -p worldwake-ai` passed.
  - `cargo test -p worldwake-sim` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace` passed.
