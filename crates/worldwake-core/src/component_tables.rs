//! Explicit typed component storage.

use crate::{
    component_schema::with_authoritative_components,
    components::{AgentData, Name},
    drives::DriveThresholds,
    items::{Container, ItemLot, UniqueItem},
    wounds::WoundList,
    EntityId,
};
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
        pub fn $insert_fn(
            &mut self,
            entity: EntityId,
            component: $component_ty,
        ) -> Option<$component_ty> {
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
            self.$field
                .iter()
                .map(|(entity, component)| (*entity, component))
        }
    };
}

macro_rules! define_component_tables_struct {
    ($({ $field:ident, $component_ty:ty, $table_insert:ident, $table_get:ident, $table_get_mut:ident, $table_remove:ident, $table_has:ident, $table_iter:ident, $insert_fn:ident, $get_fn:ident, $get_mut_fn:ident, $remove_fn:ident, $has_fn:ident, $entities_fn:ident, $query_fn:ident, $count_fn:ident, $component_name:literal, $kind_check:expr, $component_variant:ident })*) => {
        /// Explicit typed component storage for non-topological authoritative components.
        #[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
        pub struct ComponentTables {
            $(pub(crate) $field: BTreeMap<EntityId, $component_ty>,)*
        }
    };
}

macro_rules! define_component_table_impls {
    ($({ $field:ident, $component_ty:ty, $table_insert:ident, $table_get:ident, $table_get_mut:ident, $table_remove:ident, $table_has:ident, $table_iter:ident, $insert_fn:ident, $get_fn:ident, $get_mut_fn:ident, $remove_fn:ident, $has_fn:ident, $entities_fn:ident, $query_fn:ident, $count_fn:ident, $component_name:literal, $kind_check:expr, $component_variant:ident })*) => {
        impl ComponentTables {
            $(
                component_table_methods!(
                    $table_insert,
                    $table_get,
                    $table_get_mut,
                    $table_remove,
                    $table_has,
                    $table_iter,
                    $field,
                    $component_ty
                );
            )*

            pub fn remove_all(&mut self, entity: EntityId) {
                $(self.$field.remove(&entity);)*
            }
        }
    };
}

with_authoritative_components!(define_component_tables_struct);
with_authoritative_components!(define_component_table_impls);

#[cfg(test)]
mod tests {
    use super::ComponentTables;
    use crate::{
        components::{AgentData, Name},
        BodyPart, CommodityKind, Container, ControlSource, DeprivationKind, EntityId, ItemLot,
        LoadUnits, LotOperation, Permille, ProvenanceEntry, Quantity, Tick, UniqueItem,
        UniqueItemKind, Wound, WoundCause, WoundList, DriveThresholds,
    };
    use std::collections::{BTreeMap, BTreeSet};

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
        assert_eq!(tables.iter_wound_lists().count(), 0);
        assert_eq!(tables.iter_drive_thresholds().count(), 0);
        assert_eq!(tables.iter_item_lots().count(), 0);
        assert_eq!(tables.iter_unique_items().count(), 0);
        assert_eq!(tables.iter_containers().count(), 0);
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
    fn insert_and_get_wound_list() {
        let mut tables = ComponentTables::default();
        let id = entity(5);
        let wounds = WoundList {
            wounds: vec![Wound {
                body_part: BodyPart::Torso,
                cause: WoundCause::Deprivation(DeprivationKind::Starvation),
                severity: Permille::new(800).unwrap(),
                inflicted_at: Tick(7),
            }],
        };

        assert_eq!(tables.insert_wound_list(id, wounds.clone()), None);
        assert_eq!(tables.get_wound_list(id), Some(&wounds));
    }

