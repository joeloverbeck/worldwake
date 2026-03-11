# E13DECARC-003: BlockedIntentMemory component and registration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new component in worldwake-core, schema registration
**Deps**: E13DECARC-001

## Problem

Agents need failure memory to avoid retrying the same blocked action every tick. `BlockedIntentMemory` records why an intent failed, when, and when the block expires. This is the concrete dampener for the AI replan loop (Principle 8).

## Assumption Reassessment (2026-03-11)

1. The original dependency on `GoalKey` from E13DECARC-004 is architecturally wrong as written. `BlockedIntentMemory` is authoritative `worldwake-core` state, and `worldwake-core` cannot depend on `worldwake-ai`. The persisted goal identity used by blocked-intent memory must therefore be owned in `worldwake-core`, not imported from `worldwake-ai`.
2. `CommodityKind`, `UniqueItemKind`, `EntityId`, `Tick`, and `RecipeId` all already exist in `worldwake-core` — confirmed.
3. Authoritative component registration is schema-driven through `with_component_schema_entries` in `component_schema.rs`, and that schema generates typed APIs in `ComponentTables`, `World`, `WorldTxn`, and `ComponentKind` / `ComponentValue` — confirmed.
4. Because of that schema-driven path, this ticket does not need bespoke storage boilerplate beyond the owning modules, schema entry, imports / re-exports, and explicit tests that enumerate authoritative component inventories.
5. Current coverage patterns for new authoritative components are broader than the original ticket described:
   - module-local trait / serialization / helper tests in the owning module
   - `component_tables.rs` CRUD coverage
   - `world.rs` roundtrip, query/count, and wrong-kind rejection coverage
   - `world_txn.rs` delta-recording coverage for generated setters / clearers
   - `delta.rs` authoritative component inventory coverage when `ComponentKind::ALL` changes
   - any integration tests that hard-code the authoritative component inventory (currently `crates/worldwake-systems/tests/e09_needs_integration.rs`)

## Architecture Check

1. `BlockedIntentMemory` is authoritative agent memory (not derived), so it belongs as a registered component.
2. `BlockingFact` uses concrete reasons, not abstract scores.
3. The normalized goal identity stored in that authoritative component must live in the shared lower layer. Duplicating it in both `worldwake-core` and `worldwake-ai`, or making core depend upward on AI, would create long-term drift and invalid crate boundaries.
4. TTLs are AI config constants (set in E13DECARC-009 budget ticket), not world-tuning variables.

## Scope Correction

This ticket should:

1. Add `BlockedIntentMemory`, `BlockedIntent`, and `BlockingFact` as new authoritative core types.
2. Introduce the minimal shared goal-identity schema that blocked-intent memory must persist in `worldwake-core`:
   - `CommodityPurpose`
   - `GoalKind`
   - `GoalKey`
3. Register `BlockedIntentMemory` through `component_schema.rs` as agent-only authoritative state.
4. Re-export the new shared types from `worldwake-core`.
5. Extend the explicit tests that must acknowledge a new authoritative component in `component_tables.rs`, `world.rs`, `world_txn.rs`, `delta.rs`, and `crates/worldwake-systems/tests/e09_needs_integration.rs`.

This ticket should not:

1. Implement candidate generation, ranking, planning, or failure-handling logic that writes blocked intents.
2. Add runtime-only decision state as authoritative components.
3. Introduce compatibility aliases between old/new goal-key locations.
4. Refactor unrelated component infrastructure.

## What to Change

### 1. Define shared goal identity in `worldwake-core`

Create `crates/worldwake-core/src/goal.rs`:

