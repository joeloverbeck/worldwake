# DEPRTRACE-001: Add First-Class Authoritative Deprivation Traceability

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — add an authoritative deprivation/needs trace surface in `worldwake-sim`, plumb it through system dispatch, and expose opt-in harness accessors
**Deps**: `docs/FOUNDATIONS.md`, `crates/worldwake-systems/src/needs.rs`, `crates/worldwake-sim/src/action_trace.rs`, `crates/worldwake-sim/src/politics_trace.rs`, `crates/worldwake-ai/tests/golden_harness/mod.rs`, `archive/tickets/completed/S17WOULIFGOLSUI-001.md`

## Problem

Worldwake currently has strong traceability for AI decisions (`DecisionTraceSink`), action lifecycle (`ActionTraceSink`), request resolution (`RequestResolutionTraceSink`), and politics (`PoliticalTraceSink`). It does not have an equivalent first-class trace surface for authoritative deprivation processing in the needs system. When a physiology-driven scenario fails, there is no structured per-tick record showing the authoritative chain:

- need value before/after tick
- deprivation exposure before/after tick
- whether a threshold fired
- what wound delta was applied
- whether the wound was created or worsened
- whether the exposure counter reset

That gap makes deprivation debugging rely on source inspection and indirect state assertions instead of the same explicit traceability standard already applied to other important causal layers.

## Assumption Reassessment (2026-03-21)

1. Existing trace surfaces are real and distinct: `worldwake_ai::DecisionTraceSink` in `crates/worldwake-ai/src/decision_trace.rs`, `worldwake_sim::ActionTraceSink` in `crates/worldwake-sim/src/action_trace.rs`, `worldwake_sim::RequestResolutionTraceSink` in `crates/worldwake-sim/src/request_resolution_trace.rs`, and `worldwake_sim::PoliticalTraceSink` in `crates/worldwake-sim/src/politics_trace.rs`. The golden harness exposes opt-in helpers for action/request/politics tracing in `crates/worldwake-ai/tests/golden_harness/mod.rs`, and the AI driver exposes `enable_tracing()` for decision traces.
2. The authoritative deprivation logic lives in `crates/worldwake-systems/src/needs.rs`, specifically `needs_system()`, `apply_deprivation_consequences()`, and `worsen_or_create_deprivation_wound()`. Existing focused tests prove the behavior but not the traceability surface: `needs_system_adds_starvation_wound_and_resets_hunger_exposure`, `needs_system_requires_another_full_tolerance_period_before_second_wound`, and `needs_system_second_starvation_threshold_worsens_existing_wound`.
3. Existing runtime/integration coverage in `crates/worldwake-systems/tests/e09_needs_integration.rs` proves scheduler-driven deprivation effects, and the golden gap that motivated this ticket is documented in [S17WOULIFGOLSUI-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S17WOULIFGOLSUI-001.md). What is missing is not behavior coverage; it is a dedicated authoritative trace surface for deprivation consequences.
4. This is not an AI reasoning ticket. The target layer is authoritative system execution. Decision traces are not the right substrate because deprivation firing occurs in `needs_system`, not in candidate generation, ranking, or planner search.
5. No ordering-sensitive cross-branch contract is the primary goal here. If ordering appears, it is authoritative world-state ordering inside one system tick.
6. No heuristic/filter removal is involved.
7. Not a stale-request or start-failure ticket.
8. No `ControlSource`, queued-input, or runtime-intent retention question is involved.
9. Scenario isolation is not the core issue; the gap exists regardless of the specific golden scenario because the missing observability is at the authoritative needs-system layer.
10. Mismatch to correct: the current trace architecture is comprehensive for AI/action/request/politics, but not for authoritative deprivation/needs consequences. That asymmetry should be resolved with a first-class trace sink, not ad hoc logging.

## Architecture Check

1. A dedicated deprivation trace sink is cleaner than stuffing deprivation facts into unrelated trace systems or relying on `eprintln!` debugging. It preserves Principle 24 by tracing the authoritative system at its own boundary, and it preserves Principle 25 by making the trace a derived/debug view rather than a source of truth.
2. The trace must be opt-in, observational, and replaceable by recomputation from source state. It must not become a hidden authority path or a testing-only branch. No backwards-compatibility aliasing or mixed-purpose "misc trace" bucket should be introduced.

## Verification Layers

