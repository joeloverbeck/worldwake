# S73PLASNAENT-001: Add `max_snapshot_entities_per_place` to CognitiveProfile

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-core CognitiveProfile component
**Deps**: None

## Problem

S73 introduces a per-place entity cap for planning snapshot construction. The cap must live on `CognitiveProfile` so it is per-agent (P22), scenario-definable via `AgentDef`, and serializable. This ticket adds the field with no behavioral consumers yet — ticket 002 wires it in.

## Assumption Reassessment (2026-04-08)

1. `CognitiveProfile` exists at `crates/worldwake-core/src/cognitive_profile.rs:6` with 10 fields, derives `Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize`, and implements `Component`. Adding a `u16` field satisfies all existing trait bounds (`Copy`, `Serialize`, `Deserialize`, `Eq`, `Ord`).
2. `Default` impl at line 19 must be extended. Existing test `cognitive_profile_default_matches_split_defaults` (line 56) must be updated to assert the new field.
3. Bincode roundtrip test at line 72 must include the new field in the non-default profile.
4. `CognitiveProfile` is already in `AgentDef` at `crates/worldwake-cli/src/scenario/types.rs:86` as `Option<CognitiveProfile>` and spawned with `unwrap_or_default()` at `crates/worldwake-cli/src/scenario/mod.rs:297`. No changes needed there — the field participates automatically through the struct.
5. S73 spec (reassessed 2026-04-08) specifies default 50 and type `u16`.
6. The initial single-file scope was too narrow. `cargo test -p worldwake-core -- cognitive_profile` surfaced a manual `ComponentValue::CognitiveProfile(...)` literal in `crates/worldwake-core/src/delta.rs` that also required the new field. Correction applied safely because the test fixture already seeds non-default `CognitiveProfile` values explicitly.
7. `cargo clippy --workspace --all-targets -- -D warnings` then surfaced six test-only `CognitiveProfile` helper constructors in `crates/worldwake-ai` (`agent_tick/planning.rs`, `agent_tick/tests.rs`, `decision_runtime.rs`, `failure_handling.rs`, `goal_model.rs`, `search/tests.rs`). Correction applied safely by seeding the new field from `CognitiveProfile::default()` so those harnesses inherit the live default without duplicating the spec constant.

## Architecture Check

1. Adding a field to an existing per-agent profile component is the standard pattern (same as `switch_margin`, `max_node_expansions`, etc.). No new components, no new registrations.
2. No backward-compatibility shims — the field is added directly. Save format changes are expected when adding fields; the project does not maintain save compatibility across spec implementations.

## Verification Layers

1. `CognitiveProfile` derives `Serialize`/`Deserialize` and roundtrips through bincode -> bincode roundtrip test
2. Default value is 50 -> `cognitive_profile_default_matches_split_defaults` assertion
3. Field participates in `Eq`/`Ord` -> existing `cognitive_profile_component_bounds` test

## What to Change

### 1. Add field to CognitiveProfile struct

In `crates/worldwake-core/src/cognitive_profile.rs`, add `pub max_snapshot_entities_per_place: u16` to the struct definition after `max_cooldown_ticks`.

### 2. Update Default impl

Set `max_snapshot_entities_per_place: 50` in the `Default::default()` impl.

### 3. Update existing tests

- `cognitive_profile_default_matches_split_defaults`: add `assert_eq!(profile.max_snapshot_entities_per_place, 50);`
- `cognitive_profile_roundtrips_through_bincode`: add `max_snapshot_entities_per_place: 75` (or any non-default value) to the test profile construction.

### 4. Update any compile-broken struct literals

Grep for `CognitiveProfile {` across the workspace. Any struct literal that constructs a `CognitiveProfile` without the `..Default::default()` spread will fail to compile. Add the field to those sites.

## Files to Touch

- `crates/worldwake-core/src/cognitive_profile.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)

## Out of Scope

- Belief store changes — unchanged (P14, P15, P16)
- Authoritative world state — unchanged (P4)
- Any behavioral use of the new field — that is ticket 002
- Scenario RON files — the Default covers existing scenarios; new scenarios can set it explicitly

## Acceptance Criteria

### Tests That Must Pass

1. `cognitive_profile_default_matches_split_defaults` — verifies default is 50
2. `cognitive_profile_roundtrips_through_bincode` — verifies serialization
3. `cognitive_profile_component_bounds` — verifies trait bounds
4. Existing suite: `cargo test -p worldwake-core`
5. `cargo build --workspace` — no compile errors from missing field

### Invariants

1. `CognitiveProfile` remains `Copy + Serialize + Deserialize + Eq + Ord + Component`
2. Default value is 50 (matches spec)
3. All existing struct literals compile with the new field

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/cognitive_profile.rs::cognitive_profile_default_matches_split_defaults` — add assertion for new field
2. `crates/worldwake-core/src/cognitive_profile.rs::cognitive_profile_roundtrips_through_bincode` — add non-default value for new field

### Commands

1. `cargo test -p worldwake-core -- cognitive_profile`
2. `cargo build --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-08.

- Added `max_snapshot_entities_per_place: u16` to `worldwake-core` `CognitiveProfile`, with default value `50`.
- Extended the default and bincode roundtrip tests in `cognitive_profile.rs` and updated the `delta.rs` `ComponentValue::CognitiveProfile` fixture to include the new field.
- Updated six `worldwake-ai` test-only `CognitiveProfile` helper constructors to inherit `max_snapshot_entities_per_place` from `CognitiveProfile::default()`, keeping test harnesses aligned with the live default profile contract.

## Verification Result

- Passed `cargo test -p worldwake-core -- cognitive_profile`
- Passed `cargo build --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
