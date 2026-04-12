# S96OBLSAT-001: Core types and component registration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new ECS components `ObligationSatiationProfile` and `ObligationExecutionTracker`
**Deps**: S96 spec

## Problem

The obligation satiation mechanism requires two new ECS components before any downstream crate can implement satiation logic. This ticket introduces the foundational types.

## Assumption Reassessment (2026-04-12)

1. `component_schema.rs` uses the `with_component_schema_entries!` macro at line 2. The macro is expanded in 4 sites: `delta.rs`, `world_txn.rs`, `world.rs`, `component_tables.rs`. Each site imports component types via `use crate::{...}` blocks. New types must be added to those import blocks.
2. `Permille` is defined in `crates/worldwake-core/src/numerics.rs:24` as a `u16` newtype with `new_unchecked`, `value()` methods. `Tick` is defined in `crates/worldwake-core/src/ids.rs:57` as `pub struct Tick(pub u64)`.
3. Shared boundary: `component_schema.rs` macro — all component types must be registered here for ECS storage to recognize them.
4. `World::create_agent()` in `crates/worldwake-core/src/world.rs` seeds default values for other universal agent profiles (`PerceptionProfile`, `CognitiveProfile`, `ExplorationProfile`, `ExecutionBudget`, `IntentionDispositionProfile`, etc.). Because `ObligationSatiationProfile` is specified as a universal component, this ticket must extend that bootstrap path. `ObligationExecutionTracker` remains runtime-generated and should not be seeded there.
5. `crates/worldwake-core/src/world_txn.rs` has an exact `StateDelta` assertion for `WorldTxn::create_agent()`. Adding a seeded universal component changes that proof surface, so the delta test is part of the live owned fallout.

## Architecture Check

1. Two separate types (profile vs tracker) follow the existing pattern of separating configuration from runtime state (e.g., `ExplorationProfile` vs runtime counters elsewhere, `CombatProfile` vs `WoundList`). The profile is universal/scenario-configurable; the tracker is runtime-generated.
2. No backwards-compatibility shims. New types added cleanly alongside existing component registrations.
3. Universal component contract: `ObligationSatiationProfile` should be present on freshly created agents via the canonical bootstrap path, not only via later scenario wiring.

## Verification Layers

1. Component registration compiles → workspace build succeeds
2. `Default` impl produces expected values → focused unit test
3. Fresh `World::create_agent()` agents receive `ObligationSatiationProfile::default()` → focused registration/bootstrap proof
4. Single-layer ticket; no AI/planner behavior to verify yet.

## What to Change

### 1. Define `ObligationSatiationProfile` and `ObligationExecutionTracker`

Create a new module or add to an existing module in `worldwake-core`. Define both structs with derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. Add `Default` impls per spec D1 and D2.

### 2. Register in `component_schema.rs`

Add two entries to `with_component_schema_entries!`:
- `ObligationSatiationProfile` filtered to `EntityKind::Agent`
- `ObligationExecutionTracker` filtered to `EntityKind::Agent`

Follow the existing naming convention for field names, getters, setters, and query methods.

### 3. Add imports at macro expansion sites

Add `ObligationSatiationProfile` and `ObligationExecutionTracker` to the `use crate::{...}` import blocks in:
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/world_txn.rs`
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/component_tables.rs`

### 4. Export from `lib.rs`

Add both types to `crates/worldwake-core/src/lib.rs` public exports.

### 5. Seed universal default during agent creation

Update `crates/worldwake-core/src/world.rs::create_agent()` to insert
`ObligationSatiationProfile::default()` for newly created agents. Do not seed
`ObligationExecutionTracker`; it is runtime-generated.

### 6. Update create-agent delta proof

Extend the exact `StateDelta` assertion in `crates/worldwake-core/src/world_txn.rs`
so it reflects the new seeded `ObligationSatiationProfile` component.

## Files to Touch

- `crates/worldwake-core/src/obligation.rs` (new — or add to existing module)
- `crates/worldwake-core/src/component_schema.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify — imports)
- `crates/worldwake-core/src/world_txn.rs` (modify — imports)
- `crates/worldwake-core/src/world.rs` (modify — imports)
- `crates/worldwake-core/src/component_tables.rs` (modify — imports)
- `crates/worldwake-core/src/lib.rs` (modify — exports)
- `crates/worldwake-core/src/world.rs` (modify — `create_agent()` seeds universal default)
- `crates/worldwake-core/src/world_txn.rs` (modify — import fallout and `create_agent()` delta assertion)

## Out of Scope

- GoalBeliefView accessors (ticket 002)
- Scenario contract / AgentDef (ticket 003)
- Tracker update logic in commit handlers (ticket 004)
- Ranking integration (ticket 005)

## Acceptance Criteria

### Tests That Must Pass

1. `ObligationSatiationProfile::default()` produces `satiation_threshold: 2, window_ticks: 48, decay_per_execution: Permille(200), satiation_floor: Permille(50)`
2. `ObligationExecutionTracker::default()` produces empty `completion_ticks`
3. Fresh agents created through `World::create_agent()` have `ObligationSatiationProfile::default()`
4. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Both types are registered on `EntityKind::Agent` only
2. Both types derive `Serialize, Deserialize` for save/load compatibility
3. `ObligationSatiationProfile` has a `Default` impl and is seeded on freshly created agents
4. `ObligationExecutionTracker` remains absent until runtime logic creates it

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/obligation.rs` (inline `#[cfg(test)]`) — verify Default values for both types and bootstrap registration for `ObligationSatiationProfile`

### Commands

1. `cargo test -p worldwake-core`
2. `cargo build --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-12.

Outcome amended: 2026-04-12.

- Added `ObligationSatiationProfile` and `ObligationExecutionTracker` in new core module `crates/worldwake-core/src/obligation.rs` with serde-compatible derives and spec-matching `Default` behavior.
- Registered both components through `component_schema.rs`, updated the macro expansion import sites, and re-exported both types from `lib.rs`.
- Seeded `ObligationSatiationProfile::default()` in `World::create_agent()` to match the live universal-agent-profile bootstrap contract.
- Updated authoritative registration fallout in `delta.rs` and the exact `WorldTxn::create_agent()` delta assertion in `world_txn.rs`.
- Archived the completed ticket to `archive/tickets/S96OBLSAT-001.md` and updated active sibling ticket dependencies to reference the archived path.

## Deviations

- Reassessment widened the live owned surface beyond the original draft: because `ObligationSatiationProfile` is universal, this ticket also owned the canonical bootstrap seeding path in `crates/worldwake-core/src/world.rs` plus the matching `world_txn.rs` delta proof. `ObligationExecutionTracker` remained runtime-generated and was not seeded.

## Verification Result

- Passed `cargo test -p worldwake-core obligation::tests::obligation_satiation_profile_default_matches_spec_defaults`
- Passed `cargo test -p worldwake-core`
- Passed `cargo build --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Archival state: original active path removed; archived ticket is currently untracked at `archive/tickets/S96OBLSAT-001.md`
