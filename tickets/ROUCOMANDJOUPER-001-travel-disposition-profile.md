# ROUCOMANDJOUPER-001: TravelDispositionProfile Component

**Status**: PENDING
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

## Architecture Check

1. Following the existing pattern of per-agent profile components (`CombatProfile`, `MetabolismProfile`, `TradeDispositionProfile`, `UtilityProfile`). This is the established approach for agent-diverse parameters.
2. No backwards-compatibility aliasing or shims. Pure additive change.

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
- `crates/worldwake-core/src/travel_disposition.rs` (new — struct definition and Component impl) OR add to an existing appropriate module
- `crates/worldwake-core/src/component_schema.rs` (modify — add schema entry)

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
3. `TravelDispositionProfile` insertion is rejected for non-Agent entity kinds (if the schema enforces this).
4. `TravelDispositionProfile` serializes and deserializes correctly via bincode — round-trip test matching the pattern in `component_tables.rs` tests.
5. Existing suite: `cargo test -p worldwake-core`
6. Existing suite: `cargo clippy --workspace`

### Invariants

1. `TravelDispositionProfile` is an authoritative stored component, not transient runtime state.
2. No new cross-crate dependencies introduced.
3. `AgentDecisionRuntime` is NOT affected by this ticket (confirmed by separate test that it is not in component schema).
4. The `Permille` and `NonZeroU32` types enforce correct value domains at the type level — no runtime validation needed.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/travel_disposition.rs` (or equivalent) — unit test: struct construction, field access, equality
2. `crates/worldwake-core/src/component_tables.rs` tests section — integration test: insert/get/remove round-trip for `TravelDispositionProfile`
3. `crates/worldwake-core/src/component_tables.rs` tests section — serialization round-trip test

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo build --workspace`
