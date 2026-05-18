use crate::{
    BeliefStatusTag, EntityId, FrameAssumption, MotiveSourceDiscriminant, OpportunityAnchor,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum IntentionResumeCondition {
    /// Belief about an entity transitioned to a specific status.
    BeliefStatusChanged {
        subject: EntityId,
        target_status: BeliefStatusTag,
    },
    /// A specific opportunity became visible to the agent again.
    OpportunityVisible(OpportunityAnchor),
    /// The agent reached a specific place.
    LocationReached(EntityId),
    /// Resume after this many ticks have elapsed since suspension.
    TickElapsed(u32),
    /// Artifact legal effect transitioned to active.
    ArtifactLegalEffectActive(EntityId),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum IntentionAbandonCondition {
    /// The motive that produced this intention is no longer present.
    MotiveSourceLost(MotiveSourceDiscriminant),
    /// A frame assumption has been broken in a way that cannot recover.
    AssumptionPermanentlyBroken(FrameAssumption),
    /// The opportunity this intention targeted is gone.
    OpportunityForeverGone(OpportunityAnchor),
    /// `stalled_ticks` reached `patience_limit`.
    PatienceExhausted,
    /// An explicit-claim artifact transitioned to destroyed.
    ArtifactDestroyed(EntityId),
    /// An explicit-claim artifact's legal effect transitioned out of active.
    ArtifactLegalEffectLost(EntityId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum IntentionAbandonConditionDiscriminant {
    MotiveSourceLost,
    AssumptionPermanentlyBroken,
    OpportunityForeverGone,
    PatienceExhausted,
    ArtifactDestroyed,
    ArtifactLegalEffectLost,
}

impl From<&IntentionAbandonCondition> for IntentionAbandonConditionDiscriminant {
    fn from(condition: &IntentionAbandonCondition) -> Self {
        match condition {
            IntentionAbandonCondition::MotiveSourceLost(_) => Self::MotiveSourceLost,
            IntentionAbandonCondition::AssumptionPermanentlyBroken(_) => {
                Self::AssumptionPermanentlyBroken
            }
            IntentionAbandonCondition::OpportunityForeverGone(_) => Self::OpportunityForeverGone,
            IntentionAbandonCondition::PatienceExhausted => Self::PatienceExhausted,
            IntentionAbandonCondition::ArtifactDestroyed(_) => Self::ArtifactDestroyed,
            IntentionAbandonCondition::ArtifactLegalEffectLost(_) => Self::ArtifactLegalEffectLost,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        IntentionAbandonCondition, IntentionAbandonConditionDiscriminant, IntentionResumeCondition,
    };
    use crate::{
        BeliefStatusTag, EntityId, FrameAssumption, MotiveSourceDiscriminant, OpportunityAnchor,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn roundtrip<T>(value: &T) -> T
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        let bytes = bincode::serialize(value).unwrap();
        bincode::deserialize(&bytes).unwrap()
    }

    #[test]
    fn resume_conditions_roundtrip_through_bincode() {
        let cases = [
            IntentionResumeCondition::BeliefStatusChanged {
                subject: entity(1),
                target_status: BeliefStatusTag::Certain,
            },
            IntentionResumeCondition::OpportunityVisible(OpportunityAnchor::Entity(entity(2))),
            IntentionResumeCondition::LocationReached(entity(3)),
            IntentionResumeCondition::TickElapsed(5),
            IntentionResumeCondition::ArtifactLegalEffectActive(entity(4)),
        ];

        for case in cases {
            assert_eq!(roundtrip(&case), case);
        }
    }

    #[test]
    fn abandon_conditions_roundtrip_through_bincode() {
        let cases = [
            IntentionAbandonCondition::MotiveSourceLost(MotiveSourceDiscriminant::Greed),
            IntentionAbandonCondition::AssumptionPermanentlyBroken(FrameAssumption::RouteExists {
                from: entity(1),
                to: entity(2),
            }),
            IntentionAbandonCondition::OpportunityForeverGone(OpportunityAnchor::Place(entity(9))),
            IntentionAbandonCondition::PatienceExhausted,
            IntentionAbandonCondition::ArtifactDestroyed(entity(3)),
            IntentionAbandonCondition::ArtifactLegalEffectLost(entity(4)),
        ];

        for case in cases {
            assert_eq!(roundtrip(&case), case);
        }
    }

    #[test]
    fn resume_condition_ordering_is_stable() {
        let mut conditions = [
            IntentionResumeCondition::TickElapsed(3),
            IntentionResumeCondition::LocationReached(entity(2)),
            IntentionResumeCondition::BeliefStatusChanged {
                subject: entity(1),
                target_status: BeliefStatusTag::Stale,
            },
        ];

        conditions.sort();

        assert_eq!(
            conditions,
            [
                IntentionResumeCondition::BeliefStatusChanged {
                    subject: entity(1),
                    target_status: BeliefStatusTag::Stale,
                },
                IntentionResumeCondition::LocationReached(entity(2)),
                IntentionResumeCondition::TickElapsed(3),
            ]
        );
    }

    #[test]
    fn abandon_condition_ordering_is_stable() {
        let mut conditions = [
            IntentionAbandonCondition::PatienceExhausted,
            IntentionAbandonCondition::MotiveSourceLost(MotiveSourceDiscriminant::Pain),
            IntentionAbandonCondition::ArtifactDestroyed(entity(1)),
        ];

        conditions.sort();

        assert_eq!(
            conditions,
            [
                IntentionAbandonCondition::MotiveSourceLost(MotiveSourceDiscriminant::Pain),
                IntentionAbandonCondition::PatienceExhausted,
                IntentionAbandonCondition::ArtifactDestroyed(entity(1)),
            ]
        );
    }

    #[test]
    fn abandon_condition_discriminant_mirrors_every_variant() {
        let cases = [
            (
                IntentionAbandonCondition::MotiveSourceLost(MotiveSourceDiscriminant::Greed),
                IntentionAbandonConditionDiscriminant::MotiveSourceLost,
            ),
            (
                IntentionAbandonCondition::AssumptionPermanentlyBroken(
                    FrameAssumption::RouteExists {
                        from: entity(1),
                        to: entity(2),
                    },
                ),
                IntentionAbandonConditionDiscriminant::AssumptionPermanentlyBroken,
            ),
            (
                IntentionAbandonCondition::OpportunityForeverGone(OpportunityAnchor::Place(
                    entity(9),
                )),
                IntentionAbandonConditionDiscriminant::OpportunityForeverGone,
            ),
            (
                IntentionAbandonCondition::PatienceExhausted,
                IntentionAbandonConditionDiscriminant::PatienceExhausted,
            ),
            (
                IntentionAbandonCondition::ArtifactDestroyed(entity(3)),
                IntentionAbandonConditionDiscriminant::ArtifactDestroyed,
            ),
            (
                IntentionAbandonCondition::ArtifactLegalEffectLost(entity(4)),
                IntentionAbandonConditionDiscriminant::ArtifactLegalEffectLost,
            ),
        ];

        for (condition, expected) in cases {
            assert_eq!(
                IntentionAbandonConditionDiscriminant::from(&condition),
                expected
            );
        }
    }
}
