# S55: Causally Grounded Blocker Invalidation

## Summary

Replace pure TTL-based blocker expiry with condition-aware invalidation. Currently `BlockedIntentMemory` entries expire after a fixed tick count regardless of whether the blocking condition has changed. This spec adds invalidation conditions per `BlockingFact` variant so blockers clear when evidence of changed conditions arrives (restock observed, new path discovered, price change perceived), with TTL as a fallback for unresolvable blockers.

## Phase

Phase 6: Architectural Substrates II

## Status

Draft

## Crates

- `worldwake-core` (invalidation condition types on BlockedIntent, `sweep_cleared` method on BlockedIntentMemory)
- `worldwake-ai` (blocker evaluation reads beliefs for condition checks, clearing predicate logic)

## Dependencies

- S23 (refined blocked intents) — completed
- S31 (goal-aware exhaustion invalidation) — completed
- S54 (entity belief claims) — completed

## Design Goals

- Each `BlockingFact` variant declares what evidence would clear it
- Blocker evaluation checks clearing conditions against current belief state each tick
- TTL remains as a fallback safety net — if conditions never observably change, the blocker still expires
- No new systems needed — condition checking runs inside the existing AI decision pipeline
- Clearing logic lives in `worldwake-ai` (which has access to `GoalBeliefView`), not in `worldwake-core` (which cannot depend on `worldwake-sim`)

## Non-Goals

- Active investigation to clear blockers (agent planning to verify condition changes) — deferred
- Blocker negotiation (agent tries to change the condition, e.g., earn coin to clear TooExpensive) — already emergent through normal goal pursuit
- Unifying BlockingFact with FrameAssumption or ViolationKind — deferred

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P2 (No Ungrounded Triggers) | Blocker clearing is evidence-based, not arbitrary timer |
| P3 (Concrete State) | Clearing conditions reference concrete belief state, not abstract timers |
| P14 (World State Is Not Belief State) | Clearing is belief-mediated — agents observe changes through perception, not by reading authoritative state |
| P17 (Violated Expectation) | Blockers represent failed expectations; clearing represents updated evidence |
| P21 (Revisable Commitments) | Agents can resume blocked goals when conditions observably change |

## Deliverables

### New Types

```rust
/// In worldwake-core
pub enum BlockerClearingCondition {
    /// Belief about commodity availability at a place changed
    CommodityAvailabilityChanged {
        commodity: CommodityKind,
        place: EntityId,
    },
    /// Agent's inventory of a commodity changed (gained coin, acquired input)
    InventoryChanged {
        commodity: CommodityKind,
    },
    /// Agent acquired or lost a unique item relevant to the blocked action
    UniqueItemAcquired {
        kind: UniqueItemKind,
    },
    /// A new path to a destination was learned
    PathDiscovered {
        destination: EntityId,
    },
    /// Entity believed gone reappeared in beliefs
    EntityReappeared {
        entity: EntityId,
    },
    /// Danger level at a place decreased (threat left, combat resolved)
    DangerReduced {
        place: EntityId,
    },
    /// Contention state changed (queue position improved, grant expired, reservation freed)
    ContentionChanged {
        facility: EntityId,
    },
    /// No specific condition — TTL-only fallback
    TtlOnly,
}
```

### BlockedIntent Extension

```rust
pub struct BlockedIntent {
    pub blocker_key: BlockerKey,
    pub blocking_fact: BlockingFact,
    pub diagnostic_context: Option<BlockerDiagnostic>,
    pub observed_tick: Tick,
    pub expires_tick: Tick,  // TTL fallback (preserved)
    pub clearing_condition: BlockerClearingCondition,  // NEW
    pub baseline_snapshot: Option<ClearingBaseline>,   // NEW — snapshot at block time
}

pub enum ClearingBaseline {
    CommodityQuantity { quantity: Quantity },
    InventoryQuantity { quantity: Quantity },
    UniqueItemCount(u32),
    PathKnown(bool),
    EntityBelieved(bool),
    DangerLevel(Permille),
    ContentionPosition(Option<u32>),
}
```

Both `BlockerClearingCondition` and `ClearingBaseline` must derive `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize` to maintain `BlockedIntent`'s existing trait bounds.

### Clearing Condition Mapping

| BlockingFact | ClearingCondition | Baseline |
|---|---|---|
| `SellerOutOfStock` | `CommodityAvailabilityChanged` | Seller's believed quantity |
| `TooExpensive` | `InventoryChanged { Coin }` | Agent's coin balance |
| `MissingInput(kind)` | `InventoryChanged { kind }` | Agent's input quantity |
| `MissingTool(kind)` | `UniqueItemAcquired { kind }` | Agent's unique item count |
| `NoKnownSeller` | `EntityReappeared` or `CommodityAvailabilityChanged` | None |
| `NoKnownPath` | `PathDiscovered` | false |
| `TargetGone` | `EntityReappeared` | false |
| `DangerTooHigh` / `CombatTooRisky` | `DangerReduced` | Perceived danger level |
| `WorkstationBusy` / `ExclusiveFacilityUnavailable` / `ReservationConflict` | `ContentionChanged` | Queue position |
| `SourceDepleted` | `CommodityAvailabilityChanged` | Resource quantity |
| `NoBuyer` | `TtlOnly` | N/A |
| `Unknown` / `PatienceExhausted` / `AssumptionFailed` | `TtlOnly` | N/A |

### Integration Strategy

