//! # worldwake-systems
//!
//! Simulation systems: needs, production, trade, combat, perception, politics.
//! Depends on `worldwake-core` and `worldwake-sim`.

pub mod action_registry;
pub mod artifact_actions;
pub mod artifact_lifecycle;
pub mod ask_about_person_actions;
pub mod bandit_camp;
pub mod bandit_camp_actions;
pub mod combat;
mod commodity_support;
pub mod consult_record_actions;
mod contention_support;
pub mod epistemic_actions;
pub mod escort_actions;
pub mod evidence_decay;
mod evidence_support;
pub mod expectation_check;
mod experience_recording;
pub mod facility_queue;
pub mod facility_queue_actions;
mod inventory;
pub mod investigate_actions;
pub mod item_decay;
pub mod justice_actions;
pub mod needs;
pub mod needs_actions;
pub mod office_actions;
pub mod offices;
pub mod patrol;
pub mod patrol_actions;
pub mod perception;
pub mod production;
pub mod production_actions;
pub mod report_actions;
pub mod search_actions;
pub mod stock_actions;
pub mod tell_actions;
pub mod trade;
pub mod trade_actions;
pub mod transport_actions;
pub mod travel_actions;

pub use action_registry::{
    ActionRegistries, build_canonical_production_recipe_registry, build_full_action_registries,
    register_all_actions,
};
pub use artifact_actions::register_artifact_actions;
pub use artifact_lifecycle::artifact_lifecycle_system;
pub use ask_about_person_actions::register_ask_about_person_action;
pub use bandit_camp::bandit_camp_system;
pub use bandit_camp_actions::register_establish_camp_action;
pub use combat::{
    combat_system, register_attack_action, register_bury_action, register_defend_action,
    register_heal_action, register_loot_action, register_queue_for_care_target_action,
    register_queue_for_corpse_use_action,
};
pub use consult_record_actions::register_consult_record_action;
pub use epistemic_actions::register_ask_witness_action;
pub use escort_actions::register_escort_to_safety_action;
pub use evidence_decay::evidence_decay_system;
pub use expectation_check::check_overdue_expectations;
pub use facility_queue::contention_system;
pub use facility_queue_actions::register_queue_for_facility_use_action;
pub use investigate_actions::register_investigate_action;
pub use item_decay::item_decay_system;
pub use justice_actions::{register_accuse_action, register_exile_action, register_fine_action};
pub use needs::needs_system;
pub use needs_actions::register_needs_actions;
pub use office_actions::register_office_actions;
pub use offices::{
    count_present_hostile_faction_pairs_at, office_is_vacant, offices_with_jurisdiction,
    public_order, succession_system,
};
pub use patrol::patrol_route_adaptation_system;
pub use patrol_actions::register_patrol_action;
pub use perception::perception_system;
pub use production::resource_regeneration_system;
pub use production_actions::{register_craft_actions, register_harvest_actions};
pub use report_actions::{register_report_found_action, register_report_missing_action};
pub use search_actions::register_search_place_action;
pub use stock_actions::register_stock_actions;
pub use tell_actions::register_tell_action;
pub use trade::{restock_candidates, trade_system_tick};
pub use trade_actions::{register_staff_market_action, register_trade_action};
pub use transport_actions::register_transport_actions;
pub use travel_actions::register_travel_actions;

use worldwake_sim::{SystemDispatchTable, compact_event_log};

pub fn dispatch_table() -> SystemDispatchTable {
    SystemDispatchTable::from_handlers([
        needs_system,
        resource_regeneration_system,
        trade_system_tick,
        combat_system,
        artifact_lifecycle_system,
        contention_system,
        succession_system,
        perception_system,
        bandit_camp_system,
        patrol_route_adaptation_system,
        evidence_decay_system,
        item_decay_system,
        check_overdue_expectations,
        compact_event_log,
    ])
}
