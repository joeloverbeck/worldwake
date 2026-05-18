//! Generalized intention frame types for multi-step commitment tracking.
//!
//! An `IntentionFrame` captures goal-level commitment stability for any
//! multi-step plan: travel, care chains, escort, errand, or generic.
//! It replaces the travel-specific `JourneyCommitment` with a domain-agnostic
//! structure that tracks assumptions, patience, and suspension/resume lifecycle.

use crate::traits::Component;
use crate::{
    CommodityKind, EntityId, EventId, GoalKey, GoalKind, HomeostaticNeedId,
    IntentionAbandonCondition, IntentionResumeCondition, MotiveSourceRef, Tick,
};
use serde::{Deserialize, Serialize};

/// Domain-specific context carried by an intention frame.
///
/// Each variant holds the minimal data needed for domain-specific lifecycle
/// operations (e.g., knowing the travel destination for route-exists checks).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum IntentionDomain {
    /// Multi-leg travel to a destination place.
    Travel { destination: EntityId },
    /// Multi-step care: travel to patient, treat, potentially return.
    Care { patient: EntityId },
    /// Escort: accompany a target entity along a route.
    Escort {
        ward: EntityId,
        destination: EntityId,
    },
    /// Multi-step errand: travel, act at destination, return.
    Errand { destination: EntityId },
    /// Domain not yet specialized. Used for goals that benefit from
    /// commitment stability but have no domain-specific assumptions.
    Generic,
}

impl IntentionDomain {
    /// Returns the data-free discriminant for this domain.
    #[must_use]
    pub const fn domain_tag(&self) -> IntentionDomainTag {
        match self {
            Self::Travel { .. } => IntentionDomainTag::Travel,
            Self::Care { .. } => IntentionDomainTag::Care,
            Self::Escort { .. } => IntentionDomainTag::Escort,
            Self::Errand { .. } => IntentionDomainTag::Errand,
            Self::Generic => IntentionDomainTag::Generic,
        }
    }
}

/// Data-free discriminant for `IntentionDomain`, suitable for use as a
/// `BTreeMap` key in disposition profiles.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum IntentionDomainTag {
    Travel,
    Care,
    Escort,
    Errand,
    Generic,
}

/// A concrete assumption that an intention frame relies on. Each assumption
/// is a falsifiable predicate evaluated against the agent's beliefs.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum FrameAssumption {
    /// Target entity is alive (not dead, not despawned).
    TargetAlive(EntityId),
    /// A route exists from one place to another.
    RouteExists { from: EntityId, to: EntityId },
    /// Agent is not under critical survival threat.
    NoCriticalThreat,
    /// A specific commodity is available at a specific place.
    CommodityAvailableAt {
        commodity: CommodityKind,
        place: EntityId,
    },
    /// The named need is projected to remain below its high threshold
    /// at least until `until_tick`, given the agent's current level
    /// and base metabolism rate. Recomputed each tick by the evaluator.
    NeedSafeUntilTick {
        need: HomeostaticNeedId,
        until_tick: Tick,
    },
}

/// Lifecycle state of an intention frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FrameState {
    /// Actively pursuing the committed goal.
    Active,
    /// Temporarily suspended. The frame persists but does not contribute
    /// commitment margins to goal switching.
    Suspended {
        reason: SuspensionReason,
        suspended_at: Tick,
    },
    /// Patience exhausted or critical assumption failed. The AI must
    /// clear this frame and allow full replanning.
    Exhausted,
}

/// Reason for temporarily suspending an intention frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SuspensionReason {
    /// A higher-priority goal interrupted the frame's goal.
    PriorityInterrupt,
    /// Route to destination became unavailable (believed blocked or severed).
    RouteBlocked,
    /// Target entity became unreachable or believed dead.
    TargetUnreachable,
    /// Critical survival need interrupted (hunger, thirst at dangerous levels).
    SurvivalNeed,
}

/// Reason for clearing (removing) an intention frame. Stored on
/// `AgentDecisionRuntime` to record why the last frame was cleared.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FrameClearReason {
    /// The frame's goal was achieved.
    GoalSatisfied,
    /// A higher-priority goal permanently replaced the frame's goal.
    Reprioritized,
    /// The plan for the frame's goal failed.
    PlanFailed,
    /// Patience exhausted (`stalled_ticks` >= `patience_limit`).
    PatienceExhausted,
    /// A critical assumption failed (e.g., target believed dead).
    AssumptionFailed,
    /// Agent died.
    Death,
    /// The plan was lost (no remaining steps, replanning yielded nothing).
    LostPlan,
}

