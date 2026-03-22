# INSBELTRAC-001: Trace Institutional Belief Acquisition and Prerequisite Source Selection

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — add first-class traceability for institutional knowledge acquisition and prerequisite-source selection
**Deps**: `docs/FOUNDATIONS.md`; `docs/golden-e2e-testing.md`; archived traceability guidance [`archive/tickets/TRACEVIEW-001-cross-layer-timeline-for-emergent-debugging.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/TRACEVIEW-001-cross-layer-timeline-for-emergent-debugging.md); archived motivating example [`archive/tickets/completed/S19INSRECCON-002.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S19INSRECCON-002.md)

## Problem

Current traces can prove that an agent selected a plan containing `ConsultRecord`, committed `consult_record`, and later committed `declare_support`, but they do not expose the institutional knowledge transition itself with enough structure to explain the behavior architecturally. In particular, the traces do not currently answer these questions directly:

- which record or other prerequisite source satisfied the unknown institutional prerequisite
- which institutional belief key changed
- what the relevant read state changed from and to
- which source was selected when multiple lawful consultable records could satisfy the same prerequisite

That gap weakens debuggability for information-locality scenarios. The clean fix is not ad-hoc logging inside goldens; it is first-class traceability for knowledge acquisition that remains zero-cost when disabled.

## Assumption Reassessment (2026-03-22)

