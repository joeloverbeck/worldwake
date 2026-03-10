//! # worldwake-systems
//!
//! Simulation systems: needs, production, trade, combat, perception, politics.
//! Depends on `worldwake-core` and `worldwake-sim`.

pub mod needs;
pub mod needs_actions;

pub use needs::{dispatch_table, needs_system};
pub use needs_actions::register_needs_actions;
