# S18AIPLTRACE-002: Add root candidate provenance to planner decision traces

**Status**: PENDING
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
2. Reassessment against `archive/tickets/completed/E16BFORLEGJURCON-009.md` showed a real traceability gap: for `GoalKind::ClaimOffice { office }` on the force-law path, the trace reported `BudgetExhausted` but did not reveal that `press_force_claim` was absent from planner semantics at the search root.
3. The live goal family under stress is `GoalKind::ClaimOffice`, and the missing provenance sits at the search candidate surface in `search_candidates()` / `search_candidates_from_affordance()` / `planner_only_candidates()` rather than candidate generation or authoritative action validation.
4. This is an AI/runtime traceability ticket. The intended layer is runtime `agent_tick` decision-trace/integration plus focused `search.rs` trace coverage. Local needs-only harness is not sufficient because the motivating regression depends on political/system actions from the full action registries.
5. The ordering contract is not strict tick separation. The contract is planner search-root provenance ordering within one goal attempt: candidate inventory first, candidate filtering next, then successor/selection outcome. The divergence in the motivating regression depended on mixed-layer planner semantics omission plus budget exhaustion, not on downstream authoritative timing.
6. No heuristic is being removed. This ticket adds missing planner-search substrate so later debugging does not rely on ad-hoc assertions or downstream action absence.
7. The first failure boundary in the motivating bug was planner search candidate formation, specifically before authoritative start and before AI blocker reconciliation. The shared runtime symbols already checked were `search_plan()`, `search_candidates()`, `build_payload_override()`, and `build_semantics_table()`.
8. The relevant political closure boundary is not support declaration or office-holder mutation. It is the AI-layer path between `GoalKind::ClaimOffice` and the concrete `PlannerOpKind::PressForceClaim` root candidate. Current traces do not expose that boundary directly.
9. No `ControlSource` or queued-input runtime manipulation is part of the intended scope.
10. Scenario isolation is deliberate: the traceability contract under test is root candidate availability and filtering, not whether a later same-tick or later-tick authority mutation happened.
11. Mismatch corrected: the current trace system is stronger than "none", but it still lacks the root candidate inventory needed by Principle 13 and Precision Rule 15. Scope should be additive traceability, not generic "more tracing."
12. The motivating failure is reachable under current code because search can legally consume the default `PlanningBudget.max_node_expansions` budget on the wrong branch set when a root candidate is missing or silently filtered. The concrete arithmetic is the existing planner budget and root expansion count, not any office timing arithmetic.

## Architecture Check

1. The clean design is to record candidate provenance at the earliest planner boundary that materially affects search: root candidate enumeration and filtering per goal attempt. That is cleaner than relying on downstream budget-exhaustion summaries or test-side bespoke instrumentation because it keeps explanation with the planner itself.
2. The trace payload should be generic across all goals and actions, not force-law specific. The architecture must explain any missing root candidate, not just `press_force_claim`.
3. No backwards-compatibility aliasing or alternate trace-only search path should be introduced. Existing planner behavior remains authoritative; this ticket only records more precise provenance.

## Verification Layers

1. Root search candidate inventory for a goal attempt -> decision trace / focused `search.rs` trace assertions
2. Candidate rejection reason (`binding`, `missing semantics`, `payload override unsupported`, `blocked facility use`) -> focused planner trace assertions
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
- the semantics-table lookup boundary
- the `build_payload_override()` boundary
- the blocked-facility-use filter

Record precise typed reasons instead of broad "candidate missing" summaries.

### 3. Add focused and runtime trace coverage

Add focused tests in `crates/worldwake-ai/src/search.rs` and/or `crates/worldwake-ai/src/decision_trace.rs` proving:

- a valid root candidate is recorded when present
- a candidate rejected for missing semantics is trace-visible
- a candidate rejected by payload-override incompatibility is trace-visible
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
2. Focused planner trace test records at least one typed rejection reason before successor expansion
3. Runtime trace test proves a `GoalKind::ClaimOffice` attempt exposes root-candidate provenance under full action registries
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Root candidate provenance must be deterministic and derived from planner state, not ad-hoc debug output
2. Trace additions must not change planner behavior or selected plans

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search.rs` — add focused search-trace assertions for root candidate inventory and typed rejection reasons
2. `crates/worldwake-ai/src/agent_tick.rs` — add or extend decision-trace coverage proving root candidate provenance survives through runtime planning traces

### Commands

1. `cargo test -p worldwake-ai --lib search::tests::search_political_goal_uses_consult_record_as_mid_plan_prerequisite_when_belief_unknown -- --exact`
2. `cargo test -p worldwake-ai --lib agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning -- --exact`
3. `cargo test -p worldwake-ai`
