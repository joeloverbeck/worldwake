# POLTRAC-001: Add Political System Traceability for Succession Decisions

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — add an authoritative politics trace surface at the `worldwake-sim` system-execution seam, record succession evaluation facts in `worldwake-systems`, and expose the sink through the golden harness
**Deps**: `crates/worldwake-sim/src/system_dispatch.rs`, `crates/worldwake-sim/src/tick_step.rs`, `crates/worldwake-sim/src/lib.rs`, `crates/worldwake-systems/src/offices.rs`, `crates/worldwake-ai/tests/golden_harness/mod.rs`, `docs/FOUNDATIONS.md`, `archive/specs/2026-03-17-action-execution-trace.md`

## Problem

Political succession is currently authoritative and inspectable through focused unit tests, golden event-log assertions, and final world state, but not through a dedicated system-level trace sink. That makes correctness debuggable, yet still forces office-resolution forensics to be reconstructed from deltas instead of read directly as a per-tick authoritative explanation.

## Assumption Reassessment (2026-03-18)

1. `crates/worldwake-systems/src/offices.rs` implements `succession_system()` as authoritative politics logic. It activates vacancy, clears stale `vacancy_since`, resolves support-law winners, and resolves force-law installation through direct system mutations.
2. The repo already has substantial focused authoritative coverage for office succession in `crates/worldwake-systems/src/offices.rs`:
   - `vacancy_activation_sets_vacancy_since_clears_relation_and_emits_visible_event`
   - `living_holder_clears_stale_vacancy_since`
   - `support_succession_installs_unique_top_supported_candidate_and_clears_declarations`
   - `support_succession_ignores_ineligible_declarations_and_resets_timer_on_no_valid_votes`
   - `support_tie_resets_vacancy_clock_without_installing_anyone`
   - `force_succession_installs_only_uncontested_eligible_present_agent`
   - `force_succession_blocks_when_multiple_contenders_are_present`
   The gap is not missing correctness coverage for succession semantics; it is missing trace-specific coverage and trace-specific observability.
3. The repo already has golden political coverage, not just low-level focused tests:
   - `crates/worldwake-ai/tests/golden_offices.rs::golden_force_succession_sole_eligible`
   - `crates/worldwake-ai/tests/golden_emergent.rs::golden_combat_death_triggers_force_succession`
   The latter already proves death -> vacancy -> delayed install ordering via action trace plus political event-log assertions. The missing piece is a direct politics trace explanation of why the system chose install, reset, or no-op.
4. There is no existing politics-specific trace sink analogous to `DecisionTraceSink` in `worldwake-ai` or `ActionTraceSink` in `worldwake-sim`. Current authoritative debugging still depends on event-log reconstruction after the fact.
5. The ticket's original file/dependency assumptions were incomplete:
   - the architectural seam is `worldwake-sim` (`SystemExecutionContext` and `TickStepServices`), not only `worldwake-systems` plus test harness wiring
   - `specs/E16-offices-succession-factions.md` is not a current active spec path
   - `docs/golden-e2e-testing.md` exists, but this ticket does not require docs changes unless trace usage materially changes test guidance
6. `docs/FOUNDATIONS.md` Principle 27 still makes this work architecturally justified: debuggability is a product feature, and political system decisions should be reconstructable without inventing fake actions or more event-log noise.

## Architecture Check

1. The proposed change is beneficial relative to the current architecture because it adds a first-class authoritative explanation surface without changing political semantics or polluting the event log.
2. The clean seam is not a trace type owned only by `worldwake-systems`. Systems receive execution context from `worldwake-sim`, so the sink must be plumbed through `worldwake-sim` first and then recorded by the politics system.
3. The preferred design is parallel to action tracing:
   - sink is optional and zero-cost when disabled
   - authoritative behavior remains unchanged
   - tests query a structured append-only record rather than reverse-engineering deltas
4. The trace should record per-office per-tick authoritative evaluation facts, not AI intentions and not synthetic political actions. That preserves Principle 24 and avoids aliasing political system mutations behind an action façade.
5. A robust minimal scope is office succession only. General political tracing can grow later from the same sim-layer seam if other politics systems appear.

## Verification Layers

1. vacancy clock activation / reset / clear semantics -> focused authoritative tests in `crates/worldwake-systems/src/offices.rs`
2. support-law and force-law trace explanation contents -> new focused trace tests in `crates/worldwake-systems/src/offices.rs`
3. no fake action alias for force succession -> existing and retained action-trace assertions in `crates/worldwake-ai/tests/golden_offices.rs` and `crates/worldwake-ai/tests/golden_emergent.rs`
4. combat death -> political vacancy -> delayed installation ordering -> existing and retained event-log ordering assertions in `crates/worldwake-ai/tests/golden_emergent.rs`
5. real scenario exposes politics trace explanation through runtime plumbing -> new golden assertion in `crates/worldwake-ai/tests/golden_emergent.rs` or `crates/worldwake-ai/tests/golden_offices.rs`

