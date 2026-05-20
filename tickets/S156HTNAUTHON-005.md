# S156HTNAUTHON-005: Explicit traced strategic fallback + deeper method traces

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-ai` HTN selector + strategic search + decision trace; `worldwake-cli` observer rendering
**Deps**: archive/tickets/S156HTNAUTHON-002.md, archive/tickets/S156HTNAUTHON-003.md, archive/tickets/S156HTNAUTHON-004.md

## Problem

Two debuggability gaps (FND-29) make the HTN layer hard to explain:

1. When no selected method produces stages, `search/strategic.rs` emits `method_trace: None` and
   silently falls back to missing-commodities / goal-places / exploration. The fact that flat-GOAP
   fallback occurred — and why — is invisible.
2. `method_trace()` records only the selected method id, its subgoals as `Pending`, and the motive
   score (`failure_mode: None`). It never records which *other* methods were rejected for the goal
   kind or which precondition failed.

This ticket makes the strategic fallback explicit and recorded with a reason, and extends the
method trace to record rejected candidate methods with their failing precondition — so the trace
answers "why this method, why not the others, and did fallback happen?" (FND-20's explicit,
legal flat-GOAP fallback; FND-29's contrastive "why not?").

## Assumption Reassessment (2026-05-20)

1. `MethodPlanAttemptTrace` (`crates/worldwake-ai/src/decision_trace.rs:1222-1227`) currently has
   `method_id: Option<MethodSchemaId>`, `subgoals_attempted: Vec<SubgoalAttemptResult>`,
   `failure_mode: Option<MethodFailureMode>`, `motive_score: u32`. It derives only
   `Clone, Debug, Eq, PartialEq` — no `Serialize`/`Deserialize`, so it is a transient debug
   read-model with no save/replay surface to migrate (consistent with FND-29A). `method_id` is
   already `Option`, so a method-less fallback trace is representable.
2. `method_trace()` (`search/strategic.rs:446-462`) builds the trace from a single
   `&MethodSchema` + motives; it has no access to rejected methods. The fallback `method_trace: None`
   sites are in `build_stages`/the early returns of `plan_with_budget_trace`
   (`strategic.rs`, e.g. lines ~165, ~174, ~442). `StrategicSearchResult.method_trace`
   (`strategic.rs:32-37`) propagates to `SearchTraceMetadata.method_trace`
   (`search/mod.rs:54-79`) and onward to the decision trace.
3. Selector data path: `select_method_with_recipes` (`htn/selector.rs:25-47`) returns only
   `Option<&MethodSchema>` via `.filter(...).max_by(...)`, discarding rejected candidates;
   `preconditions_satisfied` (`htn/selector.rs:49-60`) returns a bare `bool` via `.all()`,
   discarding *which* precondition failed. Recording rejected methods + failing preconditions
   therefore requires restructuring the selector, not just the trace struct (per the spec's D5
   note). This ticket depends on S156HTNAUTHON-002/003 having pruned the `AgentRole`/`EntityCriterion`
   arms from `evaluate_precondition` so the restructure targets the final arm set.
4. Shared boundary under audit: the selector→strategic→decision-trace data path for method
   selection, specifically the return contract of `select_method_with_recipes` /
   `preconditions_satisfied` and the `MethodPlanAttemptTrace` shape. This is a mixed-layer change
   (AI search-control + decision-trace read-model + observer rendering).
5. Existing tests on the changed surface: golden `htn_methods.rs`
   `autonomous_produce_candidate_records_method_trace` (:843),
   `autonomous_bounty_candidate_records_method_trace` (:1092), their
   `*_method_trace_replays_deterministically` siblings (:893, :1117), and
   `disabled_produce_methods_fall_back_to_flat_strategic_search` (:1142) assert on trace content
   and on the fallback path — all shift with the new fields and the fallback reason and must be
   updated. Selector inline test `select_method_skips_methods_with_failed_preconditions`
   (`selector.rs:927`) exercises the precondition path being restructured. Observer test
   `render_method_trace_with_subgoals_produces_expected_text` (`worldwake-cli/.../observer.rs`)
   constructs `MethodPlanAttemptTrace` and renders it — it must construct the new fields and assert
   the new rendering.
6. Adjacent contradiction classification: none. The selector restructure is a required consequence
   of the rejected-method recording; the existing `.all()`/`max_by` shape is not a bug, just
   information-lossy. No unrelated regression is reopened — selection *outcomes* are unchanged
   (only the trace gains data), which the goldens' world-outcome assertions confirm.
7. Authoritative-to-AI note: this ticket touches HTN method selection (search-control) and the
   debug trace — not `ActionDef` preconditions, `validate_*`, affordance generation, candidate
   emission, or `is_satisfied`. Per the spec's D5 instruction, the full golden suite is run to
   confirm no world-outcome regression (trace changes expected, behavior changes not).

## Architecture Check

1. Making fallback explicit-and-traced (rather than silent `None`) and recording rejected methods
   turns the HTN layer into a debuggable, contrastive surface without changing what it decides.
   The trace remains a transient derived read-model, not authoritative state (FND-29A) — no new
   stored world state is introduced.
2. No shim: the selector's lossy return contract is replaced, not wrapped. The fallback path
   records a typed reason instead of emitting `None` beside a silent flat-GOAP fall-through.

## Verification Layers

1. Rejected methods + failing precondition recorded -> decision-trace assertion (golden/integration)
   that `MethodPlanAttemptTrace` for a goal kind with multiple candidate methods lists at least one
   rejected method with its failing precondition.
2. Fallback occurrence + reason recorded -> decision-trace assertion that, when no method produces
   stages, the trace carries the fallback reason (e.g. `no_viable_method` /
   `method_produced_no_stages`) instead of `None`
   (update `disabled_produce_methods_fall_back_to_flat_strategic_search`).
3. Selection outcome unchanged (no world-behavior regression) -> full `golden_ai` suite passes;
   trace fields change, committed actions do not.
4. Observer renders the new trace fields -> headless observer render test
   (`render_method_trace_with_subgoals_produces_expected_text` updated) asserting rejected-method
   and fallback-reason lines.
5. Determinism preserved -> existing `*_method_trace_replays_deterministically` goldens pass with
   the new fields (rejected-method ordering must be deterministic — collect in registry/`methods_for`
   order, no `HashMap`).

## What to Change

### 1. Surface rejected methods + failing precondition from the selector

Restructure `select_method_with_recipes` (`htn/selector.rs`) to return both the selected method
and the rejected candidates with the precondition that failed (e.g. a small result struct
`{ selected: Option<&MethodSchema>, rejected: Vec<(MethodSchemaId, MethodPrecondition)> }`).
Change `preconditions_satisfied` to surface the first failing `MethodPrecondition` rather than a
bare `bool` (e.g. return `Option<&MethodPrecondition>` — `None` meaning satisfied). Keep the
public `select_method` thin wrapper returning `Option<&MethodSchema>` for callers that only need
the winner, or update its single caller — verify call sites during implementation.

### 2. Extend `MethodPlanAttemptTrace`

Add fields to `MethodPlanAttemptTrace` (`decision_trace.rs`) to carry rejected candidate methods
with their failing precondition (a deterministic `Vec`), and a fallback reason
(`Option<StrategicFallbackReason>` or equivalent enum with at least `NoViableMethod` /
`MethodProducedNoStages`). Define the fallback-reason enum alongside the trace. Keep the type
non-serialized (transient debug read-model).

### 3. Record fallback explicitly in strategic search

In `search/strategic.rs`, replace the `method_trace: None` fallback emissions with a
`MethodPlanAttemptTrace` carrying `method_id: None`, the rejected-method list (from step 1), and
the fallback reason — so flat-GOAP fallback is traced rather than silent. Thread the
rejected-method data from the selector through `build_stages` → `StrategicSearchResult` →
(if needed) `SearchTraceMetadata`.

### 4. Render the new trace fields in the observer

Update the observer's method-trace rendering (`worldwake-cli/src/bin/observer.rs`) to display
rejected methods with their failing precondition and the fallback reason. Update the render test
`render_method_trace_with_subgoals_produces_expected_text` accordingly.

### 5. Update affected goldens + add rejection/fallback assertions (D7 distributed)

Update `autonomous_*_records_method_trace` / `*_replays_deterministically` and
`disabled_produce_methods_fall_back_to_flat_strategic_search` in
`crates/worldwake-ai/tests/scenarios/htn_methods.rs` to assert the new fields, including at least
one test proving a rejected method with its failing precondition is recorded and one proving the
fallback reason is recorded when no method applies.

## Files to Touch

- `crates/worldwake-ai/src/htn/selector.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/search/strategic.rs` (modify)
- `Likely: crates/worldwake-ai/src/search/mod.rs` (modify — confirm whether `SearchTraceMetadata` needs threading beyond the existing `Option` propagation; `grep method_trace crates/worldwake-ai/src/search/mod.rs`)
- `Likely: crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (modify — confirm whether the aggregator summarizes the new fields; `grep method_trace crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs`)
- `crates/worldwake-cli/src/bin/observer.rs` (modify)
- `crates/worldwake-ai/tests/scenarios/htn_methods.rs` (modify)

