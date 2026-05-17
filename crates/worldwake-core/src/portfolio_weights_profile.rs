use crate::{Component, Permille, SlotKind};
use serde::{Deserialize, Serialize};

/// Per-agent portfolio slot weights and planning breadth caps.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PortfolioWeightsProfile {
    /// Weight applied to need-survival slot candidates.
    pub need_survival: Permille,
    /// Weight applied to pain-care slot candidates.
    pub pain_care: Permille,
    /// Weight applied to obligation-duty slot candidates.
    pub obligation_duty: Permille,
    /// Weight applied to economic-opportunity slot candidates.
    pub economic_opportunity: Permille,
    /// Weight applied to social-motive slot candidates.
    pub social_motive: Permille,
    /// Maximum portfolio plans evaluated in normal operating mode.
    pub max_plans_normal: u8,
    /// Maximum portfolio plans evaluated in emergency operating mode.
    pub max_plans_emergency: u8,
    /// Maximum portfolio plans evaluated in idle operating mode.
    pub max_plans_idle: u8,
}

impl Default for PortfolioWeightsProfile {
    fn default() -> Self {
        Self {
            need_survival: Permille::new_unchecked(1000),
            pain_care: Permille::new_unchecked(900),
            obligation_duty: Permille::new_unchecked(800),
            economic_opportunity: Permille::new_unchecked(600),
            social_motive: Permille::new_unchecked(400),
            max_plans_normal: 5,
            max_plans_emergency: 3,
            max_plans_idle: 5,
        }
    }
}

impl PortfolioWeightsProfile {
    #[must_use]
    pub fn weight_for(&self, slot: SlotKind) -> Permille {
        match slot {
            SlotKind::NeedSurvival => self.need_survival,
            SlotKind::PainCare => self.pain_care,
            SlotKind::ObligationDuty => self.obligation_duty,
            SlotKind::EconomicOpportunity => self.economic_opportunity,
            SlotKind::SocialMotive => self.social_motive,
        }
    }
}

impl Component for PortfolioWeightsProfile {}

#[cfg(test)]
mod tests {
    use super::PortfolioWeightsProfile;
    use crate::{Permille, SlotKind, traits::Component};
    use ron::{
        de::from_str,
        ser::{PrettyConfig, to_string_pretty},
    };

    fn assert_component_bounds<T: Component>() {}

    #[test]
    fn portfolio_weights_profile_component_bounds() {
        assert_component_bounds::<PortfolioWeightsProfile>();
    }

    #[test]
    fn default_matches_spec_table() {
        let profile = PortfolioWeightsProfile::default();

        assert_eq!(profile.need_survival, Permille::new(1000).unwrap());
        assert_eq!(profile.pain_care, Permille::new(900).unwrap());
        assert_eq!(profile.obligation_duty, Permille::new(800).unwrap());
        assert_eq!(profile.economic_opportunity, Permille::new(600).unwrap());
        assert_eq!(profile.social_motive, Permille::new(400).unwrap());
        assert_eq!(profile.max_plans_normal, 5);
        assert_eq!(profile.max_plans_emergency, 3);
        assert_eq!(profile.max_plans_idle, 5);
    }

    #[test]
    fn weight_for_returns_correct_field_per_slot() {
        let profile = PortfolioWeightsProfile {
            need_survival: Permille::new(1000).unwrap(),
            pain_care: Permille::new(900).unwrap(),
            obligation_duty: Permille::new(800).unwrap(),
            economic_opportunity: Permille::new(600).unwrap(),
            social_motive: Permille::new(400).unwrap(),
            max_plans_normal: 5,
            max_plans_emergency: 3,
            max_plans_idle: 5,
        };

        assert_eq!(
            profile.weight_for(SlotKind::NeedSurvival),
            profile.need_survival
        );
        assert_eq!(profile.weight_for(SlotKind::PainCare), profile.pain_care);
        assert_eq!(
            profile.weight_for(SlotKind::ObligationDuty),
            profile.obligation_duty
        );
        assert_eq!(
            profile.weight_for(SlotKind::EconomicOpportunity),
            profile.economic_opportunity
        );
        assert_eq!(
            profile.weight_for(SlotKind::SocialMotive),
            profile.social_motive
        );
    }

    #[test]
    fn portfolio_weights_profile_roundtrips_through_bincode() {
        let profile = PortfolioWeightsProfile {
            need_survival: Permille::new(950).unwrap(),
            pain_care: Permille::new(850).unwrap(),
            obligation_duty: Permille::new(750).unwrap(),
            economic_opportunity: Permille::new(550).unwrap(),
            social_motive: Permille::new(350).unwrap(),
            max_plans_normal: 6,
            max_plans_emergency: 4,
            max_plans_idle: 7,
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: PortfolioWeightsProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn portfolio_weights_profile_roundtrips_through_ron() {
        let profile = PortfolioWeightsProfile::default();
        let serialized = to_string_pretty(&profile, PrettyConfig::default()).unwrap();
        let roundtrip: PortfolioWeightsProfile = from_str(&serialized).unwrap();

        assert_eq!(roundtrip, profile);
    }
}
