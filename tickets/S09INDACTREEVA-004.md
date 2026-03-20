# S09INDACTREEVA-004: Remove `Indefinite` from `DurationExpr` and `ActionDuration`; clean up all consumers

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — two enum variants removed; Principle 8 enforced at the type level
**Deps**: S09INDACTREEVA-003 (no production action def uses `Indefinite` after 003)

## Problem

After ticket 003 switches defend to `ActorDefendStance`, no production action definition uses `Indefinite`. The `DurationExpr::Indefinite` and `ActionDuration::Indefinite` variants are dead code. Removing them enforces Principle 8 (every action has a finite duration) at the type level — no future action can be indefinite.

## Assumption Reassessment (2026-03-20)

1. Production `Indefinite` usages (verified via grep — all are in test code or match arms after ticket 003):
   - `action_semantics.rs`: enum variant (line 109), `fixed_ticks` arm (line 126), `resolve_for` arm (line 183), test array (line 356), test assertions (lines 410, 687, 690)
   - `action_duration.rs`: enum variant (line 7), `fixed_ticks` arm (line 15), `advance` arm (line 28), tests (lines 58, 61, 66)
   - `start_gate.rs`: `reservation_range` arm (line 202), test code (lines 578, 619, 646)
   - `belief_view.rs`: estimation arm (line 714)
   - `tick_action.rs`: test code (lines 535, 852)
   - `affordance_query.rs`: fallback (line 719), test code (lines 1217, 1324)
   - `trade_valuation.rs`: fallback (line 529)
   - `search.rs`: two match arms (lines 394, 395)
   - `cli/handlers/tick.rs`: display arm (line 103)
   - `cli/handlers/world_overview.rs`: display arm (line 127)
2. After ticket 003, zero production action definitions reference `Indefinite`.
3. Not an AI regression, ordering, heuristic, stale-request, political, or ControlSource ticket.
4. No mismatch — spec matches codebase.

## Architecture Check

1. Removing enum variants is compiler-assisted — every forgotten match arm becomes a compile error. This is the safest kind of removal.
2. No backwards-compatibility shims. The variants are deleted, not deprecated.
3. `ActionDuration` becomes a single-variant enum `Finite(u32)`. The spec notes a newtype `ActionDuration(u32)` is an alternative, but keeping the enum preserves naming consistency and allows future variants if needed. Either form is acceptable.

## Verification Layers

1. `DurationExpr` has exactly 8 variants (no `Indefinite`) -> compile-time enforcement
2. `ActionDuration` has exactly 1 variant `Finite(u32)` (no `Indefinite`) -> compile-time enforcement
3. `advance()` always eventually returns `true` (finite) -> existing unit test
4. `fixed_ticks()` on `ActionDuration` always returns `Some` -> updated unit test
5. Planner `estimated_ticks` match has no zero-cost hack -> code inspection
6. `cargo test --workspace` passes -> full regression

## What to Change

### 1. Remove `DurationExpr::Indefinite`

In `crates/worldwake-sim/src/action_semantics.rs`:
- Remove `Indefinite` variant from the enum
- Remove `| Self::Indefinite` from `fixed_ticks()` catch-all (line 126)
- Remove `Self::Indefinite => Ok(ActionDuration::Indefinite)` from `resolve_for()` (line 183)
- Remove `DurationExpr::Indefinite` from `ALL_DURATION_EXPRS` test array (line 356)
- Remove test assertion for `Indefinite.fixed_ticks()` (line 410)
- Remove test that resolves `Indefinite` -> `ActionDuration::Indefinite` (lines 687–690)
- Update bincode roundtrip test if it enumerates variants

### 2. Remove `ActionDuration::Indefinite`

In `crates/worldwake-sim/src/action_duration.rs`:
- Remove `Indefinite` variant from the enum
- Simplify `fixed_ticks()`: always returns `Some(ticks)` (remove `Indefinite => None` at line 15)
- Simplify `advance()`: remove `Indefinite => false` arm (line 28)
- Remove `indefinite_duration_never_auto_completes` test (lines 58–61)
- Update bincode roundtrip test to remove `ActionDuration::Indefinite` (line 66)

### 3. Remove `Indefinite` from `belief_view.rs`

In `crates/worldwake-sim/src/belief_view.rs`:
- Remove `DurationExpr::Indefinite => Some(ActionDuration::Indefinite)` arm (line 714)

### 4. Remove `Indefinite` from `start_gate.rs`

In `crates/worldwake-sim/src/start_gate.rs`:
- Remove `ActionDuration::Indefinite => Err(...)` arm from `reservation_range()` (lines 202–205)
- Update test at line 578 that constructs `DurationExpr::Indefinite`
- Update test at line 619 that constructs `ActionDuration::Indefinite`
- Update test at line 646 that sets `DurationExpr::Indefinite`

