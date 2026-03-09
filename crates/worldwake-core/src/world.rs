//! Authoritative world boundary over entity lifecycle, component tables, and topology.

use crate::{
    component_schema::with_authoritative_components, AgentData, ComponentTables, EntityAllocator,
    EntityId, EntityKind, EntityMeta, Name, Tick, Topology, WorldError,
};

macro_rules! world_component_api {
    ($({ $field:ident, $component_ty:ty, $table_insert:ident, $table_get:ident, $table_get_mut:ident, $table_remove:ident, $table_has:ident, $table_iter:ident, $insert_fn:ident, $get_fn:ident, $get_mut_fn:ident, $remove_fn:ident, $has_fn:ident, $entities_fn:ident, $query_fn:ident, $count_fn:ident, $component_name:literal, $kind_check:expr })*) => {
        $(
            pub fn $insert_fn(
                &mut self,
                entity: EntityId,
                component: $component_ty,
            ) -> Result<(), WorldError> {
                let meta = self.ensure_alive(entity)?;
                if !(($kind_check)(meta.kind)) {
                    return Err(WorldError::InvalidOperation(format!(
                        "component {} not valid for entity kind {:?}: {}",
                        $component_name, meta.kind, entity
                    )));
                }
                if self.components.$table_has(entity) {
                    return Err(WorldError::DuplicateComponent {
                        entity,
                        component_type: $component_name,
                    });
                }
                let replaced = self.components.$table_insert(entity, component);
                debug_assert!(replaced.is_none(), "duplicate check must prevent replacement");
                Ok(())
            }

            #[must_use]
            pub fn $get_fn(&self, entity: EntityId) -> Option<&$component_ty> {
                self.is_alive(entity).then(|| self.components.$table_get(entity))?
            }

            pub fn $get_mut_fn(&mut self, entity: EntityId) -> Option<&mut $component_ty> {
                self.is_alive(entity)
                    .then(|| self.components.$table_get_mut(entity))?
            }

            pub fn $remove_fn(&mut self, entity: EntityId) -> Result<Option<$component_ty>, WorldError> {
                self.ensure_alive(entity)?;
                Ok(self.components.$table_remove(entity))
            }

            #[must_use]
            pub fn $has_fn(&self, entity: EntityId) -> bool {
                self.is_alive(entity) && self.components.$table_has(entity)
            }

            pub fn $entities_fn(&self) -> impl Iterator<Item = EntityId> + '_ {
                self.$query_fn().map(|(entity, _)| entity)
            }

            pub fn $query_fn(&self) -> impl Iterator<Item = (EntityId, &$component_ty)> + '_ {
                self.components
                    .$table_iter()
                    .filter(move |(entity, _)| self.is_alive(*entity))
            }

            #[must_use]
            pub fn $count_fn(&self) -> usize {
                self.$query_fn().count()
            }
        )*
    };
}

/// The authoritative simulation world.
///
/// All fields are private. External code accesses state through typed read
/// methods and controlled mutation methods.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct World {
    allocator: EntityAllocator,
    components: ComponentTables,
    topology: Topology,
}

impl World {
    pub fn new(topology: Topology) -> Result<Self, WorldError> {
        let mut allocator = EntityAllocator::new();
        for place_id in topology.place_ids() {
            allocator.register_existing(place_id, EntityKind::Place, Tick(0))?;
        }

        Ok(Self {
            allocator,
            components: ComponentTables::default(),
            topology,
        })
    }

    pub fn create_entity(&mut self, kind: EntityKind, tick: Tick) -> EntityId {
        self.allocator.create_entity(kind, tick)
    }

    pub fn create_agent(
        &mut self,
        name: &str,
        control_source: crate::ControlSource,
        tick: Tick,
    ) -> Result<EntityId, WorldError> {
        self.create_entity_with(EntityKind::Agent, tick, |world, entity| {
            world.insert_component_name(entity, Name(name.to_string()))?;
            world.insert_component_agent_data(entity, AgentData { control_source })?;
            Ok(())
        })
    }

    pub fn create_office(&mut self, name: &str, tick: Tick) -> Result<EntityId, WorldError> {
        self.create_named_entity(EntityKind::Office, name, tick)
    }

    pub fn create_faction(&mut self, name: &str, tick: Tick) -> Result<EntityId, WorldError> {
        self.create_named_entity(EntityKind::Faction, name, tick)
    }

