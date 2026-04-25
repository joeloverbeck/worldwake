use crate::{
    register_accuse_action, register_artifact_actions, register_ask_about_person_action,
    register_ask_witness_action, register_attack_action, register_bury_action,
    register_consult_record_action, register_craft_actions, register_defend_action,
    register_escort_to_safety_action, register_establish_camp_action, register_exile_action,
    register_fine_action, register_harvest_actions, register_heal_action,
    register_investigate_action, register_loot_action, register_needs_actions,
    register_office_actions, register_patrol_action, register_queue_for_care_target_action,
    register_queue_for_corpse_use_action, register_queue_for_facility_use_action,
    register_report_found_action, register_report_missing_action, register_search_place_action,
    register_staff_market_action, register_stock_actions, register_tell_action,
    register_trade_action, register_transport_actions, register_travel_actions,
};
use std::num::NonZeroU32;
use worldwake_core::ActionDefId;
use worldwake_core::{BodyCostPerTick, CommodityKind, Permille, Quantity, WorkstationTag};
use worldwake_sim::{
    ActionDefRegistry, ActionHandlerRegistry, RecipeDefinition, RecipeRegistry,
    action_handler_registry::verify_completeness,
};

pub struct ActionRegistries {
    pub defs: ActionDefRegistry,
    pub handlers: ActionHandlerRegistry,
}

fn nz(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("canonical recipe ticks must be non-zero")
}

fn pm(value: u16) -> Permille {
    Permille::new(value).expect("canonical recipe permille must be in range")
}

pub fn build_canonical_production_recipe_registry() -> RecipeRegistry {
    let mut recipes = RecipeRegistry::new();
    recipes.register(RecipeDefinition {
        name: "Harvest Apples".to_string(),
        inputs: vec![],
        outputs: vec![(CommodityKind::Apple, Quantity(2))],
        work_ticks: nz(3),
        required_workstation_tag: Some(WorkstationTag::OrchardRow),
        required_tool_kinds: vec![],
        body_cost_per_tick: BodyCostPerTick::new(pm(3), pm(2), pm(5), pm(0), pm(1)),
    });
    recipes.register(RecipeDefinition {
        name: "Harvest Grain".to_string(),
        inputs: vec![],
        outputs: vec![(CommodityKind::Grain, Quantity(2))],
        work_ticks: nz(3),
        required_workstation_tag: Some(WorkstationTag::FieldPlot),
        required_tool_kinds: vec![],
        body_cost_per_tick: BodyCostPerTick::new(pm(3), pm(2), pm(5), pm(0), pm(1)),
    });
    recipes.register(RecipeDefinition {
        name: "Harvest Water".to_string(),
        inputs: vec![],
        outputs: vec![(CommodityKind::Water, Quantity(2))],
        work_ticks: nz(3),
        required_workstation_tag: Some(WorkstationTag::Well),
        required_tool_kinds: vec![],
        body_cost_per_tick: BodyCostPerTick::new(pm(3), pm(2), pm(5), pm(0), pm(1)),
    });
    recipes.register(RecipeDefinition {
        name: "Bake Bread".to_string(),
        inputs: vec![(CommodityKind::Firewood, Quantity(1))],
        outputs: vec![(CommodityKind::Bread, Quantity(1))],
        work_ticks: nz(3),
        required_workstation_tag: Some(WorkstationTag::Mill),
        required_tool_kinds: vec![],
        body_cost_per_tick: BodyCostPerTick::new(pm(3), pm(2), pm(5), pm(0), pm(1)),
    });
    recipes
}

