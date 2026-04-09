# S81GLDGAP-002: Migrate all DeadAt construction sites

**Status**: NOT IMPLEMENTED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes -- mechanical update of DeadAt construction across all crates
**Deps**: S81GLDGAP-001

## Problem

After S81GLDGAP-001 changes `DeadAt` from a tuple struct to a named-field struct, all 75 construction sites across 28 files will fail to compile. This ticket performs the mechanical migration.

## Assumption Reassessment (2026-04-09)

1. 75 `DeadAt(` occurrences across 28 files confirmed via `grep -c 'DeadAt(' crates/`. Distribution: worldwake-core (6 occurrences in 4 files), worldwake-sim (7 in 2 files), worldwake-systems (31 in 12 files), worldwake-ai (31 in 10 files).
2. Production construction site: 1 site at `crates/worldwake-systems/src/combat.rs:183` in `record_fatality`. All other sites are in `#[cfg(test)]` modules or test files.
3. Macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`) use `DeadAt` as a type name in macro-generated code, not as constructor calls. These do not need manual updates -- they compile once the struct shape changes.
4. `crates/worldwake-core/src/belief.rs:5648` has one test construction site.
5. `crates/worldwake-core/src/delta.rs:351` has one test construction site in a `ComponentValue::DeadAt(DeadAt(...))` pattern.
6. Reassessment drift after implementation: `S81GLDGAP-001` was corrected during implementation to absorb the full cross-crate `DeadAt` constructor, equality, pattern-match, and field-access fallout because the shared-type change otherwise left the workspace uncompilable. The owned work this ticket described is no longer pending on the live branch.

## Architecture Check

1. Mechanical migration with no design decisions. Every `DeadAt(tick)` becomes `DeadAt { tick, cause: DeathCause::CombatWounds }` in combat/test contexts. This is the simplest correct transformation.
2. No backward-compatibility shims. The old tuple syntax is removed entirely.

## Verification Layers

1. All construction sites compile -> `cargo build --workspace` (compilation proof)
2. Existing test behavior unchanged -> all existing tests pass (regression proof)
3. Single-layer ticket: no cross-system invariants introduced. The migration is shape-only.

## What to Change

### 1. Production site: combat fatality

`crates/worldwake-systems/src/combat.rs:183`: Change `DeadAt(tick)` to `DeadAt { tick, cause: DeathCause::CombatWounds }`. Add `use worldwake_core::DeathCause;` to imports if not already present.

### 2. worldwake-core test sites

Update construction sites in:
- `crates/worldwake-core/src/delta.rs` (1 site -- `ComponentValue::DeadAt(DeadAt(...))`)
- `crates/worldwake-core/src/belief.rs` (1 site -- test helper)

All use `DeathCause::CombatWounds` as the default test cause.

### 3. worldwake-sim test sites

Update construction sites in:
- `crates/worldwake-sim/src/tick_step.rs` (4 sites -- test system fn + test assertions)
- `crates/worldwake-sim/src/action_validation.rs` (3 sites -- test helpers)

### 4. worldwake-systems production + test sites

Update construction sites in:
- `crates/worldwake-systems/src/combat.rs` (1 production + 19 test sites)
- `crates/worldwake-systems/src/needs.rs` (1 test site)
- `crates/worldwake-systems/src/offices.rs` (2 test sites)
- `crates/worldwake-systems/src/facility_queue.rs` (2 test sites)
- `crates/worldwake-systems/src/bandit_camp.rs` (1 test site)
- `crates/worldwake-systems/src/perception.rs` (2 test sites)
- `crates/worldwake-systems/src/search_actions.rs` (1 test site)
- `crates/worldwake-systems/src/epistemic_actions.rs` (1 test site)
- `crates/worldwake-systems/src/artifact_actions.rs` (1 test site)
- `crates/worldwake-systems/tests/e12_combat_integration.rs` (1 test site)

Add `use worldwake_core::DeathCause;` to each file's test module imports as needed.

### 5. worldwake-ai test sites

Update construction sites in:
- `crates/worldwake-ai/src/agent_tick/tests.rs` (3 sites)
- `crates/worldwake-ai/src/search/tests.rs` (1 site)
- `crates/worldwake-ai/tests/golden_long_scenarios.rs` (4 sites)
- `crates/worldwake-ai/tests/golden_combat.rs` (9 sites)
- `crates/worldwake-ai/tests/golden_integration.rs` (1 site)
- `crates/worldwake-ai/tests/golden_resilience.rs` (1 site)
- `crates/worldwake-ai/tests/golden_determinism.rs` (1 site)
- `crates/worldwake-ai/tests/golden_emergent.rs` (2 sites)
- `crates/worldwake-ai/tests/planner_conformance.rs` (2 sites)
- `crates/worldwake-ai/tests/golden_harness/timeline.rs` (2 sites)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (3 sites)

### 6. Update DeadAt pattern matches if any destructure the tuple

Grep for `DeadAt(` in pattern-match positions (e.g., `Some(&DeadAt(...))`). These need updating to `DeadAt { tick, .. }` or `DeadAt { tick, cause }` syntax. Known sites: `crates/worldwake-systems/src/combat.rs:3199`, `combat.rs:4527`, etc.

## Files to Touch

- `crates/worldwake-core/src/delta.rs` (modify)
- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-sim/src/tick_step.rs` (modify)
- `crates/worldwake-sim/src/action_validation.rs` (modify)
- `crates/worldwake-systems/src/combat.rs` (modify)
- `crates/worldwake-systems/src/needs.rs` (modify)
- `crates/worldwake-systems/src/offices.rs` (modify)
- `crates/worldwake-systems/src/facility_queue.rs` (modify)
- `crates/worldwake-systems/src/bandit_camp.rs` (modify)
- `crates/worldwake-systems/src/perception.rs` (modify)
- `crates/worldwake-systems/src/search_actions.rs` (modify)
- `crates/worldwake-systems/src/epistemic_actions.rs` (modify)
- `crates/worldwake-systems/src/artifact_actions.rs` (modify)
- `crates/worldwake-systems/tests/e12_combat_integration.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/tests/golden_long_scenarios.rs` (modify)
- `crates/worldwake-ai/tests/golden_combat.rs` (modify)
- `crates/worldwake-ai/tests/golden_integration.rs` (modify)
- `crates/worldwake-ai/tests/golden_resilience.rs` (modify)
- `crates/worldwake-ai/tests/golden_determinism.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `crates/worldwake-ai/tests/planner_conformance.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/timeline.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)

## Out of Scope

- Need-based mortality logic (S81GLDGAP-003)
- EventTag::Death tagging on events (S81GLDGAP-003)
- New golden tests (S81GLDGAP-004 through S81GLDGAP-006)
- Changing test semantics -- all existing tests keep their current behavior

## Acceptance Criteria

### Tests That Must Pass

1. All existing tests pass with no behavioral change
2. `cargo build --workspace` compiles cleanly
3. Existing suite: `cargo test --workspace`

### Invariants

1. Every `DeadAt` construction specifies a `DeathCause` (enforced by struct shape)
2. No `DeadAt(` tuple constructor syntax remains in the codebase
3. All existing test assertions on `DeadAt` continue to pass

## Test Plan

### New/Modified Tests

1. None -- documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Not implemented separately on 2026-04-09.

- The work originally planned here was fully absorbed by `S81GLDGAP-001` when that ticket was corrected to own the real shared-type fallout of replacing the `DeadAt` tuple struct.
- No remaining direct `DeadAt(...)` tuple-constructor or tuple-pattern migration work remains on the branch.

## Verification Result

- Covered by `S81GLDGAP-001` verification:
  - `cargo test --workspace --no-run`
  - `cargo clippy --workspace --all-targets -- -D warnings`
