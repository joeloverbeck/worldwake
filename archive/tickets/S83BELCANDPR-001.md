# S83BELCANDPR-001: Add `speculative_acquisition` field to CognitiveProfile

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — CognitiveProfile component field addition
**Deps**: None

## Problem

S83 requires per-agent control over whether acquisition candidates include "speculative" places (places the agent has heard of but doesn't currently believe contain the target resource). This requires a new `speculative_acquisition: bool` field on `CognitiveProfile`. The field must be added with a `false` default, and every explicit `CognitiveProfile { ... }` literal that does not already use `..CognitiveProfile::default()` must be updated.

## Assumption Reassessment (2026-04-10)

1. `CognitiveProfile` struct exists at `crates/worldwake-core/src/cognitive_profile.rs:6` with 12 fields. Derives: `Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize`. `Default` impl at line 21. `Component` impl at line 40. `bool` satisfies all these derives.
2. `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs:86` uses `Option<CognitiveProfile>` directly — no separate `CognitiveProfileDef` type. `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs:368` maps via `unwrap_or_default()`. Adding the field to `CognitiveProfile` with a `false` default automatically propagates through the scenario path.
3. Shared boundary: `CognitiveProfile` is a `worldwake-core` component read by `worldwake-ai` (direct import) and `worldwake-cli` (scenario + persistence). No cross-system coupling — consumers import the type directly from core.
4. Constructor sweep on 2026-04-10 showed the ticket's rough "~24" estimate was stale. Most workspace literals already use `..CognitiveProfile::default()`, so the owned fallout is limited to the shared type, the explicit no-default literals in `worldwake-core` and `worldwake-ai` test helpers, and the focused roundtrip/default tests in `cognitive_profile.rs`.

## Architecture Check

1. Adding a `bool` field with `Default` of `false` is the simplest possible extension. No new types, no new components, no trait changes. All existing behavior is preserved (agents default to non-speculative).
2. No backward-compatibility shims. All construction sites are updated to include the new field explicitly.

## Verification Layers

1. `CognitiveProfile` roundtrips through bincode with the new field -> focused unit test (`cognitive_profile_roundtrips_through_bincode`)
2. `CognitiveProfile` registers correctly for `EntityKind::Agent` -> existing focused test (`cognitive_profile_registers_for_agents`)
3. Single-layer ticket (core type extension); additional layer mapping not applicable.

## What to Change

### 1. Add field to CognitiveProfile struct

In `crates/worldwake-core/src/cognitive_profile.rs`, add after the last field:

```rust
/// Whether this agent generates acquisition candidates at places they've
/// heard of but don't currently believe have the target resource.
pub speculative_acquisition: bool,
```

### 2. Update Default impl

Add `speculative_acquisition: false` to the `Default::default()` impl (line ~23).

### 3. Update explicit no-default construction sites

Add `speculative_acquisition: false` (or `CognitiveProfile::default().speculative_acquisition` in helper constructors) to every `CognitiveProfile { ... }` literal that does not already use `..CognitiveProfile::default()`. Default-spread literals require no change.

- `crates/worldwake-core/src/delta.rs` (1 site)
- `crates/worldwake-core/src/cognitive_profile.rs` (roundtrip test literal)
- `crates/worldwake-ai/src/failure_handling.rs` (test helper)
- `crates/worldwake-ai/src/decision_runtime.rs` (test helper)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (test helper)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (test helper)
- `crates/worldwake-ai/src/goal_model.rs` (test helper)
- `crates/worldwake-ai/src/search/tests.rs` (test helper)

### 4. Verify bincode roundtrip test

The existing `cognitive_profile_roundtrips_through_bincode` test constructs a `CognitiveProfile` with explicit values. Add `speculative_acquisition: true` to the test profile to ensure the new field survives roundtrip.

## Files to Touch

- `crates/worldwake-core/src/cognitive_profile.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)

## Out of Scope

- GoalBeliefView accessor for CognitiveProfile (ticket 002)
- Belief-gated filtering logic in candidate generation (ticket 003)
- Dynamic expansion budget scaling
- Modifying `reachable_places_within_horizon()`

## Acceptance Criteria

### Tests That Must Pass

1. `cognitive_profile_roundtrips_through_bincode` — roundtrip includes `speculative_acquisition: true`
2. `cognitive_profile_registers_for_agents` — unchanged, validates component registration
3. `cognitive_profile_component_bounds` — unchanged, validates derive bounds
4. Existing suite: `cargo test -p worldwake-core`
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `CognitiveProfile::default().speculative_acquisition == false`
2. All existing tests pass without behavioral changes (new field defaults to `false`, preserving existing behavior)
3. Workspace builds cleanly: `cargo build --workspace`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/cognitive_profile.rs::cognitive_profile_roundtrips_through_bincode` — add `speculative_acquisition: true` to test profile to verify serialization roundtrip
2. `crates/worldwake-core/src/cognitive_profile.rs::cognitive_profile_default_matches_split_defaults` — verify the new field is included in the default match check

### Commands

1. `cargo test -p worldwake-core cognitive_profile`
2. `cargo build --workspace`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-10.

- Added `speculative_acquisition: bool` to `CognitiveProfile` with a default value of `false`.
- Updated the explicit no-default `CognitiveProfile` literals in `worldwake-core` and `worldwake-ai` helper/test code so the shared shape compiles cleanly.
- Extended the focused core tests so the default assertion covers the new field and the bincode roundtrip proves a `true` value survives serialization.
- Reassessed the draft constructor estimate during implementation: most workspace literals already use `..CognitiveProfile::default()`, so no edits were needed in the default-spread AI golden/conformance sites listed in the original draft.

## Verification Result

- Passed `cargo test -p worldwake-core cognitive_profile`
- Passed `cargo build --workspace`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
