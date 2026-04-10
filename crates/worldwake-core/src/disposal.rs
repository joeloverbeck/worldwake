use crate::{Component, Permille};
use serde::{Deserialize, Serialize};

/// Stable per-agent parameters governing when inventory disposal becomes worthwhile.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct DisposalProfile {
    pub capacity_strain_threshold: Permille,
}

impl Default for DisposalProfile {
    fn default() -> Self {
        Self {
            capacity_strain_threshold: Permille::new_unchecked(800),
        }
    }
}

impl Component for DisposalProfile {}

#[cfg(test)]
mod tests {
    use super::DisposalProfile;
    use crate::{ControlSource, EntityKind, Tick, Topology, World, traits::Component};
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn disposal_profile_component_bounds() {
        assert_component_bounds::<DisposalProfile>();
        assert_value_bounds::<DisposalProfile>();
    }

    #[test]
    fn disposal_profile_default_matches_spec_defaults() {
        let profile = DisposalProfile::default();

        assert_eq!(
            profile.capacity_strain_threshold,
            crate::Permille::new(800).unwrap()
        );
    }

    #[test]
    fn disposal_profile_roundtrips_through_bincode() {
        let profile = DisposalProfile {
            capacity_strain_threshold: crate::Permille::new(875).unwrap(),
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: DisposalProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn disposal_profile_registers_for_agents() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world
            .create_agent("Cleaner", ControlSource::Ai, Tick(1))
            .unwrap();
        let profile = DisposalProfile {
            capacity_strain_threshold: crate::Permille::new(900).unwrap(),
        };

        assert_eq!(
            world.get_component_disposal_profile(agent),
            Some(&DisposalProfile::default())
        );
        assert_eq!(
            world.remove_component_disposal_profile(agent).unwrap(),
            Some(DisposalProfile::default())
        );

        world
            .insert_component_disposal_profile(agent, profile)
            .unwrap();

        assert_eq!(world.get_component_disposal_profile(agent), Some(&profile));
        assert_eq!(
            world.entities_with_disposal_profile().collect::<Vec<_>>(),
            vec![agent]
        );
        assert_eq!(
            world.query_disposal_profile().collect::<Vec<_>>(),
            vec![(agent, &profile)]
        );
        assert_eq!(world.count_with_disposal_profile(), 1);
        assert_eq!(world.entity_kind(agent), Some(EntityKind::Agent));
    }
}
