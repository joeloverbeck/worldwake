# S114PLASTGUA-002: CognitiveProfile expectation-tolerance + guard-confidence-ceiling fields

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — two new `CognitiveProfile` fields (universal agent component); `SAVE_FORMAT_VERSION` bump.
**Deps**: S114 spec (`specs/S114-plan-step-guards.md`) Profile-Driven Parameters section.

## Problem

S114's plan-adoption path (ticket 008) needs `expectation_tolerance_ticks` to set `ExpectationRecord::grace_ticks`, and the revalidation guard-check pass (ticket 007) needs `guard_min_confidence_ceiling` to cap effective guard confidence per agent. Landing the profile fields first unblocks both downstream tickets in parallel without coupling them to each other.

## Assumption Reassessment (2026-04-21)

1. `CognitiveProfile` lives at `crates/worldwake-core/src/cognitive_profile.rs:23`, is universal on every `EntityKind::Agent`, and is seeded with `::default()` at `World::create_agent()` (`crates/worldwake-core/src/world.rs:183`). `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs:359` also applies scenario overrides. No other agent bootstrap path exists — the spec's `#[serde(default = "...")]` requirement is sufficient so existing RON scenarios keep deserializing.
2. S114 Profile-Driven Parameters at `specs/S114-plan-step-guards.md:431-442` defines the exact defaults: `expectation_tolerance_ticks: u32 = 2`, `guard_min_confidence_ceiling: Permille = Permille::new(1000)`.
3. Shared boundary under audit: the `CognitiveProfile` component's save/load format. `SAVE_FORMAT_VERSION = 36` at `crates/worldwake-sim/src/save_load.rs:6` — adding fields to a serialized component changes the bincode layout, so a bump to 37 is required per FND-28 (no backward-compat decode path).
4. Existing tests exercising `CognitiveProfile` serialization and defaults:
   - `cognitive_profile_default_matches_split_defaults` at `cognitive_profile.rs:217` — asserts every field's default value; both new fields must be added to the assertion list.
   - `cognitive_profile_roundtrips_through_bincode` at `cognitive_profile.rs:254` — constructs a full `CognitiveProfile` literal; both new fields must be added.
   - `cognitive_profile_deserialization_defaults_*` at `cognitive_profile.rs:297-499` — confirm the `#[serde(default)]` pattern works for each existing opt-in field; a new test following the same pattern must cover `expectation_tolerance_ticks` and `guard_min_confidence_ceiling`.

## Architecture Check

1. `#[serde(default = "...")]` with explicit `const fn` defaults matches the pattern already used for nine other `CognitiveProfile` fields (`stale_belief_backoff_ticks` et al.). No new layering, no backward-compat shim.
2. Both fields use types the profile already carries elsewhere (`u32` for tick counts, `Permille` for [0,1000] ratios). No new imports.

## Verification Layers

1. Field-default contract (defaults match spec) → focused unit test `cognitive_profile_default_matches_split_defaults`.
2. Round-trip serialization (bincode byte-for-byte) → focused unit test `cognitive_profile_roundtrips_through_bincode`.
3. Scenario-deserialization opt-in (omitted fields fall back to defaults) → two new `cognitive_profile_deserialization_defaults_*` tests mirroring existing pattern.
4. Save-format-bump contract (SAVE_FORMAT_VERSION=37) → existing outdated-save-load failure test at `save_load.rs:1122` continues to pass after bump.
5. Single-layer (core-crate data model only); no downstream behavior change in this ticket — consumers arrive in tickets 007 and 008.

## What to Change

### 1. Add fields to `CognitiveProfile`

In `crates/worldwake-core/src/cognitive_profile.rs:23`, append to the struct:

```rust
#[serde(default = "default_expectation_tolerance_ticks")]
pub expectation_tolerance_ticks: u32,

#[serde(default = "default_guard_min_confidence_ceiling")]
pub guard_min_confidence_ceiling: Permille,
```

Add at module scope:

```rust
const fn default_expectation_tolerance_ticks() -> u32 { 2 }
fn default_guard_min_confidence_ceiling() -> Permille { Permille::new_unchecked(1000) }
```

### 2. Extend the `Default` impl

Append to `impl Default for CognitiveProfile { fn default() -> Self { Self { ... } } }` at line 101:

```rust
expectation_tolerance_ticks: default_expectation_tolerance_ticks(),
guard_min_confidence_ceiling: default_guard_min_confidence_ceiling(),
```

### 3. Extend existing tests

- `cognitive_profile_default_matches_split_defaults` (line 217): add
  `assert_eq!(profile.expectation_tolerance_ticks, 2);` and
  `assert_eq!(profile.guard_min_confidence_ceiling, Permille::new(1000).unwrap());`.
- `cognitive_profile_roundtrips_through_bincode` (line 254): populate the literal with distinct non-default values for both fields.

### 4. Add new opt-in defaults tests

Mirror the existing `cognitive_profile_deserialization_defaults_*` pattern for both new fields — strip the field from serialized RON, confirm it falls back to its default via `from_str`.

### 5. Bump `SAVE_FORMAT_VERSION`

In `crates/worldwake-sim/src/save_load.rs:6`, change `36` → `37`. The existing `load_format_errors_on_outdated_save` test at `save_load.rs:1120` picks up the new value automatically via `SAVE_FORMAT_VERSION - 1`.

## Files to Touch

- `crates/worldwake-core/src/cognitive_profile.rs` (modify)
- `crates/worldwake-sim/src/save_load.rs` (modify — version bump)

## Out of Scope

- Consuming the new fields. `expectation_tolerance_ticks` is read by ticket 008 (plan-adoption ExpectationRecord writes). `guard_min_confidence_ceiling` is read by ticket 007 (`classify_revalidation` guard-check pass).
- Any change to `spawn_agent()` override plumbing; the existing scenario override path already covers any `CognitiveProfile` field.

## Acceptance Criteria

### Tests That Must Pass

1. `cognitive_profile_default_matches_split_defaults` — updated assertions for both new fields.
2. `cognitive_profile_roundtrips_through_bincode` — literal includes both new fields with non-default values.
3. Two new `cognitive_profile_deserialization_defaults_*` tests for opt-in fallback of each new field.
4. `load_format_errors_on_outdated_save` at `save_load.rs:1120` passes against the new `SAVE_FORMAT_VERSION=37`.
5. Existing suite: `cargo test -p worldwake-core cognitive_profile` and `cargo test -p worldwake-sim save_load` stay green.

### Invariants

1. Every existing `CognitiveProfile` field retains its current `#[serde(default = "...")]` annotation — this ticket only appends.
2. `guard_min_confidence_ceiling` is constructed via `Permille::new_unchecked(1000)`; the bound [0, 1000] is enforced by the type, so no additional validation is needed.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/cognitive_profile.rs::cognitive_profile_default_matches_split_defaults` — extend assertions.
2. `crates/worldwake-core/src/cognitive_profile.rs::cognitive_profile_roundtrips_through_bincode` — extend literal.
3. `crates/worldwake-core/src/cognitive_profile.rs::cognitive_profile_deserialization_defaults_expectation_tolerance_ticks` (new).
4. `crates/worldwake-core/src/cognitive_profile.rs::cognitive_profile_deserialization_defaults_guard_min_confidence_ceiling` (new).

### Commands

1. `cargo test -p worldwake-core cognitive_profile`
2. `cargo test -p worldwake-sim save_load`
3. `scripts/verify.sh`
