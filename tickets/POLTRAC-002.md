# POLTRAC-002: Extend Political Trace with Timer-State and Counted Support Snapshots

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extend the authoritative politics trace schema and focused/golden assertions, without changing political behavior
**Deps**: [`archive/tickets/POLTRAC-001-political-system-trace-sink.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/POLTRAC-001-political-system-trace-sink.md), [`archive/tickets/completed/S19INSRECCON-003.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S19INSRECCON-003.md), `docs/FOUNDATIONS.md`, `crates/worldwake-sim/src/politics_trace.rs`, `crates/worldwake-systems/src/offices.rs`, `crates/worldwake-ai/tests/golden_harness/mod.rs`, `crates/worldwake-ai/tests/golden_offices.rs`

## Problem

`POLTRAC-001` added an authoritative politics trace sink, but the current trace still leaves a mixed-layer explanation gap for timed support-law races. It records vacancy phase, support declarations, and final outcome, yet it does not directly expose the full evaluation snapshot that answers questions like:

- why did evaluation happen on this tick rather than an earlier or later tick?
- what timer state matured the office into resolvable status?
- which declarations were counted toward the winning support total?
- why did a late claimant lose without any declaration being counted?

The gap matters because Scenario 34-style political races are architecturally about the interaction between authoritative consultation duration, declaration timing, and support-law evaluation. Today that explanation still has to be reconstructed from code plus multiple trace surfaces.

## Assumption Reassessment (2026-03-22)

1. [`crates/worldwake-sim/src/politics_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/politics_trace.rs) already records `OfficeSuccessionTrace` with:
   - `holder_before`
   - `vacancy_since_before`
   - `availability_phase`
   - `outcome`
   - `support_declarations`
   - `force_candidates`
   This proves the first-round trace sink exists, but it does not yet provide a full timer-state snapshot for install/reset outcomes.
2. The current `OfficeSuccessionOutcome::WaitingForTimer` carries timer arithmetic, but `SupportInstalled` and `SupportReset*` outcomes do not. Consumers must reconstruct “why now?” by combining `vacancy_since_before`, current tick, and `succession_period_ticks` from separate data sources.
3. [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs) already computes the authoritative inputs needed for richer tracing during support-law resolution:
   - vacancy timing
   - candidate eligibility filtering
   - support counts
   The missing piece is trace schema coverage, not authoritative semantics.
4. [`archive/tickets/completed/S19INSRECCON-003.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S19INSRECCON-003.md) confirmed the current trace stack was sufficient to prove the new behavior, but not sufficient to explain the decisive support-law timing boundary directly. The current proof required code-level reasoning plus action/decision/world assertions.
5. This is a mixed-layer traceability ticket, not an AI-planner behavior ticket. The authoritative layer under change is political system tracing in `offices.rs`; the AI layer remains relevant only for one real golden scenario that consumes the richer trace.
6. Ordering is part of the contract here, but the decisive ordering is mixed-layer:
   - action lifecycle ordering for `consult_record` / `declare_support`
   - authoritative world-state ordering for support-law evaluation and office-holder mutation
   The ticket should not collapse those into one generic “politics happened later” assertion.
7. No heuristic is being removed. This ticket adds missing explanation substrate to the trace surface so future golden/debug work does not need ad hoc reconstruction.
8. The closure boundary under explanation is support-law succession resolution and subsequent office-holder mutation. The relevant authoritative symbols are [`resolve_support_succession()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs#L214) and the trace schema in [`crates/worldwake-sim/src/politics_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/politics_trace.rs).
9. Scenario isolation for the real golden consumer remains the same as Scenario 34: co-located sated agents, no unrelated branches. The intended invariant is support-law timing under knowledge asymmetry, not travel or self-care competition.
10. Mismatch + correction: `POLTRAC-001` solved the first missing trace surface, but not the full support-law evaluation snapshot needed for clean explanation of timed races. This ticket extends that same architecture rather than inventing a parallel trace system.
11. The clean design consistent with [`docs/FOUNDATIONS.md`](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md) Principles 1, 3, 16, and 24 is to record more concrete authoritative state at evaluation time:
   - timer state
   - counted support tallies
   - filtered declarations and why they were excluded
   not to add narrative “winner reason” strings or planner-facing convenience abstractions.

