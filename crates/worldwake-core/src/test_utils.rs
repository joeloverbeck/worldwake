//! Shared test utilities for the Worldwake simulation.
//!
//! These helpers are available to all crates in the workspace for
//! deterministic testing.

use crate::{
    ActionDefId, BlockedIntent, BlockedIntentMemory, BlockerKey, BlockingFact, CommodityKind,
    CommodityPurpose, DemandMemory, DemandObservation, DemandObservationReason, EntityId,
    FacilityQueueDispositionProfile, GoalKey, GoalKind, MerchandiseProfile, Permille, Quantity,
    Seed, StockAssignment, StockAssignmentKind, StockStoragePolicy, SubstitutePreferences, Tick,
    TradeCategory, TradeDispositionProfile, UtilityProfile,
};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

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
        home_market: Some(entity_id(7, 2)),
    }
}

/// Returns a representative trade disposition fixture for authoritative component tests.
pub fn sample_trade_disposition_profile() -> TradeDispositionProfile {
    TradeDispositionProfile {
        negotiation_round_ticks: NonZeroU32::new(6).unwrap(),
        initial_offer_bias: Permille::new(650).unwrap(),
        concession_rate: Permille::new(125).unwrap(),
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

/// Returns a representative facility-queue disposition fixture.
pub fn sample_facility_queue_disposition_profile() -> FacilityQueueDispositionProfile {
    FacilityQueueDispositionProfile {
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
        courage: Permille::new(350).unwrap(),
        care_weight: Permille::new(250).unwrap(),
    }
}

/// Returns a representative canonical goal identity fixture.
pub fn sample_goal_key() -> GoalKey {
    GoalKey::from(GoalKind::AcquireCommodity {
        commodity: CommodityKind::Bread,
        purpose: CommodityPurpose::SelfConsume,
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

/// Returns a representative blocked intent fixture for decision-memory tests.
pub fn sample_blocked_intent() -> BlockedIntent {
    BlockedIntent {
        blocker_key: sample_blocker_key(),
        blocking_fact: BlockingFact::SellerOutOfStock,
        diagnostic_context: None,
        observed_tick: Tick(10),
        expires_tick: Tick(15),
    }
}

/// Returns a representative blocked intent memory fixture for authoritative component tests.
pub fn sample_blocked_intent_memory() -> BlockedIntentMemory {
    let intent = sample_blocked_intent();
    let mut intents = std::collections::BTreeMap::new();
    intents.insert(intent.blocker_key, intent);
    BlockedIntentMemory { intents }
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
