use serde::{Deserialize, Serialize};

use crate::Permille;

/// Per-agent parameters for deriving route preference from traversal history.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoutePreferenceProfile {
    /// Preference increase per safe traversal.
    pub safe_traversal_weight: Permille,
    /// Preference penalty per dangerous traversal.
    pub dangerous_traversal_penalty: Permille,
    /// Whole days after which observations decay back to neutral.
    pub days_to_decay_observations: u32,
    /// Traversals required before preference can diverge from neutral.
    pub minimum_traversals: u8,
}

impl Default for RoutePreferenceProfile {
    fn default() -> Self {
        Self {
            safe_traversal_weight: Permille::new_unchecked(200),
            dangerous_traversal_penalty: Permille::new_unchecked(600),
            days_to_decay_observations: 30,
            minimum_traversals: 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RoutePreferenceProfile;
    use crate::Permille;

    #[test]
    fn route_preference_profile_default_matches_s151() {
        let profile = RoutePreferenceProfile::default();

        assert_eq!(profile.safe_traversal_weight, Permille::new_unchecked(200));
        assert_eq!(
            profile.dangerous_traversal_penalty,
            Permille::new_unchecked(600)
        );
        assert_eq!(profile.days_to_decay_observations, 30);
        assert_eq!(profile.minimum_traversals, 2);
    }
}