    pub fn archive_entity(&mut self, id: EntityId, tick: Tick) -> Result<(), WorldError> {
        if self.topology.place(id).is_some() {
            return Err(WorldError::InvalidOperation(format!(
                "cannot archive topology-owned place: {id}"
            )));
        }

        self.allocator.archive_entity(id, tick)
    }

    pub fn purge_entity(&mut self, id: EntityId) -> Result<(), WorldError> {
        if self.topology.place(id).is_some() {
            return Err(WorldError::InvalidOperation(format!(
                "cannot purge topology-owned place: {id}"
            )));
        }

        self.allocator.purge_entity(id)?;
        self.components.remove_all(id);
        Ok(())
    }

    #[must_use]
    pub fn is_alive(&self, id: EntityId) -> bool {
        self.allocator.is_alive(id)
    }

    #[must_use]
    pub fn is_archived(&self, id: EntityId) -> bool {
        self.allocator.is_archived(id)
    }

    #[must_use]
    pub fn entity_meta(&self, id: EntityId) -> Option<&EntityMeta> {
        self.allocator.get_meta(id)
    }

    #[must_use]
    pub fn entity_kind(&self, id: EntityId) -> Option<EntityKind> {
        self.entity_meta(id).map(|meta| meta.kind)
    }

    #[must_use]
    pub fn topology(&self) -> &Topology {
        &self.topology
    }

    pub fn entities(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.allocator.entity_ids()
    }

    pub fn all_entities(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.allocator.all_entity_ids()
    }

    pub fn entities_of_kind(&self, kind: EntityKind) -> impl Iterator<Item = EntityId> + '_ {
        self.entities()
            .filter(move |entity| self.entity_kind(*entity) == Some(kind))
    }

    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.entities().count()
    }

    pub fn entities_with_name_and_agent_data(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.query_name_and_agent_data().map(|(entity, _, _)| entity)
    }

    pub fn query_name_and_agent_data(
        &self,
    ) -> impl Iterator<Item = (EntityId, &Name, &AgentData)> + '_ {
        self.query_name().filter_map(move |(entity, name)| {
            self.get_component_agent_data(entity)
                .map(|agent_data| (entity, name, agent_data))
        })
    }

    fn ensure_alive(&self, id: EntityId) -> Result<&EntityMeta, WorldError> {
        let meta = self
            .allocator
            .get_meta(id)
            .ok_or(WorldError::EntityNotFound(id))?;
        if meta.archived_at.is_some() {
            return Err(WorldError::ArchivedEntity(id));
        }
        Ok(meta)
    }

    fn create_named_entity(
        &mut self,
        kind: EntityKind,
        name: &str,
        tick: Tick,
    ) -> Result<EntityId, WorldError> {
        self.create_entity_with(kind, tick, |world, entity| {
            world.insert_component_name(entity, Name(name.to_string()))
        })
    }

    fn create_entity_with<F>(
        &mut self,
        kind: EntityKind,
        tick: Tick,
        init: F,
    ) -> Result<EntityId, WorldError>
    where
        F: FnOnce(&mut Self, EntityId) -> Result<(), WorldError>,
    {
        let entity = self.create_entity(kind, tick);
        if let Err(err) = init(self, entity) {
            self.rollback_created_entity(entity, tick);
            return Err(err);
        }

        Ok(entity)
    }

    fn rollback_created_entity(&mut self, entity: EntityId, tick: Tick) {
        debug_assert!(
            self.topology.place(entity).is_none(),
            "factory rollback only supports non-topological entities"
        );

        self.archive_entity(entity, tick)
            .expect("newly created entity should archive during rollback");
        self.purge_entity(entity)
            .expect("newly created entity should purge during rollback");
    }

    with_authoritative_components!(world_component_api);
}

#[cfg(test)]
mod tests {
    use super::World;
    use crate::{AgentData, ControlSource, EntityId, EntityKind, Name, Place, PlaceTag, Tick, Topology, WorldError};
    use std::collections::BTreeSet;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn test_topology() -> Topology {
        let mut topology = Topology::new();
        topology
            .add_place(
                entity(5),
                Place {
                    name: "Square".to_string(),
                    capacity: None,
                    tags: BTreeSet::from([PlaceTag::Village]),
                },
            )
            .unwrap();
        topology
            .add_place(
                entity(2),
                Place {
                    name: "Farm".to_string(),
                    capacity: None,
                    tags: BTreeSet::from([PlaceTag::Farm]),
                },
            )
            .unwrap();
        topology
    }

