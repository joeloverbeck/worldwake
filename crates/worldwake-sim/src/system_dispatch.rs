use crate::{
    ActionDefRegistry, ActionInstance, ActionInstanceId, DeterministicRng, PoliticalTraceSink,
    SystemId,
};
use std::collections::BTreeMap;
use std::fmt;
use worldwake_core::{EventLog, Tick, World};

const SYSTEM_COUNT: usize = SystemId::ALL.len();

pub type SystemFn = fn(SystemExecutionContext<'_>) -> Result<(), SystemError>;

pub struct SystemExecutionContext<'a> {
    pub world: &'a mut World,
    pub event_log: &'a mut EventLog,
    pub rng: &'a mut DeterministicRng,
    pub active_actions: &'a BTreeMap<ActionInstanceId, ActionInstance>,
    pub action_defs: &'a ActionDefRegistry,
    pub politics_trace: Option<&'a mut PoliticalTraceSink>,
    pub tick: Tick,
    pub system_id: SystemId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemError {
    message: String,
}

impl SystemError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SystemError {}

pub struct SystemDispatchTable {
    handlers: [SystemFn; SYSTEM_COUNT],
}

impl SystemDispatchTable {
    #[must_use]
    pub const fn from_handlers(handlers: [SystemFn; SYSTEM_COUNT]) -> Self {
        Self { handlers }
    }

    #[must_use]
    pub const fn canonical_noop() -> Self {
        Self::from_handlers([noop_system; SYSTEM_COUNT])
    }

    #[must_use]
    pub fn get(&self, system_id: SystemId) -> SystemFn {
        self.handlers[system_id.ordinal()]
    }
}

#[allow(clippy::unnecessary_wraps)]
fn noop_system(_context: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{SystemDispatchTable, SystemError, SystemExecutionContext, SystemFn};
    use crate::{ActionDefRegistry, DeterministicRng, SystemId};
    use std::collections::BTreeMap;
    use std::sync::{Mutex, OnceLock};
    use worldwake_core::{build_prototype_world, EventLog, Seed, Tick, World};

    fn calls() -> &'static Mutex<Vec<SystemId>> {
        static CALLS: OnceLock<Mutex<Vec<SystemId>>> = OnceLock::new();
        CALLS.get_or_init(|| Mutex::new(Vec::new()))
    }

    fn reset_calls() {
        calls().lock().unwrap().clear();
    }

    #[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
    fn record_call(context: SystemExecutionContext<'_>) -> Result<(), SystemError> {
        calls().lock().unwrap().push(context.system_id);
        let _ = context.world;
        let _ = context.event_log;
        let _ = context.rng;
        let _ = context.politics_trace;
        let _ = context.tick;
        Ok(())
    }

    fn handlers() -> [SystemFn; SystemId::ALL.len()] {
        [record_call; SystemId::ALL.len()]
    }

    #[test]
    fn get_returns_handler_for_each_closed_system_id() {
        let table = SystemDispatchTable::from_handlers(handlers());
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([7; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        reset_calls();

        for system_id in SystemId::ALL {
            table.get(system_id)(SystemExecutionContext {
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
                active_actions: &active_actions,
                action_defs: &action_defs,
                politics_trace: None,
                tick: Tick(3),
                system_id,
            })
            .unwrap();
        }

        assert_eq!(*calls().lock().unwrap(), SystemId::ALL);
    }

    #[test]
    fn canonical_noop_accepts_every_system_id() {
        let table = SystemDispatchTable::canonical_noop();
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([5; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        for system_id in SystemId::ALL {
            table.get(system_id)(SystemExecutionContext {
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
                active_actions: &active_actions,
                action_defs: &action_defs,
                politics_trace: None,
                tick: Tick(11),
                system_id,
            })
            .unwrap();
        }

        assert!(event_log.is_empty());
    }
}
