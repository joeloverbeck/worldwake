//! Per-agent learned route and source experience state.

use crate::{CommodityKind, Component, EntityId, Permille, Tick, TravelEdgeId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct EdgeExperience {
    pub safe_trips: u16,
    pub hostile_encounters: u16,
    pub last_travel_tick: Tick,
}

#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct RouteExperience {
    pub edges: BTreeMap<TravelEdgeId, EdgeExperience>,
}

impl Component for RouteExperience {}

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SourceKey {
    pub entity: EntityId,
    pub commodity: CommodityKind,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReliabilityRecord {
    pub successful_acquisitions: u16,
    pub failed_attempts: u16,
    pub last_attempt_tick: Tick,
}

#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct SourceReliability {
    pub sources: BTreeMap<SourceKey, ReliabilityRecord>,
}

impl Component for SourceReliability {}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct PreferenceProfile {
    pub route_caution_weight: Permille,
    pub source_trust_weight: Permille,
    pub route_memory_capacity: u32,
    pub source_memory_capacity: u32,
    pub memory_retention_ticks: u64,
}

impl Component for PreferenceProfile {}

#[cfg(test)]
mod tests {
    use super::{
        EdgeExperience, PreferenceProfile, ReliabilityRecord, RouteExperience, SourceKey,
        SourceReliability,
    };
    use crate::{
        test_utils::{
            sample_preference_profile, sample_route_experience, sample_source_reliability,
        },
        traits::Component,
        ControlSource, EntityId, EntityKind, Tick, Topology, TravelEdgeId, World, WorldError,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    #[test]
    fn experience_components_satisfy_component_bounds() {
        assert_component_bounds::<RouteExperience>();
        assert_component_bounds::<SourceReliability>();
        assert_component_bounds::<PreferenceProfile>();
        assert_value_bounds::<EdgeExperience>();
        assert_value_bounds::<SourceKey>();
        assert_value_bounds::<ReliabilityRecord>();
    }

    #[test]
    fn source_key_orders_by_entity_then_commodity() {
        let a = SourceKey {
            entity: entity(1),
            commodity: crate::CommodityKind::Apple,
        };
        let b = SourceKey {
            entity: entity(1),
            commodity: crate::CommodityKind::Bread,
        };
        let c = SourceKey {
            entity: entity(2),
            commodity: crate::CommodityKind::Apple,
        };

        assert!(a < b);
        assert!(b < c);
    }

    #[test]
    fn route_experience_defaults_empty() {
        assert!(RouteExperience::default().edges.is_empty());
    }

    #[test]
    fn source_reliability_defaults_empty() {
        assert!(SourceReliability::default().sources.is_empty());
    }

    #[test]
    fn experience_values_roundtrip_through_bincode() {
        let route = sample_route_experience();
        let sources = sample_source_reliability();
        let profile = sample_preference_profile();

        let route_roundtrip: RouteExperience =
            bincode::deserialize(&bincode::serialize(&route).unwrap()).unwrap();
        let source_roundtrip: SourceReliability =
            bincode::deserialize(&bincode::serialize(&sources).unwrap()).unwrap();
        let profile_roundtrip: PreferenceProfile =
            bincode::deserialize(&bincode::serialize(&profile).unwrap()).unwrap();

        assert_eq!(route_roundtrip, route);
        assert_eq!(source_roundtrip, sources);
        assert_eq!(profile_roundtrip, profile);
    }

    #[test]
    fn experience_components_roundtrip_through_world_storage() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world.create_agent("Aster", ControlSource::Ai, Tick(1)).unwrap();
        let route = sample_route_experience();
        let sources = sample_source_reliability();
        let profile = sample_preference_profile();

        world
            .insert_component_route_experience(agent, route.clone())
            .unwrap();
        world
            .insert_component_source_reliability(agent, sources.clone())
            .unwrap();
        world
            .insert_component_preference_profile(agent, profile)
            .unwrap();

        assert_eq!(world.get_component_route_experience(agent), Some(&route));
        assert_eq!(world.get_component_source_reliability(agent), Some(&sources));
        assert_eq!(world.get_component_preference_profile(agent), Some(&profile));

        assert_eq!(
            world.remove_component_route_experience(agent).unwrap(),
            Some(route)
        );
        assert_eq!(
            world.remove_component_source_reliability(agent).unwrap(),
            Some(sources)
        );
        assert_eq!(
            world.remove_component_preference_profile(agent).unwrap(),
            Some(profile)
        );
    }

    #[test]
    fn experience_components_reject_non_agent_entities() {
        let mut world = World::new(Topology::new()).unwrap();
        let office = world.create_entity(EntityKind::Office, Tick(1));

        let route_err = world
            .insert_component_route_experience(office, sample_route_experience())
            .unwrap_err();
        let source_err = world
            .insert_component_source_reliability(office, sample_source_reliability())
            .unwrap_err();
        let profile_err = world
            .insert_component_preference_profile(office, sample_preference_profile())
            .unwrap_err();

        assert!(matches!(route_err, WorldError::InvalidOperation(_)));
        assert!(matches!(source_err, WorldError::InvalidOperation(_)));
        assert!(matches!(profile_err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn sample_route_experience_contains_expected_edge_record() {
        let route = sample_route_experience();
        assert_eq!(
            route.edges.get(&TravelEdgeId(3)),
            Some(&EdgeExperience {
                safe_trips: 4,
                hostile_encounters: 1,
                last_travel_tick: Tick(19),
            })
        );
    }

    #[test]
    fn sample_source_reliability_contains_expected_source_record() {
        let reliability = sample_source_reliability();
        assert_eq!(
            reliability.sources.get(&SourceKey {
                entity: entity(9),
                commodity: crate::CommodityKind::Bread,
            }),
            Some(&ReliabilityRecord {
                successful_acquisitions: 3,
                failed_attempts: 1,
                last_attempt_tick: Tick(21),
            })
        );
    }
}
