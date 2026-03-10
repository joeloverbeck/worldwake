//! Trade-domain authoritative components and shared schema.

use crate::{CommodityKind, Component, EntityId, Quantity, Tick};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Concrete merchant sale intent for an agent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MerchandiseProfile {
    pub sale_kinds: BTreeSet<CommodityKind>,
    pub home_market: Option<EntityId>,
}

impl Component for MerchandiseProfile {}

/// Local concrete memory of missed demand and sale opportunities.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DemandMemory {
    pub observations: Vec<DemandObservation>,
}

impl Component for DemandMemory {}

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
        DemandMemory, DemandObservation, DemandObservationReason, MerchandiseProfile,
    };
    use crate::{
        test_utils::{sample_demand_observation, sample_merchandise_profile},
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
}
