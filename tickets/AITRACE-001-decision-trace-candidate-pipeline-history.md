# AITRACE-001: Expose Candidate-Pipeline History And Absence Reasons In Decision Traces

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — AI decision trace model, sink/query helpers, focused runtime tests, golden helper consumers
**Deps**: `crates/worldwake-ai/src/decision_trace.rs`, `crates/worldwake-ai/src/agent_tick.rs`, `crates/worldwake-ai/tests/golden_*`, `docs/golden-e2e-testing.md`, `docs/FOUNDATIONS.md`

## Problem

The current decision trace system is already useful, but it is still harder than it should be to answer architecture-level questions such as:
- Did a goal never generate, or did it generate and get suppressed?
- Was a goal present but zero-motive filtered?
- Was there no fresh planning pass because the agent continued an existing commitment?
- On which ticks did a given goal appear, disappear, or lose the ranking race?

Those are exactly the questions Principle 27 says the simulation must answer cleanly. Today the information is partially present but spread across raw per-tick traces, which forces ad hoc test code and manual inference.

This ticket extends the trace architecture so the candidate pipeline and absence reasons are first-class, queryable, and stable for debugging and tests.

## Assumption Reassessment (2026-03-19)

1. Existing decision traces already expose `DecisionOutcome::Planning(p)` with candidate and planning information, and goldens use them directly in `crates/worldwake-ai/tests/golden_emergent.rs` and `crates/worldwake-ai/tests/golden_offices.rs` for `ClaimOffice` presence/absence checks.
2. Existing sink helpers support point lookup and per-agent iteration (`trace_at`, `traces_for`), but there is no higher-level query API for goal-presence history, candidate-pipeline summaries, or explicit absence reasons. Tests currently hand-roll scans across ticks.
3. Existing docs in `docs/golden-e2e-testing.md` already tell authors to prefer decision traces for AI reasoning questions, but they do not yet have a stronger, more semantic decision-trace API to lean on.
4. This is a cross-layer AI observability ticket. It touches trace model code and test/query helpers, but it must not change authoritative world behavior or planner decisions. Full action registries are not inherently required for the focused trace model work; however, golden/runtime regression coverage should continue to run in the real AI harness where political and care actions exist.
5. The ticket is not about weakening heuristics or bypassing planner filters. It adds inspectable substrate around existing reasoning so tests can distinguish `not generated`, `suppressed`, `zero motive`, `outranked`, and `no fresh planning pass` without inference hacks.
6. Mismatch + correction: current traceability is good enough to debug manually but not yet comprehensive enough to answer common "why not?" questions directly. The corrected scope is to enrich trace data and sink queries, not to bolt on ad hoc scenario-specific helpers.

## Architecture Check

1. The clean design is to enrich the canonical decision-trace model and sink queries, not to scatter custom search helpers through individual golden files. Observability belongs in the trace subsystem because it is a cross-cutting architectural product feature.
2. The trace additions must remain derived views of AI reasoning, never a second source of truth. They should reflect existing candidate-generation, suppression, ranking, and selection phases without changing planner behavior, preserving Principle 25.
3. No backwards-compatibility aliasing or parallel trace schemas should be introduced. Extend the current trace model directly and update existing readers/tests to the new shape where needed.

## Verification Layers

1. candidate presence / suppression / zero-motive / ranking visibility is recorded per planning tick -> focused decision-trace unit/runtime tests in `crates/worldwake-ai/src/decision_trace.rs` and `crates/worldwake-ai/src/agent_tick.rs`
2. absence reasons for a goal are queryable without manual ad hoc trace scans -> sink/query helper tests
3. existing golden scenarios can use the richer trace surface without changing world behavior -> golden regressions in `crates/worldwake-ai/tests/golden_emergent.rs` and `crates/worldwake-ai/tests/golden_offices.rs`
4. authoritative outcomes remain unchanged -> existing crate/workspace suites

## What to Change

### 1. Extend the decision-trace model

Add first-class trace fields or summary structs that make the candidate pipeline explicit for a planning tick:
- generated goals
- suppressed goals
- zero-motive goals
- ranked goals
- selected goal / selected plan source
- explicit "no fresh planning" continuation reason where applicable

The design should make it possible to answer "why was goal X absent?" without manually inferring from multiple unrelated fields.

### 2. Add sink/query helpers for scenario and test use

Add stable helper APIs for common observability queries, such as:
- per-agent goal presence history across ticks
- per-tick absence reason lookup for a specific goal
- compact candidate-pipeline summaries suitable for test assertions and debugging dumps

These helpers should sit with the decision trace sink/query surface, not inside individual goldens.

### 3. Add focused tests and upgrade a small number of goldens

Add focused tests that prove:
- suppressed and zero-motive goals are distinguishable
- no-fresh-planning continuation is distinguishable from true goal absence
- the new helper APIs return deterministic histories for identical runs

Then update a small number of existing golden tests to use the stronger query surface where it materially improves clarity, without rewriting unrelated tests.

## Files to Touch

- `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs` (modify)
- `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs` (modify)
- `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md` (modify if the new query surface changes recommended usage)

## Out of Scope

- Any change to candidate generation, ranking math, suppression policy, or authoritative action semantics
- Scenario-specific one-off helper functions in golden files that bypass the canonical trace sink API
- A new standalone debugging subsystem outside the current decision trace architecture

## Acceptance Criteria

### Tests That Must Pass

1. A focused trace test proves a goal can be distinguished as generated, suppressed, zero-motive, or outranked.
2. A focused trace test proves "no fresh planning pass / continued prior commitment" is distinguishable from true goal absence.
3. A sink/helper test proves per-agent goal history queries return deterministic results for identical runs.
4. Existing suite: `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_pain_first`
5. Existing suite: `cargo test -p worldwake-ai --test golden_offices golden_information_locality_for_political_facts`

### Invariants

1. Decision traces remain derived observability artifacts; they must not alter planner or authoritative behavior.
2. The richer trace surface must answer "why did/didn't this happen?" more directly without introducing duplicate truth paths or scenario-specific hacks.

## Test Plan

### New/Modified Tests

1. `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs` — add focused tests for candidate-pipeline summaries and absence-reason encoding.
2. `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs` — add runtime-focused tests that exercise the richer trace surface through real planning ticks.
3. `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs` — upgrade one or more AI-reasoning assertions to the stronger trace helpers.
4. `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs` — upgrade one or more political candidate-presence/absence assertions to the stronger trace helpers.

### Commands

1. `cargo test -p worldwake-ai --lib decision_trace::tests`
2. `cargo test -p worldwake-ai --lib agent_tick::tests`
3. `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_pain_first`
4. `cargo test -p worldwake-ai --test golden_offices golden_information_locality_for_political_facts`
5. `cargo test -p worldwake-ai`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`
8. `scripts/verify.sh`
