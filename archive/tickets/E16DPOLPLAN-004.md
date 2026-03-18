# E16DPOLPLAN-004: Implement `apply_planner_step` for `PlannerOpKind::Threaten` + exhaustive match arms

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — goal_model.rs planner step logic
**Deps**: E16DPOLPLAN-002

## Problem

`apply_planner_step` has a `_ => state` catch-all that silently skips Threaten. Additionally, the catch-all prevents compile-time detection of new `PlannerOpKind` variants.

## Assumption Reassessment (2026-03-18)

1. `PlannerOpKind::Threaten` is defined in `planner_ops.rs` — confirmed
2. `ActionPayload::as_threaten()` exists and returns threaten payload data — confirmed
3. `state.combat_profile(actor)` returns `Option<CombatProfile>` with `attack_skill` field — confirmed
4. `state.courage(target)` now available after E16DPOLPLAN-002 — confirmed dependency
5. `threat_pressure()` in `office_actions.rs` returns `profile.attack_skill` — confirmed as reference
6. All 19 `PlannerOpKind` variants: Travel, Consume, Sleep, Relieve, Wash, Heal, Loot, Bury, QueueForFacilityUse, DeclareSupport, Bribe, Threaten, Trade, Harvest, Craft, Attack, Defend, Tell, MoveCargo — confirmed

## Architecture Check

1. Conservative defaults: missing `attack_skill` -> `Permille::ZERO`, missing `courage` -> `Permille::MAX` — ensures planner only selects Threaten when outcome is predictable
2. Exhaustive match arms with explicit no-op listing forces compile errors on new variants — prevents silent regressions

## What to Change

### 1. Add `PlannerOpKind::Threaten` arm

Under `GoalKind::ClaimOffice { office }`:
- Read threaten payload from `payload_override.and_then(ActionPayload::as_threaten)`
- Read actor's `attack_skill` from `combat_profile` (default `Permille::ZERO`)
- Read target's `courage` from snapshot (default `Permille::MAX`)
- If `attack_skill > courage`: add `with_support_declaration(target, office, actor)`
- Otherwise: return state unchanged

### 2. Replace `_ => state` catch-all with explicit no-op arms

After Bribe (E16DPOLPLAN-003) and Threaten arms are added, replace the remaining `_ => state` with:
```rust
PlannerOpKind::Trade
| PlannerOpKind::Harvest
| PlannerOpKind::Craft
| PlannerOpKind::Attack
| PlannerOpKind::Defend
| PlannerOpKind::Tell
| PlannerOpKind::MoveCargo => state,
```

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify)

## Out of Scope

- Bribe arm (E16DPOLPLAN-003)
- Unit tests (E16DPOLPLAN-005)
- Integration tests (E16DPOLPLAN-006)
- Golden tests
- BlockedIntent for failed threats (E16DPOLPLAN-019)
- Changes to `commit_threaten` authoritative handler

## Acceptance Criteria

### Tests That Must Pass

1. Code compiles with no `_ => state` catch-all remaining in `apply_planner_step`
2. Existing suite: `cargo test -p worldwake-ai`
3. Adding a hypothetical new `PlannerOpKind` variant would produce a compile error

### Invariants

1. Threaten arm only activates under `GoalKind::ClaimOffice` — all other goals return state unchanged
2. Missing combat profile -> no threat possible (conservative)
3. Missing courage -> target resists (conservative)
4. No `_ => state` catch-all remains — all `PlannerOpKind` variants explicitly handled

## Test Plan

### New/Modified Tests

1. Tests deferred to E16DPOLPLAN-005

### Commands

1. `cargo build -p worldwake-ai`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-18
- **What changed**: `crates/worldwake-ai/src/goal_model.rs` — added `PlannerOpKind::Threaten` arm delegating to new `apply_threaten_for_office()` helper; replaced `_ => state` catch-all with explicit no-op arms for 7 remaining variants (Trade, Harvest, Craft, Attack, Defend, Tell, MoveCargo). Added `#[allow(clippy::too_many_lines)]` on `apply_planner_step` (102 lines, limit 100).
- **Deviations**: Used `Permille::new_unchecked(0)` / `Permille::new_unchecked(1000)` instead of `Permille::ZERO` / `Permille::MAX` (those constants don't exist). Used `map_or` instead of `map().unwrap_or()` per clippy pedantic.
- **Verification**: `cargo build -p worldwake-ai` ✅, `cargo test -p worldwake-ai` (404 tests) ✅, `cargo clippy --workspace` ✅
