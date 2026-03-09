//! Item-domain taxonomy types for stackable commodities and trade grouping.

use crate::{EntityId, EventId, Quantity, Tick};
use serde::{Deserialize, Serialize};

/// Stackable commodity kinds available in Phase 1.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum CommodityKind {
    Apple,
    Grain,
    Bread,
    Water,
    Firewood,
    Medicine,
    Coin,
    Waste,
}

impl CommodityKind {
    pub const ALL: [Self; 8] = [
        Self::Apple,
        Self::Grain,
        Self::Bread,
        Self::Water,
        Self::Firewood,
        Self::Medicine,
        Self::Coin,
        Self::Waste,
    ];

    pub const fn trade_category(self) -> TradeCategory {
        match self {
            Self::Apple | Self::Grain | Self::Bread => TradeCategory::Food,
            Self::Water => TradeCategory::Water,
            Self::Firewood => TradeCategory::Fuel,
            Self::Medicine => TradeCategory::Medicine,
            Self::Coin => TradeCategory::Coin,
            Self::Waste => TradeCategory::Waste,
        }
    }
}

/// Trade grouping that can later span both stackable and unique items.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum TradeCategory {
    Food,
    Water,
    Fuel,
    Medicine,
    Coin,
    SimpleTool,
    Weapon,
    Waste,
}

impl TradeCategory {
    pub const ALL: [Self; 8] = [
        Self::Food,
        Self::Water,
        Self::Fuel,
        Self::Medicine,
        Self::Coin,
        Self::SimpleTool,
        Self::Weapon,
        Self::Waste,
    ];
}

/// Lot lineage operations tracked in append-only provenance history.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum LotOperation {
    Created,
    Split,
    Merge,
    Produced,
    Consumed,
    Destroyed,
    Spoiled,
    Transformed,
}

impl LotOperation {
    pub const ALL: [Self; 8] = [
        Self::Created,
        Self::Split,
        Self::Merge,
        Self::Produced,
        Self::Consumed,
        Self::Destroyed,
        Self::Spoiled,
        Self::Transformed,
    ];
}

/// Immutable lineage record for a lot change.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceEntry {
    pub tick: Tick,
    pub event_id: Option<EventId>,
    pub operation: LotOperation,
    pub source_lot: Option<EntityId>,
    pub amount: Quantity,
}

#[cfg(test)]
mod tests {
    use super::{CommodityKind, LotOperation, ProvenanceEntry, TradeCategory};
    use crate::{EntityId, EventId, Quantity, Tick};
    use serde::{de::DeserializeOwned, Serialize};

    fn assert_enum_bounds<
        T: Copy + Clone + Eq + Ord + std::hash::Hash + std::fmt::Debug + Serialize + DeserializeOwned,
    >() {
    }

    #[test]
    fn commodity_kind_trait_bounds() {
        assert_enum_bounds::<CommodityKind>();
    }

    #[test]
    fn lot_operation_trait_bounds() {
        assert_enum_bounds::<LotOperation>();
    }

    #[test]
    fn trade_category_trait_bounds() {
        assert_enum_bounds::<TradeCategory>();
    }

    fn assert_struct_bounds<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn provenance_entry_trait_bounds() {
        assert_struct_bounds::<ProvenanceEntry>();
    }

    #[test]
    fn commodity_kind_all_is_canonical_variant_list() {
        assert_eq!(
            CommodityKind::ALL,
            [
                CommodityKind::Apple,
                CommodityKind::Grain,
                CommodityKind::Bread,
                CommodityKind::Water,
                CommodityKind::Firewood,
                CommodityKind::Medicine,
                CommodityKind::Coin,
                CommodityKind::Waste,
            ]
        );
    }

    #[test]
    fn trade_category_all_is_canonical_variant_list() {
        assert_eq!(
            TradeCategory::ALL,
            [
                TradeCategory::Food,
                TradeCategory::Water,
                TradeCategory::Fuel,
                TradeCategory::Medicine,
                TradeCategory::Coin,
                TradeCategory::SimpleTool,
                TradeCategory::Weapon,
                TradeCategory::Waste,
            ]
        );
    }

    #[test]
    fn lot_operation_all_is_canonical_variant_list() {
        assert_eq!(
            LotOperation::ALL,
            [
                LotOperation::Created,
                LotOperation::Split,
                LotOperation::Merge,
                LotOperation::Produced,
                LotOperation::Consumed,
                LotOperation::Destroyed,
                LotOperation::Spoiled,
                LotOperation::Transformed,
            ]
        );
    }

    #[test]
    fn commodity_kind_variants_roundtrip_through_bincode() {
        for commodity in CommodityKind::ALL {
            let bytes = bincode::serialize(&commodity).unwrap();
            let roundtrip: CommodityKind = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, commodity);
        }
    }

    #[test]
    fn trade_category_variants_roundtrip_through_bincode() {
        for category in TradeCategory::ALL {
            let bytes = bincode::serialize(&category).unwrap();
            let roundtrip: TradeCategory = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, category);
        }
    }

    #[test]
    fn lot_operation_variants_roundtrip_through_bincode() {
        for operation in LotOperation::ALL {
            let bytes = bincode::serialize(&operation).unwrap();
            let roundtrip: LotOperation = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, operation);
        }
    }

    #[test]
    fn provenance_entry_roundtrips_through_bincode() {
        let with_links = ProvenanceEntry {
            tick: Tick(11),
            event_id: Some(EventId(4)),
            operation: LotOperation::Split,
            source_lot: Some(EntityId {
                slot: 7,
                generation: 1,
            }),
            amount: Quantity(3),
        };
        let without_links = ProvenanceEntry {
            tick: Tick(12),
            event_id: None,
            operation: LotOperation::Created,
            source_lot: None,
            amount: Quantity(9),
        };

        for entry in [with_links, without_links] {
            let bytes = bincode::serialize(&entry).unwrap();
            let roundtrip: ProvenanceEntry = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, entry);
        }
    }

    #[test]
    fn commodity_kind_ordering_is_deterministic() {
        let mut reversed = CommodityKind::ALL;
        reversed.reverse();
        reversed.sort();
        assert_eq!(reversed, CommodityKind::ALL);
    }

    #[test]
    fn lot_operation_ordering_is_deterministic() {
        let mut reversed = LotOperation::ALL;
        reversed.reverse();
        reversed.sort();
        assert_eq!(reversed, LotOperation::ALL);
    }

    #[test]
    fn trade_category_ordering_is_deterministic() {
        let mut reversed = TradeCategory::ALL;
        reversed.reverse();
        reversed.sort();
        assert_eq!(reversed, TradeCategory::ALL);
    }

    #[test]
    fn commodity_kind_trade_category_mapping_matches_catalog() {
        assert_eq!(CommodityKind::Apple.trade_category(), TradeCategory::Food);
        assert_eq!(CommodityKind::Grain.trade_category(), TradeCategory::Food);
        assert_eq!(CommodityKind::Bread.trade_category(), TradeCategory::Food);
        assert_eq!(CommodityKind::Water.trade_category(), TradeCategory::Water);
        assert_eq!(
            CommodityKind::Firewood.trade_category(),
            TradeCategory::Fuel
        );
        assert_eq!(
            CommodityKind::Medicine.trade_category(),
            TradeCategory::Medicine
        );
        assert_eq!(CommodityKind::Coin.trade_category(), TradeCategory::Coin);
        assert_eq!(CommodityKind::Waste.trade_category(), TradeCategory::Waste);
    }
}
