//! Golden contract coverage for S148 five-slot portfolio and intention lifecycle
//! data surfaces.
//!
//! The full agent-decision-cycle portfolio admission path remains covered by
//! `golden_portfolio_planning.rs`. This file locks the public S148 contracts
//! that the full pipeline consumes: motive-to-slot mapping, per-slot profile
//! weights, enriched intention-frame persistence, and typed abandon
//! discrepancies.

use std::collections::BTreeSet;

use worldwake_core::{
    BeliefStatusTag, Discrepancy, EntityId, EventId, FrameAssumption, FrameState, GoalKey,
    GoalKind, HomeostaticNeedId, IntentionAbandonCondition, IntentionAbandonConditionDiscriminant,
    IntentionDomain, IntentionFrame, IntentionResumeCondition, MotiveSource,
    MotiveSourceDiscriminant, MotiveSourceRef, OperatingMode, OpportunityAnchor,
    PortfolioWeightsProfile, SlotKind, Tick, motive_source_slot_for,
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
    let bytes = bincode::serialize(value).expect("value should serialize");
    bincode::deserialize(&bytes).expect("value should deserialize")
}

#[test]
fn all_motive_discriminants_map_onto_the_five_s148_slots() {
    let cases = [
        (
            MotiveSourceDiscriminant::NeedPressure,
            SlotKind::NeedSurvival,
        ),
        (MotiveSourceDiscriminant::Pain, SlotKind::PainCare),
        (
            MotiveSourceDiscriminant::OfficeDuty,
            SlotKind::ObligationDuty,
        ),
        (MotiveSourceDiscriminant::Loyalty, SlotKind::ObligationDuty),
        (
            MotiveSourceDiscriminant::Greed,
            SlotKind::EconomicOpportunity,
        ),
        (MotiveSourceDiscriminant::Shame, SlotKind::SocialMotive),
        (MotiveSourceDiscriminant::Revenge, SlotKind::SocialMotive),
    ];

    let mut observed_slots = BTreeSet::new();
    for (discriminant, expected_slot) in cases {
        let slot = motive_source_slot_for(discriminant);
        assert_eq!(slot, expected_slot);
        observed_slots.insert(slot);
    }

    assert_eq!(
        observed_slots,
        BTreeSet::from([
            SlotKind::NeedSurvival,
            SlotKind::PainCare,
            SlotKind::ObligationDuty,
            SlotKind::EconomicOpportunity,
            SlotKind::SocialMotive,
        ])
    );
}

#[test]
fn portfolio_weights_profile_exposes_s148_slot_weights_and_mode_caps() {
    let profile = PortfolioWeightsProfile::default();

    assert_eq!(profile.weight_for(SlotKind::NeedSurvival).value(), 1000);
    assert_eq!(profile.weight_for(SlotKind::PainCare).value(), 900);
    assert_eq!(profile.weight_for(SlotKind::ObligationDuty).value(), 800);
    assert_eq!(
        profile.weight_for(SlotKind::EconomicOpportunity).value(),
        600
    );
    assert_eq!(profile.weight_for(SlotKind::SocialMotive).value(), 400);

    assert_eq!(profile.max_plans_for_mode(OperatingMode::Normal), 5);
    assert_eq!(profile.max_plans_for_mode(OperatingMode::Emergency), 3);
    assert_eq!(profile.max_plans_for_mode(OperatingMode::Idle), 5);
}

#[test]
fn enriched_intention_frame_roundtrips_every_s148_field() {
    let frame = IntentionFrame {
        goal: GoalKey::from(GoalKind::Sleep),
        domain: IntentionDomain::Generic,
        assumptions: vec![FrameAssumption::NoCriticalThreat],
        state: FrameState::Active,
        established_at: Tick(10),
        last_progress_tick: Some(Tick(11)),
        stalled_ticks: 2,
        patience_limit: 8,
        motive_refs: vec![MotiveSourceRef {
            source: MotiveSource::NeedPressure {
                need: HomeostaticNeedId::Hunger,
            },
            introduced_tick: Tick(9),
        }],
        resume_conditions: resume_condition_cases(),
        abandon_conditions: abandon_condition_cases(),
        explicit_claims: vec![entity(100), entity(101)],
        causal_links: vec![EventId(7), EventId(8)],
    };

    let decoded: IntentionFrame = roundtrip(&frame);

    assert_eq!(decoded, frame);
    assert_eq!(decoded.motive_refs.len(), 1);
    assert_eq!(decoded.resume_conditions.len(), 5);
    assert_eq!(decoded.abandon_conditions.len(), 6);
    assert_eq!(decoded.explicit_claims, vec![entity(100), entity(101)]);
    assert_eq!(decoded.causal_links, vec![EventId(7), EventId(8)]);
}

#[test]
fn resume_condition_variants_remain_distinct_and_serializable() {
    let cases = resume_condition_cases();
    let decoded: Vec<IntentionResumeCondition> = roundtrip(&cases);

    assert_eq!(decoded, cases);
    assert_eq!(decoded.len(), 5);
}

#[test]
fn abandon_condition_variants_have_typed_discrepancy_discriminants() {
    let cases = abandon_condition_cases();
    let decoded: Vec<IntentionAbandonCondition> = roundtrip(&cases);

    assert_eq!(decoded, cases);
    assert_eq!(decoded.len(), 6);

    let discriminants = decoded
        .iter()
        .map(IntentionAbandonConditionDiscriminant::from)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        discriminants,
        BTreeSet::from([
            IntentionAbandonConditionDiscriminant::MotiveSourceLost,
            IntentionAbandonConditionDiscriminant::AssumptionPermanentlyBroken,
            IntentionAbandonConditionDiscriminant::OpportunityForeverGone,
            IntentionAbandonConditionDiscriminant::PatienceExhausted,
            IntentionAbandonConditionDiscriminant::ArtifactDestroyed,
            IntentionAbandonConditionDiscriminant::ArtifactLegalEffectLost,
        ])
    );

    for discriminant in discriminants {
        let discrepancy = Discrepancy::AbandonConditionFired(discriminant);
        assert_eq!(roundtrip(&discrepancy), discrepancy);
    }
}

fn resume_condition_cases() -> Vec<IntentionResumeCondition> {
    vec![
        IntentionResumeCondition::BeliefStatusChanged {
            subject: entity(1),
            target_status: BeliefStatusTag::Certain,
        },
        IntentionResumeCondition::OpportunityVisible(OpportunityAnchor::Entity(entity(2))),
        IntentionResumeCondition::LocationReached(entity(3)),
        IntentionResumeCondition::TickElapsed(4),
        IntentionResumeCondition::ArtifactLegalEffectActive(entity(5)),
    ]
}

fn abandon_condition_cases() -> Vec<IntentionAbandonCondition> {
    vec![
        IntentionAbandonCondition::MotiveSourceLost(MotiveSourceDiscriminant::NeedPressure),
        IntentionAbandonCondition::AssumptionPermanentlyBroken(FrameAssumption::RouteExists {
            from: entity(10),
            to: entity(11),
        }),
        IntentionAbandonCondition::OpportunityForeverGone(OpportunityAnchor::Place(entity(12))),
        IntentionAbandonCondition::PatienceExhausted,
        IntentionAbandonCondition::ArtifactDestroyed(entity(13)),
        IntentionAbandonCondition::ArtifactLegalEffectLost(entity(14)),
    ]
}
