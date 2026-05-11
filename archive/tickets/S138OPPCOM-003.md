# S138OPPCOM-003: Profile field additions — CognitiveProfile (×2) and PerceptionProfile (×1)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extends two universal profile structs; bumps `SAVE_FORMAT_VERSION` 76 → 77
**Deps**: archive/tickets/S138OPPCOM-002.md (sequential SAVE_FORMAT bump)

## Problem

S138 introduces three new per-agent tuning parameters on existing universal profiles:

- `CognitiveProfile.detour_budget_permille: Permille` — governs how much salience opportunities along a detour path must offer before the travel pruner allows the detour (consumed by ticket 007)
- `CognitiveProfile.compile_opportunity_cap: u16` — soft cap on `compile_opportunities` result length per tick per agent (consumed by ticket 006)
- `PerceptionProfile.opportunity_floor_permille: Permille` — salience floor below which opportunities are not emitted (consumed by ticket 006)

Each field carries a `#[serde(default)]` annotation so existing scenario RON files continue to deserialize. The ticket also updates every `CognitiveProfile` struct-literal construction site — 16+ sites with no `..Default::default()` spread syntax — to enumerate the two new fields.

## Assumption Reassessment (2026-05-11)

1. Existing focused/unit coverage: `crates/worldwake-core/src/cognitive_profile.rs` has roundtrip and serde-default tests (e.g., `cognitive_profile_roundtrips_through_bincode`) that exercise the struct shape; `crates/worldwake-core/src/belief.rs` has `PerceptionProfile` roundtrip tests at lines 6939+ and serde-default tests using string-replace at lines 6962-6963.
2. Spec/doc reference: `specs/S138-opportunity-compiler.md` deliverable section "`PerceptionProfile` opportunity-floor field" + "Profile-Driven Parameters" section.
3. Shared abstraction boundary: per-agent profile cluster — `CognitiveProfile` and `PerceptionProfile` already host serde-default fields (`observation_budget`, `omission_log_capacity`, `slot_weights`, etc.), so the addition pattern is established.
4. Struct-literal construction sites for `CognitiveProfile`: 16+ enumerate-style literals confirmed across `crates/worldwake-core/src/{cognitive_profile.rs, delta.rs, survey_memory.rs}`, `crates/worldwake-ai/src/{decision_runtime.rs, plan_revalidation.rs, agent_tick/planning.rs, agent_tick/tests.rs, search/tests.rs, goal_model.rs, failure_handling.rs}`, `crates/worldwake-ai/tests/{conformance_execution_budget.rs, golden_ai_decisions.rs, golden_exploration.rs, golden_quantity_aware_acquisition.rs}`, `crates/worldwake-systems/src/{evidence_decay.rs, perception.rs}`. None use spread syntax — every site enumerates all fields. Effort tracks this site count (Medium).
5. `PerceptionProfile` struct-literal construction sites: mostly use `PerceptionProfile::default()` (per `world.rs:184`, scenario layer), but compile fallout confirmed additional exhaustive literals across core, systems, AI golden harnesses, and CLI scenario tests. The new field has `#[serde(default)]` so RON authors are unaffected.
6. Save-format bump: archived ticket `archive/tickets/S138OPPCOM-002.md` bumps 75→76; this ticket bumps 76→77 (cascade).

## Architecture Check

1. Field additions on existing universal profiles match the established precedent — the existing `observation_budget`, `slot_weights`, `decision_history_alternatives` were all added similarly with serde defaults.
2. `#[serde(default = "...")]` keeps existing authored RON scenarios deserializing without scenario-author churn — preserves the FND-28 "no backward-compatibility shim" rule at the live authority path while permitting graceful boundary deserialization.
3. The two new `CognitiveProfile` fields are conceptually paired (both govern compile/index work on the same per-agent budget envelope) — co-landing them keeps the per-agent tuning surface coherent and avoids double-bumping save format.
4. Per-agent `Permille` weights remain workspace-native — no external dependency introduced.

## Verification Layers

1. Field default values applied — focused unit test on each profile's `Default::default()`
2. Serde default round-trip — focused unit test deserializing a RON snippet missing the new fields, asserting defaults applied
3. Existing struct-literal sites continue to compile after field additions — workspace build
4. Save-format version 77 round-trip — `cargo test -p worldwake-sim save_load`

## What to Change

### 1. `CognitiveProfile` extensions

Modify `crates/worldwake-core/src/cognitive_profile.rs`:

Add two fields adjacent to existing per-agent budget fields (near `slot_weights` line 107 or `decision_history_alternatives` line 103):

```rust
#[serde(default = "default_detour_budget_permille")]
pub detour_budget_permille: Permille,
#[serde(default = "default_compile_opportunity_cap")]
pub compile_opportunity_cap: u16,
```

Add default functions near existing similar helpers:

```rust
fn default_detour_budget_permille() -> Permille { Permille::new_unchecked(150) }
fn default_compile_opportunity_cap() -> u16 { 16 }
```

Update the `Default for CognitiveProfile` impl at line 111 to include both new fields with the same defaults.

### 2. `PerceptionProfile` extension

Modify `crates/worldwake-core/src/belief.rs` (struct at line 2644):

Add adjacent to existing serde-default fields like `observation_budget` (line 2664):

```rust
#[serde(default = "default_opportunity_floor_permille")]
pub opportunity_floor_permille: Permille,
```

Add default function near `default_observation_budget`:

```rust
fn default_opportunity_floor_permille() -> Permille { Permille::new_unchecked(100) }
```

`PerceptionProfile` has a manual `Default` impl on the live branch. Add the new field there with `default_opportunity_floor_permille()` and update any `PerceptionProfile { … }` struct literal that previously enumerated all fields.

### 3. Update all `CognitiveProfile` struct-literal sites