## What to Change

### 1. Add political trace types and sink at the sim seam

Introduce an optional append-only politics trace sink and thread it through system execution plumbing so authoritative systems can record structured trace data without emitting additional world events.

### 2. Record succession evaluation data in `offices.rs`

Wire tracing into `succession_system()` and helper paths in `crates/worldwake-systems/src/offices.rs`. Record enough information to answer:

- why an office was considered occupied or vacant
- whether the vacancy timer blocked evaluation
- which contenders or support declarations were considered
- which candidates were filtered as ineligible
- why the result was install, reset timer, clear stale vacancy, or no-op

### 3. Expose politics tracing through the golden harness

Extend `crates/worldwake-ai/tests/golden_harness/mod.rs` so golden tests can enable and inspect politics tracing the same way they currently can for action tracing.

### 4. Add trace-focused tests on top of existing correctness coverage

Add focused trace assertions in `crates/worldwake-systems/src/offices.rs` and at least one golden scenario assertion that a real succession chain exposes the expected politics trace explanation without manual event-log forensics.

## Files to Touch

- `crates/worldwake-sim/src/system_dispatch.rs`
- `crates/worldwake-sim/src/tick_step.rs`
- `crates/worldwake-sim/src/lib.rs`
- `crates/worldwake-sim/src/` new politics-trace module if needed
- `crates/worldwake-systems/src/offices.rs`
- `crates/worldwake-systems/src/lib.rs` only if export surface is needed
- `crates/worldwake-ai/tests/golden_harness/mod.rs`
- `crates/worldwake-ai/tests/golden_emergent.rs` or `crates/worldwake-ai/tests/golden_offices.rs`

## Out of Scope

- Reifying succession as an action
- Changing political behavior or succession semantics
- Replacing existing event-log assertions that still verify authoritative ordering
- Cross-system unified timeline rendering

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-systems offices::tests:: -- --list`
2. `cargo test -p worldwake-systems offices::tests::`
3. `cargo test -p worldwake-ai --test golden_offices`
4. `cargo test -p worldwake-ai --test golden_emergent`
5. `cargo test -p worldwake-ai`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets`

### Invariants

1. Politics tracing is zero-cost when disabled and does not change authoritative outcomes.
2. Force-law succession remains a system mutation, not an AI action path or compatibility alias.
3. The trace explains both positive and negative decisions, not only successful installations.
4. Existing event-log ordering assertions remain the source of truth for authoritative mutation order; politics tracing supplements explanation, not causality.

## Tests

### New/Modified Tests

1. New focused trace tests in `crates/worldwake-systems/src/offices.rs`
   Rationale: prove the trace records activation, timer-blocked waiting, support reset, force install, and force no-op cases at the authoritative layer.
2. Modify `crates/worldwake-ai/tests/golden_emergent.rs` or `crates/worldwake-ai/tests/golden_offices.rs`
   Rationale: prove a real runtime succession scenario exposes a readable politics-trace explanation through the full sim plumbing, not just through a unit fixture.

### Commands

1. `cargo test -p worldwake-systems offices::tests::`
2. `cargo test -p worldwake-ai --test golden_offices`
3. `cargo test -p worldwake-ai --test golden_emergent`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets`

## Outcome

- Completion date: 2026-03-18
- What actually changed:
  - added an opt-in `PoliticalTraceSink` in `worldwake-sim` and threaded it through `SystemExecutionContext` / `TickStepServices`
  - recorded structured office-succession trace events in `crates/worldwake-systems/src/offices.rs`
  - exposed politics tracing through `crates/worldwake-ai/tests/golden_harness/mod.rs`
  - added focused authoritative trace tests in `crates/worldwake-systems/src/offices.rs`
  - added golden politics-trace assertions to `crates/worldwake-ai/tests/golden_emergent.rs`
- Deviations from original plan:
  - no docs changes were needed
  - the real architectural seam was `worldwake-sim`, so the implementation touched sim plumbing in addition to the politics system and harness
  - existing focused and golden succession correctness coverage was retained rather than replaced; the new work supplements it with explanation-layer coverage
- Verification results:
  - `cargo test -p worldwake-systems offices::tests::` passed
  - `cargo test -p worldwake-ai --test golden_offices` passed
  - `cargo test -p worldwake-ai --test golden_emergent` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets` passed
