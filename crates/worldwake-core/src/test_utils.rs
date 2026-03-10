//! Shared test utilities for the Worldwake simulation.
//!
//! These helpers are available to all crates in the workspace for
//! deterministic testing.

use crate::{
    CommodityKind, DemandMemory, DemandObservation, DemandObservationReason, EntityId,
    MerchandiseProfile, Quantity, Seed, Tick,
};
use std::collections::BTreeSet;

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
