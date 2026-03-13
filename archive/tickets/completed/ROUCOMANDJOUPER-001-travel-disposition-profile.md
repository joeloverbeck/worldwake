# ROUCOMANDJOUPER-001: TravelDispositionProfile Component

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new component type in worldwake-core
**Deps**: None

## Problem

The decision architecture has no per-agent parameter for route persistence behavior. The global `PlanningBudget::switch_margin_permille` applies uniformly to all goal switches regardless of whether the agent is mid-journey. To support agent diversity in travel commitment, a dedicated profile component is needed.

## Assumption Reassessment (2026-03-13)

1. `component_schema.rs` uses the `with_component_schema_entries!` macro pattern with per-component entries including field name, type, accessors, entity kind predicate, and transaction methods — confirmed by reading the file.
2. All agent-only components use the predicate `|kind| kind == EntityKind::Agent` — confirmed.
3. `Permille` is defined in `worldwake-core::numerics` and is already used in `PlanningBudget::switch_margin_permille` — confirmed.
4. `NonZeroU32` from `std::num` is already used in the codebase (e.g., travel edge ticks) — confirmed.
5. The `Component` trait is defined in `worldwake-core::traits` — confirmed.
6. In this repo, adding an authoritative component is not limited to `lib.rs` + `component_schema.rs`: the new type must also be imported into macro-driven component storage and world API surfaces (`component_tables.rs`, `world.rs`, `world_txn.rs`, `delta.rs`) and covered by their regression tests — confirmed by comparing the existing `TradeDispositionProfile` path end to end.
7. The current codebase does not already have a travel-domain profile component or a `BeliefView` accessor for it — confirmed. A belief-view accessor is still unnecessary for this ticket because no runtime or planner code reads the component yet.

## Architecture Check

1. Following the existing pattern of per-agent profile components (`CombatProfile`, `MetabolismProfile`, `TradeDispositionProfile`, `UtilityProfile`). This is the established approach for agent-diverse parameters.
2. No backwards-compatibility aliasing or shims. Pure additive change.
3. `TravelDispositionProfile` should live in its own travel-focused core module rather than inside `trade.rs`. Route commitment is a travel/runtime concern; keeping it separate avoids cross-domain leakage and makes later AI integration cleaner.

## What to Change

### 1. Define `TravelDispositionProfile` struct

In a new or existing module in `worldwake-core`, define:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TravelDispositionProfile {
    pub route_replan_margin: Permille,
    pub blocked_leg_patience_ticks: NonZeroU32,
}

