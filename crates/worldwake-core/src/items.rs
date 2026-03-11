//! Item-domain taxonomy types for stackable commodities, lots, and trade grouping.

use crate::{Component, EntityId, EventId, LoadUnits, Permille, Quantity, Tick};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

/// Stackable commodity kinds available in Phase 1.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum CommodityKind {
    Apple,
    Grain,
    Bread,
    Water,
    Firewood,
    Sword,
    Bow,
    Medicine,
    Coin,
    Waste,
}

impl CommodityKind {
    pub const ALL: [Self; 10] = [
        Self::Apple,
        Self::Grain,
        Self::Bread,
        Self::Water,
        Self::Firewood,
        Self::Sword,
        Self::Bow,
        Self::Medicine,
        Self::Coin,
        Self::Waste,
    ];

    pub const fn spec(self) -> CommodityKindSpec {
        match self {
            Self::Apple => commodity_spec(
                TradeCategory::Food,
                LoadUnits(1),
                Some(CommodityConsumableProfile::new(
                    nz(2),
                    pm(220),
                    pm(120),
                    pm(40),
                )),
                None,
            ),
            Self::Grain => commodity_spec(
                TradeCategory::Food,
                LoadUnits(1),
                Some(CommodityConsumableProfile::new(
                    nz(4),
                    pm(180),
                    pm(0),
                    pm(20),
                )),
                None,
            ),
            Self::Bread => commodity_spec(
                TradeCategory::Food,
                LoadUnits(1),
                Some(CommodityConsumableProfile::new(
                    nz(3),
                    pm(260),
                    pm(0),
                    pm(20),
                )),
                None,
            ),
            Self::Water => commodity_spec(
                TradeCategory::Water,
                LoadUnits(2),
                Some(CommodityConsumableProfile::new(
                    nz(1),
                    pm(0),
                    pm(320),
                    pm(220),
                )),
                None,
            ),
            Self::Firewood => commodity_spec(TradeCategory::Fuel, LoadUnits(3), None, None),
            Self::Sword => commodity_spec(
                TradeCategory::Weapon,
                LoadUnits(4),
                None,
                Some(CombatWeaponProfile::new(nz(4), pm(180), pm(55))),
            ),
            Self::Bow => commodity_spec(
                TradeCategory::Weapon,
                LoadUnits(3),
                None,
                Some(CombatWeaponProfile::new(nz(5), pm(140), pm(30))),
            ),
            Self::Medicine => commodity_spec(TradeCategory::Medicine, LoadUnits(1), None, None),
            Self::Coin => commodity_spec(TradeCategory::Coin, LoadUnits(1), None, None),
            Self::Waste => commodity_spec(TradeCategory::Waste, LoadUnits(1), None, None),
        }
    }
}

