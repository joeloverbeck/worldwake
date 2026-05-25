//! Shared test utilities for the Worldwake simulation.
//!
//! These helpers are available to all crates in the workspace for
//! deterministic testing.

use crate::{
    AcquisitionQuantity, ActionDefId, Blocker, BlockerClearingCondition, BlockerKey, BlockerMemory,
    BlockerScope, BlockingFact, BreachSignature, ClearingBaseline, CommodityKind, CommodityPurpose,
    CommodityValuationProfile, ContentionDispositionProfile, DemandMemory, DemandObservation,
    DemandObservationReason, Discrepancy, DiscrepancyClearing, DiscrepancyEntry, DiscrepancyMemory,
    EdgeExperience, EntityId, GoalKey, GoalKind, InvalidatorTag, LearnedOpportunityMemory,
    LearnedOpportunitySource, MemoryCapacityProfile, MerchandiseProfile, OpportunityAnchor,
    OpportunityEntry, OpportunityKey, Permille, PreferenceProfile, Quantity, ReliabilityRecord,
    RepairEntry, RepairKind, RepairMemory, RouteExperience, Seed, SourceKey, SourceReliability,
    StockAssignment, StockAssignmentKind, StockStoragePolicy, SubstitutePreferences, Tick,
    TradeCategory, TradeDispositionProfile, TravelEdgeId, UtilityProfile,
};
use std::collections::{BTreeMap, BTreeSet};
use std::num::{NonZeroU8, NonZeroU32};

/// Returns a fixed, well-known seed for deterministic test scenarios.
pub fn deterministic_seed() -> Seed {
    // All zeros — simple, memorable, deterministic.
    Seed([0u8; 32])
}

/// Returns a deterministic test entity id.
pub fn entity_id(slot: u32, generation: u32) -> EntityId {
    EntityId { slot, generation }
}

/// Returns a representative demand observation fixture for trade-domain tests.
pub fn sample_demand_observation() -> DemandObservation {
    DemandObservation {
        commodity: CommodityKind::Bread,
        quantity: Quantity(3),
        place: entity_id(5, 1),
        tick: Tick(11),
        counterparty: Some(entity_id(9, 2)),
        reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
    }
}

/// Returns a representative demand memory fixture for authoritative component tests.
pub fn sample_demand_memory() -> DemandMemory {
    DemandMemory {
        observations: vec![sample_demand_observation()],
    }
}

/// Returns a representative merchandise profile fixture for trade-domain tests.
pub fn sample_merchandise_profile() -> MerchandiseProfile {
    MerchandiseProfile {
        sale_kinds: BTreeSet::from([CommodityKind::Bread, CommodityKind::Water]),
        home_facility: Some(entity_id(7, 2)),
    }
}

/// Returns a representative trade disposition fixture for authoritative component tests.
pub fn sample_trade_disposition_profile() -> TradeDispositionProfile {
    TradeDispositionProfile {
        negotiation_round_ticks: NonZeroU32::new(6).unwrap(),
        initial_offer_bias: Permille::new(650).unwrap(),
        concession_rate: Permille::new(125).unwrap(),
        rejection_escalation_rate: Permille::new(200).unwrap(),
        demand_memory_retention_ticks: 240,
        market_presence_ticks: NonZeroU32::new(30).unwrap(),
    }
}

/// Returns a representative stock storage policy fixture for facility tests.
pub fn sample_stock_storage_policy() -> StockStoragePolicy {
    StockStoragePolicy {
        stock_container: entity_id(8, 1),
        display_container: Some(entity_id(8, 2)),
    }
}

/// Returns a representative stock assignment fixture for item lot tests.
pub fn sample_stock_assignment() -> StockAssignment {
    StockAssignment {
        facility: entity_id(7, 3),
        kind: StockAssignmentKind::Stored,
    }
}

/// Returns a representative generalized-contention disposition fixture.
pub fn sample_contention_disposition_profile() -> ContentionDispositionProfile {
    ContentionDispositionProfile {
        queue_patience_ticks: NonZeroU32::new(12),
    }
}

