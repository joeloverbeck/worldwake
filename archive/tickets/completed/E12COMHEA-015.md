# E12COMHEA-015: Integration tests — multi-tick combat scenarios

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None — tests only
**Deps**: All E12COMHEA tickets (001-014)

## Problem

E12 needs end-to-end integration tests that verify the full combat lifecycle across multiple ticks: attack → wound → bleed → clot → recover/die → loot. These tests verify cross-system interactions, invariants, and the complete spec acceptance criteria.

## Assumption Reassessment (2026-03-11)

1. All prior E12 tickets do provide the core components, action definitions, handlers, and combat system tick. That assumption holds.
2. Existing coverage is much broader than this ticket originally assumed. `crates/worldwake-systems/src/combat.rs` already contains extensive authoritative tests for attack, defend, heal, loot, wound progression, death, dispatch wiring, and several spec acceptance checks.
3. Scheduler/replay infrastructure exists and is the real remaining gap: `step_tick()`, `Scheduler`, `SimulationState`, replay checkpointing, and workspace integration-test patterns are already available.
4. The original scope overstated the need for a brand-new “full E12 test suite.” Re-implementing every already-covered combat assertion in a second file would add duplication, not architectural value.
5. The E12 spec line claiming `BodyCostPerTick` should still accrue for dead agents is inconsistent with current death-finality architecture and existing `needs_system_skips_dead_agents_entirely()` behavior. The spec must be corrected; this ticket should not preserve that contradiction in tests.

## Architecture Check

1. Keep production architecture unchanged unless a test exposes a real defect. This ticket is test-first and should not introduce redundant wrappers or compatibility paths.
2. New coverage should live in `crates/worldwake-systems/tests/` and exercise the public scheduler/runtime path, complementing the existing focused tests in `src/combat.rs`.
3. The highest-value remaining assertions are:
   - scheduler-driven combat/death/loot behavior across ticks
   - conservation after combat-driven inventory transfer
   - replay determinism for combat inputs through `SimulationState`
4. Existing unit/system tests in `src/combat.rs`, `worldwake-sim`, and `worldwake-core` remain the authoritative home for lower-level invariants already covered there.

## What to Change

### 1. Add scheduler-level combat integration coverage

Add focused integration tests that exercise the public runtime path rather than duplicating every internal combat assertion:
- scheduler-driven attack commits a fatal wound, attaches `DeadAt`, and blocks further actions from the dead actor
- scheduler-driven loot transfers corpse inventory without violating commodity conservation

### 2. Add replay coverage for combat inputs

Record a combat scenario through `SimulationState`, replay it with the recorded inputs/checkpoints, and verify the final state hash matches.

### 3. Rely on existing lower-level coverage instead of duplicating it

Do not re-copy already-covered assertions for:
- weapon profiles and duration resolution
- wound append semantics
- defend guard bonus
- heal medicine consumption and wound reduction
- clotting/recovery math
- dispatch wiring and fatality event evidence

Those are already covered in `crates/worldwake-systems/src/combat.rs`, `crates/worldwake-core`, and `crates/worldwake-sim`.

## Files to Touch

- `crates/worldwake-systems/tests/e12_combat_integration.rs` (new)
- `specs/E12-combat-health.md` (scope correction for dead-agent body-cost assumption)

## Out of Scope

- Repeating every existing combat assertion in a second location
- Production refactors without a failing test that justifies them
- AI decision-making tests (E13)
- Cross-epic integration beyond E12-owned scheduler/runtime concerns

## Acceptance Criteria

### Tests That Must Pass

1. New scheduler-level E12 integration tests pass.
2. Combat replay produces identical checkpoints/final hash for recorded combat inputs.
3. Conservation holds after scheduler-driven combat + loot.
4. Existing E12 lower-level tests remain green.
5. `cargo test --workspace` passes.
6. `cargo clippy --workspace` passes.

### Invariants

1. Phase 2 gate intent for T14 is covered through dead-agent scheduler rejection/culling behavior.
2. Conservation still holds after combat-triggered inventory transfer.
3. Replay determinism still holds when combat actions are present.
4. No backward-compatibility paths or duplicate abstractions are introduced for tests.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/tests/e12_combat_integration.rs` — scheduler/replay/conservation coverage for E12 gaps

### Commands

1. `cargo test -p worldwake-systems --test e12_combat_integration`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - Reassessed the ticket against the implemented E12 code and found that most combat/healing/death coverage already existed in `crates/worldwake-systems/src/combat.rs`.
  - Added `crates/worldwake-systems/tests/e12_combat_integration.rs` for the missing scheduler-level coverage:
    - scheduler-driven combat death + loot with conservation checks
    - dead-actor attack rejection through the public input path
    - replay determinism for recorded combat inputs
  - Corrected the E12 spec assumption that dead agents should keep accruing `BodyCostPerTick`.
- Deviations from original plan:
  - Did not create a redundant second copy of every spec assertion.
  - Narrowed the work to the uncovered runtime/replay/conservation gaps instead of rebuilding a full duplicate combat suite.
- Verification results:
  - `cargo test -p worldwake-systems --test e12_combat_integration`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
