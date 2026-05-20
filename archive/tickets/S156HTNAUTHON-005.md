# S156HTNAUTHON-005: Explicit traced strategic fallback + deeper method traces

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-ai` HTN selector + strategic search + decision trace; `worldwake-cli` observer rendering
**Deps**: archive/tickets/S156HTNAUTHON-002.md, archive/tickets/S156HTNAUTHON-003.md, archive/tickets/S156HTNAUTHON-004.md

## Problem

Before this ticket, two debuggability gaps (FND-29) made the HTN layer hard to explain:

1. When no selected method produced stages, `search/strategic.rs` emitted `method_trace: None` and
   silently fell back to missing-commodities / goal-places / exploration. The fact that flat-GOAP
   fallback occurred — and why — was invisible.
2. `method_trace()` recorded only the selected method id, its subgoals as `Pending`, and the motive
   score (`failure_mode: None`). It did not record which *other* methods were rejected for the goal
   kind or which precondition failed.

This ticket made the strategic fallback explicit and recorded with a reason, and extended the
method trace to record rejected candidate methods with their failing precondition. The trace now
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

## Verified Layers

1. Rejected methods + failing precondition recorded -> decision-trace assertions in
   `autonomous_produce_candidate_records_method_trace` and selector unit coverage now prove
   `MethodPlanAttemptTrace` lists rejected methods with the first failing precondition.
2. Fallback occurrence + reason recorded -> `disabled_produce_methods_fall_back_to_flat_strategic_search`
   now proves `StrategicFallbackReason::NoViableMethod` is recorded instead of a silent
   `method_trace: None`.
3. Selection outcome unchanged (no world-behavior regression) -> `cargo test -p worldwake-ai`
   passed with the full non-ignored `golden_ai` suite.
4. Observer renders the added trace fields -> `render_method_trace_with_subgoals_produces_expected_text`
   and `render_method_trace_none_produces_fallback_note` assert rejected-method and fallback-reason
   lines.
5. Determinism preserved -> `autonomous_produce_method_trace_replays_deterministically`,
   `autonomous_bounty_method_trace_replays_deterministically`, and
   `disabled_method_fallback_replays_deterministically` pass with the added fields.

## Landed Changes

### 1. Surfaced rejected methods + failing precondition from the selector

`select_method_with_recipes` now returns `MethodSelection { selected, rejected }`.
`RejectedMethodSelection` records the method id and first failed `MethodPrecondition`.
The public `select_method` wrapper still returns only the winner for callers that do not need
trace detail.

### 2. Extended `MethodPlanAttemptTrace`

`MethodPlanAttemptTrace` now carries deterministic `rejected_methods:
Vec<RejectedMethodTrace>` and `fallback_reason: Option<StrategicFallbackReason>`.
The fallback reason enum has `NoViableMethod` and `MethodProducedNoStages`. The trace remains a
non-serialized transient debug read-model.

### 3. Recorded fallback explicitly in strategic search

`search/strategic.rs` threads `MethodSelection` through `build_stages` and emits a method trace
for the flat strategic fallback path. Fallback without a selected method records
`NoViableMethod`; fallback after a selected method produces no stages records
`MethodProducedNoStages`.

### 4. Rendered the trace fields in the observer

The observer renders fallback reasons and rejected methods with their failed preconditions.
The observer render tests now cover both selected-method details and method-less fallback output.

### 5. Updated affected goldens + added rejection/fallback assertions

`crates/worldwake-ai/tests/scenarios/htn_methods.rs` now asserts rejected-method preconditions,
fallback reason, and deterministic replay for the updated trace shape.

## Landed Files

- `crates/worldwake-ai/src/htn/selector.rs`
- `crates/worldwake-ai/src/htn/mod.rs`
- `crates/worldwake-ai/src/decision_trace.rs`
- `crates/worldwake-ai/src/lib.rs`
- `crates/worldwake-ai/src/search/strategic.rs`
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs`
- `crates/worldwake-cli/src/bin/observer.rs`
- `crates/worldwake-ai/tests/scenarios/htn_methods.rs`

## Out of Scope

- Removing the no-op/dead preconditions and methods (S156HTNAUTHON-002/003) — prerequisites.
- Making any goal method-required or enforcing method leaves (explicit Non-Goal; flat-GOAP
  fallback stays legal).
- Changing tactical GOAP search internals.

## Acceptance Result

### Tests Passed

1. Passed: `autonomous_produce_candidate_records_method_trace` asserts a rejected production
   method and failing precondition.
2. Passed: `disabled_produce_methods_fall_back_to_flat_strategic_search` asserts the fallback
   reason is recorded.
3. Passed: observer render tests display rejected methods and fallback reasons.
4. Passed: `cargo test -p worldwake-ai` includes the non-ignored `golden_ai` suite and showed no
   world-outcome regression.

### Invariants

1. The strategic fallback is never silent: a method trace with a fallback reason is emitted instead
   of `method_trace: None` (FND-29).
2. Method-selection outcomes (which method is chosen, which actions commit) are unchanged by this
   ticket — only trace data is added (FND-20: fallback was already legal).
3. Rejected-method ordering is deterministic (registry order; no `HashMap`/`HashSet`).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/htn_methods.rs` — added rejected-method and fallback-reason
   assertions to the HTN trace goldens while preserving deterministic replay checks.
2. `crates/worldwake-cli/src/bin/observer.rs` — updated method-trace rendering tests for rejected
   methods and fallback reasons.
3. `crates/worldwake-ai/src/htn/selector.rs` — updated selector unit coverage to assert rejected
   candidates with the failing precondition.

### Commands Passed

1. Passed `cargo test -p worldwake-ai --test golden_ai htn_methods`
2. Passed `cargo test -p worldwake-ai htn::selector`
3. Passed `cargo test -p worldwake-ai`
4. Passed `cargo test -p worldwake-cli`
5. Passed `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-20.

- The HTN selector now preserves contrastive rejection information without changing method
  selection outcomes.
- Strategic fallback is no longer silent for method-selection attempts: fallback method traces
  carry `NoViableMethod` or `MethodProducedNoStages`.
- The observer and scenario diagnostics constructor sites were updated for the expanded trace
  shape.
- No follow-up ticket was required for this slice; the documentation slice later completed and
  archived as `archive/tickets/S156HTNAUTHON-006.md`.

## Deviations

- `SearchTraceMetadata` did not require a shape change; the existing `method_trace` propagation
  already carried the expanded `MethodPlanAttemptTrace`.
- `scenario_diagnostics/aggregator.rs` needed constructor fallout only; no new diagnostics
  aggregate was added for rejected methods or fallback reasons.
- `./scripts/verify.sh` was not run for this per-ticket iteration because the
  `implement-spec-tickets` harness runs that pre-push gate after the full S156 family lands.

## Verification Result

- Passed `cargo test -p worldwake-ai htn::selector`
- Passed `cargo test -p worldwake-ai --test golden_ai htn_methods`
- Passed `cargo test -p worldwake-cli --bin observer render_method_trace`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Waived `./scripts/verify.sh` for this ticket iteration because the harness owns the final
  pre-push verification gate after all S156 tickets are complete.
