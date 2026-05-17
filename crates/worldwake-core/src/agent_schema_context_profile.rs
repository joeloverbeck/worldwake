use crate::{GoalDispatchKey, GoalPlanningBudget, traits::Component};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct CandidateExtractorId(pub u16);

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct AgentSchemaContextProfile {
    pub disabled_extractors: BTreeSet<CandidateExtractorId>,
    pub budget_overrides: BTreeMap<GoalDispatchKey, GoalPlanningBudget>,
}

impl Component for AgentSchemaContextProfile {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty() {
        let profile = AgentSchemaContextProfile::default();

        assert!(profile.disabled_extractors.is_empty());
        assert!(profile.budget_overrides.is_empty());
    }

    #[test]
    fn serde_roundtrip_preserves_overrides() {
        let mut profile = AgentSchemaContextProfile::default();
        profile.disabled_extractors.insert(CandidateExtractorId(7));
        profile.budget_overrides.insert(
            GoalDispatchKey::AcquireSelfConsume,
            GoalPlanningBudget::TRAVEL_PURCHASE,
        );

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: AgentSchemaContextProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }
}
