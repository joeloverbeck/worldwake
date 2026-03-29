# S36DECGOAREG-005: Migrate decision trace goal labels to declaration-owned labels

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S36DECGOAREG-002

## Problem

`decision_trace.rs` currently renders goals via `Debug` formatting (`format!("{:?}", ...)`) rather than through a stable label surface. The declaration's `trace_label` field (from 002) provides the first stable label. Trace rendering should consume this label where a stable dispatch-family label is the contract, while preserving payload detail where it matters for debugging (P27).

## Assumption Reassessment (2026-03-29)

1. `crates/worldwake-ai/src/goal_dispatch_key.rs` and `crates/worldwake-ai/src/goal_dispatch_decl.rs` already implement the S36 declaration substrate: `GoalDispatchKey::from_goal_kind()`, `GoalDispatchKey::all()`, and `GoalDispatchKey::declaration().trace_label` all exist in live code. The original ticket assumption that declaration-owned labels still needed to be introduced is stale. This ticket is now strictly about migrating the remaining trace-rendering call sites onto that already-shipped label surface.
2. `crates/worldwake-ai/src/decision_trace.rs` (3,397 lines) still renders `GoalKind` via raw `Debug` in the decision-trace presentation layer. Live call sites include `DecisionOutcome::summary()` (`selected={:?}` and `replacement={:?}` for goal kinds), `format_outcome()` (`selected={:?}`, `fully blocked desire: goal={:?}`, `plan attempt: goal={:?}`, `unknown blockers active: goal={:?}`), `format_opportunity_key()`, and `format_same_goal_planning_trace_summary()` (`EncounteredDifferentGoal({:?})`).
3. The exact abstraction boundary under audit is the AI trace-rendering surface in `crates/worldwake-ai/src/decision_trace.rs`: stored trace data continues to carry full `GoalKind`/`GoalKey` values, while human-readable summaries should identify goal families through `GoalDispatchDeclaration.trace_label`.
4. The live `Debug` rendering of `GoalKind` still carries useful payload detail (commodity kind, target entity, office, punishment payload, etc.). Replacing it outright with only a family label would make traces less useful for P27 debugging. The corrected scope is therefore: use declaration-owned labels as the stable prefix and preserve payload detail as supplementary context at the rendering boundary.
5. `GoalTraceStatus`, `goal_history_entry()`, `omitted_political_reason_for_goal()`, and `omitted_social_reason_for_goal()` are not part of the bug. They classify or summarize trace state and do not currently define the human-readable goal-label contract.
6. This is a single-layer ticket. No AI behavior, candidate generation, planner search, or authoritative runtime path is changing. The only live symbol family under test is `GoalKind` rendering inside decision-trace summaries.
7. `cargo test -p worldwake-ai -- --list` confirms the live test binary layout. Existing focused coverage already exercises `DecisionOutcome::summary()` and `format_outcome()` inside `crates/worldwake-ai/src/decision_trace.rs`; this ticket should strengthen those focused tests rather than invent a new golden dependency.
8. Mismatch correction: `crates/worldwake-ai/src/goal_dispatch_key.rs` does not need a new convenience method for this ticket. Adding a repo-wide label helper would broaden scope without architectural need. A local trace-format helper in `decision_trace.rs` is the cleaner boundary because the bug is presentation-only.

## Architecture Check

1. The cleaner architecture is to keep `GoalKind` as the authoritative stored payload and move only the presentation policy into a small formatter inside `decision_trace.rs`. That keeps the declaration-owned label contract at the trace boundary without leaking trace-specific formatting helpers across the AI crate.
2. Using `trace_label` as the stable prefix and appending concrete payload detail only when the payload adds information is better than either extreme: it avoids unstable all-`Debug` rendering, and it avoids an under-specified label-only surface that would erase useful debugging context.
3. No backwards-compatibility shims or alias paths. The trace renderer should stop presenting raw `Debug` as the primary label surface; it should render the declaration label contract directly.

## Verification Layers

1. Stable family labeling in decision summaries -> focused unit tests over `DecisionOutcome::summary()` and `format_outcome()`.
2. Payload preservation for debugging -> focused unit tests asserting the rendered string still includes concrete payload detail for entity-bearing and commodity-bearing goals.
3. Single-layer ticket: no additional action-trace, event-log, or authoritative-state mapping is applicable because stored trace data and engine behavior do not change.

## What to Change

### 1. Replace raw `GoalKind`-label rendering in `decision_trace.rs`

Add a local helper in `crates/worldwake-ai/src/decision_trace.rs` that formats a `GoalKind` for human-readable traces by starting from `GoalDispatchKey::from_goal_kind(goal).declaration().trace_label` and appending concrete payload detail when the concrete `Debug` payload contains information beyond the stable label.

Apply that helper consistently at the remaining goal-rendering sites in `DecisionOutcome::summary()`, `format_outcome()`, `format_opportunity_key()`, and `format_same_goal_planning_trace_summary()`.

### 2. Strengthen focused trace-format tests

Update the existing `decision_trace.rs` unit coverage to prove both halves of the contract:
- declaration-owned labels are the primary rendered goal-family labels
- supplementary payload detail is still visible where debugging depends on it

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — replace primary `GoalKind` `Debug` labels with declaration-label rendering and strengthen focused tests)

## Out of Scope

- Modifying `GoalTraceStatus` or trace data structures
- Changing what data is *recorded* in traces (only changing how it's *rendered*)
- Modifying `omitted_political_reason_for_goal()` or `omitted_social_reason_for_goal()` dispatch logic
- Adding new helpers or APIs in `goal_dispatch_key.rs`, `goal_dispatch_decl.rs`, or `goal_model.rs`
- Invalidation/feasibility strategy migration (tickets 006–007)
- Any changes to `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. `DecisionOutcome::summary()` for a payload-bearing goal renders the declaration `trace_label` as the primary family label.
2. `DecisionOutcome::summary()` and/or `format_outcome()` still include payload detail for representative payload-bearing goals (for example `EngageHostile { target }` or `AcquireCommodity { commodity, purpose }`).
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Zero behavioral change — trace data recorded is unchanged; only rendering changes.
2. All existing golden tests pass unchanged (golden tests do not assert on trace formatting).
3. Declaration labels are the source of truth for family-level goal naming in traces.
4. Raw `Debug` may remain as supplementary payload context, but not as the primary family-label contract.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` (test module) — summary/formatting tests proving declaration-label-first rendering for representative goal shapes.
2. `crates/worldwake-ai/src/decision_trace.rs` (test module) — payload-preservation tests proving entity/commodity detail remains visible in formatted output.

### Commands

1. `cargo test -p worldwake-ai summary_planning`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-29
- What changed: `crates/worldwake-ai/src/decision_trace.rs` now renders goal-family names through `GoalDispatchDeclaration.trace_label` and appends concrete `GoalKind` payload detail as supplementary context when the payload adds information. The change covers `DecisionOutcome::summary()`, `format_outcome()`, `format_opportunity_key()`, and same-goal stop summaries.
- Deviations from original plan: no changes were needed in `crates/worldwake-ai/src/goal_dispatch_key.rs` or the declaration substrate because that architecture already existed in live code before this ticket. The ticket was corrected first and then implemented as a decision-trace presentation fix only.
- Verification results: `cargo test -p worldwake-ai summary_planning`, `cargo test -p worldwake-ai`, and `cargo clippy --workspace` all passed on 2026-03-29.
