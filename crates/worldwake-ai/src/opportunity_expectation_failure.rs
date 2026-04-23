use worldwake_core::{OpportunityKey, SourceKey, Tick};

use crate::OpportunityExpectationKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ExpectationFailurePhase {
    Observation,
    CandidateGeneration,
    Search,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ExpectationFailureCause {
    SourceAbsentLocally,
    SourceDepletedLocally,
    SameGoalSearchInfeasibleWhileSiblingSucceeded,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct OpportunityExpectationFailureIncident {
    pub opportunity: OpportunityKey,
    pub source: SourceKey,
    pub expectation_kind: OpportunityExpectationKind,
    pub detected_at_tick: Tick,
    pub phase: ExpectationFailurePhase,
    pub cause: ExpectationFailureCause,
}
