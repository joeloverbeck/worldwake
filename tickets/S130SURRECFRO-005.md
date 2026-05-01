# S130SURRECFRO-005: Decision-trace damping infrastructure

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `CandidateDampingReason` enum, `CandidateDampingEntry` struct, new `damped` field on `CandidateTrace`
**Deps**: `archive/tickets/S130SURRECFRO-002.md`, spec `specs/S130-survey-records-frontier-disconfirmation.md` D11

## Problem

`CandidateTrace` currently exposes hard suppression as `pub suppressed: Vec<GoalKey>` (decision_trace.rs:338) — a flat list of goals that were not emitted to ranking. S130 introduces *soft damping* — candidates emitted to ranking but with reduced `motive_score` due to per-(place, hypothesis) negative survey records. This is a different lifecycle: emitted-but-down-weighted vs. not-emitted-at-all. Mixing the two in a single collection conflates audit paths in the trace renderer. This ticket adds a parallel `damped: Vec<CandidateDampingEntry>` collection alongside `suppressed` and updates all 24 construction sites.

## Assumption Reassessment (2026-05-02)

1. `CandidateTrace` lives at `crates/worldwake-ai/src/decision_trace.rs:320` with `pub suppressed: Vec<GoalKey>` at line 338. There are 24 `CandidateTrace { ... }` construction sites in the same file — all in test scaffolding (`#[cfg(test)]` blocks and trace-builder helpers).
2. The 24 sites use explicit field listing (no spread syntax). Each one currently writes `suppressed: vec![],` (or similar) — adding a new mandatory field requires touching every site to add `damped: vec![],`.
3. Trace renderer for `CandidateTrace.suppressed` lives in `decision_trace.rs` itself (no external consumer). The new damping format is rendered alongside.
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
2. `CandidateDampingEntry` round-trips through trace rendering → focused unit test (renderer output contains place, hypothesis, recorded_tick, confidence in the expected format).
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

### 3. Update all 24 `CandidateTrace { ... }` construction sites

Sweep `crates/worldwake-ai/src/decision_trace.rs` for `CandidateTrace {` (24 sites). Add `damped: vec![],` to each. Discovery: `grep -n "CandidateTrace {" crates/worldwake-ai/src/decision_trace.rs`. All 24 sites are in `#[cfg(test)]` blocks or test-fixture helpers; no production-runtime construction sites exist (population is via ranking-arm calls in ticket 006).

### 4. Trace renderer extension

Extend the existing trace renderer (or formatter) to print damping entries in the format specified by spec D11:

```
ExploreLocation { target: Hillside Shelter, hypothesis: MayContainCommodity { Apple } } damped by SurveyMemory: found=false at tick 312, confidence=850.
```

The renderer must use exhaustive matching on `CandidateDampingReason` so future variants are caught at compile time.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — new types, `CandidateTrace` field, 24 construction site updates, renderer extension)

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