/// Returns a representative utility profile fixture for decision-architecture tests.
pub fn sample_utility_profile() -> UtilityProfile {
    UtilityProfile {
        hunger_weight: Permille::new(900).unwrap(),
        thirst_weight: Permille::new(850).unwrap(),
        fatigue_weight: Permille::new(700).unwrap(),
        bladder_weight: Permille::new(650).unwrap(),
        dirtiness_weight: Permille::new(300).unwrap(),
        pain_weight: Permille::new(950).unwrap(),
        danger_weight: Permille::new(1000).unwrap(),
        enterprise_weight: Permille::new(425).unwrap(),
        social_weight: Permille::new(200).unwrap(),
        activity_awareness_weight: Permille::new(250).unwrap(),
        side_benefit_weight: Permille::new(100).unwrap(),
        bounty_posting_weight: Permille::new(0).unwrap(),
        notice_posting_weight: Permille::new(0).unwrap(),
        courage: Permille::new(350).unwrap(),
        care_weight: Permille::new(250).unwrap(),
        office_duty_weight: Permille::new(575).unwrap(),
        loyalty_weight: Permille::new(525).unwrap(),
        greed_weight: Permille::new(475).unwrap(),
        shame_weight: Permille::new(350).unwrap(),
        revenge_weight: Permille::new(300).unwrap(),
    }
}

/// Returns a representative commodity valuation profile fixture.
pub fn sample_commodity_valuation_profile() -> CommodityValuationProfile {
    CommodityValuationProfile {
        recipe_opportunity_depth: NonZeroU8::new(3).unwrap(),
        recipe_place_horizon: 2,
        indirect_value_decay_per_step: Permille::new(150).unwrap(),
    }
}

/// Returns a representative route experience fixture.
pub fn sample_route_experience() -> RouteExperience {
    RouteExperience {
        edges: BTreeMap::from([(
            TravelEdgeId(3),
            EdgeExperience {
                safe_trips: 4,
                hostile_encounters: 1,
                last_travel_tick: Tick(19),
            },
        )]),
    }
}

/// Returns a representative source reliability fixture.
pub fn sample_source_reliability() -> SourceReliability {
    SourceReliability {
        sources: BTreeMap::from([(
            SourceKey {
                entity: entity_id(9, 0),
                commodity: CommodityKind::Bread,
            },
            ReliabilityRecord {
                successful_acquisitions: 3,
                failed_attempts: 1,
                last_attempt_tick: Tick(21),
                average_wait_ticks: 4,
                wait_observation_count: 2,
                last_observed_capacity: 7,
                last_observed_capacity_tick: Tick(20),
            },
        )]),
    }
}

/// Returns a representative learned-preference profile fixture.
pub fn sample_preference_profile() -> PreferenceProfile {
    PreferenceProfile {
        route_caution_weight: Permille::new(300).unwrap(),
        source_trust_weight: Permille::new(200).unwrap(),
        route_memory_capacity: 24,
        source_memory_capacity: 18,
        memory_retention_ticks: 400,
        wait_sensitivity_weight: Permille::new(150).unwrap(),
        capacity_observation_weight: Permille::new(20).unwrap(),
    }
}

/// Returns a representative canonical goal identity fixture.
pub fn sample_goal_key() -> GoalKey {
    GoalKey::from(GoalKind::AcquireCommodity {
        commodity: CommodityKind::Bread,
        purpose: CommodityPurpose::SelfConsume,
        quantity: AcquisitionQuantity::single(),
    })
}

/// Returns a representative blocker key fixture for blocked-intent tests.
pub fn sample_blocker_key() -> BlockerKey {
    BlockerKey {
        goal_key: sample_goal_key(),
        place: Some(entity_id(3, 0)),
        target: Some(entity_id(8, 0)),
        action_def: Some(ActionDefId(2)),
    }
}

