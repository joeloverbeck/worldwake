use crate::Component;
use serde::{Deserialize, Serialize};

/// Stable per-agent agenda memory and retry parameters used by the AI layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct AgendaProfile {
    /// Maximum number of pending agenda entries retained for the agent.
    pub pending_capacity: u32,
    /// Maximum number of suspended agenda entries retained for the agent.
    pub suspended_capacity: u32,
    /// Cooldown before a pending agenda entry is reconsidered again.
    pub revive_cooldown_ticks: u32,
}

impl Default for AgendaProfile {
    fn default() -> Self {
        Self {
            pending_capacity: 16,
            suspended_capacity: 8,
            revive_cooldown_ticks: 4,
        }
    }
}

impl Component for AgendaProfile {}

#[cfg(test)]
mod tests {
    use super::AgendaProfile;
    use crate::{ControlSource, EntityKind, Tick, Topology, World, traits::Component};
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn agenda_profile_component_bounds() {
        assert_component_bounds::<AgendaProfile>();
        assert_value_bounds::<AgendaProfile>();
    }

    #[test]
    fn agenda_profile_default_matches_spec_defaults() {
        let profile = AgendaProfile::default();

        assert_eq!(profile.pending_capacity, 16);
        assert_eq!(profile.suspended_capacity, 8);
        assert_eq!(profile.revive_cooldown_ticks, 4);
    }

    #[test]
    fn agenda_profile_roundtrips_through_bincode() {
        let profile = AgendaProfile {
            pending_capacity: 20,
            suspended_capacity: 6,
            revive_cooldown_ticks: 2,
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: AgendaProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn agenda_profile_registers_for_agents() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world
            .create_agent("Planner", ControlSource::Ai, Tick(1))
            .unwrap();
        let profile = AgendaProfile {
            pending_capacity: 24,
            ..AgendaProfile::default()
        };

        assert_eq!(
            world.get_component_agenda_profile(agent),
            Some(&AgendaProfile::default())
        );
        assert_eq!(
            world.remove_component_agenda_profile(agent).unwrap(),
            Some(AgendaProfile::default())
        );

        world
            .insert_component_agenda_profile(agent, profile)
            .unwrap();

        assert_eq!(world.get_component_agenda_profile(agent), Some(&profile));
        assert_eq!(
            world.entities_with_agenda_profile().collect::<Vec<_>>(),
            vec![agent]
        );
        assert_eq!(
            world.query_agenda_profile().collect::<Vec<_>>(),
            vec![(agent, &profile)]
        );
        assert_eq!(world.count_with_agenda_profile(), 1);
        assert_eq!(world.entity_kind(agent), Some(EntityKind::Agent));
    }
}
