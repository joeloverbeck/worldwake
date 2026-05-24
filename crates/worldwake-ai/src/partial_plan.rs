use crate::{
    GoalOffer, PlanTerminalKind, PlannedStep, PlannerOpKind,
    htn::{
        BeliefPredicate, EntityTemplate, LocationTemplate, PayloadTemplate, PayloadValueTemplate,
    },
};
use serde::{Deserialize, Serialize};
use worldwake_core::{
    AffordanceKey, BeliefStatusTag, Blocker, BlockerClearingCondition, BlockerMemory, BlockerScope,
    BlockingFact, CognitiveProfile, CommodityKind, Discrepancy, EntityId, EventId,
    IntentionAbandonCondition, IntentionResumeCondition, TellTopic, Tick,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct PartialPlanSegmentId {
    pub created_tick: Tick,
    pub local_counter: u16,
}

impl PartialPlanSegmentId {
    #[must_use]
    pub const fn new(created_tick: Tick, local_counter: u16) -> Self {
        Self {
            created_tick,
            local_counter,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PartialPlanSegment {
    pub id: PartialPlanSegmentId,
    pub goal: GoalOffer,
    pub completed_prefix: Vec<PlannedStep>,
    pub remaining_skeleton: Option<Vec<PlannedSkeletonStep>>,
    pub terminal_barrier: PlanTerminalKind,
    pub barrier_fact: BarrierFact,
    pub resume_conditions: Vec<IntentionResumeCondition>,
    pub abandon_conditions: Vec<IntentionAbandonCondition>,
    pub created_tick: Tick,
    pub last_resume_attempt_tick: Option<Tick>,
    pub resume_attempt_count: u8,
    pub causal_links: Vec<EventId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlannedSkeletonStep {
    pub op: PlannerOpKind,
    pub target_template: PayloadTemplate,
    pub expected_pre: Vec<BeliefPredicate>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BarrierFact {
    MissingBelief(BeliefPredicate),
    ContestedReservation(EntityId),
    DepletedResource {
        commodity: CommodityKind,
        place: EntityId,
    },
    NoAuthorityForAction(EntityId),
    BudgetExhausted {
        remaining_stages: u16,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartialPlanSegmentSeed {
    pub goal: GoalOffer,
    pub completed_prefix: Vec<PlannedStep>,
    pub remaining_skeleton: Option<Vec<PlannedSkeletonStep>>,
    pub terminal_barrier: PlanTerminalKind,
    pub barrier_fact: BarrierFact,
    pub created_tick: Tick,
    pub local_counter: u16,
    pub causal_links: Vec<EventId>,
}

#[must_use]
pub fn build_partial_plan_segment(
    seed: PartialPlanSegmentSeed,
    cognitive: &CognitiveProfile,
) -> Option<PartialPlanSegment> {
    if matches!(
        seed.terminal_barrier,
        PlanTerminalKind::GoalSatisfied | PlanTerminalKind::CombatCommitment
    ) {
        return None;
    }

    let resume_conditions = resume_conditions_for_barrier_fact(&seed.barrier_fact, cognitive);
    if resume_conditions.is_empty() {
        return None;
    }

    Some(PartialPlanSegment {
        id: PartialPlanSegmentId::new(seed.created_tick, seed.local_counter),
        goal: seed.goal,
        completed_prefix: seed.completed_prefix,
        remaining_skeleton: seed.remaining_skeleton,
        terminal_barrier: seed.terminal_barrier,
        barrier_fact: seed.barrier_fact,
        resume_conditions,
        abandon_conditions: vec![IntentionAbandonCondition::PatienceExhausted],
        created_tick: seed.created_tick,
        last_resume_attempt_tick: None,
        resume_attempt_count: 0,
        causal_links: seed.causal_links,
    })
}

#[must_use]
pub fn budget_exhausted_partial_plan_segment(
    goal: GoalOffer,
    expansions_used: u32,
    budget_total: u32,
    remaining_skeleton: Option<Vec<PlannedSkeletonStep>>,
    created_tick: Tick,
    local_counter: u16,
    cognitive: &CognitiveProfile,
) -> PartialPlanSegment {
    build_partial_plan_segment(
        PartialPlanSegmentSeed {
            goal,
            completed_prefix: Vec::new(),
            remaining_skeleton: remaining_skeleton.and_then(filter_preservable_skeleton),
            terminal_barrier: PlanTerminalKind::SearchBudgetExhausted {
                budget_consumed: expansions_used.min(u32::from(u16::MAX)) as u16,
                budget_total: budget_total.min(u32::from(u16::MAX)) as u16,
            },
            barrier_fact: BarrierFact::BudgetExhausted {
                remaining_stages: 1,
            },
            created_tick,
            local_counter,
            causal_links: Vec::new(),
        },
        cognitive,
    )
    .expect("budget exhaustion always has a profile-backed resume condition")
}

#[must_use]
pub fn information_barrier_partial_plan_segment(
    goal: GoalOffer,
    topic: TellTopic,
    remaining_skeleton: Option<Vec<PlannedSkeletonStep>>,
    created_tick: Tick,
    local_counter: u16,
    cognitive: &CognitiveProfile,
) -> Option<PartialPlanSegment> {
    build_partial_plan_segment(
        PartialPlanSegmentSeed {
            goal,
            completed_prefix: Vec::new(),
            remaining_skeleton: remaining_skeleton.and_then(filter_preservable_skeleton),
            terminal_barrier: PlanTerminalKind::InformationBarrier { topic },
            barrier_fact: information_barrier_fact_for_topic(topic)?,
            created_tick,
            local_counter,
            causal_links: Vec::new(),
        },
        cognitive,
    )
}

#[must_use]
pub fn filter_preservable_skeleton(
    steps: Vec<PlannedSkeletonStep>,
) -> Option<Vec<PlannedSkeletonStep>> {
    let filtered = steps
        .into_iter()
        .filter(|step| {
            skeleton_op_is_preservable(step.op)
                && payload_template_is_preservable(&step.target_template)
        })
        .collect::<Vec<_>>();
    (!filtered.is_empty()).then_some(filtered)
}

const fn skeleton_op_is_preservable(op: PlannerOpKind) -> bool {
    !matches!(op, PlannerOpKind::Attack | PlannerOpKind::Defend)
}

fn payload_template_is_preservable(payload: &PayloadTemplate) -> bool {
    match payload {
        PayloadTemplate::FromContext => true,
        PayloadTemplate::Explicit(payload) => !payload_value_template_has_fixed_entity(payload),
    }
}

fn payload_value_template_has_fixed_entity(payload: &PayloadValueTemplate) -> bool {
    match payload {
        PayloadValueTemplate::Trade { .. } | PayloadValueTemplate::Craft { .. } => false,
        PayloadValueTemplate::Attack { target }
        | PayloadValueTemplate::ClaimBounty { bounty: target } => fixed_entity(*target).is_some(),
        PayloadValueTemplate::EscortToSafety {
            escortee,
            destination,
        } => fixed_entity(*escortee).is_some() || location_template_has_fixed_entity(destination),
    }
}

fn location_template_has_fixed_entity(location: &LocationTemplate) -> bool {
    match location {
        LocationTemplate::LastKnownTargetPlace { target }
        | LocationTemplate::BountyIssuerPlace { bounty: target }
        | LocationTemplate::OfficePlace {
            institution: target,
        }
        | LocationTemplate::EscorteeHome { escortee: target }
        | LocationTemplate::StagingPlaceForConfrontation { target } => {
            fixed_entity(*target).is_some()
        }
        LocationTemplate::NearestSellerOf { .. }
        | LocationTemplate::AgentHome
        | LocationTemplate::KnownWorkstationFor { .. } => false,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoordinationBarrierBlockerRecord {
    pub scope: BlockerScope,
    pub affordance: AffordanceKey,
    pub contention_event: Option<EventId>,
    pub observed_tick: Tick,
    pub source_event: EventId,
}

#[must_use]
pub const fn terminal_to_discrepancy(terminal: &PlanTerminalKind) -> Option<Discrepancy> {
    match terminal {
        PlanTerminalKind::GoalSatisfied
        | PlanTerminalKind::CombatCommitment
        | PlanTerminalKind::CoordinationBarrier { .. } => None,
        PlanTerminalKind::InformationBarrier { .. } => Some(Discrepancy::MissingObservation),
        PlanTerminalKind::ResourceBarrier { .. } => Some(Discrepancy::BeliefStale),
        PlanTerminalKind::JurisdictionBarrier { .. } => Some(Discrepancy::NoLegalBinding),
        PlanTerminalKind::SearchBudgetExhausted { .. } => Some(Discrepancy::SearchBudgetExhausted),
    }
}

#[must_use]
pub fn coordination_barrier_blocking_fact(
    terminal: &PlanTerminalKind,
    affordance: AffordanceKey,
    contention_event: Option<EventId>,
) -> Option<BlockingFact> {
    match terminal {
        PlanTerminalKind::CoordinationBarrier { contested_resource }
            if *contested_resource == affordance.facility =>
        {
            Some(BlockingFact::ReservationConflict {
                affordance,
                contention_event,
            })
        }
        _ => None,
    }
}

pub fn record_coordination_barrier_blocker(
    memory: &mut BlockerMemory,
    terminal: &PlanTerminalKind,
    record: CoordinationBarrierBlockerRecord,
    cognitive: &CognitiveProfile,
) -> bool {
    let Some(blocking_fact) =
        coordination_barrier_blocking_fact(terminal, record.affordance, record.contention_event)
    else {
        return false;
    };
    memory.record(Blocker {
        scope: record.scope,
        blocking_fact,
        diagnostic_context: None,
        observed_tick: record.observed_tick,
        expires_tick: record.observed_tick + u64::from(cognitive.structural_block_ticks),
        clearing_condition: BlockerClearingCondition::ContentionChanged {
            facility: record.affordance.facility,
        },
        baseline_snapshot: None,
        source_event: Some(record.source_event),
    });
    true
}

fn information_barrier_fact_for_topic(topic: TellTopic) -> Option<BarrierFact> {
    match topic {
        TellTopic::EntityBelief { subject } => Some(BarrierFact::MissingBelief(
            BeliefPredicate::TargetLastSeenKnown {
                target: EntityTemplate::Fixed(subject),
            },
        )),
        TellTopic::SocialObservation { .. } | TellTopic::InstitutionalClaim { .. } => None,
    }
}

#[must_use]
pub fn resume_conditions_for_barrier_fact(
    fact: &BarrierFact,
    cognitive: &CognitiveProfile,
) -> Vec<IntentionResumeCondition> {
    match fact {
        BarrierFact::MissingBelief(predicate) => missing_belief_subject(predicate)
            .map(|subject| {
                vec![IntentionResumeCondition::BeliefStatusChanged {
                    subject,
                    target_status: BeliefStatusTag::Certain,
                }]
            })
            .unwrap_or_default(),
        BarrierFact::ContestedReservation(target) => {
            vec![IntentionResumeCondition::ArtifactLegalEffectActive(*target)]
        }
        BarrierFact::DepletedResource { place, .. } => {
            vec![IntentionResumeCondition::BeliefStatusChanged {
                subject: *place,
                target_status: BeliefStatusTag::Certain,
            }]
        }
        BarrierFact::NoAuthorityForAction(authority) => {
            vec![IntentionResumeCondition::ArtifactLegalEffectActive(
                *authority,
            )]
        }
        BarrierFact::BudgetExhausted { .. } => {
            vec![IntentionResumeCondition::TickElapsed(
                cognitive.search_exhaustion_backoff_ticks,
            )]
        }
    }
}

const fn fixed_entity(template: EntityTemplate) -> Option<EntityId> {
    match template {
        EntityTemplate::Fixed(entity) => Some(entity),
        EntityTemplate::GoalPrimaryEntity
        | EntityTemplate::GoalSecondaryEntity
        | EntityTemplate::GoalPlace
        | EntityTemplate::BountyTarget
        | EntityTemplate::Violation
        | EntityTemplate::Institution
        | EntityTemplate::Escortee => None,
    }
}

fn missing_belief_subject(predicate: &BeliefPredicate) -> Option<EntityId> {
    match predicate {
        BeliefPredicate::BountyRecordExists { bounty }
        | BeliefPredicate::BountyExpired { bounty } => fixed_entity(*bounty),
        BeliefPredicate::TargetLastSeenKnown { target }
        | BeliefPredicate::TargetBelievedDangerous { target } => fixed_entity(*target),
        BeliefPredicate::WitnessNamesKnown { violation }
        | BeliefPredicate::InstitutionalRecordBelievedExtant { violation } => {
            fixed_entity(*violation)
        }
        BeliefPredicate::EscorteeBelievedSafeAt { escortee } => fixed_entity(*escortee),
        BeliefPredicate::ResourceSourceKnown { .. }
        | BeliefPredicate::SellerKnown { .. }
        | BeliefPredicate::OwnedCommodityBelowThreshold { .. }
        | BeliefPredicate::OwnsInputsForRecipe { .. }
        | BeliefPredicate::AllyOrBountyOfficeAvailable => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BarrierFact, CoordinationBarrierBlockerRecord, PartialPlanSegment, PartialPlanSegmentId,
        PartialPlanSegmentSeed, PlannedSkeletonStep, budget_exhausted_partial_plan_segment,
        build_partial_plan_segment, coordination_barrier_blocking_fact,
        filter_preservable_skeleton, information_barrier_partial_plan_segment,
        record_coordination_barrier_blocker, resume_conditions_for_barrier_fact,
        terminal_to_discrepancy,
    };
    use crate::{
        GoalOffer, PlanTerminalKind, PlannedStep, PlannerOpKind, PlanningEntityRef,
        htn::{
            BeliefPredicate, CommodityTemplate, EntityTemplate, PayloadTemplate,
            PayloadValueTemplate,
        },
    };
    use std::collections::BTreeSet;
    use worldwake_core::{
        ActionDefId, AffordanceKey, BeliefStatusTag, BlockerMemory, BlockerScope, BlockingFact,
        CognitiveProfile, CommodityKind, CommodityPurpose, Discrepancy, EntityId, EventId, GoalKey,
        GoalKind, IntentionAbandonCondition, IntentionResumeCondition, OpportunityAnchor, Permille,
        TellTopic, Tick,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn goal_offer() -> GoalOffer {
        let kind = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: worldwake_core::AcquisitionQuantity::single(),
        };
        GoalOffer {
            key: GoalKey::from(kind),
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: Some(worldwake_core::AcquisitionQuantity::single()),
        }
    }

    fn planned_step() -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(1),
            targets: vec![PlanningEntityRef::Authoritative(entity(10))],
            target_place: Some(entity(20)),
            payload_override: None,
            op_kind: PlannerOpKind::Travel,
            estimated_ticks: 3,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        }
    }

    fn skeleton_step() -> PlannedSkeletonStep {
        PlannedSkeletonStep {
            op: PlannerOpKind::AskWitness,
            target_template: PayloadTemplate::FromContext,
            expected_pre: vec![BeliefPredicate::SellerKnown {
                commodity: CommodityTemplate::Fixed(CommodityKind::Bread),
            }],
        }
    }

    fn trade_skeleton_step() -> PlannedSkeletonStep {
        PlannedSkeletonStep {
            op: PlannerOpKind::Trade,
            target_template: PayloadTemplate::Explicit(PayloadValueTemplate::Trade {
                commodity: CommodityTemplate::Fixed(CommodityKind::Bread),
                quantity: worldwake_core::Quantity(1),
            }),
            expected_pre: vec![BeliefPredicate::SellerKnown {
                commodity: CommodityTemplate::Fixed(CommodityKind::Bread),
            }],
        }
    }

    fn fixed_target_skeleton_step() -> PlannedSkeletonStep {
        PlannedSkeletonStep {
            op: PlannerOpKind::ClaimBounty,
            target_template: PayloadTemplate::Explicit(PayloadValueTemplate::ClaimBounty {
                bounty: EntityTemplate::Fixed(entity(80)),
            }),
            expected_pre: Vec::new(),
        }
    }

    fn segment(barrier_fact: BarrierFact) -> PartialPlanSegment {
        PartialPlanSegment {
            id: PartialPlanSegmentId::new(Tick(7), 2),
            goal: goal_offer(),
            completed_prefix: vec![planned_step()],
            remaining_skeleton: Some(vec![skeleton_step()]),
            terminal_barrier: PlanTerminalKind::ResourceBarrier {
                commodity: CommodityKind::Bread,
                place: entity(20),
            },
            barrier_fact,
            resume_conditions: vec![IntentionResumeCondition::TickElapsed(4)],
            abandon_conditions: vec![IntentionAbandonCondition::PatienceExhausted],
            created_tick: Tick(7),
            last_resume_attempt_tick: Some(Tick(9)),
            resume_attempt_count: 1,
            causal_links: vec![EventId(42)],
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
    fn partial_plan_segment_roundtrips_through_bincode_with_all_barrier_facts() {
        let cases = [
            BarrierFact::MissingBelief(BeliefPredicate::SellerKnown {
                commodity: CommodityTemplate::Fixed(CommodityKind::Bread),
            }),
            BarrierFact::ContestedReservation(entity(30)),
            BarrierFact::DepletedResource {
                commodity: CommodityKind::Bread,
                place: entity(31),
            },
            BarrierFact::NoAuthorityForAction(entity(32)),
            BarrierFact::BudgetExhausted {
                remaining_stages: 2,
            },
        ];

        for barrier_fact in cases {
            let original = segment(barrier_fact);
            let decoded: PartialPlanSegment = roundtrip(&original);
            assert_eq!(decoded, original);
        }
    }

    #[test]
    fn partial_plan_segment_id_preserves_tick_and_counter_identity() {
        let first = PartialPlanSegmentId::new(Tick(11), 1);
        let second = PartialPlanSegmentId::new(Tick(11), 2);
        let later_tick = PartialPlanSegmentId::new(Tick(12), 0);

        assert_ne!(first, second);
        assert_ne!(first, later_tick);
        assert!(first < second);
        assert!(second < later_tick);
    }

    #[test]
    fn typed_terminals_map_to_existing_discrepancy_surface() {
        let place = entity(20);
        let authority = entity(21);
        let jurisdiction = entity(22);
        let contested = entity(23);

        let cases = [
            (PlanTerminalKind::GoalSatisfied, None),
            (PlanTerminalKind::CombatCommitment, None),
            (
                PlanTerminalKind::InformationBarrier {
                    topic: worldwake_core::TellTopic::EntityBelief { subject: place },
                },
                Some(Discrepancy::MissingObservation),
            ),
            (
                PlanTerminalKind::CoordinationBarrier {
                    contested_resource: contested,
                },
                None,
            ),
            (
                PlanTerminalKind::ResourceBarrier {
                    commodity: CommodityKind::Bread,
                    place,
                },
                Some(Discrepancy::BeliefStale),
            ),
            (
                PlanTerminalKind::JurisdictionBarrier {
                    authority,
                    jurisdiction,
                },
                Some(Discrepancy::NoLegalBinding),
            ),
            (
                PlanTerminalKind::SearchBudgetExhausted {
                    budget_consumed: 7,
                    budget_total: 10,
                },
                Some(Discrepancy::SearchBudgetExhausted),
            ),
        ];

        for (terminal, expected) in cases {
            assert_eq!(terminal_to_discrepancy(&terminal), expected);
        }
    }

    #[test]
    fn coordination_barrier_records_reservation_conflict_blocker() {
        let facility = entity(30);
        let affordance = AffordanceKey {
            facility,
            action: ActionDefId(3),
        };
        let terminal = PlanTerminalKind::CoordinationBarrier {
            contested_resource: facility,
        };
        let contention_event = Some(EventId(77));
        let expected_fact = BlockingFact::ReservationConflict {
            affordance,
            contention_event,
        };
        assert_eq!(
            coordination_barrier_blocking_fact(&terminal, affordance, contention_event),
            Some(expected_fact)
        );

        let mut memory = BlockerMemory::default();
        let scope = BlockerScope::Exact(worldwake_core::BlockerKey {
            goal_key: goal_offer().key,
            place: Some(entity(31)),
            target: Some(facility),
            action_def: Some(ActionDefId(3)),
        });
        let cognitive = CognitiveProfile {
            structural_block_ticks: 12,
            ..CognitiveProfile::default()
        };
        assert!(record_coordination_barrier_blocker(
            &mut memory,
            &terminal,
            CoordinationBarrierBlockerRecord {
                scope,
                affordance,
                contention_event,
                observed_tick: Tick(40),
                source_event: EventId(88),
            },
            &cognitive,
        ));
        let blocker = memory.intents.get(&scope).unwrap();
        assert_eq!(blocker.blocking_fact, expected_fact);
        assert_eq!(
            blocker.clearing_condition,
            worldwake_core::BlockerClearingCondition::ContentionChanged { facility }
        );
        assert_eq!(blocker.observed_tick, Tick(40));
        assert_eq!(blocker.expires_tick, Tick(52));
        assert_eq!(blocker.source_event, Some(EventId(88)));
    }

    #[test]
    fn barrier_facts_derive_existing_resume_conditions() {
        let target = entity(40);
        let place = entity(41);
        let authority = entity(42);
        let cognitive = CognitiveProfile {
            search_exhaustion_backoff_ticks: 17,
            ..CognitiveProfile::default()
        };

        let cases = [
            (
                BarrierFact::MissingBelief(BeliefPredicate::TargetLastSeenKnown {
                    target: EntityTemplate::Fixed(target),
                }),
                vec![IntentionResumeCondition::BeliefStatusChanged {
                    subject: target,
                    target_status: BeliefStatusTag::Certain,
                }],
            ),
            (
                BarrierFact::ContestedReservation(target),
                vec![IntentionResumeCondition::ArtifactLegalEffectActive(target)],
            ),
            (
                BarrierFact::DepletedResource {
                    commodity: CommodityKind::Bread,
                    place,
                },
                vec![IntentionResumeCondition::BeliefStatusChanged {
                    subject: place,
                    target_status: BeliefStatusTag::Certain,
                }],
            ),
            (
                BarrierFact::NoAuthorityForAction(authority),
                vec![IntentionResumeCondition::ArtifactLegalEffectActive(
                    authority,
                )],
            ),
            (
                BarrierFact::BudgetExhausted {
                    remaining_stages: 2,
                },
                vec![IntentionResumeCondition::TickElapsed(17)],
            ),
        ];

        for (fact, expected) in cases {
            assert_eq!(
                resume_conditions_for_barrier_fact(&fact, &cognitive),
                expected
            );
        }
    }

    #[test]
    fn build_partial_plan_segment_writes_concrete_barrier_segment() {
        let cognitive = CognitiveProfile {
            search_exhaustion_backoff_ticks: 17,
            ..CognitiveProfile::default()
        };
        let step = planned_step();
        let segment = build_partial_plan_segment(
            PartialPlanSegmentSeed {
                goal: goal_offer(),
                completed_prefix: vec![step.clone()],
                remaining_skeleton: None,
                terminal_barrier: PlanTerminalKind::ResourceBarrier {
                    commodity: CommodityKind::Bread,
                    place: entity(20),
                },
                barrier_fact: BarrierFact::DepletedResource {
                    commodity: CommodityKind::Bread,
                    place: entity(20),
                },
                created_tick: Tick(42),
                local_counter: 3,
                causal_links: vec![EventId(9)],
            },
            &cognitive,
        )
        .expect("resource barrier should produce a resumable segment");

        assert_eq!(segment.id, PartialPlanSegmentId::new(Tick(42), 3));
        assert_eq!(segment.completed_prefix, vec![step]);
        assert_eq!(segment.remaining_skeleton, None);
        assert_eq!(
            segment.resume_conditions,
            vec![IntentionResumeCondition::BeliefStatusChanged {
                subject: entity(20),
                target_status: BeliefStatusTag::Certain,
            }]
        );
        assert_eq!(
            segment.abandon_conditions,
            vec![IntentionAbandonCondition::PatienceExhausted]
        );
        assert_eq!(segment.causal_links, vec![EventId(9)]);
    }

    #[test]
    fn build_partial_plan_segment_preserves_seeded_skeleton() {
        let cognitive = CognitiveProfile {
            search_exhaustion_backoff_ticks: 17,
            ..CognitiveProfile::default()
        };
        let skeleton = vec![trade_skeleton_step()];
        let segment = build_partial_plan_segment(
            PartialPlanSegmentSeed {
                goal: goal_offer(),
                completed_prefix: Vec::new(),
                remaining_skeleton: Some(skeleton.clone()),
                terminal_barrier: PlanTerminalKind::ResourceBarrier {
                    commodity: CommodityKind::Bread,
                    place: entity(20),
                },
                barrier_fact: BarrierFact::DepletedResource {
                    commodity: CommodityKind::Bread,
                    place: entity(20),
                },
                created_tick: Tick(42),
                local_counter: 3,
                causal_links: Vec::new(),
            },
            &cognitive,
        )
        .expect("resource barrier should produce a resumable segment");

        assert_eq!(segment.remaining_skeleton, Some(skeleton));
    }

    #[test]
    fn build_partial_plan_segment_rejects_non_barrier_terminals() {
        let seed = PartialPlanSegmentSeed {
            goal: goal_offer(),
            completed_prefix: Vec::new(),
            remaining_skeleton: None,
            terminal_barrier: PlanTerminalKind::GoalSatisfied,
            barrier_fact: BarrierFact::BudgetExhausted {
                remaining_stages: 1,
            },
            created_tick: Tick(42),
            local_counter: 3,
            causal_links: Vec::new(),
        };

        assert_eq!(
            build_partial_plan_segment(seed, &CognitiveProfile::default()),
            None
        );
    }

    #[test]
    fn budget_exhausted_partial_plan_segment_uses_typed_terminal_and_backoff() {
        let cognitive = CognitiveProfile {
            search_exhaustion_backoff_ticks: 23,
            ..CognitiveProfile::default()
        };
        let skeleton = vec![trade_skeleton_step()];
        let segment = budget_exhausted_partial_plan_segment(
            goal_offer(),
            77,
            88,
            Some(skeleton.clone()),
            Tick(12),
            4,
            &cognitive,
        );

        assert_eq!(
            segment.terminal_barrier,
            PlanTerminalKind::SearchBudgetExhausted {
                budget_consumed: 77,
                budget_total: 88,
            }
        );
        assert_eq!(
            segment.barrier_fact,
            BarrierFact::BudgetExhausted {
                remaining_stages: 1,
            }
        );
        assert_eq!(
            segment.resume_conditions,
            vec![IntentionResumeCondition::TickElapsed(23)]
        );
        assert_eq!(segment.remaining_skeleton, Some(skeleton));
    }

    #[test]
    fn budget_exhausted_partial_plan_segment_drops_empty_skeleton() {
        let segment = budget_exhausted_partial_plan_segment(
            goal_offer(),
            77,
            88,
            Some(Vec::new()),
            Tick(12),
            4,
            &CognitiveProfile::default(),
        );

        assert_eq!(segment.remaining_skeleton, None);
    }

    #[test]
    fn information_barrier_partial_plan_segment_preserves_filtered_skeleton() {
        let subject = entity(44);
        let skeleton = vec![
            PlannedSkeletonStep {
                op: PlannerOpKind::Trade,
                target_template: PayloadTemplate::FromContext,
                expected_pre: vec![BeliefPredicate::TargetLastSeenKnown {
                    target: EntityTemplate::Fixed(subject),
                }],
            },
            PlannedSkeletonStep {
                op: PlannerOpKind::Attack,
                target_template: PayloadTemplate::FromContext,
                expected_pre: Vec::new(),
            },
        ];

        let segment = information_barrier_partial_plan_segment(
            goal_offer(),
            TellTopic::EntityBelief { subject },
            Some(skeleton.clone()),
            Tick(31),
            2,
            &CognitiveProfile::default(),
        )
        .expect("entity-belief information barrier should build a segment");

        assert_eq!(
            segment.terminal_barrier,
            PlanTerminalKind::InformationBarrier {
                topic: TellTopic::EntityBelief { subject },
            }
        );
        assert_eq!(
            segment.barrier_fact,
            BarrierFact::MissingBelief(BeliefPredicate::TargetLastSeenKnown {
                target: EntityTemplate::Fixed(subject),
            })
        );
        assert_eq!(
            segment.resume_conditions,
            vec![IntentionResumeCondition::BeliefStatusChanged {
                subject,
                target_status: BeliefStatusTag::Certain,
            }]
        );
        assert_eq!(segment.remaining_skeleton, Some(vec![skeleton[0].clone()]));
    }

    #[test]
    fn filter_preservable_skeleton_excludes_combat_and_fixed_identity_steps() {
        let preserved = trade_skeleton_step();
        let attack = PlannedSkeletonStep {
            op: PlannerOpKind::Attack,
            target_template: PayloadTemplate::FromContext,
            expected_pre: Vec::new(),
        };
        let fixed_target = fixed_target_skeleton_step();

        assert_eq!(
            filter_preservable_skeleton(vec![
                attack.clone(),
                preserved.clone(),
                fixed_target.clone()
            ]),
            Some(vec![preserved])
        );
        assert_eq!(
            filter_preservable_skeleton(vec![attack, fixed_target]),
            None
        );
    }
}
