# E16DPOLPLAN-030: Decision-trace diagnostics for omitted political candidates

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — decision-trace diagnostic surface for candidate omission reasons
**Deps**: E16DPOLPLAN-015, S08AIDECTRA-001, S08AIDECTRA-002

## Problem

While implementing Force-law office coverage, it was easy to prove positive behavior (`generated`, `suppressed`, `ranked`, committed actions), but harder to explain negative AI behavior such as:
- a political goal was never emitted because the office used the wrong succession law
- a candidate was blocked by a hard eligibility/law gate before emission

Current decision traces expose:
- `generated`
- `suppressed`
- `zero_motive`
- plan-search attempts

They do not expose why a candidate was omitted before `generated`. That makes “why didn’t the agent even consider X?” harder to diagnose than “why did it consider X and then suppress or reject it?”

## Assumption Reassessment (2026-03-18)

1. `CandidateTrace` in `crates/worldwake-ai/src/decision_trace.rs` currently records `generated`, `ranked`, `suppressed`, and `zero_motive` only — confirmed.
2. The read phase in `crates/worldwake-ai/src/agent_tick.rs` only threads `generated_keys`, `suppressed`, and `zero_motive` into traces — confirmed from `ReadPhaseResult` and the `DecisionOutcome::Planning` construction path.
3. `emit_political_candidates` in `crates/worldwake-ai/src/candidate_generation.rs` has multiple hard pre-emission gates (`entity_kind`, `office_data`, `succession_law`, visible vacancy, eligibility/candidate filtering downstream) — confirmed.
4. Existing tests already prove omission by absence for key political gates:
   - `candidate_generation::tests::political_candidates_require_visible_vacancy_and_skip_existing_declaration`
   - `candidate_generation::tests::political_candidates_skip_force_law_offices`
   - `agent_tick` trace coverage that force-law offices never enter generated candidates or plan search
   Corrected scope: this ticket is not adding the first force/non-vacancy omission coverage; it is making those negative cases explicitly diagnosable in decision traces.
5. Existing trace guidance in `AGENTS.md` and the current trace model are stronger for positive decisions than for omitted candidates — confirmed.

## Architecture Check

1. Adding explicit omission diagnostics is cleaner than relying on code inspection when a candidate never appears. It improves explainability without changing world behavior.
2. The trace surface should record hard-gate reasons at the point of omission rather than inferring them later from missing data.
3. The cleanest shape is a narrow typed omission record for political generation, not a generic free-form string bag. This keeps the trace schema extensible without pretending all candidate families share the same omission semantics yet.
4. No backwards-compatibility aliasing is needed. This is an additive trace-schema improvement inside the current decision-trace system.

## What to Change

### 1. Extend decision traces with omitted-candidate diagnostics

- Add a small typed diagnostic surface in `crates/worldwake-ai/src/decision_trace.rs` for political omissions, for example:
  - omitted goal family (`ClaimOffice` vs `SupportCandidateForOffice`)
  - typed omission reason enum
  - relevant entity ids (`office`, optional `candidate`)
- Extend `CandidateTrace` to carry omitted diagnostics alongside `generated`, `suppressed`, and `zero_motive`.

### 2. Thread omission diagnostics through the read phase

- Extend `ReadPhaseResult` in `crates/worldwake-ai/src/agent_tick.rs`.
- Capture omission diagnostics during candidate generation and include them in `DecisionOutcome::Planning`.

### 3. Start with political candidate omissions

- Implement omission reporting first for `emit_political_candidates` and its subpaths in `crates/worldwake-ai/src/candidate_generation.rs`.
- Minimum required reasons:
  - office uses `SuccessionLaw::Force`
  - office is not visibly vacant
  - actor/candidate not eligible
- Also capture the existing-declaration short-circuit if it stays cheap and local, because current tests already exercise that gate in `candidate_generation`.
- If the implementation introduces a reusable omission-reporting helper, keep it narrow and local; do not generalize the entire candidate-generation subsystem unless the code stays clearly simpler.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `AGENTS.md` (modify only if the trace guidance needs a follow-up note after implementation)

## Out of Scope

- Changing candidate-generation semantics
- Extending omission diagnostics to every non-political goal family in the same ticket
- Action-trace schema changes
- UI/CLI trace rendering improvements outside existing debug dumps

## Acceptance Criteria

### Tests That Must Pass

1. Existing absence-based political omission tests remain valid and unchanged in behavior.
2. New decision-trace unit/integration coverage proves omitted political candidates report explicit reasons for at least the Force-law and non-vacant cases.
3. Existing decision-trace tests continue to pass.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Omitted-candidate diagnostics reflect hard pre-emission gates, not post-hoc guesses.
2. Positive trace behavior (`generated`, `suppressed`, `zero_motive`) remains unchanged.
3. The trace system becomes more informative for “why didn’t the agent consider X?” without changing planning results.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — extend existing political omission tests to assert typed omission diagnostics for force-law, non-vacant, and eligibility/declaration gates where appropriate.
2. `crates/worldwake-ai/src/agent_tick.rs` or `crates/worldwake-ai/src/decision_trace.rs` — trace plumbing test proving omission reasons appear in `DecisionOutcome::Planning`.

### Commands

1. `cargo test -p worldwake-ai candidate_generation::tests::political_candidates_skip_force_law_offices`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-18
- What actually changed:
  - Added typed political omission diagnostics to `CandidateTrace` rather than a generic string-based omission channel.
  - Threaded political omission diagnostics through candidate generation and the `agent_tick` read phase without changing planning behavior.
  - Extended existing political omission tests to assert explicit omission reasons and added focused eligibility diagnostics coverage.
- Deviations from original plan:
  - Kept the schema intentionally narrow to political omissions only; did not introduce a generalized pre-emission diagnostic framework for every goal family.
  - Reused and strengthened existing force-law/non-vacancy tests instead of treating those cases as uncovered behavior.
- Verification results:
  - `cargo test -p worldwake-ai political_candidates_skip_force_law_offices`
  - `cargo test -p worldwake-ai political_candidates_require_visible_vacancy_and_skip_existing_declaration`
  - `cargo test -p worldwake-ai political_candidates_record_ineligible_actor_and_support_target_omissions`
  - `cargo test -p worldwake-ai trace_force_law_office_skips_political_candidates_and_planning`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace`
