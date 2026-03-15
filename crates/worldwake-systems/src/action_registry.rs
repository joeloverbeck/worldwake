use crate::{
    register_attack_action, register_bury_action, register_craft_actions, register_defend_action,
    register_harvest_actions, register_heal_action, register_loot_action, register_needs_actions,
    register_office_actions, register_queue_for_facility_use_action, register_tell_action,
    register_trade_action, register_transport_actions, register_travel_actions,
};
use worldwake_core::ActionDefId;
use worldwake_sim::{
    action_handler_registry::verify_completeness, ActionDefRegistry, ActionHandlerRegistry,
    RecipeRegistry,
};

pub struct ActionRegistries {
    pub defs: ActionDefRegistry,
    pub handlers: ActionHandlerRegistry,
}

pub fn register_all_actions(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
    recipes: &RecipeRegistry,
) {
    register_needs_actions(defs, handlers);
    let _ = register_queue_for_facility_use_action(defs, handlers);
    let _ = register_harvest_actions(defs, handlers, recipes);
    let _ = register_craft_actions(defs, handlers, recipes);
    let _ = register_trade_action(defs, handlers);
    let _ = register_tell_action(defs, handlers);
    let _ = register_office_actions(defs, handlers);
    let _ = register_travel_actions(defs, handlers);
    let _ = register_transport_actions(defs, handlers);
    let _ = register_attack_action(defs, handlers);
    let _ = register_defend_action(defs, handlers);
    let _ = register_loot_action(defs, handlers);
    let _ = register_bury_action(defs, handlers);
    let _ = register_heal_action(defs, handlers);
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
    use super::build_full_action_registries;
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
            "trade",
            "tell",
            "bribe",
            "threaten",
            "declare_support",
            "travel",
            "pick_up",
            "put_down",
            "attack",
            "defend",
            "loot",
            "bury",
            "heal",
        ] {
            assert!(
                action_names.contains(&required),
                "full registry should include {required}"
            );
        }
    }
}