**Crate boundary constraint**: `BlockedIntentMemory` lives in `worldwake-core`, which cannot depend on `worldwake-sim` (where `GoalBeliefView` is defined). Therefore, condition evaluation logic must live in `worldwake-ai`.

**Approach**: Add a generic `sweep_cleared` method on `BlockedIntentMemory` in `worldwake-core` that accepts a predicate closure:

```rust
/// In worldwake-core, on BlockedIntentMemory
impl BlockedIntentMemory {
    /// Remove entries for which the predicate returns true (condition cleared).
    /// TTL expiry is handled separately by `expire()`.
    pub fn sweep_cleared(&mut self, mut is_cleared: impl FnMut(&BlockedIntent) -> bool) {
        self.intents.retain(|_, intent| !is_cleared(intent));
    }
}
```

The clearing predicate logic lives in `worldwake-ai`:

```rust
/// In worldwake-ai
fn is_blocker_cleared(
    blocker: &BlockedIntent,
    view: &dyn GoalBeliefView,
    agent: EntityId,
) -> bool {
    match (&blocker.clearing_condition, &blocker.baseline_snapshot) {
        (CommodityAvailabilityChanged { commodity, place }, Some(ClearingBaseline::CommodityQuantity { quantity: baseline })) => {
            let current = view.locally_observed_commodity_quantity(agent, *place, *commodity);
            current != *baseline  // Quantity changed — condition may have cleared
        }
        (InventoryChanged { commodity }, Some(ClearingBaseline::InventoryQuantity { quantity: baseline })) => {
            let current = view.commodity_quantity(agent, *commodity);
            current != *baseline
        }
        (UniqueItemAcquired { kind }, Some(ClearingBaseline::UniqueItemCount(baseline))) => {
            let current = view.unique_item_count(agent, *kind);
            current != *baseline
        }
        // ... other variants follow same pattern
        (TtlOnly, _) => false,  // Only TTL clears this
        (_, None) => false,      // Missing baseline — TTL fallback
    }
}
```

**Tick-cycle placement**: `sweep_cleared` is called in the agent tick pipeline in `worldwake-ai`, after perception/belief updates and before candidate generation. This ensures clearing evaluations use fresh beliefs. The existing `expire()` method continues to handle TTL garbage collection separately.

### Blocker Construction

When recording a new blocker in `failure_handling.rs`, the construction site must populate `clearing_condition` and `baseline_snapshot` based on the `BlockingFact` variant and current belief state. The mapping table above defines which condition and baseline to use for each variant.

## Cross-System Interactions

- **Perception** updates beliefs → changes baseline comparisons → clears blockers on next sweep
- **Tell system** shares beliefs → may update commodity/path knowledge → clears blockers on next sweep
- **AI pipeline** calls `sweep_cleared` with belief-checking predicate before candidate generation/search pruning
- **Event log** is not directly queried — clearing is belief-mediated, consistent with P14

## Profile-Driven Parameters

No new profiles. TTL values per `BlockingFact` are already configurable via `CognitiveProfile` (`transient_block_ticks`, `unknown_block_ticks`, `structural_block_ticks`).

## Component Registration

`BlockedIntentMemory` is an ECS component (implements `Component` in `blocked_intent.rs`). The new fields `clearing_condition` and `baseline_snapshot` on `BlockedIntent` will be serialized/deserialized through the existing component pipeline. Both new types must derive `Serialize, Deserialize` to maintain bincode round-trip compatibility.

No new components are introduced — the new types are fields on the existing `BlockedIntent` struct within `BlockedIntentMemory`.

## Section H — Causal Hooks

1. **Entities, relations, and records introduced** (P30.1): `BlockerClearingCondition` enum (8 variants including `UniqueItemAcquired`), `ClearingBaseline` enum (7 variants including `UniqueItemCount`). Both stored as fields on `BlockedIntent` within the existing `BlockedIntentMemory` component. No new components or relations.
2. **Information path** (P30.3): Clearing evidence arrives through perception (observation, tell, record consultation). Blocker checks compare current beliefs against block-time baseline. The `sweep_cleared` method evaluates conditions via `GoalBeliefView` at the start of each agent tick.
3. **Positive feedback** (P30.7): None. Clearing a blocker enables retrying a goal, which may succeed or re-block.
4. **Dampeners** (P30.8): TTL fallback prevents indefinite blocking. Re-blocking on retry prevents infinite loops.
5. **Stored vs derived** (P30.9): `clearing_condition` and `baseline_snapshot` are stored on `BlockedIntent`. Clearing evaluation is derived at query time from beliefs. The `sweep_cleared` call removes entries whose condition is met — the removal is a state mutation, but the decision to remove is derived.
6. **Partial failures** (P30.6): If a baseline was not captured at block time (`baseline_snapshot: None`), the blocker falls back to TTL-only evaluation. This is safe because TTL always clears eventually.
7. **Target patterns and invariants** (P30.13): Expected behaviors: (a) a blocker with `CommodityAvailabilityChanged` clears when the agent's belief about that commodity at the specified place changes from baseline; (b) a `TtlOnly` blocker clears only at TTL expiry; (c) a blocker with a missing baseline behaves as TtlOnly; (d) `sweep_cleared` never removes entries whose condition evaluates to not-cleared.
8. **Save/load/replay** (P30.14): New fields on `BlockedIntent` survive serialization via existing `Serialize/Deserialize` derives. `BlockerClearingCondition` and `ClearingBaseline` are deterministic value types (no closures, no runtime state). Replay produces identical clearing decisions given the same belief state.
