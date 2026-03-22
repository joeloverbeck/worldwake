# POLTRAC-002: Extend Political Trace with Timer-State and Counted Support Snapshots

**Status**: COMPLETED
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

1. [`crates/worldwake-sim/src/politics_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/politics_trace.rs) already records `OfficeSuccessionTrace` with `holder_before`, `vacancy_since_before`, `availability_phase`, `outcome`, `support_declarations`, and `force_candidates`. The first-round trace sink from `POLTRAC-001` is present and correctly wired; this ticket is an extension of that surface, not a new trace architecture.
2. The live gap is narrower than the original ticket description. `OfficeSuccessionOutcome::WaitingForTimer` already carries timer arithmetic, and focused offices trace tests already cover waiting/reset behavior in [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs). What is still missing is a reusable support-resolution snapshot on decisive support-law ticks:
   - install
   - reset for no eligible declarations
   - reset for tie
3. The current support-law outcomes do not carry the timer-state snapshot or the counted-support snapshot that explains “why did resolution happen now and what exactly counted?” Consumers still have to reconstruct that by combining `vacancy_since_before`, evaluation tick, `succession_period_ticks`, and declaration filtering logic from multiple places.
4. [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs) already computes all authoritative ingredients needed for the richer trace:
   - vacancy timing
   - raw declarations
   - eligibility filtering
   - counted support totals
   This remains a trace-schema/recording gap, not a behavior gap.
5. The original ticket understated current coverage. The real golden consumer already exists as [`golden_knowledge_asymmetry_race_informed_wins_office`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs#L1716), and the focused authoritative trace tests for waiting and reset paths already exist in [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs). This ticket should strengthen those tests rather than describe them as absent.
6. This remains a mixed-layer traceability ticket, not an AI-planner semantics ticket. The authoritative closure boundary is support-law succession resolution in [`resolve_support_succession()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs#L196); the AI golden only proves that the richer authoritative trace can explain a real timed political race.
7. Ordering remains mixed-layer and must stay explicit:
   - action lifecycle ordering for `consult_record` / `declare_support` -> action trace
   - support-law evaluation timing and counted support at the resolution tick -> politics trace
   - final office-holder mutation -> authoritative world state
8. No heuristic or planner convenience layer is being added or removed. The clean extension consistent with [`docs/FOUNDATIONS.md`](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md) Principles 1, 3, 16, 24, and 27 is to record more concrete authoritative state at evaluation time:
   - timer-state snapshot
   - counted support tallies by candidate
   - declaration-level counted/excluded disposition
   not narrative “winner reason” strings or planner-facing aliases.
9. Scenario isolation for the real golden consumer remains the same as Scenario 34 from [`specs/S19-institutional-record-consultation-golden-suites.md`](/home/joeloverbeck/projects/worldwake/specs/S19-institutional-record-consultation-golden-suites.md): co-located sated agents, no travel branch, no self-care branch, and knowledge asymmetry as the decisive difference.
10. Mismatch + correction: this ticket should no longer claim it needs to add the first focused/golden coverage for political tracing. The required work is to widen the existing trace schema and update the existing focused/golden assertions to consume the richer support-resolution snapshot directly.

## Architecture Check

1. Extending the existing politics trace sink is cleaner than adding test-only helpers or embedding more political forensics into event-log assertions. The trace already exists at the correct authoritative seam.
2. The trace extension should remain concrete and state-based: record timer-state snapshots and counted support data, not synthetic high-level explanations detached from world state.
3. No backwards-compatibility aliasing should be introduced. Existing trace consumers can be updated directly to the richer schema; do not add duplicated “old” and “new” trace event types unless a single coherent migration path is impossible.

## Verification Layers

1. timer-state snapshot for support-law install/reset evaluation is recorded on decisive support-law outcomes -> focused authoritative trace tests in `crates/worldwake-systems/src/offices.rs`
2. counted support tallies and declaration-level counted/excluded disposition are recorded concretely -> focused authoritative trace tests in `crates/worldwake-systems/src/offices.rs`
3. real timed political race exposes action ordering and richer support-resolution explanation without code-level reconstruction -> golden assertion in `crates/worldwake-ai/tests/golden_offices.rs`
4. office-holder mutation remains proven by authoritative world state, not inferred from trace alone -> golden authoritative assertion retained
5. delayed authoritative effects are not used as a proxy for earlier action ordering; action trace and politics trace remain distinct proof surfaces

## What to Change

### 1. Extend the politics trace schema

Add structured support-law evaluation context to `OfficeSuccessionTrace`, reusing concrete data rather than narrative summaries. The schema should expose, at minimum:

- a reusable timer-state snapshot for vacant-office evaluation
- counted support tallies by candidate on decisive support-law ticks
- declaration-level disposition showing which declarations counted and which were excluded

The trace should answer “why did evaluation happen now?” and “what was counted?” without re-reading code.

### 2. Record richer support-law evaluation data in `offices.rs`

Update `resolve_support_succession()` and related trace helpers so every decisive support-law evaluation event records:

- timer state at evaluation
- raw declarations seen
- counted candidate totals
- declaration-level counted/excluded result

The trace remains append-only and zero-cost when disabled.

### 3. Strengthen existing focused and golden coverage

Update the existing focused trace assertions in `crates/worldwake-systems/src/offices.rs` and extend the existing Scenario 34 golden in `crates/worldwake-ai/tests/golden_offices.rs` so they assert the richer support-law explanation directly.

## Files to Touch

- `crates/worldwake-sim/src/politics_trace.rs` (modify)
- `crates/worldwake-sim/src/lib.rs` (modify if exports change)
- `crates/worldwake-systems/src/offices.rs` (modify)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify)

