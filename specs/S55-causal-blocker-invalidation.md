# S55: Causally Grounded Blocker Invalidation

## Summary

Replace pure TTL-based blocker expiry with condition-aware invalidation. Currently `BlockedIntentMemory` entries expire after a fixed tick count regardless of whether the blocking condition has changed. This spec adds invalidation conditions per `BlockingFact` variant so blockers clear when evidence of changed conditions arrives (restock observed, new path discovered, price change perceived), with TTL as a fallback for unresolvable blockers.

## Phase

Phase 6: Architectural Substrates II

## Status

Draft

## Crates

- `worldwake-core` (invalidation condition types on BlockedIntent)
- `worldwake-ai` (blocker evaluation reads beliefs for condition checks)

## Dependencies

- S23 (refined blocked intents) — completed
- S31 (goal-aware exhaustion invalidation) — completed
- S54 (entity belief claims) — beneficial but not blocking

## Design Goals

- Each `BlockingFact` variant declares what evidence would clear it
- Blocker evaluation checks clearing conditions against current belief state each tick
- TTL remains as a fallback safety net — if conditions never observably change, the blocker still expires
- No new systems needed — condition checking runs inside the existing AI decision pipeline

## Non-Goals

- Active investigation to clear blockers (agent planning to verify condition changes) — deferred
- Blocker negotiation (agent tries to change the condition, e.g., earn coin to clear TooExpensive) — already emergent through normal goal pursuit
- Unifying BlockingFact with FrameAssumption or ViolationKind — deferred

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P2 (No Ungrounded Triggers) | Blocker clearing is evidence-based, not arbitrary timer |
| P3 (Concrete State) | Clearing conditions reference concrete belief state, not abstract timers |
| P17 (Violated Expectation) | Blockers represent failed expectations; clearing represents updated evidence |
| P21 (Revisable Commitments) | Agents can resume blocked goals when conditions observably change |

## Deliverables

### New Types

```rust
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
    /// Contention state changed (queue position improved, grant expired)
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
    PathKnown(bool),
    EntityBelieved(bool),
    DangerLevel(Permille),
    ContentionPosition(Option<u32>),
}
```

### Clearing Condition Mapping

| BlockingFact | ClearingCondition | Baseline |
|---|---|---|
| `SellerOutOfStock` | `CommodityAvailabilityChanged` | Seller's believed quantity |
| `TooExpensive` | `InventoryChanged { Coin }` | Agent's coin balance |
| `MissingInput(kind)` | `InventoryChanged { kind }` | Agent's input quantity |
| `NoKnownSeller` | `EntityReappeared` or `CommodityAvailabilityChanged` | None |
| `NoKnownPath` | `PathDiscovered` | false |
| `TargetGone` | `EntityReappeared` | false |
| `DangerTooHigh` / `CombatTooRisky` | `DangerReduced` | Perceived danger level |
| `WorkstationBusy` / `ExclusiveFacilityUnavailable` | `ContentionChanged` | Queue position |
| `SourceDepleted` | `CommodityAvailabilityChanged` | Resource quantity |
| `NoBuyer` | `TtlOnly` | N/A |
| `Unknown` / `PatienceExhausted` / `AssumptionFailed` | `TtlOnly` | N/A |

### Evaluation Logic

In the existing `blocks_goal_generation()` / `is_blocked_for_search()` path:

```rust
fn is_blocker_cleared(
    blocker: &BlockedIntent,
    view: &dyn GoalBeliefView,
    agent: EntityId,
    current_tick: Tick,
) -> bool {
    // TTL expiry (existing behavior, preserved)
    if current_tick >= blocker.expires_tick {
        return true;
    }
    // Condition-based clearing
    match (&blocker.clearing_condition, &blocker.baseline_snapshot) {
        (CommodityAvailabilityChanged { commodity, place }, Some(ClearingBaseline::CommodityQuantity { quantity: baseline })) => {
            let current = view.believed_commodity_at(*place, *commodity);
            current != *baseline  // Quantity changed — condition may have cleared
        }
        // ... other variants
        (TtlOnly, _) => false,  // Only TTL clears this
    }
}
```

## Cross-System Interactions

- **Perception** updates beliefs → changes baseline comparisons → clears blockers
- **Tell system** shares beliefs → may update commodity/path knowledge → clears blockers
- **AI pipeline** evaluates clearing conditions before candidate generation/search pruning
- **Event log** is not directly queried — clearing is belief-mediated, consistent with P14

## Profile-Driven Parameters

No new profiles. TTL values per `BlockingFact` are already configurable via `ReasoningProfile` (or `CognitiveProfile` after S53).

## Component Registration

No new components. `BlockedIntentMemory` is an AI runtime structure, not an ECS component.

## Section H — Causal Hooks

1. **Information path**: Clearing evidence arrives through perception (observation, tell, record consultation). Blocker checks compare current beliefs against block-time baseline.
2. **Positive feedback**: None. Clearing a blocker enables retrying a goal, which may succeed or re-block.
3. **Dampeners**: TTL fallback prevents indefinite blocking. Re-blocking on retry prevents infinite loops.
4. **Stored vs derived**: `clearing_condition` and `baseline_snapshot` are stored on `BlockedIntent`. Clearing evaluation is derived at query time from beliefs.
