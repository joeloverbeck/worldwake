# E11TRAECO-011: Implement Merchant Restock Planning Inputs

## Summary
Implement the logic that determines when a merchant agent should consider restocking. Restock becomes a grounded candidate goal — not an automatic action. This ticket provides the query/evidence function; the actual GOAP integration is in E13.

## Dependencies
- E11TRAECO-002 (MerchandiseProfile)
- E11TRAECO-003 (DemandMemory)

## Files to Touch
- `crates/worldwake-systems/src/trade.rs` — add `needs_restock` query function
- `crates/worldwake-systems/src/lib.rs` — ensure export

## Out of Scope
- GOAP goal generation (E13)
- Physical procurement actions (travel, buy, harvest — E10 provides these)
- Trade action handler (E11TRAECO-008)
- Substitute demand (E11TRAECO-010)
- Component definitions (done in core tickets)
- AI decision architecture integration

## Implementation Details

```rust
/// Returns commodities the merchant should consider restocking.
/// Restock is grounded: the agent must have MerchandiseProfile,
/// lack stock of a sale_kind, AND have concrete reason to believe
/// restocking is worthwhile.
pub fn restock_candidates(
    agent: EntityId,
    world: &World,
) -> Vec<CommodityKind>
```

A commodity becomes a restock candidate when ALL of:
1. Agent has `MerchandiseProfile`
2. A `sale_kind` in that profile is absent from the agent's saleable stock (commodity_quantity == 0 for that kind)
3. The agent either:
   - Has recent `DemandMemory` entries for that kind (someone wanted to buy it), OR
   - Has explicit sale intent (the kind is in `sale_kinds` — this alone is sufficient intent to maintain stock)

Key correction from spec: restock is NOT "stock < threshold -> restock." It is "I am a merchant for this kind, I currently lack stock, and I have concrete reason to believe obtaining it is worthwhile."

No magical restock path exists — this function only identifies WHAT to restock. HOW is satisfied through physical procurement (travel + buy/harvest/receive delivery from E10).

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-systems` — all existing tests pass
- New test: agent with MerchandiseProfile + empty stock + recent DemandMemory returns restock candidate
- New test: agent with MerchandiseProfile + empty stock + explicit sale intent returns restock candidate
- New test: agent with MerchandiseProfile + adequate stock returns no candidates
- New test: agent without MerchandiseProfile returns no candidates (empty vec)
- New test: restock candidate does NOT appear from a naked reorder threshold
- New test: no magical restock path (function returns candidates only, no state mutation)

### Invariants That Must Remain True
- Principle 2: no base-price table, no scarcity multiplier, no threshold-based reorder
- Restock is grounded in concrete evidence (sale intent + stock absence + demand memory)
- Function is read-only (queries world state, does not mutate)
- No hidden global market state consulted
- `cargo clippy --workspace` clean
