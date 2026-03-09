//! Explicit typed component storage.

use crate::{components::{AgentData, Name}, EntityId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

macro_rules! component_table_methods {
    (
        $insert_fn:ident,
        $get_fn:ident,
        $get_mut_fn:ident,
        $remove_fn:ident,
        $has_fn:ident,
        $iter_fn:ident,
        $field:ident,
        $component_ty:ty
    ) => {
        pub fn $insert_fn(&mut self, entity: EntityId, component: $component_ty) -> Option<$component_ty> {
            self.$field.insert(entity, component)
        }

        pub fn $get_fn(&self, entity: EntityId) -> Option<&$component_ty> {
            self.$field.get(&entity)
        }

        pub fn $get_mut_fn(&mut self, entity: EntityId) -> Option<&mut $component_ty> {
            self.$field.get_mut(&entity)
        }

        pub fn $remove_fn(&mut self, entity: EntityId) -> Option<$component_ty> {
            self.$field.remove(&entity)
        }

        pub fn $has_fn(&self, entity: EntityId) -> bool {
            self.$field.contains_key(&entity)
        }

        pub fn $iter_fn(&self) -> impl Iterator<Item = (EntityId, &$component_ty)> + '_ {
            self.$field.iter().map(|(entity, component)| (*entity, component))
        }
    };
}

/// Explicit typed component storage for non-topological authoritative components.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComponentTables {
    pub(crate) names: BTreeMap<EntityId, Name>,
    pub(crate) agents: BTreeMap<EntityId, AgentData>,
}

impl ComponentTables {
    component_table_methods!(
        insert_name,
        get_name,
        get_name_mut,
        remove_name,
        has_name,
        iter_names,
        names,
        Name
    );

    component_table_methods!(
        insert_agent_data,
        get_agent_data,
        get_agent_data_mut,
        remove_agent_data,
        has_agent_data,
        iter_agent_data,
        agents,
        AgentData
    );

    pub fn remove_all(&mut self, entity: EntityId) {
        self.names.remove(&entity);
        self.agents.remove(&entity);
    }
}

#[cfg(test)]
mod tests {
    use super::ComponentTables;
    use crate::{
        components::{AgentData, Name},
        ControlSource, EntityId,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    #[test]
    fn default_tables_are_empty() {
        let tables = ComponentTables::default();

        assert_eq!(tables.iter_names().count(), 0);
        assert_eq!(tables.iter_agent_data().count(), 0);
    }

    #[test]
    fn insert_and_get_name() {
        let mut tables = ComponentTables::default();
        let id = entity(3);
        let name = Name("Aster".to_string());

        assert_eq!(tables.insert_name(id, name.clone()), None);
        assert_eq!(tables.get_name(id), Some(&name));
    }

    #[test]
    fn insert_and_get_agent_data() {
        let mut tables = ComponentTables::default();
        let id = entity(4);
        let agent = AgentData {
            control_source: ControlSource::Human,
        };

        assert_eq!(tables.insert_agent_data(id, agent.clone()), None);
        assert_eq!(tables.get_agent_data(id), Some(&agent));
    }

    #[test]
    fn remove_returns_value() {
        let mut tables = ComponentTables::default();
        let id = entity(5);
        let name = Name("Rowan".to_string());

        tables.insert_name(id, name.clone());

        assert_eq!(tables.remove_name(id), Some(name));
        assert_eq!(tables.get_name(id), None);
    }

    #[test]
    fn has_component_correct() {
        let mut tables = ComponentTables::default();
        let id = entity(6);

        assert!(!tables.has_name(id));
        tables.insert_name(id, Name("Lark".to_string()));
        assert!(tables.has_name(id));
        tables.remove_name(id);
        assert!(!tables.has_name(id));
    }

    #[test]
    fn iter_deterministic_order() {
        let mut tables = ComponentTables::default();

        for slot in [9, 1, 4, 2] {
            tables.insert_name(entity(slot), Name(format!("entity-{slot}")));
        }

        let seen = tables
            .iter_names()
            .map(|(entity, _)| entity)
            .collect::<Vec<_>>();

        assert_eq!(seen, vec![entity(1), entity(2), entity(4), entity(9)]);
    }

    #[test]
    fn remove_all_clears_entity() {
        let mut tables = ComponentTables::default();
        let id = entity(7);

        tables.insert_name(id, Name("Moth".to_string()));
        tables.insert_agent_data(
            id,
            AgentData {
                control_source: ControlSource::Ai,
            },
        );

        tables.remove_all(id);

        assert_eq!(tables.get_name(id), None);
        assert_eq!(tables.get_agent_data(id), None);
    }

    #[test]
    fn component_tables_bincode_roundtrip() {
        let mut tables = ComponentTables::default();
        let name_id = entity(2);
        let agent_id = entity(8);

        tables.insert_name(name_id, Name("Vale".to_string()));
        tables.insert_agent_data(
            agent_id,
            AgentData {
                control_source: ControlSource::None,
            },
        );

        let bytes = bincode::serialize(&tables).unwrap();
        let roundtrip: ComponentTables = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, tables);
    }
}