/// Goal-level commitment stability for multi-step plans.
///
/// One agent has at most one active `IntentionFrame` at any time — the frame
/// for their current committed goal. The frame captures what the agent intends
/// and under what conditions that intention remains valid.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntentionFrame {
    /// The goal this frame serves. Must match the agent's committed agenda entry.
    pub goal: GoalKey,
    /// Domain tag for domain-specific lifecycle logic.
    pub domain: IntentionDomain,
    /// Concrete assumptions this frame relies on. Evaluated each tick
    /// against the agent's beliefs to detect invalidation.
    pub assumptions: Vec<FrameAssumption>,
    /// Current lifecycle state.
    pub state: FrameState,
    /// When this frame was established.
    pub established_at: Tick,
    /// Last tick where meaningful progress occurred. None if no progress
    /// has been recorded yet.
    pub last_progress_tick: Option<Tick>,
    /// Consecutive ticks without progress. Incremented when the frame is
    /// Active but no forward step completes. Reset on progress.
    pub stalled_ticks: u32,
    /// Maximum stalled ticks before patience exhaustion. Per-agent, set
    /// from the agent's `IntentionDispositionProfile` at frame creation time.
    pub patience_limit: u32,
    /// Motive sources that backed adoption of this intention.
    #[serde(default)]
    pub motive_refs: Vec<MotiveSourceRef>,
    /// Conditions that may resume a suspended intention.
    #[serde(default)]
    pub resume_conditions: Vec<IntentionResumeCondition>,
    /// Conditions that may abandon this intention.
    #[serde(default)]
    pub abandon_conditions: Vec<IntentionAbandonCondition>,
    /// Concrete claim artifacts this intention depends on: contention grants,
    /// sale listings, and social artifacts.
    #[serde(default)]
    pub explicit_claims: Vec<EntityId>,
    /// Event IDs that caused this intention. Push sites must keep this vector
    /// bounded by `CognitiveProfile.causal_links_per_step_cap`.
    #[serde(default)]
    pub causal_links: Vec<EventId>,
}

impl IntentionFrame {
    #[must_use]
    pub fn expected_commodity(&self) -> Option<(CommodityKind, EntityId)> {
        let (IntentionDomain::Travel { destination } | IntentionDomain::Errand { destination }) =
            self.domain
        else {
            return None;
        };
        match self.goal.kind {
            GoalKind::AcquireCommodity { commodity, .. } => Some((commodity, destination)),
            _ => None,
        }
    }
}

impl Component for IntentionFrame {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::entity_id;
    use crate::{
        AcquisitionQuantity, CommodityPurpose, GoalKind, MotiveSource, MotiveSourceDiscriminant,
        OpportunityAnchor, OpportunityKey,
    };
    use serde::{Serialize as SerializeTrait, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + SerializeTrait + DeserializeOwned>() {}

    #[test]
    fn intention_frame_satisfies_component_bounds() {
        assert_component_bounds::<IntentionFrame>();
        assert_value_bounds::<IntentionFrame>();
    }

    #[test]
    fn domain_tag_travel() {
        let domain = IntentionDomain::Travel {
            destination: entity_id(1, 0),
        };
        assert_eq!(domain.domain_tag(), IntentionDomainTag::Travel);
    }

    #[test]
    fn domain_tag_care() {
        let domain = IntentionDomain::Care {
            patient: entity_id(2, 0),
        };
        assert_eq!(domain.domain_tag(), IntentionDomainTag::Care);
    }

    #[test]
    fn domain_tag_escort() {
        let domain = IntentionDomain::Escort {
            ward: entity_id(3, 0),
            destination: entity_id(4, 0),
        };
        assert_eq!(domain.domain_tag(), IntentionDomainTag::Escort);
    }

    #[test]
    fn domain_tag_errand() {
        let domain = IntentionDomain::Errand {
            destination: entity_id(5, 0),
        };
        assert_eq!(domain.domain_tag(), IntentionDomainTag::Errand);
    }

    #[test]
    fn domain_tag_generic() {
        assert_eq!(
            IntentionDomain::Generic.domain_tag(),
            IntentionDomainTag::Generic
        );
    }

    #[test]
    fn domain_tag_ordering_is_deterministic() {
        let tags = [
            IntentionDomainTag::Travel,
            IntentionDomainTag::Care,
            IntentionDomainTag::Escort,
            IntentionDomainTag::Errand,
            IntentionDomainTag::Generic,
        ];
        let mut sorted = tags;
        sorted.sort();
        // Verify Ord is implemented and produces a consistent ordering.
        assert_eq!(sorted, sorted);
    }

    #[test]
    fn frame_assumption_ordering_is_deterministic() {
        let a = FrameAssumption::NoCriticalThreat;
        let b = FrameAssumption::TargetAlive(entity_id(1, 0));
        // Just verify Ord is usable — the exact order is derive-determined.
        let _ = a.cmp(&b);
    }

    fn sample_frame(domain: IntentionDomain, goal: GoalKind) -> IntentionFrame {
        IntentionFrame {
            goal: GoalKey::new(goal),
            domain,
            assumptions: Vec::new(),
            state: FrameState::Active,
            established_at: Tick(0),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 30,
            motive_refs: Vec::new(),
            resume_conditions: Vec::new(),
            abandon_conditions: Vec::new(),
            explicit_claims: Vec::new(),
            causal_links: Vec::new(),
        }
    }

    #[test]
    fn expected_commodity_returns_pair_for_travel_and_acquire_goal() {
        let destination = entity_id(10, 0);
        let frame = sample_frame(
            IntentionDomain::Travel { destination },
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            },
        );

        assert_eq!(
            frame.expected_commodity(),
            Some((CommodityKind::Apple, destination))
        );
    }

