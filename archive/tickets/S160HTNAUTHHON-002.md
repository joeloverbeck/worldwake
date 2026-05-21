# S160HTNAUTHHON-002: Honest stage-hint method traces + contract doc

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — HTN method trace (`worldwake-ai`)
**Deps**: archive/tickets/S160HTNAUTHHON-001.md

## Problem

Before this ticket, `MethodPlanAttemptTrace.subgoals_attempted` recorded each
subgoal's `kind` and `outcome`, but not whether the subgoal was a mere stage
hint or an enforced leaf.
A reader of the trace cannot distinguish "method selected" from "subgoal enforced,"
which lets the trace decorate intended behavior rather than prove what the code
actually constrained (FND-29: traces must prove what the code constrained). This
ticket surfaces the `MethodSubgoalAuthority` label (from archived
`archive/tickets/S160HTNAUTHHON-001.md`) in the trace
and documents the stage-hint-vs-required-leaf distinction in the canonical HTN trace
contract.

## Assumption Reassessment (2026-05-21)

1. Before this ticket, `decision_trace.rs::SubgoalAttemptResult` held
   `template_index`, `kind: SubgoalAttemptKind`, and
   `outcome: SubgoalAttemptOutcome`, while `MethodPlanAttemptTrace` held
   `subgoals_attempted: Vec<SubgoalAttemptResult>` with no authority field.
   This ticket added that field.
2. `search/strategic.rs` was the live producer of `MethodPlanAttemptTrace` and
   `subgoals_attempted`, so it became the owner for reading authority labels from
   the selected method's subgoals and copying them into each
   `SubgoalAttemptResult`. `htn/selector.rs` selects the method whose subgoals
   carry the labels.
3. Shared boundary under audit: the `MethodPlanAttemptTrace` read-model contract,
   documented in `docs/planner-contracts.md` §4 (lines 295–326). Per that contract
   the trace is a transient debug read-model, not serialized save/replay state — so
   adding an authority field has no save-format impact.
4. Depends on `archive/tickets/S160HTNAUTHHON-001.md` having landed the
   `MethodSubgoalAuthority` enum and per-subgoal labels on `MethodSchema.subgoals`;
   without them there is nothing to copy into the trace.
5. Existing tests touching the trace: `decision_trace.rs:5438` constructed a
   `MethodPlanAttemptTrace` in a unit test; observer (`observer.rs:7398/7443/7471`)
   and `scenario_diagnostics/aggregator.rs:991/1000` also constructed it. Direct
   `SubgoalAttemptResult` construction sites were updated to supply the new field;
   empty-vector `MethodPlanAttemptTrace` samples required no code change. Golden
   HTN method coverage exists in
   `tests/scenarios/htn_methods.rs` (references `MethodPlanAttemptTrace` at
   lines 868, 1125).

## Architecture Check

1. The authority label rides on the existing `SubgoalAttemptResult` rather than a
   second trace subsystem — extends the documented `MethodPlanAttemptTrace` contract
   in place (FND-29). A selected method with only stage hints is now provably *not*
   reported as having enforced its subgoals.
2. No backward-compatibility shim: the new field is added to the existing struct and
   every construction site is updated in this ticket. The trace remains a transient
   read-model (not promoted to authoritative truth, FND-27).

## Verified Layers

1. Trace distinguishes stage-hint subgoals from enforced leaves -> focused
   strategic-search test asserting each `SubgoalAttemptResult.authority` matches
   the selected method's declared per-subgoal authority, using a mixed
   `StageHint` / `RequiredActionLeaf` fixture to prove the field is copied rather
   than defaulted.
2. Current registered methods are not reported as enforcing their subgoals ->
   already-landed dependency proof in `archive/tickets/S160HTNAUTHHON-001.md`
   still proves all current registered method subgoals are `StageHint`; this
   ticket preserves those labels in the trace.
3. Single authoritative-state mutation: none — `MethodPlanAttemptTrace` is a
   transient read-model, so no event-log / world-state layer applies (per
   `docs/planner-contracts.md` §4).

## Landed Changes

### 1. Added `authority` to `SubgoalAttemptResult`

Added `pub authority: MethodSubgoalAuthority` to `SubgoalAttemptResult`
beside the existing `template_index`, `kind`, and `outcome` fields.

### 2. Populated authority where the trace is built

`search/strategic.rs` now reads the authority from the selected method subgoal
and copies it into each `SubgoalAttemptResult` alongside `kind`/`outcome` for
both selected-method traces and method-produced-no-stages fallback traces.

### 3. Updated all `SubgoalAttemptResult` construction sites

