//! Trade-domain authoritative components and shared schema.

use crate::{CommodityKind, Component, EntityId, Permille, Quantity, Tick, TradeCategory};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

/// Concrete merchant sale intent for an agent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MerchandiseProfile {
    pub sale_kinds: BTreeSet<CommodityKind>,
    pub home_facility: Option<EntityId>,
}

impl Component for MerchandiseProfile {}

/// Marks an `ItemLot` as actively offered for sale at the time it was listed.
///
/// Only `listed_at` is stored. Seller, commodity, and place are all derived
/// from authoritative relations (direct possessor, `ItemLot.commodity`,
/// lot effective place).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SaleListing {
    pub listed_at: Tick,
}

impl Component for SaleListing {}

/// Records which containers a facility uses for merchant stock storage and
/// sale display.  Belongs on `EntityKind::Facility` entities.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StockStoragePolicy {
    /// Long-lived storage container for local market/shop inventory.
    pub stock_container: EntityId,
    /// Optional seller-facing display container for buyer-visible sale stock.
    pub display_container: Option<EntityId>,
}

impl Component for StockStoragePolicy {}

/// Whether a lot is ordinary storage stock or active sale stock.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum StockAssignmentKind {
    /// Local stock counted for inventory/audit but not automatically sale-visible.
    Stored,
    /// Local stock staged for active sale visibility.
    Displayed,
}

/// Records a lot's assignment to a facility's stock or display container.
/// Belongs on `EntityKind::ItemLot` entities.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StockAssignment {
    /// The facility this lot is assigned to.
    pub facility: EntityId,
    /// Whether the lot is stored or displayed.
    pub kind: StockAssignmentKind,
}

impl Component for StockAssignment {}

/// Local concrete memory of missed demand and sale opportunities.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DemandMemory {
    pub observations: Vec<DemandObservation>,
}

impl Component for DemandMemory {}

/// Per-agent negotiation pacing, opening stance, and demand-memory retention.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TradeDispositionProfile {
    pub negotiation_round_ticks: NonZeroU32,
    pub initial_offer_bias: Permille,
    pub concession_rate: Permille,
    pub demand_memory_retention_ticks: u32,
    pub market_presence_ticks: NonZeroU32,
}

impl Component for TradeDispositionProfile {}

/// Per-agent ordered substitute choices by trade category.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SubstitutePreferences {
    pub preferences: BTreeMap<TradeCategory, Vec<CommodityKind>>,
}

impl Component for SubstitutePreferences {}

/// A single unmet-demand or missed-sale observation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DemandObservation {
    pub commodity: CommodityKind,
    pub quantity: Quantity,
    pub place: EntityId,
    pub tick: Tick,
    pub counterparty: Option<EntityId>,
    pub reason: DemandObservationReason,
}

/// Why a concrete demand observation was recorded.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum DemandObservationReason {
    WantedToBuyButNoSeller,
    WantedToBuyButSellerOutOfStock,
    WantedToBuyButTooExpensive,
    WantedToSellButNoBuyer,
}

