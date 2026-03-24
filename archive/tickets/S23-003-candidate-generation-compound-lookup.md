# S23-003: Update candidate generation for compound blocker lookup

**Status**: COMPLETED (absorbed into S23-002)
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — candidate generation call site (worldwake-ai)
**Deps**: S23-001

## Problem

`emit_candidate()` and `emit_candidate_with_trace()` call `blocked.is_blocked(&key, current_tick)` with the old two-argument signature. After S23-001, `is_blocked()` takes `(goal_key, place, target, action_def, current_tick)`. These call sites must pass `None` for place/target/action — a global-only check at the candidate generation layer.

## Assumption Reassessment (2026-03-24)

1. `emit_candidate()` at `candidate_generation.rs:~1196` calls `blocked.is_blocked(&key, current_tick)` — confirmed from exploration.
2. `emit_candidate_with_trace()` at `candidate_generation.rs:~1234` calls `blocked.is_blocked(&key, current_tick)` — confirmed.
3. No other call sites of `is_blocked()` exist outside these two functions and the core tests — confirmed by grep.
4. Passing `(goal_key, None, None, None, current_tick)` means: global blockers (NoKnownPath, DangerTooHigh, etc. with `place: None` in BlockerKey) still suppress at candidate generation. Place-specific blockers (SourceDepleted with `place: Some(...)`) do NOT match this query, so the goal is still generated. This is consistent with the existing `blocks_goal_generation()` carve-out but now structural.
5. This is a minimal call-site update — no behavioral change for any existing test scenario.

## Architecture Check

1. The `(None, None, None)` pass-through makes the candidate generation layer explicitly global-only, matching the design intent: "generate the goal, let search prune specific locations."
2. No backward-compatibility shims.

## Verification Layers

1. Global blockers still suppress at candidate generation → existing golden tests pass unchanged
2. Place-specific blockers no longer suppress at candidate generation → verified by S23-004/006 golden tests (not this ticket)
3. Single-layer ticket: call-site signature update only

## What to Change

### 1. `emit_candidate()` — update `is_blocked` call

```rust
if blocked.is_blocked(&key, None, None, None, current_tick) {
    return;
}
```

### 2. `emit_candidate_with_trace()` — update `is_blocked` call

```rust
if blocked.is_blocked(&key, None, None, None, current_tick) {
    return;
}
```

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — two call sites)

## Out of Scope

- **No changes to `blocked_intent.rs`** — that is S23-001
- **No changes to `failure_handling.rs`** — that is S23-002
- **No changes to `search/`** — that is S23-004
- **No changes to `decision_trace.rs`** — that is S23-004/005
- **No new tests in this file** — behavioral parity means existing golden tests suffice
- **Do not change candidate generation logic** beyond the `is_blocked` call signature

## Acceptance Criteria

### Tests That Must Pass

1. All existing golden tests in `cargo test -p worldwake-ai` — no behavioral change
2. `cargo test -p worldwake-ai -- candidate_generation` — if any focused tests exist
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Global blockers (NoKnownPath, DangerTooHigh, CombatTooRisky, Unknown) still suppress candidate generation — unchanged
2. Place-specific blockers (SourceDepleted, ExclusiveFacilityUnavailable, WorkstationBusy, etc.) do NOT suppress candidate generation — now structural via key mismatch rather than `blocks_goal_generation()` carve-out (both mechanisms agree)

## Test Plan

### New/Modified Tests

1. None — call-site signature update only; verification is through existing golden/focused test suites unchanged.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`

## Outcome

**Completion date**: 2026-03-24

**What changed**: Both `emit_candidate()` and `emit_candidate_with_trace()` call sites updated to `is_blocked(&key, None, None, None, current_tick)`. Two test constructions updated for `BlockerKey` / `BTreeMap` patterns.

**Deviations from original plan**: Work absorbed into S23-002 since it was purely mechanical and required for crate compilation after S23-001.

**Verification**: `cargo test --workspace` all pass, `cargo clippy --workspace` no warnings.