Updated direct `SubgoalAttemptResult` struct literals in `decision_trace.rs` and
`observer.rs` to supply `StageHint` sample values. The reassessed
`scenario_diagnostics/aggregator.rs` sites construct `MethodPlanAttemptTrace`
with an empty `subgoals_attempted` vector, so they needed no code change.

### 4. Documented the distinction in `docs/planner-contracts.md` §4 (covers D5)

The HTN trace contract now states that each `SubgoalAttemptResult` records
`MethodSubgoalAuthority`, distinguishes `StageHint` from `RequiredActionLeaf`,
and that all current methods declare only `StageHint` until a future
method-required contract adds the corresponding search check and trace proof.

## Landed Files

- `crates/worldwake-ai/src/decision_trace.rs` (modified — field + unit-test construction)
- `crates/worldwake-ai/src/search/strategic.rs` (modified — populate authority + focused propagation assertion)
- `crates/worldwake-cli/src/bin/observer.rs` (modified — test construction sites)
- `docs/planner-contracts.md` (modified — §4 contract language, D5)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (reassessed no-change — empty subgoal vectors required no new field)

## Out of Scope

- Defining `MethodSubgoalAuthority` or labeling method subgoals —
  `archive/tickets/S160HTNAUTHHON-001.md`.
- Any strategic-search *enforcement* of `RequiredActionLeaf` — deferred to a future
  method-required spec; this ticket only surfaces the label in the trace.
- Observer rendering format beyond supplying the new field at sample-construction
  sites.

## Acceptance Result

### Tests Passed

1. Focused strategic-search test: each `SubgoalAttemptResult.authority` matches
   the selected method's declared subgoal authority.
2. Current registered methods remain stage-hint-only through the archived ticket
   001 registry tests; this ticket preserves that classification in trace output.
3. Existing suite passed: `cargo test -p worldwake-ai`.

### Invariants

1. Every `SubgoalAttemptResult` carries the authority of the subgoal it reports;
   no defaulted-at-render authority.
2. `MethodPlanAttemptTrace` remains a transient debug read-model — not serialized
   save/replay state (no `SAVE_FORMAT_VERSION` impact).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/search/strategic.rs` — extended the existing
   `method_selection_substitutes_method_subgoals_into_stage_list` test with a
   mixed-authority method fixture and assertions over `subgoals_attempted`.
2. `crates/worldwake-ai/src/decision_trace.rs` and
   `crates/worldwake-cli/src/bin/observer.rs` — updated existing trace
   construction tests to supply the new field.

### Commands Passed

1. `cargo test -p worldwake-ai --lib search::strategic::tests::method_selection_substitutes_method_subgoals_into_stage_list -- --exact`
2. `cargo test -p worldwake-ai --lib decision_trace::tests::method_plan_attempt_trace_records_selected_method_and_pending_subgoals -- --exact`
3. `cargo test -p worldwake-cli --bin observer render_method_trace`
4. `cargo test -p worldwake-ai`
5. `cargo clippy -p worldwake-ai -p worldwake-cli --all-targets -- -D warnings`
6. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-21.

- Added `MethodSubgoalAuthority` to `SubgoalAttemptResult`, making selected
  method traces carry the authority label attached to each selected method
  subgoal.
- Wired `search/strategic.rs` selected-method and fallback trace construction to
  copy the authority from the method schema instead of defaulting it at render or
  read time.
- Updated direct trace construction tests and the observer binary test samples.
- Documented the stage-hint-vs-required-leaf trace contract in
  `docs/planner-contracts.md` §4.

## Deviations

- The focused propagation assertion landed in `search/strategic.rs`, not
  `decision_trace.rs`, because strategic search is the producer that has both the
  selected method schema and the emitted `SubgoalAttemptResult` values in scope.
- `scenario_diagnostics/aggregator.rs` was reassessed but unchanged because its
  local `MethodPlanAttemptTrace` samples use empty `subgoals_attempted` vectors.
- Observer rendering behavior was intentionally unchanged; the ticket's observer
  scope was constructor fallout, while the canonical trace contract is the
  structured `SubgoalAttemptResult.authority` field.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib search::strategic::tests::method_selection_substitutes_method_subgoals_into_stage_list -- --exact`
- Passed `cargo test -p worldwake-ai --lib decision_trace::tests::method_plan_attempt_trace_records_selected_method_and_pending_subgoals -- --exact`
- Passed `cargo test -p worldwake-cli --bin observer render_method_trace`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy -p worldwake-ai -p worldwake-cli --all-targets -- -D warnings`
- Passed `./scripts/verify.sh`