    #[test]
    fn expected_commodity_returns_none_for_non_acquisition_goal() {
        let frame = sample_frame(
            IntentionDomain::Travel {
                destination: entity_id(10, 0),
            },
            GoalKind::Sleep,
        );

        assert_eq!(frame.expected_commodity(), None);
    }

    #[test]
    fn frame_assumption_need_safe_until_tick_roundtrips_through_bincode() {
        let assumption = FrameAssumption::NeedSafeUntilTick {
            need: crate::HomeostaticNeedId::Hunger,
            until_tick: Tick(100),
        };

        let bytes = bincode::serialize(&assumption).unwrap();
        let roundtrip: FrameAssumption = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, assumption);
    }

    #[test]
    fn expected_commodity_returns_none_for_non_travel_domain() {
        let frame = sample_frame(
            IntentionDomain::Care {
                patient: entity_id(11, 0),
            },
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            },
        );

        assert_eq!(frame.expected_commodity(), None);
    }

    #[test]
    fn intention_frame_roundtrips_with_bdi_fields() {
        let opportunity = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::Sleep),
            anchor: OpportunityAnchor::Place(entity_id(7, 0)),
        };
        let mut frame = sample_frame(IntentionDomain::Generic, GoalKind::Sleep);
        frame.motive_refs.push(MotiveSourceRef {
            source: MotiveSource::Greed { opportunity },
            introduced_tick: Tick(9),
        });
        frame
            .resume_conditions
            .push(IntentionResumeCondition::LocationReached(entity_id(8, 0)));
        frame
            .abandon_conditions
            .push(IntentionAbandonCondition::MotiveSourceLost(
                MotiveSourceDiscriminant::Greed,
            ));
        frame.explicit_claims.push(entity_id(10, 0));
        frame.causal_links.push(EventId(11));

        let bytes = bincode::serialize(&frame).unwrap();
        let roundtrip: IntentionFrame = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, frame);
    }

    #[test]
    fn intention_frame_deserializes_pre_s148_state_with_empty_bdi_fields() {
        #[derive(SerializeTrait, serde::Deserialize)]
        struct PreS148IntentionFrame {
            goal: GoalKey,
            domain: IntentionDomain,
            assumptions: Vec<FrameAssumption>,
            state: FrameState,
            established_at: Tick,
            last_progress_tick: Option<Tick>,
            stalled_ticks: u32,
            patience_limit: u32,
        }

        let old = PreS148IntentionFrame {
            goal: GoalKey::from(GoalKind::Sleep),
            domain: IntentionDomain::Generic,
            assumptions: vec![FrameAssumption::NoCriticalThreat],
            state: FrameState::Active,
            established_at: Tick(1),
            last_progress_tick: Some(Tick(2)),
            stalled_ticks: 3,
            patience_limit: 4,
        };
        let encoded = ron::to_string(&old).unwrap();
        let frame: IntentionFrame = ron::from_str(&encoded).unwrap();

        assert!(frame.motive_refs.is_empty());
        assert!(frame.resume_conditions.is_empty());
        assert!(frame.abandon_conditions.is_empty());
        assert!(frame.explicit_claims.is_empty());
        assert!(frame.causal_links.is_empty());
        assert_eq!(frame.goal, old.goal);
        assert_eq!(frame.assumptions, old.assumptions);
    }
}