#[cfg(test)]
mod tests {
    use super::{
        DemandMemory, DemandObservation, DemandObservationReason, MerchandiseProfile, SaleListing,
        StockAssignment, StockAssignmentKind, StockStoragePolicy, SubstitutePreferences,
        TradeDispositionProfile,
    };
    use crate::{
        test_utils::{
            sample_demand_observation, sample_merchandise_profile, sample_stock_assignment,
            sample_stock_storage_policy, sample_substitute_preferences,
            sample_trade_disposition_profile,
        },
        traits::Component,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn assert_copy_value_bounds<
        T: Copy + Clone + Eq + Ord + std::hash::Hash + Debug + Serialize + DeserializeOwned,
    >() {
    }

    #[test]
    fn merchandise_profile_component_bounds() {
        assert_component_bounds::<MerchandiseProfile>();
        assert_value_bounds::<MerchandiseProfile>();
    }

    #[test]
    fn merchandise_profile_roundtrips_through_bincode() {
        let profile = sample_merchandise_profile();

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: MerchandiseProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn demand_memory_component_bounds() {
        assert_component_bounds::<DemandMemory>();
        assert_value_bounds::<DemandMemory>();
    }

    #[test]
    fn trade_disposition_profile_component_bounds() {
        assert_component_bounds::<TradeDispositionProfile>();
        assert_value_bounds::<TradeDispositionProfile>();
    }

    #[test]
    fn demand_observation_roundtrips_through_bincode() {
        let observation = sample_demand_observation();

        let bytes = bincode::serialize(&observation).unwrap();
        let roundtrip: DemandObservation = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, observation);
    }

    #[test]
    fn demand_observation_reason_value_bounds() {
        assert_copy_value_bounds::<DemandObservationReason>();
    }

    #[test]
    fn trade_disposition_profile_roundtrips_through_bincode() {
        let profile = sample_trade_disposition_profile();

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: TradeDispositionProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn substitute_preferences_component_bounds() {
        assert_component_bounds::<SubstitutePreferences>();
        assert_value_bounds::<SubstitutePreferences>();
    }

    #[test]
    fn substitute_preferences_roundtrip_through_bincode() {
        let preferences = sample_substitute_preferences();

        let bytes = bincode::serialize(&preferences).unwrap();
        let roundtrip: SubstitutePreferences = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, preferences);
    }

    #[test]
    fn substitute_preferences_btreemap_order_is_deterministic() {
        let preferences = sample_substitute_preferences();

        let seen = preferences.preferences.keys().copied().collect::<Vec<_>>();

        assert_eq!(
            seen,
            vec![
                crate::TradeCategory::Food,
                crate::TradeCategory::Fuel,
                crate::TradeCategory::Medicine,
            ]
        );
    }

    #[test]
    fn sale_listing_component_bounds() {
        assert_component_bounds::<SaleListing>();
        assert_value_bounds::<SaleListing>();
    }

    #[test]
    fn sale_listing_roundtrips_through_bincode() {
        let listing = SaleListing {
            listed_at: crate::Tick(42),
        };

        let bytes = bincode::serialize(&listing).unwrap();
        let roundtrip: SaleListing = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, listing);
    }

    #[test]
    fn trade_disposition_profile_with_market_presence_roundtrips() {
        let profile = sample_trade_disposition_profile();
        assert!(profile.market_presence_ticks.get() > 0);

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: TradeDispositionProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn stock_storage_policy_component_bounds() {
        assert_component_bounds::<StockStoragePolicy>();
        assert_value_bounds::<StockStoragePolicy>();
    }

    #[test]
    fn stock_storage_policy_roundtrips_through_bincode() {
        let policy = sample_stock_storage_policy();

        let bytes = bincode::serialize(&policy).unwrap();
        let roundtrip: StockStoragePolicy = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, policy);
    }

    #[test]
    fn stock_storage_policy_without_display_roundtrips() {
        let policy = StockStoragePolicy {
            stock_container: crate::test_utils::entity_id(5, 1),
            display_container: None,
        };

        let bytes = bincode::serialize(&policy).unwrap();
        let roundtrip: StockStoragePolicy = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, policy);
        assert!(roundtrip.display_container.is_none());
    }

    #[test]
    fn stock_assignment_component_bounds() {
        assert_component_bounds::<StockAssignment>();
    }

    #[test]
    fn stock_assignment_kind_value_bounds() {
        assert_copy_value_bounds::<StockAssignmentKind>();
    }

    #[test]
    fn stock_assignment_roundtrips_through_bincode() {
        let assignment = sample_stock_assignment();

        let bytes = bincode::serialize(&assignment).unwrap();
        let roundtrip: StockAssignment = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, assignment);
    }

    #[test]
    fn stock_assignment_displayed_variant_roundtrips() {
        let assignment = StockAssignment {
            facility: crate::test_utils::entity_id(3, 1),
            kind: StockAssignmentKind::Displayed,
        };

        let bytes = bincode::serialize(&assignment).unwrap();
        let roundtrip: StockAssignment = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, assignment);
        assert_eq!(roundtrip.kind, StockAssignmentKind::Displayed);
    }
}