## Out of Scope

- Removing the no-op/dead preconditions and methods (S156HTNAUTHON-002/003) — prerequisites.
- Making any goal method-required or enforcing method leaves (explicit Non-Goal; flat-GOAP
  fallback stays legal).
- Changing tactical GOAP search internals.

## Acceptance Criteria

### Tests That Must Pass

1. A decision-trace test asserts `MethodPlanAttemptTrace` records at least one rejected method with
   its failing precondition for a goal kind with multiple candidates.
2. A decision-trace test asserts the fallback reason is recorded (not `None`) when no method
   produces stages.
3. Observer render test displays rejected methods and fallback reason.
4. Full golden suite: `cargo test -p worldwake-ai --test golden_ai` — no world-outcome regression.

### Invariants

1. The strategic fallback is never silent: a method trace with a fallback reason is emitted instead
   of `method_trace: None` (FND-29).
2. Method-selection outcomes (which method is chosen, which actions commit) are unchanged by this
   ticket — only trace data is added (FND-20: fallback was already legal).
3. Rejected-method ordering is deterministic (registry order; no `HashMap`/`HashSet`).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/htn_methods.rs` — extend `autonomous_*_records_method_trace`
   and `disabled_produce_methods_fall_back_to_flat_strategic_search`; add rejected-method and
   fallback-reason assertions and their deterministic-replay siblings.
2. `crates/worldwake-cli/src/bin/observer.rs` (test module) — update
   `render_method_trace_with_subgoals_produces_expected_text` for the new fields.
3. `crates/worldwake-ai/src/htn/selector.rs` (test module) — assert the restructured selector
   surfaces rejected candidates with the failing precondition.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai htn_methods`
2. `cargo test -p worldwake-ai htn::selector`
3. `cargo test -p worldwake-cli`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `./scripts/verify.sh` (before PR)
