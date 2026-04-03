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

impl RouteExperience {
    pub fn enforce_limits(&mut self, current_tick: Tick, profile: &PreferenceProfile) {
        self.edges.retain(|_, experience| {
            current_tick
                .0
                .saturating_sub(experience.last_travel_tick.0)
                <= profile.memory_retention_ticks
        });

        let capacity = profile.route_memory_capacity as usize;
        if self.edges.len() <= capacity {
            return;
        }

        let mut oldest_edges: Vec<_> = self
            .edges
            .iter()
            .map(|(edge_id, experience)| (*edge_id, experience.last_travel_tick))
            .collect();
        oldest_edges.sort_by_key(|(edge_id, last_tick)| (*last_tick, *edge_id));

        for (edge_id, _) in oldest_edges.into_iter().take(self.edges.len() - capacity) {
            self.edges.remove(&edge_id);
        }
    }

    pub fn prune_dead_edges(&mut self, is_valid_edge: impl Fn(&TravelEdgeId) -> bool) {
        self.edges.retain(|edge_id, _| is_valid_edge(edge_id));
    }
}

#[must_use]
pub fn danger_ratio_permille(experience: &EdgeExperience) -> u32 {
    let total = u32::from(experience.safe_trips) + u32::from(experience.hostile_encounters);
    if total == 0 {
        0
    } else {
        u32::from(experience.hostile_encounters) * 1000 / total
    }
}

#[must_use]
pub fn failure_ratio_permille(record: &ReliabilityRecord) -> u32 {
    let total = u32::from(record.successful_acquisitions) + u32::from(record.failed_attempts);
    if total == 0 {
        0
    } else {
        u32::from(record.failed_attempts) * 1000 / total
    }
}

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

impl SourceReliability {
    pub fn enforce_limits(&mut self, current_tick: Tick, profile: &PreferenceProfile) {
        self.sources.retain(|_, record| {
            current_tick.0.saturating_sub(record.last_attempt_tick.0) <= profile.memory_retention_ticks
        });

        let capacity = profile.source_memory_capacity as usize;
        if self.sources.len() <= capacity {
            return;
        }

        let mut oldest_sources: Vec<_> = self
            .sources
            .iter()
            .map(|(source_key, record)| (*source_key, record.last_attempt_tick))
            .collect();
        oldest_sources.sort_by_key(|(source_key, last_tick)| (*last_tick, *source_key));

        for (source_key, _) in oldest_sources.into_iter().take(self.sources.len() - capacity) {
            self.sources.remove(&source_key);
        }
    }

