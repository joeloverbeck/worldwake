# E13DECARC-003: BlockedIntentMemory component and registration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new component in worldwake-core, schema registration
**Deps**: E13DECARC-001, E13DECARC-004 (needs GoalKey)

## Problem

Agents need failure memory to avoid retrying the same blocked action every tick. `BlockedIntentMemory` records why an intent failed, when, and when the block expires. This is the concrete dampener for the AI replan loop (Principle 8).

## Assumption Reassessment (2026-03-11)

1. `GoalKey` will be defined in E13DECARC-004 — this ticket depends on it.
2. `CommodityKind`, `UniqueItemKind`, `EntityId`, `Tick` all exist in `worldwake-core` — confirmed.
3. Component registration macro pattern is established — confirmed.

## Architecture Check

1. `BlockedIntentMemory` is authoritative agent memory (not derived), so it belongs as a registered component.
2. `BlockingFact` uses concrete reasons, not abstract scores.
3. TTLs are AI config constants (set in E13DECARC-009 budget ticket), not world-tuning variables.

## What to Change

### 1. Define `BlockedIntentMemory` in `worldwake-core`

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

### 2. Add helper methods

- `BlockedIntentMemory::is_blocked(&self, key: &GoalKey, current_tick: Tick) -> bool`
- `BlockedIntentMemory::record(&mut self, intent: BlockedIntent)`
- `BlockedIntentMemory::expire(&mut self, current_tick: Tick)` — removes expired entries
- `BlockedIntentMemory::clear_for(&mut self, key: &GoalKey)` — early clear when blocker resolved

### 3. Register in component schema

Add `BlockedIntentMemory` to the `with_component_schema_entries` macro. Entity-kind guard: `Agent` only.

### 4. Export from `worldwake-core/src/lib.rs`

Add module and re-export.

## Files to Touch

- `crates/worldwake-core/src/blocked_intent.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-export)
- `crates/worldwake-core/src/component_schema.rs` (modify — add entry)
- `crates/worldwake-core/src/component_tables.rs` (modify — add storage field)

## Out of Scope

- `GoalKey` / `GoalKind` definitions — E13DECARC-004
- Failure-handling logic that writes `BlockedIntent` — E13DECARC-013
- TTL constant values — E13DECARC-009 (budget)
- Blocker-resolution clearing logic — E13DECARC-013

## Acceptance Criteria

### Tests That Must Pass

1. `BlockedIntentMemory` implements `Component` (trait bound test)
2. `BlockedIntentMemory` round-trips through bincode
3. Can be inserted on an `Agent` entity; rejected on non-Agent
4. `is_blocked()` returns true for non-expired matching goal key
5. `is_blocked()` returns false after expiry tick
6. `expire()` removes entries whose `expires_tick <= current_tick`
7. `clear_for()` removes all entries matching a given `GoalKey`
8. Existing suite: `cargo test --workspace`

### Invariants

1. `BlockedIntentMemory` is only registerable on `EntityKind::Agent`
2. No abstract fear/greed scores stored
3. `BlockingFact` enumerates concrete, inspectable reasons

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/blocked_intent.rs` — module-level tests for all helpers and bounds

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