### 5. Simplify planner search

In `crates/worldwake-ai/src/search.rs`:
- Remove `ActionDuration::Indefinite if semantics.may_appear_mid_plan => return None` (line 394)
- Remove `ActionDuration::Indefinite => 0` (line 395)
- The match becomes: `let estimated_ticks = match duration { ActionDuration::Finite(ticks) => ticks };`

### 6. Clean up remaining sim files

In `crates/worldwake-sim/src/tick_action.rs`:
- Remove test code using `DurationExpr::Indefinite` (line 535) and `ActionDuration::Indefinite` (line 852)

In `crates/worldwake-sim/src/affordance_query.rs`:
- Remove `.or(Some(crate::ActionDuration::Indefinite))` fallback (line 719) — if duration can't be estimated, return `None`
- Remove test `DurationExpr::Indefinite` usages (lines 1217, 1324)

In `crates/worldwake-sim/src/trade_valuation.rs`:
- Remove `.or(Some(crate::ActionDuration::Indefinite))` fallback (line 529) — if duration can't be estimated, return `None`

### 7. Clean up CLI display

In `crates/worldwake-cli/src/handlers/tick.rs`:
- Remove `ActionDuration::Indefinite => "indefinite".to_string()` arm (line 103)

In `crates/worldwake-cli/src/handlers/world_overview.rs`:
- Remove `ActionDuration::Indefinite => String::new()` arm (line 127)

### 8. Update CLAUDE.md

In `CLAUDE.md`:
- Line 109: Change "Finite or Indefinite" to "resolved runtime duration for active actions (always finite)"

## Files to Touch

- `crates/worldwake-sim/src/action_semantics.rs` (modify)
- `crates/worldwake-sim/src/action_duration.rs` (modify)
- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/start_gate.rs` (modify)
- `crates/worldwake-sim/src/tick_action.rs` (modify)
- `crates/worldwake-sim/src/affordance_query.rs` (modify)
- `crates/worldwake-sim/src/trade_valuation.rs` (modify)
- `crates/worldwake-ai/src/search.rs` (modify)
- `crates/worldwake-cli/src/handlers/tick.rs` (modify)
- `crates/worldwake-cli/src/handlers/world_overview.rs` (modify)
- `CLAUDE.md` (modify)

## Out of Scope

- `CombatProfile` changes (completed in ticket 001)
- `DurationExpr::ActorDefendStance` addition (completed in ticket 002)
- Defend action definition change (completed in ticket 003)
- Golden E2E test for defend re-evaluation (ticket 005)
- Refactoring `ActionDuration` from a single-variant enum to a newtype — acceptable as a future cleanup but not required

## Acceptance Criteria

### Tests That Must Pass

1. `cargo build --workspace` — compiles with no `Indefinite` variant anywhere
2. `cargo test --workspace` — all tests pass (test code using `Indefinite` removed or updated)
3. `cargo clippy --workspace` — no warnings
4. No grep hits for `Indefinite` in `crates/` (excluding comments if any)

### Invariants

1. `DurationExpr` has exactly 8 variants: `Fixed`, `TargetConsumable`, `TravelToTarget`, `ActorMetabolism`, `ActorTradeDisposition`, `ActorDefendStance`, `CombatWeapon`, `TargetTreatment`
2. `ActionDuration` has exactly 1 variant: `Finite(u32)`
3. `ActionDuration::fixed_ticks()` always returns `Some` — never `None`
4. `ActionDuration::advance()` always eventually returns `true` — no action runs forever
5. Planner `build_successor()` always gets a real tick cost — no 0-tick hack
6. Affordance cost estimation and trade valuation return `None` (not `Indefinite`) when duration is unknown
7. Principle 8 enforced at the type level: no code path can produce an indefinite action

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_duration.rs` — remove `indefinite_duration_never_auto_completes`; update bincode roundtrip
2. `crates/worldwake-sim/src/action_semantics.rs` — remove `Indefinite` from test arrays and assertions; update bincode roundtrip
3. `crates/worldwake-sim/src/start_gate.rs` — remove/update 3 test sites using `Indefinite`
4. `crates/worldwake-sim/src/tick_action.rs` — remove 2 test sites using `Indefinite`
5. `crates/worldwake-sim/src/affordance_query.rs` — remove 2 test sites using `Indefinite`

### Commands

1. `cargo build --workspace` — compilation verification (compiler catches missing arms)
2. `cargo test --workspace` — full regression
3. `cargo clippy --workspace` — lint check
4. `grep -r "Indefinite" crates/` — verify zero remaining references
