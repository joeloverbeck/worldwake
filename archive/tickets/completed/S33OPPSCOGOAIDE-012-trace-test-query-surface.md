# S33OPPSCOGOAIDE-012: Add stable decision-trace query/build helpers for tests

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` decision-trace query surface and representative trace-test migrations
**Deps**: S33OPPSCOGOAIDE-009, archive/tickets/S33OPPSCOGOAIDE-011-opportunity-scoped-stage1-trace.md, specs/S33-opportunity-scoped-goal-identity.md

## Problem

The decision-trace architecture is now correctly opportunity-scoped, but many focused and golden tests still inspect raw trace structs field-by-field. That couples tests to internal trace storage choices such as `goal` vs `opportunity`, and it turns architectural cleanups into broad mechanical test edits.

This is a debugging-contract problem, not a planner-behavior problem. Worldwake wants explainable emergence and durable debugging surfaces. Tests that must know every internal trace struct field are not a durable debugging surface.

## Assumption Reassessment (2026-03-28)

1. The shared abstraction boundary under audit is the public decision-trace read surface used by tests: `DecisionTraceSink`, `AgentDecisionTrace`, `PlanningPipelineTrace`, `CandidateTrace`, `CandidateEvidenceTrace`, `RankedGoalSummary`, and `SelectionTrace` in [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs). This ticket does not change planner/runtime behavior; it changes how tests query trace state.
2. Reassessment after `archive/tickets/S33OPPSCOGOAIDE-011-opportunity-scoped-stage1-trace.md` shows the architecture already absorbed the opportunity split correctly. The live code already exposes desire-level helpers `DecisionTraceSink::goal_status_at()` and `DecisionTraceSink::goal_history_for()`, and `SelectionTrace` already carries `selected_opportunity` alongside `selected`; this ticket is therefore a follow-up on remaining opportunity-scoped read ergonomics, not a first introduction of helper APIs.
3. The live remaining brittleness is real and specific: representative tests still traverse raw fields like `planning.candidates.generated.iter().any(|goal| goal.goal_key.kind == ...)`, `planning.candidates.evidence.iter().find(...)`, and `planning.selection.selected == Some(...)` in files such as [`crates/worldwake-ai/src/agent_tick/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs), [`crates/worldwake-ai/tests/golden_care.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_care.rs), [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs), [`crates/worldwake-ai/tests/golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs), and [`crates/worldwake-ai/tests/golden_emergent.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs).
4. This is still a proof-surface issue, not a behavior gap. Focused unit/runtime tests and golden tests already cover the underlying planning behavior, including same-goal sibling opportunity ordering and evidence provenance; the gap is the absence of explicit opportunity-scoped lookup helpers for those assertions.
5. The live `GoalKind` families exercised by the representative remaining raw traversals include `TreatWounds`, `ClaimOffice`, `RestockCommodity`, and `InvestigateViolation`, and the relevant trace surfaces are candidate generation (`generated`, `evidence`, `ranked`) and selection (`selected`, `selected_opportunity`, `selected_plan`). No authoritative validation or action start symbols are in scope for this ticket.
6. The same fact currently has two lawful transport paths in tests:
   - raw traversal over `generated`, `ranked`, `evidence`, and `selection`
   - helper queries on `DecisionTraceSink`
   After this ticket, the canonical path for opportunity-scoped trace assertions should be explicit helper/query methods on the trace types themselves. Raw struct access remains available only for facts the helper layer does not expose yet.
7. This ticket must preserve `OpportunityKey` as the canonical identity for concrete candidates. Desire-level helpers may derive from canonical trace state, but any new helper that identifies a concrete candidate must accept or return `OpportunityKey`, not rebuild goal-only aliases.
8. Mismatch + correction: the original ticket over-scoped “lightweight builders/helpers for focused tests” and broad `golden_*.rs` migration. Live `decision_trace.rs` tests already have adequate local constructors (`goal_trace`, `default_opportunity`), and the codebase does not need a new parallel fixture layer. Scope is corrected to query helpers plus a small representative migration set that proves the new surface is sufficient.
9. Mismatch + correction: `crates/worldwake-ai/src/lib.rs` does not need planned changes unless a newly introduced public helper type must be re-exported. If helper methods live on already-public trace types, `lib.rs` stays untouched.
10. This is not a stale-request, contested-affordance, or authoritative-start ticket. Additional runtime/action-trace mapping is not applicable beyond preserving the current decision-trace debugging contract.

## Architecture Check

1. A stable query/build surface for tests is cleaner than asking tests to know every field layout inside `CandidateTrace`, `RankedGoalSummary`, or `SelectionTrace`. That aligns with `docs/FOUNDATIONS.md` Principle 27: debuggability is a product feature, not an accident of internal struct shape.
2. The clean approach is additive query APIs over the canonical trace model, not compatibility aliases on the live structs and not a second fixture/builder layer. Tests should ask explicit questions like “was this opportunity generated?”, “which ranked opportunity was selected?”, or “give me the evidence trace for this opportunity/goal family” rather than reaching into storage fields.
3. This also aligns with Principle 25: helpers are derived read surfaces over canonical trace state, never parallel truth. The canonical stored trace remains opportunity-scoped.
4. No backwards-compatibility field aliases or duplicate goal-only mirrors should be introduced. Broken tests should migrate to the helper surface rather than keeping deprecated raw-field assumptions alive.

## Verification Layers

1. Stable opportunity-scoped trace lookup for generated/ranked/evidence/selected facts -> focused `decision_trace` helper tests.
2. Desire-level helper APIs still derive correct answers from canonical opportunity-scoped state -> existing `DecisionTraceSink` helper tests plus focused new helper coverage.
3. Representative focused runtime and golden tests can assert scenario facts through helper/query APIs without direct raw-field traversal -> representative migrations in `agent_tick` and selected `golden_*` tests.
4. Additional authoritative/action-trace mapping is not applicable because this ticket changes only the trace query/build proof surface, not runtime or authoritative behavior.
5. If a remaining golden assertion still genuinely needs raw struct access after helper additions, the ticket should name that gap explicitly instead of hiding it behind partial wrappers.

## What to Change

### 1. Add stable trace query helpers

- Add helper/query methods on the existing public trace types for common test assertions:
  - generated-candidate lookup by `OpportunityKey` and by `GoalKey`
  - ranked summary lookup by `OpportunityKey`
  - selected-ranked summary lookup keyed by the canonical selected opportunity
  - candidate-evidence lookup by `OpportunityKey` and, where justified, by `GoalKey`
  - selection predicates that avoid repeated raw equality checks against `selected` / `selected_opportunity`
- Keep helper signatures explicit about whether they operate at desire level or opportunity level.

### 2. Migrate representative focused and golden tests

- Update the most coupled tests to use the new helper/query/build APIs instead of direct field traversal.
- Prioritize representative tests that currently prove:
  - ranked comparison assertions
  - selected opportunity assertions
  - candidate evidence assertions
  - generated-candidate presence assertions
- Do not attempt a repo-wide style rewrite beyond the places needed to prove the new surface is sufficient.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — helper/query/build surface)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — migrate representative focused assertions)
- `crates/worldwake-ai/tests/golden_care.rs` (modify — representative selected/generated assertions)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify — representative generated assertion)
- `crates/worldwake-ai/tests/golden_supply_chain.rs` (modify — representative evidence assertion)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify — representative selected/generated assertion)

## Out of Scope

- Planner/runtime behavior changes
- Authoritative action/system changes
- Opportunity identity refactors
- Save/load changes
- Reintroducing goal-only compatibility aliases
- A repo-wide test-style normalization pass
- New standalone trace fixture/builder modules

## Acceptance Criteria

### Tests That Must Pass

1. Focused decision-trace tests can assert generated/ranked/evidence/selected facts through stable helper/query APIs without direct raw-field traversal for the covered scenarios.
2. The helper surface preserves the distinction between desire-level queries and opportunity-level queries.
3. Representative golden tests use the helper/query surface instead of raw struct layout for their trace assertions.
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo clippy --workspace`
6. Existing suite: `cargo test --workspace`

### Invariants

1. `OpportunityKey` remains the canonical identity for concrete candidate-stage trace facts.
2. Helper/query APIs are derived views over trace state, not duplicate stored truth.
3. No backward-compatibility aliases are added to keep obsolete raw trace field assumptions alive.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` — add focused helper/query tests for generated, ranked, selected, and evidence lookup.
   Rationale: proves the new helper surface is strong enough to replace raw storage traversal for the core trace questions.
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — migrate one or more representative focused assertions to the helper surface.
   Rationale: proves the helpers work in realistic traced agent-tick scenarios, not just synthetic unit fixtures.
3. `crates/worldwake-ai/tests/golden_care.rs`, `crates/worldwake-ai/tests/golden_offices.rs`, `crates/worldwake-ai/tests/golden_supply_chain.rs`, and `crates/worldwake-ai/tests/golden_emergent.rs` — migrate representative golden assertions that currently inspect `generated`, selected goal/opportunity state, or evidence traces by raw field layout.
   Rationale: proves the helper surface is durable enough for end-to-end trace assertions across multiple goal families without requiring a repo-wide migration.

### Commands

1. `cargo test -p worldwake-ai decision_trace::tests`
2. `cargo test -p worldwake-ai trace_force_law_office_skips_political_candidates_and_planning`
3. `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker`
4. `cargo test -p worldwake-ai golden_force_claim_ai_installation`
5. `cargo test -p worldwake-ai golden_stale_prerequisite_belief_discovery_replan`
6. `cargo test -p worldwake-ai golden_same_place_concurrent_violations_stay_distinct`
7. `cargo test -p worldwake-ai`
8. `cargo clippy --workspace`
9. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-28
- What changed:
  - added explicit trace query helpers on `CandidateTrace`, `SelectionTrace`, and `PlanningPipelineTrace` for generated-candidate lookup, evidence lookup, canonical selected-opportunity lookup, and selected-goal predicates
  - kept `OpportunityKey` as the concrete candidate identity; no goal-only aliases or compatibility shims were introduced
  - migrated representative focused and golden assertions in `agent_tick/tests.rs`, `golden_care.rs`, `golden_offices.rs`, `golden_supply_chain.rs`, and `golden_emergent.rs` onto the helper surface
- Deviations from original plan:
  - no new builder/fixture layer was added because existing local test constructors in `decision_trace.rs` were already sufficient
  - `crates/worldwake-ai/src/lib.rs` did not need changes because the new API lives on already-public trace types
- Verification results:
  - targeted helper and representative scenario commands listed above passed
  - `cargo test -p worldwake-ai` passed
  - `cargo clippy --workspace` passed
  - `cargo test --workspace` passed
