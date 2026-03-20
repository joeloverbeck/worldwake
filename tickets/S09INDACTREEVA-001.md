# S09INDACTREEVA-001: Add `defend_stance_ticks` field to `CombatProfile`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `CombatProfile` struct gains 11th field
**Deps**: None

## Problem

The defend action currently uses `DurationExpr::Indefinite`, which has no finite endpoint. To replace it with a profile-driven duration (`DurationExpr::ActorDefendStance`), the `CombatProfile` struct must first carry the `defend_stance_ticks` parameter. This is the prerequisite for all subsequent S09 tickets.

## Assumption Reassessment (2026-03-20)

1. `CombatProfile` currently has exactly 10 fields (verified: `crates/worldwake-core/src/combat.rs` lines 9–20). Constructor `CombatProfile::new()` takes 10 positional parameters (lines 23–49).
2. `sample_combat_profile()` test helper exists at line 84–97 in `combat.rs`.
3. Not an AI regression ticket — this is a pure data-model addition.
4. No ordering dependency.
5. No heuristic removal.
6. Not a stale-request ticket.
7. Not a political ticket.
8. No ControlSource manipulation.
9. No golden scenario isolation.
10. No mismatch — spec matches codebase.

## Architecture Check

1. Adding a field to `CombatProfile` follows the existing pattern where per-agent combat parameters are profile-driven (e.g., `unarmed_attack_ticks: NonZeroU32`). The `defend_stance_ticks` field mirrors this pattern exactly.
2. No backwards-compatibility shims — all call sites are updated atomically.

## Verification Layers

1. `CombatProfile::new()` accepts 11 parameters and stores `defend_stance_ticks` -> focused unit test
2. `sample_combat_profile()` returns a profile with `defend_stance_ticks` populated -> existing test helper usage
3. All existing tests continue to compile and pass with the new argument -> `cargo test --workspace`
4. Single-layer ticket: purely additive struct change with no behavioral impact yet.

## What to Change

### 1. Add `defend_stance_ticks` field to `CombatProfile`

In `crates/worldwake-core/src/combat.rs`:
- Add `pub defend_stance_ticks: NonZeroU32` as the 11th field on the struct (after `unarmed_attack_ticks`)
- Add the 11th parameter to `CombatProfile::new()` constructor
- Update `sample_combat_profile()` to include `defend_stance_ticks: nz(10)` (or use `NonZeroU32::new(10).unwrap()`)

### 2. Update all `CombatProfile::new()` call sites

Every call site must pass a `defend_stance_ticks` value as the 11th argument. Default: `nz(10)` unless a specific test needs a different value.

Call sites per the spec (39 occurrences across ~18 files):

| Crate | File | Count | Context |
|-------|------|-------|---------|
| core | `combat.rs` | 1 | `sample_combat_profile()` |
| core | `world.rs` | 1 | Test helper |
| core | `delta.rs` | 1 | Test helper |
| core | `component_tables.rs` | 2 | Test helpers |
| core | `wounds.rs` | 1 | Test helper |
| sim | `action_semantics.rs` | 1 | Duration resolution test |
| sim | `action_validation.rs` | 1 | Action validation test |
| sim | `start_gate.rs` | 1 | Start gate test |
| systems | `combat.rs` | 7 | Defend handler + combat tests |
| systems | `office_actions.rs` | 2 | Office action tests |
| systems | `tests/e12_combat_integration.rs` | 2 | Integration tests |
| ai | `goal_model.rs` | 2 | Goal model tests |
| ai | `plan_revalidation.rs` | 1 | Plan revalidation test |
| ai | `search.rs` | 1 | Search test |
| ai | `tests/golden_harness/mod.rs` | 1 | Golden test harness default |
| ai | `tests/golden_combat.rs` | 4 | Combat golden tests |
| ai | `tests/golden_emergent.rs` | 3 | Emergent golden tests |
| ai | `tests/golden_production.rs` | 1 | Production golden tests |
| ai | `tests/golden_offices.rs` | 1 | Office golden tests |

## Files to Touch

- `crates/worldwake-core/src/combat.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify)
- `crates/worldwake-core/src/component_tables.rs` (modify)
- `crates/worldwake-core/src/wounds.rs` (modify)
- `crates/worldwake-sim/src/action_semantics.rs` (modify)
- `crates/worldwake-sim/src/action_validation.rs` (modify)
- `crates/worldwake-sim/src/start_gate.rs` (modify)
- `crates/worldwake-systems/src/combat.rs` (modify)
- `crates/worldwake-systems/src/office_actions.rs` (modify)
- `crates/worldwake-systems/tests/e12_combat_integration.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify)
- `crates/worldwake-ai/src/search.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)
- `crates/worldwake-ai/tests/golden_combat.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `crates/worldwake-ai/tests/golden_production.rs` (modify)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify)

## Out of Scope

- Adding `DurationExpr::ActorDefendStance` (ticket 002)
- Removing `DurationExpr::Indefinite` or `ActionDuration::Indefinite` (ticket 004)
- Changing the defend action definition (ticket 003)
- Any behavioral changes to defend or the planner
- Modifying any `DurationExpr` or `ActionDuration` enums
- Golden tests for defend re-evaluation (ticket 005)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core` — all core tests compile and pass with 11-field `CombatProfile`
2. `cargo test --workspace` — all workspace tests compile and pass (no regressions from call site updates)
3. `cargo clippy --workspace` — no new warnings

### Invariants

1. `CombatProfile` has exactly 11 fields, all `pub`, with `defend_stance_ticks: NonZeroU32` as the last field
2. `CombatProfile::new()` is `const fn` and accepts 11 positional parameters
3. No behavioral change to any existing action, system, or AI logic — this is purely additive
4. All existing tests pass without changes to their assertions (only constructor calls change)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/combat.rs` — `sample_combat_profile()` updated to include `defend_stance_ticks: nz(10)`
2. All 39 call sites updated mechanically — no new test logic, just adding the 11th argument

### Commands

1. `cargo test -p worldwake-core` — targeted core tests
2. `cargo test --workspace` — full workspace regression
3. `cargo clippy --workspace` — lint check
