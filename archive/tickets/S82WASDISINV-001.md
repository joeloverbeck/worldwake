# S82WASDISINV-001: Add GoalKind::FreeCarryCapacity and DisposalProfile component

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new GoalKind variant, new ECS component
**Deps**: None

## Problem

Agents have no mechanism to shed unwanted inventory. This ticket adds the foundational types: a `FreeCarryCapacity` goal kind and a `DisposalProfile` component that parameterizes when agents consider disposal.

## Assumption Reassessment (2026-04-10)

1. `GoalKind` enum exists at `crates/worldwake-core/src/goal.rs:18` with 33 variants. `FreeCarryCapacity` does not exist yet. Confirmed via grep.
2. `DisposalProfile` does not exist anywhere in the codebase. Confirmed via grep returning 0 matches.
3. Shared boundary: `component_schema.rs` macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`, `world_txn.rs`) — all 4 must import the new type.
4. `Component` trait at `traits.rs:14` requires `'static + Send + Sync + Clone + Debug + Serialize + DeserializeOwned`. `Permille` derives `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize` — satisfies all field-level bounds.
5. `EntityKind::Agent` exists at `entity.rs:8`. Universal profiles use `|kind| kind == EntityKind::Agent` filter predicate in `component_schema.rs`.
6. Ticket said schema registration was sufficient for universal-agent availability. Live code also default-seeds universal agent profiles in `crates/worldwake-core/src/world.rs:create_agent()`. Correction applied: this ticket must seed `DisposalProfile::default()` there as part of the universal-profile contract.
7. Ticket said exhaustive fallout was limited to `GoalKind` matches in `worldwake-core`. Live code has no other `GoalKind` exhaustive matches in this crate beyond `GoalKey::from`, but the component addition does require updating hand-maintained component inventories and create-agent delta expectations in `crates/worldwake-core/src/delta.rs` and `crates/worldwake-core/src/world_txn.rs`. Correction applied: added those live fallout surfaces to scope.

## Architecture Check

1. Standard pattern: add variant to GoalKind, register component on EntityKind::Agent with Default impl. Follows existing patterns for `CombatProfile`, `ExplorationProfile`, etc.
2. No backward-compatibility shims. New variant and component only.

## Verification Layers

1. GoalKind::FreeCarryCapacity compiles and is matchable -> focused unit test constructing the variant
2. DisposalProfile registered on EntityKind::Agent -> unit test: `world.set_component_disposal_profile(agent, DisposalProfile::default())` succeeds
3. Single-layer ticket (core types only) — no cross-system verification needed

## What to Change

### 1. GoalKind variant

In `crates/worldwake-core/src/goal.rs`, add to the `GoalKind` enum:

```rust
/// Agent wants to free carry capacity by dropping low-value items.
FreeCarryCapacity,
```

### 2. DisposalProfile struct

Create or add to an appropriate module in `crates/worldwake-core/src/`:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DisposalProfile {
    pub capacity_strain_threshold: Permille,
}

impl Default for DisposalProfile {
    fn default() -> Self {
        Self {
            capacity_strain_threshold: Permille::new_unchecked(800),
        }
    }
}

impl Component for DisposalProfile {}
```

### 3. Component registration

In `crates/worldwake-core/src/component_schema.rs`, register `DisposalProfile` on `EntityKind::Agent` (universal, with Default), following the existing 20-parameter macro pattern used by `CombatProfile`, `MetabolismProfile`, etc.

In `crates/worldwake-core/src/world.rs`, seed `DisposalProfile::default()` inside `World::create_agent()` so newly created agents receive the universal profile by default.

### 4. Macro expansion imports

Add `use crate::DisposalProfile;` (or equivalent re-export) at all 4 macro expansion sites:
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/component_tables.rs`
- `crates/worldwake-core/src/world_txn.rs`

### 5. Exhaustive match sites

