//! Authoritative world boundary over entity lifecycle, component tables, and topology.

use crate::{
    component_schema::with_component_schema_entries, AgentData, BlockedIntentMemory, CarryCapacity,
    CombatProfile, CombatStance, CommodityKind, ComponentTables, ComponentValue, Container, DeadAt,
    DemandMemory, DeprivationExposure, DriveThresholds, EntityAllocator, EntityId, EntityKind,
    EntityMeta, EventId, HomeostaticNeeds, InTransitOnEdge, ItemLot, KnownRecipes, LoadUnits,
    LotOperation, MerchandiseProfile, MetabolismProfile, Name, PlaceTag, ProductionJob,
    ProvenanceEntry, Quantity, RelationTables, ResourceSource, SubstitutePreferences, Tick,
    Topology, TradeDispositionProfile, UniqueItem, UniqueItemKind, UtilityProfile,
    WorkstationMarker, WorldError, WoundList,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod lifecycle;
mod ownership;
mod placement;
mod relation_mutation;
mod reservations;
mod social;

macro_rules! world_component_api {
    ($({ $field:ident, $component_ty:ty, $table_insert:ident, $table_get:ident, $table_get_mut:ident, $table_remove:ident, $table_has:ident, $table_iter:ident, $insert_fn:ident, $get_fn:ident, $get_mut_fn:ident, $remove_fn:ident, $has_fn:ident, $entities_fn:ident, $query_fn:ident, $count_fn:ident, $component_name:literal, $kind_check:expr, $component_variant:ident })*) => {
        $(
            #[allow(dead_code)]
            pub(crate) fn $insert_fn(
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

            #[allow(dead_code)]
            pub(crate) fn $get_mut_fn(&mut self, entity: EntityId) -> Option<&mut $component_ty> {
                self.is_alive(entity)
                    .then(|| self.components.$table_get_mut(entity))?
            }

            #[allow(dead_code)]
            pub(crate) fn $remove_fn(&mut self, entity: EntityId) -> Result<Option<$component_ty>, WorldError> {
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

macro_rules! world_component_value_api {
    ($({ $field:ident, $component_ty:ty, $table_insert:ident, $table_get:ident, $table_get_mut:ident, $table_remove:ident, $table_has:ident, $table_iter:ident, $insert_fn:ident, $get_fn:ident, $get_mut_fn:ident, $remove_fn:ident, $has_fn:ident, $entities_fn:ident, $query_fn:ident, $count_fn:ident, $component_name:literal, $kind_check:expr, $component_variant:ident })*) => {
        pub(crate) fn component_values(&self, entity: EntityId) -> Vec<ComponentValue> {
            let mut values = Vec::new();

            $(
                if let Some(value) = self.$get_fn(entity).cloned() {
                    values.push(ComponentValue::$component_variant(value));
                }
            )*

            values
        }
    };
}

/// The authoritative simulation world.
///
/// All fields are private. External code accesses state through typed read
/// methods and controlled mutation methods.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct World {
    allocator: EntityAllocator,
    components: ComponentTables,
    relations: RelationTables,
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
            relations: RelationTables::default(),
            topology,
        })
    }

    pub(crate) fn create_entity(&mut self, kind: EntityKind, tick: Tick) -> EntityId {
        let entity = self.allocator.create_entity(kind, tick);
        if Self::requires_physical_placement(kind) {
            self.relations.in_transit.insert(entity);
        }
        entity
    }

    pub(crate) fn create_agent(
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

    pub(crate) fn create_office(&mut self, name: &str, tick: Tick) -> Result<EntityId, WorldError> {
        self.create_named_entity(EntityKind::Office, name, tick)
    }

    pub(crate) fn create_faction(
        &mut self,
        name: &str,
        tick: Tick,
    ) -> Result<EntityId, WorldError> {
        self.create_named_entity(EntityKind::Faction, name, tick)
    }

    pub(crate) fn create_item_lot(
        &mut self,
        commodity: CommodityKind,
        quantity: Quantity,
        tick: Tick,
    ) -> Result<EntityId, WorldError> {
        self.create_item_lot_with_provenance(
            commodity,
            quantity,
            tick,
            vec![ProvenanceEntry {
                tick,
                event_id: None,
                operation: LotOperation::Created,
                related_lot: None,
                amount: quantity,
            }],
        )
    }

    pub(crate) fn split_lot(
        &mut self,
        lot_id: EntityId,
        amount: Quantity,
        tick: Tick,
        event_id: Option<EventId>,
    ) -> Result<(EntityId, EntityId), WorldError> {
        if amount == Quantity(0) {
            return Err(WorldError::InvalidOperation(
                "split amount must be greater than zero".to_string(),
            ));
        }

        let (commodity, available) = {
            let lot = self.require_item_lot(lot_id)?;
            (lot.commodity, lot.quantity)
        };
        let remaining = available
            .checked_sub(amount)
            .ok_or(WorldError::InsufficientQuantity {
                entity: lot_id,
                requested: amount.0,
                available: available.0,
            })?;
        if remaining == Quantity(0) {
            return Err(WorldError::InsufficientQuantity {
                entity: lot_id,
                requested: amount.0,
                available: available.0,
            });
        }

        let new_lot_id = self.create_item_lot_with_provenance(
            commodity,
            amount,
            tick,
            vec![
                ProvenanceEntry {
                    tick,
                    event_id: None,
                    operation: LotOperation::Created,
                    related_lot: None,
                    amount,
                },
                ProvenanceEntry {
                    tick,
                    event_id,
                    operation: LotOperation::Split,
                    related_lot: Some(lot_id),
                    amount,
                },
            ],
        )?;

        {
            let source_lot = self.require_item_lot_mut(lot_id)?;
            source_lot.quantity = remaining;
            source_lot.provenance.push(ProvenanceEntry {
                tick,
                event_id,
                operation: LotOperation::Split,
                related_lot: Some(new_lot_id),
                amount,
            });
        }

        Ok((lot_id, new_lot_id))
    }

    pub(crate) fn merge_lots(
        &mut self,
        target_id: EntityId,
        source_id: EntityId,
        tick: Tick,
        event_id: Option<EventId>,
    ) -> Result<EntityId, WorldError> {
        if target_id == source_id {
            return Err(WorldError::InvalidOperation(
                "cannot merge a lot into itself".to_string(),
            ));
        }

        let (target_commodity, source_commodity, source_quantity) = {
            let target_lot = self.require_item_lot(target_id)?;
            let source_lot = self.require_item_lot(source_id)?;
            (
                target_lot.commodity,
                source_lot.commodity,
                source_lot.quantity,
            )
        };
        if target_commodity != source_commodity {
            return Err(WorldError::InvalidOperation(format!(
                "cannot merge {source_commodity:?} lot {source_id} into {target_commodity:?} lot {target_id}"
            )));
        }

        {
            let target_lot = self.require_item_lot_mut(target_id)?;
            target_lot.quantity = target_lot.quantity + source_quantity;
            target_lot.provenance.push(ProvenanceEntry {
                tick,
                event_id,
                operation: LotOperation::Merge,
                related_lot: Some(source_id),
                amount: source_quantity,
            });
        }

        {
            let source_lot = self.require_item_lot_mut(source_id)?;
            source_lot.provenance.push(ProvenanceEntry {
                tick,
                event_id,
                operation: LotOperation::Merge,
                related_lot: Some(target_id),
                amount: source_quantity,
            });
        }

        self.archive_entity(source_id, tick)?;

        Ok(target_id)
    }

    pub(crate) fn create_unique_item(
        &mut self,
        kind: UniqueItemKind,
        name: Option<&str>,
        metadata: BTreeMap<String, String>,
        tick: Tick,
    ) -> Result<EntityId, WorldError> {
        self.create_entity_with(EntityKind::UniqueItem, tick, |world, entity| {
            world.insert_component_unique_item(
                entity,
                UniqueItem {
                    kind,
                    name: name.map(str::to_owned),
                    metadata,
                },
            )
        })
    }

    pub(crate) fn create_container(
        &mut self,
        container: Container,
        tick: Tick,
    ) -> Result<EntityId, WorldError> {
        if container.capacity == LoadUnits(0) {
            return Err(WorldError::InvalidOperation(
                "container capacity must be greater than zero".to_string(),
            ));
        }

        self.create_entity_with(EntityKind::Container, tick, |world, entity| {
            world.insert_component_container(entity, container)
        })
    }

    pub(crate) fn archive_entity(&mut self, id: EntityId, tick: Tick) -> Result<(), WorldError> {
        if self.topology.place(id).is_some() {
            return Err(WorldError::InvalidOperation(format!(
                "cannot archive topology-owned place: {id}"
            )));
        }
        let dependencies = self.archive_dependencies(id)?;
        if !dependencies.is_empty() {
            let summary = dependencies
                .iter()
                .map(|dependency| dependency.kind.description())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(WorldError::PreconditionFailed(format!(
                "cannot archive entity {id} because it still has archive dependencies: {summary}"
            )));
        }

        self.allocator.archive_entity(id, tick)?;
        self.relations.remove_all(id);
        Ok(())
    }

    pub(crate) fn purge_entity(&mut self, id: EntityId) -> Result<(), WorldError> {
        if self.topology.place(id).is_some() {
            return Err(WorldError::InvalidOperation(format!(
                "cannot purge topology-owned place: {id}"
            )));
        }

        self.allocator.purge_entity(id)?;
        self.components.remove_all(id);
        self.relations.remove_all(id);
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

    #[must_use]
    pub fn place_has_tag(&self, place: EntityId, tag: PlaceTag) -> bool {
        self.topology
            .place(place)
            .is_some_and(|place_data| place_data.tags.contains(&tag))
    }

    pub fn archive_dependencies(
        &self,
        entity: EntityId,
    ) -> Result<Vec<crate::relations::ArchiveDependency>, WorldError> {
        self.ensure_alive(entity)?;
        Ok(self.relations.archive_dependencies(entity))
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
        self.query_name_and_agent_data()
            .map(|(entity, _, _)| entity)
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

    fn requires_physical_placement(kind: EntityKind) -> bool {
        matches!(
            kind,
            EntityKind::Agent
                | EntityKind::ItemLot
                | EntityKind::UniqueItem
                | EntityKind::Container
        )
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

    fn create_item_lot_with_provenance(
        &mut self,
        commodity: CommodityKind,
        quantity: Quantity,
        tick: Tick,
        provenance: Vec<ProvenanceEntry>,
    ) -> Result<EntityId, WorldError> {
        if quantity == Quantity(0) {
            return Err(WorldError::InvalidOperation(
                "item lot quantity must be greater than zero".to_string(),
            ));
        }

        self.create_entity_with(EntityKind::ItemLot, tick, |world, entity| {
            world.insert_component_item_lot(
                entity,
                ItemLot {
                    commodity,
                    quantity,
                    provenance,
                },
            )
        })
    }

    fn require_item_lot(&self, entity: EntityId) -> Result<&ItemLot, WorldError> {
        self.ensure_alive(entity)?;
        self.get_component_item_lot(entity)
            .ok_or(WorldError::ComponentNotFound {
                entity,
                component_type: "ItemLot",
            })
    }

    fn require_item_lot_mut(&mut self, entity: EntityId) -> Result<&mut ItemLot, WorldError> {
        self.ensure_alive(entity)?;
        self.get_component_item_lot_mut(entity)
            .ok_or(WorldError::ComponentNotFound {
                entity,
                component_type: "ItemLot",
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

    with_component_schema_entries!(forward_authoritative_components, world_component_api);
    with_component_schema_entries!(forward_authoritative_components, world_component_value_api);
}

#[cfg(test)]
mod tests {
    use super::World;
    use crate::{
        build_prototype_world,
        test_utils::{
            sample_blocked_intent_memory, sample_demand_memory, sample_merchandise_profile,
            sample_substitute_preferences, sample_trade_disposition_profile,
            sample_utility_profile,
        },
        AgentData, BodyPart, CarryCapacity, CombatProfile, CommodityKind, Container, ControlSource,
        DeadAt, DemandMemory, DeprivationExposure, DeprivationKind, DriveThresholds, EntityId,
        EntityKind, EventId, FactId, HomeostaticNeeds, InTransitOnEdge, ItemLot, KnownRecipes,
        LoadUnits, LotOperation, MerchandiseProfile, MetabolismProfile, Name, Permille, Place,
        PlaceTag, ProductionJob, ProvenanceEntry, Quantity, ReservationId, ReservationRecord,
        ResourceSource, SubstitutePreferences, Tick, TickRange, Topology, TradeDispositionProfile,
        TravelEdgeId, UniqueItem, UniqueItemKind, WorkstationMarker, WorkstationTag, WorldError,
        Wound, WoundCause, WoundList,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;

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

    fn open_container(capacity: u32) -> Container {
        Container {
            capacity: LoadUnits(capacity),
            allowed_commodities: None,
            allows_unique_items: true,
            allows_nested_containers: true,
        }
    }

    fn sample_wound_list() -> WoundList {
        WoundList {
            wounds: vec![Wound {
                id: crate::WoundId(1),
                body_part: BodyPart::Torso,
                cause: WoundCause::Deprivation(DeprivationKind::Starvation),
                severity: Permille::new(725).unwrap(),
                inflicted_at: Tick(6),
                bleed_rate_per_tick: Permille::new(0).unwrap(),
            }],
        }
    }

    fn sample_drive_thresholds() -> DriveThresholds {
        DriveThresholds::default()
    }

    fn sample_combat_profile() -> CombatProfile {
        CombatProfile::new(
            Permille::new(1000).unwrap(),
            Permille::new(700).unwrap(),
            Permille::new(630).unwrap(),
            Permille::new(590).unwrap(),
            Permille::new(70).unwrap(),
            Permille::new(20).unwrap(),
            Permille::new(15).unwrap(),
            Permille::new(125).unwrap(),
            Permille::new(30).unwrap(),
            NonZeroU32::new(6).unwrap(),
        )
    }

    fn sample_dead_at() -> DeadAt {
        DeadAt(Tick(21))
    }

    fn sample_homeostatic_needs() -> HomeostaticNeeds {
        HomeostaticNeeds::new(
            Permille::new(120).unwrap(),
            Permille::new(140).unwrap(),
            Permille::new(160).unwrap(),
            Permille::new(180).unwrap(),
            Permille::new(200).unwrap(),
        )
    }

    fn sample_deprivation_exposure() -> DeprivationExposure {
        DeprivationExposure {
            hunger_critical_ticks: 3,
            thirst_critical_ticks: 5,
            fatigue_critical_ticks: 7,
            bladder_critical_ticks: 11,
        }
    }

    fn sample_metabolism_profile() -> MetabolismProfile {
        MetabolismProfile::new(
            Permille::new(2).unwrap(),
            Permille::new(3).unwrap(),
            Permille::new(4).unwrap(),
            Permille::new(5).unwrap(),
            Permille::new(6).unwrap(),
            Permille::new(25).unwrap(),
            NonZeroU32::new(100).unwrap(),
            NonZeroU32::new(90).unwrap(),
            NonZeroU32::new(80).unwrap(),
            NonZeroU32::new(70).unwrap(),
            NonZeroU32::new(8).unwrap(),
            NonZeroU32::new(10).unwrap(),
        )
    }

    fn sample_resource_source() -> ResourceSource {
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(9),
            max_quantity: Quantity(15),
            regeneration_ticks_per_unit: Some(NonZeroU32::new(4).unwrap()),
            last_regeneration_tick: Some(Tick(7)),
        }
    }

    struct PurgeRelationFixture {
        item: EntityId,
        container: EntityId,
        holder: EntityId,
        owner: EntityId,
        reserver: EntityId,
        faction: EntityId,
        loyal_target: EntityId,
        office: EntityId,
        enemy: EntityId,
        place: EntityId,
        reservation_id: ReservationId,
        known_fact: FactId,
        believed_fact: FactId,
    }

    fn populate_relation_rows_for_purge_test(world: &mut World, fx: &PurgeRelationFixture) {
        world.relations.located_in.insert(fx.item, fx.place);
        world
            .relations
            .entities_at
            .insert(fx.place, [fx.item].into_iter().collect());
        world.relations.contained_by.insert(fx.item, fx.container);
        world
            .relations
            .contents_of
            .insert(fx.container, [fx.item].into_iter().collect());
        world.relations.possessed_by.insert(fx.item, fx.holder);
        world
            .relations
            .possessions_of
            .insert(fx.holder, [fx.item].into_iter().collect());
        world.relations.owned_by.insert(fx.item, fx.owner);
        world
            .relations
            .property_of
            .insert(fx.owner, [fx.item].into_iter().collect());
        world
            .relations
            .member_of
            .insert(fx.item, [fx.faction].into_iter().collect());
        world
            .relations
            .members_of
            .insert(fx.faction, [fx.item].into_iter().collect());
        world.relations.loyal_to.insert(
            fx.item,
            BTreeMap::from([(fx.loyal_target, Permille::new(650).unwrap())]),
        );
        world.relations.loyalty_from.insert(
            fx.loyal_target,
            BTreeMap::from([(fx.item, Permille::new(650).unwrap())]),
        );
        world.relations.office_holder.insert(fx.office, fx.item);
        world
            .relations
            .offices_held
            .insert(fx.item, [fx.office].into_iter().collect());
        world
            .relations
            .hostile_to
            .insert(fx.item, [fx.enemy].into_iter().collect());
        world
            .relations
            .hostility_from
            .insert(fx.enemy, [fx.item].into_iter().collect());
        world
            .relations
            .knows_fact
            .insert(fx.item, [fx.known_fact].into_iter().collect());
        world
            .relations
            .believes_fact
            .insert(fx.item, [fx.believed_fact].into_iter().collect());
        world.relations.reservations.insert(
            fx.reservation_id,
            ReservationRecord {
                id: fx.reservation_id,
                entity: fx.item,
                reserver: fx.reserver,
                range: TickRange::new(Tick(4), Tick(7)).unwrap(),
            },
        );
        world
            .relations
            .reservations_by_entity
            .insert(fx.item, [fx.reservation_id].into_iter().collect());
    }

    fn assert_populated_world_roundtrip(
        roundtrip: &World,
        agent: EntityId,
        office: EntityId,
        faction: EntityId,
        lot: EntityId,
        unique_item: EntityId,
        container: EntityId,
    ) {
        assert_eq!(
            roundtrip.entities().collect::<Vec<_>>(),
            vec![
                entity(2),
                entity(5),
                agent,
                office,
                faction,
                lot,
                unique_item,
                container,
            ]
        );
        assert_eq!(roundtrip.entity_kind(entity(2)), Some(EntityKind::Place));
        assert_eq!(roundtrip.entity_kind(entity(5)), Some(EntityKind::Place));
        assert_eq!(roundtrip.entity_kind(agent), Some(EntityKind::Agent));
        assert_eq!(roundtrip.entity_kind(office), Some(EntityKind::Office));
        assert_eq!(roundtrip.entity_kind(faction), Some(EntityKind::Faction));
        assert_eq!(roundtrip.entity_kind(lot), Some(EntityKind::ItemLot));
        assert_eq!(
            roundtrip.entity_kind(unique_item),
            Some(EntityKind::UniqueItem)
        );
        assert_eq!(
            roundtrip.entity_kind(container),
            Some(EntityKind::Container)
        );
        assert_eq!(
            roundtrip.get_component_name(agent),
            Some(&Name("Aster".to_string()))
        );
        assert_eq!(
            roundtrip.get_component_name(office),
            Some(&Name("Ledger Hall".to_string()))
        );
        assert_eq!(
            roundtrip.get_component_name(faction),
            Some(&Name("River Pact".to_string()))
        );
        assert_eq!(
            roundtrip.get_component_agent_data(agent),
            Some(&AgentData {
                control_source: ControlSource::Ai,
            })
        );
        assert_eq!(
            roundtrip.get_component_item_lot(lot),
            Some(&ItemLot {
                commodity: CommodityKind::Grain,
                quantity: Quantity(6),
                provenance: vec![ProvenanceEntry {
                    tick: Tick(10),
                    event_id: None,
                    operation: LotOperation::Created,
                    related_lot: None,
                    amount: Quantity(6),
                }],
            })
        );
        assert_eq!(
            roundtrip.get_component_unique_item(unique_item),
            Some(&UniqueItem {
                kind: UniqueItemKind::Artifact,
                name: Some("Court Seal".to_string()),
                metadata: BTreeMap::from([("origin".to_string(), "vault".to_string())]),
            })
        );
        assert_eq!(
            roundtrip.get_component_container(container),
            Some(&Container {
                capacity: LoadUnits(20),
                allowed_commodities: Some(BTreeSet::from([
                    CommodityKind::Coin,
                    CommodityKind::Grain,
                ])),
                allows_unique_items: false,
                allows_nested_containers: false,
            })
        );
    }

    fn seed_social_archive_blockers(
        world: &mut World,
        faction: EntityId,
        member: EntityId,
        loyal_subject: EntityId,
        hostile_subject: EntityId,
        office: EntityId,
        item: EntityId,
    ) {
        world
            .relations
            .member_of
            .insert(member, BTreeSet::from([faction]));
        world
            .relations
            .members_of
            .insert(faction, BTreeSet::from([member]));
        world.relations.loyal_to.insert(
            loyal_subject,
            BTreeMap::from([(faction, Permille::new(700).unwrap())]),
        );
        world.relations.loyalty_from.insert(
            faction,
            BTreeMap::from([(loyal_subject, Permille::new(700).unwrap())]),
        );
        world
            .relations
            .hostile_to
            .insert(hostile_subject, BTreeSet::from([faction]));
        world
            .relations
            .hostility_from
            .insert(faction, BTreeSet::from([hostile_subject]));
        world.set_owner(item, faction).unwrap();
        world.relations.office_holder.insert(office, faction);
        world
            .relations
            .offices_held
            .insert(faction, BTreeSet::from([office]));
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
        assert_eq!(
            world.get_component_name(id),
            Some(&Name("Aster".to_string()))
        );
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
        let id = world
            .create_agent("Bram", ControlSource::Ai, Tick(3))
            .unwrap();

        assert_eq!(world.entities_with_name().collect::<Vec<_>>(), vec![id]);
        assert_eq!(
            world.entities_with_agent_data().collect::<Vec<_>>(),
            vec![id]
        );
        assert_eq!(
            world
                .entities_with_name_and_agent_data()
                .collect::<Vec<_>>(),
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
    fn create_item_lot_produces_correct_entity() {
        let mut world = World::new(Topology::new()).unwrap();

        let id = world
            .create_item_lot(CommodityKind::Apple, Quantity(10), Tick(5))
            .unwrap();

        assert!(world.is_alive(id));
        assert_eq!(world.entity_kind(id), Some(EntityKind::ItemLot));
        assert_eq!(
            world.get_component_item_lot(id),
            Some(&ItemLot {
                commodity: CommodityKind::Apple,
                quantity: Quantity(10),
                provenance: vec![ProvenanceEntry {
                    tick: Tick(5),
                    event_id: None,
                    operation: LotOperation::Created,
                    related_lot: None,
                    amount: Quantity(10),
                }],
            })
        );
    }

    #[test]
    fn create_item_lot_rejects_zero_quantity() {
        let mut world = World::new(Topology::new()).unwrap();

        let err = world
            .create_item_lot(CommodityKind::Water, Quantity(0), Tick(8))
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn split_lot_creates_child_and_preserves_total_quantity() {
        let mut world = World::new(Topology::new()).unwrap();
        let source = world
            .create_item_lot(CommodityKind::Apple, Quantity(10), Tick(1))
            .unwrap();

        let (returned_source, split_off) = world
            .split_lot(source, Quantity(3), Tick(2), Some(EventId(9)))
            .unwrap();

        assert_eq!(returned_source, source);
        assert!(world.is_alive(source));
        assert!(world.is_alive(split_off));

        let source_lot = world.get_component_item_lot(source).unwrap();
        let split_off_lot = world.get_component_item_lot(split_off).unwrap();

        assert_eq!(source_lot.commodity, CommodityKind::Apple);
        assert_eq!(split_off_lot.commodity, CommodityKind::Apple);
        assert_eq!(source_lot.quantity, Quantity(7));
        assert_eq!(split_off_lot.quantity, Quantity(3));
        assert_eq!(source_lot.quantity + split_off_lot.quantity, Quantity(10));

        assert_eq!(
            source_lot.provenance,
            vec![
                ProvenanceEntry {
                    tick: Tick(1),
                    event_id: None,
                    operation: LotOperation::Created,
                    related_lot: None,
                    amount: Quantity(10),
                },
                ProvenanceEntry {
                    tick: Tick(2),
                    event_id: Some(EventId(9)),
                    operation: LotOperation::Split,
                    related_lot: Some(split_off),
                    amount: Quantity(3),
                },
            ]
        );
        assert_eq!(
            split_off_lot.provenance,
            vec![
                ProvenanceEntry {
                    tick: Tick(2),
                    event_id: None,
                    operation: LotOperation::Created,
                    related_lot: None,
                    amount: Quantity(3),
                },
                ProvenanceEntry {
                    tick: Tick(2),
                    event_id: Some(EventId(9)),
                    operation: LotOperation::Split,
                    related_lot: Some(source),
                    amount: Quantity(3),
                },
            ]
        );
    }

    #[test]
    fn split_lot_rejects_zero_full_and_excessive_amounts() {
        let mut world = World::new(Topology::new()).unwrap();
        let source = world
            .create_item_lot(CommodityKind::Grain, Quantity(10), Tick(1))
            .unwrap();

        let zero = world
            .split_lot(source, Quantity(0), Tick(2), None)
            .unwrap_err();
        assert!(matches!(zero, WorldError::InvalidOperation(_)));

        let full = world
            .split_lot(source, Quantity(10), Tick(2), None)
            .unwrap_err();
        assert!(matches!(
            full,
            WorldError::InsufficientQuantity {
                entity,
                requested: 10,
                available: 10,
            } if entity == source
        ));

        let excessive = world
            .split_lot(source, Quantity(11), Tick(2), None)
            .unwrap_err();
        assert!(matches!(
            excessive,
            WorldError::InsufficientQuantity {
                entity,
                requested: 11,
                available: 10,
            } if entity == source
        ));
    }

    #[test]
    fn split_lot_rejects_non_item_lot_entities() {
        let mut world = World::new(Topology::new()).unwrap();
        let office = world.create_office("Ledger Hall", Tick(1)).unwrap();

        let err = world
            .split_lot(office, Quantity(1), Tick(2), None)
            .unwrap_err();

        assert!(matches!(
            err,
            WorldError::ComponentNotFound {
                entity,
                component_type: "ItemLot",
            } if entity == office
        ));
    }

    #[test]
    fn merge_lots_combines_quantity_archives_source_and_preserves_traceability() {
        let mut world = World::new(Topology::new()).unwrap();
        let source = world
            .create_item_lot(CommodityKind::Water, Quantity(10), Tick(1))
            .unwrap();
        let (_, split_off) = world
            .split_lot(source, Quantity(4), Tick(2), Some(EventId(3)))
            .unwrap();

        let merged = world
            .merge_lots(source, split_off, Tick(3), Some(EventId(4)))
            .unwrap();

        assert_eq!(merged, source);
        assert!(world.is_alive(source));
        assert!(world.is_archived(split_off));
        assert_eq!(world.get_component_item_lot(split_off), None);

        let merged_lot = world.get_component_item_lot(source).unwrap();
        assert_eq!(merged_lot.quantity, Quantity(10));
        assert_eq!(
            merged_lot.provenance,
            vec![
                ProvenanceEntry {
                    tick: Tick(1),
                    event_id: None,
                    operation: LotOperation::Created,
                    related_lot: None,
                    amount: Quantity(10),
                },
                ProvenanceEntry {
                    tick: Tick(2),
                    event_id: Some(EventId(3)),
                    operation: LotOperation::Split,
                    related_lot: Some(split_off),
                    amount: Quantity(4),
                },
                ProvenanceEntry {
                    tick: Tick(3),
                    event_id: Some(EventId(4)),
                    operation: LotOperation::Merge,
                    related_lot: Some(split_off),
                    amount: Quantity(4),
                },
            ]
        );

        let archived_source_lot = world.components.item_lots.get(&split_off).unwrap();
        assert_eq!(
            archived_source_lot.provenance.last(),
            Some(&ProvenanceEntry {
                tick: Tick(3),
                event_id: Some(EventId(4)),
                operation: LotOperation::Merge,
                related_lot: Some(source),
                amount: Quantity(4),
            })
        );
    }

    #[test]
    fn merge_lots_rejects_mismatched_or_identical_lots() {
        let mut world = World::new(Topology::new()).unwrap();
        let apples = world
            .create_item_lot(CommodityKind::Apple, Quantity(4), Tick(1))
            .unwrap();
        let grain = world
            .create_item_lot(CommodityKind::Grain, Quantity(5), Tick(1))
            .unwrap();

        let same_entity = world.merge_lots(apples, apples, Tick(2), None).unwrap_err();
        assert!(matches!(same_entity, WorldError::InvalidOperation(_)));

        let mismatched = world.merge_lots(apples, grain, Tick(2), None).unwrap_err();
        assert!(matches!(mismatched, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn waste_lots_support_split_and_merge() {
        let mut world = World::new(Topology::new()).unwrap();
        let source = world
            .create_item_lot(CommodityKind::Waste, Quantity(6), Tick(1))
            .unwrap();

        let (_, split_off) = world.split_lot(source, Quantity(2), Tick(2), None).unwrap();
        world.merge_lots(source, split_off, Tick(3), None).unwrap();

        assert_eq!(
            world.get_component_item_lot(source).unwrap().quantity,
            Quantity(6)
        );
        assert!(world.is_archived(split_off));
    }

    #[test]
    fn create_unique_item_produces_correct_entity() {
        let mut world = World::new(Topology::new()).unwrap();

        let id = world
            .create_unique_item(
                UniqueItemKind::SimpleTool,
                Some("Hammer"),
                BTreeMap::from([("condition".to_string(), "worn".to_string())]),
                Tick(5),
            )
            .unwrap();

        assert!(world.is_alive(id));
        assert_eq!(world.entity_kind(id), Some(EntityKind::UniqueItem));
        assert_eq!(
            world.get_component_unique_item(id),
            Some(&UniqueItem {
                kind: UniqueItemKind::SimpleTool,
                name: Some("Hammer".to_string()),
                metadata: BTreeMap::from([("condition".to_string(), "worn".to_string())]),
            })
        );
    }

    #[test]
    fn create_container_produces_correct_entity() {
        let mut world = World::new(Topology::new()).unwrap();
        let container = Container {
            capacity: LoadUnits(30),
            allowed_commodities: Some(BTreeSet::from([CommodityKind::Apple, CommodityKind::Bread])),
            allows_unique_items: true,
            allows_nested_containers: false,
        };

        let id = world.create_container(container.clone(), Tick(6)).unwrap();

        assert!(world.is_alive(id));
        assert_eq!(world.entity_kind(id), Some(EntityKind::Container));
        assert_eq!(world.get_component_container(id), Some(&container));
    }

    #[test]
    fn create_container_rejects_zero_capacity() {
        let mut world = World::new(Topology::new()).unwrap();

        let err = world
            .create_container(
                Container {
                    capacity: LoadUnits(0),
                    allowed_commodities: None,
                    allows_unique_items: true,
                    allows_nested_containers: true,
                },
                Tick(7),
            )
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
        assert_eq!(world.entity_count(), 0);
        assert_eq!(world.all_entities().count(), 0);
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
        assert_eq!(
            factory_world.entity_meta(factory_id),
            manual_world.entity_meta(manual_id)
        );
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

        let first = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let second = world
            .create_agent("Bram", ControlSource::Human, Tick(2))
            .unwrap();

        assert_ne!(first, second);
        assert_eq!(
            world
                .entities_of_kind(EntityKind::Agent)
                .collect::<Vec<_>>(),
            vec![first, second]
        );
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
        assert_eq!(world.count_with_item_lot(), 0);
        assert_eq!(world.count_with_container(), 0);
    }

    #[test]
    fn world_bincode_roundtrip_empty() {
        let world = World::new(Topology::new()).unwrap();

        let bytes = bincode::serialize(&world).unwrap();
        let roundtrip: World = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, world);
        assert_eq!(roundtrip.entity_count(), 0);
        assert_eq!(
            roundtrip.entities().collect::<Vec<_>>(),
            Vec::<EntityId>::new()
        );
        assert_eq!(roundtrip.count_with_name(), 0);
        assert_eq!(roundtrip.count_with_agent_data(), 0);
        assert_eq!(roundtrip.count_with_item_lot(), 0);
    }

    #[test]
    fn world_bincode_roundtrip_populated() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(7))
            .unwrap();
        let office = world.create_office("Ledger Hall", Tick(8)).unwrap();
        let faction = world.create_faction("River Pact", Tick(9)).unwrap();
        let lot = world
            .create_item_lot(CommodityKind::Grain, Quantity(6), Tick(10))
            .unwrap();
        let unique_item = world
            .create_unique_item(
                UniqueItemKind::Artifact,
                Some("Court Seal"),
                BTreeMap::from([("origin".to_string(), "vault".to_string())]),
                Tick(11),
            )
            .unwrap();
        let container = world
            .create_container(
                Container {
                    capacity: LoadUnits(20),
                    allowed_commodities: Some(BTreeSet::from([
                        CommodityKind::Coin,
                        CommodityKind::Grain,
                    ])),
                    allows_unique_items: false,
                    allows_nested_containers: false,
                },
                Tick(12),
            )
            .unwrap();

        let bytes = bincode::serialize(&world).unwrap();
        let roundtrip: World = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, world);
        assert_populated_world_roundtrip(
            &roundtrip,
            agent,
            office,
            faction,
            lot,
            unique_item,
            container,
        );
    }

    #[test]
    fn deserialized_world_remains_operational() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Human, Tick(4))
            .unwrap();

        let bytes = bincode::serialize(&world).unwrap();
        let mut roundtrip: World = bincode::deserialize(&bytes).unwrap();

        let office = roundtrip.create_office("Ledger Hall", Tick(5)).unwrap();
        roundtrip
            .get_component_name_mut(agent)
            .unwrap()
            .0
            .push_str(" of the Square");

        assert_eq!(
            roundtrip.get_component_name(agent),
            Some(&Name("Aster of the Square".to_string()))
        );
        assert_eq!(
            roundtrip
                .query_name()
                .map(|(entity, _)| entity)
                .collect::<Vec<_>>(),
            vec![agent, office]
        );
        assert_eq!(
            roundtrip
                .query_name_and_agent_data()
                .map(|(entity, name, data)| (entity, name.0.as_str(), data.control_source))
                .collect::<Vec<_>>(),
            vec![(agent, "Aster of the Square", ControlSource::Human)]
        );
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
    fn archive_entity_cleans_outbound_relation_rows() {
        let mut world = World::new(Topology::new()).unwrap();
        let item = world.create_entity(EntityKind::ItemLot, Tick(1));
        let container = world.create_entity(EntityKind::Container, Tick(1));
        let holder = world.create_entity(EntityKind::Agent, Tick(1));
        let owner = world.create_entity(EntityKind::Faction, Tick(1));
        let reserver = world.create_entity(EntityKind::Agent, Tick(1));
        let faction = world.create_entity(EntityKind::Faction, Tick(1));
        let loyal_target = world.create_entity(EntityKind::Faction, Tick(1));
        let office = world.create_entity(EntityKind::Office, Tick(1));
        let enemy = world.create_entity(EntityKind::Agent, Tick(1));
        let place = entity(22);
        let reservation_id = ReservationId(3);
        let known_fact = FactId(41);
        let believed_fact = FactId(42);
        let fixture = PurgeRelationFixture {
            item,
            container,
            holder,
            owner,
            reserver,
            faction,
            loyal_target,
            office,
            enemy,
            place,
            reservation_id,
            known_fact,
            believed_fact,
        };

        populate_relation_rows_for_purge_test(&mut world, &fixture);
        world.relations.office_holder.remove(&office);
        world.relations.offices_held.remove(&item);

        world.archive_entity(item, Tick(2)).unwrap();

        assert_eq!(world.relations.located_in.get(&item), None);
        assert_eq!(world.relations.contained_by.get(&item), None);
        assert_eq!(world.relations.possessed_by.get(&item), None);
        assert_eq!(world.relations.owned_by.get(&item), None);
        assert_eq!(world.relations.member_of.get(&item), None);
        assert_eq!(world.relations.loyal_to.get(&item), None);
        assert_eq!(world.relations.hostile_to.get(&item), None);
        assert_eq!(world.relations.knows_fact.get(&item), None);
        assert_eq!(world.relations.believes_fact.get(&item), None);
        assert_eq!(world.relations.reservations.get(&reservation_id), None);
        assert_eq!(world.relations.reservations_by_entity.get(&item), None);
        assert_eq!(world.relations.entities_at.get(&place), None);
        assert_eq!(world.relations.contents_of.get(&container), None);
        assert_eq!(world.relations.possessions_of.get(&holder), None);
        assert_eq!(world.relations.property_of.get(&owner), None);
        assert_eq!(world.relations.members_of.get(&faction), None);
        assert_eq!(world.relations.loyalty_from.get(&loyal_target), None);
        assert_eq!(world.relations.hostility_from.get(&enemy), None);
    }

    #[test]
    fn archive_entity_rejects_entities_that_still_anchor_live_dependents() {
        let mut world = World::new(test_topology()).unwrap();
        let container = world.create_container(open_container(20), Tick(1)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Medicine, Quantity(1), Tick(2))
            .unwrap();
        let place = entity(5);

        world.set_ground_location(container, place).unwrap();
        world.put_into_container(item, container).unwrap();

        let err = world.archive_entity(container, Tick(3)).unwrap_err();

        assert!(matches!(err, WorldError::PreconditionFailed(_)));
        assert_eq!(world.relations.contained_by.get(&item), Some(&container));
        assert_eq!(
            world.relations.contents_of.get(&container),
            Some(&BTreeSet::from([item]))
        );
        assert_eq!(world.relations.located_in.get(&container), Some(&place));
        assert_eq!(
            world.relations.entities_at.get(&place),
            Some(&BTreeSet::from([container, item]))
        );
    }

    #[test]
    fn archive_entity_rejects_live_owners_and_holders_until_relations_are_cleared() {
        let mut world = World::new(Topology::new()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Coin, Quantity(1), Tick(1))
            .unwrap();
        let owner = world.create_faction("River Pact", Tick(2)).unwrap();
        let holder = world
            .create_agent("Aster", ControlSource::Ai, Tick(3))
            .unwrap();

        world.set_owner(item, owner).unwrap();
        world.set_possessor(item, holder).unwrap();

        let owner_err = world.archive_entity(owner, Tick(4)).unwrap_err();
        let holder_err = world.archive_entity(holder, Tick(5)).unwrap_err();

        assert!(matches!(owner_err, WorldError::PreconditionFailed(_)));
        assert!(matches!(holder_err, WorldError::PreconditionFailed(_)));
        assert_eq!(world.owner_of(item), Some(owner));
        assert_eq!(world.possessor_of(item), Some(holder));
    }

    #[test]
    fn archive_dependencies_reports_blockers_deterministically() {
        let mut world = World::new(Topology::new()).unwrap();
        let owner = world.create_faction("River Pact", Tick(1)).unwrap();
        let holder = world
            .create_agent("Aster", ControlSource::Ai, Tick(2))
            .unwrap();
        let office_a = world.create_office("Granary Chair", Tick(3)).unwrap();
        let office_b = world.create_office("Market Chair", Tick(4)).unwrap();
        let item_a = world
            .create_item_lot(CommodityKind::Coin, Quantity(1), Tick(5))
            .unwrap();
        let item_b = world
            .create_item_lot(CommodityKind::Bread, Quantity(1), Tick(6))
            .unwrap();

        world.set_owner(item_b, owner).unwrap();
        world.set_possessor(item_a, holder).unwrap();
        world
            .relations
            .offices_held
            .insert(owner, BTreeSet::from([office_a, office_b]));

        assert_eq!(
            world.archive_dependencies(owner).unwrap(),
            vec![
                crate::relations::ArchiveDependency {
                    kind: crate::relations::ArchiveDependencyKind::OwnsEntities,
                    dependents: vec![item_b],
                },
                crate::relations::ArchiveDependency {
                    kind: crate::relations::ArchiveDependencyKind::HoldsOffices,
                    dependents: vec![office_a, office_b],
                },
            ]
        );
        assert_eq!(
            world.archive_dependencies(holder).unwrap(),
            vec![crate::relations::ArchiveDependency {
                kind: crate::relations::ArchiveDependencyKind::PossessesEntities,
                dependents: vec![item_a],
            }]
        );
    }

    #[test]
    fn plan_entity_archive_preparation_reports_actions_without_mutating_state() {
        let mut world = World::new(test_topology()).unwrap();
        let container = world.create_container(open_container(20), Tick(1)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Bread, Quantity(1), Tick(2))
            .unwrap();
        let held_item = world
            .create_item_lot(CommodityKind::Coin, Quantity(1), Tick(3))
            .unwrap();
        let place = entity(5);

        world.set_ground_location(container, place).unwrap();
        world.put_into_container(item, container).unwrap();
        world.set_possessor(held_item, container).unwrap();

        let policy = crate::ArchivePreparationPolicy::with_resolutions([(
            crate::ArchiveDependencyKind::ContainsEntities,
            crate::ArchiveResolution::DetachContentsToGround,
        )]);

        let plan = world
            .plan_entity_archive_preparation_with_policy(container, &policy)
            .unwrap();
        assert!(!plan.is_ready_for_archive());
        assert_eq!(
            plan,
            crate::ArchivePreparationPlan {
                actions: vec![crate::ArchivePreparationAction {
                    dependency: crate::ArchiveDependency {
                        kind: crate::ArchiveDependencyKind::ContainsEntities,
                        dependents: vec![item],
                    },
                    resolution: crate::ArchiveResolution::DetachContentsToGround,
                }],
                blocked: vec![crate::ArchiveDependency {
                    kind: crate::ArchiveDependencyKind::PossessesEntities,
                    dependents: vec![held_item],
                }],
            }
        );

        assert_eq!(world.direct_container(item), Some(container));
        assert_eq!(world.possessor_of(held_item), Some(container));
    }

    #[test]
    fn prepare_entity_for_archive_clears_container_blockers_and_allows_archive() {
        let mut world = World::new(test_topology()).unwrap();
        let container = world.create_container(open_container(20), Tick(1)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Bread, Quantity(1), Tick(2))
            .unwrap();
        let place = entity(5);

        world.set_ground_location(container, place).unwrap();
        world.put_into_container(item, container).unwrap();

        let report = world.prepare_entity_for_archive(container).unwrap();
        assert!(report.is_ready_for_archive());
        assert_eq!(
            report,
            crate::ArchivePreparationReport {
                applied: vec![crate::ArchivePreparationAction {
                    dependency: crate::ArchiveDependency {
                        kind: crate::ArchiveDependencyKind::ContainsEntities,
                        dependents: vec![item],
                    },
                    resolution: crate::ArchiveResolution::DetachContentsToGround,
                }],
                blocked: vec![],
            }
        );
        assert_eq!(world.direct_container(item), None);
        assert_eq!(world.effective_place(item), Some(place));
        assert_eq!(world.archive_dependencies(container).unwrap(), Vec::new());

        world.archive_entity(container, Tick(3)).unwrap();

        assert!(world.is_archived(container));
    }

    #[test]
    fn prepare_entity_for_archive_clears_social_property_and_office_blockers() {
        let mut world = World::new(Topology::new()).unwrap();
        let faction = world.create_faction("River Pact", Tick(1)).unwrap();
        let member = world
            .create_agent("Aster", ControlSource::Ai, Tick(2))
            .unwrap();
        let loyal_subject = world
            .create_agent("Bram", ControlSource::Ai, Tick(3))
            .unwrap();
        let hostile_subject = world
            .create_agent("Cato", ControlSource::Ai, Tick(4))
            .unwrap();
        let office = world.create_office("Granary Chair", Tick(5)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Coin, Quantity(1), Tick(6))
            .unwrap();

        seed_social_archive_blockers(
            &mut world,
            faction,
            member,
            loyal_subject,
            hostile_subject,
            office,
            item,
        );

        assert_eq!(
            world.prepare_entity_for_archive(faction).unwrap(),
            crate::ArchivePreparationReport {
                applied: vec![
                    crate::ArchivePreparationAction {
                        dependency: crate::ArchiveDependency {
                            kind: crate::ArchiveDependencyKind::OwnsEntities,
                            dependents: vec![item],
                        },
                        resolution: crate::ArchiveResolution::RelinquishOwnership,
                    },
                    crate::ArchivePreparationAction {
                        dependency: crate::ArchiveDependency {
                            kind: crate::ArchiveDependencyKind::HasMembers,
                            dependents: vec![member],
                        },
                        resolution: crate::ArchiveResolution::RevokeMemberships,
                    },
                    crate::ArchivePreparationAction {
                        dependency: crate::ArchiveDependency {
                            kind: crate::ArchiveDependencyKind::HasLoyalSubjects,
                            dependents: vec![loyal_subject],
                        },
                        resolution: crate::ArchiveResolution::RevokeLoyalty,
                    },
                    crate::ArchivePreparationAction {
                        dependency: crate::ArchiveDependency {
                            kind: crate::ArchiveDependencyKind::HasHostileSubjects,
                            dependents: vec![hostile_subject],
                        },
                        resolution: crate::ArchiveResolution::RevokeHostility,
                    },
                    crate::ArchivePreparationAction {
                        dependency: crate::ArchiveDependency {
                            kind: crate::ArchiveDependencyKind::HoldsOffices,
                            dependents: vec![office],
                        },
                        resolution: crate::ArchiveResolution::RelinquishOffices,
                    },
                ],
                blocked: vec![],
            }
        );

        assert_eq!(world.owner_of(item), None);
        assert_eq!(world.relations.member_of.get(&member), None);
        assert_eq!(world.relations.loyal_to.get(&loyal_subject), None);
        assert_eq!(world.relations.hostile_to.get(&hostile_subject), None);
        assert_eq!(world.relations.office_holder.get(&office), None);
        assert_eq!(world.archive_dependencies(faction).unwrap(), Vec::new());

        world.archive_entity(faction, Tick(7)).unwrap();

        assert!(world.is_archived(faction));
    }

    #[test]
    fn prepare_entity_for_archive_with_policy_leaves_disallowed_blockers_intact() {
        let mut world = World::new(test_topology()).unwrap();
        let container = world.create_container(open_container(20), Tick(1)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Bread, Quantity(1), Tick(2))
            .unwrap();
        let _holder = world
            .create_agent("Aster", ControlSource::Ai, Tick(3))
            .unwrap();
        let held_item = world
            .create_item_lot(CommodityKind::Coin, Quantity(1), Tick(4))
            .unwrap();
        let place = entity(5);

        world.set_ground_location(container, place).unwrap();
        world.put_into_container(item, container).unwrap();
        world.set_possessor(held_item, container).unwrap();

        let policy = crate::ArchivePreparationPolicy::with_resolutions([(
            crate::ArchiveDependencyKind::ContainsEntities,
            crate::ArchiveResolution::DetachContentsToGround,
        )]);
        assert_eq!(
            world
                .prepare_entity_for_archive_with_policy(container, &policy)
                .unwrap(),
            crate::ArchivePreparationReport {
                applied: vec![crate::ArchivePreparationAction {
                    dependency: crate::ArchiveDependency {
                        kind: crate::ArchiveDependencyKind::ContainsEntities,
                        dependents: vec![item],
                    },
                    resolution: crate::ArchiveResolution::DetachContentsToGround,
                }],
                blocked: vec![crate::ArchiveDependency {
                    kind: crate::ArchiveDependencyKind::PossessesEntities,
                    dependents: vec![held_item],
                }],
            }
        );

        assert_eq!(world.direct_container(item), None);
        assert_eq!(world.possessor_of(held_item), Some(container));
        assert_eq!(
            world.archive_dependencies(container).unwrap(),
            vec![crate::ArchiveDependency {
                kind: crate::ArchiveDependencyKind::PossessesEntities,
                dependents: vec![held_item],
            }]
        );
    }

    #[test]
    fn prepare_entity_for_archive_with_invalid_resolution_errors() {
        let mut world = World::new(test_topology()).unwrap();
        let container = world.create_container(open_container(20), Tick(1)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Bread, Quantity(1), Tick(2))
            .unwrap();
        let place = entity(5);

        world.set_ground_location(container, place).unwrap();
        world.put_into_container(item, container).unwrap();

        let policy = crate::ArchivePreparationPolicy::with_resolutions([(
            crate::ArchiveDependencyKind::ContainsEntities,
            crate::ArchiveResolution::RelinquishOwnership,
        )]);

        assert!(matches!(
            world.plan_entity_archive_preparation_with_policy(container, &policy),
            Err(WorldError::InvalidOperation(_))
        ));
        assert!(matches!(
            world.prepare_entity_for_archive_with_policy(container, &policy),
            Err(WorldError::InvalidOperation(_))
        ));
        assert_eq!(world.direct_container(item), Some(container));
    }

    #[test]
    fn prepare_entity_for_archive_with_recursive_spill_flattens_nested_contents() {
        let mut world = World::new(test_topology()).unwrap();
        let root = world.create_container(open_container(20), Tick(1)).unwrap();
        let inner = world.create_container(open_container(10), Tick(2)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Bread, Quantity(1), Tick(3))
            .unwrap();
        let place = entity(5);

        world.set_ground_location(root, place).unwrap();
        world.put_into_container(inner, root).unwrap();
        world.put_into_container(item, inner).unwrap();

        let policy = crate::ArchivePreparationPolicy::with_resolutions([(
            crate::ArchiveDependencyKind::ContainsEntities,
            crate::ArchiveResolution::SpillContentsRecursively,
        )]);

        assert_eq!(
            world
                .prepare_entity_for_archive_with_policy(root, &policy)
                .unwrap(),
            crate::ArchivePreparationReport {
                applied: vec![crate::ArchivePreparationAction {
                    dependency: crate::ArchiveDependency {
                        kind: crate::ArchiveDependencyKind::ContainsEntities,
                        dependents: vec![inner],
                    },
                    resolution: crate::ArchiveResolution::SpillContentsRecursively,
                }],
                blocked: vec![],
            }
        );

        assert_eq!(world.direct_container(inner), None);
        assert_eq!(world.direct_container(item), None);
        assert_eq!(world.effective_place(inner), Some(place));
        assert_eq!(world.effective_place(item), Some(place));
        assert_eq!(world.ground_entities_at(place), vec![root, inner, item]);
    }

    #[test]
    fn prepare_entity_for_archive_can_transfer_ownership_and_possessions() {
        let mut world = World::new(Topology::new()).unwrap();
        let current_owner = world.create_faction("River Pact", Tick(1)).unwrap();
        let successor_owner = world.create_faction("Granary Guild", Tick(2)).unwrap();
        let current_holder = world
            .create_agent("Aster", ControlSource::Ai, Tick(3))
            .unwrap();
        let successor_holder = world
            .create_agent("Bram", ControlSource::Ai, Tick(4))
            .unwrap();
        let owned_item = world
            .create_item_lot(CommodityKind::Coin, Quantity(1), Tick(5))
            .unwrap();
        let held_item = world
            .create_item_lot(CommodityKind::Bread, Quantity(1), Tick(6))
            .unwrap();

        world.set_owner(owned_item, current_owner).unwrap();
        world.set_possessor(held_item, current_holder).unwrap();

        let policy = crate::ArchivePreparationPolicy::with_resolutions([
            (
                crate::ArchiveDependencyKind::OwnsEntities,
                crate::ArchiveResolution::TransferOwnershipTo(successor_owner),
            ),
            (
                crate::ArchiveDependencyKind::PossessesEntities,
                crate::ArchiveResolution::TransferPossessionsTo(successor_holder),
            ),
        ]);

        assert_eq!(
            world
                .prepare_entity_for_archive_with_policy(current_owner, &policy)
                .unwrap(),
            crate::ArchivePreparationReport {
                applied: vec![crate::ArchivePreparationAction {
                    dependency: crate::ArchiveDependency {
                        kind: crate::ArchiveDependencyKind::OwnsEntities,
                        dependents: vec![owned_item],
                    },
                    resolution: crate::ArchiveResolution::TransferOwnershipTo(successor_owner),
                }],
                blocked: vec![],
            }
        );
        assert_eq!(world.owner_of(owned_item), Some(successor_owner));

        assert_eq!(
            world
                .prepare_entity_for_archive_with_policy(current_holder, &policy)
                .unwrap(),
            crate::ArchivePreparationReport {
                applied: vec![crate::ArchivePreparationAction {
                    dependency: crate::ArchiveDependency {
                        kind: crate::ArchiveDependencyKind::PossessesEntities,
                        dependents: vec![held_item],
                    },
                    resolution: crate::ArchiveResolution::TransferPossessionsTo(successor_holder),
                }],
                blocked: vec![],
            }
        );
        assert_eq!(world.possessor_of(held_item), Some(successor_holder));
    }

    #[test]
    fn prepare_entity_for_archive_rejects_self_transfer_resolution() {
        let mut world = World::new(Topology::new()).unwrap();
        let owner = world.create_faction("River Pact", Tick(1)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Coin, Quantity(1), Tick(2))
            .unwrap();

        world.set_owner(item, owner).unwrap();

        let policy = crate::ArchivePreparationPolicy::with_resolutions([(
            crate::ArchiveDependencyKind::OwnsEntities,
            crate::ArchiveResolution::TransferOwnershipTo(owner),
        )]);

        assert!(matches!(
            world.prepare_entity_for_archive_with_policy(owner, &policy),
            Err(WorldError::InvalidOperation(_))
        ));
        assert_eq!(world.owner_of(item), Some(owner));
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
        assert_eq!(
            world.all_entities().collect::<Vec<_>>(),
            vec![live, archived]
        );

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

        assert_eq!(
            world
                .entities_of_kind(EntityKind::Agent)
                .collect::<Vec<_>>(),
            vec![agent]
        );
        assert_eq!(
            world
                .entities_of_kind(EntityKind::Place)
                .collect::<Vec<_>>(),
            vec![entity(2), entity(5)]
        );
        assert_eq!(
            world
                .entities_of_kind(EntityKind::Office)
                .collect::<Vec<_>>(),
            vec![office]
        );
    }

    #[test]
    fn query_item_lot_returns_live_entities_only() {
        let mut world = World::new(Topology::new()).unwrap();
        let live = world
            .create_item_lot(CommodityKind::Bread, Quantity(4), Tick(1))
            .unwrap();
        let archived = world
            .create_item_lot(CommodityKind::Bread, Quantity(7), Tick(2))
            .unwrap();

        world.archive_entity(archived, Tick(3)).unwrap();

        assert_eq!(
            world.entities_with_item_lot().collect::<Vec<_>>(),
            vec![live]
        );
        assert_eq!(
            world
                .query_item_lot()
                .map(|(entity, lot)| (entity, lot.quantity))
                .collect::<Vec<_>>(),
            vec![(live, Quantity(4))]
        );
    }

    #[test]
    fn query_unique_item_returns_live_entities_only() {
        let mut world = World::new(Topology::new()).unwrap();
        let live = world
            .create_unique_item(
                UniqueItemKind::SimpleTool,
                Some("Hammer"),
                BTreeMap::new(),
                Tick(1),
            )
            .unwrap();
        let archived = world
            .create_unique_item(
                UniqueItemKind::Contract,
                Some("Charter"),
                BTreeMap::from([("issuer".to_string(), "council".to_string())]),
                Tick(2),
            )
            .unwrap();

        world.archive_entity(archived, Tick(3)).unwrap();

        assert_eq!(
            world.entities_with_unique_item().collect::<Vec<_>>(),
            vec![live]
        );
        assert_eq!(
            world
                .query_unique_item()
                .map(|(entity, item)| (entity, item.kind))
                .collect::<Vec<_>>(),
            vec![(live, UniqueItemKind::SimpleTool)]
        );
    }

    #[test]
    fn query_container_returns_live_entities_only() {
        let mut world = World::new(Topology::new()).unwrap();
        let live = world
            .create_container(
                Container {
                    capacity: LoadUnits(15),
                    allowed_commodities: None,
                    allows_unique_items: true,
                    allows_nested_containers: false,
                },
                Tick(1),
            )
            .unwrap();
        let archived = world
            .create_container(
                Container {
                    capacity: LoadUnits(9),
                    allowed_commodities: Some(BTreeSet::from([CommodityKind::Water])),
                    allows_unique_items: false,
                    allows_nested_containers: true,
                },
                Tick(2),
            )
            .unwrap();

        world.archive_entity(archived, Tick(3)).unwrap();

        assert_eq!(
            world.entities_with_container().collect::<Vec<_>>(),
            vec![live]
        );
        assert_eq!(
            world
                .query_container()
                .map(|(entity, container)| (entity, container.capacity))
                .collect::<Vec<_>>(),
            vec![(live, LoadUnits(15))]
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
    fn purge_cleans_item_lot_components() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world
            .create_item_lot(CommodityKind::Coin, Quantity(3), Tick(1))
            .unwrap();

        world.archive_entity(id, Tick(2)).unwrap();
        world.purge_entity(id).unwrap();

        assert_eq!(world.entity_meta(id), None);
        assert_eq!(world.get_component_item_lot(id), None);
        assert_eq!(world.query_item_lot().count(), 0);
    }

    #[test]
    fn purge_cleans_unique_item_components() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world
            .create_unique_item(
                UniqueItemKind::Artifact,
                Some("Seal"),
                BTreeMap::new(),
                Tick(1),
            )
            .unwrap();

        world.archive_entity(id, Tick(2)).unwrap();
        world.purge_entity(id).unwrap();

        assert_eq!(world.entity_meta(id), None);
        assert_eq!(world.get_component_unique_item(id), None);
        assert_eq!(world.query_unique_item().count(), 0);
    }

    #[test]
    fn purge_cleans_container_components() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world
            .create_container(
                Container {
                    capacity: LoadUnits(18),
                    allowed_commodities: Some(BTreeSet::from([CommodityKind::Medicine])),
                    allows_unique_items: true,
                    allows_nested_containers: false,
                },
                Tick(1),
            )
            .unwrap();

        world.archive_entity(id, Tick(2)).unwrap();
        world.purge_entity(id).unwrap();

        assert_eq!(world.entity_meta(id), None);
        assert_eq!(world.get_component_container(id), None);
        assert_eq!(world.query_container().count(), 0);
    }

    #[test]
    fn purge_cleans_relation_rows() {
        let mut world = World::new(Topology::new()).unwrap();
        let item = world.create_entity(EntityKind::ItemLot, Tick(1));
        let container = world.create_entity(EntityKind::Container, Tick(1));
        let holder = world.create_entity(EntityKind::Agent, Tick(1));
        let owner = world.create_entity(EntityKind::Faction, Tick(1));
        let reserver = world.create_entity(EntityKind::Agent, Tick(1));
        let faction = world.create_entity(EntityKind::Faction, Tick(1));
        let loyal_target = world.create_entity(EntityKind::Faction, Tick(1));
        let office = world.create_entity(EntityKind::Office, Tick(1));
        let enemy = world.create_entity(EntityKind::Agent, Tick(1));
        let place = entity(22);
        let reservation_id = ReservationId(3);
        let known_fact = FactId(41);
        let believed_fact = FactId(42);
        let fixture = PurgeRelationFixture {
            item,
            container,
            holder,
            owner,
            reserver,
            faction,
            loyal_target,
            office,
            enemy,
            place,
            reservation_id,
            known_fact,
            believed_fact,
        };

        populate_relation_rows_for_purge_test(&mut world, &fixture);

        world.allocator.archive_entity(item, Tick(2)).unwrap();
        world.purge_entity(item).unwrap();

        assert_eq!(world.entity_meta(item), None);
        assert_eq!(world.relations.located_in.get(&item), None);
        assert_eq!(world.relations.contained_by.get(&item), None);
        assert_eq!(world.relations.possessed_by.get(&item), None);
        assert_eq!(world.relations.owned_by.get(&item), None);
        assert_eq!(world.relations.member_of.get(&item), None);
        assert_eq!(world.relations.loyal_to.get(&item), None);
        assert_eq!(world.relations.office_holder.get(&office), None);
        assert_eq!(world.relations.offices_held.get(&item), None);
        assert_eq!(world.relations.hostile_to.get(&item), None);
        assert_eq!(world.relations.knows_fact.get(&item), None);
        assert_eq!(world.relations.believes_fact.get(&item), None);
        assert_eq!(world.relations.reservations.get(&reservation_id), None);
        assert_eq!(world.relations.reservations_by_entity.get(&item), None);
        assert_eq!(world.relations.entities_at.get(&place), None);
        assert_eq!(world.relations.contents_of.get(&container), None);
        assert_eq!(world.relations.possessions_of.get(&holder), None);
        assert_eq!(world.relations.property_of.get(&owner), None);
        assert_eq!(world.relations.members_of.get(&faction), None);
        assert_eq!(world.relations.loyalty_from.get(&loyal_target), None);
        assert_eq!(world.relations.hostility_from.get(&enemy), None);
    }

    #[test]
    fn set_ground_location_places_entity_and_moves_it_between_places() {
        let mut world = World::new(test_topology()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Apple, Quantity(2), Tick(1))
            .unwrap();
        let square = entity(5);
        let farm = entity(2);

        assert!(world.is_in_transit(item));
        world.set_ground_location(item, square).unwrap();
        assert!(!world.is_in_transit(item));
        assert_eq!(world.relations.located_in.get(&item), Some(&square));
        assert_eq!(
            world.relations.entities_at.get(&square),
            Some(&BTreeSet::from([item]))
        );

        world.set_ground_location(item, farm).unwrap();
        assert_eq!(world.relations.located_in.get(&item), Some(&farm));
        assert_eq!(world.relations.entities_at.get(&square), None);
        assert_eq!(
            world.relations.entities_at.get(&farm),
            Some(&BTreeSet::from([item]))
        );
    }

    #[test]
    fn set_ground_location_rejects_non_place_targets() {
        let mut world = World::new(test_topology()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Apple, Quantity(1), Tick(1))
            .unwrap();
        let non_place = world
            .create_agent("Aster", ControlSource::Ai, Tick(2))
            .unwrap();

        let err = world.set_ground_location(item, non_place).unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn set_ground_location_on_container_updates_descendant_locations() {
        let mut world = World::new(test_topology()).unwrap();
        let outer = world.create_container(open_container(20), Tick(1)).unwrap();
        let inner = world.create_container(open_container(10), Tick(2)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Bread, Quantity(3), Tick(3))
            .unwrap();
        let square = entity(5);
        let farm = entity(2);

        world.set_ground_location(outer, square).unwrap();
        world.put_into_container(inner, outer).unwrap();
        world.put_into_container(item, inner).unwrap();

        world.set_ground_location(outer, farm).unwrap();

        assert!(!world.is_in_transit(outer));
        assert!(!world.is_in_transit(inner));
        assert!(!world.is_in_transit(item));
        assert_eq!(world.relations.located_in.get(&outer), Some(&farm));
        assert_eq!(world.relations.located_in.get(&inner), Some(&farm));
        assert_eq!(world.relations.located_in.get(&item), Some(&farm));
    }

    #[test]
    fn put_into_container_sets_containment_and_inherited_place() {
        let mut world = World::new(test_topology()).unwrap();
        let container = world.create_container(open_container(20), Tick(1)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Water, Quantity(2), Tick(2))
            .unwrap();
        let place = entity(5);

        world.set_ground_location(container, place).unwrap();
        world.put_into_container(item, container).unwrap();

        assert!(!world.is_in_transit(item));
        assert_eq!(world.relations.contained_by.get(&item), Some(&container));
        assert_eq!(
            world.relations.contents_of.get(&container),
            Some(&BTreeSet::from([item]))
        );
        assert_eq!(world.relations.located_in.get(&item), Some(&place));
        assert_eq!(
            world.relations.entities_at.get(&place),
            Some(&BTreeSet::from([container, item]))
        );
    }

    #[test]
    fn put_into_container_rejects_self_containment_and_cycles() {
        let mut world = World::new(test_topology()).unwrap();
        let outer = world.create_container(open_container(20), Tick(1)).unwrap();
        let inner = world.create_container(open_container(10), Tick(2)).unwrap();
        let place = entity(5);

        world.set_ground_location(outer, place).unwrap();
        world.put_into_container(inner, outer).unwrap();

        let self_err = world.put_into_container(outer, outer).unwrap_err();
        assert!(matches!(
            self_err,
            WorldError::ContainmentCycle { entity, container }
            if entity == outer && container == outer
        ));

        let cycle_err = world.put_into_container(outer, inner).unwrap_err();
        assert!(matches!(
            cycle_err,
            WorldError::ContainmentCycle { entity, container }
            if entity == outer && container == inner
        ));
    }

    #[test]
    fn put_into_container_rejects_non_container_targets() {
        let mut world = World::new(test_topology()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Grain, Quantity(1), Tick(1))
            .unwrap();
        let office = world.create_office("Ledger Hall", Tick(2)).unwrap();

        let err = world.put_into_container(item, office).unwrap_err();

        assert!(matches!(
            err,
            WorldError::ComponentNotFound {
                entity,
                component_type: "Container",
            } if entity == office
        ));
    }

    #[test]
    fn put_into_container_rejects_unplaced_container_targets() {
        let mut world = World::new(test_topology()).unwrap();
        let container = world.create_container(open_container(20), Tick(1)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Apple, Quantity(1), Tick(2))
            .unwrap();

        assert!(world.is_in_transit(container));
        assert!(world.is_in_transit(item));
        let err = world.put_into_container(item, container).unwrap_err();

        assert!(matches!(err, WorldError::PreconditionFailed(_)));
    }

    #[test]
    fn put_into_container_rejects_policy_and_capacity_violations() {
        let mut world = World::new(test_topology()).unwrap();
        let grain_only = world
            .create_container(
                Container {
                    capacity: LoadUnits(6),
                    allowed_commodities: Some(BTreeSet::from([CommodityKind::Grain])),
                    allows_unique_items: false,
                    allows_nested_containers: false,
                },
                Tick(1),
            )
            .unwrap();
        let place = entity(5);
        let apples = world
            .create_item_lot(CommodityKind::Apple, Quantity(1), Tick(2))
            .unwrap();
        let contract = world
            .create_unique_item(
                UniqueItemKind::Contract,
                Some("Lease"),
                BTreeMap::new(),
                Tick(3),
            )
            .unwrap();
        let nested = world.create_container(open_container(3), Tick(4)).unwrap();
        let heavy = world
            .create_item_lot(CommodityKind::Grain, Quantity(7), Tick(5))
            .unwrap();

        world.set_ground_location(grain_only, place).unwrap();

        assert!(matches!(
            world.put_into_container(apples, grain_only),
            Err(WorldError::InvalidOperation(_))
        ));
        assert!(matches!(
            world.put_into_container(contract, grain_only),
            Err(WorldError::InvalidOperation(_))
        ));
        assert!(matches!(
            world.put_into_container(nested, grain_only),
            Err(WorldError::InvalidOperation(_))
        ));
        assert!(matches!(
            world.put_into_container(heavy, grain_only),
            Err(WorldError::CapacityExceeded {
                container,
                requested: 7,
                remaining: 6,
            }) if container == grain_only
        ));
    }

    #[test]
    fn put_into_container_on_container_updates_descendant_locations() {
        let mut world = World::new(test_topology()).unwrap();
        let outer = world.create_container(open_container(20), Tick(1)).unwrap();
        let inner = world.create_container(open_container(10), Tick(2)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Medicine, Quantity(2), Tick(3))
            .unwrap();
        let square = entity(5);
        let farm = entity(2);

        world.set_ground_location(outer, square).unwrap();
        world.set_ground_location(inner, farm).unwrap();
        world.put_into_container(item, inner).unwrap();

        world.put_into_container(inner, outer).unwrap();

        assert!(!world.is_in_transit(inner));
        assert!(!world.is_in_transit(item));
        assert_eq!(world.relations.contained_by.get(&inner), Some(&outer));
        assert_eq!(world.relations.located_in.get(&inner), Some(&square));
        assert_eq!(world.relations.located_in.get(&item), Some(&square));
    }

    #[test]
    fn remove_from_container_clears_parent_but_keeps_effective_place() {
        let mut world = World::new(test_topology()).unwrap();
        let container = world.create_container(open_container(20), Tick(1)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Coin, Quantity(5), Tick(2))
            .unwrap();
        let place = entity(5);

        world.set_ground_location(container, place).unwrap();
        world.put_into_container(item, container).unwrap();

        world.remove_from_container(item).unwrap();

        assert!(!world.is_in_transit(item));
        assert_eq!(world.relations.contained_by.get(&item), None);
        assert_eq!(world.relations.contents_of.get(&container), None);
        assert_eq!(world.relations.located_in.get(&item), Some(&place));
    }

    #[test]
    fn remove_from_container_rejects_entities_not_in_containers() {
        let mut world = World::new(test_topology()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Coin, Quantity(1), Tick(1))
            .unwrap();

        let err = world.remove_from_container(item).unwrap_err();

        assert!(matches!(err, WorldError::PreconditionFailed(_)));
    }

    #[test]
    fn move_container_subtree_updates_recursive_effective_places() {
        let mut world = World::new(test_topology()).unwrap();
        let root = world.create_container(open_container(30), Tick(1)).unwrap();
        let mid = world.create_container(open_container(20), Tick(2)).unwrap();
        let leaf = world.create_container(open_container(10), Tick(3)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Bread, Quantity(4), Tick(4))
            .unwrap();
        let square = entity(5);
        let farm = entity(2);

        world.set_ground_location(root, square).unwrap();
        world.put_into_container(mid, root).unwrap();
        world.put_into_container(leaf, mid).unwrap();
        world.put_into_container(item, leaf).unwrap();

        world.move_container_subtree(root, farm).unwrap();

        for entity in [root, mid, leaf, item] {
            assert!(!world.is_in_transit(entity));
            assert_eq!(world.relations.located_in.get(&entity), Some(&farm));
        }
    }

    #[test]
    fn physical_entities_start_in_explicit_transit_until_placed() {
        let mut world = World::new(test_topology()).unwrap();

        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let lot = world
            .create_item_lot(CommodityKind::Apple, Quantity(1), Tick(2))
            .unwrap();
        let unique = world
            .create_unique_item(
                UniqueItemKind::SimpleTool,
                Some("Hammer"),
                BTreeMap::new(),
                Tick(3),
            )
            .unwrap();
        let container = world.create_container(open_container(10), Tick(4)).unwrap();
        let faction = world.create_faction("River Pact", Tick(5)).unwrap();
        let office = world.create_office("Ledger Hall", Tick(6)).unwrap();

        for entity in [agent, lot, unique, container] {
            assert!(world.is_in_transit(entity));
            assert_eq!(world.effective_place(entity), None);
        }

        for entity in [faction, office] {
            assert!(!world.is_in_transit(entity));
            assert_eq!(world.effective_place(entity), None);
        }
    }

    #[test]
    fn inventory_query_helpers_follow_authoritative_relation_indices() {
        let mut world = World::new(test_topology()).unwrap();
        let root = world.create_container(open_container(30), Tick(1)).unwrap();
        let mid = world.create_container(open_container(20), Tick(2)).unwrap();
        let leaf = world.create_container(open_container(10), Tick(3)).unwrap();
        let nested_item = world
            .create_item_lot(CommodityKind::Bread, Quantity(4), Tick(4))
            .unwrap();
        let ground_item = world
            .create_item_lot(CommodityKind::Coin, Quantity(5), Tick(5))
            .unwrap();
        let loose_item = world
            .create_item_lot(CommodityKind::Apple, Quantity(1), Tick(6))
            .unwrap();
        let square = entity(5);
        let farm = entity(2);

        world.set_ground_location(root, square).unwrap();
        world.put_into_container(mid, root).unwrap();
        world.put_into_container(leaf, mid).unwrap();
        world.put_into_container(nested_item, leaf).unwrap();
        world.set_ground_location(ground_item, square).unwrap();

        assert_eq!(world.effective_place(root), Some(square));
        assert_eq!(world.effective_place(nested_item), Some(square));
        assert_eq!(world.effective_place(loose_item), None);
        assert!(world.is_in_transit(loose_item));

        assert_eq!(world.direct_container(root), None);
        assert_eq!(world.direct_container(nested_item), Some(leaf));
        assert_eq!(world.possessions_of(root), Vec::<EntityId>::new());

        assert_eq!(world.direct_contents_of(root), vec![mid]);
        assert_eq!(world.direct_contents_of(mid), vec![leaf]);
        assert_eq!(world.direct_contents_of(leaf), vec![nested_item]);
        assert_eq!(
            world.direct_contents_of(nested_item),
            Vec::<EntityId>::new()
        );

        assert_eq!(
            world.recursive_contents_of(root),
            vec![mid, leaf, nested_item]
        );
        assert_eq!(world.recursive_contents_of(leaf), vec![nested_item]);
        assert_eq!(
            world.recursive_contents_of(nested_item),
            Vec::<EntityId>::new()
        );

        assert_eq!(
            world.entities_effectively_at(square),
            vec![root, mid, leaf, nested_item, ground_item]
        );
        assert_eq!(world.ground_entities_at(square), vec![root, ground_item]);

        world.move_container_subtree(root, farm).unwrap();

        assert_eq!(world.effective_place(root), Some(farm));
        assert_eq!(world.effective_place(nested_item), Some(farm));
        assert_eq!(
            world.entities_effectively_at(farm),
            vec![root, mid, leaf, nested_item]
        );
        assert_eq!(world.ground_entities_at(farm), vec![root]);
        assert_eq!(world.entities_effectively_at(square), vec![ground_item]);
        assert_eq!(world.ground_entities_at(square), vec![ground_item]);
    }

    #[test]
    fn relation_query_helpers_hide_archived_entities_from_public_results() {
        let mut world = World::new(test_topology()).unwrap();
        let container = world.create_container(open_container(20), Tick(1)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Medicine, Quantity(1), Tick(2))
            .unwrap();
        let other_item = world
            .create_item_lot(CommodityKind::Bread, Quantity(2), Tick(3))
            .unwrap();
        let owner = world.create_faction("River Pact", Tick(4)).unwrap();
        let holder = world
            .create_agent("Aster", ControlSource::Ai, Tick(5))
            .unwrap();
        let place = entity(5);

        world.set_ground_location(container, place).unwrap();
        world.put_into_container(item, container).unwrap();
        world.set_ground_location(other_item, place).unwrap();
        world.set_owner(item, owner).unwrap();
        world.set_possessor(item, holder).unwrap();

        world.archive_entity(item, Tick(6)).unwrap();

        assert_eq!(world.effective_place(item), None);
        assert!(!world.is_in_transit(item));
        assert_eq!(world.direct_container(item), None);
        assert_eq!(world.owner_of(item), None);
        assert_eq!(world.possessor_of(item), None);
        assert_eq!(world.possessions_of(holder), Vec::<EntityId>::new());
        assert_eq!(world.direct_contents_of(container), Vec::<EntityId>::new());
        assert_eq!(
            world.recursive_contents_of(container),
            Vec::<EntityId>::new()
        );
        assert_eq!(
            world.entities_effectively_at(place),
            vec![container, other_item]
        );
        assert_eq!(world.ground_entities_at(place), vec![container, other_item]);

        world.archive_entity(owner, Tick(7)).unwrap();
        world.archive_entity(holder, Tick(8)).unwrap();

        assert_eq!(world.owner_of(item), None);
        assert_eq!(world.possessor_of(item), None);
    }

    #[test]
    fn possessions_and_controlled_commodity_queries_follow_custody_graph() {
        let mut world = World::new(test_topology()).unwrap();
        let holder = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let satchel = world
            .create_container(open_container(100), Tick(2))
            .unwrap();
        let bread = world
            .create_item_lot(CommodityKind::Bread, Quantity(2), Tick(3))
            .unwrap();
        let nested_bread = world
            .create_item_lot(CommodityKind::Bread, Quantity(4), Tick(4))
            .unwrap();
        let nested_water = world
            .create_item_lot(CommodityKind::Water, Quantity(9), Tick(5))
            .unwrap();
        let foreign_bread = world
            .create_item_lot(CommodityKind::Bread, Quantity(8), Tick(6))
            .unwrap();
        let place = entity(5);

        world.set_possessor(bread, holder).unwrap();
        world.set_possessor(satchel, holder).unwrap();
        world.set_ground_location(satchel, place).unwrap();
        world.put_into_container(nested_bread, satchel).unwrap();
        world.put_into_container(nested_water, satchel).unwrap();

        assert_eq!(world.possessions_of(holder), vec![satchel, bread]);
        assert_eq!(
            world.controlled_commodity_quantity(holder, CommodityKind::Bread),
            Quantity(6)
        );
        assert_eq!(
            world.controlled_commodity_quantity(holder, CommodityKind::Water),
            Quantity(9)
        );
        assert_eq!(
            world.controlled_commodity_quantity(holder, CommodityKind::Coin),
            Quantity(0)
        );
        assert_eq!(
            world.controlled_commodity_quantity(foreign_bread, CommodityKind::Bread),
            Quantity(8)
        );

        world.archive_entity(bread, Tick(7)).unwrap();

        assert_eq!(world.possessions_of(holder), vec![satchel]);
        assert_eq!(
            world.controlled_commodity_quantity(holder, CommodityKind::Bread),
            Quantity(4)
        );
    }

    #[test]
    fn try_reserve_assigns_monotonic_ids_and_lists_in_id_order() {
        let mut world = World::new(Topology::new()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Medicine, Quantity(1), Tick(1))
            .unwrap();
        let reserver = world
            .create_agent("Aster", ControlSource::Ai, Tick(2))
            .unwrap();

        let first = world
            .try_reserve(item, reserver, TickRange::new(Tick(1), Tick(3)).unwrap())
            .unwrap();
        let second = world
            .try_reserve(item, reserver, TickRange::new(Tick(3), Tick(5)).unwrap())
            .unwrap();

        assert_eq!(first, ReservationId(0));
        assert_eq!(second, ReservationId(1));
        assert_eq!(world.relations.next_reservation_id, 2);
        assert_eq!(
            world.relations.reservations_by_entity.get(&item),
            Some(&BTreeSet::from([first, second]))
        );
        assert_eq!(
            world.reservations_for(item),
            vec![
                ReservationRecord {
                    id: first,
                    entity: item,
                    reserver,
                    range: TickRange::new(Tick(1), Tick(3)).unwrap(),
                },
                ReservationRecord {
                    id: second,
                    entity: item,
                    reserver,
                    range: TickRange::new(Tick(3), Tick(5)).unwrap(),
                },
            ]
        );
    }

    #[test]
    fn try_reserve_rejects_overlaps_and_allows_adjacent_windows() {
        let mut world = World::new(Topology::new()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Medicine, Quantity(1), Tick(1))
            .unwrap();
        let reserver = world
            .create_agent("Aster", ControlSource::Ai, Tick(2))
            .unwrap();

        world
            .try_reserve(item, reserver, TickRange::new(Tick(5), Tick(10)).unwrap())
            .unwrap();

        let conflict = world
            .try_reserve(item, reserver, TickRange::new(Tick(7), Tick(12)).unwrap())
            .unwrap_err();
        assert!(matches!(
            conflict,
            WorldError::ConflictingReservation { entity } if entity == item
        ));

        let adjacent = world
            .try_reserve(item, reserver, TickRange::new(Tick(10), Tick(15)).unwrap())
            .unwrap();
        assert_eq!(adjacent, ReservationId(1));
    }

    #[test]
    fn release_reservation_removes_rows_and_reopens_the_window() {
        let mut world = World::new(Topology::new()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Bread, Quantity(2), Tick(1))
            .unwrap();
        let reserver = world
            .create_agent("Aster", ControlSource::Ai, Tick(2))
            .unwrap();

        let reservation_id = world
            .try_reserve(item, reserver, TickRange::new(Tick(4), Tick(8)).unwrap())
            .unwrap();

        world.release_reservation(reservation_id).unwrap();

        assert_eq!(world.relations.reservations.get(&reservation_id), None);
        assert_eq!(world.relations.reservations_by_entity.get(&item), None);
        assert_eq!(
            world.reservations_for(item),
            Vec::<ReservationRecord>::new()
        );

        let replacement = world
            .try_reserve(item, reserver, TickRange::new(Tick(4), Tick(8)).unwrap())
            .unwrap();
        assert_eq!(replacement, ReservationId(1));
    }

    #[test]
    fn reservation_queries_hide_missing_or_archived_entities_and_release_errors_for_unknown_ids() {
        let mut world = World::new(Topology::new()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Coin, Quantity(1), Tick(1))
            .unwrap();
        let reserver = world
            .create_agent("Aster", ControlSource::Ai, Tick(2))
            .unwrap();
        let archived_item = world
            .create_item_lot(CommodityKind::Apple, Quantity(1), Tick(3))
            .unwrap();
        let archived_reserver = world
            .create_agent("Bram", ControlSource::Human, Tick(4))
            .unwrap();
        let missing = entity(999);

        world.archive_entity(archived_item, Tick(5)).unwrap();
        world.archive_entity(archived_reserver, Tick(6)).unwrap();

        assert!(matches!(
            world.try_reserve(missing, reserver, TickRange::new(Tick(1), Tick(2)).unwrap()),
            Err(WorldError::EntityNotFound(id)) if id == missing
        ));
        assert!(matches!(
            world.try_reserve(item, archived_reserver, TickRange::new(Tick(1), Tick(2)).unwrap()),
            Err(WorldError::ArchivedEntity(id)) if id == archived_reserver
        ));
        assert_eq!(
            world.reservations_for(missing),
            Vec::<ReservationRecord>::new()
        );
        assert_eq!(
            world.reservations_for(archived_item),
            Vec::<ReservationRecord>::new()
        );

        let err = world.release_reservation(ReservationId(42)).unwrap_err();
        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn reservation_roundtrip_preserves_records_and_next_id() {
        let mut world = World::new(Topology::new()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Medicine, Quantity(1), Tick(1))
            .unwrap();
        let reserver = world
            .create_agent("Aster", ControlSource::Ai, Tick(2))
            .unwrap();

        let first = world
            .try_reserve(item, reserver, TickRange::new(Tick(2), Tick(4)).unwrap())
            .unwrap();
        let second = world
            .try_reserve(item, reserver, TickRange::new(Tick(6), Tick(9)).unwrap())
            .unwrap();

        let bytes = bincode::serialize(&world).unwrap();
        let mut roundtrip: World = bincode::deserialize(&bytes).unwrap();

        assert_eq!(
            roundtrip.reservations_for(item),
            world.reservations_for(item)
        );
        assert_eq!(roundtrip.relations.next_reservation_id, 2);

        let third = roundtrip
            .try_reserve(item, reserver, TickRange::new(Tick(9), Tick(12)).unwrap())
            .unwrap();
        assert_eq!(first, ReservationId(0));
        assert_eq!(second, ReservationId(1));
        assert_eq!(third, ReservationId(2));
    }

    #[test]
    fn set_owner_sets_and_replaces_reverse_index() {
        let mut world = World::new(Topology::new()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Coin, Quantity(3), Tick(1))
            .unwrap();
        let first_owner = world.create_faction("River Pact", Tick(2)).unwrap();
        let second_owner = world.create_faction("Granary Guild", Tick(3)).unwrap();

        world.set_owner(item, first_owner).unwrap();
        assert_eq!(world.relations.owned_by.get(&item), Some(&first_owner));
        assert_eq!(
            world.relations.property_of.get(&first_owner),
            Some(&BTreeSet::from([item]))
        );

        world.set_owner(item, second_owner).unwrap();
        assert_eq!(world.relations.owned_by.get(&item), Some(&second_owner));
        assert_eq!(world.relations.property_of.get(&first_owner), None);
        assert_eq!(
            world.relations.property_of.get(&second_owner),
            Some(&BTreeSet::from([item]))
        );
    }

    #[test]
    fn clear_owner_removes_relation_and_is_idempotent() {
        let mut world = World::new(Topology::new()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Coin, Quantity(3), Tick(1))
            .unwrap();
        let owner = world.create_faction("River Pact", Tick(2)).unwrap();

        world.set_owner(item, owner).unwrap();
        world.clear_owner(item).unwrap();
        world.clear_owner(item).unwrap();

        assert_eq!(world.relations.owned_by.get(&item), None);
        assert_eq!(world.relations.property_of.get(&owner), None);
    }

    #[test]
    fn set_possessor_sets_and_replaces_reverse_index() {
        let mut world = World::new(Topology::new()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Bread, Quantity(2), Tick(1))
            .unwrap();
        let first_holder = world
            .create_agent("Aster", ControlSource::Ai, Tick(2))
            .unwrap();
        let second_holder = world
            .create_agent("Bram", ControlSource::Human, Tick(3))
            .unwrap();

        world.set_possessor(item, first_holder).unwrap();
        assert_eq!(world.relations.possessed_by.get(&item), Some(&first_holder));
        assert_eq!(
            world.relations.possessions_of.get(&first_holder),
            Some(&BTreeSet::from([item]))
        );

        world.set_possessor(item, second_holder).unwrap();
        assert_eq!(
            world.relations.possessed_by.get(&item),
            Some(&second_holder)
        );
        assert_eq!(world.relations.possessions_of.get(&first_holder), None);
        assert_eq!(
            world.relations.possessions_of.get(&second_holder),
            Some(&BTreeSet::from([item]))
        );
    }

    #[test]
    fn clear_possessor_removes_relation_and_is_idempotent() {
        let mut world = World::new(Topology::new()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Bread, Quantity(2), Tick(1))
            .unwrap();
        let holder = world
            .create_agent("Aster", ControlSource::Ai, Tick(2))
            .unwrap();

        world.set_possessor(item, holder).unwrap();
        world.clear_possessor(item).unwrap();
        world.clear_possessor(item).unwrap();

        assert_eq!(world.relations.possessed_by.get(&item), None);
        assert_eq!(world.relations.possessions_of.get(&holder), None);
    }

    #[test]
    fn ownership_and_possession_stay_independent() {
        let mut world = World::new(Topology::new()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Medicine, Quantity(1), Tick(1))
            .unwrap();
        let owner = world.create_faction("River Pact", Tick(2)).unwrap();
        let holder = world
            .create_agent("Aster", ControlSource::Ai, Tick(3))
            .unwrap();

        world.set_owner(item, owner).unwrap();
        world.set_possessor(item, holder).unwrap();
        world.clear_owner(item).unwrap();

        assert_eq!(world.relations.owned_by.get(&item), None);
        assert_eq!(world.relations.property_of.get(&owner), None);
        assert_eq!(world.relations.possessed_by.get(&item), Some(&holder));
        assert_eq!(
            world.relations.possessions_of.get(&holder),
            Some(&BTreeSet::from([item]))
        );
    }

    #[test]
    fn ownership_query_helpers_return_live_relations_or_none() {
        let mut world = World::new(Topology::new()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Medicine, Quantity(1), Tick(1))
            .unwrap();
        let owner = world.create_faction("River Pact", Tick(2)).unwrap();
        let holder = world
            .create_agent("Aster", ControlSource::Ai, Tick(3))
            .unwrap();

        assert_eq!(world.owner_of(item), None);
        assert_eq!(world.possessor_of(item), None);

        world.set_owner(item, owner).unwrap();
        world.set_possessor(item, holder).unwrap();

        assert_eq!(world.owner_of(item), Some(owner));
        assert_eq!(world.possessor_of(item), Some(holder));

        world.clear_owner(item).unwrap();
        world.clear_possessor(item).unwrap();
        world.archive_entity(owner, Tick(4)).unwrap();
        world.archive_entity(holder, Tick(5)).unwrap();

        assert_eq!(world.owner_of(item), None);
        assert_eq!(world.possessor_of(item), None);
    }

    #[test]
    fn can_exercise_control_enforces_possession_then_unpossessed_ownership() {
        let mut world = World::new(Topology::new()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Apple, Quantity(1), Tick(1))
            .unwrap();
        let owner = world.create_faction("River Pact", Tick(2)).unwrap();
        let holder = world
            .create_agent("Aster", ControlSource::Ai, Tick(3))
            .unwrap();
        let stranger = world
            .create_agent("Bram", ControlSource::Human, Tick(4))
            .unwrap();

        world.set_owner(item, owner).unwrap();
        assert!(world.can_exercise_control(owner, item).is_ok());

        world.set_possessor(item, holder).unwrap();
        assert!(world.can_exercise_control(holder, item).is_ok());

        let blocked_owner = world.can_exercise_control(owner, item).unwrap_err();
        assert!(matches!(blocked_owner, WorldError::PreconditionFailed(_)));

        let unrelated = world.can_exercise_control(stranger, item).unwrap_err();
        assert!(matches!(unrelated, WorldError::PreconditionFailed(_)));
    }

    #[test]
    fn can_exercise_control_flows_through_controlled_containers() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world
            .topology()
            .place_ids()
            .next()
            .expect("prototype world provides at least one place");
        let actor = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let satchel = world
            .create_container(
                Container {
                    capacity: LoadUnits(20),
                    allowed_commodities: None,
                    allows_unique_items: true,
                    allows_nested_containers: true,
                },
                Tick(2),
            )
            .unwrap();
        let bread = world
            .create_item_lot(CommodityKind::Bread, Quantity(1), Tick(3))
            .unwrap();

        world.set_ground_location(actor, place).unwrap();
        world.set_ground_location(satchel, place).unwrap();
        world.set_possessor(satchel, actor).unwrap();
        world.put_into_container(bread, satchel).unwrap();

        assert!(world.can_exercise_control(actor, satchel).is_ok());
        assert!(world.can_exercise_control(actor, bread).is_ok());
    }

    #[test]
    fn ownership_and_control_methods_reject_missing_and_archived_entities() {
        let mut world = World::new(Topology::new()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Coin, Quantity(1), Tick(1))
            .unwrap();
        let owner = world.create_faction("River Pact", Tick(2)).unwrap();
        let archived_holder = world
            .create_agent("Aster", ControlSource::Ai, Tick(3))
            .unwrap();
        let archived_item = world
            .create_item_lot(CommodityKind::Apple, Quantity(1), Tick(4))
            .unwrap();
        let missing = entity(999);

        world.archive_entity(archived_holder, Tick(5)).unwrap();
        world.archive_entity(archived_item, Tick(6)).unwrap();

        assert!(matches!(
            world.set_owner(missing, owner),
            Err(WorldError::EntityNotFound(id)) if id == missing
        ));
        assert!(matches!(
            world.set_owner(item, archived_holder),
            Err(WorldError::ArchivedEntity(id)) if id == archived_holder
        ));
        assert!(matches!(
            world.set_possessor(item, archived_holder),
            Err(WorldError::ArchivedEntity(id)) if id == archived_holder
        ));
        assert!(matches!(
            world.clear_owner(archived_item),
            Err(WorldError::ArchivedEntity(id)) if id == archived_item
        ));
        assert!(matches!(
            world.clear_possessor(missing),
            Err(WorldError::EntityNotFound(id)) if id == missing
        ));
        assert!(matches!(
            world.can_exercise_control(owner, archived_item),
            Err(WorldError::ArchivedEntity(id)) if id == archived_item
        ));
        assert!(matches!(
            world.can_exercise_control(missing, item),
            Err(WorldError::EntityNotFound(id)) if id == missing
        ));
    }

    #[test]
    fn faction_membership_queries_stay_bidirectional_and_idempotent() {
        let mut world = World::new(Topology::new()).unwrap();
        let member = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let faction_a = world.create_faction("River Pact", Tick(2)).unwrap();
        let faction_b = world.create_faction("Granary Guild", Tick(3)).unwrap();

        world.add_member(member, faction_b).unwrap();
        world.add_member(member, faction_a).unwrap();
        world.add_member(member, faction_a).unwrap();

        assert_eq!(world.factions_of(member), vec![faction_a, faction_b]);
        assert_eq!(world.members_of(faction_a), vec![member]);
        assert_eq!(world.members_of(faction_b), vec![member]);
        assert_eq!(
            world.relations.member_of.get(&member),
            Some(&BTreeSet::from([faction_a, faction_b]))
        );
        assert_eq!(
            world.relations.members_of.get(&faction_a),
            Some(&BTreeSet::from([member]))
        );

        world.remove_member(member, faction_a).unwrap();
        world.remove_member(member, faction_a).unwrap();

        assert_eq!(world.factions_of(member), vec![faction_b]);
        assert_eq!(world.members_of(faction_a), Vec::<EntityId>::new());
        assert_eq!(world.members_of(faction_b), vec![member]);
    }

    #[test]
    fn loyalty_and_hostility_queries_stay_bidirectional_and_strength_aware() {
        let mut world = World::new(Topology::new()).unwrap();
        let subject = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let target_a = world.create_faction("River Pact", Tick(2)).unwrap();
        let target_b = world.create_office("Granary Chair", Tick(3)).unwrap();
        let weak = Permille::new(250).unwrap();
        let strong = Permille::new(900).unwrap();

        world.set_loyalty(subject, target_b, weak).unwrap();
        world.set_loyalty(subject, target_a, weak).unwrap();
        world.set_loyalty(subject, target_a, strong).unwrap();
        world.add_hostility(subject, target_b).unwrap();
        world.add_hostility(subject, target_b).unwrap();

        assert_eq!(world.loyalty_to(subject, target_a), Some(strong));
        assert_eq!(
            world.loyal_targets_of(subject),
            vec![(target_a, strong), (target_b, weak)]
        );
        assert_eq!(world.loyal_subjects_of(target_a), vec![(subject, strong)]);
        assert_eq!(world.hostile_targets_of(subject), vec![target_b]);
        assert_eq!(world.hostile_towards(target_b), vec![subject]);

        world.clear_loyalty(subject, target_a).unwrap();
        world.clear_loyalty(subject, target_a).unwrap();
        world.remove_hostility(subject, target_b).unwrap();
        world.remove_hostility(subject, target_b).unwrap();

        assert_eq!(world.loyal_targets_of(subject), vec![(target_b, weak)]);
        assert_eq!(
            world.loyal_subjects_of(target_a),
            Vec::<(EntityId, Permille)>::new()
        );
        assert_eq!(world.hostile_targets_of(subject), Vec::<EntityId>::new());
        assert_eq!(world.hostile_towards(target_b), Vec::<EntityId>::new());
    }

    #[test]
    fn office_assignment_replaces_prior_holder_and_vacates_idempotently() {
        let mut world = World::new(Topology::new()).unwrap();
        let office_a = world.create_office("Granary Chair", Tick(1)).unwrap();
        let office_b = world.create_office("Market Chair", Tick(2)).unwrap();
        let first_holder = world
            .create_agent("Aster", ControlSource::Ai, Tick(3))
            .unwrap();
        let second_holder = world
            .create_agent("Bram", ControlSource::Human, Tick(4))
            .unwrap();

        world.assign_office(office_a, first_holder).unwrap();
        world.assign_office(office_b, first_holder).unwrap();
        world.assign_office(office_a, second_holder).unwrap();

        assert_eq!(world.office_holder(office_a), Some(second_holder));
        assert_eq!(world.office_holder(office_b), Some(first_holder));
        assert_eq!(world.offices_held_by(first_holder), vec![office_b]);
        assert_eq!(world.offices_held_by(second_holder), vec![office_a]);
        assert_eq!(
            world.relations.office_holder.get(&office_a),
            Some(&second_holder)
        );
        assert_eq!(
            world.relations.offices_held.get(&first_holder),
            Some(&BTreeSet::from([office_b]))
        );

        world.vacate_office(office_a).unwrap();
        world.vacate_office(office_a).unwrap();

        assert_eq!(world.office_holder(office_a), None);
        assert_eq!(world.offices_held_by(second_holder), Vec::<EntityId>::new());
    }

    #[test]
    fn fact_queries_are_agent_scoped_and_idempotent() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let known_a = FactId(7);
        let known_b = FactId(9);
        let believed = FactId(11);

        world.add_known_fact(agent, known_b).unwrap();
        world.add_known_fact(agent, known_a).unwrap();
        world.add_known_fact(agent, known_a).unwrap();
        world.add_believed_fact(agent, believed).unwrap();
        world.add_believed_fact(agent, believed).unwrap();

        assert_eq!(world.known_facts(agent), vec![known_a, known_b]);
        assert_eq!(world.believed_facts(agent), vec![believed]);

        world.remove_known_fact(agent, known_a).unwrap();
        world.remove_known_fact(agent, known_a).unwrap();
        world.remove_believed_fact(agent, believed).unwrap();
        world.remove_believed_fact(agent, believed).unwrap();

        assert_eq!(world.known_facts(agent), vec![known_b]);
        assert_eq!(world.believed_facts(agent), Vec::<FactId>::new());
    }

    #[test]
    fn social_query_helpers_hide_archived_entities_even_if_rows_are_stale() {
        let mut world = World::new(Topology::new()).unwrap();
        let member = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let faction = world.create_faction("River Pact", Tick(2)).unwrap();
        let loyal_target = world.create_faction("Granary Guild", Tick(3)).unwrap();
        let office = world.create_office("Granary Chair", Tick(4)).unwrap();
        let hostile_target = world
            .create_agent("Bram", ControlSource::Human, Tick(5))
            .unwrap();
        let knower = world
            .create_agent("Cato", ControlSource::Ai, Tick(6))
            .unwrap();
        let fact = FactId(21);

        world.add_member(member, faction).unwrap();
        world
            .set_loyalty(member, loyal_target, Permille::new(700).unwrap())
            .unwrap();
        world.assign_office(office, member).unwrap();
        world.add_hostility(member, hostile_target).unwrap();
        world.add_known_fact(knower, fact).unwrap();

        world.vacate_office(office).unwrap();
        world.archive_entity(member, Tick(7)).unwrap();
        world.archive_entity(faction, Tick(8)).unwrap();
        world.archive_entity(loyal_target, Tick(9)).unwrap();
        world.archive_entity(office, Tick(10)).unwrap();
        world.archive_entity(hostile_target, Tick(11)).unwrap();
        world.archive_entity(knower, Tick(12)).unwrap();

        world
            .relations
            .member_of
            .insert(member, BTreeSet::from([faction]));
        world
            .relations
            .members_of
            .insert(faction, BTreeSet::from([member]));
        world.relations.loyal_to.insert(
            member,
            BTreeMap::from([(loyal_target, Permille::new(700).unwrap())]),
        );
        world.relations.loyalty_from.insert(
            loyal_target,
            BTreeMap::from([(member, Permille::new(700).unwrap())]),
        );
        world.relations.office_holder.insert(office, member);
        world
            .relations
            .offices_held
            .insert(member, BTreeSet::from([office]));
        world
            .relations
            .hostile_to
            .insert(member, BTreeSet::from([hostile_target]));
        world
            .relations
            .hostility_from
            .insert(hostile_target, BTreeSet::from([member]));
        world
            .relations
            .knows_fact
            .insert(knower, BTreeSet::from([fact]));

        assert_eq!(world.factions_of(member), Vec::<EntityId>::new());
        assert_eq!(world.members_of(faction), Vec::<EntityId>::new());
        assert_eq!(
            world.loyal_targets_of(member),
            Vec::<(EntityId, Permille)>::new()
        );
        assert_eq!(
            world.loyal_subjects_of(loyal_target),
            Vec::<(EntityId, Permille)>::new()
        );
        assert_eq!(world.office_holder(office), None);
        assert_eq!(world.offices_held_by(member), Vec::<EntityId>::new());
        assert_eq!(world.hostile_targets_of(member), Vec::<EntityId>::new());
        assert_eq!(
            world.hostile_towards(hostile_target),
            Vec::<EntityId>::new()
        );
        assert_eq!(world.known_facts(knower), Vec::<FactId>::new());
    }

    #[test]
    fn social_apis_reject_wrong_kinds_missing_and_archived_entities() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let faction = world.create_faction("River Pact", Tick(2)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Coin, Quantity(1), Tick(4))
            .unwrap();
        let archived_agent = world
            .create_agent("Bram", ControlSource::Human, Tick(5))
            .unwrap();
        let missing = entity(999);

        world.archive_entity(archived_agent, Tick(6)).unwrap();

        assert!(matches!(
            world.add_member(agent, item),
            Err(WorldError::InvalidOperation(_))
        ));
        assert!(matches!(
            world.assign_office(item, agent),
            Err(WorldError::InvalidOperation(_))
        ));
        assert!(matches!(
            world.add_known_fact(faction, FactId(1)),
            Err(WorldError::InvalidOperation(_))
        ));
        assert!(matches!(
            world.set_loyalty(agent, archived_agent, Permille::new(500).unwrap()),
            Err(WorldError::ArchivedEntity(id)) if id == archived_agent
        ));
        assert!(matches!(
            world.add_hostility(missing, faction),
            Err(WorldError::EntityNotFound(id)) if id == missing
        ));
        assert!(matches!(
            world.remove_believed_fact(archived_agent, FactId(2)),
            Err(WorldError::ArchivedEntity(id)) if id == archived_agent
        ));
        assert!(matches!(
            world.vacate_office(missing),
            Err(WorldError::EntityNotFound(id)) if id == missing
        ));
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

        world
            .get_component_name_mut(id)
            .unwrap()
            .0
            .push_str(" Annex");
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
    fn insert_duplicate_item_lot_component_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world
            .create_item_lot(CommodityKind::Medicine, Quantity(2), Tick(1))
            .unwrap();

        let err = world
            .insert_component_item_lot(
                id,
                ItemLot {
                    commodity: CommodityKind::Medicine,
                    quantity: Quantity(2),
                    provenance: vec![ProvenanceEntry {
                        tick: Tick(1),
                        event_id: None,
                        operation: LotOperation::Created,
                        related_lot: None,
                        amount: Quantity(2),
                    }],
                },
            )
            .unwrap_err();

        assert!(matches!(
            err,
            WorldError::DuplicateComponent {
                entity,
                component_type: "ItemLot",
            } if entity == id
        ));
    }

    #[test]
    fn insert_duplicate_unique_item_component_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world
            .create_unique_item(
                UniqueItemKind::SimpleTool,
                Some("Hammer"),
                BTreeMap::new(),
                Tick(1),
            )
            .unwrap();

        let err = world
            .insert_component_unique_item(
                id,
                UniqueItem {
                    kind: UniqueItemKind::SimpleTool,
                    name: Some("Hammer".to_string()),
                    metadata: BTreeMap::new(),
                },
            )
            .unwrap_err();

        assert!(matches!(
            err,
            WorldError::DuplicateComponent {
                entity,
                component_type: "UniqueItem",
            } if entity == id
        ));
    }

    #[test]
    fn insert_duplicate_container_component_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world
            .create_container(
                Container {
                    capacity: LoadUnits(11),
                    allowed_commodities: None,
                    allows_unique_items: true,
                    allows_nested_containers: false,
                },
                Tick(1),
            )
            .unwrap();

        let err = world
            .insert_component_container(
                id,
                Container {
                    capacity: LoadUnits(11),
                    allowed_commodities: None,
                    allows_unique_items: true,
                    allows_nested_containers: false,
                },
            )
            .unwrap_err();

        assert!(matches!(
            err,
            WorldError::DuplicateComponent {
                entity,
                component_type: "Container",
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
    fn insert_wound_list_on_non_agent_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Office, Tick(1));

        let err = world
            .insert_component_wound_list(id, sample_wound_list())
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn combat_profile_component_roundtrip_on_agent() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Agent, Tick(1));
        let profile = sample_combat_profile();

        world.insert_component_combat_profile(id, profile).unwrap();
        assert_eq!(world.get_component_combat_profile(id), Some(&profile));
        assert!(world.has_component_combat_profile(id));

        let removed = world.remove_component_combat_profile(id).unwrap();
        assert_eq!(removed, Some(profile));
        assert_eq!(world.get_component_combat_profile(id), None);
    }

    #[test]
    fn insert_combat_profile_on_non_agent_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Office, Tick(1));

        let err = world
            .insert_component_combat_profile(id, sample_combat_profile())
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn dead_at_component_roundtrip_on_agent() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Agent, Tick(1));
        let dead_at = sample_dead_at();

        world.insert_component_dead_at(id, dead_at).unwrap();
        assert_eq!(world.get_component_dead_at(id), Some(&dead_at));
        assert!(world.has_component_dead_at(id));

        let removed = world.remove_component_dead_at(id).unwrap();
        assert_eq!(removed, Some(dead_at));
        assert_eq!(world.get_component_dead_at(id), None);
    }

    #[test]
    fn insert_dead_at_on_non_agent_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Office, Tick(1));

        let err = world
            .insert_component_dead_at(id, sample_dead_at())
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn drive_thresholds_component_roundtrip_on_agent() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Agent, Tick(1));
        let thresholds = sample_drive_thresholds();

        world
            .insert_component_drive_thresholds(id, thresholds)
            .unwrap();
        assert_eq!(world.get_component_drive_thresholds(id), Some(&thresholds));
        assert!(world.has_component_drive_thresholds(id));

        let removed = world.remove_component_drive_thresholds(id).unwrap();
        assert_eq!(removed, Some(thresholds));
        assert_eq!(world.get_component_drive_thresholds(id), None);
    }

    #[test]
    fn utility_profile_component_roundtrip_on_agent() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Agent, Tick(1));
        let profile = sample_utility_profile();

        world
            .insert_component_utility_profile(id, profile.clone())
            .unwrap();
        assert_eq!(world.get_component_utility_profile(id), Some(&profile));
        assert!(world.has_component_utility_profile(id));
        assert_eq!(
            world.query_utility_profile().collect::<Vec<_>>(),
            vec![(id, &profile)]
        );
        assert_eq!(world.count_with_utility_profile(), 1);

        let removed = world.remove_component_utility_profile(id).unwrap();
        assert_eq!(removed, Some(profile));
        assert_eq!(world.get_component_utility_profile(id), None);
    }

    #[test]
    fn blocked_intent_memory_component_roundtrip_on_agent() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Agent, Tick(1));
        let memory = sample_blocked_intent_memory();

        world
            .insert_component_blocked_intent_memory(id, memory.clone())
            .unwrap();
        assert_eq!(world.get_component_blocked_intent_memory(id), Some(&memory));
        assert!(world.has_component_blocked_intent_memory(id));
        assert_eq!(
            world.query_blocked_intent_memory().collect::<Vec<_>>(),
            vec![(id, &memory)]
        );
        assert_eq!(world.count_with_blocked_intent_memory(), 1);

        let removed = world.remove_component_blocked_intent_memory(id).unwrap();
        assert_eq!(removed, Some(memory));
        assert_eq!(world.get_component_blocked_intent_memory(id), None);
    }

    #[test]
    fn insert_drive_thresholds_on_non_agent_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Office, Tick(1));

        let err = world
            .insert_component_drive_thresholds(id, sample_drive_thresholds())
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn insert_utility_profile_on_non_agent_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Office, Tick(1));

        let err = world
            .insert_component_utility_profile(id, sample_utility_profile())
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
        assert_eq!(world.get_component_utility_profile(id), None);
    }

    #[test]
    fn insert_blocked_intent_memory_on_non_agent_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Office, Tick(1));

        let err = world
            .insert_component_blocked_intent_memory(id, sample_blocked_intent_memory())
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
        assert_eq!(world.get_component_blocked_intent_memory(id), None);
    }

    #[test]
    fn homeostatic_needs_component_roundtrip_on_agent() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Agent, Tick(1));
        let needs = sample_homeostatic_needs();

        world.insert_component_homeostatic_needs(id, needs).unwrap();
        assert_eq!(world.get_component_homeostatic_needs(id), Some(&needs));
        assert!(world.has_component_homeostatic_needs(id));

        let removed = world.remove_component_homeostatic_needs(id).unwrap();
        assert_eq!(removed, Some(needs));
        assert_eq!(world.get_component_homeostatic_needs(id), None);
    }

    #[test]
    fn insert_homeostatic_needs_on_non_agent_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Office, Tick(1));

        let err = world
            .insert_component_homeostatic_needs(id, sample_homeostatic_needs())
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn deprivation_exposure_component_roundtrip_on_agent() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Agent, Tick(1));
        let exposure = sample_deprivation_exposure();

        world
            .insert_component_deprivation_exposure(id, exposure)
            .unwrap();
        assert_eq!(
            world.get_component_deprivation_exposure(id),
            Some(&exposure)
        );
        assert!(world.has_component_deprivation_exposure(id));

        let removed = world.remove_component_deprivation_exposure(id).unwrap();
        assert_eq!(removed, Some(exposure));
        assert_eq!(world.get_component_deprivation_exposure(id), None);
    }

    #[test]
    fn insert_deprivation_exposure_on_non_agent_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Office, Tick(1));

        let err = world
            .insert_component_deprivation_exposure(id, sample_deprivation_exposure())
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn metabolism_profile_component_roundtrip_on_agent() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Agent, Tick(1));
        let profile = sample_metabolism_profile();

        world
            .insert_component_metabolism_profile(id, profile)
            .unwrap();
        assert_eq!(world.get_component_metabolism_profile(id), Some(&profile));
        assert!(world.has_component_metabolism_profile(id));

        let removed = world.remove_component_metabolism_profile(id).unwrap();
        assert_eq!(removed, Some(profile));
        assert_eq!(world.get_component_metabolism_profile(id), None);
    }

    #[test]
    fn insert_metabolism_profile_on_non_agent_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Office, Tick(1));

        let err = world
            .insert_component_metabolism_profile(id, sample_metabolism_profile())
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn insert_item_lot_on_non_item_lot_entity_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Office, Tick(1));

        let err = world
            .insert_component_item_lot(
                id,
                ItemLot {
                    commodity: CommodityKind::Firewood,
                    quantity: Quantity(5),
                    provenance: vec![ProvenanceEntry {
                        tick: Tick(1),
                        event_id: None,
                        operation: LotOperation::Created,
                        related_lot: None,
                        amount: Quantity(5),
                    }],
                },
            )
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn insert_unique_item_on_non_unique_item_entity_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Office, Tick(1));

        let err = world
            .insert_component_unique_item(
                id,
                UniqueItem {
                    kind: UniqueItemKind::OfficeInsignia,
                    name: Some("Seal".to_string()),
                    metadata: BTreeMap::new(),
                },
            )
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn insert_container_on_non_container_entity_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Office, Tick(1));

        let err = world
            .insert_component_container(
                id,
                Container {
                    capacity: LoadUnits(10),
                    allowed_commodities: Some(BTreeSet::from([CommodityKind::Firewood])),
                    allows_unique_items: false,
                    allows_nested_containers: false,
                },
            )
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn resource_source_component_roundtrip_on_facility() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Facility, Tick(1));
        let source = sample_resource_source();

        world
            .insert_component_resource_source(id, source.clone())
            .unwrap();
        assert_eq!(world.get_component_resource_source(id), Some(&source));
        assert!(world.has_component_resource_source(id));

        let removed = world.remove_component_resource_source(id).unwrap();
        assert_eq!(removed, Some(source));
        assert_eq!(world.get_component_resource_source(id), None);
    }

    #[test]
    fn resource_source_component_roundtrip_on_place() {
        let mut world = World::new(test_topology()).unwrap();
        let place = entity(2);
        let source = ResourceSource {
            commodity: CommodityKind::Grain,
            available_quantity: Quantity(5),
            max_quantity: Quantity(18),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        };

        world
            .insert_component_resource_source(place, source.clone())
            .unwrap();
        assert_eq!(world.get_component_resource_source(place), Some(&source));
        assert!(world.has_component_resource_source(place));

        let removed = world.remove_component_resource_source(place).unwrap();
        assert_eq!(removed, Some(source));
        assert_eq!(world.get_component_resource_source(place), None);
    }

    #[test]
    fn insert_resource_source_on_non_matching_entity_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Agent, Tick(1));

        let err = world
            .insert_component_resource_source(id, sample_resource_source())
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn carry_capacity_component_roundtrip_on_agent() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world.create_entity(EntityKind::Agent, Tick(1));
        let capacity = CarryCapacity(LoadUnits(16));

        world
            .insert_component_carry_capacity(agent, capacity)
            .unwrap();
        assert_eq!(world.get_component_carry_capacity(agent), Some(&capacity));
        assert!(world.has_component_carry_capacity(agent));

        let removed = world.remove_component_carry_capacity(agent).unwrap();
        assert_eq!(removed, Some(capacity));
        assert_eq!(world.get_component_carry_capacity(agent), None);
    }

    #[test]
    fn known_recipes_component_roundtrip_on_agent() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world.create_entity(EntityKind::Agent, Tick(1));
        let known = KnownRecipes::with([crate::RecipeId(5), crate::RecipeId(1)]);

        world
            .insert_component_known_recipes(agent, known.clone())
            .unwrap();
        assert_eq!(world.get_component_known_recipes(agent), Some(&known));
        assert!(world.has_component_known_recipes(agent));

        let removed = world.remove_component_known_recipes(agent).unwrap();
        assert_eq!(removed, Some(known));
        assert_eq!(world.get_component_known_recipes(agent), None);
    }

    #[test]
    fn demand_memory_component_roundtrip_on_agent() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world.create_entity(EntityKind::Agent, Tick(1));
        let memory = sample_demand_memory();

        world
            .insert_component_demand_memory(agent, memory.clone())
            .unwrap();
        assert_eq!(world.get_component_demand_memory(agent), Some(&memory));
        assert!(world.has_component_demand_memory(agent));
        assert_eq!(
            world.query_demand_memory().collect::<Vec<_>>(),
            vec![(agent, &memory)]
        );
        assert_eq!(world.count_with_demand_memory(), 1);

        let removed = world.remove_component_demand_memory(agent).unwrap();
        assert_eq!(removed, Some(memory));
        assert_eq!(world.get_component_demand_memory(agent), None);
    }

    #[test]
    fn merchandise_profile_component_roundtrip_on_agent() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world.create_entity(EntityKind::Agent, Tick(1));
        let profile = sample_merchandise_profile();

        world
            .insert_component_merchandise_profile(agent, profile.clone())
            .unwrap();
        assert_eq!(
            world.get_component_merchandise_profile(agent),
            Some(&profile)
        );
        assert!(world.has_component_merchandise_profile(agent));
        assert_eq!(
            world.query_merchandise_profile().collect::<Vec<_>>(),
            vec![(agent, &profile)]
        );
        assert_eq!(world.count_with_merchandise_profile(), 1);

        let removed = world.remove_component_merchandise_profile(agent).unwrap();
        assert_eq!(removed, Some(profile));
        assert_eq!(world.get_component_merchandise_profile(agent), None);
    }

    #[test]
    fn substitute_preferences_component_roundtrip_on_agent() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world.create_entity(EntityKind::Agent, Tick(1));
        let preferences = sample_substitute_preferences();

        world
            .insert_component_substitute_preferences(agent, preferences.clone())
            .unwrap();
        assert_eq!(
            world.get_component_substitute_preferences(agent),
            Some(&preferences)
        );
        assert!(world.has_component_substitute_preferences(agent));
        assert_eq!(
            world.query_substitute_preferences().collect::<Vec<_>>(),
            vec![(agent, &preferences)]
        );
        assert_eq!(world.count_with_substitute_preferences(), 1);

        let removed = world
            .remove_component_substitute_preferences(agent)
            .unwrap();
        assert_eq!(removed, Some(preferences));
        assert_eq!(world.get_component_substitute_preferences(agent), None);
    }

    #[test]
    fn trade_disposition_profile_component_roundtrip_on_agent() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world.create_entity(EntityKind::Agent, Tick(1));
        let profile = sample_trade_disposition_profile();

        world
            .insert_component_trade_disposition_profile(agent, profile.clone())
            .unwrap();
        assert_eq!(
            world.get_component_trade_disposition_profile(agent),
            Some(&profile)
        );
        assert!(world.has_component_trade_disposition_profile(agent));
        assert_eq!(
            world.query_trade_disposition_profile().collect::<Vec<_>>(),
            vec![(agent, &profile)]
        );
        assert_eq!(world.count_with_trade_disposition_profile(), 1);

        let removed = world
            .remove_component_trade_disposition_profile(agent)
            .unwrap();
        assert_eq!(removed, Some(profile));
        assert_eq!(world.get_component_trade_disposition_profile(agent), None);
    }

    #[test]
    fn workstation_marker_component_roundtrip_on_facility() {
        let mut world = World::new(Topology::new()).unwrap();
        let facility = world.create_entity(EntityKind::Facility, Tick(1));
        let marker = WorkstationMarker(WorkstationTag::Forge);

        world
            .insert_component_workstation_marker(facility, marker)
            .unwrap();
        assert_eq!(
            world.get_component_workstation_marker(facility),
            Some(&marker)
        );
        assert!(world.has_component_workstation_marker(facility));

        let removed = world.remove_component_workstation_marker(facility).unwrap();
        assert_eq!(removed, Some(marker));
        assert_eq!(world.get_component_workstation_marker(facility), None);
    }

    #[test]
    fn insert_known_recipes_on_non_agent_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Facility, Tick(1));

        let err = world
            .insert_component_known_recipes(id, KnownRecipes::with([crate::RecipeId(2)]))
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn insert_demand_memory_on_non_agent_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Facility, Tick(1));

        let err = world
            .insert_component_demand_memory(
                id,
                DemandMemory {
                    observations: vec![],
                },
            )
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn insert_merchandise_profile_on_non_agent_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Facility, Tick(1));

        let err = world
            .insert_component_merchandise_profile(
                id,
                MerchandiseProfile {
                    sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                    home_market: None,
                },
            )
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn insert_trade_disposition_profile_on_non_agent_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Facility, Tick(1));

        let err = world
            .insert_component_trade_disposition_profile(
                id,
                TradeDispositionProfile {
                    negotiation_round_ticks: NonZeroU32::new(4).unwrap(),
                    initial_offer_bias: Permille::new(500).unwrap(),
                    concession_rate: Permille::new(100).unwrap(),
                    demand_memory_retention_ticks: 60,
                },
            )
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn insert_substitute_preferences_on_non_agent_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Facility, Tick(1));

        let err = world
            .insert_component_substitute_preferences(
                id,
                SubstitutePreferences {
                    preferences: BTreeMap::from([(
                        crate::TradeCategory::Food,
                        vec![CommodityKind::Bread, CommodityKind::Apple],
                    )]),
                },
            )
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn production_job_component_roundtrip_on_facility() {
        let mut world = World::new(Topology::new()).unwrap();
        let facility = world.create_entity(EntityKind::Facility, Tick(1));
        let job = ProductionJob {
            recipe_id: crate::RecipeId(4),
            worker: entity(7),
            staged_inputs_container: entity(8),
            progress_ticks: 12,
        };

        world
            .insert_component_production_job(facility, job.clone())
            .unwrap();
        assert_eq!(world.get_component_production_job(facility), Some(&job));
        assert!(world.has_component_production_job(facility));

        let removed = world.remove_component_production_job(facility).unwrap();
        assert_eq!(removed, Some(job));
        assert_eq!(world.get_component_production_job(facility), None);
    }

    #[test]
    fn insert_carry_capacity_on_non_agent_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Facility, Tick(1));

        let err = world
            .insert_component_carry_capacity(id, CarryCapacity(LoadUnits(9)))
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn insert_workstation_marker_on_non_facility_errors() {
        let mut world = World::new(test_topology()).unwrap();
        let id = entity(2);

        let err = world
            .insert_component_workstation_marker(id, WorkstationMarker(WorkstationTag::Mill))
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn insert_production_job_on_non_facility_errors() {
        let mut world = World::new(Topology::new()).unwrap();
        let id = world.create_entity(EntityKind::Agent, Tick(1));

        let err = world
            .insert_component_production_job(
                id,
                ProductionJob {
                    recipe_id: crate::RecipeId(6),
                    worker: entity(3),
                    staged_inputs_container: entity(4),
                    progress_ticks: 2,
                },
            )
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn in_transit_on_edge_component_roundtrip_on_agent() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world.create_entity(EntityKind::Agent, Tick(1));
        let transit = InTransitOnEdge {
            edge_id: TravelEdgeId(6),
            origin: entity(2),
            destination: entity(3),
            departure_tick: Tick(4),
            arrival_tick: Tick(12),
        };

        world
            .insert_component_in_transit_on_edge(agent, transit.clone())
            .unwrap();
        assert_eq!(
            world.get_component_in_transit_on_edge(agent),
            Some(&transit)
        );
        assert!(world.has_component_in_transit_on_edge(agent));

        let removed = world.remove_component_in_transit_on_edge(agent).unwrap();
        assert_eq!(removed, Some(transit));
        assert_eq!(world.get_component_in_transit_on_edge(agent), None);
    }

    #[test]
    fn insert_in_transit_on_edge_on_non_agent_errors() {
        let mut world = World::new(test_topology()).unwrap();
        let id = entity(2);

        let err = world
            .insert_component_in_transit_on_edge(
                id,
                InTransitOnEdge {
                    edge_id: TravelEdgeId(1),
                    origin: entity(2),
                    destination: entity(3),
                    departure_tick: Tick(5),
                    arrival_tick: Tick(9),
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

        assert_eq!(
            world.entities_with_name().collect::<Vec<_>>(),
            vec![named_agent]
        );
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
    fn entities_with_wound_list_returns_live_entities() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world.create_entity(EntityKind::Agent, Tick(1));
        let archived_agent = world.create_entity(EntityKind::Agent, Tick(2));

        world
            .insert_component_wound_list(agent, sample_wound_list())
            .unwrap();
        world
            .insert_component_wound_list(archived_agent, sample_wound_list())
            .unwrap();
        world.archive_entity(archived_agent, Tick(3)).unwrap();

        assert_eq!(
            world.entities_with_wound_list().collect::<Vec<_>>(),
            vec![agent]
        );
    }

    #[test]
    fn entities_with_drive_thresholds_returns_live_entities() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world.create_entity(EntityKind::Agent, Tick(1));
        let archived_agent = world.create_entity(EntityKind::Agent, Tick(2));

        world
            .insert_component_drive_thresholds(agent, sample_drive_thresholds())
            .unwrap();
        world
            .insert_component_drive_thresholds(archived_agent, sample_drive_thresholds())
            .unwrap();
        world.archive_entity(archived_agent, Tick(3)).unwrap();

        assert_eq!(
            world.entities_with_drive_thresholds().collect::<Vec<_>>(),
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
            vec![(first, ControlSource::Human), (second, ControlSource::Ai),]
        );
    }

    #[test]
    fn query_wound_list_returns_sorted_pairs() {
        let mut world = World::new(Topology::new()).unwrap();
        let first = world.create_entity(EntityKind::Agent, Tick(1));
        let second = world.create_entity(EntityKind::Agent, Tick(2));

        world
            .insert_component_wound_list(first, sample_wound_list())
            .unwrap();
        world
            .insert_component_wound_list(
                second,
                WoundList {
                    wounds: vec![Wound {
                        id: crate::WoundId(2),
                        body_part: BodyPart::LeftLeg,
                        cause: WoundCause::Deprivation(DeprivationKind::Dehydration),
                        severity: Permille::new(450).unwrap(),
                        inflicted_at: Tick(8),
                        bleed_rate_per_tick: Permille::new(0).unwrap(),
                    }],
                },
            )
            .unwrap();

        let pairs = world
            .query_wound_list()
            .map(|(entity, wound_list)| (entity, wound_list.wounds.len()))
            .collect::<Vec<_>>();

        assert_eq!(pairs, vec![(first, 1), (second, 1)]);
    }

    #[test]
    fn query_drive_thresholds_returns_sorted_pairs() {
        let mut world = World::new(Topology::new()).unwrap();
        let first = world.create_entity(EntityKind::Agent, Tick(1));
        let second = world.create_entity(EntityKind::Agent, Tick(2));

        world
            .insert_component_drive_thresholds(first, sample_drive_thresholds())
            .unwrap();

        let mut second_thresholds = sample_drive_thresholds();
        second_thresholds.danger = second_thresholds.pain;
        world
            .insert_component_drive_thresholds(second, second_thresholds)
            .unwrap();

        let pairs = world
            .query_drive_thresholds()
            .map(|(entity, thresholds)| (entity, thresholds.danger.critical().value()))
            .collect::<Vec<_>>();

        assert_eq!(
            pairs,
            vec![
                (first, sample_drive_thresholds().danger.critical().value()),
                (second, second_thresholds.danger.critical().value()),
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
        assert_eq!(world.entities_with_wound_list().count(), 0);
        assert_eq!(world.query_wound_list().count(), 0);
        assert_eq!(world.entities_with_blocked_intent_memory().count(), 0);
        assert_eq!(world.query_blocked_intent_memory().count(), 0);
        assert_eq!(world.entities_with_drive_thresholds().count(), 0);
        assert_eq!(world.query_drive_thresholds().count(), 0);
        assert_eq!(world.entities_with_item_lot().count(), 0);
        assert_eq!(world.query_item_lot().count(), 0);
        assert_eq!(world.entities_with_unique_item().count(), 0);
        assert_eq!(world.query_unique_item().count(), 0);
        assert_eq!(world.entities_with_container().count(), 0);
        assert_eq!(world.query_container().count(), 0);
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
            .insert_component_wound_list(live_named_agent, sample_wound_list())
            .unwrap();
        world
            .insert_component_wound_list(archived_named_agent, sample_wound_list())
            .unwrap();
        world
            .insert_component_drive_thresholds(live_named_agent, sample_drive_thresholds())
            .unwrap();
        world
            .insert_component_drive_thresholds(archived_named_agent, sample_drive_thresholds())
            .unwrap();
        world
            .insert_component_utility_profile(live_named_agent, sample_utility_profile())
            .unwrap();
        world
            .insert_component_utility_profile(archived_named_agent, sample_utility_profile())
            .unwrap();
        world
            .insert_component_blocked_intent_memory(
                live_named_agent,
                sample_blocked_intent_memory(),
            )
            .unwrap();
        world
            .insert_component_blocked_intent_memory(
                archived_named_agent,
                sample_blocked_intent_memory(),
            )
            .unwrap();
        world
            .insert_component_name(live_named_office, Name("Ledger Hall".to_string()))
            .unwrap();
        world.archive_entity(archived_named_agent, Tick(4)).unwrap();

        assert_eq!(world.entity_count(), 4);
        assert_eq!(world.count_with_name(), 2);
        assert_eq!(world.count_with_agent_data(), 1);
        assert_eq!(world.count_with_wound_list(), 1);
        assert_eq!(world.count_with_drive_thresholds(), 1);
        assert_eq!(world.count_with_utility_profile(), 1);
        assert_eq!(world.count_with_blocked_intent_memory(), 1);
        assert_eq!(world.count_with_item_lot(), 0);
        assert_eq!(world.count_with_unique_item(), 0);
        assert_eq!(world.count_with_container(), 0);
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

        assert_eq!(
            left.entities().collect::<Vec<_>>(),
            right.entities().collect::<Vec<_>>()
        );
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
                .map(|(entity, name, agent_data)| (
                    entity,
                    name.0.as_str(),
                    agent_data.control_source
                ))
                .collect::<Vec<_>>(),
            right
                .query_name_and_agent_data()
                .map(|(entity, name, agent_data)| (
                    entity,
                    name.0.as_str(),
                    agent_data.control_source
                ))
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
            assert_eq!(world.get_component_wound_list(place_id), None);
            assert_eq!(world.get_component_utility_profile(place_id), None);
            assert_eq!(world.get_component_blocked_intent_memory(place_id), None);
            assert_eq!(world.get_component_drive_thresholds(place_id), None);
            assert_eq!(world.get_component_item_lot(place_id), None);
            assert_eq!(world.get_component_unique_item(place_id), None);
            assert_eq!(world.get_component_container(place_id), None);
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
