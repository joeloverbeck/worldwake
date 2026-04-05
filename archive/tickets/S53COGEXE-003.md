# S53COGEXE-003: Remove ReasoningProfile and save format migration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — remove component type, component schema update, coexistence-save cleanup migration, version bump
**Deps**: S53COGEXE-002

## Problem

After ticket 002, `ReasoningProfile` no longer drives the live production AI decision pipeline, but it still exists as an authoritative component type, remains registered in the component schema, and still appears in temporary test/public/setup compatibility surfaces. Per Principle 28 (No Backward Compatibility), that legacy carrier must now be removed completely. The coexistence save format from ticket 001 still persists all three components, so this ticket cleans that format up to the final split-only shape.

## Assumption Reassessment (2026-04-05)

1. After ticket 002, zero live production AI readers should remain on `ReasoningProfile`, but test-only compatibility helpers, public re-exports, and CLI scenario/persistence setup may still mention it until this cleanup ticket lands.
2. `ReasoningProfile` registered in `component_schema.rs` on `EntityKind::Agent` — must be removed.
3. `reasoning_profile.rs` module in worldwake-core — must be removed.
4. `AgentDef.reasoning_profile: Option<ReasoningProfile>` in CLI scenario types — must be removed.
5. `spawn_agent()` applies ReasoningProfile — must be removed.
6. After ticket 001, `SAVE_FORMAT_VERSION` is 24 and the live save shape contains `ReasoningProfile`, `CognitiveProfile`, and `ExecutionBudget` together. This ticket bumps to 25 and removes the old component from persisted state.
7. CLI evaluation scenario `scenarios/cli-evaluation.ron` has `reasoning_profile` on Merchant Vara — must be migrated to `cognitive_profile` + `execution_budget`.
8. Macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`) import ReasoningProfile — imports must be removed.
9. `CognitiveProfile::default()` / `ExecutionBudget::default()` and their `from_reasoning_profile(...)` constructors still depend directly on `ReasoningProfile` in `cognitive_profile.rs` and `execution_budget.rs`. Removing the legacy carrier requires collapsing those defaults to direct split-profile defaults and replacing any remaining conversion need with ticket-local compatibility code on the save-migration boundary instead of keeping the old type alive in core.
10. Save/load does not currently have a versioned migration path for pre-current full-world schemas; `load_current_format()` directly bincode-deserializes the current `SimulationState`. A real `24 -> 25` migration therefore needs an explicit legacy decode shape rather than only bumping `SAVE_FORMAT_VERSION`.

## Architecture Check

1. Clean P28 removal — no aliases, no deprecated wrappers. The old type is deleted, not hidden.
2. Save migration is a one-way transform at load time: read coexistence-format saves containing `ReasoningProfile` plus the split profiles, drop the old component, and keep the split profiles as authoritative. The old coexistence format is consumed, not preserved.
3. After this ticket, the codebase has no reference to `ReasoningProfile` anywhere, including temporary test wrappers, public re-exports, and CLI setup/persistence surfaces that remained transitional after ticket 002.
4. Split-profile defaults remain first-class authoritative state after removal. They must no longer derive from a deleted legacy type at runtime; any field-splitting compatibility logic belongs only on the migration or test-compat boundary that still reads version-24 saves.

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
- Add migration function for version 24 → 25 using explicit legacy decode structs for the old save shape. For each agent entity with `ReasoningProfile`, drop the old component and preserve the already-authoritative `CognitiveProfile` + `ExecutionBudget`.

### 6. Clean up any remaining references

Grep for `ReasoningProfile` across the entire workspace. Any remaining references (test helpers, documentation, comments) must be updated or removed.

### 7. Collapse split-profile constructors off the legacy type

In `crates/worldwake-core/src/cognitive_profile.rs` and `crates/worldwake-core/src/execution_budget.rs`:
- replace `Default` implementations that currently derive through `ReasoningProfile`
- remove `from_reasoning_profile(...)`
- rewrite tests to prove direct defaults and registration without the deleted type

## Files to Touch

- `crates/worldwake-core/src/reasoning_profile.rs` (delete)
- `crates/worldwake-core/src/cognitive_profile.rs` (modify — direct defaults, remove legacy conversion helper)
- `crates/worldwake-core/src/execution_budget.rs` (modify — direct defaults, remove legacy conversion helper)
- `crates/worldwake-core/src/lib.rs` (modify — remove module)
- `crates/worldwake-core/src/component_schema.rs` (modify — remove registration)
- `crates/worldwake-core/src/world.rs` (modify — remove macro import)
- `crates/worldwake-core/src/delta.rs` (modify — remove macro import)
- `crates/worldwake-core/src/component_tables.rs` (modify — remove macro import)
- `crates/worldwake-core/src/world_txn.rs` (modify — remove delta/sample references)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — remove field)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — remove spawn logic)
- `crates/worldwake-cli/src/handlers/persistence.rs` (modify — remove legacy persistence tests/setup)
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
5. `CognitiveProfile` and `ExecutionBudget` defaults remain direct, authoritative defaults after the legacy carrier is deleted

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/save_load.rs` — Migration test: save with version 24 (coexistence format) → load → verify `CognitiveProfile` + `ExecutionBudget` remain and `ReasoningProfile` is gone
2. `crates/worldwake-sim/src/save_load.rs` — Round-trip test: save version 25 → load → verify identical
3. `crates/worldwake-core/src/cognitive_profile.rs` and `crates/worldwake-core/src/execution_budget.rs` — updated direct-default tests with no legacy carrier

### Commands

1. `cargo test -p worldwake-sim -- save`
2. `cargo test -p worldwake-ai` (golden tests verify behavioral equivalence)
3. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- **Completed**: 2026-04-05
- **What changed**:
  - Removed the legacy `ReasoningProfile` carrier from the live codebase by deleting [`reasoning_profile.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/reasoning_profile.rs), removing its crate-root exports in [`lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/lib.rs), and deleting its authoritative schema/registry presence in [`component_schema.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/component_schema.rs), [`world.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world.rs), [`delta.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/delta.rs), and [`world_txn.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs).
  - Collapsed split-profile defaults to direct authoritative values in [`cognitive_profile.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/cognitive_profile.rs) and [`execution_budget.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/execution_budget.rs), removing any runtime dependency on the deleted combined profile.
  - Removed legacy scenario and CLI setup surfaces in [`types.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/scenario/types.rs), [`mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/scenario/mod.rs), [`persistence.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/persistence.rs), and [`cli-evaluation.ron`](/home/joeloverbeck/projects/worldwake/scenarios/cli-evaluation.ron).
  - Added explicit save migration in [`save_load.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs): `SAVE_FORMAT_VERSION` now advances from 24 to 25, coexistence-format version-24 saves are decoded through explicit legacy structs, and the migrated world keeps only `CognitiveProfile` plus `ExecutionBudget`.
  - Migrated remaining AI test/public fallout to the post-removal world, including the test-only `ProfileFixture` path in [`worldwake-ai/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/lib.rs) and the affected AI test modules.
- **Deviations from original plan**:
  - The original ticket already owned the `24 -> 25` save migration after reassessment, but focused verification exposed one stale assumption in the existing save suite: the old “previous version must fail” test had to be rewritten because version 24 became an intentionally supported migration source.
  - The migration helper in [`save_load.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs) needed a small test-only legacy encoder so the focused proof could exercise a real coexistence-format payload rather than a hand-waved mock.
- **Verification**:
  - `rg -n "ReasoningProfile|reasoning_profile|from_reasoning_profile" crates scenarios`
  - `cargo test -p worldwake-sim -- save`
  - `cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec quit`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
