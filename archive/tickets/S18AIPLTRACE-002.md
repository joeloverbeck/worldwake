# S18AIPLTRACE-002: Add root candidate provenance to planner decision traces

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` decision-trace/search trace model, planner search instrumentation, focused trace tests
**Deps**: `docs/FOUNDATIONS.md`, `archive/tickets/completed/S18AIPLTRACE-001.md`, `archive/tickets/completed/E16BFORLEGJURCON-009.md`

## Problem

Current decision traces show generated goals, plan attempts, and final selection, but they do not expose the root search candidate inventory for a given goal. When a plan fails at the planner boundary, the trace cannot currently answer whether the planner never saw the expected candidate, rejected it for binding/payload reasons, or explored the wrong search space until budget exhaustion. That weakens explainability and violates the project's traceability standard for emergent behavior.

## Assumption Reassessment (2026-03-22)

1. Current trace coverage already records useful planner provenance:
   - `PlanningPipelineTrace`, `PlanSearchTrace`, `PlanAttemptTrace`, and `BindingRejection` live in `crates/worldwake-ai/src/decision_trace.rs`
   - `search_plan()` in `crates/worldwake-ai/src/search.rs` already emits expansion summaries and binding rejections
   - focused coverage exists at `planner_ops::tests::build_semantics_table_classifies_registered_planner_action_defs`, `search::tests::search_political_goal_uses_consult_record_as_mid_plan_prerequisite_when_belief_unknown`, and `agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning`
2. Reassessment against current code shows the original motivating regression no longer reproduces as written:
   - `planner_ops::tests::build_semantics_table_classifies_registered_planner_action_defs` now explicitly asserts `press_force_claim -> PlannerOpKind::PressForceClaim`
   - `agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning` already proves `GoalKind::ClaimOffice { office }` reaches planning and selects a one-step `PressForceClaim` plan under full registries
   - the ticket scope must therefore shift from "recover the old force-law omission" to "make current planner root-candidate formation directly explainable for any goal family"
3. The live goal family still worth stressing is `GoalKind::ClaimOffice`, but the generic missing provenance sits at the search candidate surface in `search_candidates()` / `search_candidates_from_affordance()` / `planner_only_candidates()` rather than candidate generation, ranking, or authoritative action validation.
4. This remains an AI/runtime traceability ticket. The intended verification layer is focused `search.rs` planner-trace coverage plus runtime `agent_tick` decision-trace coverage. Local needs-only harness is not sufficient for the runtime proof because the political root-candidate surface depends on the full action registries.
5. The ordering contract is not strict tick separation. The contract is planner root-candidate provenance ordering within one goal attempt: candidate inventory first, candidate filtering next, then successor/selection outcome. The current gap is observability of those root boundaries, not downstream authoritative timing.
6. No heuristic or planner behavior is being removed. The proposed change is additive planner-search trace substrate so debugging can distinguish "candidate was never formed", "candidate was formed then filtered", and "candidate survived filtering but still lost in search" without bespoke instrumentation.
7. The first boundary this ticket targets is root candidate formation before successor expansion and before authoritative action start. The current shared symbols verified during reassessment are `search_plan()`, `search_candidates()`, `search_candidates_from_affordance()`, `planner_only_candidates()`, `build_payload_override()`, and `build_semantics_table()`.
8. For the political runtime scenario, the relevant closure boundary is the AI-layer path between `GoalKind::ClaimOffice` and the concrete root candidate set that includes `PlannerOpKind::PressForceClaim`. The current trace proves the selected plan, but it does not expose the full root inventory or any pre-expansion filtering reasons directly.
9. No `ControlSource`, queued-input, or runtime-driver retention behavior is part of scope.
10. Scenario isolation is deliberate: the traceability contract under test is root candidate availability and filtering, not whether later same-tick or later-tick office mutation happened.
11. Mismatch corrected: the current trace system is materially stronger than the original ticket narrative assumed, and the force-law path is already architecturally wired. The remaining gap is narrower and cleaner: root candidate provenance is still absent, so scope should be additive traceability rather than force-law bug repair.
12. The reachable failure envelope under current code is any search attempt where the planner can legally exhaust `PlanningBudget.max_node_expansions` or terminate unsupported/frontier-exhausted after the wrong or incomplete root candidate set enters search. The ticket should capture that generic invariant rather than a stale office-specific arithmetic story.

## Architecture Check

1. The clean design is to record candidate provenance at the earliest planner boundary that materially affects search: root candidate enumeration and filtering per goal attempt. That is cleaner than relying on downstream budget-exhaustion summaries or test-side bespoke instrumentation because it keeps explanation with the planner itself.
2. The trace payload should be generic across all goals and actions, not force-law specific. The architecture must explain any missing root candidate, not just `press_force_claim`.
3. No backwards-compatibility aliasing or alternate trace-only search path should be introduced. Existing planner behavior remains authoritative; this ticket only records more precise provenance.

## Verification Layers

1. Root search candidate inventory for a goal attempt -> decision trace / focused `search.rs` trace assertions
2. Candidate rejection reason (`binding`, `payload override unsupported`, `blocked facility use`) -> focused planner trace assertions
3. Downstream selected plan or `BudgetExhausted` result still remains intact -> existing `PlanSearchTrace` + runtime `agent_tick` decision trace
4. Authoritative action lifecycle ordering is not the contract here; action traces are out of scope except as existing downstream sanity checks
5. Additional layer mapping beyond planner trace is not primary because this ticket is about exposing missing planner provenance at the earliest causal boundary

## What to Change

### 1. Extend decision-trace types with root candidate provenance

Add a new compact trace payload under `PlanAttemptTrace` or `PlanSearchTrace` that records, per goal attempt:

- concrete root candidates seen by search
- action def id / action name
- planner op kind when classified
- authoritative targets / whether candidate was planner-only
- payload-override status
- filter/rejection reason when a candidate was excluded before successor expansion

The type should stay compact and deterministic. It must not dump arbitrary debug strings or duplicate the full affordance model.

### 2. Instrument `search_candidates()` and adjacent planner boundaries

Wire provenance collection through:

- `search_candidates()`
- `search_candidates_from_affordance()`
- `planner_only_candidates()`
- the `build_payload_override()` boundary
- the blocked-facility-use filter

Record precise typed reasons instead of broad "candidate missing" summaries.

### 3. Add focused and runtime trace coverage

Add focused tests in `crates/worldwake-ai/src/search.rs` and/or `crates/worldwake-ai/src/decision_trace.rs` proving:

- a valid root candidate is recorded when present
- a candidate rejected by payload-override incompatibility is trace-visible
- a blocked-facility-use pre-expansion filter is trace-visible
- runtime `agent_tick` traces expose the root-candidate inventory for a political goal using full action registries

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/search.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (modify, trace plumbing/tests only if needed)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify only if a runtime trace assertion is the strongest proof)

## Out of Scope

- Changing planner selection behavior, budgets, or search semantics
- Adding force-law-specific trace hacks
- Broader action-trace redesign

## Acceptance Criteria

### Tests That Must Pass

1. Focused planner trace test records root candidate provenance for a political search attempt
2. Focused planner trace test records typed rejection reasons and successor-build failures at the root boundary, including at least one non-binding filter
3. Runtime trace test proves a `GoalKind::ClaimOffice` attempt exposes root-candidate provenance under full action registries
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Root candidate provenance must be deterministic and derived from planner state, not ad-hoc debug output
2. Trace additions must not change planner behavior or selected plans

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search.rs` — add focused search-trace assertions for root candidate inventory and typed rejection reasons, including payload-override and blocked-facility-use surfaces
2. `crates/worldwake-ai/src/agent_tick.rs` — extend decision-trace coverage proving root candidate provenance survives through runtime planning traces for `ClaimOffice`

