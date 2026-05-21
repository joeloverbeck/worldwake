# S160HTNAUTHHON-002: Honest stage-hint method traces + contract doc

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — HTN method trace (`worldwake-ai`)
**Deps**: S160HTNAUTHHON-001

## Problem

`MethodPlanAttemptTrace.subgoals_attempted` records each subgoal's `kind` and
`outcome`, but not whether the subgoal was a mere stage hint or an enforced leaf.
A reader of the trace cannot distinguish "method selected" from "subgoal enforced,"
which lets the trace decorate intended behavior rather than prove what the code
actually constrained (FND-29: traces must prove what the code constrained). This
ticket surfaces the `MethodSubgoalAuthority` label (from ticket 001) in the trace
and documents the stage-hint-vs-required-leaf distinction in the canonical HTN trace
contract.

## Assumption Reassessment (2026-05-21)

1. `decision_trace.rs::SubgoalAttemptResult` (lines 1269–1274) holds
   `template_index`, `kind: SubgoalAttemptKind`, `outcome: SubgoalAttemptOutcome`.
   `MethodPlanAttemptTrace` (line 1248) holds `subgoals_attempted:
   Vec<SubgoalAttemptResult>`. No authority field exists yet.
2. `search/strategic.rs` builds `MethodPlanAttemptTrace` at lines 468–469 and
   492–493 and populates `subgoals_attempted`; this is where the authority label
   must be read from the selected method's subgoals and copied into each
   `SubgoalAttemptResult`. `htn/selector.rs` selects the method whose subgoals carry
   the labels.
3. Shared boundary under audit: the `MethodPlanAttemptTrace` read-model contract,
   documented in `docs/planner-contracts.md` §4 (lines 295–326). Per that contract
   the trace is a transient debug read-model, not serialized save/replay state — so
   adding an authority field has no save-format impact.
4. Depends on ticket 001's `MethodSubgoalAuthority` enum and per-subgoal labels
   being present on `MethodSchema.subgoals`; without them there is nothing to copy
   into the trace.
5. Existing tests touching the trace: `decision_trace.rs:5438` constructs a
   `MethodPlanAttemptTrace` in a unit test; observer (`observer.rs:7398/7443/7471`)
   and `scenario_diagnostics/aggregator.rs:991/1000` also construct it. Each
   construction site of `SubgoalAttemptResult`/`MethodPlanAttemptTrace` must supply
   the new field. Golden HTN method coverage exists in
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

## Verification Layers

1. Trace distinguishes stage-hint subgoals from enforced leaves -> decision-trace
   focused test asserting each `SubgoalAttemptResult.authority` matches the selected
   method's declared subgoal authority.
2. A stage-hint-only method is not reported as enforcing its subgoals -> decision-trace
   assertion that all `subgoals_attempted` carry `StageHint` for the canonical
   group-hunt / produce methods.
3. Single new authoritative-state mutation: none — `MethodPlanAttemptTrace` is a
   transient read-model, so no event-log / world-state layer applies (per
   `docs/planner-contracts.md` §4).

## What to Change

### 1. Add `authority` to `SubgoalAttemptResult`

Add `pub authority: MethodSubgoalAuthority` to `SubgoalAttemptResult`
(`decision_trace.rs:1269`). Keep the existing `kind`/`outcome`/`template_index`
fields.

### 2. Populate authority where the trace is built

In `search/strategic.rs` (the `subgoals_attempted` population at the trace-build
sites near lines 468 and 492), read the authority from the selected method's
subgoal (introduced in ticket 001) and copy it into each `SubgoalAttemptResult`
alongside `kind`/`outcome`.

### 3. Update all `SubgoalAttemptResult` / `MethodPlanAttemptTrace` construction sites

Supply the new field at every construction site: `decision_trace.rs:5438` (unit
test), `scenario_diagnostics/aggregator.rs` (lines 991/1000), and
`observer.rs` (lines 7398/7443/7471). For diagnostic/observer sample constructions,
`StageHint` is the correct sample value.

### 4. Document the distinction in `docs/planner-contracts.md` §4 (covers D5)

Add to the §4 HTN trace contract language (lines 295–326): each
`SubgoalAttemptResult` records a `MethodSubgoalAuthority` so a reader can tell a
stage-hint subgoal (not enforced as an ActionDef leaf) from a `RequiredActionLeaf`
(enforced; trace proves selected/skipped/failed). Note that all current methods
declare only `StageHint`.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — field + unit-test construction)
- `crates/worldwake-ai/src/search/strategic.rs` (modify — populate authority)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (modify — construction sites)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — construction sites)
- `docs/planner-contracts.md` (modify — §4 contract language, D5)

## Out of Scope

- Defining `MethodSubgoalAuthority` or labeling method subgoals — ticket 001.
- Any strategic-search *enforcement* of `RequiredActionLeaf` — deferred to a future
  method-required spec; this ticket only surfaces the label in the trace.
- Observer rendering format beyond supplying the new field at sample-construction
  sites.

## Acceptance Criteria

### Tests That Must Pass

1. New decision-trace test: each `SubgoalAttemptResult.authority` matches the
   selected method's declared subgoal authority.
2. New decision-trace test: a stage-hint-only selected method reports all
   `subgoals_attempted` as `StageHint` (not as enforced).
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Every `SubgoalAttemptResult` carries the authority of the subgoal it reports;
   no defaulted-at-render authority.
2. `MethodPlanAttemptTrace` remains a transient debug read-model — not serialized
   save/replay state (no `SAVE_FORMAT_VERSION` impact).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` (or `search/tests.rs`) — focused
   trace tests asserting authority propagation from selected method to
   `subgoals_attempted`.
2. Update the existing `decision_trace.rs:5438` trace-construction test to supply
   the new field.

### Commands

1. `cargo test -p worldwake-ai htn:: search::`
2. `cargo clippy -p worldwake-ai -p worldwake-cli --all-targets -- -D warnings`
3. `./scripts/verify.sh`
