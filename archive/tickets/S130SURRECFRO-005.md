# S130SURRECFRO-005: Decision-trace damping infrastructure

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `CandidateDampingReason` enum, `CandidateDampingEntry` struct, new `damped` field on `CandidateTrace`
**Deps**: `archive/tickets/S130SURRECFRO-002.md`, spec `archive/specs/S130-survey-records-frontier-disconfirmation.md` D11

## Problem

`CandidateTrace` currently exposes hard suppression as `pub suppressed: Vec<GoalKey>` (decision_trace.rs:338) — a flat list of goals that were not emitted to ranking. S130 introduces *soft damping* — candidates emitted to ranking but with reduced `motive_score` due to per-(place, hypothesis) negative survey records. This is a different lifecycle: emitted-but-down-weighted vs. not-emitted-at-all. Mixing the two in a single collection conflates audit paths in the trace renderer. This ticket adds a parallel `damped: Vec<CandidateDampingEntry>` collection alongside `suppressed` and updates all live construction sites.

## Assumption Reassessment (2026-05-02)

1. `CandidateTrace` lives in `crates/worldwake-ai/src/decision_trace.rs`; the new field lands beside `pub suppressed: Vec<GoalKey>`.
2. Live reassessment found constructor fallout beyond the drafted single-file count: `CandidateTrace { ... }` literals exist in `decision_trace.rs`, the production trace builder in `agent_tick/mod.rs`, `survival_forensics.rs`, the golden harness helper, the CLI observer test helper, and the visualizer trace-buffer test helper.
3. Trace rendering is owned by `decision_trace.rs`; the new damping format is rendered alongside existing planning diagnostics.
4. `CandidateDampingReason::SurveyMemoryNegative { place, hypothesis, recorded_tick, confidence }` is the only damping reason for this spec — future damping reasons (e.g., commitment-bias damping) would extend the enum non-exhaustively.
5. No existing focused/unit, runtime, or golden test specifically exercises `CandidateTrace.suppressed` for `ExploreLocation` damping — coverage of the new `damped` field is added in this ticket and expanded by tickets 006 (ranking populates it) and 009 (golden test asserts trace contents).
6. `HypothesisKind` (added in `archive/tickets/S130SURRECFRO-002.md`) is required for the damping payload — this ticket's compile depends on the archived 002 foundation types.

## Architecture Check

1. Parallel-field design preserves the lifecycle distinction: `suppressed` = candidates that never reached ranking (gates, vetoes, cooldowns); `damped` = candidates that reached ranking but had their score reduced. The two collections answer different audit questions and have different downstream consumers in trace rendering.
2. `CandidateDampingReason` is an enum so future damping reasons land cleanly without changing field shape on `CandidateDampingEntry`.
3. No backward-compat shim — the field is net-new; trace consumers are local to the file.
4. Per FND-29 (Debuggability): every soft damping must surface with reason, place, hypothesis, recorded_tick, and confidence — full provenance for "why this candidate's score was reduced," not just "score was lower."

## Verification Layers

1. `CandidateTrace::default()` produces an empty `damped` vec → focused unit test.
2. `CandidateDampingEntry` renders through the local trace formatter → focused unit test (renderer output contains place, hypothesis, recorded_tick, confidence in the expected format).
3. `CandidateDampingReason::SurveyMemoryNegative` exhaustive-match completeness — the renderer match arm covers all current variants → compile-time gate via exhaustive match (no `_` catch-all in the renderer).
4. Single-layer ticket — pure decision-trace surface; no SystemFn integration, no event-log emission, no world-state mutation. Ranking population (ticket 006) is the consumer.

## What to Change

### 1. New types in `decision_trace.rs`