    #[test]
    fn insert_and_get_drive_thresholds() {
        let mut tables = ComponentTables::default();
        let id = entity(6);
        let thresholds = DriveThresholds::default();

        assert_eq!(tables.insert_drive_thresholds(id, thresholds), None);
        assert_eq!(tables.get_drive_thresholds(id), Some(&thresholds));
        assert!(tables.has_drive_thresholds(id));
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
        tables.insert_wound_list(
            id,
            WoundList {
                wounds: vec![Wound {
                    body_part: BodyPart::Head,
                    cause: WoundCause::Deprivation(DeprivationKind::Dehydration),
                    severity: Permille::new(550).unwrap(),
                    inflicted_at: Tick(2),
                }],
            },
        );
        tables.insert_item_lot(
            id,
            ItemLot {
                commodity: crate::CommodityKind::Apple,
                quantity: Quantity(2),
                provenance: vec![ProvenanceEntry {
                    tick: Tick(1),
                    event_id: None,
                    operation: LotOperation::Created,
                    related_lot: None,
                    amount: Quantity(2),
                }],
            },
        );
        tables.insert_unique_item(
            id,
            UniqueItem {
                kind: UniqueItemKind::Weapon,
                name: Some("Rusty Sword".to_string()),
                metadata: BTreeMap::from([("quality".to_string(), "poor".to_string())]),
            },
        );
        tables.insert_container(
            id,
            Container {
                capacity: LoadUnits(9),
                allowed_commodities: Some(BTreeSet::from([CommodityKind::Apple])),
                allows_unique_items: false,
                allows_nested_containers: false,
            },
        );

        tables.remove_all(id);

        assert_eq!(tables.get_name(id), None);
        assert_eq!(tables.get_agent_data(id), None);
        assert_eq!(tables.get_wound_list(id), None);
        assert_eq!(tables.get_drive_thresholds(id), None);
        assert_eq!(tables.get_item_lot(id), None);
        assert_eq!(tables.get_unique_item(id), None);
        assert_eq!(tables.get_container(id), None);
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
        tables.insert_wound_list(
            entity(9),
            WoundList {
                wounds: vec![Wound {
                    body_part: BodyPart::LeftArm,
                    cause: WoundCause::Deprivation(DeprivationKind::Starvation),
                    severity: Permille::new(300).unwrap(),
                    inflicted_at: Tick(4),
                }],
            },
        );
        tables.insert_drive_thresholds(entity(10), DriveThresholds::default());
        tables.insert_item_lot(
            entity(11),
            ItemLot {
                commodity: crate::CommodityKind::Water,
                quantity: Quantity(6),
                provenance: vec![ProvenanceEntry {
                    tick: Tick(3),
                    event_id: None,
                    operation: LotOperation::Created,
                    related_lot: None,
                    amount: Quantity(6),
                }],
            },
        );
        tables.insert_unique_item(
            entity(12),
            UniqueItem {
                kind: UniqueItemKind::Artifact,
                name: None,
                metadata: BTreeMap::from([("origin".to_string(), "vault".to_string())]),
            },
        );
        tables.insert_container(
            entity(15),
            Container {
                capacity: LoadUnits(12),
                allowed_commodities: Some(BTreeSet::from([
                    CommodityKind::Bread,
                    CommodityKind::Water,
                ])),
                allows_unique_items: true,
                allows_nested_containers: false,
            },
        );

        let bytes = bincode::serialize(&tables).unwrap();
        let roundtrip: ComponentTables = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, tables);
    }

    #[test]
    fn insert_and_get_item_lot() {
        let mut tables = ComponentTables::default();
        let id = entity(10);
        let lot = ItemLot {
            commodity: crate::CommodityKind::Bread,
            quantity: Quantity(4),
            provenance: vec![ProvenanceEntry {
                tick: Tick(2),
                event_id: None,
                operation: LotOperation::Created,
                related_lot: None,
                amount: Quantity(4),
            }],
        };

        assert_eq!(tables.insert_item_lot(id, lot.clone()), None);
        assert_eq!(tables.get_item_lot(id), Some(&lot));
        assert!(tables.has_item_lot(id));
    }

    #[test]
    fn insert_and_get_unique_item() {
        let mut tables = ComponentTables::default();
        let id = entity(12);
        let item = UniqueItem {
            kind: UniqueItemKind::SimpleTool,
            name: Some("Hammer".to_string()),
            metadata: BTreeMap::from([("material".to_string(), "wood".to_string())]),
        };

        assert_eq!(tables.insert_unique_item(id, item.clone()), None);
        assert_eq!(tables.get_unique_item(id), Some(&item));
        assert!(tables.has_unique_item(id));
    }

    #[test]
    fn insert_and_get_container() {
        let mut tables = ComponentTables::default();
        let id = entity(16);
        let container = Container {
            capacity: LoadUnits(21),
            allowed_commodities: Some(BTreeSet::from([
                CommodityKind::Coin,
                CommodityKind::Medicine,
            ])),
            allows_unique_items: true,
            allows_nested_containers: false,
        };

        assert_eq!(tables.insert_container(id, container.clone()), None);
        assert_eq!(tables.get_container(id), Some(&container));
        assert!(tables.has_container(id));
    }

    #[test]
    fn remove_all_clears_only_target_container_storage() {
        let mut tables = ComponentTables::default();
        let target = entity(17);
        let other = entity(18);

        tables.insert_container(
            target,
            Container {
                capacity: LoadUnits(5),
                allowed_commodities: None,
                allows_unique_items: true,
                allows_nested_containers: false,
            },
        );
        tables.insert_container(
            other,
            Container {
                capacity: LoadUnits(8),
                allowed_commodities: Some(BTreeSet::from([CommodityKind::Water])),
                allows_unique_items: false,
                allows_nested_containers: true,
            },
        );

        tables.remove_all(target);

        assert_eq!(tables.get_container(target), None);
        assert_eq!(
            tables
                .get_container(other)
                .map(|container| container.capacity),
            Some(LoadUnits(8))
        );
    }

    #[test]
    fn remove_all_clears_only_target_unique_item_storage() {
        let mut tables = ComponentTables::default();
        let target = entity(14);
        let other = entity(15);

        tables.insert_unique_item(
            target,
            UniqueItem {
                kind: UniqueItemKind::Contract,
                name: Some("Grain Charter".to_string()),
                metadata: BTreeMap::new(),
            },
        );
        tables.insert_unique_item(
            other,
            UniqueItem {
                kind: UniqueItemKind::Artifact,
                name: None,
                metadata: BTreeMap::from([("era".to_string(), "old".to_string())]),
            },
        );

        tables.remove_all(target);

        assert_eq!(tables.get_unique_item(target), None);
        assert_eq!(
            tables.get_unique_item(other).map(|item| item.kind),
            Some(UniqueItemKind::Artifact)
        );
    }
}
