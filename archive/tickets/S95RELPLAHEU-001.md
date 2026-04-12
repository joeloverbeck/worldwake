# S95RELPLAHEU-001: Add `use_ff_heuristic` to CognitiveProfile

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — CognitiveProfile field addition in worldwake-core
**Deps**: S95 spec

## Problem

The FF relaxed-plan heuristic (S95) requires a per-agent toggle so agents can independently enable or disable the RPG-based search guidance. Without this field, there is no way to configure cognitive diversity in heuristic usage (P22).

## Assumption Reassessment (2026-04-12)

1. `CognitiveProfile` exists at `crates/worldwake-core/src/cognitive_profile.rs:6-28` with 15 fields. `use_ff_heuristic` does not yet exist. The struct derives `Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize`. The new `bool` field satisfies all these derives.
2. `Default` impl exists at `cognitive_profile.rs:30-50` and must be extended with `use_ff_heuristic: true`.
3. `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs:86` uses `Option<CognitiveProfile>` directly — no `CognitiveProfileDef` wrapper exists. `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs:368` uses `unwrap_or_default()`, so existing scenarios without the field automatically get the default.
4. Explicit struct literal construction sites exist in test files (e.g., `crates/worldwake-ai/src/search/tests.rs:49-66`) that enumerate all fields without `..Default::default()` — these require the new field.

## Architecture Check

1. Adding a `bool` field with `#[serde(default)]` to an existing profile struct is the established pattern (matches `speculative_acquisition`, `landmark_extraction_depth`). No new types, no new components, and no new production crate dependencies are required.
2. No backward-compatibility shims. The serde default handles deserialization of existing scenarios.

## Verification Layers

1. Field exists and defaults to `true` → focused unit test on `CognitiveProfile::default()`
2. Serde deserialization without field produces default → focused unit test
3. Single-layer ticket (profile field addition) — no cross-system mapping needed.

## What to Change

### 1. Add field to CognitiveProfile struct

In `crates/worldwake-core/src/cognitive_profile.rs`:

- Add field after `landmark_extraction_depth`:
  ```rust
  /// Whether this agent uses the FF-style relaxed-plan heuristic for
  /// tactical search guidance. When `false`, search uses spatial heuristic
  /// only with landmark-based preferred operators (pre-S95 behavior).
  #[serde(default = "default_use_ff_heuristic")]
  pub use_ff_heuristic: bool,
  ```

### 2. Update Default impl

Add `use_ff_heuristic: true` to the `Default` impl.

### 3. Add default function

```rust
const fn default_use_ff_heuristic() -> bool {
    true
}
```

### 4. Update explicit construction sites

Grep for `CognitiveProfile {` across `crates/` to find all struct literal constructions. Add `use_ff_heuristic: CognitiveProfile::default().use_ff_heuristic` (or `true`) to each. Key known sites:
- `crates/worldwake-ai/src/search/tests.rs:49-66`
- Other test files constructing CognitiveProfile explicitly

## Files to Touch

- `crates/worldwake-core/Cargo.toml` (dev-dependency for focused serde omission proof)
- `crates/worldwake-core/src/cognitive_profile.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (update explicit manifest sample)
- `crates/worldwake-ai/src/failure_handling.rs` (update explicit helper literal)
- `crates/worldwake-ai/src/decision_runtime.rs` (update explicit helper literal)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (update explicit helper literal)
- `crates/worldwake-ai/src/goal_model.rs` (update explicit helper literal)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (update explicit helper literal)
- `crates/worldwake-ai/src/search/tests.rs` (modify)

## Out of Scope

- Search integration logic (ticket 004)
- Decision trace fields (ticket 002)
- RPG algorithm (ticket 003)
- Scenario RON file updates (serde default handles existing files)

## Acceptance Criteria

### Tests That Must Pass

1. `CognitiveProfile::default().use_ff_heuristic == true`
2. Deserialization of a CognitiveProfile RON/JSON without `use_ff_heuristic` yields `true`
3. Existing suite: `cargo test --workspace`

### Invariants

1. `CognitiveProfile` remains `Copy + Clone + Serialize + Deserialize`
2. All existing scenarios produce identical behavior (field defaults to `true`, no consumer reads it yet)

## Test Plan

### New/Modified Tests

1. `cognitive_profile_default_matches_split_defaults`
2. `cognitive_profile_deserialization_defaults_use_ff_heuristic`

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-12.

- Added `use_ff_heuristic` to `CognitiveProfile` with a serde default helper and default value of `true`.
- Extended the core test coverage to prove both `CognitiveProfile::default()` and omitted-field RON deserialization preserve `use_ff_heuristic: true`.
- Updated the remaining explicit full `CognitiveProfile` literals in `worldwake-core` and AI helper/test modules so the shared type change compiles cleanly across the workspace.
- Added `ron = "0.8"` as a `worldwake-core` dev-dependency only, so the focused omitted-field serde proof stays in the owning crate instead of moving to a broader integration crate.

## Deviations

- The original ticket said no new tests were needed. Reassessment showed `worldwake-core` already owned focused `CognitiveProfile` test coverage, and the omitted-field serde contract needed an explicit proof there.

## Verification Result

- Passed `cargo test -p worldwake-core`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
