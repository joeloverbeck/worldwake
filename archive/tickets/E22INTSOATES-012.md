# E22INTSOATES-012: Emit diagnostic trace when violation detection skips due to missing ViolationDispositionProfile

**Status**: ✅ COMPLETED
**Priority**: LOW
**Effort**: Small
**Engine Changes**: Yes — candidate_generation.rs diagnostic emission, decision_trace.rs new omission variant
**Deps**: None

## Problem

`emit_expectation_violation_candidates` in `crates/worldwake-ai/src/candidate_generation.rs:2741` returns early with an empty list when the agent lacks a `ViolationDispositionProfile`. No diagnostic trace is emitted. When a golden test expects `ViolationKind::EntityMissing` in `ViolationMemory` and the component is absent, the only way to diagnose the silent skip is to read the source code. This violates Principle 29 (Debuggability Is a Product Feature).

Discovered during E22INTSOATES-003 implementation: the bandit arrived at Crossroads, found the target absent, but `ViolationMemory` remained empty. Decision traces showed zero violation-related candidates but gave no reason why. The root cause — missing `ViolationDispositionProfile` — required reading `candidate_generation.rs` to find the early return.

## Assumption Reassessment (2026-03-31)

1. `emit_expectation_violation_candidates` at `crates/worldwake-ai/src/candidate_generation.rs:2727` takes a `&mut CandidateGenerationDiagnostics` parameter but does not write to it on the early return paths (lines 2737 and 2742) — confirmed by code inspection.
2. `CandidateGenerationDiagnostics` at `crates/worldwake-ai/src/candidate_generation.rs:152` has `omitted_political`, `omitted_bandit`, and `omitted_social` Vec fields — no violation-detection omission field exists.
3. `CandidateTrace` at `crates/worldwake-ai/src/decision_trace.rs:293` mirrors the diagnostics structure with `omitted_political`, `omitted_bandit`, `omitted_social` — same gap.
4. The `dump_agent` formatting in `decision_trace.rs` has formatting functions for political, bandit, and social omissions — no violation-detection counterpart exists.
5. No existing ticket in `tickets/` covers this gap.
6. Single-layer ticket: candidate-generation diagnostics only. No cross-system or mixed-layer concerns.

## Architecture Check

1. Follows the established pattern: other candidate-generation early-return gates (political, bandit, social) already emit omission diagnostics into `CandidateGenerationDiagnostics`. This change extends the same pattern to the violation-detection gate. No new abstractions needed.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. Diagnostic emission on missing ViolationDispositionProfile → decision trace (`CandidateTrace.omitted_violation_detection` populated when profile absent)
2. No spurious emission when profile is present → existing golden tests that have ViolationDispositionProfile must not show the omission
3. Single-layer ticket: violation detection is entirely within candidate generation. No action-trace or event-log layers involved.

## What to Change

### 1. Add `ViolationDetectionOmission` diagnostic type in `crates/worldwake-ai/src/candidate_generation.rs`

- Define a struct (e.g., `ViolationDetectionOmission`) with a `reason` enum field: `MissingViolationDispositionProfile` and `AgentInTransit`.
- Add an `omitted_violation_detection: Vec<ViolationDetectionOmission>` field to `CandidateGenerationDiagnostics`.
- At lines 2737 and 2742, push an omission diagnostic before returning.

### 2. Mirror in `CandidateTrace` in `crates/worldwake-ai/src/decision_trace.rs`

- Add `omitted_violation_detection: Vec<ViolationDetectionOmission>` to `CandidateTrace`.
- Wire the field through from `CandidateGenerationDiagnostics` → `CandidateTrace` in the trace-building code.
- Add a formatting function for `dump_agent` output (following the `format_political_omission` / `format_bandit_omission` pattern).

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify)

## Out of Scope

- Adding `ViolationDispositionProfile` to `seed_agent` by default (that is a setup choice per test)
- Changing the early-return behavior itself (the gate is architecturally correct)
- Adding diagnostics to the perception system's `MismatchKind::EntityMissing` path (separate pipeline)

## Acceptance Criteria

### Tests That Must Pass

1. Focused unit test: agent without `ViolationDispositionProfile` produces `MissingViolationDispositionProfile` in diagnostics
2. Focused unit test: agent in transit (no `effective_place`) produces `AgentInTransit` in diagnostics
3. Focused unit test: agent with both profile and place does NOT produce violation-detection omission
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No behavioral change: the early returns remain; only diagnostic emission is added
2. Existing golden tests must not regress

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` (new unit tests) — verify omission diagnostics for both early-return paths
2. No golden test changes expected — this is diagnostic-only

### Commands

1. `cargo test -p worldwake-ai -- violation_detection_omission` (targeted new tests)
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-31
- **What changed**: Added `ViolationDetectionOmission` struct and `ViolationDetectionOmissionReason` enum (`MissingViolationDispositionProfile`, `AgentInTransit`) in `decision_trace.rs`. Added `omitted_violation_detection` field to `CandidateGenerationDiagnostics`, `ReadPhaseResult`, and `CandidateTrace`. Emitted diagnostics at both early-return paths in `emit_expectation_violation_candidates`. Added `dump_agent` formatting. Exported new types from `lib.rs`. Three focused unit tests added.
- **Deviations**: None. Implemented exactly as specified.
- **Verification**: `cargo test -p worldwake-ai` (954 tests pass), `cargo clippy --workspace` clean.
