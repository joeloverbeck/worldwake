use crate::{ActionInstance, ActionInstanceId, DeterministicRng};
use std::collections::BTreeMap;
use worldwake_core::{CauseRef, EventLog, Tick, World};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ActionExecutionContext {
    pub cause: CauseRef,
    pub tick: Tick,
}

pub struct ActionExecutionAuthority<'a> {
    pub active_actions: &'a mut BTreeMap<ActionInstanceId, ActionInstance>,
    pub world: &'a mut World,
    pub event_log: &'a mut EventLog,
    pub rng: &'a mut DeterministicRng,
}