const fn commodity_spec(
    trade_category: TradeCategory,
    load_per_unit: LoadUnits,
    consumable_profile: Option<CommodityConsumableProfile>,
    combat_weapon_profile: Option<CombatWeaponProfile>,
) -> CommodityKindSpec {
    CommodityKindSpec {
        trade_category,
        physical_profile: CommodityPhysicalProfile { load_per_unit },
        consumable_profile,
        combat_weapon_profile,
        treatment_profile: match trade_category {
            TradeCategory::Medicine => Some(CommodityTreatmentProfile::new(nz(4), pm(60), pm(120))),
            _ => None,
        },
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct CommodityKindSpec {
    pub trade_category: TradeCategory,
    pub physical_profile: CommodityPhysicalProfile,
    pub consumable_profile: Option<CommodityConsumableProfile>,
    pub combat_weapon_profile: Option<CombatWeaponProfile>,
    pub treatment_profile: Option<CommodityTreatmentProfile>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct CommodityPhysicalProfile {
    pub load_per_unit: LoadUnits,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommodityConsumableProfile {
    pub consumption_ticks_per_unit: NonZeroU32,
    pub hunger_relief_per_unit: Permille,
    pub thirst_relief_per_unit: Permille,
    pub bladder_fill_per_unit: Permille,
}

impl CommodityConsumableProfile {
    #[must_use]
    pub const fn new(
        consumption_ticks_per_unit: NonZeroU32,
        hunger_relief_per_unit: Permille,
        thirst_relief_per_unit: Permille,
        bladder_fill_per_unit: Permille,
    ) -> Self {
        Self {
            consumption_ticks_per_unit,
            hunger_relief_per_unit,
            thirst_relief_per_unit,
            bladder_fill_per_unit,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommodityTreatmentProfile {
    pub treatment_ticks_per_unit: NonZeroU32,
    pub bleed_reduction_per_tick: Permille,
    pub severity_reduction_per_tick: Permille,
}

impl CommodityTreatmentProfile {
    #[must_use]
    pub const fn new(
        treatment_ticks_per_unit: NonZeroU32,
        bleed_reduction_per_tick: Permille,
        severity_reduction_per_tick: Permille,
    ) -> Self {
        Self {
            treatment_ticks_per_unit,
            bleed_reduction_per_tick,
            severity_reduction_per_tick,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CombatWeaponProfile {
    pub attack_duration_ticks: NonZeroU32,
    pub base_wound_severity: Permille,
    pub base_bleed_rate: Permille,
}

impl CombatWeaponProfile {
    #[must_use]
    pub const fn new(
        attack_duration_ticks: NonZeroU32,
        base_wound_severity: Permille,
        base_bleed_rate: Permille,
    ) -> Self {
        Self {
            attack_duration_ticks,
            base_wound_severity,
            base_bleed_rate,
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
    Traded,
}

impl LotOperation {
    pub const ALL: [Self; 9] = [
        Self::Created,
        Self::Split,
        Self::Merge,
        Self::Produced,
        Self::Consumed,
        Self::Destroyed,
        Self::Spoiled,
        Self::Transformed,
        Self::Traded,
    ];
}

/// Unique-item kinds for singular objects that cannot be stacked into lots.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum UniqueItemKind {
    SimpleTool,
    Contract,
    Artifact,
    OfficeInsignia,
    Misc,
}

impl UniqueItemKind {
    pub const ALL: [Self; 5] = [
        Self::SimpleTool,
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

const fn pm(value: u16) -> Permille {
    Permille::new_unchecked(value)
}

const fn nz(value: u32) -> NonZeroU32 {
    match NonZeroU32::new(value) {
        Some(value) => value,
        None => panic!("NonZeroU32 value must be greater than zero"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CombatWeaponProfile, CommodityConsumableProfile, CommodityKind, CommodityKindSpec,
        CommodityPhysicalProfile, CommodityTreatmentProfile, Container, ItemLot, LotOperation,
        ProvenanceEntry, TradeCategory, UniqueItem, UniqueItemKind, UniqueItemKindSpec,
        UniqueItemPhysicalProfile,
    };
    use crate::{traits::Component, EntityId, EventId, LoadUnits, Permille, Quantity, Tick};
    use serde::{de::DeserializeOwned, Serialize};
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;

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
                CommodityKind::Sword,
                CommodityKind::Bow,
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
                LotOperation::Traded,
            ]
        );
    }

    #[test]
    fn unique_item_kind_all_is_canonical_variant_list() {
        assert_eq!(
            UniqueItemKind::ALL,
            [
                UniqueItemKind::SimpleTool,
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
            operation: LotOperation::Traded,
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
            kind: UniqueItemKind::SimpleTool,
            name: Some("Hammer".to_string()),
            metadata: BTreeMap::from([
                ("condition".to_string(), "worn".to_string()),
                ("material".to_string(), "oak".to_string()),
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

    fn expected_spec(
        trade_category: TradeCategory,
        load_per_unit: LoadUnits,
        consumable_profile: Option<CommodityConsumableProfile>,
        combat_weapon_profile: Option<CombatWeaponProfile>,
        treatment_profile: Option<CommodityTreatmentProfile>,
    ) -> CommodityKindSpec {
        CommodityKindSpec {
            trade_category,
            physical_profile: CommodityPhysicalProfile { load_per_unit },
            consumable_profile,
            combat_weapon_profile,
            treatment_profile,
        }
    }

    fn expected_commodity_spec(kind: CommodityKind) -> CommodityKindSpec {
        match kind {
            CommodityKind::Apple => expected_spec(
                TradeCategory::Food,
                LoadUnits(1),
                Some(CommodityConsumableProfile::new(
                    NonZeroU32::new(2).unwrap(),
                    Permille::new_unchecked(220),
                    Permille::new_unchecked(120),
                    Permille::new_unchecked(40),
                )),
                None,
                None,
            ),
            CommodityKind::Grain => expected_spec(
                TradeCategory::Food,
                LoadUnits(1),
                Some(CommodityConsumableProfile::new(
                    NonZeroU32::new(4).unwrap(),
                    Permille::new_unchecked(180),
                    Permille::new_unchecked(0),
                    Permille::new_unchecked(20),
                )),
                None,
                None,
            ),
            CommodityKind::Bread => expected_spec(
                TradeCategory::Food,
                LoadUnits(1),
                Some(CommodityConsumableProfile::new(
                    NonZeroU32::new(3).unwrap(),
                    Permille::new_unchecked(260),
                    Permille::new_unchecked(0),
                    Permille::new_unchecked(20),
                )),
                None,
                None,
            ),
            CommodityKind::Water => expected_spec(
                TradeCategory::Water,
                LoadUnits(2),
                Some(CommodityConsumableProfile::new(
                    NonZeroU32::new(1).unwrap(),
                    Permille::new_unchecked(0),
                    Permille::new_unchecked(320),
                    Permille::new_unchecked(220),
                )),
                None,
                None,
            ),
            CommodityKind::Firewood => {
                expected_spec(TradeCategory::Fuel, LoadUnits(3), None, None, None)
            }
            CommodityKind::Sword => expected_spec(
                TradeCategory::Weapon,
                LoadUnits(4),
                None,
                Some(CombatWeaponProfile::new(
                    NonZeroU32::new(4).unwrap(),
                    Permille::new_unchecked(180),
                    Permille::new_unchecked(55),
                )),
                None,
            ),
            CommodityKind::Bow => expected_spec(
                TradeCategory::Weapon,
                LoadUnits(3),
                None,
                Some(CombatWeaponProfile::new(
                    NonZeroU32::new(5).unwrap(),
                    Permille::new_unchecked(140),
                    Permille::new_unchecked(30),
                )),
                None,
            ),
            CommodityKind::Medicine => expected_spec(
                TradeCategory::Medicine,
                LoadUnits(1),
                None,
                None,
                Some(CommodityTreatmentProfile::new(
                    NonZeroU32::new(4).unwrap(),
                    Permille::new_unchecked(60),
                    Permille::new_unchecked(120),
                )),
            ),
            CommodityKind::Coin => expected_spec(TradeCategory::Coin, LoadUnits(1), None, None, None),
            CommodityKind::Waste => {
                expected_spec(TradeCategory::Waste, LoadUnits(1), None, None, None)
            }
        }
    }

    #[test]
    fn commodity_kind_specs_match_catalog() {
        const APPLE_SPEC: CommodityKindSpec = CommodityKind::Apple.spec();

        assert_eq!(APPLE_SPEC.trade_category, TradeCategory::Food);
        for kind in CommodityKind::ALL {
            assert_eq!(kind.spec(), expected_commodity_spec(kind));
        }
    }

    #[test]
    fn food_and_water_commodities_have_consumable_profiles_with_non_zero_duration() {
        for kind in [
            CommodityKind::Apple,
            CommodityKind::Grain,
            CommodityKind::Bread,
            CommodityKind::Water,
        ] {
            let profile = kind.spec().consumable_profile.unwrap();
            assert!(profile.consumption_ticks_per_unit.get() > 0);
        }
    }

    #[test]
    fn non_consumable_commodities_have_no_consumable_profile() {
        for kind in [
            CommodityKind::Firewood,
            CommodityKind::Sword,
            CommodityKind::Bow,
            CommodityKind::Medicine,
            CommodityKind::Coin,
            CommodityKind::Waste,
        ] {
            assert_eq!(kind.spec().consumable_profile, None);
        }
    }

    #[test]
    fn water_consumable_profile_prioritizes_thirst_relief_and_bladder_fill() {
        let profile = CommodityKind::Water.spec().consumable_profile.unwrap();

        assert_eq!(profile.hunger_relief_per_unit, Permille::new(0).unwrap());
        assert!(profile.thirst_relief_per_unit.value() > 0);
        assert!(profile.bladder_fill_per_unit.value() >= 200);
    }

    #[test]
    fn commodity_consumable_profile_roundtrips_through_bincode() {
        let profile = CommodityConsumableProfile::new(
            NonZeroU32::new(3).unwrap(),
            Permille::new(210).unwrap(),
            Permille::new(90).unwrap(),
            Permille::new(30).unwrap(),
        );

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: CommodityConsumableProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn combat_weapon_profile_roundtrips_through_bincode() {
        let profile = CombatWeaponProfile::new(
            NonZeroU32::new(6).unwrap(),
            Permille::new(150).unwrap(),
            Permille::new(40).unwrap(),
        );

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: CombatWeaponProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn commodity_treatment_profile_roundtrips_through_bincode() {
        let profile = CommodityTreatmentProfile::new(
            NonZeroU32::new(5).unwrap(),
            Permille::new(70).unwrap(),
            Permille::new(90).unwrap(),
        );

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: CommodityTreatmentProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
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
            CommodityKind::Sword.spec().trade_category,
            TradeCategory::Weapon
        );
        assert_eq!(
            CommodityKind::Bow.spec().trade_category,
            TradeCategory::Weapon
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
    fn weapon_commodities_expose_combat_profiles() {
        assert_eq!(
            CommodityKind::Sword.spec().combat_weapon_profile,
            Some(CombatWeaponProfile::new(
                NonZeroU32::new(4).unwrap(),
                Permille::new(180).unwrap(),
                Permille::new(55).unwrap(),
            ))
        );
        assert_eq!(
            CommodityKind::Bow.spec().combat_weapon_profile,
            Some(CombatWeaponProfile::new(
                NonZeroU32::new(5).unwrap(),
                Permille::new(140).unwrap(),
                Permille::new(30).unwrap(),
            ))
        );
    }

    #[test]
    fn non_weapon_commodities_have_no_combat_profile() {
        for kind in [
            CommodityKind::Apple,
            CommodityKind::Grain,
            CommodityKind::Bread,
            CommodityKind::Water,
            CommodityKind::Firewood,
            CommodityKind::Medicine,
            CommodityKind::Coin,
            CommodityKind::Waste,
        ] {
            assert_eq!(kind.spec().combat_weapon_profile, None);
        }
    }

    #[test]
    fn only_medicine_exposes_treatment_profile() {
        assert_eq!(
            CommodityKind::Medicine.spec().treatment_profile,
            Some(CommodityTreatmentProfile::new(
                NonZeroU32::new(4).unwrap(),
                Permille::new(60).unwrap(),
                Permille::new(120).unwrap(),
            ))
        );

        for kind in [
            CommodityKind::Apple,
            CommodityKind::Grain,
            CommodityKind::Bread,
            CommodityKind::Water,
            CommodityKind::Firewood,
            CommodityKind::Sword,
            CommodityKind::Bow,
            CommodityKind::Coin,
            CommodityKind::Waste,
        ] {
            assert_eq!(kind.spec().treatment_profile, None);
        }
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
