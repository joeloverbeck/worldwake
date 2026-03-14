//! Shared test utilities for the Worldwake simulation.
//!
//! These helpers are available to all crates in the workspace for
//! deterministic testing.

use crate::{
    ActionDefId, BlockedIntent, BlockedIntentMemory, BlockingFact, CommodityKind, CommodityPurpose,
    DemandMemory, DemandObservation, DemandObservationReason, EntityId,
    FacilityQueueDispositionProfile, GoalKey, GoalKind, MerchandiseProfile, Permille, Quantity,
    Seed, SubstitutePreferences, Tick, TradeCategory, TradeDispositionProfile,
    TravelDispositionProfile, UtilityProfile,
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
    }
}

/// Returns a representative travel disposition fixture for authoritative component tests.
pub fn sample_travel_disposition_profile() -> TravelDispositionProfile {
    TravelDispositionProfile {
        route_replan_margin: Permille::new(180).unwrap(),
        blocked_leg_patience_ticks: NonZeroU32::new(9).unwrap(),
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
    }
}

/// Returns a representative canonical goal identity fixture.
pub fn sample_goal_key() -> GoalKey {
    GoalKey::from(GoalKind::AcquireCommodity {
        commodity: CommodityKind::Bread,
        purpose: CommodityPurpose::SelfConsume,
    })
}

/// Returns a representative blocked intent fixture for decision-memory tests.
pub fn sample_blocked_intent() -> BlockedIntent {
    BlockedIntent {
        goal_key: sample_goal_key(),
        blocking_fact: BlockingFact::SellerOutOfStock,
        related_entity: Some(entity_id(8, 0)),
        related_place: Some(entity_id(3, 0)),
        related_action: Some(ActionDefId(2)),
        observed_tick: Tick(10),
        expires_tick: Tick(15),
    }
}

/// Returns a representative blocked intent memory fixture for authoritative component tests.
pub fn sample_blocked_intent_memory() -> BlockedIntentMemory {
    BlockedIntentMemory {
        intents: vec![sample_blocked_intent()],
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
