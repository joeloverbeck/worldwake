# E11TRAECO-011: Implement Merchant Restock Planning Inputs

**Status**: COMPLETED

## Summary
Implement the read-only query that determines which merchant sale kinds are currently out of stock and therefore should surface as restock planning inputs. This ticket provides a world-query helper only; the actual GOAP integration remains in E13.

## Dependencies
- Archived `E11TRAECO-002` (`MerchandiseProfile`)
- Archived `E11TRAECO-009` (`trade.rs` system module exists now)

## Assumption Reassessment (2026-03-11)

1. The original ticket assumed the implementation needed a `needs_restock` helper. That does not match the spec text or the current code layout. The more coherent API is a read-only `restock_candidates(agent, world) -> Vec<CommodityKind>` query in `crates/worldwake-systems/src/trade.rs`, alongside `trade_system_tick`.
2. The original dependency on `DemandMemory` is stale for the actual Phase 2 candidate-query surface. In the current spec wording, explicit merchant sale intent is already sufficient reason to maintain stock. That means `DemandMemory` does not change whether a sale kind is a restock candidate; it may matter later for prioritization, valuation, or procurement choice, but not for candidate presence.
3. The authoritative stock query already exists in `worldwake-core` as `World::controlled_commodity_quantity`. This ticket should use that existing ownership/inventory traversal rather than introducing a parallel trade-specific stock helper.
4. `crates/worldwake-systems/src/trade.rs` already exists from `E11TRAECO-009`, so this ticket extends that module rather than creating a new one.
5. Because `DemandMemory` is not decision-making input for the candidate set, tests that require it to produce a candidate would only restate the already-sufficient `MerchandiseProfile` intent and would not add architectural value.

## Architecture Check

1. Adding a pure query helper to `crates/worldwake-systems/src/trade.rs` is better than pushing this logic into AI code or into the trade system tick. The helper belongs with other trade-domain system queries, while E13 should decide how to consume it.
2. The narrowest robust rule is: `MerchandiseProfile.sale_kinds` defines merchant intent, and `World::controlled_commodity_quantity` defines whether stock is absent. That keeps restock grounded in concrete authoritative state and avoids introducing hidden heuristics, thresholds, or duplicate memory-based gating.
3. Pulling `DemandMemory` into the candidate query would make the architecture worse right now. It would couple merchant restock affordance discovery to demand-signal history even though sale intent already carries the merchant's long-lived purpose. Demand memory is better reserved for later prioritization, valuation, and route/source choice, where it can actually differentiate decisions.
4. No compatibility alias or wrapper is justified here. The codebase already split `trade.rs` for trade-domain system logic and `trade_actions.rs` for exchange action behavior; this ticket should reinforce that split, not blur it.

## Scope Correction

This ticket should:

1. Add `restock_candidates(agent, world)` to `crates/worldwake-systems/src/trade.rs`.
2. Re-export that helper from `crates/worldwake-systems/src/lib.rs`.
3. Use `MerchandiseProfile.sale_kinds` plus `World::controlled_commodity_quantity` to derive candidates deterministically.
4. Add focused tests for absent stock, present stock, missing merchant profile, demand-memory non-effect, and read-only behavior.

This ticket should not:

1. Require `DemandMemory` for candidate presence.
2. Introduce reorder thresholds, scarcity scores, or hidden market-state checks.
3. Mutate state, record events, or perform procurement.
4. Integrate with E13 planner code.

## Files to Touch
- `crates/worldwake-systems/src/trade.rs` — add `restock_candidates` query helper and tests
- `crates/worldwake-systems/src/lib.rs` — re-export helper

## Out of Scope
- GOAP goal generation (E13)
- Physical procurement actions (travel, buy, harvest — E10 provides these)
- Trade action handler (E11TRAECO-008)
- Substitute demand (E11TRAECO-010)
- Component definitions (done in core tickets)
- AI decision architecture integration

## Implementation Details

```rust
/// Returns merchant sale kinds currently absent from the agent's controlled stock.
pub fn restock_candidates(
    agent: EntityId,
    world: &World,
) -> Vec<CommodityKind>
```

A commodity becomes a restock candidate when BOTH of:
1. Agent has `MerchandiseProfile`
2. A `sale_kind` in that profile is absent from the agent's controlled stock (`World::controlled_commodity_quantity(agent, kind) == Quantity(0)`)

Key correction from the original ticket: `DemandMemory` is not part of the candidate-presence rule in the current architecture because explicit merchant sale intent already supplies the durable reason to keep the commodity in circulation. `DemandMemory` may later influence ordering or source selection, but not whether the commodity appears at all.

Restock is still not "stock < threshold -> restock." It is "I am a merchant for this kind, and I currently have none of it under my control."

No magical restock path exists. This function only identifies WHAT is missing; HOW is still satisfied through physical procurement in E10 and planner integration in E13.

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-systems` — all existing tests pass
- New test: agent with `MerchandiseProfile` + empty stock returns the missing sale kind as a restock candidate
- New test: agent with MerchandiseProfile + adequate stock returns no candidates
- New test: agent without MerchandiseProfile returns no candidates (empty vec)
- New test: `DemandMemory` without `MerchandiseProfile` does not create a candidate
- New test: `DemandMemory` does not change the candidate set when sale intent and stock state are unchanged
- New test: restock candidate does NOT appear from a naked reorder threshold or partial-stock heuristic
- New test: no magical restock path (function returns candidates only, no state mutation)

### Invariants That Must Remain True
- Principle 2: no base-price table, no scarcity multiplier, no threshold-based reorder
- Restock candidate presence is grounded in concrete authoritative state (`MerchandiseProfile` + stock absence)
- Function is read-only (queries world state, does not mutate)
- No hidden global market state consulted
- `cargo clippy --workspace` clean

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - Added `restock_candidates(agent, world)` to `crates/worldwake-systems/src/trade.rs`.
  - Re-exported `restock_candidates` from `crates/worldwake-systems/src/lib.rs`.
  - Added focused `worldwake-systems` tests covering empty stock, stocked sale kinds, missing `MerchandiseProfile`, demand-memory non-effect, threshold-free behavior, and read-only behavior.
- Deviations from original plan:
  - The ticket was corrected before implementation because its original dependency and test assumptions overstated the role of `DemandMemory`.
  - The implemented query is narrower and cleaner than the original draft: it derives candidates strictly from merchant intent plus concrete stock absence, leaving demand memory for later prioritization work rather than candidate presence.
  - The code uses the existing authoritative `World::controlled_commodity_quantity` API instead of introducing a parallel trade-specific stock helper.
- Verification results:
  - `cargo test -p worldwake-systems restock_candidates` passed.
  - `cargo test -p worldwake-systems` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