## Architecture Check

1. Extending the existing politics trace sink is cleaner than adding test-only helpers or embedding more political forensics into event-log assertions. The trace already exists at the correct authoritative seam.
2. The trace extension should remain concrete and state-based: record timer-state snapshots and counted support data, not synthetic high-level explanations detached from world state.
3. No backwards-compatibility aliasing should be introduced. Existing trace consumers can be updated directly to the richer schema; do not add duplicated “old” and “new” trace event types unless a single coherent migration path is impossible.

## Verification Layers

1. timer-state snapshot for support-law evaluation is recorded on install/reset outcomes -> focused authoritative trace tests in `crates/worldwake-systems/src/offices.rs`
2. counted support declarations and filtered exclusions are recorded concretely -> focused authoritative trace tests
3. real timed political race exposes both action ordering and richer succession explanation -> golden assertion in `crates/worldwake-ai/tests/golden_offices.rs`
4. office-holder mutation remains proven by authoritative world state, not inferred from trace alone -> golden authoritative assertion retained
5. delayed authoritative effects are not used as a proxy for earlier action ordering; action trace and political trace must remain distinct proof surfaces

## What to Change

### 1. Extend the politics trace schema

Add structured support-law evaluation context to `OfficeSuccessionTrace`, likely including:

- a reusable timer-state snapshot for vacant-office evaluation
- counted support tallies by candidate
- filtered declaration details when a declaration existed but did not count

The schema should answer “why did evaluation happen now?” and “what was counted?” without re-reading code.

### 2. Record richer support-law evaluation data in `offices.rs`

Update `resolve_support_succession()` and related trace helpers so every support-law evaluation event records:

- timer state at evaluation
- raw declarations seen
- counted candidate totals
- filtered declaration reasons where applicable

The trace should remain append-only and zero-cost when disabled.

### 3. Expose the richer trace through focused and golden coverage

Add focused trace assertions in `crates/worldwake-systems/src/offices.rs` and extend the Scenario 34 golden in `crates/worldwake-ai/tests/golden_offices.rs` to assert the richer support-law explanation directly.

## Files to Touch

- `crates/worldwake-sim/src/politics_trace.rs` (modify)
- `crates/worldwake-sim/src/lib.rs` (modify if exports change)
- `crates/worldwake-systems/src/offices.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify only if helper surface needs widening)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify)

## Out of Scope

- Changing support-law semantics
- Adding planner-intent information to authoritative political traces
- Replacing decision traces or action traces with politics traces
- Cross-system omnibus timeline rendering

## Acceptance Criteria

### Tests That Must Pass

1. focused trace tests in `crates/worldwake-systems/src/offices.rs` cover support-law timer/install/reset explanation
2. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office`
3. `cargo test -p worldwake-ai --test golden_offices`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Politics tracing remains zero-cost when disabled and does not change authoritative political outcomes
2. Trace data stays concrete and reconstructable from authoritative system state; no narrative “winner reason” abstraction becomes source of truth
3. Timed support-law races can be explained from structured action trace + structured politics trace + authoritative world state without code-level reconstruction

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/offices.rs` focused trace tests — prove support-law install/reset outcomes carry timer-state and counted-support explanation.
2. `crates/worldwake-ai/tests/golden_offices.rs::golden_knowledge_asymmetry_race_informed_wins_office` — strengthen the golden so the timed race explanation is asserted directly at the politics-trace layer in addition to existing action/declaration/final-holder boundaries.

### Commands

1. `cargo test -p worldwake-systems offices::tests::`
2. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office`
3. `cargo test -p worldwake-ai --test golden_offices`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`
