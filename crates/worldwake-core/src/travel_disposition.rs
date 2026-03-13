//! Travel-domain authoritative profile components.

use crate::{Component, Permille};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// Per-agent journey commitment and blocked-leg tolerance.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TravelDispositionProfile {
    pub route_replan_margin: Permille,
    pub blocked_leg_patience_ticks: NonZeroU32,
}

impl Component for TravelDispositionProfile {}

#[cfg(test)]
mod tests {
    use super::TravelDispositionProfile;
    use crate::{test_utils::sample_travel_disposition_profile, traits::Component};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn travel_disposition_profile_component_bounds() {
        assert_component_bounds::<TravelDispositionProfile>();
        assert_value_bounds::<TravelDispositionProfile>();
    }

    #[test]
    fn travel_disposition_profile_roundtrips_through_bincode() {
        let profile = sample_travel_disposition_profile();

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: TravelDispositionProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }
}
