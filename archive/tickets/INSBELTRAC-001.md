# INSBELTRAC-001: Trace Institutional Belief Acquisition From Record Consultation

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — add first-class authoritative traceability for institutional belief acquisition on `consult_record`
**Deps**: `docs/FOUNDATIONS.md`; `docs/golden-e2e-testing.md`; `docs/precision-rules.md`; `specs/S19-institutional-record-consultation-golden-suites.md`; archived traceability guidance [`archive/tickets/TRACEVIEW-001-cross-layer-timeline-for-emergent-debugging.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/TRACEVIEW-001-cross-layer-timeline-for-emergent-debugging.md)

## Problem

The live code can already prove that a political plan selected `ConsultRecord`, targeted a specific record entity, committed `consult_record`, and then later committed `declare_support`. What it still cannot expose cleanly is the authoritative institutional belief transition that happened at consult commit time. In particular, current traces do not directly answer:

- which institutional belief key changed
- what the effective read state changed from and to
- which consulted record entry or entries caused that change
- whether the consult produced no effective read-state change at all

That gap weakens debuggability for information-locality scenarios. The clean fix is first-class authoritative traceability for belief acquisition, not ad-hoc logging and not more downstream event-log inference.

## Assumption Reassessment (2026-03-22)

1. Existing adjacent coverage is real and must not be restated as missing:
   - decision selection via `DecisionTraceSink` in [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)
   - action lifecycle via `ActionTraceSink` in [`crates/worldwake-sim/src/action_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs)
   - focused `consult_record` authority tests in [`crates/worldwake-systems/src/consult_record_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/consult_record_actions.rs)
   - golden consult-record E2E coverage in [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs), including `golden_remote_record_consultation_political_action`
2. The live `GoalKind` under the existing S19 remote-record golden is `ClaimOffice`, and the selected-plan trace already exposes the concrete `PlannerOpKind::ConsultRecord` step target via `SelectedPlanTrace.steps[*].targets` in [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs). The current golden already asserts that the consult step targets the remote record entity.
3. Because the selected-plan trace already carries the chosen `ConsultRecord` target, this ticket does not need new planner provenance fields just to answer “which record was chosen?” for the selected branch. Adding more planner-specific source-selection fields now would duplicate live data and broaden the AI trace surface without a demonstrated coverage gap.
4. The real missing layer is authoritative mutation visibility. `consult_record` projects record entries into the actor belief store in [`crates/worldwake-systems/src/consult_record_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/consult_record_actions.rs), but current traces do not expose which institutional read changed at commit time.
5. `ActionTraceDetail` is payload-derived and currently only carries `Tell` semantics in [`crates/worldwake-sim/src/action_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs). `ConsultRecord` commits return `CommitOutcome::empty()`, so the action trace today cannot explain what was learned.
6. The current belief-store mutation path always appends institutional claims through `WorldTxn::project_institutional_belief()` and `AgentBeliefStore::record_institutional_belief()` in [`crates/worldwake-core/src/world_txn.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs) and [`crates/worldwake-core/src/belief.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs). Effective read state is derived later through helpers such as `believed_office_holder()`, so a trace surface must report derived before/after read summaries rather than raw “entry appended” facts if it wants to answer the architectural question cleanly.
7. This is mixed-layer traceability work, not gameplay behavior work. It must not change planner legality, political behavior, institutional belief semantics, or office succession. The live remote-record golden already proves the existing behavior path.
8. Foundation alignment remains strong:
   - Principle 7: record consultation is an explicit local information carrier
   - Principle 14: unknown and contradiction are first-class, so `Unknown -> Certain` and `Certain -> Conflicted` transitions are architecturally meaningful
   - Principle 16: records are world state
   - Principle 24: systems interact through state; trace output must remain observational only
9. Mismatch + correction: the current ticket narrative overstated two missing pieces:
   - S19 consult-record golden coverage is not missing; it already exists
   - selected prerequisite source for the chosen branch is not missing from the selected plan; the missing substrate is the authoritative belief-transition trace at commit time

## Architecture Check

1. The cleaner design is a dedicated institutional knowledge-acquisition trace surface in `worldwake-sim`, emitted from the authoritative action-execution boundary after a consult commit. That keeps belief-mutation observability separate from AI plan selection and separate from generic action lifecycle events.
2. Extending decision traces further for this ticket would be a weaker architecture than the current one because it would duplicate source-target information that `SelectedPlanTrace.steps[*].targets` already provides, while still failing to answer the authoritative “what belief read changed?” question.
3. Overloading `ActionTraceDetail` with post-commit belief semantics would also be weaker than a dedicated sink because `ActionTraceDetail` is payload-derived and action-start-oriented today. Belief acquisition is a commit-time semantic effect that deserves its own append-only contract.
4. No backward-compatibility aliasing or duplicate temporary trace paths should be introduced. Add one authoritative trace contract and wire it cleanly.

## Verification Layers

1. Selected political branch targeted a specific remote record -> existing decision trace selected-plan steps in [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)
2. `consult_record` committed before `declare_support` -> existing action trace ordering in [`crates/worldwake-sim/src/action_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs)
3. `consult_record` changed an institutional read from one semantic state to another -> new institutional knowledge-acquisition trace surface
4. `consult_record` produced no effective institutional read change -> focused knowledge-trace coverage asserting no emitted acquisition event
5. Political outcome still resolves unchanged after consultation -> existing authoritative world state / relation assertions in the golden

