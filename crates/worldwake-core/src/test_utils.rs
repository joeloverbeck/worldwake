//! Shared test utilities for the Worldwake simulation.
//!
//! These helpers are available to all crates in the workspace for
//! deterministic testing.

use crate::Seed;

/// Returns a fixed, well-known seed for deterministic test scenarios.
pub fn deterministic_seed() -> Seed {
    // All zeros — simple, memorable, deterministic.
    Seed([0u8; 32])
}