```rust
pub enum CommodityPurpose {
    SelfConsume,
    Restock,
    RecipeInput(RecipeId),
    Treatment,
}

pub enum GoalKind {
    ConsumeOwnedCommodity { commodity: CommodityKind },
    AcquireCommodity { commodity: CommodityKind, purpose: CommodityPurpose },
    Sleep,
    Relieve,
    Wash,
    ReduceDanger,
    Heal { target: EntityId },
    ProduceCommodity { recipe_id: RecipeId },
    SellCommodity { commodity: CommodityKind },
    RestockCommodity { commodity: CommodityKind },
    MoveCargo { lot: EntityId, destination: EntityId },
    LootCorpse { corpse: EntityId },
    BuryCorpse { corpse: EntityId, burial_site: EntityId },
}

pub struct GoalKey {
    pub kind: GoalKind,
    pub commodity: Option<CommodityKind>,
    pub entity: Option<EntityId>,
    pub place: Option<EntityId>,
}
```

Implement:

- `GoalKey::from(&GoalKind)` normalization
- `Clone`, `Debug`, `Eq`, `PartialEq`, `Ord`, `PartialOrd`, `Serialize`, `Deserialize`
- `Copy` only where lawful (`CommodityPurpose` may stay non-`Copy` if preferred, but `GoalPriorityClass` remains out of scope for this ticket)

This keeps the persisted goal identity in the shared foundational crate and lets later AI tickets consume the same canonical type instead of re-defining it.

### 2. Define `BlockedIntentMemory` in `worldwake-core`

Create `crates/worldwake-core/src/blocked_intent.rs`:

```rust
pub struct BlockedIntentMemory {
    pub intents: Vec<BlockedIntent>,
}

pub struct BlockedIntent {
    pub goal_key: GoalKey,
    pub blocking_fact: BlockingFact,
    pub related_entity: Option<EntityId>,
    pub related_place: Option<EntityId>,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
}

pub enum BlockingFact {
    NoKnownPath,
    NoKnownSeller,
    SellerOutOfStock,
    TooExpensive,
    SourceDepleted,
    WorkstationBusy,
    ReservationConflict,
    MissingTool(UniqueItemKind),
    MissingInput(CommodityKind),
    TargetGone,
    DangerTooHigh,
    CombatTooRisky,
    Unknown,
}
```

Implement `Component`, `Default` (empty intents), `Clone`, `Debug`, `Eq`, `PartialEq`, `Serialize`, `Deserialize`.

### 3. Add helper methods

- `BlockedIntentMemory::is_blocked(&self, key: &GoalKey, current_tick: Tick) -> bool`
- `BlockedIntentMemory::record(&mut self, intent: BlockedIntent)`
- `BlockedIntentMemory::expire(&mut self, current_tick: Tick)` — removes expired entries
- `BlockedIntentMemory::clear_for(&mut self, key: &GoalKey)` — early clear when blocker resolved

### 4. Register in component schema

Add `BlockedIntentMemory` to the `with_component_schema_entries` macro. Entity-kind guard: `Agent` only.

### 5. Export from `worldwake-core/src/lib.rs`