    #[test]
    fn world_new_registers_topology_places_as_live_entities() {
        let world = World::new(test_topology()).unwrap();

        for place_id in [entity(2), entity(5)] {
            assert!(world.is_alive(place_id));
            assert_eq!(world.entity_kind(place_id), Some(EntityKind::Place));
            assert_eq!(world.entity_meta(place_id).unwrap().created_at, Tick(0));
        }
    }

    #[test]
    fn create_entity_returns_alive_id() {
        let mut world = World::new(Topology::new()).unwrap();

        let id = world.create_entity(EntityKind::Agent, Tick(4));

        assert!(world.is_alive(id));
        assert_eq!(world.entity_kind(id), Some(EntityKind::Agent));
        assert_eq!(world.entity_meta(id).unwrap().created_at, Tick(4));
    }

    #[test]
    fn create_agent_produces_correct_entity() {
        let mut world = World::new(Topology::new()).unwrap();

        let id = world
            .create_agent("Aster", ControlSource::Human, Tick(7))
            .unwrap();

        assert!(world.is_alive(id));
        assert_eq!(world.entity_kind(id), Some(EntityKind::Agent));
        assert_eq!(world.get_component_name(id), Some(&Name("Aster".to_string())));
        assert_eq!(
            world.get_component_agent_data(id),
            Some(&AgentData {
                control_source: ControlSource::Human,
            })
        );
    }

    #[test]
    fn create_agent_components_queryable() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_agent("Bram", ControlSource::Ai, Tick(3)).unwrap();

