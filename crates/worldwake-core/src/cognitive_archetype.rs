use crate::{BackoffScalePermille, EntityId, MethodSchemaId, StateHash};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CognitiveArchetype {
    Cautious,
    Bold,
    Stubborn,
    Methodical,
    Opportunistic,
    Sociable,
    Skeptical,
    Dutiful,
    Greedy,
    Fearful,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArchetypeProfileTemplate {
    pub archetype: CognitiveArchetype,
    pub max_plan_depth_delta: i8,
    pub repair_budget_fraction_delta: i32,
    pub switch_margin_delta: i32,
    pub planning_switch_margin_delta: i32,
    pub guard_min_confidence_ceiling_delta: i32,
    pub backoff_ticks_scale: BackoffScalePermille,
    pub observation_budget_delta: i8,
    pub testimony_confirmation_weight_delta: i32,
    pub testimony_refutation_penalty_delta: i32,
    pub threat_aversion_delta: i32,
    pub dangerous_traversal_penalty_delta: i32,
    pub ask_memory_retention_ticks_delta: i32,
    pub portfolio_need_survival_delta: i32,
    pub portfolio_pain_care_delta: i32,
    pub portfolio_obligation_duty_delta: i32,
    pub portfolio_economic_opportunity_delta: i32,
    pub portfolio_social_motive_delta: i32,
    pub method_disable: Vec<MethodSchemaId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub enum ArchetypeAssignmentPolicy {
    #[default]
    DefaultUniformFive,
    Uniform(BTreeSet<CognitiveArchetype>),
    Weighted(BTreeMap<CognitiveArchetype, u32>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ArchetypeAssignmentSource {
    Policy(ArchetypeAssignmentPolicy),
    Explicit,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersonalityAssignedPayload {
    pub agent: EntityId,
    pub archetype: CognitiveArchetype,
    pub seed: u64,
    pub source: ArchetypeAssignmentSource,
    pub resolved_profile_hash: StateHash,
}

pub const DEFAULT_UNIFORM_FIVE_ARCHETYPES: [CognitiveArchetype; 5] = [
    CognitiveArchetype::Cautious,
    CognitiveArchetype::Bold,
    CognitiveArchetype::Methodical,
    CognitiveArchetype::Opportunistic,
    CognitiveArchetype::Sociable,
];

#[must_use]
pub fn default_uniform_five_archetypes() -> BTreeSet<CognitiveArchetype> {
    DEFAULT_UNIFORM_FIVE_ARCHETYPES.into_iter().collect()
}

#[must_use]
pub fn template_for(archetype: CognitiveArchetype) -> ArchetypeProfileTemplate {
    match archetype {
        CognitiveArchetype::Cautious => cautious_template(),
        CognitiveArchetype::Bold => bold_template(),
        CognitiveArchetype::Stubborn => stubborn_template(),
        CognitiveArchetype::Methodical => methodical_template(),
        CognitiveArchetype::Opportunistic => opportunistic_template(),
        CognitiveArchetype::Sociable => sociable_template(),
        CognitiveArchetype::Skeptical => skeptical_template(),
        CognitiveArchetype::Dutiful => dutiful_template(),
        CognitiveArchetype::Greedy => greedy_template(),
        CognitiveArchetype::Fearful => fearful_template(),
    }
}

#[must_use]
pub fn cautious_template() -> ArchetypeProfileTemplate {
    ArchetypeProfileTemplate {
        archetype: CognitiveArchetype::Cautious,
        max_plan_depth_delta: 2,
        repair_budget_fraction_delta: 100,
        switch_margin_delta: 100,
        planning_switch_margin_delta: 100,
        guard_min_confidence_ceiling_delta: -100,
        backoff_ticks_scale: BackoffScalePermille::new_unchecked(1500),
        observation_budget_delta: 1,
        testimony_confirmation_weight_delta: -50,
        testimony_refutation_penalty_delta: 100,
        threat_aversion_delta: 200,
        dangerous_traversal_penalty_delta: 150,
        ask_memory_retention_ticks_delta: -4,
        portfolio_need_survival_delta: 50,
        portfolio_pain_care_delta: 0,
        portfolio_obligation_duty_delta: 0,
        portfolio_economic_opportunity_delta: -50,
        portfolio_social_motive_delta: 0,
        method_disable: Vec::new(),
    }
}

#[must_use]
pub fn bold_template() -> ArchetypeProfileTemplate {
    ArchetypeProfileTemplate {
        archetype: CognitiveArchetype::Bold,
        max_plan_depth_delta: -1,
        repair_budget_fraction_delta: -75,
        switch_margin_delta: -75,
        planning_switch_margin_delta: -75,
        guard_min_confidence_ceiling_delta: 75,
        backoff_ticks_scale: BackoffScalePermille::new_unchecked(600),
        observation_budget_delta: -1,
        testimony_confirmation_weight_delta: 50,
        testimony_refutation_penalty_delta: -50,
        threat_aversion_delta: -175,
        dangerous_traversal_penalty_delta: -125,
        ask_memory_retention_ticks_delta: 4,
        portfolio_need_survival_delta: -25,
        portfolio_pain_care_delta: 0,
        portfolio_obligation_duty_delta: -25,
        portfolio_economic_opportunity_delta: 75,
        portfolio_social_motive_delta: 0,
        method_disable: Vec::new(),
    }
}

#[must_use]
pub fn stubborn_template() -> ArchetypeProfileTemplate {
    ArchetypeProfileTemplate {
        archetype: CognitiveArchetype::Stubborn,
        max_plan_depth_delta: 1,
        repair_budget_fraction_delta: 75,
        switch_margin_delta: 175,
        planning_switch_margin_delta: 175,
        guard_min_confidence_ceiling_delta: 25,
        backoff_ticks_scale: BackoffScalePermille::new_unchecked(1250),
        observation_budget_delta: 0,
        testimony_confirmation_weight_delta: -50,
        testimony_refutation_penalty_delta: 125,
        threat_aversion_delta: 25,
        dangerous_traversal_penalty_delta: 25,
        ask_memory_retention_ticks_delta: 8,
        portfolio_need_survival_delta: 0,
        portfolio_pain_care_delta: 0,
        portfolio_obligation_duty_delta: 50,
        portfolio_economic_opportunity_delta: 0,
        portfolio_social_motive_delta: -50,
        method_disable: Vec::new(),
    }
}

#[must_use]
pub fn methodical_template() -> ArchetypeProfileTemplate {
    ArchetypeProfileTemplate {
        archetype: CognitiveArchetype::Methodical,
        max_plan_depth_delta: 2,
        repair_budget_fraction_delta: 125,
        switch_margin_delta: 75,
        planning_switch_margin_delta: 100,
        guard_min_confidence_ceiling_delta: -25,
        backoff_ticks_scale: BackoffScalePermille::IDENTITY,
        observation_budget_delta: 1,
        testimony_confirmation_weight_delta: 0,
        testimony_refutation_penalty_delta: 50,
        threat_aversion_delta: 50,
        dangerous_traversal_penalty_delta: 50,
        ask_memory_retention_ticks_delta: -2,
        portfolio_need_survival_delta: 0,
        portfolio_pain_care_delta: 25,
        portfolio_obligation_duty_delta: 25,
        portfolio_economic_opportunity_delta: -25,
        portfolio_social_motive_delta: -25,
        method_disable: Vec::new(),
    }
}

#[must_use]
pub fn opportunistic_template() -> ArchetypeProfileTemplate {
    ArchetypeProfileTemplate {
        archetype: CognitiveArchetype::Opportunistic,
        max_plan_depth_delta: 0,
        repair_budget_fraction_delta: -50,
        switch_margin_delta: -100,
        planning_switch_margin_delta: -75,
        guard_min_confidence_ceiling_delta: 50,
        backoff_ticks_scale: BackoffScalePermille::new_unchecked(750),
        observation_budget_delta: 0,
        testimony_confirmation_weight_delta: 50,
        testimony_refutation_penalty_delta: -25,
        threat_aversion_delta: -75,
        dangerous_traversal_penalty_delta: -75,
        ask_memory_retention_ticks_delta: 0,
        portfolio_need_survival_delta: -25,
        portfolio_pain_care_delta: -25,
        portfolio_obligation_duty_delta: -50,
        portfolio_economic_opportunity_delta: 150,
        portfolio_social_motive_delta: -50,
        method_disable: Vec::new(),
    }
}

#[must_use]
pub fn sociable_template() -> ArchetypeProfileTemplate {
    ArchetypeProfileTemplate {
        archetype: CognitiveArchetype::Sociable,
        max_plan_depth_delta: 0,
        repair_budget_fraction_delta: 0,
        switch_margin_delta: -25,
        planning_switch_margin_delta: -25,
        guard_min_confidence_ceiling_delta: 0,
        backoff_ticks_scale: BackoffScalePermille::new_unchecked(900),
        observation_budget_delta: 1,
        testimony_confirmation_weight_delta: 125,
        testimony_refutation_penalty_delta: -50,
        threat_aversion_delta: 0,
        dangerous_traversal_penalty_delta: 0,
        ask_memory_retention_ticks_delta: -8,
        portfolio_need_survival_delta: -25,
        portfolio_pain_care_delta: 0,
        portfolio_obligation_duty_delta: 25,
        portfolio_economic_opportunity_delta: -25,
        portfolio_social_motive_delta: 125,
        method_disable: Vec::new(),
    }
}

#[must_use]
pub fn skeptical_template() -> ArchetypeProfileTemplate {
    ArchetypeProfileTemplate {
        archetype: CognitiveArchetype::Skeptical,
        max_plan_depth_delta: 1,
        repair_budget_fraction_delta: 50,
        switch_margin_delta: 100,
        planning_switch_margin_delta: 75,
        guard_min_confidence_ceiling_delta: -125,
        backoff_ticks_scale: BackoffScalePermille::new_unchecked(1200),
        observation_budget_delta: 1,
        testimony_confirmation_weight_delta: -125,
        testimony_refutation_penalty_delta: 150,
        threat_aversion_delta: 75,
        dangerous_traversal_penalty_delta: 75,
        ask_memory_retention_ticks_delta: -6,
        portfolio_need_survival_delta: 25,
        portfolio_pain_care_delta: 0,
        portfolio_obligation_duty_delta: 0,
        portfolio_economic_opportunity_delta: -50,
        portfolio_social_motive_delta: 25,
        method_disable: Vec::new(),
    }
}

#[must_use]
pub fn dutiful_template() -> ArchetypeProfileTemplate {
    ArchetypeProfileTemplate {
        archetype: CognitiveArchetype::Dutiful,
        max_plan_depth_delta: 1,
        repair_budget_fraction_delta: 75,
        switch_margin_delta: 125,
        planning_switch_margin_delta: 100,
        guard_min_confidence_ceiling_delta: -25,
        backoff_ticks_scale: BackoffScalePermille::new_unchecked(1100),
        observation_budget_delta: 0,
        testimony_confirmation_weight_delta: 25,
        testimony_refutation_penalty_delta: 25,
        threat_aversion_delta: 25,
        dangerous_traversal_penalty_delta: 25,
        ask_memory_retention_ticks_delta: 0,
        portfolio_need_survival_delta: 0,
        portfolio_pain_care_delta: 25,
        portfolio_obligation_duty_delta: 150,
        portfolio_economic_opportunity_delta: -75,
        portfolio_social_motive_delta: 0,
        method_disable: Vec::new(),
    }
}

#[must_use]
pub fn greedy_template() -> ArchetypeProfileTemplate {
    ArchetypeProfileTemplate {
        archetype: CognitiveArchetype::Greedy,
        max_plan_depth_delta: 0,
        repair_budget_fraction_delta: -25,
        switch_margin_delta: -50,
        planning_switch_margin_delta: -50,
        guard_min_confidence_ceiling_delta: 75,
        backoff_ticks_scale: BackoffScalePermille::new_unchecked(850),
        observation_budget_delta: 0,
        testimony_confirmation_weight_delta: 25,
        testimony_refutation_penalty_delta: -25,
        threat_aversion_delta: -50,
        dangerous_traversal_penalty_delta: -50,
        ask_memory_retention_ticks_delta: 2,
        portfolio_need_survival_delta: -25,
        portfolio_pain_care_delta: -25,
        portfolio_obligation_duty_delta: -100,
        portfolio_economic_opportunity_delta: 200,
        portfolio_social_motive_delta: -50,
        method_disable: Vec::new(),
    }
}

#[must_use]
pub fn fearful_template() -> ArchetypeProfileTemplate {
    ArchetypeProfileTemplate {
        archetype: CognitiveArchetype::Fearful,
        max_plan_depth_delta: 1,
        repair_budget_fraction_delta: 75,
        switch_margin_delta: 75,
        planning_switch_margin_delta: 75,
        guard_min_confidence_ceiling_delta: -150,
        backoff_ticks_scale: BackoffScalePermille::new_unchecked(1750),
        observation_budget_delta: 1,
        testimony_confirmation_weight_delta: -25,
        testimony_refutation_penalty_delta: 75,
        threat_aversion_delta: 250,
        dangerous_traversal_penalty_delta: 225,
        ask_memory_retention_ticks_delta: -6,
        portfolio_need_survival_delta: 100,
        portfolio_pain_care_delta: 50,
        portfolio_obligation_duty_delta: -25,
        portfolio_economic_opportunity_delta: -100,
        portfolio_social_motive_delta: -25,
        method_disable: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_copy_bounds<
        T: Copy + Clone + Eq + Ord + std::hash::Hash + Debug + Serialize + DeserializeOwned,
    >() {
    }

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn cognitive_archetype_satisfies_required_traits() {
        assert_copy_bounds::<CognitiveArchetype>();
        assert_value_bounds::<ArchetypeProfileTemplate>();
        assert_value_bounds::<ArchetypeAssignmentPolicy>();
        assert_value_bounds::<ArchetypeAssignmentSource>();
        assert_value_bounds::<PersonalityAssignedPayload>();
    }

    #[test]
    fn template_for_returns_matching_archetype() {
        let archetypes = [
            CognitiveArchetype::Cautious,
            CognitiveArchetype::Bold,
            CognitiveArchetype::Stubborn,
            CognitiveArchetype::Methodical,
            CognitiveArchetype::Opportunistic,
            CognitiveArchetype::Sociable,
            CognitiveArchetype::Skeptical,
            CognitiveArchetype::Dutiful,
            CognitiveArchetype::Greedy,
            CognitiveArchetype::Fearful,
        ];

        for archetype in archetypes {
            assert_eq!(template_for(archetype).archetype, archetype);
        }
    }

    #[test]
    fn template_functions_are_deterministic() {
        assert_eq!(cautious_template(), cautious_template());
        assert_eq!(bold_template(), bold_template());
        assert_eq!(stubborn_template(), stubborn_template());
        assert_eq!(methodical_template(), methodical_template());
        assert_eq!(opportunistic_template(), opportunistic_template());
        assert_eq!(sociable_template(), sociable_template());
        assert_eq!(skeptical_template(), skeptical_template());
        assert_eq!(dutiful_template(), dutiful_template());
        assert_eq!(greedy_template(), greedy_template());
        assert_eq!(fearful_template(), fearful_template());
    }

    #[test]
    fn default_uniform_five_policy_and_set_match_contract() {
        assert_eq!(
            ArchetypeAssignmentPolicy::default(),
            ArchetypeAssignmentPolicy::DefaultUniformFive
        );
        assert_eq!(
            default_uniform_five_archetypes(),
            BTreeSet::from([
                CognitiveArchetype::Cautious,
                CognitiveArchetype::Bold,
                CognitiveArchetype::Methodical,
                CognitiveArchetype::Opportunistic,
                CognitiveArchetype::Sociable,
            ])
        );
    }

    #[test]
    fn personality_assigned_payload_roundtrips_through_bincode() {
        let payload = PersonalityAssignedPayload {
            agent: EntityId {
                slot: 7,
                generation: 2,
            },
            archetype: CognitiveArchetype::Skeptical,
            seed: 99,
            source: ArchetypeAssignmentSource::Policy(ArchetypeAssignmentPolicy::Weighted(
                BTreeMap::from([
                    (CognitiveArchetype::Skeptical, 3),
                    (CognitiveArchetype::Sociable, 1),
                ]),
            )),
            resolved_profile_hash: StateHash([4; 32]),
        };

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: PersonalityAssignedPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }
}