/// Returns a representative blocker scope fixture for blocked-intent tests.
pub fn sample_blocker_scope() -> BlockerScope {
    BlockerScope::Exact(sample_blocker_key())
}

/// Returns a representative blocked intent fixture for decision-memory tests.
pub fn sample_blocker() -> Blocker {
    Blocker {
        scope: sample_blocker_scope(),
        blocking_fact: BlockingFact::SellerOutOfStock,
        diagnostic_context: None,
        observed_tick: Tick(10),
        expires_tick: Tick(15),
        clearing_condition: BlockerClearingCondition::CommodityAvailabilityChanged {
            commodity: CommodityKind::Bread,
            place: entity_id(3, 0),
        },
        baseline_snapshot: Some(ClearingBaseline::CommodityQuantity {
            quantity: Quantity(2),
        }),
        source_event: Some(crate::EventId(1)),
    }
}

/// Returns a representative blocked intent memory fixture for authoritative component tests.
pub fn sample_blocker_memory() -> BlockerMemory {
    let intent = sample_blocker();
    let mut intents = std::collections::BTreeMap::new();
    intents.insert(intent.scope, intent);
    BlockerMemory { intents }
}

/// Returns a representative discrepancy memory fixture for authoritative component tests.
pub fn sample_discrepancy_memory() -> DiscrepancyMemory {
    let entry = DiscrepancyEntry {
        scope: sample_blocker_scope(),
        discrepancy: Discrepancy::BeliefContradicted,
        observed_tick: Tick(12),
        expires_tick: Tick(18),
        clearing_condition: DiscrepancyClearing::TtlExpiry,
        source_event: Some(crate::EventId(1)),
    };
    let mut entries = BTreeMap::new();
    entries.insert(entry.scope, entry);
    DiscrepancyMemory { entries }
}

/// Returns a representative repair memory fixture for authoritative component tests.
pub fn sample_repair_memory() -> RepairMemory {
    let entry = RepairEntry {
        signature: BreachSignature {
            goal_key: sample_goal_key(),
            invalidator: InvalidatorTag::TargetMoved,
            step_target: Some(entity_id(14, 0)),
        },
        kind: RepairKind::RebindTarget,
        succeeded: true,
        observed_tick: Tick(13),
        expires_tick: Tick(133),
        success_count: 2,
    };
    let mut repairs = BTreeMap::new();
    repairs.insert(entry.signature, entry);
    RepairMemory { repairs }
}

/// Returns a representative learned-opportunity memory fixture for authoritative component tests.
pub fn sample_learned_opportunity_memory() -> LearnedOpportunityMemory {
    let entry = OpportunityEntry {
        opportunity: OpportunityKey {
            goal_key: sample_goal_key(),
            anchor: OpportunityAnchor::Place(entity_id(15, 0)),
        },
        observed_tick: Tick(14),
        expires_tick: Tick(74),
        observed_at: entity_id(16, 0),
        source: LearnedOpportunitySource::ReadPhaseInference,
    };
    let mut opportunities = BTreeMap::new();
    opportunities.insert(entry.opportunity, entry);
    LearnedOpportunityMemory { opportunities }
}

/// Returns a representative memory-capacity profile fixture for authoritative component tests.
pub fn sample_memory_capacity_profile() -> MemoryCapacityProfile {
    MemoryCapacityProfile {
        memory_capacity: 24,
    }
}

/// Returns a representative substitute-preference fixture for trade-domain tests.
pub fn sample_substitute_preferences() -> SubstitutePreferences {
    SubstitutePreferences {
        preferences: BTreeMap::from([
            (TradeCategory::Medicine, vec![CommodityKind::Medicine]),
            (
                TradeCategory::Food,
                vec![
                    CommodityKind::Bread,
                    CommodityKind::Apple,
                    CommodityKind::Grain,
                ],
            ),
            (TradeCategory::Fuel, vec![CommodityKind::Firewood]),
        ]),
    }
}
