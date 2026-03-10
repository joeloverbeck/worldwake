# E11TRAECO-009: Implement Trade System Tick Function

## Summary
Implement the trade system tick function that replaces the noop at `SystemId::Trade` dispatch table index 2. The tick has two responsibilities: aging `DemandMemory` and recording unmet demand observations.

## Dependencies
- E11TRAECO-003 (DemandMemory component)
- E11TRAECO-004 (TradeDispositionProfile component, for retention threshold)

## Files to Touch
- `crates/worldwake-systems/src/trade.rs` — add `trade_system_tick` function (or extend if created in E11TRAECO-008)
- `crates/worldwake-systems/src/lib.rs` — ensure trade module is exported

## Out of Scope
- Trade action handler (E11TRAECO-008)
- Substitute demand (E11TRAECO-010)
- Merchant restock (E11TRAECO-011)
- Valuation logic (E11TRAECO-007)
- Component definitions (done in core tickets)
- Wiring into `SystemDispatchTable` in simulation_state (that's integration; this ticket provides the `SystemFn`)

## Implementation Details

### 1. DemandMemory Aging
For each agent with both `DemandMemory` and `TradeDispositionProfile`:
- Drop all `DemandObservation` entries where `current_tick.0 - observation.tick.0 > agent.trade_disposition_profile.demand_memory_retention_ticks as u64`
- This is the concrete dampener for the demand-memory feedback loop (Principle 8)
- The per-agent threshold supports agent diversity (Principle 11)

### 2. Record Unmet Demand
For agents who attempted to find a trade counterparty but failed:
- No seller present for wanted commodity → append `DemandObservation` with reason `WantedToBuyButNoSeller`
- No buyer present for goods to sell → append `DemandObservation` with reason `WantedToSellButNoBuyer`

The function signature must match `SystemFn`:
```rust
pub fn trade_system_tick(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError>
```

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-systems` — all existing tests pass
- New test: aging prunes observations older than per-agent retention threshold
- New test: aging respects per-agent `demand_memory_retention_ticks` (different agents have different retention)
- New test: observations within retention window are preserved
- New test: agent without `TradeDispositionProfile` has no aging applied
- New test: agent without `DemandMemory` is skipped without panic
- New test: failed buy attempt records `WantedToBuyButNoSeller` observation
- New test: failed sell attempt records `WantedToSellButNoBuyer` observation
- New test: `DemandMemory` aging is deterministic (same input tick → same pruning result)

### Invariants That Must Remain True
- Principle 8: demand memory ages out — no signal lives forever
- Principle 11: per-agent retention threshold supports diversity
- Function signature matches `SystemFn` type
- No HashMap, no floats, no wall-clock time
- `cargo clippy --workspace` clean
