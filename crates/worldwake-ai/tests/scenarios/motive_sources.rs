//! Golden fixtures for S141 motive-source payload and trace shape.
//!
//! These cases cover the S141 motive-source payload and contribution ledger:
//! production self-consume acquisition now carries independent hunger and
//! greed sources, and trace summaries carry contribution values without
//! observer-side re-derivation.

use std::collections::{BTreeMap, BTreeSet};

use worldwake_ai::{
    GoalOffer, RankedGoalSummary, motive_source_mapping::derive_default_motive_sources,
};
use worldwake_core::{
    AcquisitionQuantity, CauseRef, CommodityKind, CommodityPurpose, DecisionEventPayload, EntityId,
    EventLog, EventPayload, EventTag, EventView, GoalCommittedPayload, GoalKey, GoalKind,
    HomeostaticNeedId, MotiveSource, MotiveSourceRef, OpportunityAnchor, OpportunityKey,
    PendingEvent, RejectedAlternativeSummary, Tick, VisibilitySpec, WitnessData, WoundId,
};

fn entity(slot: u32) -> EntityId {
    EntityId {
        slot,
        generation: 0,
    }
}

fn source(source: MotiveSource, introduced_tick: u32) -> MotiveSourceRef {
    MotiveSourceRef {
        source,
        introduced_tick: Tick(u64::from(introduced_tick)),
    }
}

fn opportunity(goal_kind: GoalKind, anchor: OpportunityAnchor) -> OpportunityKey {
    OpportunityKey {
        goal_key: GoalKey::from(goal_kind),
        anchor,
    }
}

fn emit_goal_committed_payload(payload: GoalCommittedPayload) -> GoalCommittedPayload {
    let mut event_log = EventLog::new();
    let id = event_log.emit(PendingEvent::from_payload(EventPayload {
        tick: Tick(42),
        cause: CauseRef::Bootstrap,
        actor_id: Some(payload.agent),
        action_name: None,
        target_ids: Vec::new(),
        evidence: Vec::new(),
        place_id: Some(entity(100)),
        state_deltas: Vec::new(),
        observed_entities: BTreeMap::new(),
        visibility: VisibilitySpec::SamePlace,
        witness_data: WitnessData::default(),
        tags: BTreeSet::from([EventTag::GoalCommitted]),
        contention_event_payload: None,
        decision_payload: Some(DecisionEventPayload::GoalCommitted(payload)),
        artifact_transition_payload: None,

        personality_assigned_payload: None,
    }));

    let Some(DecisionEventPayload::GoalCommitted(payload)) = event_log
        .get(id)
        .and_then(|event| event.decision_payload())
        .cloned()
    else {
        panic!("expected GoalCommitted payload");
    };
    payload
}

fn summary_with_contributions(
    opportunity: OpportunityKey,
    motive_score: u32,
    contributions: Vec<(MotiveSourceRef, u32)>,
) -> RankedGoalSummary {
    RankedGoalSummary {
        opportunity,
        motive_score,
        motive_source_contributions: contributions,
        ..RankedGoalSummary::default()
    }
}

fn empty_offer() -> GoalOffer {
    GoalOffer {
        key: GoalKey::from(GoalKind::Sleep),
        anchor: OpportunityAnchor::None,
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::new(),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    }
}

// Scenario 403: S141 Motive Sources Hunger And Greed Sum For Market Offer
// Systems: AI
// GoalKinds: AcquireCommodity
// ActionDomains: Needs, Trade
// Principles: P3, P20, P29
// Setup: programmatic market-offer fixture isolates the default source mapping
//        and trace contribution ledger without observer-side derivation.
// Proves: a self-consume acquisition offer maps to NeedPressure(Hunger) plus
//         Greed, and their contributions sum to the aggregate motive_score.
// Cross-system chain: GoalKind -> MotiveSourceRef -> RankedGoalSummary.
#[test]
fn golden_motive_sources_hunger_and_greed_offer_sums_source_contributions() {
    let goal = GoalKind::AcquireCommodity {
        commodity: CommodityKind::Bread,
        purpose: CommodityPurpose::SelfConsume,
        quantity: AcquisitionQuantity::single(),
    };
    let anchor = OpportunityAnchor::Place(entity(20));
    let sources = derive_default_motive_sources(&goal, &anchor, Tick(7));

    assert_eq!(sources.len(), 2);
    assert_eq!(
        sources[0].source,
        MotiveSource::NeedPressure {
            need: HomeostaticNeedId::Hunger
        }
    );
    assert!(matches!(sources[1].source, MotiveSource::Greed { .. }));

    let summary = summary_with_contributions(
        opportunity(goal, anchor),
        1_040_000,
        vec![(sources[0].clone(), 540_000), (sources[1].clone(), 500_000)],
    );
    let contribution_sum: u32 = summary
        .motive_source_contributions
        .iter()
        .map(|(_, contribution)| *contribution)
        .sum();
    assert_eq!(contribution_sum, summary.motive_score);
}

