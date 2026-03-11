//! # worldwake-systems
//!
//! Simulation systems: needs, production, trade, combat, perception, politics.
//! Depends on `worldwake-core` and `worldwake-sim`.

pub mod needs;
pub mod needs_actions;
pub mod production;
pub mod production_actions;
pub mod trade;
pub mod trade_actions;
pub mod transport_actions;
pub mod travel_actions;

pub use needs::needs_system;
pub use needs_actions::register_needs_actions;
pub use production::resource_regeneration_system;
pub use production_actions::{register_craft_actions, register_harvest_actions};
pub use trade::trade_system_tick;
pub use trade_actions::register_trade_action;
pub use transport_actions::register_transport_actions;
pub use travel_actions::register_travel_actions;

use worldwake_sim::{SystemDispatchTable, SystemError, SystemExecutionContext};

pub fn dispatch_table() -> SystemDispatchTable {
    SystemDispatchTable::from_handlers([
        needs_system,
        resource_regeneration_system,
        trade_system_tick,
        noop_system,
        noop_system,
        noop_system,
    ])
}

#[allow(clippy::unnecessary_wraps)]
fn noop_system(_ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    Ok(())
}
