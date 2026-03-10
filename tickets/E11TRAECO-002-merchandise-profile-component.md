# E11TRAECO-002: Add `MerchandiseProfile` Component

## Summary
Define the `MerchandiseProfile` struct in a new `crates/worldwake-core/src/trade.rs` module and register it as a component on `EntityKind::Agent`. This component declares what an agent is trying to carry and sell.

## Files to Touch
- `crates/worldwake-core/src/trade.rs` — **new file**: `MerchandiseProfile` struct
- `crates/worldwake-core/src/lib.rs` — add `pub mod trade;` and re-export `MerchandiseProfile`
- `crates/worldwake-core/src/component_schema.rs` — register `MerchandiseProfile` on `EntityKind::Agent`
- `crates/worldwake-core/src/component_tables.rs` — add `MerchandiseProfile` import and storage

## Out of Scope
- `DemandMemory`, `TradeDispositionProfile`, `SubstitutePreferences` (separate tickets)
- Trade action handler, valuation, or system tick logic
- `ActionPayload` changes
- `BeliefView` changes

## Implementation Details

```rust
// crates/worldwake-core/src/trade.rs
use crate::{Component, EntityId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use crate::CommodityKind;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MerchandiseProfile {
    pub sale_kinds: BTreeSet<CommodityKind>,
    pub home_market: Option<EntityId>,
}

impl Component for MerchandiseProfile {}
```

Register in `component_schema.rs` with `|kind| kind == EntityKind::Agent` kind-check, following the existing macro pattern (see `AgentData`, `WoundList` entries as templates). Add storage in `component_tables.rs` import list.

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-core` — all existing tests pass
- New test: `MerchandiseProfile` satisfies `Clone + Eq + Debug + Serialize + DeserializeOwned`
- New test: bincode roundtrip of `MerchandiseProfile`
- New test: component table insert/get/remove/has cycle works
- `BTreeSet` is used for `sale_kinds` (not `Vec` or `HashSet`)

### Invariants That Must Remain True
- Component is only registerable on `EntityKind::Agent`
- No `HashMap`/`HashSet` in authoritative state
- Existing component registrations unchanged
- `cargo clippy --workspace` clean
