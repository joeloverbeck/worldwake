use crate::{GoalDispatchKey, GoalPlanningBudget, traits::Component};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CandidateExtractorId {
    Need,
    Production,
    Enterprise,
    Disposal,
    Bounty,
    ArtifactPosting,
    Combat,
    Crime,
    Social,
    AskWitness,
    Patrol,
    Political,
    RecordedViolation,
    Search,
    ReportFound,
    Escort,
    Exploration,
    ProactiveExploration,
    ExpectationViolation,
    OpportunityCompiler,
}

impl CandidateExtractorId {
    pub const ALL: [Self; 20] = [
        Self::Need,
        Self::Production,
        Self::Enterprise,
        Self::Disposal,
        Self::Bounty,
        Self::ArtifactPosting,
        Self::Combat,
        Self::Crime,
        Self::Social,
        Self::AskWitness,
        Self::Patrol,
        Self::Political,
        Self::RecordedViolation,
        Self::Search,
        Self::ReportFound,
        Self::Escort,
        Self::Exploration,
        Self::ProactiveExploration,
        Self::ExpectationViolation,
        Self::OpportunityCompiler,
    ];
}

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
        profile
            .disabled_extractors
            .insert(CandidateExtractorId::Patrol);
        profile.budget_overrides.insert(
            GoalDispatchKey::AcquireSelfConsume,
            GoalPlanningBudget::TRAVEL_PURCHASE,
        );

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: AgentSchemaContextProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn candidate_extractor_id_all_covers_variant_set() {
        let all = BTreeSet::from(CandidateExtractorId::ALL);

        assert_eq!(all.len(), 20);
        assert!(all.contains(&CandidateExtractorId::Need));
        assert!(all.contains(&CandidateExtractorId::OpportunityCompiler));
    }
}
