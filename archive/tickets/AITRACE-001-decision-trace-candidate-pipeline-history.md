# AITRACE-001: Expose Candidate-Pipeline History And Absence Reasons In Decision Traces

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — decision-trace sink query helpers, focused sink tests, targeted golden helper consumers
**Deps**: `crates/worldwake-ai/src/decision_trace.rs`, `crates/worldwake-ai/tests/golden_emergent.rs`, `crates/worldwake-ai/tests/golden_offices.rs`, `docs/golden-e2e-testing.md`

## Problem

The current decision trace system is already useful, but it is still harder than it should be to answer architecture-level questions such as:
- Did a goal never generate, or did it generate and get suppressed?
- Was a goal present but zero-motive filtered?
- Was there no fresh planning pass because the agent continued an existing commitment?
- On which ticks did a given goal appear, disappear, or lose the ranking race?

Today the underlying information is mostly already present, but it is spread across raw per-tick trace fields and repeated manual scans in tests. That makes common "why not?" queries noisier and more error-prone than they should be.

This ticket adds a canonical derived query surface on top of the existing trace records so candidate-pipeline history and absence reasons are stable, queryable, and reusable in tests.

## Assumption Reassessment (2026-03-19)

1. The current planning trace already records the core candidate pipeline fields this ticket originally described as missing. `crates/worldwake-ai/src/decision_trace.rs` exposes `CandidateTrace.generated`, `CandidateTrace.suppressed`, `CandidateTrace.zero_motive`, `CandidateTrace.ranked`, `CandidateTrace.omitted_political`, `PlanningPipelineTrace.plan_continued`, and `SelectionTrace.selected_plan_source`.
2. The runtime path already populates those fields. `crates/worldwake-ai/src/agent_tick.rs` records suppressed / zero-motive / omitted-political diagnostics in the final `CandidateTrace`, and traced snapshot continuation already records `SelectedPlanSource::SnapshotContinuation`.
3. Focused runtime coverage for two key "absence reason" branches already exists:
   - `agent_tick::tests::trace_snapshot_continuation_records_selected_plan_provenance`
   - `agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning`
   The real gap is not raw trace capture; it is the lack of a canonical helper layer that derives stable per-goal history and status from those traces.
4. Golden tests already use decision traces for AI reasoning, but they currently hand-roll repeated scans across ticks in `crates/worldwake-ai/tests/golden_emergent.rs` and `crates/worldwake-ai/tests/golden_offices.rs` instead of relying on shared sink helpers.
5. `docs/golden-e2e-testing.md` already instructs authors to prefer decision traces for AI reasoning questions. The missing architectural substrate is not documentation or a new trace schema; it is a reusable derived query API on the existing sink.
6. Mismatch + correction: the original ticket overstated the missing architecture and cited `docs/FOUNDATIONS.md` as if it still contained a numbered "Principle 27". The corrected scope is narrower and cleaner: add derived sink helpers over the existing trace schema, avoid new stored trace fields unless a truly missing state is discovered during implementation, and do not touch planner or authoritative behavior.

## Architecture Check

1. The clean design is to keep the existing `AgentDecisionTrace` / `PlanningPipelineTrace` schema as the single source of truth and add derived helper methods on `DecisionTraceSink`. The current trace model is already rich enough; duplicating the same facts into a second schema would make observability harder to trust, not easier.
2. The helper layer should answer semantic questions such as "suppressed vs zero motive vs outranked vs omitted before generation" without requiring each test to manually stitch together `generated`, `suppressed`, `zero_motive`, `ranked`, and `selected_plan_source`.
3. No backwards-compatibility aliasing or parallel trace schemas should be introduced. If a helper type is added, it should be an explicitly derived view with no stored state and no planner-side branching.

## Verification Layers

1. per-tick goal status is derived correctly from the existing planning trace fields -> focused helper tests in `crates/worldwake-ai/src/decision_trace.rs`
2. continuation-vs-selection provenance remains visible alongside per-goal status -> existing `agent_tick` trace tests plus targeted helper tests
3. existing golden scenarios can replace manual trace scans with the shared helper surface without changing world behavior -> golden regressions in `crates/worldwake-ai/tests/golden_emergent.rs` and `crates/worldwake-ai/tests/golden_offices.rs`
4. authoritative outcomes and planner behavior remain unchanged -> crate and workspace verification suites

