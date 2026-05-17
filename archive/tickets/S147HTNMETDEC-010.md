# S147HTNMETDEC-010: Observer plan-attempt method rendering

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: No — observer and diagnostics JSON rendering extension in `crates/worldwake-cli`.
**Deps**: `archive/tickets/S147HTNMETDEC-009.md` (PlanAttemptTrace.method_trace field exists)

## Problem

S147 D9 extends the observer to surface the method chosen per plan attempt and its decomposition trace. Without this rendering, the `method_trace` field added in ticket 009 is invisible to operators inspecting scenario runs.

## Assumption Reassessment (2026-05-17)

1. `PlanAttemptTrace.method_trace: Option<MethodPlanAttemptTrace>` exists after ticket 009. Selected method subgoals are recorded as pending at selection time; the observer must render the trace without inferring later action success.
2. The spec's old "observer Section 7" target was stale. Live Section 7 is End-State Inventory & Resources. The landed observer surfaces are Section 8 failed plan-attempt details and Section 13 scenario diagnostics.
3. Section 8 already renders failed plan attempts in a compact table. This ticket adds a Method column and method-detail lines after the table.
4. Section 13 already renders `PlanningMetrics`. This ticket adds the `method_usage` aggregate from ticket 009.
5. Diagnostics JSON needed a vector representation for `method_usage` because `Option<MethodSchemaId>` is not a valid JSON object-key shape.

## Architecture Check

1. Observer rendering remains a derived view: it reads `PlanAttemptTrace.method_trace` and `PlanningMetrics.method_usage` without modifying simulation state.
2. Flat-GOAP fallback renders as `none (flat GOAP fallback)` rather than a synthesized method.
3. Failed method traces render the rich `MethodFailureMode` plus the `Discrepancy::MethodFailure(MethodFailureKind)` bridge.
4. No new observer section or compatibility shim was added.

## Verified Layers

1. Method-rendering produces expected selected-method and subgoal text -> `render_method_trace_with_subgoals_produces_expected_text`.
2. Flat-GOAP fallback renders cleanly -> `render_method_trace_none_produces_fallback_note`.
3. Failed method traces include discrepancy reference -> `render_method_trace_failure_includes_discrepancy_reference`.
4. Section 13 method usage renders -> `render_scenario_diagnostics_section_text_renders_section_13_and_top_n`.
5. Diagnostics JSON round-trips method usage through an entry vector -> `render_scenario_diagnostics_section_json_round_trips_payload_map_keys`.

## Landed Changes

### 1. Section 8 method rendering

The failed plan-attempt table now includes a Method column. After the table, the observer emits method-detail lines for shown attempts, including selected method name, subgoal kind/status rows, and failure-mode/discrepancy linkage when present.

### 2. Section 13 method usage

Scenario Diagnostics now prints a Method usage block from `PlanningMetrics.method_usage`, including selected, fallback, and failed counts.

### 3. Diagnostics JSON projection

`crates/worldwake-cli/src/diagnostics_json.rs` now serializes method usage as a vector of `{ method_id, counts }` entries and reconstructs the internal `BTreeMap<Option<MethodSchemaId>, MethodUsageCounts>` on read.

### 4. Spec truth sync

`specs/S147-htn-method-decomposition.md` now names the real Section 8/13 observer surfaces instead of the stale Section 7 claim.

## Landed Files

- `crates/worldwake-cli/src/bin/observer.rs`
- `crates/worldwake-cli/src/diagnostics_json.rs`
- `specs/S147-htn-method-decomposition.md`

## Outcome

Completed: 2026-05-17.

Observer output now exposes selected HTN method traces in failed plan-attempt diagnostics and aggregate method usage in scenario diagnostics.

## Deviations

- The landed per-attempt detail is attached to Section 8 failed plan attempts, which is the live plan-attempt rendering surface. Section 13 carries aggregate usage. The stale Section 7 spec reference was corrected.
- Subgoal status labels use ASCII `Pending`/`Succeeded`/`Failed` text instead of symbolic check/cross markers.

## Out of Scope

- Trace recording (ticket 009).
- Golden end-to-end method assertions (ticket 011).
- Adding a new observer section.

## Acceptance Result

### Tests Passed

1. `cargo test -p worldwake-cli --bin observer`
2. `cargo test -p worldwake-cli`
3. `cargo clippy -p worldwake-cli --all-targets -- -D warnings`

### Invariants

1. `method_trace: None` renders as flat-GOAP fallback.
2. Failed method attempts render both rich `MethodFailureMode` and typed `Discrepancy::MethodFailure(MethodFailureKind)` linkage.
3. Section 13 method usage follows existing diagnostics text conventions and JSON conversion remains round-trip safe.

## Verification Result

Observer bin tests, the full `worldwake-cli` package test suite, and all-target clippy passed.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` — selected method trace, fallback method trace, failure linkage, and Section 13 method-usage text coverage.
2. `crates/worldwake-cli/src/bin/observer.rs` / `crates/worldwake-cli/src/diagnostics_json.rs` — diagnostics JSON round-trip coverage includes method usage.

### Verification Commands

1. `cargo test -p worldwake-cli --bin observer`
2. `cargo test -p worldwake-cli`
3. `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
