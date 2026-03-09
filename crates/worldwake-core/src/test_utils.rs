//! Shared test utilities for the Worldwake simulation.
//!
//! These helpers are available to all crates in the workspace for
//! deterministic testing.

use crate::Seed;
use serde::Serialize;

/// Returns a fixed, well-known seed for deterministic test scenarios.
pub fn deterministic_seed() -> Seed {
    // All zeros — simple, memorable, deterministic.
    Seed([0u8; 32])
}

/// Serialize a value to canonical bytes using bincode.
///
/// The same input must always produce the same output within a build,
/// enabling stable hashing and snapshot comparisons.
pub fn canonical_bytes<T: Serialize>(val: &T) -> Vec<u8> {
    bincode::serialize(val).expect("canonical_bytes: serialization must not fail")
}