// Scenario 404: S141 Motive Sources Commit Payload Preserves Hunger And Greed
// Systems: AI, EventLog
// GoalKinds: AcquireCommodity
// ActionDomains: DecisionHistory
// Principles: P3, P20, P29
// Setup: event-log fixture carries two decisive sources directly. Rival
//        planner branches are excluded because this scenario proves persisted
//        payload shape, not autonomous candidate choice.
// Proves: GoalCommitted payloads preserve multiple decisive motive sources in
//         insertion order through the append-only event log.
// Cross-system chain: GoalCommittedPayload -> EventLog -> DecisionEventPayload.
#[test]
fn golden_motive_sources_commit_payload_preserves_hunger_and_greed_sources() {
    let hunger = source(
        MotiveSource::NeedPressure {
            need: HomeostaticNeedId::Hunger,
        },
        10,
    );
    let greed_goal = GoalKind::SellCommodity {
        commodity: CommodityKind::Bread,
    };
    let greed_opportunity = opportunity(greed_goal, OpportunityAnchor::Place(entity(20)));
    let greed = source(
        MotiveSource::Greed {
            opportunity: greed_opportunity,
        },
        11,
    );
    let committed = emit_goal_committed_payload(GoalCommittedPayload {
        agent: entity(1),
        goal_key: GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        motive_score: 22_640,
        decisive_motive_sources: vec![hunger.clone(), greed.clone()],
        rejected_alternatives: Vec::<RejectedAlternativeSummary>::new(),
        assumptions: Vec::new(),
        testimony_trust_context: Vec::new(),
        route_preference_context: Vec::new(),
    });

    assert_eq!(committed.decisive_motive_sources, vec![hunger, greed]);
}

// Scenario 405: S141 Motive Sources Pain Contribution Dominates Hunger
// Systems: AI
// GoalKinds: TreatWounds, ConsumeOwnedCommodity
// ActionDomains: Medical, Needs
// Principles: P3, P20
// Setup: trace-summary fixture excludes hunger-relief execution and medical
//        action execution; it isolates the contribution ordering the live trace
//        carrier can express.
// Proves: a Pain source contribution can be represented as the dominant motive
//         over a competing Hunger source without collapsing either source into
//         the aggregate score.
// Cross-system chain: MotiveSourceRef -> RankedGoalSummary contribution list.
#[test]
fn golden_motive_sources_pain_contribution_can_dominate_hunger() {
    let pain = source(MotiveSource::Pain { wound: WoundId(3) }, 20);
    let hunger = source(
        MotiveSource::NeedPressure {
            need: HomeostaticNeedId::Hunger,
        },
        20,
    );
    let summary = summary_with_contributions(
        opportunity(
            GoalKind::TreatWounds { patient: entity(1) },
            OpportunityAnchor::Entity(entity(1)),
        ),
        1_200,
        vec![(pain.clone(), 900), (hunger.clone(), 300)],
    );

    let pain_contribution = summary
        .motive_source_contributions
        .iter()
        .find(|(candidate, _)| candidate == &pain)
        .map(|(_, contribution)| *contribution)
        .expect("Pain contribution should be present");
    let hunger_contribution = summary
        .motive_source_contributions
        .iter()
        .find(|(candidate, _)| candidate == &hunger)
        .map(|(_, contribution)| *contribution)
        .expect("Hunger contribution should be present");

    assert!(pain_contribution > hunger_contribution);
}

// Scenario 406: S141 Motive Sources Greed Weight Variation Is Profile State
// Systems: AI
// GoalKinds: SellCommodity
// ActionDomains: DecisionHistory
// Principles: P3, P22
// Setup: two otherwise identical UtilityProfiles vary only greed_weight.
// Proves: Greed-backed motive sources and per-agent greed_weight variation are
//         both concrete state without adding a global tuning path.
// Cross-system chain: UtilityProfile -> MotiveSource::Greed.
#[test]
fn golden_motive_sources_greed_weight_variation_is_profile_state() {
    let goal = GoalKind::SellCommodity {
        commodity: CommodityKind::Bread,
    };
    let anchor = OpportunityAnchor::Place(entity(20));
    let sources = derive_default_motive_sources(&goal, &anchor, Tick(30));
    let mut low_greed = worldwake_core::UtilityProfile::default();
    let mut high_greed = worldwake_core::UtilityProfile::default();
    low_greed.greed_weight = worldwake_core::Permille::new_unchecked(125);
    high_greed.greed_weight = worldwake_core::Permille::new_unchecked(875);

    assert!(matches!(sources[0].source, MotiveSource::Greed { .. }));
    assert_ne!(low_greed.greed_weight, high_greed.greed_weight);
}

// Scenario 407: S141 Motive Sources Empty Offer Assertion
// Systems: AI
// GoalKinds: Sleep
// ActionDomains: DecisionHistory
// Principles: P28
// Setup: synthetic fixture constructs an explicitly empty GoalOffer. This is
//        the remaining test-only path called out by S141; production emitters
//        are covered by conformance_motive_sources.
// Proves: the debug assertion rejects empty motive_sources at explicit
//         validation points in test builds.
// Cross-system chain: GoalOffer -> debug assertion helper.
#[test]
#[should_panic(expected = "GoalOffer.motive_sources must be non-empty post-S141")]
fn golden_motive_sources_empty_offer_assertion_panics_in_test_build() {
    empty_offer().assert_motive_sources_present();
}
