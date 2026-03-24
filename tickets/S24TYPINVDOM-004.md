# S24TYPINVDOM-004: Remove DirtyReason enum, migrate traces to DirtySet

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `DirtyReason` enum removed, `PlanningPipelineTrace.dirty_reasons` replaced with `dirty: DirtySet`, `ReadPhaseResult.dirty_reasons` field removed, trace output updated
**Deps**: S24TYPINVDOM-003 (observation phase no longer produces Vec<DirtyReason>, planning functions no longer consume it)

## Problem

After S24TYPINVDOM-003, the `DirtyReason` enum and `Vec<DirtyReason>` are vestigial — they exist only for trace construction in `PlanningPipelineTrace` and the temporary conversion bridge in `ReadPhaseResult`. The authoritative dirty state is now `DirtySet` on `AgentDecisionRuntime`. This ticket removes the enum, replaces the trace field, removes the bridge, and updates trace output to display typed domain names.

## Assumption Reassessment (2026-03-24)

1. `DirtyReason` enum at `decision_trace.rs:597-605` with 7 variants. After -003, no site constructs `DirtyReason` values except the conversion bridge in `ReadPhaseResult`.
2. `PlanningPipelineTrace.dirty_reasons: Vec<DirtyReason>` at `decision_trace.rs:143`. Only consumer is `mod.rs:581` where `read_result.dirty_reasons` flows in.
3. `ReadPhaseResult.dirty_reasons: Vec<DirtyReason>` at `observation.rs:46`. After -003, populated by a conversion bridge from `runtime.dirty`.
4. `DirtyReason` is exported from `lib.rs:40`. After removal, `DirtySet` replaces it.
5. `format_outcome()` at `decision_trace.rs:823` does NOT currently include dirty reasons in output. The spec adds `dirty.display_names()` to both `format_outcome()` and `summary()`.
6. `summary()` at `decision_trace.rs:97` does NOT currently include dirty reasons. Same addition.
7. Tests in `decision_trace.rs` construct `PlanningPipelineTrace` with `dirty_reasons: vec![DirtyReason::NoPlan]` at lines 1494, 1579, 1633, 1907, and `dirty_reasons: Vec::new()` at line 1136. All must migrate to `dirty: DirtySet::NO_PLAN` or `dirty: DirtySet::default()`.
8. Test at `agent_tick/tests.rs:3702` that previously asserted on `read_result.dirty_reasons` was updated in -003. After this ticket, the `dirty_reasons` field on `ReadPhaseResult` is removed entirely.
9. `DirtyReason` import at `agent_tick/tests.rs:19` must be removed/replaced with `DirtySet`.
11. No mismatch.

## Architecture Check

1. Removing `DirtyReason` eliminates the dual representation (enum + bitflag) that was the core divergence risk. `DirtySet` becomes the single source of truth for both runtime logic and trace diagnostics.
2. Adding dirty domain names to `summary()` and `format_outcome()` satisfies the spec's debuggability goal (P27).
3. No backwards-compatibility shims — the enum is deleted outright.

## Verification Layers

1. `DirtyReason` has zero references remaining in codebase → `cargo build` + grep verification
2. `PlanningPipelineTrace.dirty: DirtySet` carries typed domains → trace output tests verify `display_names()` content
3. `summary()` includes dirty domain names → updated test assertions
4. `format_outcome()` includes dirty domain names → updated test or manual verification via `dump_agent()`
5. All golden tests pass → behavioral equivalence

## What to Change

### 1. Replace `PlanningPipelineTrace.dirty_reasons` with `dirty: DirtySet`

At `decision_trace.rs:143`:
- `pub dirty_reasons: Vec<DirtyReason>` → `pub dirty: DirtySet`
- Update doc comment on `plan_continued` (line 144-146) to reference `dirty.is_snapshot_only()` instead of `SnapshotChanged`

### 2. Update trace construction in `mod.rs`

At `mod.rs:580-581`:
- `dirty_reasons: read_result.dirty_reasons` → `dirty: runtime.dirty`
- Note: `runtime.dirty` has been cleared to `DirtySet::default()` by planning. Need to capture `runtime.dirty` BEFORE planning clears it. Add a `let dirty_snapshot = runtime.dirty;` before the planning call, then use `dirty: dirty_snapshot` in trace construction.