## What to Change

### 1. Add a first-class institutional knowledge trace sink

Introduce a dedicated opt-in trace sink in `worldwake-sim` for authoritative institutional knowledge acquisition events. Each event should report:

- actor
- tick and in-tick sequence
- acquisition source kind (`RecordConsultation` for this ticket)
- consulted record entity
- one or more affected institutional belief transitions

Each transition should report:

- institutional belief key
- consulted record entry ids that contributed to that key in this commit
- previous effective read summary
- new effective read summary

The read summary must be semantic, not raw storage internals. It should distinguish `Unknown`, `Certain(...)`, and `Conflicted(...)` for the current institutional key families rather than just echoing “a belief vector changed.”

### 2. Emit trace events from the authoritative consult commit boundary

Wire the sink from `step_tick` / active-action progression so that after a `consult_record` commit it compares the actor’s pre-commit and post-commit institutional read state for the consulted entries and emits an event only for effective read-state changes.

This ticket should not change the `consult_record` gameplay semantics. It should only observe and report the state-mediated transition that already happened.

### 3. Expose the sink in the golden harness

Extend the golden harness so existing consult-record goldens can enable and inspect the new sink without ad-hoc instrumentation.

### 4. Add focused and golden coverage

Add focused tests for:

- office-holder `Unknown -> Certain(None)` acquisition on consult commit
- same-key multi-entry transitions if the consulted entries lawfully produce conflict
- no emitted acquisition event when consulted entries append raw claims without changing the effective read

Update the existing remote-record political golden so it asserts that the new trace surface records the office-holder acquisition from the consulted record while preserving the existing decision-trace, action-trace, and authoritative-state assertions.

## Files to Touch

- `crates/worldwake-sim/src/` (new institutional knowledge trace sink module plus `tick_step` wiring)
- `crates/worldwake-sim/src/lib.rs` (export the new trace surface)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (expose the new sink cleanly)
- `crates/worldwake-ai/tests/golden_offices.rs` (extend the existing remote-record golden)
- `crates/worldwake-sim/src/tick_step.rs` and/or nearby action-progression plumbing (emit trace events at authoritative commit)

## Out of Scope

- Changing planner legality or political behavior
- Changing institutional belief semantics or belief-store storage rules
- Adding new planner provenance fields for selected consult-record source choice
- Replacing existing action-trace or decision-trace responsibilities
- Ad-hoc `eprintln!` or test-only instrumentation

## Acceptance Criteria

### Tests That Must Pass

1. New focused institutional knowledge-trace tests
2. Existing focused `consult_record` tests in `worldwake-systems`
3. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Traceability remains observational only. It does not change authoritative world behavior, belief semantics, or planning outcomes.
2. Knowledge-acquisition traces describe local, concrete carriers and belief transitions, never global-truth shortcuts.
3. When tracing is disabled, runtime behavior remains unchanged.
4. The selected-plan decision trace remains the proof surface for “which consult-record branch was chosen”; this ticket must not duplicate that data with redundant AI-only fields.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/tick_step.rs` focused institutional knowledge-trace test(s) — prove consult-record commits emit semantic office-holder belief transitions and suppress no-op effective reads.
2. `crates/worldwake-ai/tests/golden_offices.rs::golden_remote_record_consultation_political_action` — prove the new sink is usable in the live S19 remote-record path alongside the existing decision/action/state assertions.

### Commands

1. `cargo test -p worldwake-sim knowledge_trace`
2. `cargo test -p worldwake-systems consult_record`
3. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-03-22
- What changed:
  - added `InstitutionalKnowledgeTraceSink` in `worldwake-sim` for authoritative institutional belief-acquisition events
  - wired consult-record commit observation through `tick_step` so record consultation emits semantic read transitions only when the effective institutional read actually changes
  - exposed the new sink in the golden harness
  - extended the existing S19 remote-record golden to assert the authoritative office-holder acquisition trace
  - added focused consult-record trace coverage for `Unknown -> Certain(None)`, conflicted multi-entry acquisition, and no-op effective reads
- Deviations from original plan:
  - no new decision-trace planner provenance fields were added because live `SelectedPlanTrace.steps[*].targets` already exposes the chosen `ConsultRecord` target for the selected branch
  - no new consult-record golden was created because the live repo already had `golden_remote_record_consultation_political_action`; the work was folded into that existing scenario instead
- Verification results:
  - `cargo test -p worldwake-sim institutional_knowledge_trace -- --nocapture`
  - `cargo test -p worldwake-systems consult_record -- --nocapture`
  - `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
