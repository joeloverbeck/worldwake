# S09INDACTREEVA-002: Add `DurationExpr::ActorDefendStance` variant and wire resolution

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `DurationExpr` enum gains new variant; `resolve_for()` and `estimate_duration_from_beliefs()` gain new arms
**Deps**: S09INDACTREEVA-001 (`defend_stance_ticks` must exist on `CombatProfile`)

## Problem

The defend action needs a finite, profile-driven duration. Before removing `DurationExpr::Indefinite`, the replacement variant `ActorDefendStance` must exist and resolve correctly — both authoritatively (via `resolve_for()`) and from beliefs (via `estimate_duration_from_beliefs()`).

## Assumption Reassessment (2026-03-20)

1. `DurationExpr` has 8 variants (verified: `action_semantics.rs` lines 97–115). `Indefinite` is at line 109.
2. `resolve_for()` handles all variants at lines 132–232. `CombatWeapon` reads from `CombatProfile` at lines 184–199 — `ActorDefendStance` follows the same pattern.
3. `estimate_duration_from_beliefs()` in `belief_view.rs` lines 678–754. `Indefinite` arm at line 714. `CombatWeapon` arm at lines 715–725 — `ActorDefendStance` follows the same pattern.
4. `fixed_ticks()` at lines 118–130 returns `None` for all non-Fixed variants. `ActorDefendStance` should also return `None` (resolved at action start).
5. `ALL_DURATION_EXPRS` test array at line 356 lists all variants for roundtrip testing.
6. Not an AI regression, ordering, heuristic, stale-request, political, or ControlSource ticket.
7. No mismatch — spec matches codebase.

## Architecture Check

1. `ActorDefendStance` follows the exact same resolution pattern as `CombatWeapon` — reads a `NonZeroU32` from `CombatProfile` and returns `ActionDuration::Finite(n)`. This is the established pattern for profile-driven durations.
2. No backwards-compatibility shims — `Indefinite` is NOT removed in this ticket (that's ticket 004). Both variants coexist temporarily.

## Verification Layers

1. `ActorDefendStance` resolves to `Finite(defend_stance_ticks)` when actor has `CombatProfile` -> focused unit test in `action_semantics.rs`
2. `ActorDefendStance` returns `Err` when actor lacks `CombatProfile` -> focused unit test in `action_semantics.rs`
3. `estimate_duration_from_beliefs()` returns `Finite(defend_stance_ticks)` for `ActorDefendStance` -> focused unit test in `belief_view.rs`
4. `fixed_ticks()` returns `None` for `ActorDefendStance` -> focused unit test
5. Bincode roundtrip for `ActorDefendStance` succeeds -> existing roundtrip test updated

## What to Change

### 1. Add `ActorDefendStance` to `DurationExpr` enum

In `crates/worldwake-sim/src/action_semantics.rs`:
- Add `ActorDefendStance` variant to `DurationExpr` (after `ActorTradeDisposition`, before `Indefinite`)
- Add `| Self::ActorDefendStance` to the `fixed_ticks()` catch-all arm (line 126) that returns `None`

### 2. Add `resolve_for()` arm for `ActorDefendStance`

In `crates/worldwake-sim/src/action_semantics.rs` `resolve_for()`:
```rust
Self::ActorDefendStance => world
    .get_component_combat_profile(actor)
    .map(|profile| ActionDuration::Finite(profile.defend_stance_ticks.get()))
    .ok_or_else(|| format!("actor {actor} lacks combat profile")),
```

### 3. Add `estimate_duration_from_beliefs()` arm for `ActorDefendStance`

In `crates/worldwake-sim/src/belief_view.rs`:
```rust
DurationExpr::ActorDefendStance => view
    .combat_profile(actor)
    .map(|profile| ActionDuration::Finite(profile.defend_stance_ticks.get())),
```

### 4. Update tests

- Add `DurationExpr::ActorDefendStance` to `ALL_DURATION_EXPRS` array
- Add resolve test: actor with `CombatProfile` -> `Finite(defend_stance_ticks)`
- Add resolve test: actor without `CombatProfile` -> `Err`
- Add `fixed_ticks()` test: `ActorDefendStance` -> `None`

## Files to Touch

- `crates/worldwake-sim/src/action_semantics.rs` (modify)
- `crates/worldwake-sim/src/belief_view.rs` (modify)

## Out of Scope

- Removing `DurationExpr::Indefinite` (ticket 004)
- Removing `ActionDuration::Indefinite` (ticket 004)
- Changing the defend action definition from `Indefinite` to `ActorDefendStance` (ticket 003)
- Any planner or scheduler changes
- CLI display changes
- Golden tests

## Acceptance Criteria

### Tests That Must Pass

1. New: `ActorDefendStance` resolves to `ActionDuration::Finite(defend_stance_ticks)` with `CombatProfile` present
2. New: `ActorDefendStance` returns `Err` without `CombatProfile`
3. New: `ActorDefendStance.fixed_ticks()` returns `None`
4. Updated: `ALL_DURATION_EXPRS` bincode roundtrip includes `ActorDefendStance`
5. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. `DurationExpr` now has 9 variants (8 existing + `ActorDefendStance`)
2. `Indefinite` still exists and is unchanged (coexists temporarily)
3. `ActorDefendStance` follows the same resolution pattern as `CombatWeapon`
4. No behavioral change to any running action — `ActorDefendStance` is not yet used by any action def

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_semantics.rs` — resolve test for `ActorDefendStance` with/without `CombatProfile`; `fixed_ticks` assertion; bincode roundtrip
2. `crates/worldwake-sim/src/belief_view.rs` — estimation test for `ActorDefendStance` (if belief_view tests exist for other variants; otherwise covered by integration)

### Commands

1. `cargo test -p worldwake-sim` — targeted sim tests
2. `cargo test --workspace` — full workspace regression
3. `cargo clippy --workspace` — lint check
