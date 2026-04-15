# S104SURBASREC-003: Fix TellProfile panic in emit_social_candidates

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — candidate generation (worldwake-ai)
**Deps**: None

## Problem

`emit_social_candidates` in `candidate_generation.rs` panics via `unwrap_or_else(|| panic!(...))` when an agent lacks a `TellProfile`. All other ~45 emitter functions use graceful skip patterns (`let Some(profile) = ... else { return; }`). An agent with only survival profiles crashes the simulation instead of simply not generating social goals. This violates FND-22 (Agent Diversity): role-specific profiles must be optional.

## Assumption Reassessment (2026-04-15)

1. `emit_social_candidates` exists at line 1140 of `crates/worldwake-ai/src/candidate_generation.rs`. The panic pattern `unwrap_or_else(|| panic!("agent {} lacks TellProfile", ctx.agent))` is at line 1153 — confirmed via grep during reassessment.
2. All other emitter functions (~45) use graceful skip patterns — confirmed via sampling 5 emitters during reassessment. No other `unwrap()` or `panic!()` for profile access found.
3. `TellProfile` is a role-specific profile defined in `crates/worldwake-core/src/belief.rs` at line 2432. It is optional in `AgentDef` (scenario types).
4. This is a candidate-generation focused fix. The change prevents crash paths but does not alter planning behavior for agents that DO have TellProfile.

## Architecture Check

1. Aligning `emit_social_candidates` with the established pattern used by all other emitters. The graceful skip is the canonical pattern — this fix removes the sole exception.
2. No backwards-compatibility shims introduced. The panic path is replaced with a return, matching existing conventions.

## Verification Layers

1. Agent without TellProfile does not panic → focused unit test or runtime test with survival-only agent profiles
2. Agent WITH TellProfile still generates social candidates → existing KEEP/TRIAGE tests that exercise social candidates (if any survive triage)
3. Single-layer ticket — candidate generation only, no planner or action changes.

## What to Change

### 1. Replace panic with graceful skip in `emit_social_candidates`

In `crates/worldwake-ai/src/candidate_generation.rs`, at line ~1150-1153, change:

```rust
let profile = ctx
    .view
    .tell_profile(ctx.agent)
    .unwrap_or_else(|| panic!("agent {} lacks TellProfile", ctx.agent));
```

To:

```rust
let Some(profile) = ctx.view.tell_profile(ctx.agent) else {
    return;
};
```

This matches the pattern used by `emit_justice_candidates`, `emit_patrol_candidates`, `emit_exploration_candidates`, and all other profile-gated emitters.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — lines ~1150-1153)

## Out of Scope

- Changing any other emitter function
- Modifying TellProfile definition or registration
- Changing GOAP planner behavior
- Adding new candidate generation logic

## Acceptance Criteria

### Tests That Must Pass

1. Agent without TellProfile does not panic when candidate generation runs
2. Agent with TellProfile still generates social goal candidates
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Role-specific profiles are optional — absent profiles cause graceful skips, never panics (FND-22)
2. No change to behavior for agents that have TellProfile

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` (in `#[cfg(test)]` block) — add a focused test that calls candidate generation with an agent lacking TellProfile and confirms no panic and no social candidates emitted. If a suitable test harness already exists for profile-absent scenarios, extend it rather than creating a new one.

### Commands

1. `cargo test -p worldwake-ai -- emit_social` — targeted test
2. `cargo clippy --workspace --all-targets -- -D warnings` — clean
3. `cargo test -p worldwake-ai` — full AI crate suite
