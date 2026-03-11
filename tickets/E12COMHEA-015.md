# E12COMHEA-015: Integration tests — multi-tick combat scenarios

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None — tests only
**Deps**: All E12COMHEA tickets (001-014)

## Problem

E12 needs end-to-end integration tests that verify the full combat lifecycle across multiple ticks: attack → wound → bleed → clot → recover/die → loot. These tests verify cross-system interactions, invariants, and the complete spec acceptance criteria.

## Assumption Reassessment (2026-03-11)

1. All prior E12 tickets provide the components, actions, handlers, and system tick — this ticket only adds tests.
2. Test infrastructure exists: `build_prototype_world()`, `SimulationState`, `step_tick()` — confirmed.
3. E12 spec lists 25+ specific test cases — all must be covered.

## Architecture Check

1. Integration tests live in `crates/worldwake-systems/tests/` or as `#[cfg(test)]` modules.
2. Tests set up a complete simulation state, submit inputs, step multiple ticks, and verify outcomes.
3. No new production code — only test code.

## What to Change

### 1. Multi-tick combat scenario tests

Test the complete lifecycle:
- Agent A attacks Agent B → wound appears → bleeding progresses → severity increases
- Agent B defends → guard_skill boosted → fewer/lighter wounds
- Agent C heals Agent B → bleeding stops → severity decreases → wound heals
- Agent A attacks until Agent B dies → DeadAt attached → scheduler excludes B
- Agent A loots Agent B → items transfer → conservation holds

### 2. Spec acceptance criteria tests

From the spec test list:
- T14: Dead agents generate no new plans or actions
- Combat resolves deterministically with same RNG state
- New wounds append with correct cause and body_part
- Per-wound bleeding increases severity over time
- Treatment reduces bleeding and consumes medicine
- Non-bleeding wounds naturally stabilize/recover under acceptable conditions
- Deprivation wounds coexist with combat wounds in same WoundList
- Death triggers when wound load reaches wound_capacity
- Incapacitation triggers when wound load reaches incapacitation_threshold
- Corpse retains inventory and location context
- Death event traces back to cause chain
- Cannot attack dead agents
- No stored Health component exists
- Different CombatProfile values produce different outcomes
- Durations derive from weapon/medicine profiles
- DeadAt attached on death, scheduler excludes
- Scheduler excludes agents with DeadAt from planning and action starts
- Loot transfers items from dead agent
- Defend boosts effective guard_skill
- Sword and Bow in CommodityKind with TradeCategory::Weapon
- Natural clotting reduces bleed_rate_per_tick over time
- Recovery only when not bleeding and physiological conditions acceptable
- BodyCostPerTick still accrues for dead agents
- DurationExpr::Indefinite keeps Defend running until cancelled
- CombatWeaponRef::Commodity(Sword) produces different profile than Unarmed

### 3. Deterministic replay test

Run a combat scenario, record the outcome, replay with same seed, verify identical result.

### 4. Conservation test

After any combat/loot sequence, verify `verify_conservation()` passes.

## Files to Touch

- `crates/worldwake-systems/tests/combat_integration.rs` (new)
- Or `crates/worldwake-systems/src/combat.rs` (modify — add integration test module)

## Out of Scope

- Changing any production code
- Adding new features or components
- AI decision-making tests (E13)
- Cross-epic integration (E22)

## Acceptance Criteria

### Tests That Must Pass

1. All 25+ spec test cases listed above pass
2. Multi-tick scenarios complete without panics
3. Deterministic replay produces identical outcomes
4. Conservation invariant holds after combat + loot
5. No stored Health component exists (compile-time or runtime check)
6. `cargo test --workspace` — all tests pass
7. `cargo clippy --workspace` — no warnings

### Invariants

1. All E12 spec acceptance criteria verified
2. Phase 2 gate test T14 (dead agents inactive) passes
3. Conservation (T02) still passes after combat sequences
4. Replay determinism (T08) still passes with combat actions

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/tests/combat_integration.rs` — full integration suite

### Commands

1. `cargo test -p worldwake-systems -- combat_integration`
2. `cargo test --workspace && cargo clippy --workspace`