Add `GoalKind::FreeCarryCapacity` arms to exhaustive `GoalKind` consumers. In `worldwake-core`, update `GoalKey::from`. In downstream crates, add explicit compile-safe inert handling where the new shared variant participates in dispatch or formatting, without making the disposal goal behavior live ahead of tickets 004-007.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify)
- `crates/worldwake-core/src/production.rs` or new file (modify/new — for DisposalProfile)
- `crates/worldwake-core/src/component_schema.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify — import, universal default seeding, focused tests)
- `crates/worldwake-core/src/delta.rs` (modify — import, manual component inventory/sample tests)
- `crates/worldwake-core/src/component_tables.rs` (modify — import)
- `crates/worldwake-core/src/world_txn.rs` (modify — import, create-agent delta expectation, set/clear tests)
- `crates/worldwake-core/src/lib.rs` (modify — re-export)
- `crates/worldwake-ai/src/goal_dispatch_key.rs` (modify — inert dispatch key coverage for new `GoalKind`)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — inert declaration for compile-safe downstream wiring)
- `crates/worldwake-ai/src/goal_model.rs` (modify — inert exhaustive handling, no live planner behavior)
- `crates/worldwake-ai/src/ranking.rs` (modify — inert ranking coverage with zero motive)
- `crates/worldwake-cli/src/display.rs` (modify — format branch for new `GoalKind`)

## Out of Scope

- Action definition and handler (ticket 002)
- Planner integration (tickets 004-006)
- Candidate generation (ticket 007)
- CLI/scenario integration (ticket 007)
- Golden tests (ticket 008)

## Acceptance Criteria

### Tests That Must Pass

1. Construct `GoalKind::FreeCarryCapacity` and verify it compiles
2. Create `DisposalProfile::default()` and verify `capacity_strain_threshold` is `Permille(800)`
3. Set and get `DisposalProfile` on an agent entity via world API
4. `World::create_agent()` seeds `DisposalProfile::default()` on newly created agents
5. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `GoalKind` enum remains `Copy + Clone + Eq + PartialEq + Ord + PartialOrd + Hash + Debug + Serialize + Deserialize`
2. `DisposalProfile` satisfies the `Component` trait bounds
3. Downstream `GoalKind` consumers compile with explicit inert handling; `FreeCarryCapacity` is not behaviorally live before tickets 004-007
4. `cargo clippy --workspace --all-targets -- -D warnings` passes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` (or test module) — verify FreeCarryCapacity variant round-trips through serde
2. `crates/worldwake-core/src/disposal.rs` (new test module) — verify default, serde roundtrip, and agent registration behavior
3. `crates/worldwake-core/src/world_txn.rs` — verify set/clear delta behavior and create-agent delta includes default DisposalProfile

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-10.

- Added `GoalKind::FreeCarryCapacity` to [`crates/worldwake-core/src/goal.rs`] and wired its canonical `GoalKey` behavior plus serde-focused proof.
- Added new authoritative `DisposalProfile` in [`crates/worldwake-core/src/disposal.rs`], re-exported it from [`crates/worldwake-core/src/lib.rs`], and registered it through the core component schema and typed storage surfaces.
- Seeded `DisposalProfile::default()` in [`crates/worldwake-core/src/world.rs:create_agent()`] so newly created agents receive the universal profile by default.
- Updated component inventory/sample and transaction-delta proof surfaces in [`crates/worldwake-core/src/delta.rs`] and [`crates/worldwake-core/src/world_txn.rs`].
- Added explicit inert downstream handling in `worldwake-ai` and `worldwake-cli` so the new shared `GoalKind` compiles cleanly without making disposal behavior live before the follow-up tickets.
- Active ticket state remains untracked (`git status --short -- tickets/S82WASDISINV-001.md` reported `??` before implementation).

## Deviations

- Reassessment corrected the ticket’s original assumption that schema registration alone satisfied the universal-agent profile contract; live code also required default seeding in `World::create_agent()`.
- Shared-enum compile fallout extended beyond `worldwake-core`. The implementation absorbed bounded inert branches in `worldwake-ai` and `worldwake-cli` so workspace compile and CI-matching clippy remained green while keeping disposal behavior out of scope.

## Verification Result

- Passed `cargo test -p worldwake-core goal::tests::free_carry_capacity_goal_roundtrips_through_bincode`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test --workspace --no-run`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
