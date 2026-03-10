# E11TRAECO — Trade, Exchange & Merchant Restock: Ticket Index

## Spec
`specs/E11-trade-economy.md`

## Dependency Graph

```text
E11TRAECO-001 (LotOperation::Traded)  ─────────────────────────────────┐
                                                                        │
E11TRAECO-002 (MerchandiseProfile)  ──┬─ E11TRAECO-003 (DemandMemory)  │
                                      ├─ E11TRAECO-004 (TradeDisposition)│
                                      └─ E11TRAECO-005 (SubstitutePrefs)│
                                                                        │
E11TRAECO-006 (TradeActionPayload)  ────────────────────────────────────┤
                                                                        │
E11TRAECO-003 + E11TRAECO-002 ──────── E11TRAECO-007 (Valuation)        │
                                                                        │
E11TRAECO-001 + 006 + 007 + 002-004 ── E11TRAECO-008 (Trade Handler) ──┤
                                                                        │
E11TRAECO-003 + 004 ────────────────── E11TRAECO-009 (System Tick)  ────┤
                                                                        │
E11TRAECO-005 + 007 + 008 ─────────── E11TRAECO-010 (Substitute Demand)│
                                                                        │
E11TRAECO-002 + 003 ───────────────── E11TRAECO-011 (Restock Inputs)   │
                                                                        │
E11TRAECO-006 + 008 + 009 + 002-005 ── E11TRAECO-012 (Integration)  ───┘
```

## Recommended Execution Order

### Wave 1 (parallel, no cross-dependencies)
- **E11TRAECO-001**: LotOperation::Traded variant
- **E11TRAECO-002**: MerchandiseProfile component
- **E11TRAECO-006**: TradeActionPayload

### Wave 2 (parallel, depends on 002 for trade.rs module)
- **E11TRAECO-003**: DemandMemory component
- **E11TRAECO-004**: TradeDispositionProfile component
- **E11TRAECO-005**: SubstitutePreferences component

### Wave 3 (depends on components)
- **E11TRAECO-007**: evaluate_trade_bundle valuation helper

### Wave 4 (depends on valuation + payload + components)
- **E11TRAECO-008**: Trade action handler
- **E11TRAECO-009**: Trade system tick (can parallel with 008)
- **E11TRAECO-011**: Merchant restock inputs (can parallel with 008)

### Wave 5 (depends on handler)
- **E11TRAECO-010**: Substitute demand logic

### Wave 6 (integration, depends on all above)
- **E11TRAECO-012**: Affordance query + system dispatch wiring

## Verification Gate
After all tickets complete, the following must hold:
- `cargo test --workspace` — all tests pass
- `cargo clippy --workspace` — no warnings
- `verify_conservation` passes after any trade sequence
- No `HashMap`/`HashSet` in authoritative state
- No `f32`/`f64` in trade logic
- No global base price, scarcity table, or hidden market state
- Pricing emerges from bilateral bundle valuation only
