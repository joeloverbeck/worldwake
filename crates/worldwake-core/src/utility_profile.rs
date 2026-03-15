//! Decision-architecture authoritative utility weighting for agents.

use crate::{Component, Permille};
use serde::{Deserialize, Serialize};

/// Stable per-agent utility weights used to diversify decision making.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UtilityProfile {
    pub hunger_weight: Permille,
    pub thirst_weight: Permille,
    pub fatigue_weight: Permille,
    pub bladder_weight: Permille,
    pub dirtiness_weight: Permille,
    pub pain_weight: Permille,
    pub danger_weight: Permille,
    pub enterprise_weight: Permille,
    pub social_weight: Permille,
}

impl Default for UtilityProfile {
    fn default() -> Self {
        let balanced = Permille::new_unchecked(500);
        let social = Permille::new_unchecked(200);
        Self {
            hunger_weight: balanced,
            thirst_weight: balanced,
            fatigue_weight: balanced,
            bladder_weight: balanced,
            dirtiness_weight: balanced,
            pain_weight: balanced,
            danger_weight: balanced,
            enterprise_weight: balanced,
            social_weight: social,
        }
    }
}

impl Component for UtilityProfile {}

#[cfg(test)]
mod tests {
    use super::UtilityProfile;
    use crate::traits::Component;
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn utility_profile_component_bounds() {
        assert_component_bounds::<UtilityProfile>();
        assert_value_bounds::<UtilityProfile>();
    }

    #[test]
    fn utility_profile_default_is_balanced() {
        let profile = UtilityProfile::default();

        assert_eq!(profile.hunger_weight.value(), 500);
        assert_eq!(profile.thirst_weight.value(), 500);
        assert_eq!(profile.fatigue_weight.value(), 500);
        assert_eq!(profile.bladder_weight.value(), 500);
        assert_eq!(profile.dirtiness_weight.value(), 500);
        assert_eq!(profile.pain_weight.value(), 500);
        assert_eq!(profile.danger_weight.value(), 500);
        assert_eq!(profile.enterprise_weight.value(), 500);
        assert_eq!(profile.social_weight.value(), 200);
        assert!(profile.social_weight < profile.enterprise_weight);
    }

    #[test]
    fn utility_profile_roundtrips_through_bincode() {
        let profile = UtilityProfile {
            social_weight: crate::Permille::new(875).unwrap(),
            ..UtilityProfile::default()
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: UtilityProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }
}