pub fn register_all_actions(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
    recipes: &RecipeRegistry,
) {
    register_needs_actions(defs, handlers);
    let _ = register_queue_for_facility_use_action(defs, handlers);
    let _ = register_queue_for_corpse_use_action(defs, handlers);
    let _ = register_queue_for_care_target_action(defs, handlers);
    let _ = register_harvest_actions(defs, handlers, recipes);
    let _ = register_craft_actions(defs, handlers, recipes);
    let _ = register_trade_action(defs, handlers);
    let _ = register_staff_market_action(defs, handlers);
    let _ = register_tell_action(defs, handlers);
    let _ = register_consult_record_action(defs, handlers);
    let _ = register_office_actions(defs, handlers);
    let _ = register_artifact_actions(defs, handlers);
    let _ = register_travel_actions(defs, handlers);
    let _ = register_transport_actions(defs, handlers);
    let _ = register_attack_action(defs, handlers);
    let _ = register_defend_action(defs, handlers);
    let _ = register_loot_action(defs, handlers);
    let _ = register_bury_action(defs, handlers);
    let _ = register_heal_action(defs, handlers);
    let _ = register_establish_camp_action(defs, handlers);
    let _ = register_investigate_action(defs, handlers);
    let _ = register_patrol_action(defs, handlers);
    let _ = register_ask_witness_action(defs, handlers);
    let _ = register_ask_about_person_action(defs, handlers);
    let _ = register_search_place_action(defs, handlers);
    let _ = register_report_missing_action(defs, handlers);
    let _ = register_report_found_action(defs, handlers);
    let _ = register_escort_to_safety_action(defs, handlers);
    let _ = register_accuse_action(defs, handlers);
    let _ = register_fine_action(defs, handlers);
    let _ = register_exile_action(defs, handlers);
    let _ = register_stock_actions(defs, handlers);
}

pub fn build_full_action_registries(
    recipes: &RecipeRegistry,
) -> Result<ActionRegistries, Vec<ActionDefId>> {
    let mut defs = ActionDefRegistry::new();
    let mut handlers = ActionHandlerRegistry::new();
    register_all_actions(&mut defs, &mut handlers, recipes);
    verify_completeness(&defs, &handlers)?;
    Ok(ActionRegistries { defs, handlers })
}

#[cfg(test)]
mod tests {
    use super::{build_canonical_production_recipe_registry, build_full_action_registries};
    use worldwake_sim::RecipeRegistry;

    #[test]
    fn build_full_action_registries_returns_complete_action_catalog() {
        let recipes = RecipeRegistry::new();
        let registries = build_full_action_registries(&recipes).unwrap();

        assert!(!registries.defs.is_empty());
        assert!(!registries.handlers.is_empty());

        let action_names = registries
            .defs
            .iter()
            .map(|def| def.name.as_str())
            .collect::<Vec<_>>();

        for required in [
            "eat",
            "drink",
            "sleep",
            "toilet",
            "wash",
            "queue_for_facility_use",
            "queue_for_corpse_use",
            "queue_for_care_target",
            "trade",
            "staff_market",
            "tell",
            "consult_record",
            "post_bounty",
            "post_notice",
            "claim_bounty",
            "withdraw_bounty",
            "bribe",
            "threaten",
            "declare_support",
            "press_force_claim",
            "yield_force_claim",
            "travel",
            "pick_up",
            "put_down",
            "steal",
            "attack",
            "defend",
            "loot",
            "bury",
            "heal",
            "establish_camp",
            "investigate",
            "patrol",
            "ask_witness",
            "ask_about_person",
            "search_place",
            "report_missing",
            "report_found",
            "escort_to_safety",
            "accuse",
            "fine",
            "exile",
        ] {
            assert!(
                action_names.contains(&required),
                "full registry should include {required}"
            );
        }
    }

    #[test]
    fn canonical_production_recipe_registry_contains_expected_recipes() {
        let recipes = build_canonical_production_recipe_registry();

        assert!(recipes.recipe_by_name("Harvest Apples").is_some());
        assert!(recipes.recipe_by_name("Harvest Grain").is_some());
        assert!(recipes.recipe_by_name("Harvest Water").is_some());
        assert!(recipes.recipe_by_name("Bake Bread").is_some());
    }
}
