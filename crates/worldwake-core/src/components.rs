//! Phase 1 authoritative component types not owned by topology.

use crate::{Component, ControlSource};
use serde::{Deserialize, Serialize};

/// Human-readable name for any named entity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Name(pub String);

impl Component for Name {}

/// Agent-specific data attached to agent entities.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentData {
    pub control_source: ControlSource,
}

impl Component for AgentData {}

#[cfg(test)]
mod tests {
    use super::{AgentData, Name};
    use crate::{traits::Component, ControlSource};

    fn assert_component_bounds<T: Component>() {}

    #[test]
    fn name_component_bounds() {
        assert_component_bounds::<Name>();
    }

    #[test]
    fn agent_data_component_bounds() {
        assert_component_bounds::<AgentData>();
    }

    #[test]
    fn name_bincode_roundtrip() {
        let name = Name("Mira".to_string());

        let bytes = bincode::serialize(&name).unwrap();
        let roundtrip: Name = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, name);
    }

    #[test]
    fn agent_data_bincode_roundtrip() {
        let agent = AgentData {
            control_source: ControlSource::Ai,
        };

        let bytes = bincode::serialize(&agent).unwrap();
        let roundtrip: AgentData = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, agent);
    }
}