## What to Change

### 1. Add derived sink/query helpers for goal history and status

Add stable helper APIs on `DecisionTraceSink` that derive, from the existing trace records:
- per-agent goal history across recorded ticks
- per-tick semantic goal status for a specific goal
- a canonical absence/presence classification suitable for test assertions

The helper must distinguish, from existing trace data, at least:
- omitted before generation when `omitted_political` matches
- generated but suppressed
- generated but zero-motive filtered
- ranked but not selected
- selected
- no trace for that agent/tick

If continuation provenance materially improves the helper surface, expose it as metadata derived from `plan_continued` / `selected_plan_source` rather than as a new planner-side trace field.

### 2. Keep the trace schema minimal

Do not add new stored fields to `AgentDecisionTrace` or `PlanningPipelineTrace` unless implementation proves there is a genuinely missing fact that cannot be derived from the current fields. The default path for this ticket is helper-layer work, not trace-schema growth.

### 3. Add focused helper tests and upgrade a small number of goldens

Add focused tests that prove:
- suppressed, zero-motive, omitted-before-generation, ranked, and selected states are distinguished correctly
- continuation metadata remains visible without pretending a fresh plan search happened
- the new helper APIs return deterministic histories for identical trace inputs

Then update a small number of existing goldens to use the shared helper surface where it materially improves clarity, without rewriting unrelated tests.

## Files to Touch

- `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs` (modify)
- `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md` (modify only if helper naming materially changes recommended usage)

## Out of Scope

- Any change to candidate generation, ranking math, suppression policy, or authoritative action semantics
- Scenario-specific one-off helper functions in golden files that bypass the canonical trace sink API
- A new standalone debugging subsystem outside the current decision trace architecture
- Re-encoding facts already present in the current trace schema into a parallel stored structure

## Acceptance Criteria

### Tests That Must Pass

1. A focused helper test proves a goal can be distinguished as omitted-before-generation, suppressed, zero-motive, ranked-but-not-selected, or selected from existing trace data.
2. A focused helper test proves continuation provenance remains queryable alongside per-goal status without implying a fresh search occurred.
3. A sink/helper test proves per-agent goal history queries return deterministic results for identical trace inputs.
4. Existing suite: `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_pain_first`
5. Existing suite: `cargo test -p worldwake-ai --test golden_offices golden_information_locality_for_political_facts`

### Invariants

1. Decision traces and their new helpers remain derived observability artifacts; they must not alter planner or authoritative behavior.
2. The richer query surface must answer common "why did/didn't this happen?" questions more directly without introducing duplicate truth paths or scenario-specific hacks.

## Test Plan

### New/Modified Tests

1. `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs` — add focused tests for derived goal-status and goal-history helpers.
2. `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs` — replace one or more manual goal-history scans with the shared helper.
3. `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs` — replace one or more manual political goal-history scans with the shared helper.

### Commands

1. `cargo test -p worldwake-ai decision_trace::tests`
3. `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_pain_first`
4. `cargo test -p worldwake-ai --test golden_offices golden_information_locality_for_political_facts`
5. `cargo test -p worldwake-ai`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`
8. `scripts/verify.sh`

## Outcome

- Completion date: 2026-03-19
- What actually changed:
  - Added derived goal-history and goal-status helpers to `DecisionTraceSink` and `AgentDecisionTrace` in `crates/worldwake-ai/src/decision_trace.rs`.
  - Added focused helper tests covering omitted-before-generation, suppressed, zero-motive, ranked/selected, and snapshot-continuation metadata.
  - Updated `crates/worldwake-ai/tests/golden_emergent.rs` and `crates/worldwake-ai/tests/golden_offices.rs` to use the shared helper surface instead of hand-rolled trace scans.
- Deviations from original plan:
  - Did not expand the stored decision-trace schema or modify `agent_tick.rs`; reassessment showed the necessary observability facts were already recorded.
  - Did not modify `docs/golden-e2e-testing.md`; the existing guidance remained accurate once the helper surface was added.
- Verification results:
  - `cargo test -p worldwake-ai decision_trace::tests`
  - `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_pain_first`
  - `cargo test -p worldwake-ai --test golden_offices golden_information_locality_for_political_facts`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `scripts/verify.sh`
