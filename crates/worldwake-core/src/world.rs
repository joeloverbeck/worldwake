//! Authoritative world boundary over entity lifecycle, component tables, and topology.

use crate::{
    AgentData, ComponentTables, EntityAllocator, EntityId, EntityKind, EntityMeta, Name, Tick,
    Topology, WorldError,
};

macro_rules! world_component_methods {
    (
        $insert_fn:ident,
        $get_fn:ident,
        $get_mut_fn:ident,
        $remove_fn:ident,
        $has_fn:ident,
        $table_insert:ident,
        $table_get:ident,
        $table_get_mut:ident,
        $table_remove:ident,
        $table_has:ident,
        $component_ty:ty,
        $component_name:literal,
        $kind_check:expr
    ) => {
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

    world_component_methods!(
        insert_component_name,
        get_component_name,
        get_component_name_mut,
        remove_component_name,
        has_component_name,
        insert_name,
        get_name,
        get_name_mut,
        remove_name,
        has_name,
        Name,
        "Name",
        |_| true
    );

    world_component_methods!(
        insert_component_agent_data,
        get_component_agent_data,
        get_component_agent_data_mut,
        remove_component_agent_data,
        has_component_agent_data,
        insert_agent_data,
        get_agent_data,
        get_agent_data_mut,
        remove_agent_data,
        has_agent_data,
        AgentData,
        "AgentData",
        |kind| kind == EntityKind::Agent
    );
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
    fn archive_entity_marks_non_live() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Office, Tick(3));

        world.archive_entity(id, Tick(9)).unwrap();

        assert!(!world.is_alive(id));
        assert!(world.is_archived(id));
        assert_eq!(world.entity_meta(id).unwrap().archived_at, Some(Tick(9)));
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