impl Component for TravelDispositionProfile {}
```

- `route_replan_margin`: during an active journey, this value replaces `budget.switch_margin_permille` in `compare_goal_switch()` — how much better a challenger goal must be before the agent abandons a journey.
- `blocked_leg_patience_ticks`: how long the agent tolerates repeated next-leg failure before dropping commitment.

Both values are per-agent, seeded at creation time to preserve agent diversity.

### 2. Register in component schema

Add entry to `component_schema.rs` following the existing pattern (e.g., `CombatProfile` or `TradeDispositionProfile`):

- Field name: `travel_disposition_profiles`
- Type: `TravelDispositionProfile`
- Entity kind predicate: `|kind| kind == EntityKind::Agent`
- Full set of accessor methods: `insert_travel_disposition_profile`, `get_travel_disposition_profile`, etc.
- Transaction methods: `set_component_travel_disposition_profile`, `clear_component_travel_disposition_profile`

### 3. Re-export from `worldwake-core` lib

Ensure `TravelDispositionProfile` is publicly exported from the crate root, following the pattern of other profile types.

## Files to Touch

- `crates/worldwake-core/src/lib.rs` (modify — add module declaration and re-export)
- `crates/worldwake-core/src/travel_disposition.rs` (new — struct definition, `Component` impl, local round-trip tests)
- `crates/worldwake-core/src/component_schema.rs` (modify — add schema entry)
- `crates/worldwake-core/src/component_tables.rs` (modify — import new type and extend component-table regression tests)
- `crates/worldwake-core/src/world.rs` (modify — import new type; add world-level round-trip and non-agent rejection coverage)
- `crates/worldwake-core/src/world_txn.rs` (modify — import new type; add transaction delta coverage)
- `crates/worldwake-core/src/delta.rs` (modify — import new type; extend `ComponentKind` / `ComponentValue` regression coverage)
- `crates/worldwake-core/src/test_utils.rs` (modify — add deterministic fixture for the new component)

## Out of Scope

- Journey temporal fields on `AgentDecisionRuntime` (ticket 002)
- Goal switching margin override logic (ticket 003)
- Any changes to `worldwake-ai`, `worldwake-sim`, or `worldwake-systems`
- Default/factory helpers for creating agents with this profile (can be added when needed)
- BeliefView accessor for this profile (will be added in ticket 003 if needed)
- Changes to `build_prototype_world` or agent creation helpers

## Acceptance Criteria

### Tests That Must Pass

1. `TravelDispositionProfile` implements `Component` trait — confirmed by compilation.
2. `TravelDispositionProfile` can be inserted, retrieved, and removed from `ComponentTables` for an Agent entity — round-trip test.
3. `World` rejects `TravelDispositionProfile` insertion on non-Agent entity kinds — this is where entity-kind enforcement actually happens in the current architecture.
4. `TravelDispositionProfile` serializes and deserializes correctly via bincode — round-trip test matching existing profile-component module patterns.
5. `WorldTxn::set_component_travel_disposition_profile` records the correct `ComponentDelta` and commits the updated component value.
6. `delta.rs` regression coverage includes the new `ComponentKind` / `ComponentValue` variant so event-log typing stays exhaustive.
5. Existing suite: `cargo test -p worldwake-core`
6. Existing suite: `cargo clippy --workspace`

### Invariants

1. `TravelDispositionProfile` is an authoritative stored component, not transient runtime state.
2. No new cross-crate dependencies introduced.
3. `AgentDecisionRuntime` is NOT affected by this ticket. No AI runtime field, planner, or belief-view changes belong here.
4. The `Permille` and `NonZeroU32` types enforce correct value domains at the type level — no runtime validation needed.
5. No extension of unrelated profile components (`TradeDispositionProfile`, `UtilityProfile`) is allowed; travel commitment remains isolated in its own domain type.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/travel_disposition.rs` — unit tests: trait bounds and bincode round-trip
2. `crates/worldwake-core/src/component_tables.rs` — integration test: insert/get/remove/has cycle for `TravelDispositionProfile`
3. `crates/worldwake-core/src/world.rs` — world-level round-trip test on an agent
4. `crates/worldwake-core/src/world.rs` — non-agent rejection test
5. `crates/worldwake-core/src/world_txn.rs` — transaction delta/commit test
6. `crates/worldwake-core/src/delta.rs` — component enum coverage updated to include the new variant

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo build --workspace`

## Outcome

- Completed: 2026-03-13
- What actually changed:
  - Added a dedicated `TravelDispositionProfile` authoritative component in a new `worldwake-core::travel_disposition` module.
  - Registered the component through the macro-driven authoritative schema so it now participates in `ComponentTables`, `World`, `WorldTxn`, and typed event-log delta surfaces.
  - Added deterministic test fixtures and regression coverage for core round-trip, non-agent rejection, delta typing, and transaction commit behavior.
  - Updated the downstream E09 schema-contract integration test to include the new authoritative component kind.
- Deviations from original plan:
  - The ticket originally understated scope as only `lib.rs` + `component_schema.rs` + one component file. Actual implementation required the full authoritative-component plumbing used by this repo, plus downstream schema-contract test maintenance.
  - The component was implemented in a dedicated `travel_disposition.rs` module rather than being folded into an existing module. This keeps travel commitment concerns isolated from trade-domain profiles.
- Verification results:
  - `cargo test -p worldwake-core` ✅
  - `cargo test -p worldwake-systems --test e09_needs_integration` ✅
  - `cargo clippy --workspace` ✅
  - `cargo build --workspace` ✅
