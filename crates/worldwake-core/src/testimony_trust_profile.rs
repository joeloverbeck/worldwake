use serde::{Deserialize, Serialize};

use crate::Permille;

/// Per-agent parameters for deriving witness trust from testimony reliability.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestimonyTrustProfile {
    /// Trust increase per directly confirmed report.
    pub confirmation_weight: Permille,
    /// Trust penalty per directly refuted report.
    pub refutation_penalty: Permille,
    /// Trust penalty per stale report observation.
    pub stale_decay_per_tick: Permille,
    /// Trust penalty per contradicted report observation.
    pub contradicted_penalty: Permille,
    /// Observations required before trust can diverge from neutral.
    pub minimum_observations: u8,
    /// Trust below this value can damp or suppress testimony use.
    pub trust_threshold: Permille,
    /// Topic salience multiplier for route-hazard testimony.
    pub topic_weight_route_hazard: Permille,
    /// Topic salience multiplier for resource-availability testimony.
    pub topic_weight_resource_availability: Permille,
    /// Topic salience multiplier for office-holder testimony.
    pub topic_weight_office_holder: Permille,
    /// Topic salience multiplier for accusation-credibility testimony.
    pub topic_weight_accusation_credibility: Permille,
    /// Topic salience multiplier for bounty-validity testimony.
    pub topic_weight_bounty_validity: Permille,
    /// Topic salience multiplier for price-level testimony.
    pub topic_weight_price_level: Permille,
    /// Topic salience multiplier for entity-whereabouts testimony.
    pub topic_weight_entity_whereabouts: Permille,
    /// Topic salience multiplier for general-fact testimony.
    pub topic_weight_general_fact: Permille,
}

impl Default for TestimonyTrustProfile {
    fn default() -> Self {
        Self {
            confirmation_weight: Permille::new_unchecked(250),
            refutation_penalty: Permille::new_unchecked(400),
            stale_decay_per_tick: Permille::new_unchecked(1),
            contradicted_penalty: Permille::new_unchecked(350),
            minimum_observations: 2,
            trust_threshold: Permille::new_unchecked(400),
            topic_weight_route_hazard: Permille::new_unchecked(500),
            topic_weight_resource_availability: Permille::new_unchecked(500),
            topic_weight_office_holder: Permille::new_unchecked(500),
            topic_weight_accusation_credibility: Permille::new_unchecked(500),
            topic_weight_bounty_validity: Permille::new_unchecked(500),
            topic_weight_price_level: Permille::new_unchecked(500),
            topic_weight_entity_whereabouts: Permille::new_unchecked(500),
            topic_weight_general_fact: Permille::new_unchecked(500),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TestimonyTrustProfile;
    use crate::Permille;

    #[test]
    fn testimony_trust_profile_default_matches_s151() {
        let profile = TestimonyTrustProfile::default();

        assert_eq!(profile.confirmation_weight, Permille::new_unchecked(250));
        assert_eq!(profile.refutation_penalty, Permille::new_unchecked(400));
        assert_eq!(profile.stale_decay_per_tick, Permille::new_unchecked(1));
        assert_eq!(profile.contradicted_penalty, Permille::new_unchecked(350));
        assert_eq!(profile.minimum_observations, 2);
        assert_eq!(profile.trust_threshold, Permille::new_unchecked(400));
        assert_eq!(
            profile.topic_weight_route_hazard,
            Permille::new_unchecked(500)
        );
        assert_eq!(
            profile.topic_weight_resource_availability,
            Permille::new_unchecked(500)
        );
        assert_eq!(
            profile.topic_weight_office_holder,
            Permille::new_unchecked(500)
        );
        assert_eq!(
            profile.topic_weight_accusation_credibility,
            Permille::new_unchecked(500)
        );
        assert_eq!(
            profile.topic_weight_bounty_validity,
            Permille::new_unchecked(500)
        );
        assert_eq!(
            profile.topic_weight_price_level,
            Permille::new_unchecked(500)
        );
        assert_eq!(
            profile.topic_weight_entity_whereabouts,
            Permille::new_unchecked(500)
        );
        assert_eq!(
            profile.topic_weight_general_fact,
            Permille::new_unchecked(500)
        );
    }
}
