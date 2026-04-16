//! Agent-local bounds for commodity opportunity valuation reasoning.

use crate::{Component, Permille};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU8;

/// Stable per-agent limits for bounded indirect commodity valuation.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct CommodityValuationProfile {
    /// Maximum recipe chain depth for indirect commodity valuation (e.g., ore -> bar -> sword).
    pub recipe_opportunity_depth: NonZeroU8,
    /// Maximum graph distance (in hops) to consider recipe opportunities at remote locations.
    pub recipe_place_horizon: u8,
    /// Value decay applied per step in an indirect valuation chain.
    pub indirect_value_decay_per_step: Permille,
}

impl Component for CommodityValuationProfile {}

#[cfg(test)]
mod tests {
    use super::CommodityValuationProfile;
    use crate::{test_utils::sample_commodity_valuation_profile, traits::Component};
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_copy_value_bounds<
        T: Copy + Clone + Eq + Ord + std::hash::Hash + Debug + Serialize + DeserializeOwned,
    >() {
    }

    #[test]
    fn commodity_valuation_profile_component_bounds() {
        assert_component_bounds::<CommodityValuationProfile>();
        assert_copy_value_bounds::<CommodityValuationProfile>();
    }

    #[test]
    fn commodity_valuation_profile_roundtrips_through_bincode() {
        let profile = sample_commodity_valuation_profile();

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: CommodityValuationProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }
}