Add modules and re-exports for the new blocked-intent and shared goal-identity types.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (new)
- `crates/worldwake-core/src/blocked_intent.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-export)
- `crates/worldwake-core/src/component_schema.rs` (modify — add entry)
- `crates/worldwake-core/src/component_tables.rs` (modify — schema-generated storage/tests)
- `crates/worldwake-core/src/world.rs` (modify — generated component API tests)
- `crates/worldwake-core/src/world_txn.rs` (modify — generated setter/clearer delta tests)
- `crates/worldwake-core/src/delta.rs` (modify — authoritative component inventory tests)
- `crates/worldwake-core/src/test_utils.rs` (only if shared fixtures reduce duplication)
- `crates/worldwake-systems/tests/e09_needs_integration.rs` (modify only if authoritative component inventory assertions require it)

## Out of Scope

- `GoalPriorityClass` / `GroundedGoal` read-model types — follow-up AI ticket
- Failure-handling logic that writes `BlockedIntent` — E13DECARC-013
- TTL constant values — E13DECARC-009 (budget)
- Blocker-resolution clearing logic — E13DECARC-013

## Acceptance Criteria

### Tests That Must Pass

1. `BlockedIntentMemory` implements `Component` (trait bound test)
2. `BlockedIntentMemory` round-trips through bincode
3. `GoalKey::from(&GoalKind::AcquireCommodity { .. })` extracts the canonical commodity correctly
4. `GoalKey::from(&GoalKind::LootCorpse { .. })` extracts the canonical entity correctly
5. `GoalKind` / `GoalKey` round-trip through bincode
6. `ComponentTables` supports insert/get/remove/has for `BlockedIntentMemory`
7. `World` accepts `BlockedIntentMemory` on `EntityKind::Agent`, exposes generated query/count APIs, and rejects insertion on non-agent kinds
8. `WorldTxn` setter / clearer coverage records the expected `ComponentDelta`
9. `delta.rs` authoritative component inventories remain complete after adding `BlockedIntentMemory`
10. `is_blocked()` returns true for non-expired matching goal key
11. `is_blocked()` returns false after expiry tick
12. `expire()` removes entries whose `expires_tick <= current_tick`
13. `clear_for()` removes all entries matching a given `GoalKey`
14. Existing suite: `cargo test --workspace`

### Invariants

1. `BlockedIntentMemory` is only registerable on `EntityKind::Agent`
2. No abstract fear/greed scores stored
3. `BlockingFact` enumerates concrete, inspectable reasons
4. Shared persisted goal identity lives in `worldwake-core`, not in an AI-only crate
5. No compatibility aliases between duplicate goal-key types

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` — normalization, ordering, trait-bound, and bincode tests for shared goal identity
2. `crates/worldwake-core/src/blocked_intent.rs` — module-level tests for helpers, trait bounds, and serialization
3. `crates/worldwake-core/src/component_tables.rs` — CRUD coverage for `BlockedIntentMemory`
4. `crates/worldwake-core/src/world.rs` — agent roundtrip, query/count, and wrong-kind rejection for `BlockedIntentMemory`
5. `crates/worldwake-core/src/world_txn.rs` — setter / clearer delta coverage for `BlockedIntentMemory`
6. `crates/worldwake-core/src/delta.rs` — authoritative component inventory coverage updated for `BlockedIntentMemory`
7. `crates/worldwake-systems/tests/e09_needs_integration.rs` — only if that test hard-codes the authoritative component inventory

### Commands

1. `cargo test -p worldwake-core blocked_intent`
2. `cargo test -p worldwake-core goal`
3. `cargo test -p worldwake-core`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - Added `crates/worldwake-core/src/blocked_intent.rs` with `BlockedIntentMemory`, `BlockedIntent`, `BlockingFact`, helper methods, and focused module-local tests.
  - Added `crates/worldwake-core/src/goal.rs` with the shared canonical goal-identity types `CommodityPurpose`, `GoalKind`, and `GoalKey`, including normalization and serialization tests.
  - Registered `BlockedIntentMemory` as an agent-only authoritative component through `component_schema.rs`, and re-exported the new blocked-intent and goal types from `worldwake-core`.
  - Extended `component_tables.rs`, `world.rs`, `world_txn.rs`, and `delta.rs` so the new component participates in generated storage/APIs, delta recording, and authoritative component inventories.
  - Added shared deterministic fixtures in `test_utils.rs`.
  - Updated `crates/worldwake-systems/tests/e09_needs_integration.rs` because that integration test hard-codes the authoritative component inventory.
  - Updated `tickets/E13DECARC-004-goal-model-types.md` so the related ticket no longer contradicts the corrected shared-type ownership.
- Deviations from original plan:
  - The original ticket was wrong to depend on an AI-owned `GoalKey`; the clean architecture required moving the shared goal identity into `worldwake-core`.
  - The real implementation surface was wider than the original ticket claimed because schema-driven authoritative components also require explicit `world.rs`, `world_txn.rs`, `delta.rs`, and downstream inventory-test updates.
  - `record()` was implemented to replace any existing entry for the same `GoalKey`, which keeps blocked-intent memory canonical and avoids duplicate stale entries for one intent.
- Verification results:
  - `cargo test -p worldwake-core blocked_intent` passed.
  - `cargo test -p worldwake-core goal` passed.
  - `cargo test -p worldwake-core` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
