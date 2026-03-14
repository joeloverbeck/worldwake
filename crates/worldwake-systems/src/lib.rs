//! # worldwake-systems
//!
//! Simulation systems: needs, production, trade, combat, perception, politics.
//! Depends on `worldwake-core` and `worldwake-sim`.

pub mod action_registry;
pub mod combat;
pub mod facility_queue;
pub mod facility_queue_actions;
mod inventory;
pub mod needs;
pub mod needs_actions;
pub mod perception;
pub mod production;
pub mod production_actions;
pub mod trade;
pub mod trade_actions;
pub mod transport_actions;
pub mod travel_actions;

pub use action_registry::{build_full_action_registries, register_all_actions, ActionRegistries};
pub use combat::{
    combat_system, register_attack_action, register_bury_action, register_defend_action,
    register_heal_action, register_loot_action,
};
pub use facility_queue::facility_queue_system;
pub use facility_queue_actions::register_queue_for_facility_use_action;
pub use needs::needs_system;
pub use needs_actions::register_needs_actions;
pub use perception::perception_system;
pub use production::resource_regeneration_system;
pub use production_actions::{register_craft_actions, register_harvest_actions};
pub use trade::{restock_candidates, trade_system_tick};
pub use trade_actions::register_trade_action;
pub use transport_actions::register_transport_actions;
pub use travel_actions::register_travel_actions;

use worldwake_sim::{SystemDispatchTable, SystemError, SystemExecutionContext};

pub fn dispatch_table() -> SystemDispatchTable {
    SystemDispatchTable::from_handlers([
        needs_system,
        resource_regeneration_system,
        trade_system_tick,
        combat_system,
        facility_queue_system,
        perception_system,
        noop_system,
    ])
}

#[allow(clippy::unnecessary_wraps)]
fn noop_system(_ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    Ok(())
}
