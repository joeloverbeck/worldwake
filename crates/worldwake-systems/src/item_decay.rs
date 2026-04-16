use worldwake_sim::{SystemError, SystemExecutionContext};

#[allow(clippy::unnecessary_wraps)]
pub fn item_decay_system(_ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::item_decay_system;
    use crate::dispatch_table;
    use std::collections::BTreeMap;
    use worldwake_core::{EventLog, Seed, Tick, World, build_prototype_world};
    use worldwake_sim::{ActionDefRegistry, DeterministicRng, SystemExecutionContext, SystemId};

    #[test]
    fn dispatch_table_routes_item_decay_to_stub() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([3; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        dispatch_table().get(SystemId::ItemDecay)(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(5),
            system_id: SystemId::ItemDecay,
        })
        .unwrap();
    }

    #[test]
    fn item_decay_stub_returns_ok() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([4; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        item_decay_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(8),
            system_id: SystemId::ItemDecay,
        })
        .unwrap();
    }
}
