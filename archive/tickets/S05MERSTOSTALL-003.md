# S05MERSTOSTALL-003: Add store_stock and collect_display_stock actions

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new action definitions and handlers in worldwake-systems
**Deps**: S05MERSTOSTALL-001, S05MERSTOSTALL-002

## Problem

Merchants need explicit actions to move goods between direct possession and facility storage. Without `store_stock` and `collect_display_stock`, there is no mechanism for agents to interact with the facility container system introduced by S05MERSTOSTALL-001/002.

## Assumption Reassessment (2026-04-01)

1. `StockStoragePolicy` and `StockAssignment` exist in `trade.rs` — confirmed via S05MERSTOSTALL-001 outcome.
2. Facility creation helpers exist — assumed via S05MERSTOSTALL-002 dependency.
3. Action registration pattern established in `action_registry.rs` — confirmed existing action definitions follow this pattern.
4. Item movement between entities follows existing possession/containment patterns — check current `Possession` component and containment model.
5. `StockAssignmentKind::Stored` variant exists — confirmed via S05MERSTOSTALL-001.
6. Conservation invariant must hold — items moved, never created or destroyed.

## Architecture Check

1. New actions follow the existing action handler pattern in worldwake-systems. Each action validates preconditions, mutates state, and produces events. No new abstractions needed.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. `store_stock` moves lot from possession into stock container → event-log delta + authoritative world state (focused test)
2. `collect_display_stock` moves lot from container back to possession → event-log delta + authoritative world state (focused test)
3. Authorization checked — only facility controller can store/collect → action trace (focused test)
4. Conservation holds across store/collect → `verify_conservation` (focused test)

## What to Change

### 1. Create stock_actions.rs

In `crates/worldwake-systems/src/stock_actions.rs`, implement:
- `store_stock` action: validates agent controls facility, lot is in agent's possession, facility has `StockStoragePolicy`. Moves lot into `stock_container`, sets `StockAssignment { facility, kind: Stored }`.
- `collect_display_stock` action: validates agent controls facility, lot is in a facility container (stored or displayed). Moves lot back to agent's possession, clears `StockAssignment`.

### 2. Register in action_registry.rs

Add both actions to the action registry following existing patterns.

### 3. Update lib.rs

Re-export the new module from `crates/worldwake-systems/src/lib.rs`.

## Files to Touch

- `crates/worldwake-systems/src/stock_actions.rs` (new)
- `crates/worldwake-systems/src/action_registry.rs` (modify)
- `crates/worldwake-systems/src/lib.rs` (modify)
- `crates/worldwake-systems/src/action_handler.rs` (modify — if handler wiring needed)

## Out of Scope

- Stage/unstage actions (004)
- Sale visibility evolution (005)
- AI planning for stock actions (007)
- Theft distinction (008)

## Acceptance Criteria

### Tests That Must Pass

1. `store_stock` moves lot into stock container and sets `StockAssignment` to `Stored`
2. `collect_display_stock` moves lot from container to possession and clears `StockAssignment`
3. Authorization check: non-controller agent cannot store/collect
4. Conservation holds across both actions
5. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Items cannot be created/destroyed — conservation enforced by `verify_conservation`
2. Every entity exists in exactly one place after action completes
3. `StockAssignment` always matches the lot's actual location

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/stock_actions.rs` — store_stock moves lot, sets assignment, conservation holds
2. `crates/worldwake-systems/src/stock_actions.rs` — collect_display_stock reverses store, clears assignment
3. `crates/worldwake-systems/src/stock_actions.rs` — authorization rejection for non-controller

### Commands

1. `cargo test -p worldwake-systems -- stock`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome (2026-04-01)

### What changed

1. Created `crates/worldwake-systems/src/stock_actions.rs` with `store_stock` and `collect_display_stock` action handlers (start/tick/commit/abort for each)
2. `store_stock`: validates actor possesses lot and controls a local facility, moves lot from possession into `stock_container` via `put_into_container`, sets `StockAssignment { kind: Stored }`
3. `collect_display_stock`: validates lot has `StockAssignment` and actor controls the facility, moves lot from container to direct possession via `move_entity_to_direct_possession`, clears `StockAssignment`
4. `resolve_controlled_facility` helper finds a `StockStoragePolicy`-bearing facility at the actor's place that the actor can control
5. Registered both actions in `action_registry.rs`, declared module and re-exported `register_stock_actions` in `lib.rs`
6. 5 focused tests: store moves lot, store sets assignment, collect reverses, authorization rejection, conservation

### Deviations

- Used `ActionDomain::Transport` (not a new domain) — stock movement is physical transport of goods
- Used `PreconditionFailed` for missing target and actor place errors — `ActionError` has no `MissingTarget` or `InvalidActor` variants

### Verification

- `cargo test -p worldwake-systems`: 441 passed, 0 failed (436 baseline + 5 new)
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
