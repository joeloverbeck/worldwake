//! Authoritative patrol route state and per-agent patrol configuration.

use crate::{Component, EntityId, Permille};
use serde::{Deserialize, Serialize};

/// Ordered patrol route assignment plus persistent waypoint progress.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatrolRoute {
    pub assigned_places: Vec<EntityId>,
    pub current_index: usize,
}

impl Component for PatrolRoute {}

/// Stable per-agent parameters that shape patrol behavior.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatrolProfile {
    /// Minimum ticks the agent dwells at each patrol waypoint.
    pub base_dwell_ticks: u32,
    /// Additional dwell ticks scaled by vigilance level.
    pub dwell_vigilance_scale_ticks: u32,
    /// Baseline vigilance level during patrol; affects observation thoroughness and dwell time.
    pub vigilance: Permille,
    /// How quickly the agent adapts patrol routes in response to observed threats.
    pub route_adaptation_sensitivity: Permille,
    /// Base motive weight for patrol-driven goals.
    pub patrol_motive_weight: Permille,
}

impl Component for PatrolProfile {}

#[cfg(test)]
mod tests {
    use super::{PatrolProfile, PatrolRoute};
    use crate::{EntityId, Permille, traits::Component};
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn sample_patrol_route() -> PatrolRoute {
        PatrolRoute {
            assigned_places: vec![entity(3), entity(5), entity(8)],
            current_index: 1,
        }
    }

    fn sample_patrol_profile() -> PatrolProfile {
        PatrolProfile {
            base_dwell_ticks: 12,
            dwell_vigilance_scale_ticks: 12,
            vigilance: pm(700),
            route_adaptation_sensitivity: pm(450),
            patrol_motive_weight: pm(550),
        }
    }

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn patrol_components_satisfy_required_traits() {
        assert_component_bounds::<PatrolRoute>();
        assert_component_bounds::<PatrolProfile>();
        assert_value_bounds::<PatrolRoute>();
        assert_value_bounds::<PatrolProfile>();
    }

    #[test]
    fn patrol_route_roundtrips_through_bincode() {
        let route = sample_patrol_route();

        let bytes = bincode::serialize(&route).unwrap();
        let roundtrip: PatrolRoute = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, route);
    }

    #[test]
    fn patrol_profile_roundtrips_through_bincode() {
        let profile = sample_patrol_profile();

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: PatrolProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }
}
