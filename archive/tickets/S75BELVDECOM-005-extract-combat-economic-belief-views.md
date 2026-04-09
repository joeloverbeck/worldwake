# S75BELVDECOM-005: Extract CombatBeliefView + EconomicBeliefView sub-traits

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — RuntimeBeliefView trait decomposition
**Deps**: S75BELVDECOM-001

## Problem

Extract CombatBeliefView (10 methods: combat profile, wounds, hostiles, courage, patrol, pursuit) and EconomicBeliefView (9 methods: trade, sale, demand, merchandise, valuation) from RuntimeBeliefView.

## Assumption Reassessment (2026-04-08)

1. CombatBeliefView methods confirmed (10): `combat_profile`, `courage`, `consultation_speed_factor`, `wounds`, `hostile_targets_of`, `visible_hostiles_for`, `current_attackers_of`, `patrol_profile`, `pursuit_profile`, `has_wounds`.
2. EconomicBeliefView methods confirmed (9): `trade_disposition_profile`, `commodity_valuation_profile`, `controlled_commodity_quantity_at_place`, `local_controlled_lots_for`, `listed_sale_lots_at`, `seller_for_sale_lot`, `has_sale_listing`, `demand_memory`, `merchandise_profile`.
3. The production split remained owned by `PerAgentBeliefView` and `PlanningState`, but the actual fallout surface also included goal-belief macro delegation, broad AI/sim/systems test stubs, and golden trade/merchant-selling helpers. Updating those callers was required to keep the trait move honest and compile/test all targets.

## Architecture Check

1. Same supertrait pattern. No backward-compatibility shims.
2. `patrol_profile` is categorized under Combat (not Profile) per the spec's note that categorization is finalized during ticket decomposition. The rationale: `patrol_profile` is consumed alongside `pursuit_profile` and `combat_profile` in combat/patrol planning contexts, not alongside needs/metabolism.
3. `impl_goal_belief_view!` delegation was updated only as needed to follow the new ownership split; broader macro cleanup remains owned by `S75BELVDECOM-008`.

## Verification Layers

1. Combat queries -> golden tests exercise wound, hostile, and combat profile queries via combat scenarios
2. Economic queries -> golden tests exercise trade/sale queries via merchant and trade scenarios
3. Compile-time proof -> `cargo build --workspace`
4. Full target fallout proof -> `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`

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
- `crates/worldwake-sim/src/affordance_query.rs` (modify — test stub fallout)
- `crates/worldwake-sim/src/trade_valuation.rs` (modify — test stub fallout)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — test stub fallout)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — test stub fallout)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — test stub fallout)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — test stub fallout)
- `crates/worldwake-ai/src/enterprise.rs` (modify — test stub fallout)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — test stub fallout)
- `crates/worldwake-ai/src/goal_explanation.rs` (modify — test stub fallout)
- `crates/worldwake-ai/src/goal_model.rs` (modify — test stub fallout)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — test stub fallout)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — test stub fallout)
- `crates/worldwake-ai/src/pressure.rs` (modify — test stub fallout)
- `crates/worldwake-ai/src/ranking.rs` (modify — test stub fallout)
- `crates/worldwake-ai/src/search/tests.rs` (modify — test stub fallout)
- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify — trait import fallout)
- `crates/worldwake-ai/tests/golden_trade.rs` (modify — UFCS fallout)
- `crates/worldwake-systems/src/tell_actions.rs` (modify — test stub fallout)

## Out of Scope

- Other domain sub-trait extractions
- SnapshotEntity sub-struct decomposition (ticket 007)
- Broader GoalBeliefView macro cleanup beyond the delegation needed for this ownership move (ticket 008)

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

## Outcome

Completed: 2026-04-09.

`CombatBeliefView` and `EconomicBeliefView` now own the extracted methods in `belief_view.rs`, `RuntimeBeliefView` composes them, production impl ownership is split in `per_agent_belief_view.rs` and `planning_state.rs`, and the required goal-belief delegation, AI/sim/systems mock fallout, and golden merchant/trade call-site fallout were migrated to the new trait boundary.

Deviation from original plan: the ticket's initial "same 18 impl blocks" assumption was too narrow. The real owned fallout also included golden helper imports/call sites and a broader set of test stubs than the original ticket listed, but no additional production architecture beyond the combat/economic trait split was needed.

## Verification Results

1. `cargo fmt --all`
2. `cargo build --workspace`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

### Commands

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
