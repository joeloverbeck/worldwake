use crate::{
    ActionDefRegistry, ActionHandlerRegistry, DeterministicRng, RecipeRegistry, ReplanNeeded,
    Scheduler,
};
use std::fmt;
use worldwake_core::{EventLog, Tick, World};

pub struct TickInputContext<'a> {
    pub world: &'a mut World,
    pub event_log: &'a mut EventLog,
    pub scheduler: &'a mut Scheduler,
    pub rng: &'a mut DeterministicRng,
    pub action_defs: &'a ActionDefRegistry,
    pub action_handlers: &'a ActionHandlerRegistry,
    pub recipe_registry: &'a RecipeRegistry,
    pub pending_replans: &'a [ReplanNeeded],
    pub tick: Tick,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TickInputError {
    message: String,
}

impl TickInputError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for TickInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for TickInputError {}

pub trait TickInputProducer {
    fn produce_inputs(&mut self, ctx: TickInputContext<'_>) -> Result<(), TickInputError>;
}