1. Trace sink records per-tick deprivation processing facts (needs/exposure before and after, fire/no-fire, wound delta, reset) -> focused unit/runtime tests around the new trace sink and `needs_system`
2. Trace sink is opt-in and zero-cost when disabled -> focused runtime tests on `needs_system` dispatch path
3. Golden harness can enable and query the new sink without changing live behavior -> harness tests in `crates/worldwake-ai/tests/golden_harness/mod.rs`
4. Existing authoritative state behavior remains unchanged while tracing is enabled -> focused needs-system tests plus an integration/golden check on the motivating deprivation scenario
5. Later world-state assertions are not a substitute for this ticket; the contract starts earlier at authoritative deprivation processing, so the trace surface itself must be asserted directly.

## What to Change

### 1. Add a new trace sink in `worldwake-sim`

Create a first-class `NeedsTraceSink` (or equivalently precise name) in `crates/worldwake-sim/src/` following the same opt-in pattern as `ActionTraceSink` and `PoliticalTraceSink`.

The trace event shape should be specific and authoritative, not generic. It should at minimum capture:

- actor/entity id
- tick
- deprivation kind
- needs before/after relevant to the fired branch
- exposure before/after
- whether a threshold fired
- whether the outcome was `Created`, `Worsened`, `ResetOnly`, or `NoEffect`
- wound id when applicable
- severity delta / resulting severity when applicable

The sink must remain a derived diagnostic artifact, never authority.

### 2. Plumb the sink through system execution cleanly

Extend the system-execution plumbing so `needs_system` can receive an optional mutable sink without polluting unrelated systems:

- add the optional trace sink reference in the sim/system dispatch boundary
- thread it through the canonical tick path
- avoid politics-specific naming or a catch-all trace blob

If the current `SystemExecutionContext` shape makes this awkward, refactor the context in a principled way rather than adding a deprivation-only hack field with unclear ownership.

### 3. Expose harness accessors

Extend `crates/worldwake-ai/tests/golden_harness/mod.rs` with opt-in helpers equivalent to the existing action/request/politics tracing helpers so future goldens can inspect deprivation traces directly when the contract begins at authoritative needs processing.

### 4. Add focused and integration coverage

Add focused tests proving:

- the sink records starvation fire creation
- the sink records a second starvation fire as a worsening with preserved wound id
- the sink records exposure reset semantics
- tracing disabled leaves behavior unchanged

Add one higher-level test using the existing deprivation golden scenario or a narrow scheduler/integration harness to prove the trace is queryable end to end without changing the scenario outcome.

### 5. Document trace guidance

Update the relevant debugging guidance so deprivation/needs debugging tells contributors to use the new authoritative trace surface before dropping to ad hoc instrumentation.

That documentation should distinguish it clearly from AI decision traces and action traces.

## Files to Touch

- `crates/worldwake-sim/src/` (new trace sink module)
- `crates/worldwake-sim/src/lib.rs` (modify — exports)
- `crates/worldwake-sim/src/system_dispatch.rs` (modify)
- `crates/worldwake-sim/src/tick_step.rs` (modify)
- `crates/worldwake-systems/src/needs.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)
- `docs/golden-e2e-testing.md` and/or `AGENTS.md` (modify — trace guidance, depending on where the authoritative debugging rule best belongs)

## Out of Scope

- Changing deprivation severity semantics
- Changing `worsen_or_create_deprivation_wound()` behavior
- Folding deprivation facts into decision traces or action traces
- Adding a generic "trace everything" sink
- Reworking unrelated system traces

## Acceptance Criteria

### Tests That Must Pass

1. Focused needs trace tests prove creation, worsening, and reset recording through `needs_system`
2. Harness/runtime tests prove tracing is opt-in and behavior is unchanged when enabled
3. Existing suite: `cargo test -p worldwake-systems needs:: -- --list`

### Invariants

1. The new trace surface records authoritative deprivation processing at the system boundary, not AI reasoning or downstream action facts
2. Tracing remains derived, opt-in, and non-authoritative
3. Existing deprivation behavior remains unchanged with tracing enabled

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs.rs` focused trace tests — prove the sink records deprivation creation/worsening/reset semantics at the authoritative layer
2. `crates/worldwake-ai/tests/golden_harness/mod.rs` harness tests — prove the new sink can be enabled and queried without affecting runtime behavior
3. `crates/worldwake-ai/tests/golden_emergent.rs` or `crates/worldwake-systems/tests/e09_needs_integration.rs` trace-enabled scenario test — prove end-to-end queryability on a real deprivation scenario

### Commands

1. `cargo test -p worldwake-systems needs:: -- --list`
2. `cargo test -p worldwake-systems needs::tests::needs_system_second_starvation_threshold_worsens_existing_wound -- --exact`
3. `cargo test --workspace`
