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

pub mod control;
pub mod error;
pub mod ids;
pub mod numerics;
pub mod test_utils;
pub mod traits;

pub use control::ControlSource;
pub use error::WorldError;
pub use ids::{EntityId, EventId, Seed, Tick};
pub use numerics::{LoadUnits, Permille, Quantity};
pub use traits::{Component, RelationRecord};