### Commands

1. `cargo test -p worldwake-ai --lib search::tests::search_political_goal_uses_consult_record_as_mid_plan_prerequisite_when_belief_unknown -- --exact`
2. `cargo test -p worldwake-ai --lib agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning -- --exact`
3. `cargo test -p worldwake-ai`

## Outcome

- Outcome amended: 2026-03-22
- Completion date: 2026-03-22
- Actual changes:
  - added typed root-candidate provenance to root `SearchExpansionSummary` entries in `worldwake-ai` decision traces
  - recorded root candidate payload status plus typed root outcomes for filtered candidates and successor-build skips
  - wired root trace capture through `search_candidates()` and root successor construction without changing planner selection behavior
  - extended focused `search.rs` coverage for force-claim payload failure visibility and blocked-facility-use filtering
  - extended runtime `agent_tick` coverage so `ClaimOffice` traces assert retained `PressForceClaim` visibility at the root search boundary
- Deviations from original plan:
  - the original force-law motivating regression was already fixed in current code, so the ticket was narrowed from “recover missing `press_force_claim` semantics” to additive generic root provenance
  - “missing semantics” root-candidate coverage was removed from scope because the current planner architecture omits unclassified actions before any concrete `SearchCandidate` exists; tracing that would require a separate classifier-audit path rather than root candidate provenance
  - the new provenance lives on the root `SearchExpansionSummary` rather than a separate top-level `PlanAttemptTrace` payload because it is an expansion-boundary fact and this kept planner APIs stable
- Verification results:
  - passed `cargo test -p worldwake-ai`
  - passed `cargo clippy -p worldwake-ai --lib -- -D warnings`
  - passed `cargo clippy --workspace --all-targets -- -D warnings` after follow-up cleanup of the previously pre-existing pedantic violations in `worldwake-core`, `worldwake-sim`, `worldwake-systems`, and `worldwake-ai` test helpers
