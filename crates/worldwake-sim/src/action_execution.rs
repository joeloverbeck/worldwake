use crate::{
    ActionDefRegistry, ActionInstance, ActionInstanceId, DeterministicRng, RecipeRegistry,
};
use std::{collections::BTreeMap, sync::OnceLock};
use worldwake_core::{CauseRef, EventLog, Tick, World};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ActionExecutionContext<'a> {
    pub cause: CauseRef,
    pub tick: Tick,
    pub recipe_registry: &'a RecipeRegistry,
    pub action_defs: &'a ActionDefRegistry,
}

impl ActionExecutionContext<'static> {
    #[must_use]
    pub fn without_recipes(cause: CauseRef, tick: Tick) -> Self {
        static EMPTY_RECIPES: OnceLock<RecipeRegistry> = OnceLock::new();
        static EMPTY_ACTION_DEFS: OnceLock<ActionDefRegistry> = OnceLock::new();
        Self {
            cause,
            tick,
            recipe_registry: EMPTY_RECIPES.get_or_init(RecipeRegistry::new),
            action_defs: EMPTY_ACTION_DEFS.get_or_init(ActionDefRegistry::new),
        }
    }
}

pub struct ActionExecutionAuthority<'a> {
    pub active_actions: &'a mut BTreeMap<ActionInstanceId, ActionInstance>,
    pub world: &'a mut World,
    pub event_log: &'a mut EventLog,
    pub rng: &'a mut DeterministicRng,
}