    pub fn prune_dead_sources(&mut self, is_alive: impl Fn(&EntityId) -> bool) {
        self.sources.retain(|source_key, _| is_alive(&source_key.entity));
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct PreferenceProfile {
    pub route_caution_weight: Permille,
    pub source_trust_weight: Permille,
    pub route_memory_capacity: u32,
    pub source_memory_capacity: u32,
    pub memory_retention_ticks: u64,
}

impl Default for PreferenceProfile {
    fn default() -> Self {
        Self {
            route_caution_weight: Permille::new_unchecked(300),
            source_trust_weight: Permille::new_unchecked(200),
            route_memory_capacity: 24,
            source_memory_capacity: 18,
            memory_retention_ticks: 400,
        }
    }
}

impl Component for PreferenceProfile {}

#[cfg(test)]
mod tests {
    use super::{
        EdgeExperience, PreferenceProfile, ReliabilityRecord, RouteExperience, SourceKey,
        SourceReliability, danger_ratio_permille, failure_ratio_permille,
    };
    use crate::{
        test_utils::{
            sample_preference_profile, sample_route_experience, sample_source_reliability,
        },
        traits::Component,
        ControlSource, EntityId, EntityKind, Tick, Topology, TravelEdgeId, World, WorldError,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use std::collections::BTreeMap;
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
    fn preference_profile_default_matches_fixture_baseline() {
        let profile = PreferenceProfile::default();

        assert_eq!(profile.route_caution_weight, crate::Permille::new(300).unwrap());
        assert_eq!(profile.source_trust_weight, crate::Permille::new(200).unwrap());
        assert_eq!(profile.route_memory_capacity, 24);
        assert_eq!(profile.source_memory_capacity, 18);
        assert_eq!(profile.memory_retention_ticks, 400);
    }

    #[test]
    fn failure_ratio_permille_returns_zero_without_attempts() {
        let record = ReliabilityRecord {
            successful_acquisitions: 0,
            failed_attempts: 0,
            last_attempt_tick: Tick(1),
        };

        assert_eq!(failure_ratio_permille(&record), 0);
    }

    #[test]
    fn failure_ratio_permille_handles_boundary_values() {
        let zero_failures = ReliabilityRecord {
            successful_acquisitions: 4,
            failed_attempts: 0,
            last_attempt_tick: Tick(1),
        };
        let even_split = ReliabilityRecord {
            successful_acquisitions: 3,
            failed_attempts: 3,
            last_attempt_tick: Tick(1),
        };
        let all_failures = ReliabilityRecord {
            successful_acquisitions: 0,
            failed_attempts: u16::MAX,
            last_attempt_tick: Tick(1),
        };

        assert_eq!(failure_ratio_permille(&zero_failures), 0);
        assert_eq!(failure_ratio_permille(&even_split), 500);
        assert_eq!(failure_ratio_permille(&all_failures), 1000);
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

    #[test]
    fn danger_ratio_permille_returns_zero_when_total_trips_is_zero() {
        assert_eq!(
            danger_ratio_permille(&EdgeExperience {
                safe_trips: 0,
                hostile_encounters: 0,
                last_travel_tick: Tick(0),
            }),
            0
        );
    }

    #[test]
    fn danger_ratio_permille_returns_expected_boundary_values() {
        assert_eq!(
            danger_ratio_permille(&EdgeExperience {
                safe_trips: 5,
                hostile_encounters: 0,
                last_travel_tick: Tick(0),
            }),
            0
        );
        assert_eq!(
            danger_ratio_permille(&EdgeExperience {
                safe_trips: 1,
                hostile_encounters: 1,
                last_travel_tick: Tick(0),
            }),
            500
        );
        assert_eq!(
            danger_ratio_permille(&EdgeExperience {
                safe_trips: 0,
                hostile_encounters: 3,
                last_travel_tick: Tick(0),
            }),
            1000
        );
    }

    #[test]
    fn route_experience_enforce_limits_prunes_stale_records() {
        let mut route = RouteExperience {
            edges: BTreeMap::from([
                (
                    TravelEdgeId(1),
                    EdgeExperience {
                        safe_trips: 1,
                        hostile_encounters: 0,
                        last_travel_tick: Tick(5),
                    },
                ),
                (
                    TravelEdgeId(2),
                    EdgeExperience {
                        safe_trips: 0,
                        hostile_encounters: 1,
                        last_travel_tick: Tick(17),
                    },
                ),
            ]),
        };
        let profile = PreferenceProfile {
            memory_retention_ticks: 10,
            ..sample_preference_profile()
        };

        route.enforce_limits(Tick(20), &profile);

        assert_eq!(route.edges.len(), 1);
        assert!(route.edges.contains_key(&TravelEdgeId(2)));
    }

    #[test]
    fn route_experience_enforce_limits_evicts_oldest_records_when_over_capacity() {
        let mut route = RouteExperience {
            edges: BTreeMap::from([
                (
                    TravelEdgeId(1),
                    EdgeExperience {
                        safe_trips: 1,
                        hostile_encounters: 0,
                        last_travel_tick: Tick(10),
                    },
                ),
                (
                    TravelEdgeId(2),
                    EdgeExperience {
                        safe_trips: 2,
                        hostile_encounters: 0,
                        last_travel_tick: Tick(11),
                    },
                ),
                (
                    TravelEdgeId(3),
                    EdgeExperience {
                        safe_trips: 3,
                        hostile_encounters: 0,
                        last_travel_tick: Tick(12),
                    },
                ),
            ]),
        };
        let profile = PreferenceProfile {
            route_memory_capacity: 2,
            memory_retention_ticks: u64::MAX,
            ..sample_preference_profile()
        };

        route.enforce_limits(Tick(20), &profile);

        assert_eq!(route.edges.len(), 2);
        assert!(!route.edges.contains_key(&TravelEdgeId(1)));
        assert!(route.edges.contains_key(&TravelEdgeId(2)));
        assert!(route.edges.contains_key(&TravelEdgeId(3)));
    }

    #[test]
    fn route_experience_enforce_limits_breaks_oldest_tick_ties_deterministically_by_edge_id() {
        let mut route = RouteExperience {
            edges: BTreeMap::from([
                (
                    TravelEdgeId(1),
                    EdgeExperience {
                        safe_trips: 1,
                        hostile_encounters: 0,
                        last_travel_tick: Tick(10),
                    },
                ),
                (
                    TravelEdgeId(2),
                    EdgeExperience {
                        safe_trips: 1,
                        hostile_encounters: 0,
                        last_travel_tick: Tick(10),
                    },
                ),
            ]),
        };
        let profile = PreferenceProfile {
            route_memory_capacity: 1,
            memory_retention_ticks: u64::MAX,
            ..sample_preference_profile()
        };

        route.enforce_limits(Tick(20), &profile);

        assert_eq!(route.edges.len(), 1);
        assert!(!route.edges.contains_key(&TravelEdgeId(1)));
        assert!(route.edges.contains_key(&TravelEdgeId(2)));
    }

    #[test]
    fn source_reliability_enforce_limits_prunes_stale_records() {
        let mut reliability = SourceReliability {
            sources: BTreeMap::from([
                (
                    SourceKey {
                        entity: entity(1),
                        commodity: crate::CommodityKind::Apple,
                    },
                    ReliabilityRecord {
                        successful_acquisitions: 1,
                        failed_attempts: 0,
                        last_attempt_tick: Tick(4),
                    },
                ),
                (
                    SourceKey {
                        entity: entity(2),
                        commodity: crate::CommodityKind::Bread,
                    },
                    ReliabilityRecord {
                        successful_acquisitions: 0,
                        failed_attempts: 1,
                        last_attempt_tick: Tick(18),
                    },
                ),
            ]),
        };
        let profile = PreferenceProfile {
            memory_retention_ticks: 10,
            ..sample_preference_profile()
        };

        reliability.enforce_limits(Tick(20), &profile);

        assert_eq!(reliability.sources.len(), 1);
        assert!(reliability.sources.contains_key(&SourceKey {
            entity: entity(2),
            commodity: crate::CommodityKind::Bread,
        }));
    }

    #[test]
    fn source_reliability_enforce_limits_evicts_oldest_records_when_over_capacity() {
        let mut reliability = SourceReliability {
            sources: BTreeMap::from([
                (
                    SourceKey {
                        entity: entity(1),
                        commodity: crate::CommodityKind::Apple,
                    },
                    ReliabilityRecord {
                        successful_acquisitions: 1,
                        failed_attempts: 0,
                        last_attempt_tick: Tick(10),
                    },
                ),
                (
                    SourceKey {
                        entity: entity(2),
                        commodity: crate::CommodityKind::Bread,
                    },
                    ReliabilityRecord {
                        successful_acquisitions: 1,
                        failed_attempts: 0,
                        last_attempt_tick: Tick(11),
                    },
                ),
                (
                    SourceKey {
                        entity: entity(3),
                        commodity: crate::CommodityKind::Water,
                    },
                    ReliabilityRecord {
                        successful_acquisitions: 1,
                        failed_attempts: 0,
                        last_attempt_tick: Tick(12),
                    },
                ),
            ]),
        };
        let profile = PreferenceProfile {
            source_memory_capacity: 2,
            memory_retention_ticks: u64::MAX,
            ..sample_preference_profile()
        };

        reliability.enforce_limits(Tick(20), &profile);

        assert_eq!(reliability.sources.len(), 2);
        assert!(!reliability.sources.contains_key(&SourceKey {
            entity: entity(1),
            commodity: crate::CommodityKind::Apple,
        }));
    }

    #[test]
    fn source_reliability_enforce_limits_breaks_oldest_tick_ties_deterministically_by_source_key() {
        let mut reliability = SourceReliability {
            sources: BTreeMap::from([
                (
                    SourceKey {
                        entity: entity(1),
                        commodity: crate::CommodityKind::Apple,
                    },
                    ReliabilityRecord {
                        successful_acquisitions: 1,
                        failed_attempts: 0,
                        last_attempt_tick: Tick(10),
                    },
                ),
                (
                    SourceKey {
                        entity: entity(1),
                        commodity: crate::CommodityKind::Bread,
                    },
                    ReliabilityRecord {
                        successful_acquisitions: 1,
                        failed_attempts: 0,
                        last_attempt_tick: Tick(10),
                    },
                ),
            ]),
        };
        let profile = PreferenceProfile {
            source_memory_capacity: 1,
            memory_retention_ticks: u64::MAX,
            ..sample_preference_profile()
        };

        reliability.enforce_limits(Tick(20), &profile);

        assert_eq!(reliability.sources.len(), 1);
        assert!(!reliability.sources.contains_key(&SourceKey {
            entity: entity(1),
            commodity: crate::CommodityKind::Apple,
        }));
        assert!(reliability.sources.contains_key(&SourceKey {
            entity: entity(1),
            commodity: crate::CommodityKind::Bread,
        }));
    }

    #[test]
    fn route_experience_prune_dead_edges_removes_invalid_entries() {
        let mut route = RouteExperience {
            edges: BTreeMap::from([
                (
                    TravelEdgeId(1),
                    EdgeExperience {
                        safe_trips: 1,
                        hostile_encounters: 0,
                        last_travel_tick: Tick(8),
                    },
                ),
                (
                    TravelEdgeId(2),
                    EdgeExperience {
                        safe_trips: 1,
                        hostile_encounters: 0,
                        last_travel_tick: Tick(9),
                    },
                ),
            ]),
        };

        route.prune_dead_edges(|edge_id| *edge_id != TravelEdgeId(1));

        assert_eq!(route.edges.len(), 1);
        assert!(route.edges.contains_key(&TravelEdgeId(2)));
    }

    #[test]
    fn source_reliability_prune_dead_sources_removes_archived_entities() {
        let mut reliability = SourceReliability {
            sources: BTreeMap::from([
                (
                    SourceKey {
                        entity: entity(1),
                        commodity: crate::CommodityKind::Apple,
                    },
                    ReliabilityRecord {
                        successful_acquisitions: 1,
                        failed_attempts: 0,
                        last_attempt_tick: Tick(8),
                    },
                ),
                (
                    SourceKey {
                        entity: entity(2),
                        commodity: crate::CommodityKind::Bread,
                    },
                    ReliabilityRecord {
                        successful_acquisitions: 1,
                        failed_attempts: 0,
                        last_attempt_tick: Tick(9),
                    },
                ),
            ]),
        };

        reliability.prune_dead_sources(|entity_id| *entity_id != entity(1));

        assert_eq!(reliability.sources.len(), 1);
        assert!(reliability.sources.contains_key(&SourceKey {
            entity: entity(2),
            commodity: crate::CommodityKind::Bread,
        }));
    }
}