Add to `crates/worldwake-ai/src/decision_trace.rs` (alongside existing diagnostic types):

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CandidateDampingReason {
    SurveyMemoryNegative {
        place: EntityId,
        hypothesis: HypothesisKind,
        recorded_tick: Tick,
        confidence: Permille,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CandidateDampingEntry {
    pub goal_key: GoalKey,
    pub reason: CandidateDampingReason,
}
```

### 2. Add `damped` field to `CandidateTrace`

In the existing `CandidateTrace` struct at `decision_trace.rs:320`, add:

```rust
pub struct CandidateTrace {
    // ... existing fields ...
    pub suppressed: Vec<GoalKey>,
    pub damped: Vec<CandidateDampingEntry>,
}
```

### 3. Update all live `CandidateTrace { ... }` construction sites

Sweep the workspace for `CandidateTrace {` and add an empty `damped` vector to every live constructor. Population is still deferred to ticket 006; this ticket only makes the carrier and renderer available.

### 4. Trace renderer extension

Extend the existing trace renderer (or formatter) to print damping entries in the format specified by spec D11:

```
ExploreLocation { target: <place-id>, hypothesis: MayContainCommodity { commodity: Apple } } damped by SurveyMemory: found=false at tick 312, confidence=850.
```

The renderer must use exhaustive matching on `CandidateDampingReason` so future variants are caught at compile time.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — new types, `CandidateTrace` field/default, renderer extension, focused tests, local construction sites)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — production trace constructor initializes `damped` empty until ticket 006)
- `crates/worldwake-ai/src/survival_forensics.rs` (modify — test helper constructor fallout)
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs` (modify — test helper constructor fallout)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — all-target test helper constructor fallout)
- `crates/worldwake-visualizer/src/trace_buffers.rs` (modify — all-target test helper constructor fallout)
- `archive/specs/S130-survey-records-frontier-disconfirmation.md` (truth-sync D11 constructor/format wording)

## Out of Scope

- Populating `damped` from ranking (ticket 006 — wraps `exploration_motive` and pushes `CandidateDampingEntry` instances)
- Asserting damping content in goldens (ticket 009)
- Future damping reasons beyond `SurveyMemoryNegative` — explicitly out of scope; the enum is extensible but no other variants land in this spec

## Acceptance Criteria

### Tests That Must Pass

1. New: `candidate_trace_default_has_empty_damped_vec`.
2. New: `candidate_damping_entry_renders_survey_memory_negative_with_full_provenance` — asserts the renderer string contains place, hypothesis, recorded_tick, confidence.
3. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. `CandidateTrace.suppressed` and `CandidateTrace.damped` are disjoint by lifecycle — a goal that was suppressed (not emitted) will never appear in `damped` (only emitted goals can be damped). This invariant is enforced by ranking-call ordering in ticket 006.
2. The trace renderer matches `CandidateDampingReason` exhaustively — adding a new variant in a future spec is a compile-time-required update.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` (`#[cfg(test)]` block) — 2 new unit tests covering the default `damped` vec and the renderer output format.

### Commands

1. `cargo test -p worldwake-ai decision_trace`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-02.

- Added `CandidateDampingReason::SurveyMemoryNegative`, `CandidateDampingEntry`, and `CandidateTrace.damped`.
- Derived `Default` for `CandidateTrace`; `CandidateTrace::default()` now produces an empty `damped` vector.
- Extended the decision trace renderer with an exhaustive `CandidateDampingReason` match and a focused formatter for survey-memory damping.
- Updated all live `CandidateTrace` constructors to initialize `damped` empty until ranking population lands in ticket 006.

## Deviations

- Reassessment disproved the drafted "24 sites in `decision_trace.rs` only" constructor scope. The live shared trace struct also has constructors in `agent_tick`, `survival_forensics`, golden harness helpers, CLI observer tests, and visualizer trace-buffer tests.
- The renderer formats the place as the live `EntityId`; `decision_trace.rs` does not have a place-name registry, so the ticket/spec example was truth-synced from a named place to a place id.
- This ticket remains staged infrastructure only: ranking still does not populate `damped`; ticket 006 owns that runtime write.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib decision_trace -- --list` (resolved the focused selector; 60 matching tests listed).
- Passed `cargo test -p worldwake-ai --lib decision_trace`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
