# S36DECGOAREG-005: Migrate decision trace goal labels to declaration-owned labels

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S36DECGOAREG-002

## Problem

`decision_trace.rs` currently renders goals via `Debug` formatting (`format!("{:?}", ...)`) rather than through a stable label surface. The declaration's `trace_label` field (from 002) provides the first stable label. Trace rendering should consume this label where a stable dispatch-family label is the contract, while preserving payload detail where it matters for debugging (P27).

## Assumption Reassessment (2026-03-29)

1. `decision_trace.rs` (3,397 lines) uses `Debug` formatting for goal kinds in several locations. Key rendering sites include `format_outcome()` at line 1088+, `goal_history_entry()` at line 986, and summary functions.
2. The `Debug` impl for `GoalKind` includes full payload data (entity IDs, commodity kinds, recipe IDs, etc.). This is valuable for debugging specific instances but noisy for dispatch-family classification.
3. The declaration `trace_label` (e.g., `"AcquireCommodity(Restock)"`) provides a stable family-level label. For full debugging, the concrete `GoalKind` should still be available as supplementary context.
4. `GoalTraceStatus` enum and related types in `decision_trace.rs` do not need modification — they track status, not rendering.
5. The `dump_agent()` function uses `format!("{:?}", ...)` for goal rendering — this is a primary target for label migration.
6. `omitted_political_reason_for_goal()` (line 1049-1070) and `omitted_social_reason_for_goal()` (line 1072-1084) match on `GoalKind` for classification, not rendering. These are not label consumers.

## Architecture Check

1. Declaration-owned labels provide a stable contract for trace output that doesn't change when `Debug` formatting changes. This supports P27 (Debuggability) by giving traces a well-defined, human-readable label surface. Supplementary payload context (entity IDs, commodity kinds) can still be appended from the concrete `GoalKind` where needed.
2. No backwards-compatibility shims. `Debug` formatting is replaced, not wrapped.

## Verification Layers

1. Label stability → focused test: declaration labels are used in trace output for known goal shapes.
2. Payload preservation → focused test: trace output still includes entity-specific detail where needed for debugging.
3. Single-layer ticket: trace rendering only, no behavioral change.

## What to Change

### 1. Identify trace rendering sites in `decision_trace.rs`

Grep for `"{:?}"` patterns applied to `GoalKind` or goal-related types. Replace family-level labels with `goal.dispatch_key().declaration().trace_label`. Preserve payload detail (entity IDs, commodity kinds) as supplementary context where the rendering site is for debugging, not classification.

### 2. Add helper method

Consider adding a convenience method like `GoalKind::trace_label() -> &'static str` that delegates to `dispatch_key().declaration().trace_label` to reduce verbosity at call sites.

### 3. Update `dump_agent()` output format

Ensure the human-readable dump uses declaration labels for goal family identification.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — replace Debug formatting with declaration labels at rendering sites)
- `crates/worldwake-ai/src/goal_dispatch_key.rs` (modify — optional convenience method)

## Out of Scope

- Modifying `GoalTraceStatus` or trace data structures
- Changing what data is *recorded* in traces (only changing how it's *rendered*)
- Modifying `omitted_political_reason_for_goal()` or `omitted_social_reason_for_goal()` dispatch logic
- Invalidation/feasibility strategy migration (tickets 006–007)
- Any changes to `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. `test_trace_label_used_in_dump`: `dump_agent()` output for a known goal shape contains the declaration `trace_label` string.
2. `test_trace_preserves_payload_detail`: Trace output for an entity-bearing goal (e.g., `EngageHostile`) still includes the target entity ID.
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Zero behavioral change — trace data recorded is unchanged; only rendering changes.
2. All existing golden tests pass unchanged (golden tests do not assert on trace formatting).
3. Declaration labels are the source of truth for family-level goal naming in traces.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` (test module) — label rendering tests for representative goal shapes.

### Commands

1. `cargo test -p worldwake-ai -- trace_label`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
