# S88TWOPHALAN-001: Add `landmark_extraction_depth` to CognitiveProfile

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — extends existing component with new field
**Deps**: None

## Problem

The two-phase landmark-guided planner (S88) needs a per-agent parameter controlling landmark chain extraction depth. Without this field, landmark extraction cannot be configured per-agent, violating FND-22 (Agent Diversity). Agents with `landmark_extraction_depth = 0` disable landmarks entirely (graceful degradation to current behavior).

## Assumption Reassessment (2026-04-11)

1. `CognitiveProfile` exists at `crates/worldwake-core/src/cognitive_profile.rs:6` with 13 fields. Default impl at line 24. Component registration confirmed via `impl Component for CognitiveProfile` at line 44. Struct literal construction sites: 34 occurrences across 16 files in `crates/` — all must gain the new field.
2. `AgentDef` uses `Option<CognitiveProfile>` at `crates/worldwake-cli/src/scenario/types.rs:86`. `spawn_agent()` applies via `unwrap_or_default()` at `crates/worldwake-cli/src/scenario/mod.rs:1445`. New fields with defaults are picked up automatically — no CLI changes needed.
3. Shared boundary: `CognitiveProfile` is defined in `worldwake-core` and read by `worldwake-ai`, `worldwake-sim`, and `worldwake-cli`. Adding a field with a default is a non-breaking extension of this data contract.

## Architecture Check

1. Adding a field to an existing profile struct with a Default impl is the cleanest extension pattern — no new components, no new registration, no cross-crate API changes. The field is read-only from the AI layer's perspective.
2. No backwards-compatibility shims. All construction sites are updated to include the new field explicitly or via `..Default::default()`.

## Verification Layers

1. Field exists with correct type and default → focused unit test in `cognitive_profile.rs`
2. Serialization roundtrip → existing `cognitive_profile_roundtrips_through_bincode` test (updated)
3. Component registration unchanged → existing `cognitive_profile_registers_for_agents` test
4. Single-layer ticket (core data struct extension) — no cross-layer mapping needed.

## What to Change

### 1. Add field to `CognitiveProfile` struct

In `crates/worldwake-core/src/cognitive_profile.rs`, add after `speculative_acquisition`:

```rust
/// Maximum depth of landmark chain extraction during tactical planning.
/// Higher values produce more landmarks for better search guidance at
/// increased extraction cost. 0 = no landmarks (preferred operators disabled).
pub landmark_extraction_depth: u8,
```

### 2. Update Default impl

Add `landmark_extraction_depth: 4` to the Default impl.

### 3. Update default assertion test

Add `assert_eq!(profile.landmark_extraction_depth, 4);` to `cognitive_profile_default_matches_split_defaults`.

### 4. Update bincode roundtrip test

Add `landmark_extraction_depth: 5` (or similar non-default) to the test fixture in `cognitive_profile_roundtrips_through_bincode`.

### 5. Update explicit struct literal construction sites

Only full `CognitiveProfile { ... }` literals require edits. Call sites that use
`..CognitiveProfile::default()` pick up the new field automatically. The required
explicit constructor fallout is:

- `crates/worldwake-core/src/cognitive_profile.rs` (roundtrip fixture)
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-ai/src/failure_handling.rs`
- `crates/worldwake-ai/src/decision_runtime.rs`
- `crates/worldwake-ai/src/search/tests.rs`
- `crates/worldwake-ai/src/goal_model.rs`
- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `crates/worldwake-ai/src/agent_tick/tests.rs`

One existing partial test fixture in `crates/worldwake-sim/src/per_agent_belief_view.rs`
also now sets a non-default `landmark_extraction_depth` explicitly to prove the
new field survives profile reads.

## Files to Touch

- `crates/worldwake-core/src/cognitive_profile.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)

## Out of Scope

- Using `landmark_extraction_depth` in the planner (that's S88TWOPHALAN-007)
- Modifying scenario files to set non-default values
- Any behavioral changes to planning

## Acceptance Criteria

### Tests That Must Pass

1. `cognitive_profile_default_matches_split_defaults` — asserts default is 4
2. `cognitive_profile_roundtrips_through_bincode` — roundtrip with non-default value
3. `cognitive_profile_registers_for_agents` — unchanged, still passes
4. Existing suite: `cargo test -p worldwake-core -- cognitive_profile`
5. Existing suite: `cargo test --workspace`

### Invariants

1. `CognitiveProfile::default().landmark_extraction_depth == 4`
2. All existing tests pass without behavioral changes (field is added but not yet consumed)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/cognitive_profile.rs::cognitive_profile_default_matches_split_defaults` — add assertion for new field default
2. `crates/worldwake-core/src/cognitive_profile.rs::cognitive_profile_roundtrips_through_bincode` — add field to test fixture

### Commands

1. `cargo test -p worldwake-core -- cognitive_profile`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

Completed on 2026-04-11.

- Added `landmark_extraction_depth: u8` to `CognitiveProfile` with default `4`
  and updated the core default/roundtrip coverage accordingly.
- Updated every full `CognitiveProfile` struct literal that would otherwise have
  gone stale after the shared-type extension.
- Reassessment initially overestimated constructor fallout: call sites using
  `..CognitiveProfile::default()` required no edits because the new field is
  inherited automatically.
- Added one explicit non-default profile read in
  `crates/worldwake-sim/src/per_agent_belief_view.rs` so the new field is part
  of an existing profile-surface proof.

## Verification Result

- Passed `cargo test -p worldwake-core -- cognitive_profile`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