For each of the 16+ enumerated sites listed in Assumption Reassessment item 4, add the two new fields with appropriate values (typically the default literals or scenario-tuned values where the test exercises detour/compile behavior — most sites should use the same defaults as `Default::default()`).

### 4. Update `PerceptionProfile` struct-literal sites (if any)

Confirm via grep during implementation; update enumerate-style literals to include `opportunity_floor_permille`.

### 5. SAVE_FORMAT bump

Modify `crates/worldwake-sim/src/save_load.rs:6`: `SAVE_FORMAT_VERSION = 77`. Update the version-assertion test at line 1198.

### 6. Profile-docs regeneration

Run `python3 scripts/profile_docs.py` and commit the regenerated `docs/profiles/all-profiles.md`.

## Files to Touch

- `crates/worldwake-core/src/cognitive_profile.rs` (modify — 2 new fields + 2 default fns + Default impl update)
- `crates/worldwake-core/src/belief.rs` (modify — 1 new field on PerceptionProfile + 1 default fn)
- `crates/worldwake-sim/src/save_load.rs` (modify — SAVE_FORMAT_VERSION 76→77 + test)
- Construction sites: `crates/worldwake-core/src/delta.rs`, `crates/worldwake-core/src/survey_memory.rs`, `crates/worldwake-ai/src/{decision_runtime.rs, plan_revalidation.rs, agent_tick/planning.rs, agent_tick/tests.rs, search/tests.rs, goal_model.rs, failure_handling.rs}`, `crates/worldwake-ai/tests/{conformance_execution_budget.rs, golden_ai_decisions.rs, golden_exploration.rs, golden_quantity_aware_acquisition.rs}`, `crates/worldwake-systems/src/{evidence_decay.rs, perception.rs}` — all enumerate `CognitiveProfile { … }` and need the two new fields appended
- `docs/profiles/all-profiles.md` (regenerate via `scripts/profile_docs.py`)

## Out of Scope

- Reading the new fields anywhere (consumers land in tickets 006 and 007)
- New universal-on-Agent components — landed in `archive/tickets/S138OPPCOM-002.md`
- `SaliencePolicy` enum modification — the spec explicitly hosts the floor on `PerceptionProfile`, not `SaliencePolicy`

## Acceptance Criteria

### Tests That Must Pass

1. New unit test in `cognitive_profile.rs`: `Default::default()` produces `detour_budget_permille = 150` and `compile_opportunity_cap = 16`
2. New unit test in `cognitive_profile.rs`: deserializing a RON snippet missing the two new fields applies their defaults
3. New unit test in `belief.rs`: `PerceptionProfile` deserialized from a snippet missing `opportunity_floor_permille` defaults to `Permille::new_unchecked(100)`
4. Save-format roundtrip test at `save_load.rs:1198` passes with new version 77
5. Existing suite: `cargo test --workspace` — all CognitiveProfile struct-literal sites compile
6. `cargo clippy --workspace --all-targets -- -D warnings` — no warnings

### Invariants

1. Adding `#[serde(default)]` fields does not break authored RON scenarios — `scenarios/survival-baseline.ron` continues to load
2. All `CognitiveProfile` struct-literal construction sites compile after the addition (workspace-builds-after-each-ticket invariant)
3. Older save files (SAVE_FORMAT_VERSION 76 from `archive/tickets/S138OPPCOM-002.md`) fail to load — no silent backward-compat

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/cognitive_profile.rs` (inline `#[cfg(test)]`) — defaults + serde-default deserialization for both new fields
2. `crates/worldwake-core/src/belief.rs` (inline `#[cfg(test)]`) — serde-default deserialization for `opportunity_floor_permille`
3. `crates/worldwake-sim/src/save_load.rs` (extend existing test at 1198) — version 77 round-trip

### Commands

1. `cargo test -p worldwake-core cognitive_profile belief`
2. `cargo test -p worldwake-sim save_load`
3. `cargo build --workspace`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `python3 scripts/profile_docs.py` then `git diff docs/profiles/all-profiles.md`

Merge note: Bumps `SAVE_FORMAT_VERSION` 76→77 — must land after `archive/tickets/S138OPPCOM-002.md` (75→76). See Step 6 Merge-Order Constraints.

## Outcome

Completed on 2026-05-11.

- Added `CognitiveProfile.detour_budget_permille` and `CognitiveProfile.compile_opportunity_cap` with serde defaults, `Default` values, bincode round-trip coverage, omitted-field serde coverage, and explicit constructor fallout updates across core, AI, systems, CLI scenario tests, and golden harness fixtures.
- Added `PerceptionProfile.opportunity_floor_permille` with serde/default support, explicit authored-input coverage, and constructor fallout updates across full `PerceptionProfile` literals.
- Bumped `SAVE_FORMAT_VERSION` from 76 to 77 and updated the save/load version assertion.
- Regenerated `docs/profiles/all-profiles.md` with the new profile fields.

## Deviations

- Live reassessment corrected the draft note that `PerceptionProfile` lacked a manual `Default` impl; the implementation updated the existing manual impl instead.
- `default_opportunity_floor_permille` is public and re-exported beside `default_omission_log_capacity` so downstream exhaustive literals can use the same canonical default helper.
- `python3 scripts/profile_docs.py --write` passed and regenerated the profile docs while reporting 15 pre-existing documentation-gap warnings for unrelated profile fields.

## Verification Result

- Passed `cargo test --workspace --no-run`
- Passed `python3 scripts/profile_docs.py --write`
- Passed `cargo fmt --all`
- Passed `cargo test -p worldwake-core cognitive_profile`
- Passed `cargo test -p worldwake-core perception_profile`
- Passed `cargo test -p worldwake-sim save_load`
- Passed `cargo build --workspace`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
