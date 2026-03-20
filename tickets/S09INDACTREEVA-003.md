# S09INDACTREEVA-003: Switch defend action definition to `ActorDefendStance`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — defend action changes from indefinite to finite profile-driven duration
**Deps**: S09INDACTREEVA-002 (`DurationExpr::ActorDefendStance` must exist and resolve correctly)

## Problem

The defend action currently uses `DurationExpr::Indefinite`, causing agents to defend forever and never re-evaluate. Switching to `DurationExpr::ActorDefendStance` makes defend a finite action driven by `CombatProfile.defend_stance_ticks`. When defend expires, the agent re-enters the decision cycle and either re-selects defend (if danger persists) or switches goals.

## Assumption Reassessment (2026-03-20)

1. `defend_action_def()` in `crates/worldwake-systems/src/combat.rs` line 399 uses `duration: DurationExpr::Indefinite` (verified).
2. Test at line 1613 asserts `defend.duration == DurationExpr::Indefinite` (verified).
3. Tests at lines 2876 and 2904 assert `ActionDuration::Indefinite` for active defend actions (verified).
4. After this ticket, no production action definition uses `DurationExpr::Indefinite`. Test code in other files still references it — that's cleaned up in ticket 004.
5. Not an AI regression, ordering, heuristic, stale-request, political, or ControlSource ticket.
6. No mismatch — spec matches codebase.

## Architecture Check

1. Switching one field in `defend_action_def()` from `DurationExpr::Indefinite` to `DurationExpr::ActorDefendStance` is the minimal change. No shims, no conditional logic — the old variant is simply no longer used by any action definition.
2. The defend action already has `FreelyInterruptible` semantics, so the interrupt system can still force re-evaluation before the duration expires if a higher-priority goal emerges.

## Verification Layers

1. Defend action def uses `DurationExpr::ActorDefendStance` -> focused unit test assertion
2. Active defend action resolves to `ActionDuration::Finite(defend_stance_ticks)` -> integration test assertion
3. Defend action completes after `defend_stance_ticks` ticks -> action lifecycle test (or updated existing test)
4. Single-file behavioral change — integration/golden coverage deferred to ticket 005.

## What to Change

### 1. Change defend action definition

In `crates/worldwake-systems/src/combat.rs`:
- Line 399: Change `duration: DurationExpr::Indefinite` to `duration: DurationExpr::ActorDefendStance`

### 2. Update unit test assertions

In `crates/worldwake-systems/src/combat.rs`:
- Line 1613: Change assertion from `DurationExpr::Indefinite` to `DurationExpr::ActorDefendStance`
- Lines 2876, 2904: Change assertions from `ActionDuration::Indefinite` to `ActionDuration::Finite(10)` (matching `defend_stance_ticks: nz(10)` from the test profile)

### 3. Verify defend_stance_ticks value in test profiles

Confirm the combat profiles used in the tests at lines 2876 and 2904 have `defend_stance_ticks: nz(10)` (from ticket 001). The expected `Finite` value must match.

## Files to Touch

- `crates/worldwake-systems/src/combat.rs` (modify)

## Out of Scope

- Removing `DurationExpr::Indefinite` from the enum (ticket 004)
- Removing `ActionDuration::Indefinite` from the enum (ticket 004)
- Cleaning up `Indefinite` references in sim, ai, or cli crates (ticket 004)
- Golden E2E test for defend re-evaluation cycle (ticket 005)
- Any changes to the planner, scheduler, or other action definitions

## Acceptance Criteria

### Tests That Must Pass

1. `defend_action_def().duration == DurationExpr::ActorDefendStance` (updated assertion at line 1613)
2. Active defend action resolves to `ActionDuration::Finite(10)` (updated assertions at lines 2876, 2904)
3. Existing suite: `cargo test -p worldwake-systems`
4. Existing suite: `cargo test --workspace` (no regressions — planner and other systems still work)

### Invariants

1. Defend is the only action that previously used `DurationExpr::Indefinite` — after this ticket, no action def uses `Indefinite`
2. Defend still has `FreelyInterruptible` semantics (unchanged)
3. Defend duration is now agent-specific via `CombatProfile.defend_stance_ticks`
4. All other combat actions (attack, loot) are unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs` — 3 assertion updates: action def duration variant + 2 active-action duration assertions

### Commands

1. `cargo test -p worldwake-systems` — targeted systems tests
2. `cargo test --workspace` — full workspace regression
3. `cargo clippy --workspace` — lint check
