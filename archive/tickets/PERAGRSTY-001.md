# PERAGRSTY-001: Add `ReasoningProfile` component and registration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new ECS component in worldwake-core
**Deps**: None

## Problem

All agents share identical planning parameters (224 expansions, 8 beam width, 100 permille switch margin, same cooldown curves) via a single `PlanningBudget` in `AgentTickDriver`. This violates Principle 22 (Agent Diversity Through Concrete Variation). The first step is to define the per-agent `ReasoningProfile` component and register it in the component schema.

## Assumption Reassessment (2026-04-03)

1. `PlanningBudget` exists at `crates/worldwake-ai/src/budget.rs:4-18` with 12 fields. `Default` impl at lines 20-37 provides: `max_candidates_to_plan: 2`, `max_plan_depth: 8`, `snapshot_travel_horizon: 6`, `max_prerequisite_locations: 3`, `max_node_expansions: 224`, `beam_width: 8`, `switch_margin_permille: Permille::new_unchecked(100)`, `transient_block_ticks: 20`, `unknown_block_ticks: 5`, `structural_block_ticks: 200`, `initial_cooldown_ticks: 4`, `max_cooldown_ticks: 64`. These are the exact values `ReasoningProfile::default()` must reproduce.
2. The `Component` trait at `crates/worldwake-core/src/traits.rs:11-15` requires `'static + Send + Sync + Clone + Debug + Serialize + DeserializeOwned`. All existing profiles use empty `impl Component for ProfileType {}`.
3. Component registration uses the `with_component_schema_entries!` macro in `crates/worldwake-core/src/component_schema.rs`. Agent components use `|kind| kind == EntityKind::Agent` predicate. Existing examples: `CombatProfile` (lines 82-104), `UtilityProfile` (lines 233-255), `PerceptionProfile` (lines 583-605).
4. All live macro expansion sites that materialize authoritative component APIs must see the new bare type name. On the current branch that includes `delta.rs`, `world.rs`, `component_tables.rs`, and `world_txn.rs`.
5. The field `switch_margin_permille` in `PlanningBudget` is renamed to `switch_margin` in `ReasoningProfile`. This is intentional — the `_permille` suffix is redundant given the `Permille` type.
6. `Permille` at `crates/worldwake-core/src/numerics.rs:24-25` derives `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize`.
7. Not a planner/golden/ranking/stale-request/political/ControlSource/heuristic-removal ticket — domain-specific precision items 5-15 are N/A.

## Architecture Check

1. Follows the established profile-component pattern exactly (`PerceptionProfile`, `TellProfile`, `UtilityProfile`, `CombatProfile`, `IntentionDispositionProfile`, `PursuitProfile`). No new abstractions introduced.
2. No backward-compatibility shims. `ReasoningProfile` is a new addition; `PlanningBudget` removal happens in PERAGRSTY-002.

## Verification Layers

1. `ReasoningProfile::default()` matches `PlanningBudget::default()` field-for-field -> focused unit test comparing every field value
2. Component registered for `EntityKind::Agent` and wired through generated core component APIs -> unit test attaching profile to an agent entity and reading it back
3. Bincode round-trip preservation -> focused unit test serializing and deserializing
4. Single-layer ticket (new component definition only, no cross-system interaction) — additional layer mapping not applicable

## What to Change

### 1. Define `ReasoningProfile` struct

Create `crates/worldwake-core/src/reasoning_profile.rs`:

- Define `ReasoningProfile` struct with 12 fields matching the spec (same types as `PlanningBudget` except `switch_margin_permille` -> `switch_margin`).
- Derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.
- `impl Component for ReasoningProfile {}`.
- `impl Default for ReasoningProfile` with values exactly matching `PlanningBudget::default()`.
- Add `#[cfg(test)] mod tests` with:
  - `reasoning_profile_default_matches_planning_budget` — assert every field equals the expected constant.
  - `reasoning_profile_roundtrips_through_bincode` — serialize + deserialize + assert equality.

### 2. Export from worldwake-core

Add `pub mod reasoning_profile;` and `pub use reasoning_profile::ReasoningProfile;` in `crates/worldwake-core/src/lib.rs`.

### 3. Register in component schema

Add a `ReasoningProfile` entry in the `with_component_schema_entries!` macro in `crates/worldwake-core/src/component_schema.rs`, following the pattern of `PerceptionProfile` (lines 583-605). Use `|kind| kind == EntityKind::Agent` predicate.

Verify that the live macro expansion sites that materially require the bare type name (`delta.rs`, `world.rs`, `component_tables.rs`) import `ReasoningProfile`, and update any explicit component inventories that must enumerate the new variant.

## Files to Touch

- `crates/worldwake-core/src/reasoning_profile.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-export)
- `crates/worldwake-core/src/component_schema.rs` (modify — add macro entry)
- `crates/worldwake-core/src/delta.rs` (modify — add import if needed by macro expansion)
- `crates/worldwake-core/src/world.rs` (modify — add import if needed by macro expansion)
- `crates/worldwake-core/src/component_tables.rs` (modify — add import if needed by macro expansion)

## Out of Scope

- Consuming `ReasoningProfile` in `worldwake-ai` (PERAGRSTY-002)
- Removing `PlanningBudget` (PERAGRSTY-002)
- Save/load version bump (PERAGRSTY-002)
- Golden test for behavioral diversity (PERAGRSTY-003)

## Acceptance Criteria

### Tests That Must Pass

1. `reasoning_profile_default_matches_planning_budget` — every field matches `PlanningBudget::default()` values
2. `reasoning_profile_roundtrips_through_bincode` — serialize/deserialize preserves all fields
3. `cargo test -p worldwake-core reasoning_profile` exercises the new focused unit tests and registration plumbing
4. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `ReasoningProfile` is registered only for `EntityKind::Agent`, not other entity kinds
2. `ReasoningProfile::default()` is value-identical to `PlanningBudget::default()` across all 12 fields

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/reasoning_profile.rs` — unit tests for default-value fidelity and bincode round-trip

### Commands

1. `cargo test -p worldwake-core reasoning_profile`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

- **Completion date**: 2026-04-03
- **What changed**:
  - Added [`crates/worldwake-core/src/reasoning_profile.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/reasoning_profile.rs) with the 12-field `ReasoningProfile` component, `Default`, and focused unit tests.
  - Exported `ReasoningProfile` from [`crates/worldwake-core/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/lib.rs).
  - Registered the component for `EntityKind::Agent` in [`crates/worldwake-core/src/component_schema.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/component_schema.rs).
  - Updated generated component surfaces and explicit schema inventories in [`crates/worldwake-core/src/component_tables.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/component_tables.rs), [`crates/worldwake-core/src/world.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world.rs), and [`crates/worldwake-core/src/delta.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/delta.rs).
- **Deviations from original plan**:
  - Reassessment initially flagged `world_txn.rs` as a possible macro-expansion fallout site, but the final implementation did not require a code change there because the selected transaction-surface macro path did not need the new bare type import.
  - The completed proof surface includes explicit `ComponentKind` / `ComponentValue` inventory updates in `delta.rs`, which were required by the live schema manifest tests.
- **Verification results**:
  - Passed `cargo test -p worldwake-core reasoning_profile`
  - Passed `cargo test -p worldwake-core`
  - Passed `cargo clippy --workspace --all-targets -- -D warnings`
  - Passed `cargo test --workspace`