1. Existing trace sinks already cover adjacent layers:
   - decision selection via `DecisionTraceSink` in [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)
   - action lifecycle via `ActionTraceSink` in [`crates/worldwake-sim/src/action_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs)
   These sinks prove plan shape and action ordering, but not the institutional belief transition itself.
2. `consult_record` authoritative mutation already happens in [`crates/worldwake-systems/src/consult_record_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/consult_record_actions.rs). That is the correct authoritative boundary to capture knowledge-acquisition facts because it is where record entries are projected into the agent belief store.
3. The current action trace detail model is payload-oriented and too thin for this need. [`crates/worldwake-sim/src/action_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs) supports `Tell` payload detail, but `ConsultRecord` currently contributes no semantic post-commit detail about what was learned.
4. The current decision trace selected-plan surface already includes step summaries and selected-plan provenance, but it does not expose why one consultable prerequisite source won over another when multiple records could satisfy the same institutional gap. That missing planner provenance matters to future debugging and aligns with `docs/precision-rules.md` section 15 on traceability escalation.
5. This ticket is traceability work, not a gameplay behavior change. It must not alter authoritative rules, planner legality, or belief semantics; it should only expose structured facts about those existing transitions.
6. The information under trace is architecturally grounded and local, which aligns with [`docs/FOUNDATIONS.md`](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md):
   - Principle 7: information must travel via explicit carriers
   - Principle 14: unknown, uncertainty, and contradiction are first-class
   - Principle 16: records are world state
   - Principle 24: systems interact through state, not direct forcing
   A trace sink that reports these state-mediated transitions strengthens legibility without creating new causal shortcuts.
7. No existing trace sink should be overloaded into a grab bag. The clean architecture is either:
   - a dedicated knowledge/institutional trace sink for authoritative belief-acquisition events, plus a small decision-trace provenance addition for prerequisite-source choice, or
   - an equally coherent extension that preserves clear layer boundaries.
   This ticket should decide and implement one clean contract, not scatter one-off fields across unrelated trace structs.
8. Verification is mixed-layer:
   - planner/source-selection provenance belongs to decision traces
   - belief acquisition belongs to an authoritative knowledge-acquisition trace surface
   - end-to-end use in a golden should still rely on authoritative state and action ordering where appropriate
9. Mismatch + correction: the missing substrate is not “more event log assertions” or “broader action traces.” The real architectural gap is dedicated visibility into institutional belief transitions and prerequisite-source selection.

## Architecture Check

1. The right design is first-class traceability for knowledge acquisition, not ad-hoc debug output in tests. This keeps the engine clean, keeps authority and observability separate, and makes future record-, report-, and rumor-based debugging extensible.
2. The trace contract should remain opt-in and zero-cost when disabled, matching the current sink pattern. No backward-compatibility aliasing, alternate debug channels, or duplicate “temporary” trace paths should be introduced.

## Verification Layers

1. Planner selected a specific prerequisite source for an institutional gap -> decision trace provenance on the selected plan or plan attempt.
2. `consult_record` projected institutional knowledge into the actor belief store -> dedicated knowledge-acquisition trace surface at authoritative commit.
3. The knowledge-acquisition trace reports the exact institutional key and read-state transition (`Unknown -> Certain(None)`, etc.) -> focused authoritative trace test.
4. Existing action lifecycle ordering remains distinct (`consult_record` before `declare_support`) -> action trace.
5. Golden E2E adoption should prove the feature is usable without replacing stronger authoritative or action-order assertions -> one targeted golden update or new golden assertion in an existing consult-record scenario.

## What to Change

### 1. Add a first-class knowledge-acquisition trace contract

Introduce a dedicated trace surface for authoritative belief acquisition, likely in `worldwake-sim`, with deterministic append-only events such as:

- actor
- tick and in-tick sequence
- acquisition source kind (`RecordConsultation`, with room for later `Tell`, `Report`, `Witness`, etc.)
- source entity/place identifiers
- affected institutional belief keys
- previous and new read-state summaries for each affected key

The contract must be extensible beyond office-holder vacancy facts without overfitting to one scenario.

### 2. Wire consult-record authoritative commits into that trace

Update the `consult_record` commit path so that when it projects record entries into the actor’s institutional belief store, it also emits structured knowledge-acquisition trace events when tracing is enabled. This must capture what actually changed, not just what entries were present on the record.

### 3. Expose prerequisite-source provenance in decision traces

Extend the selected-plan or plan-attempt trace surface to capture which prerequisite source satisfied a political/institutional knowledge gap when the planner selected a consult-record branch. The trace should answer “why this record/place?” in planner terms without embedding authoritative mutations into the decision layer.

### 4. Add focused and golden coverage

Add focused tests for:
- knowledge-acquisition trace emission on consult-record commit
- correct previous/new read-state summaries for office-holder beliefs
- no trace emission when consult-record produces no effective institutional change
- planner provenance when multiple consultable records exist and one is selected

Add or update one golden consult-record scenario so the new trace surface is exercised in a real end-to-end path without weakening stronger existing assertions.

## Files to Touch

- `crates/worldwake-sim/src/` (new or modified trace sink module plus wiring)
- `crates/worldwake-systems/src/consult_record_actions.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` and/or planner-trace plumbing (modify if required)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify to expose the new sink cleanly)
- `crates/worldwake-ai/tests/golden_offices.rs` or another consult-record golden (modify)

## Out of Scope

- Changing planner legality or political behavior
- Changing institutional belief semantics
- Adding ad-hoc `eprintln!` or test-only instrumentation
- Replacing existing action-trace or decision-trace responsibilities instead of complementing them

## Acceptance Criteria

### Tests That Must Pass

1. Focused trace test(s) covering consult-record knowledge-acquisition events
2. Focused planner trace test(s) covering prerequisite-source provenance
3. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Traceability must remain observational only. It must not change authoritative world behavior, belief semantics, or planning outcomes.
2. Knowledge-acquisition traces must describe local, concrete carriers and belief transitions, never global truth shortcuts.
3. The contract must be extensible beyond office-holder vacancy facts without introducing backward-compatibility aliases or one-off special cases.
4. When tracing is disabled, runtime behavior and cost should remain unchanged.

## Test Plan

### New/Modified Tests

1. Focused trace test in the consult-record authority layer — proves that a real consult-record commit emits a structured knowledge-acquisition event with source, key, and read-state transition.
2. Focused planner trace test — proves the selected prerequisite source is carried into decision-trace provenance when a consult-record branch is selected.
3. Modified consult-record golden — proves the new trace surface is usable in a full remote-record scenario without replacing authoritative-state or action-order assertions.

### Commands

1. `cargo test -p worldwake-systems consult_record`
2. `cargo test -p worldwake-ai decision_trace`
3. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`
