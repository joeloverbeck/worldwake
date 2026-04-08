# S75BELVDECOM-005: Extract CombatBeliefView + EconomicBeliefView sub-traits

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — RuntimeBeliefView trait decomposition
**Deps**: S75BELVDECOM-001

## Problem

Extract CombatBeliefView (10 methods: combat profile, wounds, hostiles, courage, patrol, pursuit) and EconomicBeliefView (9 methods: trade, sale, demand, merchandise, valuation) from RuntimeBeliefView.

## Assumption Reassessment (2026-04-08)

1. CombatBeliefView methods confirmed (10): `combat_profile`, `courage`, `consultation_speed_factor`, `wounds`, `hostile_targets_of`, `visible_hostiles_for`, `current_attackers_of`, `patrol_profile`, `pursuit_profile`, `has_wounds`.
2. EconomicBeliefView methods confirmed (9): `trade_disposition_profile`, `commodity_valuation_profile`, `controlled_commodity_quantity_at_place`, `local_controlled_lots_for`, `listed_sale_lots_at`, `seller_for_sale_lot`, `has_sale_listing`, `demand_memory`, `merchandise_profile`.
3. Same 18 impl blocks.

## Architecture Check

1. Same supertrait pattern. No backward-compatibility shims.
2. `patrol_profile` is categorized under Combat (not Profile) per the spec's note that categorization is finalized during ticket decomposition. The rationale: `patrol_profile` is consumed alongside `pursuit_profile` and `combat_profile` in combat/patrol planning contexts, not alongside needs/metabolism.

## Verification Layers

1. Combat queries -> golden tests exercise wound, hostile, and combat profile queries via combat scenarios
2. Economic queries -> golden tests exercise trade/sale queries via merchant and trade scenarios
3. Compile-time proof -> `cargo build --workspace`

## What to Change

### 1. Define CombatBeliefView and EconomicBeliefView sub-traits

Move 10 combat and 9 economic method signatures from RuntimeBeliefView.

### 2. Add supertrait bounds and remove methods from RuntimeBeliefView

### 3. Update all 18 impl blocks

### 4. Export new sub-traits

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/lib.rs` (modify — exports)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- All 16 test mock files (modify)

## Out of Scope

- Other domain sub-trait extractions
- SnapshotEntity sub-struct decomposition (ticket 007)
- GoalBeliefView changes (ticket 008)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `&dyn RuntimeBeliefView` usable at all existing call sites.
2. No behavioral change.

## Test Plan

### New/Modified Tests

1. None — pure structural refactor.

### Commands

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
