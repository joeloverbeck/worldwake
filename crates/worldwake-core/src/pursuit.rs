//! Per-agent pursuit parameters governing remote pursuit willingness.

use crate::{Component, Permille};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// Stable per-agent parameters that govern whether and when to attempt
/// remote pursuit of a target (e.g., chasing a fleeing bandit or fugitive).
///
/// `min_location_confidence` is a threshold — the agent will only pursue
/// if their belief confidence in the target's location meets or exceeds it.
/// `max_pursuit_travel_ticks` caps how far (in travel ticks) the agent is
/// willing to chase.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PursuitProfile {
    pub min_location_confidence: Permille,
    pub max_pursuit_travel_ticks: NonZeroU32,
}

impl Component for PursuitProfile {}

#[cfg(test)]
mod tests {
    use super::PursuitProfile;
    use crate::{traits::Component, Permille};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;
    use std::num::NonZeroU32;

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn sample_pursuit_profile() -> PursuitProfile {
        PursuitProfile {
            min_location_confidence: pm(600),
            max_pursuit_travel_ticks: NonZeroU32::new(10).unwrap(),
        }
    }

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn pursuit_profile_satisfies_required_traits() {
        assert_component_bounds::<PursuitProfile>();
        assert_value_bounds::<PursuitProfile>();
    }

    #[test]
    fn pursuit_profile_roundtrips_through_bincode() {
        let profile = sample_pursuit_profile();

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: PursuitProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }
}