## Out of Scope

- Changing support-law semantics
- Adding planner-intent information to authoritative political traces
- Replacing decision traces or action traces with politics traces
- Cross-system omnibus timeline rendering

## Acceptance Criteria

### Tests That Must Pass

1. focused trace tests in `crates/worldwake-systems/src/offices.rs` cover support-law timer/install/reset explanation with counted support snapshots
2. `cargo test -p worldwake-systems support_succession_trace_records_install_with_resolution_snapshot`
3. `cargo test -p worldwake-systems support_succession_trace_records_tie_reset_with_resolution_snapshot`
4. `cargo test -p worldwake-systems support_succession_trace_records_no_eligible_reset_with_resolution_snapshot`
5. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office`
6. `cargo test -p worldwake-ai --test golden_offices`
7. `cargo test -p worldwake-ai`
8. `cargo test --workspace`
9. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Politics tracing remains zero-cost when disabled and does not change authoritative political outcomes
2. Trace data stays concrete and reconstructable from authoritative system state; no narrative “winner reason” abstraction becomes source of truth
3. Decisive support-law ticks expose enough structured timer/counting state to explain install/reset outcomes without code-level reconstruction
4. Timed support-law races can be explained from structured action trace + structured politics trace + authoritative world state without code-level reconstruction

## Tests

### New/Modified Tests

1. `crates/worldwake-systems/src/offices.rs::support_succession_trace_records_install_with_resolution_snapshot`
   Rationale: proves the install tick carries the timer-state snapshot and counted-support snapshot for the winning candidate.
2. `crates/worldwake-systems/src/offices.rs::support_succession_trace_records_tie_reset_with_resolution_snapshot`
   Rationale: proves a tie reset carries the decisive timer snapshot plus the counted tallies that caused the reset.
3. `crates/worldwake-systems/src/offices.rs::support_succession_trace_records_no_eligible_reset_with_resolution_snapshot`
   Rationale: proves a no-eligible reset carries the decisive timer snapshot and shows that declarations existed but none counted.
4. `crates/worldwake-ai/tests/golden_offices.rs::golden_knowledge_asymmetry_race_informed_wins_office`
   Rationale: proves the real timed race can now be explained directly from the politics trace in addition to action ordering and final holder assertions.

### Commands

1. `cargo test -p worldwake-systems support_succession_trace_records_install_with_resolution_snapshot`
2. `cargo test -p worldwake-systems support_succession_trace_records_tie_reset_with_resolution_snapshot`
3. `cargo test -p worldwake-systems support_succession_trace_records_no_eligible_reset_with_resolution_snapshot`
4. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office`
5. `cargo test -p worldwake-ai --test golden_offices`
6. `cargo test -p worldwake-ai`
7. `cargo test --workspace`
8. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-22
- What actually changed:
  - extended [`OfficeSuccessionTrace`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/politics_trace.rs) with reusable `vacancy_timer` and `support_resolution` snapshots
  - extended [`SupportDeclarationTrace`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/politics_trace.rs) with concrete `counted` disposition
  - updated [`resolve_support_succession()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs) and trace construction in [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs) so decisive support-law ticks record timer-state plus counted-support data without changing political behavior
  - strengthened focused offices trace tests for waiting/install/tie/no-eligible paths and extended Scenario 34 in [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs) to assert the richer politics trace directly
- Deviations from original plan:
  - no `golden_harness/mod.rs` changes were needed because the existing politics-trace plumbing was already sufficient
  - the outcome kept the existing `OfficeSuccessionOutcome` variants and added reusable snapshots alongside them rather than rewriting the entire outcome enum around the new snapshot types
- Verification results:
  - `cargo test -p worldwake-systems support_succession_trace_records_install_with_resolution_snapshot` passed
  - `cargo test -p worldwake-systems support_succession_trace_records_tie_reset_with_resolution_snapshot` passed
  - `cargo test -p worldwake-systems support_succession_trace_records_no_eligible_reset_with_resolution_snapshot` passed
  - `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office` passed
  - `cargo test -p worldwake-ai --test golden_offices` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
