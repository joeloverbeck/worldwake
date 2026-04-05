# S53COGEXE-003: Remove ReasoningProfile and save format migration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — remove component type, component schema update, coexistence-save cleanup migration, version bump
**Deps**: S53COGEXE-002

## Problem

ReasoningProfile has zero consumers after ticket 002 but still exists as a component type and is registered in the component schema. Per Principle 28 (No Backward Compatibility), the dead type must be removed. The coexistence save format from ticket 001 still persists all three components, so this ticket cleans that format up to the final split-only shape.

## Assumption Reassessment (2026-04-05)

1. After ticket 002, zero files read ReasoningProfile — confirmed by design (002 migrates all 13 consumers).
2. `ReasoningProfile` registered in `component_schema.rs` on `EntityKind::Agent` — must be removed.
3. `reasoning_profile.rs` module in worldwake-core — must be removed.
4. `AgentDef.reasoning_profile: Option<ReasoningProfile>` in CLI scenario types — must be removed.
5. `spawn_agent()` applies ReasoningProfile — must be removed.
6. After ticket 001, `SAVE_FORMAT_VERSION` is 24 and the live save shape contains `ReasoningProfile`, `CognitiveProfile`, and `ExecutionBudget` together. This ticket bumps to 25 and removes the old component from persisted state.
7. CLI evaluation scenario `scenarios/cli-evaluation.ron` has `reasoning_profile` on Merchant Vara — must be migrated to `cognitive_profile` + `execution_budget`.
8. Macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`) import ReasoningProfile — imports must be removed.

## Architecture Check

1. Clean P28 removal — no aliases, no deprecated wrappers. The old type is deleted, not hidden.
2. Save migration is a one-way transform at load time: read coexistence-format saves containing `ReasoningProfile` plus the split profiles, drop the old component, and keep the split profiles as authoritative. The old coexistence format is consumed, not preserved.
3. After this ticket, the codebase has no reference to ReasoningProfile anywhere.

## Verification Layers

1. Zero references to ReasoningProfile in codebase → grep confirms
2. Save migration: old-format save loads correctly with split profiles → focused migration test
3. New-format save round-trips correctly → save/load test
4. All golden tests pass → behavioral equivalence preserved across migration
5. CLI scenario loads with split profiles → `cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec quit`

## What to Change

### 1. Remove ReasoningProfile type

Delete `crates/worldwake-core/src/reasoning_profile.rs`.
Remove `pub mod reasoning_profile;` and re-exports from `crates/worldwake-core/src/lib.rs`.

### 2. Remove from component_schema.rs

Remove `ReasoningProfile` registration entry. Remove imports at all macro expansion sites.

### 3. Remove from AgentDef and spawn_agent

In `crates/worldwake-cli/src/scenario/types.rs`: remove `reasoning_profile: Option<ReasoningProfile>` field.
In `crates/worldwake-cli/src/scenario/mod.rs`: remove ReasoningProfile application in `spawn_agent()`.

### 4. Update CLI evaluation scenario

In `scenarios/cli-evaluation.ron`: replace Merchant Vara's `reasoning_profile: (...)` with equivalent `cognitive_profile: (...)` and `execution_budget: (...)` using the same field values split per the classification table.

### 5. Save format migration

In `crates/worldwake-sim/src/save_load.rs`:
- Bump `SAVE_FORMAT_VERSION` from 24 to 25.
- Add migration function for version 24 → 25: for each agent entity with `ReasoningProfile`, remove the old component and preserve the already-authoritative `CognitiveProfile` + `ExecutionBudget`.

### 6. Clean up any remaining references

Grep for `ReasoningProfile` across the entire workspace. Any remaining references (test helpers, documentation, comments) must be updated or removed.

## Files to Touch

- `crates/worldwake-core/src/reasoning_profile.rs` (delete)
- `crates/worldwake-core/src/lib.rs` (modify — remove module)
- `crates/worldwake-core/src/component_schema.rs` (modify — remove registration)
- `crates/worldwake-core/src/world.rs` (modify — remove macro import)
- `crates/worldwake-core/src/delta.rs` (modify — remove macro import)
- `crates/worldwake-core/src/component_tables.rs` (modify — remove macro import)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — remove field)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — remove spawn logic)
- `crates/worldwake-sim/src/save_load.rs` (modify — migration + version bump)
- `scenarios/cli-evaluation.ron` (modify — split reasoning_profile)

## Out of Scope

- Behavioral validation conformance test — ticket 004
- Adding new cognitive parameters
- Changing planner algorithm

## Acceptance Criteria

### Tests That Must Pass

1. `grep -r "ReasoningProfile" crates/` returns zero matches
2. Save file with version 24 (coexistence format) loads correctly and produces split-only profiles
3. Save file with version 25 round-trips correctly
4. CLI evaluation scenario loads: `cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec quit`
5. All golden tests pass — no behavioral change from migration
6. Existing suite: `cargo test --workspace`

### Invariants

1. Zero references to ReasoningProfile in the codebase (P28 — no backward compatibility)
2. Save migration preserves all field values — no data loss
3. SAVE_FORMAT_VERSION == 25
4. Old saves are read-migrated, not aliased

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/save_load.rs` — Migration test: save with version 24 (coexistence format) → load → verify `CognitiveProfile` + `ExecutionBudget` remain and `ReasoningProfile` is gone
2. `crates/worldwake-sim/src/save_load.rs` — Round-trip test: save version 25 → load → verify identical

### Commands

1. `cargo test -p worldwake-sim -- save`
2. `cargo test -p worldwake-ai` (golden tests verify behavioral equivalence)
3. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