### 3. Remove `ReadPhaseResult.dirty_reasons` field

At `observation.rs:46`: remove the field entirely. Remove the conversion bridge that populated it (introduced in -003). Remove `dirty_reasons` from `ReadPhaseResult` construction site in `refresh_runtime_for_read_phase()`.

### 4. Remove `DirtyReason` enum

At `decision_trace.rs:593-605`: delete the enum definition and its section comment.

### 5. Update `lib.rs` exports

Remove `DirtyReason` from `pub use decision_trace::{...}`. Ensure `DirtySet` is already exported (from -001).

### 6. Remove all `DirtyReason` imports

Remove `DirtyReason` from import statements across:
- `agent_tick/tests.rs:19`
- `agent_tick/observation.rs` (if still imported)
- Any other file

### 7. Update `summary()` to include dirty domains

At `decision_trace.rs:97` in `DecisionOutcome::Planning` arm, add dirty domain display. E.g.:
```
format!("PLAN (dirty: {dirty}): selected=...
```
where `dirty` is `planning.dirty.display_names()`.

### 8. Update `format_outcome()` to include dirty domains

At `decision_trace.rs:845` in `DecisionOutcome::Planning` arm, include `planning.dirty` in formatted output. E.g. prepend `dirty: NEEDS|POSITION` to the PLAN line.

### 9. Update tests constructing `PlanningPipelineTrace`

5 test sites in `decision_trace.rs`:
- Line 1136: `dirty_reasons: Vec::new()` → `dirty: DirtySet::default()`
- Line 1494: `dirty_reasons: vec![DirtyReason::NoPlan]` → `dirty: DirtySet::NO_PLAN`
- Line 1579: same
- Line 1633: same
- Line 1907: same

Update test assertions on `summary()` output to expect dirty domain names in the output string.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — remove enum, change field, update format/summary, update tests)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — capture dirty before planning, update trace construction)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — remove `dirty_reasons` field and bridge from `ReadPhaseResult`)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — remove `DirtyReason` import, update any remaining references)
- `crates/worldwake-ai/src/lib.rs` (modify — remove `DirtyReason` from exports)

## Out of Scope

- Modifying `AgentDecisionRuntime.dirty` field type (already `DirtySet` from S24TYPINVDOM-002)
- Modifying `observation_snapshot_changed()` (already returns `DirtySet` from S24TYPINVDOM-003)
- Modifying planning function signatures (already updated in S24TYPINVDOM-003)
- Modifying mutation/clear/read sites for `runtime.dirty` (already done in S24TYPINVDOM-002)
- Touching any crate other than `worldwake-ai`
- Adding new golden tests (S24TYPINVDOM-005 covers final verification)

## Acceptance Criteria

### Tests That Must Pass

1. No `DirtyReason` references remain in codebase (verified by `grep -r "DirtyReason" crates/` returning only archive/spec files)
2. `summary()` output includes dirty domain names for Planning outcomes (e.g., "PLAN (dirty: NO_PLAN): selected=...")
3. `format_outcome()` output includes dirty domain names for Planning outcomes
4. All 5 test construction sites in `decision_trace.rs` compile with `dirty: DirtySet::...`
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `DirtySet` is the single representation for invalidation domains — no parallel `Vec<DirtyReason>` exists anywhere
2. Trace output preserves all information that `Vec<DirtyReason>` carried, plus adds per-snapshot-dimension granularity
3. `dirty` snapshot for trace is captured before planning clears it — traces reflect the state that triggered planning, not the post-planning clean state
4. No `DirtyReason` type exists in compiled code

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` — update 5 test construction sites, update `summary()` assertion strings to expect dirty domain names
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — remove `DirtyReason` import, update any residual references

### Commands

1. `cargo test -p worldwake-ai` — full crate regression
2. `cargo clippy -p worldwake-ai` — no new warnings
3. `cargo build --workspace` — cross-crate compilation
4. `grep -r "DirtyReason" crates/` — verify no remaining references in source code
