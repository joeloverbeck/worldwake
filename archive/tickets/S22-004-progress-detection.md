# S22-004: Implement progress detection via PlannerOpKind

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new progress tracking in agent_tick step completion
**Deps**: S22-002 (IntentionFrame must be active, with `stalled_ticks` and `last_progress_tick` fields)

## Problem

IntentionFrames track patience via `stalled_ticks` and `last_progress_tick`, but without progress detection, every tick increments the stall counter regardless of whether the agent is making forward progress. The spec defines domain-specific progress: a Travel frame counts only travel-action completions as progress, not eating food mid-journey. Without this, patience would drain even when the agent is actively traveling.

## Assumption Reassessment (2026-03-24)

1. `PlannerOpKind` is defined in `crates/worldwake-ai/src/planner_ops.rs`. It includes `Travel`, `Heal`, `DeclareSupport`, `PressForceClaim`, `YieldForceClaim`, and others.
2. `PlannerOpSemantics` table maps action defs to `PlannerOpKind`. Already used throughout the planning pipeline.
3. Step completion is handled in `agent_tick/mod.rs` (or `active_action.rs` / `execution.rs`). When a plan step completes, the next step is advanced.
4. The `progress_op_kinds()` function must live in worldwake-ai (since `PlannerOpKind` is defined there), not on `IntentionDomain` (which lives in worldwake-core).
5. This is a small, focused ticket: one new function + one integration point in step completion.

## Architecture Check

1. A standalone `progress_op_kinds()` function avoids coupling worldwake-core's `IntentionDomain` to worldwake-ai's `PlannerOpKind`. The function lives in ai and takes `&IntentionDomain` as input.
2. No backward-compatibility concerns — this is new functionality.

## Verification Layers

1. `progress_op_kinds(Travel)` returns `[PlannerOpKind::Travel]` → focused unit test
2. Travel action completion resets `stalled_ticks` → focused test
3. Non-travel action (e.g., eat) during Travel frame does NOT reset `stalled_ticks` → focused test
4. Care frame: both Travel and Heal completions count → focused unit test
5. Golden tests pass → `cargo test -p worldwake-ai`

## What to Change

### 1. New function: `progress_op_kinds()` in `agent_tick/frame.rs`

```rust
pub fn progress_op_kinds(domain: &IntentionDomain) -> &[PlannerOpKind] {
    match domain {
        IntentionDomain::Travel { .. } => &[PlannerOpKind::Travel],
        IntentionDomain::Care { .. } => &[PlannerOpKind::Heal, PlannerOpKind::Travel],
        IntentionDomain::Escort { .. } => &[PlannerOpKind::Travel],
        IntentionDomain::Errand { .. } => &[
            PlannerOpKind::Travel,
            PlannerOpKind::DeclareSupport,
            PlannerOpKind::PressForceClaim,
            PlannerOpKind::YieldForceClaim,
        ],
        IntentionDomain::Generic => GENERIC_PROGRESS_OPS, // all op kinds
    }
}
```

Define `GENERIC_PROGRESS_OPS` as a static slice containing all `PlannerOpKind` variants.

### 2. Integration in step completion

In the agent_tick step completion path (where a plan step finishes and the next step is advanced), look up the completed step's `PlannerOpKind` via the semantics table. If it appears in `progress_op_kinds(frame.domain)`:
- Set `frame.stalled_ticks = 0`
- Set `frame.last_progress_tick = Some(current_tick)`

### 3. Stall increment

Each tick where the frame is `Active` and no progress was recorded:
- Increment `frame.stalled_ticks += 1`

This may already be partially implemented from S22-002's migration of `consecutive_blocked_leg_ticks`. Verify and adjust.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — add `progress_op_kinds()`, `GENERIC_PROGRESS_OPS`)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — integrate progress detection on step completion)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify — if step completion is handled here)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — if step completion is handled here)

## Out of Scope

- Patience exhaustion logic and BlockedIntent creation (S22-005)
- Assumption evaluation (S22-003)
- Decision trace recording of progress events (S22-006)
- Non-Travel frame creation — this ticket implements progress detection for all domains but only Travel frames are created by the current pipeline
- Changes to `PlannerOpKind` enum or `PlannerOpSemantics` table

## Acceptance Criteria

### Tests That Must Pass

1. Focused test: `progress_op_kinds(Travel { .. })` returns exactly `[PlannerOpKind::Travel]`
2. Focused test: `progress_op_kinds(Care { .. })` returns exactly `[PlannerOpKind::Heal, PlannerOpKind::Travel]`
3. Focused test: `progress_op_kinds(Generic)` returns all PlannerOpKind variants
4. Focused test: Travel step completion resets `stalled_ticks` to 0 and sets `last_progress_tick`
5. Focused test: Eat action during Travel frame does NOT reset `stalled_ticks`
6. Focused test: `stalled_ticks` increments by 1 each tick without progress
7. `cargo test -p worldwake-ai` — all golden tests pass
8. `cargo clippy --workspace` — no new warnings

### Invariants

1. `progress_op_kinds()` is deterministic and has no side effects
2. `stalled_ticks` only resets on genuine forward progress, never on non-progress actions
3. `last_progress_tick` is only set when progress occurs, never on stall increment
4. Generic domain treats all op kinds as progress (maximum leniency for unspecialized frames)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/frame.rs` (test module) — `progress_op_kinds` mapping coverage for all 5 domains
2. `crates/worldwake-ai/src/agent_tick/frame.rs` (test module) — progress detection integration: step completion + stall increment

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

- **Completion date**: 2026-03-24
- **What changed**:
  - `frame.rs`: Added `GENERIC_PROGRESS_OPS` (all 22 variants) and `progress_op_kinds()` mapping each `IntentionDomain` to its progress-relevant `PlannerOpKind` slice. 6 unit tests added.
  - `active_action.rs`: Generalized `advance_completed_step` — replaced hardcoded `Travel`-only check with `progress_op_kinds(&domain).contains()`.
  - `mod.rs`: Added per-tick stall increment in `process_agent` before finalization: increments `stalled_ticks` when frame is `Active` and no progress was recorded this tick.
- **Deviations**: `execution.rs` was not touched (step completion lives entirely in `active_action.rs`, called from `observation.rs`). No other deviations.
- **Verification**: 6 new tests pass, all 555+ workspace tests pass, `cargo clippy --workspace` clean.
