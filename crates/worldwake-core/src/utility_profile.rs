//! Decision-architecture authoritative utility weighting for agents.

use crate::{Component, Permille};
use serde::{Deserialize, Serialize};

/// Stable per-agent utility weights used to diversify decision making.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UtilityProfile {
    /// Weight applied to hunger-driven goals in utility scoring.
    pub hunger_weight: Permille,
    /// Weight applied to thirst-driven goals in utility scoring.
    pub thirst_weight: Permille,
    /// Weight applied to fatigue-driven goals in utility scoring.
    pub fatigue_weight: Permille,
    /// Weight applied to bladder-driven goals in utility scoring.
    pub bladder_weight: Permille,
    /// Weight applied to dirtiness-driven goals in utility scoring.
    pub dirtiness_weight: Permille,
    /// Weight applied to pain-driven goals in utility scoring.
    pub pain_weight: Permille,
    /// Weight applied to danger-avoidance goals in utility scoring.
    pub danger_weight: Permille,
    /// Weight applied to economic and productive goals in utility scoring.
    pub enterprise_weight: Permille,
    /// Weight applied to social interaction goals in utility scoring.
    pub social_weight: Permille,
    /// Weight applied to awareness-of-activity goals (observation, investigation).
    pub activity_awareness_weight: Permille,
    /// Weight applied to opportunistic side-benefit actions discovered during planning.
    pub side_benefit_weight: Permille,
    /// Weight applied to bounty-posting goals. 0 disables bounty posting.
    pub bounty_posting_weight: Permille,
    /// Weight applied to notice-posting goals. 0 disables notice posting.
    pub notice_posting_weight: Permille,
    /// Willingness to engage in risky actions such as combat or confrontation.
    pub courage: Permille,
    /// Weight applied to caregiving and obligation-fulfillment goals.
    pub care_weight: Permille,
    /// Weight applied to office-duty motive sources.
    #[serde(default = "default_office_duty_weight")]
    pub office_duty_weight: Permille,
    /// Weight applied to loyalty motive sources.
    #[serde(default = "default_loyalty_weight")]
    pub loyalty_weight: Permille,
    /// Weight applied to greed/opportunity motive sources.
    #[serde(default = "default_greed_weight")]
    pub greed_weight: Permille,
    /// Weight applied to shame motive sources.
    #[serde(default = "default_shame_weight")]
    pub shame_weight: Permille,
    /// Weight applied to revenge motive sources.
    #[serde(default = "default_revenge_weight")]
    pub revenge_weight: Permille,
}

fn default_office_duty_weight() -> Permille {
    Permille::new_unchecked(500)
}

fn default_loyalty_weight() -> Permille {
    Permille::new_unchecked(500)
}

fn default_greed_weight() -> Permille {
    Permille::new_unchecked(500)
}

fn default_shame_weight() -> Permille {
    Permille::new_unchecked(400)
}

fn default_revenge_weight() -> Permille {
    Permille::new_unchecked(400)
}

impl Default for UtilityProfile {
    fn default() -> Self {
        let balanced = Permille::new_unchecked(500);
        let social = Permille::new_unchecked(200);
        let side_benefit = Permille::new_unchecked(100);
        let disabled = Permille::new_unchecked(0);
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
            activity_awareness_weight: social,
            side_benefit_weight: side_benefit,
            bounty_posting_weight: disabled,
            notice_posting_weight: disabled,
            courage: balanced,
            care_weight: social,
            office_duty_weight: default_office_duty_weight(),
            loyalty_weight: default_loyalty_weight(),
            greed_weight: default_greed_weight(),
            shame_weight: default_shame_weight(),
            revenge_weight: default_revenge_weight(),
        }
    }
}

impl Component for UtilityProfile {}

#[cfg(test)]
mod tests {
    use super::UtilityProfile;
    use crate::traits::Component;
    use serde::{Serialize, de::DeserializeOwned};
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
        assert_eq!(profile.activity_awareness_weight.value(), 200);
        assert_eq!(profile.side_benefit_weight.value(), 100);
        assert_eq!(profile.bounty_posting_weight.value(), 0);
        assert_eq!(profile.notice_posting_weight.value(), 0);
        assert_eq!(profile.courage.value(), 500);
        assert_eq!(profile.care_weight.value(), 200);
        assert_eq!(profile.office_duty_weight.value(), 500);
        assert_eq!(profile.loyalty_weight.value(), 500);
        assert_eq!(profile.greed_weight.value(), 500);
        assert_eq!(profile.shame_weight.value(), 400);
        assert_eq!(profile.revenge_weight.value(), 400);
        assert!(profile.social_weight < profile.enterprise_weight);
    }

    #[test]
    fn utility_profile_roundtrips_through_bincode() {
        let profile = UtilityProfile {
            social_weight: crate::Permille::new(875).unwrap(),
            activity_awareness_weight: crate::Permille::new(375).unwrap(),
            side_benefit_weight: crate::Permille::new(150).unwrap(),
            bounty_posting_weight: crate::Permille::new(225).unwrap(),
            notice_posting_weight: crate::Permille::new(325).unwrap(),
            courage: crate::Permille::new(125).unwrap(),
            care_weight: crate::Permille::new(750).unwrap(),
            office_duty_weight: crate::Permille::new(625).unwrap(),
            loyalty_weight: crate::Permille::new(675).unwrap(),
            greed_weight: crate::Permille::new(725).unwrap(),
            shame_weight: crate::Permille::new(275).unwrap(),
            revenge_weight: crate::Permille::new(325).unwrap(),
            ..UtilityProfile::default()
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: UtilityProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn utility_profile_serde_defaults_motive_class_weights_when_omitted() {
        let profile: UtilityProfile = ron::from_str(
            r"(
                hunger_weight: (501),
                thirst_weight: (502),
                fatigue_weight: (503),
                bladder_weight: (504),
                dirtiness_weight: (505),
                pain_weight: (506),
                danger_weight: (507),
                enterprise_weight: (508),
                social_weight: (509),
                activity_awareness_weight: (510),
                side_benefit_weight: (511),
                bounty_posting_weight: (512),
                notice_posting_weight: (513),
                courage: (514),
                care_weight: (515),
            )",
        )
        .unwrap();

        assert_eq!(profile.hunger_weight.value(), 501);
        assert_eq!(profile.care_weight.value(), 515);
        assert_eq!(profile.office_duty_weight.value(), 500);
        assert_eq!(profile.loyalty_weight.value(), 500);
        assert_eq!(profile.greed_weight.value(), 500);
        assert_eq!(profile.shame_weight.value(), 400);
        assert_eq!(profile.revenge_weight.value(), 400);
    }
}
