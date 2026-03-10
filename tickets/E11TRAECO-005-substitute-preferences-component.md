# E11TRAECO-005: Add `SubstitutePreferences` Component

## Summary
Define `SubstitutePreferences` in `crates/worldwake-core/src/trade.rs` and register it as a component on `EntityKind::Agent`. This component defines per-agent commodity substitution ordering within trade categories.

## Dependencies
- E11TRAECO-002 (trade.rs module must exist)

## Files to Touch
- `crates/worldwake-core/src/trade.rs` — add `SubstitutePreferences` struct
- `crates/worldwake-core/src/lib.rs` — re-export `SubstitutePreferences`
- `crates/worldwake-core/src/component_schema.rs` — register on `EntityKind::Agent`
- `crates/worldwake-core/src/component_tables.rs` — add import and storage

## Out of Scope
- Substitute demand logic in trade handler (E11TRAECO-010)
- Other trade components (separate tickets)
- Valuation or negotiation logic

## Implementation Details

```rust
use std::collections::BTreeMap;
use crate::{TradeCategory, CommodityKind};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SubstitutePreferences {
    pub preferences: BTreeMap<TradeCategory, Vec<CommodityKind>>,
}

impl Component for SubstitutePreferences {}
```

`BTreeMap` keyed by `TradeCategory` for deterministic iteration. `Vec<CommodityKind>` is ordered by preference (index 0 = most preferred). An agent without this component does not consider substitutes.

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-core` — all existing tests pass
- New test: `SubstitutePreferences` satisfies `Clone + Eq + Debug + Serialize + DeserializeOwned`
- New test: bincode roundtrip
- New test: component table insert/get/remove/has cycle
- New test: `BTreeMap` iteration order is deterministic (insert in non-sorted order, verify sorted iteration)

### Invariants That Must Remain True
- Component only registerable on `EntityKind::Agent`
- Uses `BTreeMap` (not `HashMap`)
- Existing component registrations unchanged
- `cargo clippy --workspace` clean
