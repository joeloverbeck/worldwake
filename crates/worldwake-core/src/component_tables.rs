//! Explicit typed component storage.

use crate::{
    component_schema::with_component_schema_entries,
    components::{AgentData, Name},
    drives::DriveThresholds,
    items::{Container, ItemLot, UniqueItem},
    needs::{DeprivationExposure, HomeostaticNeeds, MetabolismProfile},
    production::{
        CarryCapacity, InTransitOnEdge, KnownRecipes, ProductionJob, ResourceSource,
        WorkstationMarker,
    },
    trade::{DemandMemory, MerchandiseProfile, SubstitutePreferences, TradeDispositionProfile},
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

with_component_schema_entries!(
    forward_authoritative_components,
    define_component_tables_struct
);
with_component_schema_entries!(
    forward_authoritative_components,
    define_component_table_impls
);

#[cfg(test)]
mod tests {
    use super::ComponentTables;
    use crate::{
        components::{AgentData, Name},
        test_utils::{
            sample_demand_memory, sample_merchandise_profile, sample_substitute_preferences,
            sample_trade_disposition_profile,
        },
        BodyPart, CarryCapacity, CommodityKind, Container, ControlSource, DeprivationExposure,
        DeprivationKind, DriveThresholds, EntityId, HomeostaticNeeds, InTransitOnEdge, ItemLot,
        KnownRecipes, LoadUnits, LotOperation, MetabolismProfile, Permille, ProductionJob,
        ProvenanceEntry, Quantity, ResourceSource, Tick, TravelEdgeId, UniqueItem, UniqueItemKind,
        WorkstationMarker, WorkstationTag, Wound, WoundCause, WoundList,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;

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
        assert_eq!(tables.iter_homeostatic_needs().count(), 0);
        assert_eq!(tables.iter_deprivation_exposures().count(), 0);
        assert_eq!(tables.iter_metabolism_profiles().count(), 0);
        assert_eq!(tables.iter_carry_capacities().count(), 0);
        assert_eq!(tables.iter_known_recipes().count(), 0);
        assert_eq!(tables.iter_demand_memories().count(), 0);
        assert_eq!(tables.iter_trade_disposition_profiles().count(), 0);
        assert_eq!(tables.iter_merchandise_profiles().count(), 0);
        assert_eq!(tables.iter_substitute_preferences().count(), 0);
        assert_eq!(tables.iter_workstation_markers().count(), 0);
        assert_eq!(tables.iter_resource_sources().count(), 0);
        assert_eq!(tables.iter_production_jobs().count(), 0);
        assert_eq!(tables.iter_in_transit_on_edges().count(), 0);
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
                bleed_rate_per_tick: Permille::new(0).unwrap(),
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
    fn insert_and_get_homeostatic_needs() {
        let mut tables = ComponentTables::default();
        let id = entity(16);
        let needs = HomeostaticNeeds::new(
            Permille::new(10).unwrap(),
            Permille::new(20).unwrap(),
            Permille::new(30).unwrap(),
            Permille::new(40).unwrap(),
            Permille::new(50).unwrap(),
        );

        assert_eq!(tables.insert_homeostatic_needs(id, needs), None);
        assert_eq!(tables.get_homeostatic_needs(id), Some(&needs));
        assert!(tables.has_homeostatic_needs(id));
    }

    #[test]
    fn insert_and_get_deprivation_exposure() {
        let mut tables = ComponentTables::default();
        let id = entity(17);
        let exposure = DeprivationExposure {
            hunger_critical_ticks: 4,
            thirst_critical_ticks: 5,
            fatigue_critical_ticks: 6,
            bladder_critical_ticks: 7,
        };

        assert_eq!(tables.insert_deprivation_exposure(id, exposure), None);
        assert_eq!(tables.get_deprivation_exposure(id), Some(&exposure));
        assert!(tables.has_deprivation_exposure(id));
    }

    #[test]
    fn insert_and_get_metabolism_profile() {
        let mut tables = ComponentTables::default();
        let id = entity(18);
        let profile = MetabolismProfile::new(
            Permille::new(2).unwrap(),
            Permille::new(3).unwrap(),
            Permille::new(4).unwrap(),
            Permille::new(5).unwrap(),
            Permille::new(6).unwrap(),
            Permille::new(30).unwrap(),
            NonZeroU32::new(100).unwrap(),
            NonZeroU32::new(90).unwrap(),
            NonZeroU32::new(80).unwrap(),
            NonZeroU32::new(70).unwrap(),
            NonZeroU32::new(8).unwrap(),
            NonZeroU32::new(10).unwrap(),
        );

        assert_eq!(tables.insert_metabolism_profile(id, profile), None);
        assert_eq!(tables.get_metabolism_profile(id), Some(&profile));
        assert!(tables.has_metabolism_profile(id));
    }

    #[test]
    fn insert_and_get_resource_source() {
        let mut tables = ComponentTables::default();
        let id = entity(19);
        let source = ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(7),
            max_quantity: Quantity(12),
            regeneration_ticks_per_unit: Some(NonZeroU32::new(5).unwrap()),
            last_regeneration_tick: Some(Tick(9)),
        };

        assert_eq!(tables.insert_resource_source(id, source.clone()), None);
        assert_eq!(tables.get_resource_source(id), Some(&source));
        assert!(tables.has_resource_source(id));
        assert_eq!(tables.remove_resource_source(id), Some(source));
        assert_eq!(tables.get_resource_source(id), None);
    }

    #[test]
    fn insert_and_get_carry_capacity() {
        let mut tables = ComponentTables::default();
        let id = entity(20);
        let capacity = CarryCapacity(LoadUnits(18));

        assert_eq!(tables.insert_carry_capacity(id, capacity), None);
        assert_eq!(tables.get_carry_capacity(id), Some(&capacity));
        assert!(tables.has_carry_capacity(id));
        assert_eq!(tables.remove_carry_capacity(id), Some(capacity));
        assert_eq!(tables.get_carry_capacity(id), None);
    }

    #[test]
    fn insert_and_get_known_recipes() {
        let mut tables = ComponentTables::default();
        let id = entity(21);
        let recipes = KnownRecipes::with([crate::RecipeId(3), crate::RecipeId(1)]);

        assert_eq!(tables.insert_known_recipes(id, recipes.clone()), None);
        assert_eq!(tables.get_known_recipes(id), Some(&recipes));
        assert!(tables.has_known_recipes(id));
        assert_eq!(tables.remove_known_recipes(id), Some(recipes));
        assert_eq!(tables.get_known_recipes(id), None);
    }

    #[test]
    fn insert_and_get_demand_memory() {
        let mut tables = ComponentTables::default();
        let id = entity(30);
        let memory = sample_demand_memory();

        assert_eq!(tables.insert_demand_memory(id, memory.clone()), None);
        assert_eq!(tables.get_demand_memory(id), Some(&memory));
        assert!(tables.has_demand_memory(id));
        assert_eq!(tables.remove_demand_memory(id), Some(memory));
        assert_eq!(tables.get_demand_memory(id), None);
    }

    #[test]
    fn trade_disposition_profile_insert_get_remove_has_cycle() {
        let mut tables = ComponentTables::default();
        let id = entity(32);
        let profile = sample_trade_disposition_profile();

        assert_eq!(
            tables.insert_trade_disposition_profile(id, profile.clone()),
            None
        );
        assert_eq!(tables.get_trade_disposition_profile(id), Some(&profile));
        assert!(tables.has_trade_disposition_profile(id));
        assert_eq!(tables.remove_trade_disposition_profile(id), Some(profile));
        assert_eq!(tables.get_trade_disposition_profile(id), None);
    }

    #[test]
    fn insert_and_get_merchandise_profile() {
        let mut tables = ComponentTables::default();
        let id = entity(31);
        let mut profile = sample_merchandise_profile();
        profile.sale_kinds.insert(CommodityKind::Firewood);

        assert_eq!(tables.insert_merchandise_profile(id, profile.clone()), None);
        assert_eq!(tables.get_merchandise_profile(id), Some(&profile));
        assert!(tables.has_merchandise_profile(id));
        assert_eq!(tables.remove_merchandise_profile(id), Some(profile));
        assert_eq!(tables.get_merchandise_profile(id), None);
    }

    #[test]
    fn substitute_preferences_insert_get_remove_has_cycle() {
        let mut tables = ComponentTables::default();
        let id = entity(33);
        let preferences = sample_substitute_preferences();

        assert_eq!(
            tables.insert_substitute_preferences(id, preferences.clone()),
            None
        );
        assert_eq!(tables.get_substitute_preferences(id), Some(&preferences));
        assert!(tables.has_substitute_preferences(id));
        assert_eq!(tables.remove_substitute_preferences(id), Some(preferences));
        assert_eq!(tables.get_substitute_preferences(id), None);
    }

    #[test]
    fn insert_and_get_workstation_marker() {
        let mut tables = ComponentTables::default();
        let id = entity(22);
        let marker = WorkstationMarker(WorkstationTag::Mill);

        assert_eq!(tables.insert_workstation_marker(id, marker), None);
        assert_eq!(tables.get_workstation_marker(id), Some(&marker));
        assert!(tables.has_workstation_marker(id));
        assert_eq!(tables.remove_workstation_marker(id), Some(marker));
        assert_eq!(tables.get_workstation_marker(id), None);
    }

    #[test]
    fn insert_and_get_production_job() {
        let mut tables = ComponentTables::default();
        let id = entity(23);
        let job = ProductionJob {
            recipe_id: crate::RecipeId(6),
            worker: entity(3),
            staged_inputs_container: entity(8),
            progress_ticks: 14,
        };

        assert_eq!(tables.insert_production_job(id, job.clone()), None);
        assert_eq!(tables.get_production_job(id), Some(&job));
        assert!(tables.has_production_job(id));
        assert_eq!(tables.remove_production_job(id), Some(job));
        assert_eq!(tables.get_production_job(id), None);
    }

    #[test]
    fn insert_and_get_in_transit_on_edge() {
        let mut tables = ComponentTables::default();
        let id = entity(24);
        let transit = InTransitOnEdge {
            edge_id: TravelEdgeId(5),
            origin: entity(2),
            destination: entity(3),
            departure_tick: Tick(11),
            arrival_tick: Tick(17),
        };

        assert_eq!(tables.insert_in_transit_on_edge(id, transit.clone()), None);
        assert_eq!(tables.get_in_transit_on_edge(id), Some(&transit));
        assert!(tables.has_in_transit_on_edge(id));
        assert_eq!(tables.remove_in_transit_on_edge(id), Some(transit));
        assert_eq!(tables.get_in_transit_on_edge(id), None);
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
                    bleed_rate_per_tick: Permille::new(0).unwrap(),
                }],
            },
        );
        tables.insert_homeostatic_needs(id, HomeostaticNeeds::default());
        tables.insert_deprivation_exposure(id, DeprivationExposure::default());
        tables.insert_metabolism_profile(id, MetabolismProfile::default());
        tables.insert_carry_capacity(id, CarryCapacity(LoadUnits(7)));
        tables.insert_known_recipes(id, KnownRecipes::with([crate::RecipeId(8)]));
        tables.insert_substitute_preferences(id, sample_substitute_preferences());
        tables.insert_workstation_marker(id, WorkstationMarker(WorkstationTag::Forge));
        tables.insert_production_job(
            id,
            ProductionJob {
                recipe_id: crate::RecipeId(9),
                worker: entity(11),
                staged_inputs_container: entity(12),
                progress_ticks: 5,
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
        tables.insert_in_transit_on_edge(
            id,
            InTransitOnEdge {
                edge_id: TravelEdgeId(3),
                origin: entity(1),
                destination: entity(2),
                departure_tick: Tick(3),
                arrival_tick: Tick(9),
            },
        );

        tables.remove_all(id);

        assert_eq!(tables.get_name(id), None);
        assert_eq!(tables.get_agent_data(id), None);
        assert_eq!(tables.get_wound_list(id), None);
        assert_eq!(tables.get_drive_thresholds(id), None);
        assert_eq!(tables.get_homeostatic_needs(id), None);
        assert_eq!(tables.get_deprivation_exposure(id), None);
        assert_eq!(tables.get_metabolism_profile(id), None);
        assert_eq!(tables.get_carry_capacity(id), None);
        assert_eq!(tables.get_known_recipes(id), None);
        assert_eq!(tables.get_substitute_preferences(id), None);
        assert_eq!(tables.get_item_lot(id), None);
        assert_eq!(tables.get_unique_item(id), None);
        assert_eq!(tables.get_container(id), None);
        assert_eq!(tables.get_in_transit_on_edge(id), None);
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
                    bleed_rate_per_tick: Permille::new(0).unwrap(),
                }],
            },
        );
        tables.insert_drive_thresholds(entity(10), DriveThresholds::default());
        tables.insert_homeostatic_needs(entity(13), HomeostaticNeeds::default());
        tables.insert_deprivation_exposure(entity(14), DeprivationExposure::default());
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
        tables.insert_metabolism_profile(entity(16), MetabolismProfile::default());
        tables.insert_substitute_preferences(entity(17), sample_substitute_preferences());

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
