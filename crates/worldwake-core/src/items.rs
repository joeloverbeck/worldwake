//! Item-domain taxonomy types for stackable commodities, lots, and trade grouping.

use crate::{Component, EntityId, EventId, LoadUnits, Quantity, Tick};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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

    pub const fn spec(self) -> CommodityKindSpec {
        match self {
            Self::Apple | Self::Grain | Self::Bread => CommodityKindSpec {
                trade_category: TradeCategory::Food,
                physical_profile: CommodityPhysicalProfile {
                    load_per_unit: LoadUnits(1),
                },
            },
            Self::Water => CommodityKindSpec {
                trade_category: TradeCategory::Water,
                physical_profile: CommodityPhysicalProfile {
                    load_per_unit: LoadUnits(2),
                },
            },
            Self::Firewood => CommodityKindSpec {
                trade_category: TradeCategory::Fuel,
                physical_profile: CommodityPhysicalProfile {
                    load_per_unit: LoadUnits(3),
                },
            },
            Self::Medicine => CommodityKindSpec {
                trade_category: TradeCategory::Medicine,
                physical_profile: CommodityPhysicalProfile {
                    load_per_unit: LoadUnits(1),
                },
            },
            Self::Coin => CommodityKindSpec {
                trade_category: TradeCategory::Coin,
                physical_profile: CommodityPhysicalProfile {
                    load_per_unit: LoadUnits(1),
                },
            },
            Self::Waste => CommodityKindSpec {
                trade_category: TradeCategory::Waste,
                physical_profile: CommodityPhysicalProfile {
                    load_per_unit: LoadUnits(1),
                },
            },
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct CommodityKindSpec {
    pub trade_category: TradeCategory,
    pub physical_profile: CommodityPhysicalProfile,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct CommodityPhysicalProfile {
    pub load_per_unit: LoadUnits,
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

/// Unique-item kinds for singular objects that cannot be stacked into lots.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum UniqueItemKind {
    SimpleTool,
    Weapon,
    Contract,
    Artifact,
    OfficeInsignia,
    Misc,
}

impl UniqueItemKind {
    pub const ALL: [Self; 6] = [
        Self::SimpleTool,
        Self::Weapon,
        Self::Contract,
        Self::Artifact,
        Self::OfficeInsignia,
        Self::Misc,
    ];

    pub const fn spec(self) -> UniqueItemKindSpec {
        match self {
            Self::SimpleTool | Self::Artifact => UniqueItemKindSpec {
                physical_profile: UniqueItemPhysicalProfile { load: LoadUnits(5) },
            },
            Self::Weapon => UniqueItemKindSpec {
                physical_profile: UniqueItemPhysicalProfile {
                    load: LoadUnits(10),
                },
            },
            Self::Contract => UniqueItemKindSpec {
                physical_profile: UniqueItemPhysicalProfile { load: LoadUnits(1) },
            },
            Self::OfficeInsignia => UniqueItemKindSpec {
                physical_profile: UniqueItemPhysicalProfile { load: LoadUnits(2) },
            },
            Self::Misc => UniqueItemKindSpec {
                physical_profile: UniqueItemPhysicalProfile { load: LoadUnits(3) },
            },
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct UniqueItemKindSpec {
    pub physical_profile: UniqueItemPhysicalProfile,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct UniqueItemPhysicalProfile {
    pub load: LoadUnits,
}

/// Immutable lineage record for a lot change.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceEntry {
    pub tick: Tick,
    pub event_id: Option<EventId>,
    pub operation: LotOperation,
    pub related_lot: Option<EntityId>,
    pub amount: Quantity,
}

/// Stackable conserved commodity lot stored as an ECS component on `ItemLot` entities.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ItemLot {
    pub commodity: CommodityKind,
    pub quantity: Quantity,
    pub provenance: Vec<ProvenanceEntry>,
}

impl Component for ItemLot {}

/// Singular item state stored on `UniqueItem` entities.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UniqueItem {
    pub kind: UniqueItemKind,
    pub name: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl Component for UniqueItem {}

/// Deterministic storage and admission policy for container entities.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Container {
    pub capacity: crate::LoadUnits,
    pub allowed_commodities: Option<BTreeSet<CommodityKind>>,
    pub allows_unique_items: bool,
    pub allows_nested_containers: bool,
}

impl Component for Container {}

#[cfg(test)]
mod tests {
    use super::{
        CommodityKind, CommodityKindSpec, CommodityPhysicalProfile, Container, ItemLot,
        LotOperation, ProvenanceEntry, TradeCategory, UniqueItem, UniqueItemKind,
        UniqueItemKindSpec, UniqueItemPhysicalProfile,
    };
    use crate::{traits::Component, EntityId, EventId, LoadUnits, Quantity, Tick};
    use serde::{de::DeserializeOwned, Serialize};
    use std::collections::{BTreeMap, BTreeSet};

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

    #[test]
    fn unique_item_kind_trait_bounds() {
        assert_enum_bounds::<UniqueItemKind>();
    }

    fn assert_struct_bounds<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn provenance_entry_trait_bounds() {
        assert_struct_bounds::<ProvenanceEntry>();
    }

    #[test]
    fn item_lot_component_bounds() {
        fn assert_component_bounds<T: Component + Eq + PartialEq>() {}
        assert_component_bounds::<ItemLot>();
    }

    #[test]
    fn unique_item_component_bounds() {
        fn assert_component_bounds<T: Component + Eq + PartialEq>() {}
        assert_component_bounds::<UniqueItem>();
    }

    #[test]
    fn container_component_bounds() {
        fn assert_component_bounds<T: Component + Eq + PartialEq>() {}
        assert_component_bounds::<Container>();
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
    fn unique_item_kind_all_is_canonical_variant_list() {
        assert_eq!(
            UniqueItemKind::ALL,
            [
                UniqueItemKind::SimpleTool,
                UniqueItemKind::Weapon,
                UniqueItemKind::Contract,
                UniqueItemKind::Artifact,
                UniqueItemKind::OfficeInsignia,
                UniqueItemKind::Misc,
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
    fn unique_item_kind_variants_roundtrip_through_bincode() {
        for kind in UniqueItemKind::ALL {
            let bytes = bincode::serialize(&kind).unwrap();
            let roundtrip: UniqueItemKind = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, kind);
        }
    }

    #[test]
    fn provenance_entry_roundtrips_through_bincode() {
        let with_links = ProvenanceEntry {
            tick: Tick(11),
            event_id: Some(EventId(4)),
            operation: LotOperation::Split,
            related_lot: Some(EntityId {
                slot: 7,
                generation: 1,
            }),
            amount: Quantity(3),
        };
        let without_links = ProvenanceEntry {
            tick: Tick(12),
            event_id: None,
            operation: LotOperation::Created,
            related_lot: None,
            amount: Quantity(9),
        };

        for entry in [with_links, without_links] {
            let bytes = bincode::serialize(&entry).unwrap();
            let roundtrip: ProvenanceEntry = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, entry);
        }
    }

    #[test]
    fn item_lot_roundtrips_through_bincode() {
        let lot = ItemLot {
            commodity: CommodityKind::Grain,
            quantity: Quantity(12),
            provenance: vec![
                ProvenanceEntry {
                    tick: Tick(3),
                    event_id: None,
                    operation: LotOperation::Created,
                    related_lot: None,
                    amount: Quantity(12),
                },
                ProvenanceEntry {
                    tick: Tick(5),
                    event_id: Some(EventId(2)),
                    operation: LotOperation::Produced,
                    related_lot: Some(EntityId {
                        slot: 4,
                        generation: 1,
                    }),
                    amount: Quantity(4),
                },
            ],
        };

        let bytes = bincode::serialize(&lot).unwrap();
        let roundtrip: ItemLot = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, lot);
    }

    #[test]
    fn unique_item_roundtrips_through_bincode() {
        let item = UniqueItem {
            kind: UniqueItemKind::Weapon,
            name: Some("Rusty Sword".to_string()),
            metadata: BTreeMap::from([
                ("condition".to_string(), "worn".to_string()),
                ("material".to_string(), "iron".to_string()),
            ]),
        };

        let bytes = bincode::serialize(&item).unwrap();
        let roundtrip: UniqueItem = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, item);
    }

    #[test]
    fn unique_item_with_empty_metadata_and_no_name_roundtrips_through_bincode() {
        let item = UniqueItem {
            kind: UniqueItemKind::Contract,
            name: None,
            metadata: BTreeMap::new(),
        };

        let bytes = bincode::serialize(&item).unwrap();
        let roundtrip: UniqueItem = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, item);
    }

    #[test]
    fn container_with_open_policy_roundtrips_through_bincode() {
        let container = Container {
            capacity: LoadUnits(25),
            allowed_commodities: None,
            allows_unique_items: true,
            allows_nested_containers: false,
        };

        let bytes = bincode::serialize(&container).unwrap();
        let roundtrip: Container = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, container);
    }

    #[test]
    fn container_with_commodity_restrictions_roundtrips_deterministically() {
        let container_a = Container {
            capacity: LoadUnits(40),
            allowed_commodities: Some(BTreeSet::from([
                CommodityKind::Water,
                CommodityKind::Bread,
                CommodityKind::Apple,
            ])),
            allows_unique_items: false,
            allows_nested_containers: true,
        };
        let mut allowed = BTreeSet::new();
        allowed.insert(CommodityKind::Apple);
        allowed.insert(CommodityKind::Water);
        allowed.insert(CommodityKind::Bread);
        let container_b = Container {
            capacity: LoadUnits(40),
            allowed_commodities: Some(allowed),
            allows_unique_items: false,
            allows_nested_containers: true,
        };

        let bytes_a = bincode::serialize(&container_a).unwrap();
        let bytes_b = bincode::serialize(&container_b).unwrap();
        let roundtrip: Container = bincode::deserialize(&bytes_a).unwrap();

        assert_eq!(bytes_a, bytes_b);
        assert_eq!(roundtrip, container_a);
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
    fn unique_item_kind_ordering_is_deterministic() {
        let mut reversed = UniqueItemKind::ALL;
        reversed.reverse();
        reversed.sort();
        assert_eq!(reversed, UniqueItemKind::ALL);
    }

    #[test]
    fn commodity_kind_specs_match_catalog() {
        let expected = [
            (
                CommodityKind::Apple,
                CommodityKindSpec {
                    trade_category: TradeCategory::Food,
                    physical_profile: CommodityPhysicalProfile {
                        load_per_unit: LoadUnits(1),
                    },
                },
            ),
            (
                CommodityKind::Grain,
                CommodityKindSpec {
                    trade_category: TradeCategory::Food,
                    physical_profile: CommodityPhysicalProfile {
                        load_per_unit: LoadUnits(1),
                    },
                },
            ),
            (
                CommodityKind::Bread,
                CommodityKindSpec {
                    trade_category: TradeCategory::Food,
                    physical_profile: CommodityPhysicalProfile {
                        load_per_unit: LoadUnits(1),
                    },
                },
            ),
            (
                CommodityKind::Water,
                CommodityKindSpec {
                    trade_category: TradeCategory::Water,
                    physical_profile: CommodityPhysicalProfile {
                        load_per_unit: LoadUnits(2),
                    },
                },
            ),
            (
                CommodityKind::Firewood,
                CommodityKindSpec {
                    trade_category: TradeCategory::Fuel,
                    physical_profile: CommodityPhysicalProfile {
                        load_per_unit: LoadUnits(3),
                    },
                },
            ),
            (
                CommodityKind::Medicine,
                CommodityKindSpec {
                    trade_category: TradeCategory::Medicine,
                    physical_profile: CommodityPhysicalProfile {
                        load_per_unit: LoadUnits(1),
                    },
                },
            ),
            (
                CommodityKind::Coin,
                CommodityKindSpec {
                    trade_category: TradeCategory::Coin,
                    physical_profile: CommodityPhysicalProfile {
                        load_per_unit: LoadUnits(1),
                    },
                },
            ),
            (
                CommodityKind::Waste,
                CommodityKindSpec {
                    trade_category: TradeCategory::Waste,
                    physical_profile: CommodityPhysicalProfile {
                        load_per_unit: LoadUnits(1),
                    },
                },
            ),
        ];

        assert_eq!(expected.len(), CommodityKind::ALL.len());
        for (kind, spec) in expected {
            assert_eq!(kind.spec(), spec);
        }
    }

    #[test]
    fn unique_item_kind_specs_match_catalog() {
        let expected = [
            (
                UniqueItemKind::SimpleTool,
                UniqueItemKindSpec {
                    physical_profile: UniqueItemPhysicalProfile { load: LoadUnits(5) },
                },
            ),
            (
                UniqueItemKind::Weapon,
                UniqueItemKindSpec {
                    physical_profile: UniqueItemPhysicalProfile {
                        load: LoadUnits(10),
                    },
                },
            ),
            (
                UniqueItemKind::Contract,
                UniqueItemKindSpec {
                    physical_profile: UniqueItemPhysicalProfile { load: LoadUnits(1) },
                },
            ),
            (
                UniqueItemKind::Artifact,
                UniqueItemKindSpec {
                    physical_profile: UniqueItemPhysicalProfile { load: LoadUnits(5) },
                },
            ),
            (
                UniqueItemKind::OfficeInsignia,
                UniqueItemKindSpec {
                    physical_profile: UniqueItemPhysicalProfile { load: LoadUnits(2) },
                },
            ),
            (
                UniqueItemKind::Misc,
                UniqueItemKindSpec {
                    physical_profile: UniqueItemPhysicalProfile { load: LoadUnits(3) },
                },
            ),
        ];

        assert_eq!(expected.len(), UniqueItemKind::ALL.len());
        for (kind, spec) in expected {
            assert_eq!(kind.spec(), spec);
        }
    }

    #[test]
    fn commodity_kind_specs_include_trade_category_mapping() {
        assert_eq!(
            CommodityKind::Apple.spec().trade_category,
            TradeCategory::Food
        );
        assert_eq!(
            CommodityKind::Grain.spec().trade_category,
            TradeCategory::Food
        );
        assert_eq!(
            CommodityKind::Bread.spec().trade_category,
            TradeCategory::Food
        );
        assert_eq!(
            CommodityKind::Water.spec().trade_category,
            TradeCategory::Water
        );
        assert_eq!(
            CommodityKind::Firewood.spec().trade_category,
            TradeCategory::Fuel
        );
        assert_eq!(
            CommodityKind::Medicine.spec().trade_category,
            TradeCategory::Medicine
        );
        assert_eq!(
            CommodityKind::Coin.spec().trade_category,
            TradeCategory::Coin
        );
        assert_eq!(
            CommodityKind::Waste.spec().trade_category,
            TradeCategory::Waste
        );
    }

    #[test]
    fn unique_item_metadata_serialization_is_deterministic() {
        let item_a = UniqueItem {
            kind: UniqueItemKind::Artifact,
            name: Some("Seal".to_string()),
            metadata: BTreeMap::from([
                ("era".to_string(), "old".to_string()),
                ("origin".to_string(), "court".to_string()),
            ]),
        };
        let mut metadata = BTreeMap::new();
        metadata.insert("origin".to_string(), "court".to_string());
        metadata.insert("era".to_string(), "old".to_string());
        let item_b = UniqueItem {
            kind: UniqueItemKind::Artifact,
            name: Some("Seal".to_string()),
            metadata,
        };

        let bytes_a = bincode::serialize(&item_a).unwrap();
        let bytes_b = bincode::serialize(&item_b).unwrap();

        assert_eq!(bytes_a, bytes_b);
    }
}