        assert_eq!(world.entities_with_name().collect::<Vec<_>>(), vec![id]);
        assert_eq!(world.entities_with_agent_data().collect::<Vec<_>>(), vec![id]);
        assert_eq!(
            world.entities_with_name_and_agent_data().collect::<Vec<_>>(),
            vec![id]
        );
    }

    #[test]
    fn create_office_produces_correct_entity() {
        let mut world = World::new(Topology::new()).unwrap();

        let id = world.create_office("Ledger Hall", Tick(5)).unwrap();

        assert!(world.is_alive(id));
        assert_eq!(world.entity_kind(id), Some(EntityKind::Office));
        assert_eq!(
            world.get_component_name(id),
            Some(&Name("Ledger Hall".to_string()))
        );
        assert_eq!(world.get_component_agent_data(id), None);
    }

    #[test]
    fn create_faction_produces_correct_entity() {
        let mut world = World::new(Topology::new()).unwrap();

        let id = world.create_faction("River Pact", Tick(6)).unwrap();

        assert!(world.is_alive(id));
        assert_eq!(world.entity_kind(id), Some(EntityKind::Faction));
        assert_eq!(
            world.get_component_name(id),
            Some(&Name("River Pact".to_string()))
        );
        assert_eq!(world.get_component_agent_data(id), None);
    }

    #[test]
    fn factory_equivalent_to_manual_creation() {
        let mut factory_world = World::new(Topology::new()).unwrap();
        let factory_id = factory_world
            .create_agent("Aster", ControlSource::Ai, Tick(9))
            .unwrap();

        let mut manual_world = World::new(Topology::new()).unwrap();
        let manual_id = manual_world.create_entity(EntityKind::Agent, Tick(9));
        manual_world
            .insert_component_name(manual_id, Name("Aster".to_string()))
            .unwrap();
        manual_world
            .insert_component_agent_data(
                manual_id,
                AgentData {
                    control_source: ControlSource::Ai,
                },
            )
            .unwrap();

        assert_eq!(factory_id, manual_id);
        assert_eq!(factory_world.entity_meta(factory_id), manual_world.entity_meta(manual_id));
        assert_eq!(
            factory_world.get_component_name(factory_id),
            manual_world.get_component_name(manual_id)
        );
        assert_eq!(
            factory_world.get_component_agent_data(factory_id),
            manual_world.get_component_agent_data(manual_id)
        );
    }

    #[test]
    fn multiple_agents_unique_ids() {
        let mut world = World::new(Topology::new()).unwrap();

        let first = world.create_agent("Aster", ControlSource::Ai, Tick(1)).unwrap();
        let second = world
            .create_agent("Bram", ControlSource::Human, Tick(2))
            .unwrap();

        assert_ne!(first, second);
        assert_eq!(world.entities_of_kind(EntityKind::Agent).collect::<Vec<_>>(), vec![first, second]);
    }

    #[test]
    fn factory_failure_rolls_back_allocated_entity() {
        let mut world = World::new(Topology::new()).unwrap();

        let err = world
            .create_entity_with(EntityKind::Office, Tick(12), |_, _| {
                Err(WorldError::InvalidOperation("boom".to_string()))
            })
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(message) if message == "boom"));
        assert_eq!(world.entity_count(), 0);
        assert_eq!(world.all_entities().count(), 0);
        assert_eq!(world.count_with_name(), 0);
        assert_eq!(world.count_with_agent_data(), 0);
    }

    #[test]
    fn archive_entity_marks_non_live() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Office, Tick(3));

        world.archive_entity(id, Tick(9)).unwrap();

        assert!(!world.is_alive(id));
        assert!(world.is_archived(id));
        assert_eq!(world.entity_meta(id).unwrap().archived_at, Some(Tick(9)));
    }

    #[test]
    fn entities_returns_sorted_live_ids() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world.create_entity(EntityKind::Agent, Tick(1));
        let office = world.create_entity(EntityKind::Office, Tick(2));
        let faction = world.create_entity(EntityKind::Faction, Tick(3));

        world.archive_entity(office, Tick(4)).unwrap();

        let ids = world.entities().collect::<Vec<_>>();

        assert_eq!(ids, vec![entity(2), entity(5), agent, faction]);
    }

    #[test]
    fn all_entities_includes_archived_but_not_purged() {
        let mut world = World::new(Topology::new()).unwrap();
        let live = world.create_entity(EntityKind::Agent, Tick(1));
        let archived = world.create_entity(EntityKind::Office, Tick(2));

        world.archive_entity(archived, Tick(3)).unwrap();
        assert_eq!(world.all_entities().collect::<Vec<_>>(), vec![live, archived]);

        world.purge_entity(archived).unwrap();

        assert_eq!(world.all_entities().collect::<Vec<_>>(), vec![live]);
    }

    #[test]
    fn entities_of_kind_filters_live_entities() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world.create_entity(EntityKind::Agent, Tick(1));
        let archived_agent = world.create_entity(EntityKind::Agent, Tick(2));
        let office = world.create_entity(EntityKind::Office, Tick(3));

        world.archive_entity(archived_agent, Tick(4)).unwrap();

        assert_eq!(world.entities_of_kind(EntityKind::Agent).collect::<Vec<_>>(), vec![agent]);
        assert_eq!(
            world.entities_of_kind(EntityKind::Place).collect::<Vec<_>>(),
            vec![entity(2), entity(5)]
        );
        assert_eq!(
            world.entities_of_kind(EntityKind::Office).collect::<Vec<_>>(),
            vec![office]
        );
    }

    #[test]
    fn purge_cleans_components() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Agent, Tick(1));
        world
            .insert_component_name(id, Name("Aster".to_string()))
            .unwrap();
        world
            .insert_component_agent_data(
                id,
                AgentData {
                    control_source: ControlSource::Ai,
                },
            )
            .unwrap();

        world.archive_entity(id, Tick(2)).unwrap();
        world.purge_entity(id).unwrap();

        assert_eq!(world.entity_meta(id), None);
        assert_eq!(world.get_component_name(id), None);
        assert_eq!(world.get_component_agent_data(id), None);
    }

    #[test]
    fn component_crud_roundtrip() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Office, Tick(1));

        world
            .insert_component_name(id, Name("Ledger Hall".to_string()))
            .unwrap();
        assert_eq!(
            world.get_component_name(id),
            Some(&Name("Ledger Hall".to_string()))
        );

        world.get_component_name_mut(id).unwrap().0.push_str(" Annex");
        assert_eq!(
            world.get_component_name(id),
            Some(&Name("Ledger Hall Annex".to_string()))
        );

        let removed = world.remove_component_name(id).unwrap();
        assert_eq!(removed, Some(Name("Ledger Hall Annex".to_string())));
        assert_eq!(world.get_component_name(id), None);
    }

    #[test]
    fn insert_on_archived_entity_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Agent, Tick(1));
        world.archive_entity(id, Tick(2)).unwrap();

        let err = world
            .insert_component_name(id, Name("Ash".to_string()))
            .unwrap_err();

        assert!(matches!(err, WorldError::ArchivedEntity(actual) if actual == id));
    }

    #[test]
    fn insert_duplicate_component_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Agent, Tick(1));
        world
            .insert_component_name(id, Name("Mira".to_string()))
            .unwrap();

        let err = world
            .insert_component_name(id, Name("Mira".to_string()))
            .unwrap_err();

        assert!(matches!(
            err,
            WorldError::DuplicateComponent {
                entity,
                component_type: "Name",
            } if entity == id
        ));
    }

    #[test]
    fn insert_agent_data_on_non_agent_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Office, Tick(1));

        let err = world
            .insert_component_agent_data(
                id,
                AgentData {
                    control_source: ControlSource::Human,
                },
            )
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn get_missing_component_returns_none() {
        let world = World::new(Topology::new()).unwrap();
        let missing = entity(99);

        assert_eq!(world.get_component_name(missing), None);
    }

    #[test]
    fn remove_missing_component_returns_none() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Faction, Tick(1));

        assert_eq!(world.remove_component_name(id).unwrap(), None);
    }

    #[test]
    fn entities_with_name_returns_live_entities() {
        let mut world = World::new(Topology::new()).unwrap();
        let named_agent = world.create_entity(EntityKind::Agent, Tick(1));
        let unnamed_agent = world.create_entity(EntityKind::Agent, Tick(2));
        let archived_named = world.create_entity(EntityKind::Office, Tick(3));

        world
            .insert_component_name(named_agent, Name("Aster".to_string()))
            .unwrap();
        world
            .insert_component_name(archived_named, Name("Old Hall".to_string()))
            .unwrap();
        world.archive_entity(archived_named, Tick(4)).unwrap();

        assert_eq!(world.entities_with_name().collect::<Vec<_>>(), vec![named_agent]);
        assert!(!world.has_component_name(unnamed_agent));
    }

    #[test]
    fn query_name_returns_sorted_pairs() {
        let mut world = World::new(Topology::new()).unwrap();
        let second = world.create_entity(EntityKind::Office, Tick(1));
        let third = world.create_entity(EntityKind::Faction, Tick(2));
        let first = world.create_entity(EntityKind::Agent, Tick(3));

        world
            .insert_component_name(second, Name("Ledger Hall".to_string()))
            .unwrap();
        world
            .insert_component_name(third, Name("River Pact".to_string()))
            .unwrap();
        world
            .insert_component_name(first, Name("Aster".to_string()))
            .unwrap();

        let pairs = world
            .query_name()
            .map(|(entity, name)| (entity, name.0.as_str()))
            .collect::<Vec<_>>();

        assert_eq!(
            pairs,
            vec![
                (second, "Ledger Hall"),
                (third, "River Pact"),
                (first, "Aster"),
            ]
        );
    }

    #[test]
    fn entities_with_agent_data_returns_live_entities() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world.create_entity(EntityKind::Agent, Tick(1));
        let archived_agent = world.create_entity(EntityKind::Agent, Tick(2));

        world
            .insert_component_agent_data(
                agent,
                AgentData {
                    control_source: ControlSource::Human,
                },
            )
            .unwrap();
        world
            .insert_component_agent_data(
                archived_agent,
                AgentData {
                    control_source: ControlSource::Ai,
                },
            )
            .unwrap();
        world.archive_entity(archived_agent, Tick(3)).unwrap();

        assert_eq!(
            world.entities_with_agent_data().collect::<Vec<_>>(),
            vec![agent]
        );
    }

    #[test]
    fn query_agent_data_returns_sorted_pairs() {
        let mut world = World::new(Topology::new()).unwrap();
        let first = world.create_entity(EntityKind::Agent, Tick(1));
        let second = world.create_entity(EntityKind::Agent, Tick(2));

        world
            .insert_component_agent_data(
                first,
                AgentData {
                    control_source: ControlSource::Human,
                },
            )
            .unwrap();
        world
            .insert_component_agent_data(
                second,
                AgentData {
                    control_source: ControlSource::Ai,
                },
            )
            .unwrap();

        let pairs = world
            .query_agent_data()
            .map(|(entity, agent_data)| (entity, agent_data.control_source))
            .collect::<Vec<_>>();

        assert_eq!(
            pairs,
            vec![
                (first, ControlSource::Human),
                (second, ControlSource::Ai),
            ]
        );
    }

    #[test]
    fn entities_with_name_and_agent_data_returns_intersection() {
        let mut world = World::new(Topology::new()).unwrap();
        let full = world.create_entity(EntityKind::Agent, Tick(1));
        let name_only = world.create_entity(EntityKind::Office, Tick(2));
        let agent_only = world.create_entity(EntityKind::Agent, Tick(3));
        let archived_full = world.create_entity(EntityKind::Agent, Tick(4));

        world
            .insert_component_name(full, Name("Aster".to_string()))
            .unwrap();
        world
            .insert_component_agent_data(
                full,
                AgentData {
                    control_source: ControlSource::Ai,
                },
            )
            .unwrap();
        world
            .insert_component_name(name_only, Name("Ledger Hall".to_string()))
            .unwrap();
        world
            .insert_component_agent_data(
                agent_only,
                AgentData {
                    control_source: ControlSource::Human,
                },
            )
            .unwrap();
        world
            .insert_component_name(archived_full, Name("Ash".to_string()))
            .unwrap();
        world
            .insert_component_agent_data(
                archived_full,
                AgentData {
                    control_source: ControlSource::Human,
                },
            )
            .unwrap();
        world.archive_entity(archived_full, Tick(5)).unwrap();

        assert_eq!(
            world
                .entities_with_name_and_agent_data()
                .collect::<Vec<_>>(),
            vec![full]
        );
    }

    #[test]
    fn query_name_and_agent_data_returns_sorted_tuples() {
        let mut world = World::new(Topology::new()).unwrap();
        let first = world.create_entity(EntityKind::Agent, Tick(1));
        let second = world.create_entity(EntityKind::Agent, Tick(2));
        let partial = world.create_entity(EntityKind::Office, Tick(3));

        world
            .insert_component_name(first, Name("Aster".to_string()))
            .unwrap();
        world
            .insert_component_agent_data(
                first,
                AgentData {
                    control_source: ControlSource::Human,
                },
            )
            .unwrap();
        world
            .insert_component_name(second, Name("Bram".to_string()))
            .unwrap();
        world
            .insert_component_agent_data(
                second,
                AgentData {
                    control_source: ControlSource::Ai,
                },
            )
            .unwrap();
        world
            .insert_component_name(partial, Name("Ledger Hall".to_string()))
            .unwrap();

        let tuples = world
            .query_name_and_agent_data()
            .map(|(entity, name, agent_data)| (entity, name.0.as_str(), agent_data.control_source))
            .collect::<Vec<_>>();

        assert_eq!(
            tuples,
            vec![
                (first, "Aster", ControlSource::Human),
                (second, "Bram", ControlSource::Ai),
            ]
        );
    }

    #[test]
    fn filtering_query_preserves_relative_order() {
        let mut world = World::new(Topology::new()).unwrap();
        let first = world.create_entity(EntityKind::Agent, Tick(1));
        let second = world.create_entity(EntityKind::Office, Tick(2));
        let third = world.create_entity(EntityKind::Agent, Tick(3));

        world
            .insert_component_name(first, Name("Aster".to_string()))
            .unwrap();
        world
            .insert_component_name(second, Name("Ledger Hall".to_string()))
            .unwrap();
        world
            .insert_component_name(third, Name("Bram".to_string()))
            .unwrap();

        let filtered = world
            .query_name()
            .filter(|(_, name)| name.0.starts_with('A') || name.0.contains("Hall"))
            .map(|(entity, _)| entity)
            .collect::<Vec<_>>();

        assert_eq!(filtered, vec![first, second]);
    }

    #[test]
    fn empty_queries_are_safe() {
        let world = World::new(Topology::new()).unwrap();

        assert_eq!(world.entities().count(), 0);
        assert_eq!(world.all_entities().count(), 0);
        assert_eq!(world.entities_with_name().count(), 0);
        assert_eq!(world.query_name().count(), 0);
        assert_eq!(world.entities_with_agent_data().count(), 0);
        assert_eq!(world.query_agent_data().count(), 0);
        assert_eq!(world.entities_with_name_and_agent_data().count(), 0);
        assert_eq!(world.query_name_and_agent_data().count(), 0);
    }

    #[test]
    fn count_helpers_report_live_totals() {
        let mut world = World::new(test_topology()).unwrap();
        let live_named_agent = world.create_entity(EntityKind::Agent, Tick(1));
        let archived_named_agent = world.create_entity(EntityKind::Agent, Tick(2));
        let live_named_office = world.create_entity(EntityKind::Office, Tick(3));

        world
            .insert_component_name(live_named_agent, Name("Aster".to_string()))
            .unwrap();
        world
            .insert_component_name(archived_named_agent, Name("Ash".to_string()))
            .unwrap();
        world
            .insert_component_agent_data(
                live_named_agent,
                AgentData {
                    control_source: ControlSource::Ai,
                },
            )
            .unwrap();
        world
            .insert_component_agent_data(
                archived_named_agent,
                AgentData {
                    control_source: ControlSource::Human,
                },
            )
            .unwrap();
        world
            .insert_component_name(live_named_office, Name("Ledger Hall".to_string()))
            .unwrap();
        world.archive_entity(archived_named_agent, Tick(4)).unwrap();

        assert_eq!(world.entity_count(), 4);
        assert_eq!(world.count_with_name(), 2);
        assert_eq!(world.count_with_agent_data(), 1);
    }

    #[test]
    fn query_results_are_deterministic_across_identical_sequences() {
        fn build_world() -> World {
            let mut world = World::new(test_topology()).unwrap();
            let aster = world.create_entity(EntityKind::Agent, Tick(1));
            let ledger = world.create_entity(EntityKind::Office, Tick(2));
            let bram = world.create_entity(EntityKind::Agent, Tick(3));

            world
                .insert_component_name(aster, Name("Aster".to_string()))
                .unwrap();
            world
                .insert_component_agent_data(
                    aster,
                    AgentData {
                        control_source: ControlSource::Human,
                    },
                )
                .unwrap();
            world
                .insert_component_name(ledger, Name("Ledger Hall".to_string()))
                .unwrap();
            world
                .insert_component_name(bram, Name("Bram".to_string()))
                .unwrap();
            world
                .insert_component_agent_data(
                    bram,
                    AgentData {
                        control_source: ControlSource::Ai,
                    },
                )
                .unwrap();

            world
        }

        let left = build_world();
        let right = build_world();

        assert_eq!(left.entities().collect::<Vec<_>>(), right.entities().collect::<Vec<_>>());
        assert_eq!(
            left.all_entities().collect::<Vec<_>>(),
            right.all_entities().collect::<Vec<_>>()
        );
        assert_eq!(
            left.query_name()
                .map(|(entity, name)| (entity, name.0.as_str()))
                .collect::<Vec<_>>(),
            right
                .query_name()
                .map(|(entity, name)| (entity, name.0.as_str()))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            left.query_name_and_agent_data()
                .map(|(entity, name, agent_data)| (entity, name.0.as_str(), agent_data.control_source))
                .collect::<Vec<_>>(),
            right
                .query_name_and_agent_data()
                .map(|(entity, name, agent_data)| (entity, name.0.as_str(), agent_data.control_source))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn topology_accessible() {
        let world = World::new(test_topology()).unwrap();

        assert_eq!(world.topology().place_count(), 2);
        assert_eq!(world.topology().place(entity(5)).unwrap().name, "Square");
    }

    #[test]
    fn world_new_starts_with_empty_non_topological_component_tables() {
        let world = World::new(test_topology()).unwrap();

        for place_id in [entity(2), entity(5)] {
            assert_eq!(world.get_component_name(place_id), None);
            assert_eq!(world.get_component_agent_data(place_id), None);
        }
    }

    #[test]
    fn archive_topology_place_errors() {
        let mut world = World::new(test_topology()).unwrap();
        let place_id = entity(2);

        let err = world.archive_entity(place_id, Tick(8)).unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
        assert!(world.is_alive(place_id));
    }

    #[test]
    fn purge_topology_place_errors() {
        let mut world = World::new(test_topology()).unwrap();
        let place_id = entity(5);

        let err = world.purge_entity(place_id).unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
        assert!(world.is_alive(place_id));
    }
}
