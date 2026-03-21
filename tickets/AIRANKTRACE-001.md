# AIRANKTRACE-001: Decision Trace — Ranking Provenance For Priority Promotion And Motive Inputs

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` ranking/decision-trace payloads, runtime trace capture, and trace-focused tests
**Deps**: [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md), [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md), [archive/tickets/completed/S16S09TRACE-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S16S09TRACE-001.md), [archive/tickets/completed/S16S09GOLVAL-006.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S16S09GOLVAL-006.md), [archive/tickets/completed/S17WOULIFGOLSUI-002.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S17WOULIFGOLSUI-002.md)

## Problem

The current decision trace can show which goal won, its final `priority_class`, and its final `motive_score`, but it cannot explain the concrete causal chain that produced those values. During Scenario 30 work, proving why `eat` beat `wash` still required reading `crates/worldwake-ai/src/ranking.rs` by hand, because the trace did not expose:

- the base class before recovery-aware promotion,
- whether a promotion was applied and why,
- the concrete drive pressure and utility weight that produced the motive score.

That is a traceability gap, not a behavior gap. For a causality-first project, the canonical AI trace should explain ranking decisions from concrete local state without requiring source-diving.

## Assumption Reassessment (2026-03-21)

1. The current ranked-candidate trace surface is `RankedGoalSummary` in [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs). It currently stores `goal`, `priority_class`, `motive_score`, and optional `RankedGoalProvenance`, but that provenance is currently only populated for danger-driven goals. There is no shared structured provenance for ordinary drive ranking or motive composition.
2. The relevant ranking logic already exists in one canonical place in [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs):
   - `drive_priority()`
   - `promote_for_clotted_wound_recovery()`
   - `motive_score()`
   - `relevant_self_consume_factors()`
   - `score_product()`
   The missing substrate is not behavior or data collection; it is structured exposure of these already-computed causal inputs.
3. Existing focused coverage proves behavior but not provenance:
   - `ranking::tests::clotted_wound_boosts_hunger_high_to_critical`
   - `ranking::tests::clotted_wound_no_boost_relieve_or_wash`
   - `ranking::tests::hunger_candidate_becomes_critical_and_uses_weight_times_pressure`
   - `candidate_generation::tests::wash_requires_dirtiness_and_local_water`
   - `golden_recovery_aware_boost_eats_before_wash` in [crates/worldwake-ai/tests/golden_combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs)
   None of these let the runtime decision trace answer "what was the base class?", "what promotion fired?", or "which pressure/weight pair produced this motive?".
4. This is a runtime AI traceability ticket, not a planner-behavior or authoritative-validation ticket. The intended verification layers are focused ranking/trace tests, one focused runtime trace assertion, and a strengthened existing golden assertion. Full action registries are required for the golden boundary because the existing Scenario 30 golden crosses ranking, execution, and recovery.
5. This is not an ordering-centric ticket. Ordering remains covered by action traces in existing goldens. The contract here is provenance completeness for a single ranked decision tick.
6. No heuristic, filter, or gameplay rule is being removed. The missing architectural substrate is a reusable typed ranking-provenance surface attached to the canonical decision trace.
7. The clean extension point is the shared ranked-goal trace surface, not a Scenario-30-specific blob. This follows the same architectural pattern used by danger provenance and selected-plan search provenance: one canonical trace surface that remains reusable across goldens.
8. Not a stale-request, contested-affordance, political-closure, or `ControlSource` ticket.
9. The scenario that exposed this gap depends on mixed ranking substrates. In [archive/tickets/completed/S17WOULIFGOLSUI-002.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S17WOULIFGOLSUI-002.md), `eat` beats `wash` because class promotion (`High -> Critical`) overrides a stronger competing `wash` motive. The current trace can prove the final outcome but not the causal arithmetic or promotion path that produced it.
10. Mismatch corrected: the gap is not "decision traces are missing" and not "ranking behavior is wrong"; it is "the shared ranked-goal trace surface does not yet expose the concrete ranking inputs that already exist in the runtime."

## Architecture Check

1. The clean solution is to enrich the canonical ranked-goal trace schema with typed ranking provenance rather than introduce ad-hoc debug dumps or scenario-specific helper assertions. Worldwake already treats traces as first-class architecture, and this keeps AI explanation on one authoritative surface.
2. Provenance should be captured from the same ranking pass that computes the final result, not recomputed later from a looser view. That keeps the trace honest, deterministic, and aligned with Principle 3 (concrete state), Principle 7 (locality of information), and Principle 27 (debuggability).
3. The provenance shape should stay compact and typed. A candidate should expose the minimum causal facts needed to explain its ranking, such as:
   - base priority class
   - final priority class
   - promotion/cap reason, if any
   - motive components (drive kind, pressure, weight, score product)
   It should not become a free-form string dump.
4. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Ranking provenance type captures base/final class and motive inputs without duplicating ranking logic -> focused unit tests in [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs)
2. Decision-trace schema and summaries expose the new provenance cleanly on the shared ranked-goal surface -> focused unit tests in [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)
3. Runtime decision tracing populates the richer ranked-goal provenance from a real `agent_tick` -> focused runtime/integration test in [crates/worldwake-ai/src/agent_tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs)
4. Scenario 30 golden can assert the selection cause from the canonical decision trace rather than source-reading -> strengthen [crates/worldwake-ai/tests/golden_combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs)
5. Later `eat` before `wash` and wound recovery remain downstream proof surfaces already covered by action trace and authoritative state; they are not a proxy for ranking provenance

## What to Change

### 1. Add canonical ranking-provenance types

Extend the ranking/decision-trace model with typed provenance that can explain why a ranked candidate received its class and motive. The shape should be reusable across drive-based candidates and future ranking tickets, not tied only to hunger recovery promotion.

At minimum the shared trace surface should be able to represent:

- the base class before any promotion/cap,
- the final class after promotion/cap,
- the reason for the class change, if any,
- the concrete motive inputs that produced the final `motive_score`.

### 2. Populate provenance from the real ranking path

Update the ranking pipeline so provenance is emitted from the same logic that computes class and motive. Do not reconstruct motive components or promotions in `agent_tick` purely for tracing.

### 3. Surface the richer payload in runtime decision traces

Update runtime trace capture and summary formatting so ranked candidates and selected candidates expose the new provenance in a test-friendly way.

### 4. Strengthen focused and golden coverage

Add focused tests for the new provenance payload and strengthen the existing Scenario 30 golden so it can assert on the canonical ranking explanation directly.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (modify)
- `crates/worldwake-ai/tests/golden_combat.rs` (modify)
- `AGENTS.md` (modify only if the debugging guidance should mention the richer ranking provenance explicitly)

## Out of Scope

- Changing ranking behavior, thresholds, or recovery rules
- Adding a second trace system outside the canonical decision trace
- Adding free-form string parsing as a substitute for structured provenance
- Scenario-specific debug helpers that do not generalize beyond Scenario 30
- Refactoring unrelated golden tests

## Acceptance Criteria

### Tests That Must Pass

1. Focused ranking/decision-trace/runtime tests for the new ranking-provenance payload pass
2. `cargo test -p worldwake-ai --test golden_combat golden_recovery_aware_boost_eats_before_wash`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Decision traces remain optional and zero-cost when disabled
2. Ranking provenance reflects the same runtime inputs used for the actual decision, not a parallel reconstruction
3. The canonical ranking logic continues to have a single source of truth
4. Trace enrichment does not change ranking behavior or authoritative world behavior
5. The richer trace remains concrete, typed, deterministic, and extensible

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` — add focused tests for ranking provenance on promoted and unpromoted drive goals
2. `crates/worldwake-ai/src/decision_trace.rs` — add focused tests for ranked-goal provenance formatting and summary/dump behavior
3. `crates/worldwake-ai/src/agent_tick.rs` — extend an existing trace-focused runtime test to prove the new provenance is present on a real tick
4. `crates/worldwake-ai/tests/golden_combat.rs::golden_recovery_aware_boost_eats_before_wash` — strengthen the existing golden to assert on the canonical ranking explanation rather than only the final selected goal

### Commands

1. `cargo test -p worldwake-ai clotted_wound_boosts_hunger_high_to_critical -- --exact`
2. `cargo test -p worldwake-ai trace_planning_outcome_includes_danger_provenance_for_threatened_agent -- --exact`
3. `cargo test -p worldwake-ai --test golden_combat golden_recovery_aware_boost_eats_before_wash -- --exact`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`
