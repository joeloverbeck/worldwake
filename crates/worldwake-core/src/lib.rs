//! # worldwake-core
//!
//! Foundation crate for the Worldwake simulation: IDs, types, ECS store,
//! topology, items, and relations. Has no internal crate dependencies.
//!
//! ## Deterministic Data Policy
//!
//! All authoritative simulation state **must** use only deterministic,
//! serializable data structures:
//!
//! **Allowed** in authoritative state:
//! - `Vec`, `Option`, `BTreeMap`, `BTreeSet`
//! - Fixed-width integers (`u8`..`u128`, `i8`..`i128`)
//! - Enums / structs composed of the above
//!
//! **Forbidden** in authoritative or hashed state:
//! - `HashMap`, `HashSet` (non-deterministic iteration order)
//! - `TypeId`, `Box<dyn Any>` (opaque, not serializable)
//! - Raw pointer identity
//! - Wall-clock time
//! - Floating-point values unless there is a written exception and a
//!   canonicalization rule
//!
//! This policy is enforced by integration tests that scan source files for
//! forbidden patterns.

pub mod allocator;
pub mod canonical;
pub mod cause;
pub mod component_schema;
pub mod component_tables;
pub mod components;
pub mod conservation;
pub mod control;
pub mod delta;
pub mod entity;
pub mod error;
pub mod event_log;
pub mod event_record;
pub mod event_tag;
pub mod ids;
pub mod items;
pub mod load;
pub mod numerics;
pub mod relations;
pub mod test_utils;
pub mod topology;
pub mod traits;
pub mod verification;
pub mod visibility;
pub mod witness;
pub mod world;
pub mod world_txn;
pub mod wounds;

pub use allocator::EntityAllocator;
pub use canonical::{
    canonical_bytes, hash_bytes, hash_event_log, hash_serializable, hash_world, CanonicalError,
    StateHash,
};
pub use cause::CauseRef;
pub use component_tables::ComponentTables;
pub use components::{AgentData, Name};
pub use conservation::{total_commodity_quantity, verify_conservation};
pub use control::ControlSource;
pub use delta::{
    ComponentDelta, ComponentKind, ComponentValue, EntityDelta, QuantityDelta, RelationDelta,
    RelationKind, RelationValue, ReservationDelta, StateDelta,
};
pub use entity::{EntityKind, EntityMeta};
pub use error::WorldError;
pub use event_log::EventLog;
pub use event_record::{EventRecord, PendingEvent};
pub use event_tag::EventTag;
pub use ids::{EntityId, EventId, FactId, ReservationId, Seed, Tick, TickRange, TravelEdgeId};
pub use items::{
    CommodityKind, CommodityKindSpec, CommodityPhysicalProfile, Container, ItemLot, LotOperation,
    ProvenanceEntry, TradeCategory, UniqueItem, UniqueItemKind, UniqueItemKindSpec,
    UniqueItemPhysicalProfile,
};
pub use load::{
    current_container_load, load_of_entity, load_of_lot, load_of_unique_item,
    load_of_unique_item_kind, load_per_unit, remaining_container_capacity,
};
pub use numerics::{LoadUnits, Permille, Quantity};
pub use relations::{ArchiveDependency, ArchiveDependencyKind, RelationTables, ReservationRecord};
pub use topology::{build_prototype_world, Place, PlaceTag, Route, Topology, TravelEdge};
pub use traits::{Component, RelationRecord};
pub use verification::{verify_completeness, VerificationError};
pub use visibility::VisibilitySpec;
pub use witness::WitnessData;
pub use world::lifecycle::{
    ArchiveMutationSnapshot, ArchivePreparationAction, ArchivePreparationPlan,
    ArchivePreparationPolicy, ArchivePreparationReport, ArchiveResolution,
};
pub use world::World;
pub use world_txn::WorldTxn;
pub use wounds::{BodyPart, DeprivationKind, Wound, WoundCause, WoundList};
