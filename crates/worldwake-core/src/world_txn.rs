use crate::{
    build_observed_entity_snapshot,
    component_schema::with_component_schema_entries, ArchiveMutationSnapshot, CommodityKind,
    Container, ControlSource, EntityId, EntityKind, EventId, Permille, Quantity, ReservationId,
    Tick, TickRange, UniqueItemKind, World, WorldError,
};
use crate::{
    CauseRef, ComponentDelta, ComponentKind, ComponentValue, EntityDelta, EventLog, EventTag,
    EvidenceRef, PendingEvent, ProvenanceEntry, QuantityDelta, RelationDelta, RelationKind,
    RelationValue, ReservationDelta, StateDelta, VisibilitySpec, WitnessData,
};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;

pub struct WorldTxn<'w> {
    world: &'w mut World,
    staged_world: World,
    tick: Tick,
    cause: CauseRef,
    actor_id: Option<EntityId>,
    place_id: Option<EntityId>,
    tags: BTreeSet<EventTag>,
    target_ids: Vec<EntityId>,
    visibility: VisibilitySpec,
    witness_data: WitnessData,
    deltas: Vec<StateDelta>,
    evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct PlacementSnapshot {
    located_in: Option<EntityId>,
    in_transit: bool,
    contained_by: Option<EntityId>,
}

macro_rules! world_txn_component_setters {
    ($({ $component_ty:ty, $get_fn:ident, $remove_fn:ident, $insert_fn:ident, $set_fn:ident, $clear_fn:ident, $component_variant:ident })*) => {
        $(
            pub fn $set_fn(
                &mut self,
                entity: EntityId,
                component: $component_ty,
            ) -> Result<(), WorldError> {
                self.replace_simple_component(
                    entity,
                    component,
                    |world, entity| world.$get_fn(entity).cloned(),
                    |world, entity| world.$remove_fn(entity),
                    |world, entity, component: $component_ty| world.$insert_fn(entity, component),
                    ComponentKind::$component_variant,
                    ComponentValue::$component_variant,
                )
            }

            pub fn $clear_fn(&mut self, entity: EntityId) -> Result<(), WorldError> {
                self.clear_simple_component(
                    entity,
                    |world, entity| world.$get_fn(entity).cloned(),
                    |world, entity| world.$remove_fn(entity),
                    ComponentKind::$component_variant,
                    ComponentValue::$component_variant,
                )
            }
        )*
    };
}

impl<'w> WorldTxn<'w> {
    #[must_use]
    pub fn new(
        world: &'w mut World,
        tick: Tick,
        cause: CauseRef,
        actor_id: Option<EntityId>,
        place_id: Option<EntityId>,
        visibility: VisibilitySpec,
        witness_data: WitnessData,
    ) -> Self {
        Self {
            staged_world: world.clone(),
            world,
            tick,
            cause,
            actor_id,
            place_id,
            tags: BTreeSet::new(),
            target_ids: Vec::new(),
            visibility,
            witness_data,
            deltas: Vec::new(),
            evidence: Vec::new(),
        }
    }

    #[must_use]
    pub const fn tick(&self) -> Tick {
        self.tick
    }

    #[must_use]
    pub const fn cause(&self) -> CauseRef {
        self.cause
    }

    #[must_use]
    pub const fn actor_id(&self) -> Option<EntityId> {
        self.actor_id
    }

    #[must_use]
    pub const fn place_id(&self) -> Option<EntityId> {
        self.place_id
    }

    #[must_use]
    pub const fn visibility(&self) -> VisibilitySpec {
        self.visibility
    }

    #[must_use]
    pub const fn witness_data(&self) -> &WitnessData {
        &self.witness_data
    }

    #[must_use]
    pub fn target_ids(&self) -> &[EntityId] {
        &self.target_ids
    }

    #[must_use]
    pub const fn tags(&self) -> &BTreeSet<EventTag> {
        &self.tags
    }

    #[must_use]
    pub fn deltas(&self) -> &[StateDelta] {
        &self.deltas
    }

    #[must_use]
    pub fn evidence(&self) -> &[EvidenceRef] {
        &self.evidence
    }

    pub fn into_pending_event(self) -> PendingEvent {
        let observed_entities = self.capture_observed_entities();
        *self.world = self.staged_world;
        PendingEvent::new_with_evidence(
            self.tick,
            self.cause,
            self.actor_id,
            self.target_ids,
            self.evidence,
            self.place_id,
            self.deltas,
            self.visibility,
            self.witness_data,
            self.tags,
        )
        .with_observed_entities(observed_entities)
    }

    pub fn commit(self, event_log: &mut EventLog) -> EventId {
        event_log.emit(self.into_pending_event())
    }

    pub fn add_target(&mut self, target_id: EntityId) -> &mut Self {
        if !self.target_ids.contains(&target_id) {
            self.target_ids.push(target_id);
        }
        self
    }

    pub fn add_tag(&mut self, tag: EventTag) -> &mut Self {
        self.tags.insert(tag);
        self
    }

    pub fn add_evidence(&mut self, evidence: EvidenceRef) -> &mut Self {
        self.evidence.push(evidence);
        self.evidence.sort();
        self.evidence.dedup();
        self
    }

    pub fn extend_evidence(
        &mut self,
        evidence: impl IntoIterator<Item = EvidenceRef>,
    ) -> &mut Self {
        self.evidence.extend(evidence);
        self.evidence.sort();
        self.evidence.dedup();
        self
    }

    pub fn create_entity(&mut self, kind: EntityKind) -> EntityId {
        let entity = self.staged_world.create_entity(kind, self.tick);
        self.record_created_entity(entity, kind);
        entity
    }

    pub fn create_agent(
        &mut self,
        name: &str,
        control_source: ControlSource,
    ) -> Result<EntityId, WorldError> {
        let entity = self
            .staged_world
            .create_agent(name, control_source, self.tick)?;
        self.record_created_entity(entity, EntityKind::Agent);
        Ok(entity)
    }

    pub fn create_office(&mut self, name: &str) -> Result<EntityId, WorldError> {
        let entity = self.staged_world.create_office(name, self.tick)?;
        self.record_created_entity(entity, EntityKind::Office);
        Ok(entity)
    }

    pub fn create_faction(&mut self, name: &str) -> Result<EntityId, WorldError> {
        let entity = self.staged_world.create_faction(name, self.tick)?;
        self.record_created_entity(entity, EntityKind::Faction);
        Ok(entity)
    }

    pub fn create_item_lot(
        &mut self,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Result<EntityId, WorldError> {
        let entity = self
            .staged_world
            .create_item_lot(commodity, quantity, self.tick)?;
        self.record_created_entity(entity, EntityKind::ItemLot);
        Ok(entity)
    }

    pub fn create_unique_item(
        &mut self,
        kind: UniqueItemKind,
        name: Option<&str>,
        metadata: BTreeMap<String, String>,
    ) -> Result<EntityId, WorldError> {
        let entity = self
            .staged_world
            .create_unique_item(kind, name, metadata, self.tick)?;
        self.record_created_entity(entity, EntityKind::UniqueItem);
        Ok(entity)
    }

    pub fn create_container(&mut self, container: Container) -> Result<EntityId, WorldError> {
        let entity = self.staged_world.create_container(container, self.tick)?;
        self.record_created_entity(entity, EntityKind::Container);
        Ok(entity)
    }

    pub fn archive_entity(&mut self, entity: EntityId) -> Result<(), WorldError> {
        let snapshot = self.staged_world.archive_mutation_snapshot(entity)?;
        self.staged_world.archive_entity(entity, self.tick)?;
        self.push_archive_snapshot(snapshot);
        Ok(())
    }

    pub fn set_ground_location(
        &mut self,
        entity: EntityId,
        place: EntityId,
    ) -> Result<(), WorldError> {
        self.record_placement_operation(entity, |world| world.set_ground_location(entity, place))
    }

    pub fn put_into_container(
        &mut self,
        entity: EntityId,
        container: EntityId,
    ) -> Result<(), WorldError> {
        self.record_placement_operation(entity, |world| world.put_into_container(entity, container))
    }

    pub fn remove_from_container(&mut self, entity: EntityId) -> Result<(), WorldError> {
        self.record_placement_operation(entity, |world| world.remove_from_container(entity))
    }

    pub fn move_container_subtree(
        &mut self,
        container: EntityId,
        new_place: EntityId,
    ) -> Result<(), WorldError> {
        self.record_placement_operation(container, |world| {
            world.move_container_subtree(container, new_place)
        })
    }

    pub fn set_in_transit(&mut self, entity: EntityId) -> Result<(), WorldError> {
        self.record_placement_operation(entity, |world| world.set_in_transit(entity))
    }

    pub fn try_reserve(
        &mut self,
        entity: EntityId,
        reserver: EntityId,
        range: TickRange,
    ) -> Result<ReservationId, WorldError> {
        let reservation_id = self.staged_world.try_reserve(entity, reserver, range)?;
        let reservation = self
            .staged_world
            .reservation(reservation_id)
            .cloned()
            .expect("created reservation should be readable immediately");
        self.deltas
            .push(StateDelta::Reservation(ReservationDelta::Created {
                reservation,
            }));
        Ok(reservation_id)
    }

    pub fn release_reservation(&mut self, reservation_id: ReservationId) -> Result<(), WorldError> {
        let reservation = self
            .staged_world
            .reservation(reservation_id)
            .cloned()
            .ok_or_else(|| {
                WorldError::InvalidOperation(format!("reservation {reservation_id} does not exist"))
            })?;
        self.staged_world.release_reservation(reservation_id)?;
        self.deltas
            .push(StateDelta::Reservation(ReservationDelta::Released {
                reservation,
            }));
        Ok(())
    }

    pub fn split_lot(
        &mut self,
        lot_id: EntityId,
        amount: Quantity,
    ) -> Result<(EntityId, EntityId), WorldError> {
        let before = self.staged_world.get_component_item_lot(lot_id).cloned();
        let (source_id, new_lot_id) = self
            .staged_world
            .split_lot(lot_id, amount, self.tick, None)?;
        let before = before.expect("successful split must start from an item lot");
        let after = self
            .staged_world
            .get_component_item_lot(source_id)
            .cloned()
            .expect("split source lot should remain available");
        let new_lot = self
            .staged_world
            .get_component_item_lot(new_lot_id)
            .cloned()
            .expect("split should create a readable child lot");

        self.deltas.push(StateDelta::Component(ComponentDelta::Set {
            entity: source_id,
            component_kind: ComponentKind::ItemLot,
            before: Some(ComponentValue::ItemLot(before.clone())),
            after: ComponentValue::ItemLot(after.clone()),
        }));
        self.deltas
            .push(StateDelta::Quantity(QuantityDelta::Changed {
                entity: source_id,
                commodity: before.commodity,
                before: before.quantity,
                after: after.quantity,
            }));
        self.record_created_entity(new_lot_id, EntityKind::ItemLot);
        self.deltas
            .push(StateDelta::Quantity(QuantityDelta::Changed {
                entity: new_lot_id,
                commodity: new_lot.commodity,
                before: Quantity(0),
                after: new_lot.quantity,
            }));

        Ok((source_id, new_lot_id))
    }

    pub fn merge_lots(
        &mut self,
        target_id: EntityId,
        source_id: EntityId,
    ) -> Result<EntityId, WorldError> {
        let target_before = self.staged_world.get_component_item_lot(target_id).cloned();
        let source_before = self.staged_world.get_component_item_lot(source_id).cloned();
        let source_snapshot = self.staged_world.archive_mutation_snapshot(source_id)?;
        let merged_id = self
            .staged_world
            .merge_lots(target_id, source_id, self.tick, None)?;
        let target_before = target_before.expect("successful merge must start from a target lot");
        let source_before = source_before.expect("successful merge must start from a source lot");
        let target_after = self
            .staged_world
            .get_component_item_lot(merged_id)
            .cloned()
            .expect("merge target should remain available");

        self.deltas.push(StateDelta::Component(ComponentDelta::Set {
            entity: merged_id,
            component_kind: ComponentKind::ItemLot,
            before: Some(ComponentValue::ItemLot(target_before.clone())),
            after: ComponentValue::ItemLot(target_after.clone()),
        }));
        self.deltas
            .push(StateDelta::Quantity(QuantityDelta::Changed {
                entity: merged_id,
                commodity: target_before.commodity,
                before: target_before.quantity,
                after: target_after.quantity,
            }));
        self.deltas
            .push(StateDelta::Quantity(QuantityDelta::Changed {
                entity: source_id,
                commodity: source_before.commodity,
                before: source_before.quantity,
                after: Quantity(0),
            }));
        self.push_archive_snapshot(source_snapshot);

        Ok(merged_id)
    }

    pub fn append_lot_provenance(
        &mut self,
        lot_id: EntityId,
        entry: ProvenanceEntry,
    ) -> Result<(), WorldError> {
        let before = self
            .staged_world
            .get_component_item_lot(lot_id)
            .cloned()
            .ok_or(WorldError::ComponentNotFound {
                entity: lot_id,
                component_type: "ItemLot",
            })?;
        let lot = self.staged_world.get_component_item_lot_mut(lot_id).ok_or(
            WorldError::ComponentNotFound {
                entity: lot_id,
                component_type: "ItemLot",
            },
        )?;
        lot.provenance.push(entry);
        let after = lot.clone();
        self.deltas.push(StateDelta::Component(ComponentDelta::Set {
            entity: lot_id,
            component_kind: ComponentKind::ItemLot,
            before: Some(ComponentValue::ItemLot(before)),
            after: ComponentValue::ItemLot(after),
        }));
        Ok(())
    }

    pub fn set_owner(&mut self, entity: EntityId, owner: EntityId) -> Result<(), WorldError> {
        let before = self.staged_world.owner_of(entity);
        self.staged_world.set_owner(entity, owner)?;
        let after = self.staged_world.owner_of(entity);
        self.push_single_target_relation_delta(
            entity,
            before,
            after,
            RelationKind::OwnedBy,
            |entity, owner| RelationValue::OwnedBy { entity, owner },
        );
        Ok(())
    }

    pub fn clear_owner(&mut self, entity: EntityId) -> Result<(), WorldError> {
        let before = self.staged_world.owner_of(entity);
        self.staged_world.clear_owner(entity)?;
        let after = self.staged_world.owner_of(entity);
        self.push_single_target_relation_delta(
            entity,
            before,
            after,
            RelationKind::OwnedBy,
            |entity, owner| RelationValue::OwnedBy { entity, owner },
        );
        Ok(())
    }

    pub fn set_possessor(&mut self, entity: EntityId, holder: EntityId) -> Result<(), WorldError> {
        let before = self.staged_world.possessor_of(entity);
        self.staged_world.set_possessor(entity, holder)?;
        let after = self.staged_world.possessor_of(entity);
        self.push_single_target_relation_delta(
            entity,
            before,
            after,
            RelationKind::PossessedBy,
            |entity, holder| RelationValue::PossessedBy { entity, holder },
        );
        Ok(())
    }

    pub fn clear_possessor(&mut self, entity: EntityId) -> Result<(), WorldError> {
        let before = self.staged_world.possessor_of(entity);
        self.staged_world.clear_possessor(entity)?;
        let after = self.staged_world.possessor_of(entity);
        self.push_single_target_relation_delta(
            entity,
            before,
            after,
            RelationKind::PossessedBy,
            |entity, holder| RelationValue::PossessedBy { entity, holder },
        );
        Ok(())
    }

    pub fn add_member(&mut self, member: EntityId, faction: EntityId) -> Result<(), WorldError> {
        let before = self.staged_world.factions_of(member).contains(&faction);
        self.staged_world.add_member(member, faction)?;
        let after = self.staged_world.factions_of(member).contains(&faction);
        self.push_presence_relation_delta(
            before,
            after,
            RelationKind::MemberOf,
            RelationValue::MemberOf { member, faction },
        );
        Ok(())
    }

    pub fn remove_member(&mut self, member: EntityId, faction: EntityId) -> Result<(), WorldError> {
        let before = self.staged_world.factions_of(member).contains(&faction);
        self.staged_world.remove_member(member, faction)?;
        let after = self.staged_world.factions_of(member).contains(&faction);
        self.push_presence_relation_delta(
            before,
            after,
            RelationKind::MemberOf,
            RelationValue::MemberOf { member, faction },
        );
        Ok(())
    }

    /// Sets loyalty through the event-sourced transaction boundary.
    ///
    /// Loyalty changes reuse the shared weighted relation delta semantics: unchanged values emit
    /// no delta, new values emit `Added`, and updates emit `Removed(old)` then `Added(new)`.
    pub fn set_loyalty(
        &mut self,
        subject: EntityId,
        target: EntityId,
        strength: Permille,
    ) -> Result<(), WorldError> {
        let before = self.staged_world.loyalty_to(subject, target);
        self.staged_world.set_loyalty(subject, target, strength)?;
        let after = self.staged_world.loyalty_to(subject, target);
        self.push_weighted_relation_delta(subject, target, before, after);
        Ok(())
    }

    /// Clears loyalty through the event-sourced transaction boundary.
    ///
    /// This records the canonical loyalty removal delta via the shared weighted relation helper
    /// rather than introducing loyalty-specific event plumbing.
    pub fn clear_loyalty(&mut self, subject: EntityId, target: EntityId) -> Result<(), WorldError> {
        let before = self.staged_world.loyalty_to(subject, target);
        self.staged_world.clear_loyalty(subject, target)?;
        let after = self.staged_world.loyalty_to(subject, target);
        self.push_weighted_relation_delta(subject, target, before, after);
        Ok(())
    }

    pub fn assign_office(&mut self, office: EntityId, holder: EntityId) -> Result<(), WorldError> {
        let before = self.staged_world.office_holder(office);
        self.staged_world.assign_office(office, holder)?;
        let after = self.staged_world.office_holder(office);
        self.push_single_target_relation_delta(
            office,
            before,
            after,
            RelationKind::OfficeHolder,
            |office, holder| RelationValue::OfficeHolder { office, holder },
        );
        Ok(())
    }

    pub fn vacate_office(&mut self, office: EntityId) -> Result<(), WorldError> {
        let before = self.staged_world.office_holder(office);
        self.staged_world.vacate_office(office)?;
        let after = self.staged_world.office_holder(office);
        self.push_single_target_relation_delta(
            office,
            before,
            after,
            RelationKind::OfficeHolder,
            |office, holder| RelationValue::OfficeHolder { office, holder },
        );
        Ok(())
    }

    pub fn add_hostility(&mut self, subject: EntityId, target: EntityId) -> Result<(), WorldError> {
        let before = self
            .staged_world
            .hostile_targets_of(subject)
            .contains(&target);
        self.staged_world.add_hostility(subject, target)?;
        let after = self
            .staged_world
            .hostile_targets_of(subject)
            .contains(&target);
        self.push_presence_relation_delta(
            before,
            after,
            RelationKind::HostileTo,
            RelationValue::HostileTo { subject, target },
        );
        Ok(())
    }

    pub fn remove_hostility(
        &mut self,
        subject: EntityId,
        target: EntityId,
    ) -> Result<(), WorldError> {
        let before = self
            .staged_world
            .hostile_targets_of(subject)
            .contains(&target);
        self.staged_world.remove_hostility(subject, target)?;
        let after = self
            .staged_world
            .hostile_targets_of(subject)
            .contains(&target);
        self.push_presence_relation_delta(
            before,
            after,
            RelationKind::HostileTo,
            RelationValue::HostileTo { subject, target },
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn replace_simple_component<T, Get, Remove, Insert, Wrap>(
        &mut self,
        entity: EntityId,
        component: T,
        get: Get,
        remove: Remove,
        insert: Insert,
        component_kind: ComponentKind,
        wrap: Wrap,
    ) -> Result<(), WorldError>
    where
        T: Clone + Eq,
        Get: Fn(&World, EntityId) -> Option<T>,
        Remove: Fn(&mut World, EntityId) -> Result<Option<T>, WorldError>,
        Insert: Fn(&mut World, EntityId, T) -> Result<(), WorldError>,
        Wrap: Fn(T) -> ComponentValue,
    {
        let before = get(&self.staged_world, entity);
        if before.as_ref() == Some(&component) {
            return Ok(());
        }

        if before.is_some() {
            let _ = remove(&mut self.staged_world, entity)?;
        }
        insert(&mut self.staged_world, entity, component.clone())?;
        self.deltas.push(StateDelta::Component(ComponentDelta::Set {
            entity,
            component_kind,
            before: before.map(&wrap),
            after: wrap(component),
        }));
        Ok(())
    }

    fn clear_simple_component<T, Get, Remove, Wrap>(
        &mut self,
        entity: EntityId,
        get: Get,
        remove: Remove,
        component_kind: ComponentKind,
        wrap: Wrap,
    ) -> Result<(), WorldError>
    where
        T: Clone,
        Get: Fn(&World, EntityId) -> Option<T>,
        Remove: Fn(&mut World, EntityId) -> Result<Option<T>, WorldError>,
        Wrap: Fn(T) -> ComponentValue,
    {
        let Some(before) = get(&self.staged_world, entity) else {
            return Ok(());
        };

        let removed = remove(&mut self.staged_world, entity)?
            .expect("component read before removal must still exist during clear");
        self.deltas
            .push(StateDelta::Component(ComponentDelta::Removed {
                entity,
                component_kind,
                before: wrap(removed),
            }));
        debug_assert!(get(&self.staged_world, entity).is_none());
        debug_assert!(matches!(wrap(before).kind(), kind if kind == component_kind));
        Ok(())
    }

    with_component_schema_entries!(
        select_txn_simple_set_components,
        world_txn_component_setters
    );

    fn record_created_entity(&mut self, entity: EntityId, kind: EntityKind) {
        self.deltas
            .push(StateDelta::Entity(EntityDelta::Created { entity, kind }));
        self.deltas
            .extend(self.component_deltas_after_create(entity));
        if self.staged_world.is_in_transit(entity) {
            self.deltas.push(StateDelta::Relation(RelationDelta::Added {
                relation_kind: RelationKind::InTransit,
                relation: RelationValue::InTransit { entity },
            }));
        }
    }

    fn component_deltas_after_create(&self, entity: EntityId) -> Vec<StateDelta> {
        let mut deltas = Vec::new();

        for value in self.staged_world.component_values(entity) {
            deltas.push(StateDelta::Component(ComponentDelta::Set {
                entity,
                component_kind: value.kind(),
                before: None,
                after: value,
            }));
        }

        deltas
    }

    fn record_placement_operation<F>(
        &mut self,
        entity: EntityId,
        mutate: F,
    ) -> Result<(), WorldError>
    where
        F: FnOnce(&mut World) -> Result<(), WorldError>,
    {
        let before_scope = self.placement_scope(entity);
        let before_snapshots = before_scope
            .iter()
            .copied()
            .map(|id| (id, self.placement_snapshot(id)))
            .collect::<BTreeMap<_, _>>();

        mutate(&mut self.staged_world)?;

        let after_scope = self.placement_scope(entity);
        let mut ordered_scope = before_scope;
        for id in after_scope {
            if !ordered_scope.contains(&id) {
                ordered_scope.push(id);
            }
        }

        for id in ordered_scope {
            let before = before_snapshots.get(&id).copied().unwrap_or_default();
            let after = self.placement_snapshot(id);
            self.push_placement_delta_diff(id, before, after);
        }

        Ok(())
    }

    fn placement_scope(&self, entity: EntityId) -> Vec<EntityId> {
        let mut scope = vec![entity];
        if self.staged_world.get_component_container(entity).is_some() {
            scope.extend(self.staged_world.recursive_contents_of(entity));
        }
        scope
    }

    fn placement_snapshot(&self, entity: EntityId) -> PlacementSnapshot {
        PlacementSnapshot {
            located_in: self.staged_world.effective_place(entity),
            in_transit: self.staged_world.is_in_transit(entity),
            contained_by: self.staged_world.direct_container(entity),
        }
    }

    fn capture_observed_entities(&self) -> BTreeMap<EntityId, crate::ObservedEntitySnapshot> {
        self.observed_entity_ids()
            .into_iter()
            .filter_map(|entity| {
                build_observed_entity_snapshot(&self.staged_world, entity)
                    .map(|snapshot| (entity, snapshot))
            })
            .collect()
    }

    fn observed_entity_ids(&self) -> BTreeSet<EntityId> {
        let mut entities = BTreeSet::new();
        if let Some(actor) = self.actor_id {
            entities.insert(actor);
        }
        entities.extend(self.target_ids.iter().copied());
        entities.extend(self.evidence.iter().flat_map(observed_evidence_entities));
        for delta in &self.deltas {
            match delta {
                StateDelta::Entity(entity_delta) => match entity_delta {
                    EntityDelta::Created { entity, .. } | EntityDelta::Archived { entity, .. } => {
                        entities.insert(*entity);
                    }
                },
                StateDelta::Component(component_delta) => match component_delta {
                    ComponentDelta::Set { entity, .. } | ComponentDelta::Removed { entity, .. } => {
                        entities.insert(*entity);
                    }
                },
                StateDelta::Relation(relation_delta) => {
                    entities.extend(observed_relation_entities(relation_delta));
                }
                StateDelta::Quantity(quantity_delta) => match quantity_delta {
                    QuantityDelta::Changed { entity, .. } => {
                        entities.insert(*entity);
                    }
                },
                StateDelta::Reservation(reservation_delta) => match reservation_delta {
                    ReservationDelta::Created { reservation }
                    | ReservationDelta::Released { reservation } => {
                        entities.insert(reservation.entity);
                        entities.insert(reservation.reserver);
                    }
                },
            }
        }
        entities
    }

    fn push_placement_delta_diff(
        &mut self,
        entity: EntityId,
        before: PlacementSnapshot,
        after: PlacementSnapshot,
    ) {
        if before.contained_by != after.contained_by {
            if let Some(container) = before.contained_by {
                self.deltas
                    .push(StateDelta::Relation(RelationDelta::Removed {
                        relation_kind: RelationKind::ContainedBy,
                        relation: RelationValue::ContainedBy { entity, container },
                    }));
            }
            if let Some(container) = after.contained_by {
                self.deltas.push(StateDelta::Relation(RelationDelta::Added {
                    relation_kind: RelationKind::ContainedBy,
                    relation: RelationValue::ContainedBy { entity, container },
                }));
            }
        }

        if before.in_transit && !after.in_transit {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::InTransit,
                    relation: RelationValue::InTransit { entity },
                }));
        }

        if before.located_in != after.located_in {
            if let Some(place) = before.located_in {
                self.deltas
                    .push(StateDelta::Relation(RelationDelta::Removed {
                        relation_kind: RelationKind::LocatedIn,
                        relation: RelationValue::LocatedIn { entity, place },
                    }));
            }
            if let Some(place) = after.located_in {
                self.deltas.push(StateDelta::Relation(RelationDelta::Added {
                    relation_kind: RelationKind::LocatedIn,
                    relation: RelationValue::LocatedIn { entity, place },
                }));
            }
        }

        if !before.in_transit && after.in_transit {
            self.deltas.push(StateDelta::Relation(RelationDelta::Added {
                relation_kind: RelationKind::InTransit,
                relation: RelationValue::InTransit { entity },
            }));
        }
    }

    fn push_single_target_relation_delta<F>(
        &mut self,
        subject: EntityId,
        before: Option<EntityId>,
        after: Option<EntityId>,
        relation_kind: RelationKind,
        relation: F,
    ) where
        F: Fn(EntityId, EntityId) -> RelationValue,
    {
        if before == after {
            return;
        }

        if let Some(target) = before {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind,
                    relation: relation(subject, target),
                }));
        }
        if let Some(target) = after {
            self.deltas.push(StateDelta::Relation(RelationDelta::Added {
                relation_kind,
                relation: relation(subject, target),
            }));
        }
    }

    fn push_presence_relation_delta(
        &mut self,
        before: bool,
        after: bool,
        relation_kind: RelationKind,
        relation: RelationValue,
    ) {
        if before == after {
            return;
        }

        let delta = if after {
            RelationDelta::Added {
                relation_kind,
                relation,
            }
        } else {
            RelationDelta::Removed {
                relation_kind,
                relation,
            }
        };
        self.deltas.push(StateDelta::Relation(delta));
    }

    fn push_weighted_relation_delta(
        &mut self,
        subject: EntityId,
        target: EntityId,
        before: Option<Permille>,
        after: Option<Permille>,
    ) {
        if before == after {
            return;
        }

        if let Some(strength) = before {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::LoyalTo,
                    relation: RelationValue::LoyalTo {
                        subject,
                        target,
                        strength,
                    },
                }));
        }
        if let Some(strength) = after {
            self.deltas.push(StateDelta::Relation(RelationDelta::Added {
                relation_kind: RelationKind::LoyalTo,
                relation: RelationValue::LoyalTo {
                    subject,
                    target,
                    strength,
                },
            }));
        }
    }

    fn push_archive_snapshot(&mut self, snapshot: ArchiveMutationSnapshot) {
        self.deltas.push(StateDelta::Entity(EntityDelta::Archived {
            entity: snapshot.entity,
            kind: snapshot.kind,
        }));

        self.push_archive_removed_located_in(snapshot.entity, snapshot.located_in);
        self.push_archive_removed_in_transit(snapshot.entity, snapshot.in_transit);
        self.push_archive_removed_contained_by(snapshot.entity, snapshot.contained_by);
        self.push_archive_removed_contained_dependents(snapshot.entity, &snapshot.contents_of);
        self.push_archive_removed_possessed_by(snapshot.entity, snapshot.possessed_by);
        self.push_archive_removed_possession_dependents(snapshot.entity, &snapshot.possessions_of);
        self.push_archive_removed_owned_by(snapshot.entity, snapshot.owned_by);
        self.push_archive_removed_owned_dependents(snapshot.entity, &snapshot.property_of);
        self.push_archive_removed_memberships(snapshot.entity, &snapshot.member_of);
        self.push_archive_removed_members(snapshot.entity, &snapshot.members_of);
        self.push_archive_removed_loyalty_targets(snapshot.entity, &snapshot.loyal_to);
        self.push_archive_removed_loyalty_subjects(snapshot.entity, &snapshot.loyalty_from);
        self.push_archive_removed_office_holder(snapshot.entity, snapshot.office_holder);
        self.push_archive_removed_offices_held(snapshot.entity, &snapshot.offices_held);
        self.push_archive_removed_hostility_targets(snapshot.entity, &snapshot.hostile_to);
        self.push_archive_removed_hostility_subjects(snapshot.entity, &snapshot.hostility_from);
        for reservation in snapshot.released_reservations {
            self.deltas
                .push(StateDelta::Reservation(ReservationDelta::Released {
                    reservation,
                }));
        }
    }

    fn push_archive_removed_located_in(&mut self, entity: EntityId, place: Option<EntityId>) {
        if let Some(place) = place {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::LocatedIn,
                    relation: RelationValue::LocatedIn { entity, place },
                }));
        }
    }

    fn push_archive_removed_in_transit(&mut self, entity: EntityId, in_transit: bool) {
        if in_transit {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::InTransit,
                    relation: RelationValue::InTransit { entity },
                }));
        }
    }

    fn push_archive_removed_contained_by(&mut self, entity: EntityId, container: Option<EntityId>) {
        if let Some(container) = container {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::ContainedBy,
                    relation: RelationValue::ContainedBy { entity, container },
                }));
        }
    }

    fn push_archive_removed_contained_dependents(
        &mut self,
        container: EntityId,
        dependents: &[EntityId],
    ) {
        for entity in dependents {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::ContainedBy,
                    relation: RelationValue::ContainedBy {
                        entity: *entity,
                        container,
                    },
                }));
        }
    }

    fn push_archive_removed_possessed_by(&mut self, entity: EntityId, holder: Option<EntityId>) {
        if let Some(holder) = holder {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::PossessedBy,
                    relation: RelationValue::PossessedBy { entity, holder },
                }));
        }
    }

    fn push_archive_removed_possession_dependents(
        &mut self,
        holder: EntityId,
        dependents: &[EntityId],
    ) {
        for entity in dependents {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::PossessedBy,
                    relation: RelationValue::PossessedBy {
                        entity: *entity,
                        holder,
                    },
                }));
        }
    }

    fn push_archive_removed_owned_by(&mut self, entity: EntityId, owner: Option<EntityId>) {
        if let Some(owner) = owner {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::OwnedBy,
                    relation: RelationValue::OwnedBy { entity, owner },
                }));
        }
    }

    fn push_archive_removed_owned_dependents(&mut self, owner: EntityId, dependents: &[EntityId]) {
        for entity in dependents {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::OwnedBy,
                    relation: RelationValue::OwnedBy {
                        entity: *entity,
                        owner,
                    },
                }));
        }
    }

    fn push_archive_removed_memberships(&mut self, member: EntityId, factions: &[EntityId]) {
        for faction in factions {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::MemberOf,
                    relation: RelationValue::MemberOf {
                        member,
                        faction: *faction,
                    },
                }));
        }
    }

    fn push_archive_removed_members(&mut self, faction: EntityId, members: &[EntityId]) {
        for member in members {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::MemberOf,
                    relation: RelationValue::MemberOf {
                        member: *member,
                        faction,
                    },
                }));
        }
    }

    fn push_archive_removed_loyalty_targets(
        &mut self,
        subject: EntityId,
        targets: &[(EntityId, Permille)],
    ) {
        for (target, strength) in targets {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::LoyalTo,
                    relation: RelationValue::LoyalTo {
                        subject,
                        target: *target,
                        strength: *strength,
                    },
                }));
        }
    }

    fn push_archive_removed_loyalty_subjects(
        &mut self,
        target: EntityId,
        subjects: &[(EntityId, Permille)],
    ) {
        for (subject, strength) in subjects {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::LoyalTo,
                    relation: RelationValue::LoyalTo {
                        subject: *subject,
                        target,
                        strength: *strength,
                    },
                }));
        }
    }

    fn push_archive_removed_office_holder(&mut self, office: EntityId, holder: Option<EntityId>) {
        if let Some(holder) = holder {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::OfficeHolder,
                    relation: RelationValue::OfficeHolder { office, holder },
                }));
        }
    }

    fn push_archive_removed_offices_held(&mut self, holder: EntityId, offices: &[EntityId]) {
        for office in offices {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::OfficeHolder,
                    relation: RelationValue::OfficeHolder {
                        office: *office,
                        holder,
                    },
                }));
        }
    }

    fn push_archive_removed_hostility_targets(&mut self, subject: EntityId, targets: &[EntityId]) {
        for target in targets {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::HostileTo,
                    relation: RelationValue::HostileTo {
                        subject,
                        target: *target,
                    },
                }));
        }
    }

    fn push_archive_removed_hostility_subjects(&mut self, target: EntityId, subjects: &[EntityId]) {
        for subject in subjects {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::HostileTo,
                    relation: RelationValue::HostileTo {
                        subject: *subject,
                        target,
                    },
                }));
        }
    }
}

impl Deref for WorldTxn<'_> {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        &self.staged_world
    }
}

fn observed_relation_entities(relation_delta: &RelationDelta) -> BTreeSet<EntityId> {
    use RelationDelta::{Added, Removed};

    let relation = match relation_delta {
        Added { relation, .. } | Removed { relation, .. } => relation,
    };

    match relation {
        RelationValue::LocatedIn { entity, place } => BTreeSet::from([*entity, *place]),
        RelationValue::InTransit { entity } => BTreeSet::from([*entity]),
        RelationValue::ContainedBy { entity, container } => BTreeSet::from([*entity, *container]),
        RelationValue::PossessedBy { entity, holder } => BTreeSet::from([*entity, *holder]),
        RelationValue::OwnedBy { entity, owner } => BTreeSet::from([*entity, *owner]),
        RelationValue::MemberOf { member, faction } => BTreeSet::from([*member, *faction]),
        RelationValue::LoyalTo {
            subject, target, ..
        }
        | RelationValue::HostileTo { subject, target } => BTreeSet::from([*subject, *target]),
        RelationValue::OfficeHolder { office, holder } => BTreeSet::from([*office, *holder]),
    }
}

fn observed_evidence_entities(evidence: &EvidenceRef) -> BTreeSet<EntityId> {
    match evidence {
        EvidenceRef::Wound { entity, .. } => BTreeSet::from([*entity]),
        EvidenceRef::Mismatch {
            observer, subject, ..
        } => BTreeSet::from([*observer, *subject]),
    }
}

#[cfg(test)]
mod tests {
    use super::WorldTxn;
    use crate::{
        component_schema::with_component_schema_entries,
        test_utils::{
            sample_blocked_intent_memory, sample_demand_memory, sample_merchandise_profile,
            sample_substitute_preferences, sample_trade_disposition_profile,
            sample_travel_disposition_profile, sample_utility_profile,
        },
        AgentBeliefStore, BelievedEntityState, BlockedIntentMemory, DemandMemory,
        MerchandiseProfile, PerceptionProfile, PerceptionSource, SubstitutePreferences,
        TellProfile,
        TradeDispositionProfile, TravelDispositionProfile, UtilityProfile,
    };
    use crate::{
        CarryCapacity, CauseRef, ComponentDelta, ComponentKind, ComponentValue, EntityDelta,
        EventLog, EventTag, EvidenceRef, InTransitOnEdge, KnownRecipes, MismatchKind,
        QuantityDelta, RelationDelta, RelationKind, RelationValue, ReservationDelta, StateDelta,
        TravelEdgeId, VisibilitySpec, WitnessData, WoundId,
    };
    use crate::{
        CommodityKind, Container, ControlSource, DeprivationExposure, EntityId, EntityKind,
        HomeostaticNeeds, LoadUnits, Name, Permille, Place, PlaceTag, Quantity, ReservationId,
        ReservationRecord, ResourceSource, Tick, TickRange, Topology, UniqueItemKind, World,
        WorldError,
    };
    use std::collections::{BTreeMap, BTreeSet};

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

    fn sample_resource_source() -> ResourceSource {
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(4),
            max_quantity: Quantity(9),
            regeneration_ticks_per_unit: Some(std::num::NonZeroU32::new(6).unwrap()),
            last_regeneration_tick: Some(Tick(3)),
        }
    }

    struct ArchiveTeardownFixture {
        archived: EntityId,
        owner: EntityId,
        holder: EntityId,
        faction: EntityId,
        loyal_target: EntityId,
        hostile_target: EntityId,
        reserved_target: EntityId,
        loyal_strength: Permille,
        first_reservation: ReservationId,
        first_range: TickRange,
        second_reservation: ReservationId,
        second_range: TickRange,
    }

    fn new_txn(world: &mut World) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(9),
            CauseRef::Bootstrap,
            Some(entity(11)),
            Some(entity(5)),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        )
    }

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    macro_rules! define_txn_simple_set_component_kinds {
        ($({ $component_ty:ty, $get_fn:ident, $remove_fn:ident, $insert_fn:ident, $set_fn:ident, $clear_fn:ident, $component_variant:ident })*) => {
            const TXN_SIMPLE_SET_COMPONENT_KINDS: &[ComponentKind] = &[
                $(ComponentKind::$component_variant,)*
            ];
        };
    }

    with_component_schema_entries!(
        select_txn_simple_set_components,
        define_txn_simple_set_component_kinds
    );

    fn archive_teardown_fixture(world: &mut World) -> ArchiveTeardownFixture {
        let archived = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let owner = world.create_faction("River Pact", Tick(2)).unwrap();
        let holder = world
            .create_agent("Bram", ControlSource::Ai, Tick(3))
            .unwrap();
        let faction = world.create_faction("Granary Guild", Tick(4)).unwrap();
        let loyal_target = world.create_office("Chair", Tick(5)).unwrap();
        let hostile_target = world.create_faction("Watch", Tick(6)).unwrap();
        let reserved_target = world
            .create_item_lot(CommodityKind::Bread, Quantity(2), Tick(8))
            .unwrap();
        let loyal_strength = Permille::new(650).unwrap();
        let first_range = TickRange::new(Tick(10), Tick(12)).unwrap();
        let second_range = TickRange::new(Tick(12), Tick(14)).unwrap();

        world.set_owner(archived, owner).unwrap();
        world.set_possessor(archived, holder).unwrap();
        world.add_member(archived, faction).unwrap();
        world
            .set_loyalty(archived, loyal_target, loyal_strength)
            .unwrap();
        world.add_hostility(archived, hostile_target).unwrap();
        let first_reservation = world.try_reserve(archived, holder, first_range).unwrap();
        let second_reservation = world
            .try_reserve(reserved_target, archived, second_range)
            .unwrap();

        ArchiveTeardownFixture {
            archived,
            owner,
            holder,
            faction,
            loyal_target,
            hostile_target,
            reserved_target,
            loyal_strength,
            first_reservation,
            first_range,
            second_reservation,
            second_range,
        }
    }

    #[test]
    fn new_constructs_with_required_metadata() {
        let mut world = World::new(test_topology()).unwrap();
        let txn = new_txn(&mut world);

        assert_eq!(txn.tick(), Tick(9));
        assert_eq!(txn.cause(), CauseRef::Bootstrap);
        assert_eq!(txn.actor_id(), Some(entity(11)));
        assert_eq!(txn.place_id(), Some(entity(5)));
        assert_eq!(txn.visibility(), VisibilitySpec::SamePlace);
        assert_eq!(txn.witness_data(), &WitnessData::default());
        assert!(txn.target_ids().is_empty());
        assert!(txn.tags().is_empty());
        assert!(txn.deltas().is_empty());
    }

    #[test]
    fn dropping_uncommitted_transaction_leaves_authoritative_world_unchanged() {
        let mut world = World::new(test_topology()).unwrap();

        {
            let mut txn = new_txn(&mut world);
            let _ = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            assert_eq!(txn.query_agent_data().count(), 1);
        }

        assert_eq!(world.query_agent_data().count(), 0);
    }

    #[test]
    fn commit_publishes_staged_world_to_authoritative_world() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = {
            let mut txn = new_txn(&mut world);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            commit_txn(txn);
            agent
        };

        assert!(world.is_alive(agent));
        assert_eq!(world.query_agent_data().count(), 1);
    }

    #[test]
    fn create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through() {
        let mut world = World::new(test_topology()).unwrap();
        let mut txn = new_txn(&mut world);

        let agent = txn.create_agent("Aster", ControlSource::Human).unwrap();

        assert_eq!(
            txn.get_component_name(agent),
            Some(&Name("Aster".to_string()))
        );
        assert_eq!(
            txn.deltas(),
            &[
                StateDelta::Entity(EntityDelta::Created {
                    entity: agent,
                    kind: EntityKind::Agent,
                }),
                StateDelta::Component(ComponentDelta::Set {
                    entity: agent,
                    component_kind: ComponentKind::Name,
                    before: None,
                    after: ComponentValue::Name(Name("Aster".to_string())),
                }),
                StateDelta::Component(ComponentDelta::Set {
                    entity: agent,
                    component_kind: ComponentKind::AgentData,
                    before: None,
                    after: ComponentValue::AgentData(crate::AgentData {
                        control_source: ControlSource::Human,
                    }),
                }),
                StateDelta::Component(ComponentDelta::Set {
                    entity: agent,
                    component_kind: ComponentKind::AgentBeliefStore,
                    before: None,
                    after: ComponentValue::AgentBeliefStore(AgentBeliefStore::new()),
                }),
                StateDelta::Component(ComponentDelta::Set {
                    entity: agent,
                    component_kind: ComponentKind::PerceptionProfile,
                    before: None,
                    after: ComponentValue::PerceptionProfile(PerceptionProfile::default()),
                }),
                StateDelta::Component(ComponentDelta::Set {
                    entity: agent,
                    component_kind: ComponentKind::TellProfile,
                    before: None,
                    after: ComponentValue::TellProfile(TellProfile::default()),
                }),
                StateDelta::Relation(RelationDelta::Added {
                    relation_kind: RelationKind::InTransit,
                    relation: RelationValue::InTransit { entity: agent },
                }),
            ]
        );
    }

    #[test]
    fn create_unique_item_records_typed_component_delta() {
        let mut world = World::new(test_topology()).unwrap();
        let mut txn = new_txn(&mut world);

        let unique_item = txn
            .create_unique_item(
                UniqueItemKind::Artifact,
                Some("Seal"),
                BTreeMap::from([("origin".to_string(), "vault".to_string())]),
            )
            .unwrap();

        assert!(matches!(
            txn.deltas()[1],
            StateDelta::Component(ComponentDelta::Set {
                entity,
                component_kind: ComponentKind::UniqueItem,
                ..
            }) if entity == unique_item
        ));
    }

    #[test]
    fn txn_simple_set_components_match_manifest_projection() {
        let expected: Vec<_> = ComponentKind::ALL
            .into_iter()
            .filter(|kind| {
                !matches!(
                    kind,
                    ComponentKind::ItemLot | ComponentKind::UniqueItem | ComponentKind::Container
                )
            })
            .collect();

        assert_eq!(TXN_SIMPLE_SET_COMPONENT_KINDS, expected.as_slice());
    }

    #[test]
    fn txn_simple_setter_methods_exist_for_every_selected_component() {
        macro_rules! assert_world_txn_setters_exist {
            ($({ $component_ty:ty, $get_fn:ident, $remove_fn:ident, $insert_fn:ident, $set_fn:ident, $clear_fn:ident, $component_variant:ident })*) => {
                $(
                    let _ = WorldTxn::$set_fn;
                    let _ = WorldTxn::$clear_fn;
                )*
            };
        }

        with_component_schema_entries!(
            select_txn_simple_set_components,
            assert_world_txn_setters_exist
        );
    }

    #[test]
    fn set_ground_location_records_canonical_relation_deltas() {
        let mut world = World::new(test_topology()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Bread, Quantity(2), Tick(1))
            .unwrap();
        let mut txn = new_txn(&mut world);

        txn.set_ground_location(item, entity(5)).unwrap();

        assert_eq!(
            txn.deltas(),
            &[
                StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::InTransit,
                    relation: RelationValue::InTransit { entity: item },
                }),
                StateDelta::Relation(RelationDelta::Added {
                    relation_kind: RelationKind::LocatedIn,
                    relation: RelationValue::LocatedIn {
                        entity: item,
                        place: entity(5),
                    },
                }),
            ]
        );
    }

    #[test]
    fn move_container_subtree_records_descendant_location_updates() {
        let mut world = World::new(test_topology()).unwrap();
        let root = world.create_container(open_container(20), Tick(1)).unwrap();
        let inner = world.create_container(open_container(10), Tick(2)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Coin, Quantity(1), Tick(3))
            .unwrap();
        world.set_ground_location(root, entity(2)).unwrap();
        world.put_into_container(inner, root).unwrap();
        world.put_into_container(item, inner).unwrap();

        let mut txn = new_txn(&mut world);
        txn.move_container_subtree(root, entity(5)).unwrap();

        assert_eq!(
            txn.deltas(),
            &[
                StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::LocatedIn,
                    relation: RelationValue::LocatedIn {
                        entity: root,
                        place: entity(2),
                    },
                }),
                StateDelta::Relation(RelationDelta::Added {
                    relation_kind: RelationKind::LocatedIn,
                    relation: RelationValue::LocatedIn {
                        entity: root,
                        place: entity(5),
                    },
                }),
                StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::LocatedIn,
                    relation: RelationValue::LocatedIn {
                        entity: inner,
                        place: entity(2),
                    },
                }),
                StateDelta::Relation(RelationDelta::Added {
                    relation_kind: RelationKind::LocatedIn,
                    relation: RelationValue::LocatedIn {
                        entity: inner,
                        place: entity(5),
                    },
                }),
                StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::LocatedIn,
                    relation: RelationValue::LocatedIn {
                        entity: item,
                        place: entity(2),
                    },
                }),
                StateDelta::Relation(RelationDelta::Added {
                    relation_kind: RelationKind::LocatedIn,
                    relation: RelationValue::LocatedIn {
                        entity: item,
                        place: entity(5),
                    },
                }),
            ]
        );
    }

    #[test]
    fn archive_entity_records_full_relation_and_reservation_teardown_for_unplaced_agent() {
        let mut world = World::new(test_topology()).unwrap();
        let fx = archive_teardown_fixture(&mut world);

        let mut txn = new_txn(&mut world);
        txn.archive_entity(fx.archived).unwrap();

        assert_eq!(
            txn.deltas(),
            &[
                StateDelta::Entity(EntityDelta::Archived {
                    entity: fx.archived,
                    kind: EntityKind::Agent,
                }),
                StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::InTransit,
                    relation: RelationValue::InTransit {
                        entity: fx.archived
                    },
                }),
                StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::PossessedBy,
                    relation: RelationValue::PossessedBy {
                        entity: fx.archived,
                        holder: fx.holder,
                    },
                }),
                StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::OwnedBy,
                    relation: RelationValue::OwnedBy {
                        entity: fx.archived,
                        owner: fx.owner,
                    },
                }),
                StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::MemberOf,
                    relation: RelationValue::MemberOf {
                        member: fx.archived,
                        faction: fx.faction,
                    },
                }),
                StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::LoyalTo,
                    relation: RelationValue::LoyalTo {
                        subject: fx.archived,
                        target: fx.loyal_target,
                        strength: fx.loyal_strength,
                    },
                }),
                StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::HostileTo,
                    relation: RelationValue::HostileTo {
                        subject: fx.archived,
                        target: fx.hostile_target,
                    },
                }),
                StateDelta::Reservation(ReservationDelta::Released {
                    reservation: ReservationRecord {
                        id: fx.first_reservation,
                        entity: fx.archived,
                        reserver: fx.holder,
                        range: fx.first_range,
                    },
                }),
                StateDelta::Reservation(ReservationDelta::Released {
                    reservation: ReservationRecord {
                        id: fx.second_reservation,
                        entity: fx.reserved_target,
                        reserver: fx.archived,
                        range: fx.second_range,
                    },
                }),
            ]
        );
    }

    #[test]
    fn archive_entity_records_placement_teardown_for_contained_item() {
        let mut world = World::new(test_topology()).unwrap();
        let container = world.create_container(open_container(20), Tick(1)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Coin, Quantity(1), Tick(2))
            .unwrap();
        world.set_ground_location(container, entity(5)).unwrap();
        world.put_into_container(item, container).unwrap();

        let mut txn = new_txn(&mut world);
        txn.archive_entity(item).unwrap();

        assert_eq!(
            txn.deltas(),
            &[
                StateDelta::Entity(EntityDelta::Archived {
                    entity: item,
                    kind: EntityKind::ItemLot,
                }),
                StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::LocatedIn,
                    relation: RelationValue::LocatedIn {
                        entity: item,
                        place: entity(5),
                    },
                }),
                StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::ContainedBy,
                    relation: RelationValue::ContainedBy {
                        entity: item,
                        container,
                    },
                }),
            ]
        );
    }

    #[test]
    fn reservation_wrappers_snapshot_created_and_released_records() {
        let mut world = World::new(test_topology()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Bread, Quantity(2), Tick(1))
            .unwrap();
        let reserver = world
            .create_agent("Aster", ControlSource::Ai, Tick(2))
            .unwrap();
        let range = TickRange::new(Tick(4), Tick(7)).unwrap();

        let mut txn = new_txn(&mut world);
        let reservation_id = txn.try_reserve(item, reserver, range).unwrap();
        txn.release_reservation(reservation_id).unwrap();

        assert_eq!(txn.deltas().len(), 2);
        assert!(matches!(
            txn.deltas()[0],
            StateDelta::Reservation(ReservationDelta::Created { ref reservation })
                if reservation.id == reservation_id
                    && reservation.entity == item
                    && reservation.reserver == reserver
                    && reservation.range == range
        ));
        assert!(matches!(
            txn.deltas()[1],
            StateDelta::Reservation(ReservationDelta::Released { ref reservation })
                if reservation.id == reservation_id
                    && reservation.entity == item
                    && reservation.reserver == reserver
                    && reservation.range == range
        ));
    }

    #[test]
    fn split_and_merge_lot_wrappers_record_quantity_audit_deltas() {
        let mut world = World::new(test_topology()).unwrap();
        let source = world
            .create_item_lot(CommodityKind::Bread, Quantity(6), Tick(1))
            .unwrap();

        let mut split_txn = new_txn(&mut world);
        let (_, split_off) = split_txn.split_lot(source, Quantity(2)).unwrap();

        assert!(split_txn.deltas().iter().any(|delta| matches!(
            delta,
            StateDelta::Quantity(QuantityDelta::Changed {
                entity,
                commodity: CommodityKind::Bread,
                before: Quantity(6),
                after: Quantity(4),
            }) if *entity == source
        )));
        assert!(split_txn.deltas().iter().any(|delta| matches!(
            delta,
            StateDelta::Quantity(QuantityDelta::Changed {
                entity,
                commodity: CommodityKind::Bread,
                before: Quantity(0),
                after: Quantity(2),
            }) if *entity == split_off
        )));

        commit_txn(split_txn);

        let mut merge_txn = new_txn(&mut world);
        merge_txn.merge_lots(source, split_off).unwrap();

        assert!(merge_txn.deltas().iter().any(|delta| matches!(
            delta,
            StateDelta::Quantity(QuantityDelta::Changed {
                entity,
                commodity: CommodityKind::Bread,
                before: Quantity(4),
                after: Quantity(6),
            }) if *entity == source
        )));
        assert!(merge_txn.deltas().iter().any(|delta| matches!(
            delta,
            StateDelta::Quantity(QuantityDelta::Changed {
                entity,
                commodity: CommodityKind::Bread,
                before: Quantity(2),
                after: Quantity(0),
            }) if *entity == split_off
        )));
        assert!(merge_txn.deltas().iter().any(|delta| matches!(
            delta,
            StateDelta::Entity(EntityDelta::Archived { entity, kind: EntityKind::ItemLot })
                if *entity == split_off
        )));
    }

    #[test]
    fn social_and_ownership_wrappers_record_relation_deltas() {
        let mut world = World::new(test_topology()).unwrap();
        let member = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let owner = world.create_faction("River Pact", Tick(2)).unwrap();
        let faction = world.create_faction("Granary Guild", Tick(3)).unwrap();
        let target = world.create_office("Chair", Tick(4)).unwrap();
        let strength = Permille::new(700).unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_owner(member, owner).unwrap();
        txn.add_member(member, faction).unwrap();
        txn.set_loyalty(member, target, strength).unwrap();
        txn.assign_office(target, member).unwrap();
        txn.add_hostility(member, owner).unwrap();

        assert!(txn.deltas().iter().any(|delta| matches!(
            delta,
            StateDelta::Relation(RelationDelta::Added {
                relation_kind: RelationKind::OwnedBy,
                relation: RelationValue::OwnedBy { entity, owner: actual_owner },
            }) if *entity == member && *actual_owner == owner
        )));
        assert!(txn.deltas().iter().any(|delta| matches!(
            delta,
            StateDelta::Relation(RelationDelta::Added {
                relation_kind: RelationKind::MemberOf,
                relation: RelationValue::MemberOf { member: actual_member, faction: actual_faction },
            }) if *actual_member == member && *actual_faction == faction
        )));
        assert!(txn.deltas().iter().any(|delta| matches!(
            delta,
            StateDelta::Relation(RelationDelta::Added {
                relation_kind: RelationKind::LoyalTo,
                relation: RelationValue::LoyalTo {
                    subject,
                    target: actual_target,
                    strength: actual_strength,
                },
            }) if *subject == member && *actual_target == target && *actual_strength == strength
        )));
        assert!(txn.deltas().iter().any(|delta| matches!(
            delta,
            StateDelta::Relation(RelationDelta::Added {
                relation_kind: RelationKind::OfficeHolder,
                relation: RelationValue::OfficeHolder { office, holder },
            }) if *office == target && *holder == member
        )));
        assert!(txn.deltas().iter().any(|delta| matches!(
            delta,
            StateDelta::Relation(RelationDelta::Added {
                relation_kind: RelationKind::HostileTo,
                relation: RelationValue::HostileTo { subject, target: actual_target },
            }) if *subject == member && *actual_target == owner
        )));
    }

    #[test]
    fn clear_loyalty_records_removed_relation_delta() {
        let mut world = World::new(test_topology()).unwrap();
        let subject = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let target = world.create_faction("River Pact", Tick(2)).unwrap();
        let strength = Permille::new(700).unwrap();
        world.set_loyalty(subject, target, strength).unwrap();

        let mut txn = new_txn(&mut world);
        txn.clear_loyalty(subject, target).unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Relation(RelationDelta::Removed {
                relation_kind: RelationKind::LoyalTo,
                relation: RelationValue::LoyalTo {
                    subject,
                    target,
                    strength,
                },
            })]
        );
    }

    #[test]
    fn updating_loyalty_strength_records_removed_and_added_deltas() {
        let mut world = World::new(test_topology()).unwrap();
        let subject = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let target = world.create_office("Granary Chair", Tick(2)).unwrap();
        let old_strength = Permille::new(250).unwrap();
        let new_strength = Permille::new(900).unwrap();
        world.set_loyalty(subject, target, old_strength).unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_loyalty(subject, target, new_strength).unwrap();

        assert_eq!(
            txn.deltas(),
            &[
                StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::LoyalTo,
                    relation: RelationValue::LoyalTo {
                        subject,
                        target,
                        strength: old_strength,
                    },
                }),
                StateDelta::Relation(RelationDelta::Added {
                    relation_kind: RelationKind::LoyalTo,
                    relation: RelationValue::LoyalTo {
                        subject,
                        target,
                        strength: new_strength,
                    },
                }),
            ]
        );
    }

    #[test]
    fn commit_preserves_loyalty_relation_deltas_in_event_log() {
        let mut world = World::new(test_topology()).unwrap();
        let subject = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let target = world.create_office("Granary Chair", Tick(2)).unwrap();
        let old_strength = Permille::new(300).unwrap();
        let new_strength = Permille::new(800).unwrap();
        world.set_loyalty(subject, target, old_strength).unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_loyalty(subject, target, new_strength).unwrap();
        let expected_deltas = txn.deltas().to_vec();

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas, expected_deltas);
        assert_eq!(world.loyalty_to(subject, target), Some(new_strength));
    }

    #[test]
    fn commit_captures_observed_entities_from_actor_targets_and_deltas() {
        let mut world = World::new(test_topology()).unwrap();
        let place = entity(5);
        let actor = world
            .create_agent("Actor", ControlSource::Ai, Tick(1))
            .unwrap();
        let target = world
            .create_agent("Target", ControlSource::Ai, Tick(1))
            .unwrap();
        let bread = world
            .create_item_lot(CommodityKind::Bread, Quantity(2), Tick(1))
            .unwrap();

        world.set_ground_location(actor, place).unwrap();
        world.set_ground_location(target, place).unwrap();
        world.set_ground_location(bread, place).unwrap();

        let mut txn = WorldTxn::new(
            &mut world,
            Tick(9),
            CauseRef::Bootstrap,
            Some(actor),
            Some(place),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        txn.add_target(target);
        txn.set_possessor(bread, target).unwrap();

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(
            record
                .observed_entities
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            vec![actor, target, bread]
        );
        assert_eq!(
            record
                .observed_entities
                .get(&target)
                .unwrap()
                .last_known_inventory,
            BTreeMap::from([(CommodityKind::Bread, Quantity(2))])
        );
        assert_eq!(
            record.observed_entities.get(&bread).unwrap().last_known_place,
            Some(place)
        );
    }

    #[test]
    fn commit_orders_and_deduplicates_transaction_owned_evidence() {
        let mut world = World::new(test_topology()).unwrap();
        let actor = world
            .create_agent("Actor", ControlSource::Ai, Tick(1))
            .unwrap();
        let subject = world
            .create_agent("Subject", ControlSource::Ai, Tick(1))
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.add_evidence(EvidenceRef::Mismatch {
            observer: actor,
            subject,
            kind: MismatchKind::AliveStatusChanged,
        })
        .add_evidence(EvidenceRef::Wound {
            entity: subject,
            wound_id: WoundId(2),
        })
        .add_evidence(EvidenceRef::Mismatch {
            observer: actor,
            subject,
            kind: MismatchKind::AliveStatusChanged,
        })
        .extend_evidence([
            EvidenceRef::Wound {
                entity: subject,
                wound_id: WoundId(1),
            },
            EvidenceRef::Wound {
                entity: subject,
                wound_id: WoundId(2),
            },
        ]);

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(
            record.evidence,
            vec![
                EvidenceRef::Wound {
                    entity: subject,
                    wound_id: WoundId(1),
                },
                EvidenceRef::Wound {
                    entity: subject,
                    wound_id: WoundId(2),
                },
                EvidenceRef::Mismatch {
                    observer: actor,
                    subject,
                    kind: MismatchKind::AliveStatusChanged,
                },
            ]
        );
    }

    #[test]
    fn commit_captures_observed_entities_from_evidence_without_deltas() {
        let mut world = World::new(test_topology()).unwrap();
        let place = entity(5);
        let actor = world
            .create_agent("Actor", ControlSource::Ai, Tick(1))
            .unwrap();
        let subject = world
            .create_agent("Subject", ControlSource::Ai, Tick(1))
            .unwrap();
        let item = world
            .create_item_lot(CommodityKind::Bread, Quantity(3), Tick(1))
            .unwrap();

        world.set_ground_location(actor, place).unwrap();
        world.set_ground_location(subject, place).unwrap();
        world.set_possessor(item, subject).unwrap();

        let mut txn = WorldTxn::new(
            &mut world,
            Tick(9),
            CauseRef::Bootstrap,
            Some(actor),
            Some(place),
            VisibilitySpec::ParticipantsOnly,
            WitnessData::default(),
        );
        txn.add_evidence(EvidenceRef::Wound {
            entity: subject,
            wound_id: WoundId(7),
        });

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(
            record
                .observed_entities
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            vec![actor, subject]
        );
        assert_eq!(
            record
                .observed_entities
                .get(&subject)
                .unwrap()
                .last_known_inventory,
            BTreeMap::from([(CommodityKind::Bread, Quantity(3))])
        );
    }

    #[test]
    fn set_component_homeostatic_needs_records_component_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before = HomeostaticNeeds::new(
            Permille::new(100).unwrap(),
            Permille::new(200).unwrap(),
            Permille::new(300).unwrap(),
            Permille::new(400).unwrap(),
            Permille::new(500).unwrap(),
        );
        let after = HomeostaticNeeds::new(
            Permille::new(101).unwrap(),
            Permille::new(202).unwrap(),
            Permille::new(303).unwrap(),
            Permille::new(404).unwrap(),
            Permille::new(505).unwrap(),
        );
        world
            .insert_component_homeostatic_needs(agent, before)
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_component_homeostatic_needs(agent, after).unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Set {
                entity: agent,
                component_kind: ComponentKind::HomeostaticNeeds,
                before: Some(ComponentValue::HomeostaticNeeds(before)),
                after: ComponentValue::HomeostaticNeeds(after),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_homeostatic_needs(agent), Some(&after));
    }

    #[test]
    fn set_component_deprivation_exposure_records_component_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before = DeprivationExposure {
            hunger_critical_ticks: 1,
            thirst_critical_ticks: 2,
            fatigue_critical_ticks: 3,
            bladder_critical_ticks: 4,
        };
        let after = DeprivationExposure {
            hunger_critical_ticks: 10,
            thirst_critical_ticks: 20,
            fatigue_critical_ticks: 30,
            bladder_critical_ticks: 40,
        };
        world
            .insert_component_deprivation_exposure(agent, before)
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_component_deprivation_exposure(agent, after)
            .unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Set {
                entity: agent,
                component_kind: ComponentKind::DeprivationExposure,
                before: Some(ComponentValue::DeprivationExposure(before)),
                after: ComponentValue::DeprivationExposure(after),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(
            world.get_component_deprivation_exposure(agent),
            Some(&after)
        );
    }

    #[test]
    fn set_component_resource_source_records_component_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let facility = world.create_entity(EntityKind::Facility, Tick(1));
        let before = sample_resource_source();
        let after = ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(5),
            max_quantity: Quantity(9),
            regeneration_ticks_per_unit: Some(std::num::NonZeroU32::new(6).unwrap()),
            last_regeneration_tick: Some(Tick(9)),
        };
        world
            .insert_component_resource_source(facility, before.clone())
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_component_resource_source(facility, after.clone())
            .unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Set {
                entity: facility,
                component_kind: ComponentKind::ResourceSource,
                before: Some(ComponentValue::ResourceSource(before)),
                after: ComponentValue::ResourceSource(after.clone()),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_resource_source(facility), Some(&after));
    }

    #[test]
    fn set_component_carry_capacity_records_component_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before = CarryCapacity(LoadUnits(12));
        let after = CarryCapacity(LoadUnits(18));
        world
            .insert_component_carry_capacity(agent, before)
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_component_carry_capacity(agent, after).unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Set {
                entity: agent,
                component_kind: ComponentKind::CarryCapacity,
                before: Some(ComponentValue::CarryCapacity(before)),
                after: ComponentValue::CarryCapacity(after),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_carry_capacity(agent), Some(&after));
    }

    #[test]
    fn set_component_known_recipes_records_component_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before = KnownRecipes::with([crate::RecipeId(1), crate::RecipeId(4)]);
        let after = KnownRecipes::with([crate::RecipeId(2), crate::RecipeId(7)]);
        world
            .insert_component_known_recipes(agent, before.clone())
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_component_known_recipes(agent, after.clone())
            .unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Set {
                entity: agent,
                component_kind: ComponentKind::KnownRecipes,
                before: Some(ComponentValue::KnownRecipes(before)),
                after: ComponentValue::KnownRecipes(after.clone()),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_known_recipes(agent), Some(&after));
    }

    #[test]
    fn set_component_demand_memory_records_component_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before: DemandMemory = sample_demand_memory();
        let mut after = before.clone();
        after
            .observations
            .push(crate::test_utils::sample_demand_observation());
        world
            .insert_component_demand_memory(agent, before.clone())
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_component_demand_memory(agent, after.clone())
            .unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Set {
                entity: agent,
                component_kind: ComponentKind::DemandMemory,
                before: Some(ComponentValue::DemandMemory(before)),
                after: ComponentValue::DemandMemory(after.clone()),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_demand_memory(agent), Some(&after));
    }

    #[test]
    fn set_component_trade_disposition_profile_records_component_delta_and_updates_world_on_commit()
    {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before: TradeDispositionProfile = sample_trade_disposition_profile();
        let after = TradeDispositionProfile {
            demand_memory_retention_ticks: before.demand_memory_retention_ticks + 5,
            ..before.clone()
        };
        world
            .insert_component_trade_disposition_profile(agent, before.clone())
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_component_trade_disposition_profile(agent, after.clone())
            .unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Set {
                entity: agent,
                component_kind: ComponentKind::TradeDispositionProfile,
                before: Some(ComponentValue::TradeDispositionProfile(before)),
                after: ComponentValue::TradeDispositionProfile(after.clone()),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(
            world.get_component_trade_disposition_profile(agent),
            Some(&after)
        );
    }

    #[test]
    fn set_component_travel_disposition_profile_records_component_delta_and_updates_world_on_commit(
    ) {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before: TravelDispositionProfile = sample_travel_disposition_profile();
        let after = TravelDispositionProfile {
            route_replan_margin: Permille::new(240).unwrap(),
            ..before.clone()
        };
        world
            .insert_component_travel_disposition_profile(agent, before.clone())
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_component_travel_disposition_profile(agent, after.clone())
            .unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Set {
                entity: agent,
                component_kind: ComponentKind::TravelDispositionProfile,
                before: Some(ComponentValue::TravelDispositionProfile(before)),
                after: ComponentValue::TravelDispositionProfile(after.clone()),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(
            world.get_component_travel_disposition_profile(agent),
            Some(&after)
        );
    }

    #[test]
    fn set_component_merchandise_profile_records_component_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before: MerchandiseProfile = sample_merchandise_profile();
        let mut after = before.clone();
        after.home_market = Some(entity(5));
        world
            .insert_component_merchandise_profile(agent, before.clone())
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_component_merchandise_profile(agent, after.clone())
            .unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Set {
                entity: agent,
                component_kind: ComponentKind::MerchandiseProfile,
                before: Some(ComponentValue::MerchandiseProfile(before)),
                after: ComponentValue::MerchandiseProfile(after.clone()),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_merchandise_profile(agent), Some(&after));
    }

    #[test]
    fn set_component_utility_profile_records_component_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before: UtilityProfile = sample_utility_profile();
        let mut after = before.clone();
        after.enterprise_weight = Permille::new(800).unwrap();
        world
            .insert_component_utility_profile(agent, before.clone())
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_component_utility_profile(agent, after.clone())
            .unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Set {
                entity: agent,
                component_kind: ComponentKind::UtilityProfile,
                before: Some(ComponentValue::UtilityProfile(before)),
                after: ComponentValue::UtilityProfile(after.clone()),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_utility_profile(agent), Some(&after));
    }

    #[test]
    fn set_component_agent_belief_store_records_component_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before = world
            .get_component_agent_belief_store(agent)
            .cloned()
            .unwrap();
        let mut after = before.clone();
        after.known_entities.insert(
            entity(22),
            BelievedEntityState {
                last_known_place: Some(entity(2)),
                last_known_inventory: BTreeMap::new(),
                workstation_tag: None,
                resource_source: None,
                alive: false,
                wounds: Vec::new(),
                observed_tick: Tick(12),
                source: PerceptionSource::Inference,
            },
        );

        let mut txn = new_txn(&mut world);
        txn.set_component_agent_belief_store(agent, after.clone())
            .unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Set {
                entity: agent,
                component_kind: ComponentKind::AgentBeliefStore,
                before: Some(ComponentValue::AgentBeliefStore(before)),
                after: ComponentValue::AgentBeliefStore(after.clone()),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_agent_belief_store(agent), Some(&after));
    }

    #[test]
    fn set_component_perception_profile_records_component_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before = world
            .get_component_perception_profile(agent)
            .copied()
            .unwrap();
        let mut after = before;
        after.memory_capacity += 3;
        after.observation_fidelity = Permille::new(990).unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_component_perception_profile(agent, after).unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Set {
                entity: agent,
                component_kind: ComponentKind::PerceptionProfile,
                before: Some(ComponentValue::PerceptionProfile(before)),
                after: ComponentValue::PerceptionProfile(after),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_perception_profile(agent), Some(&after));
    }

    #[test]
    fn set_component_tell_profile_records_component_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before = world.get_component_tell_profile(agent).copied().unwrap();
        let after = TellProfile {
            max_tell_candidates: before.max_tell_candidates + 2,
            max_relay_chain_len: before.max_relay_chain_len + 1,
            acceptance_fidelity: Permille::new(910).unwrap(),
        };

        let mut txn = new_txn(&mut world);
        txn.set_component_tell_profile(agent, after).unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Set {
                entity: agent,
                component_kind: ComponentKind::TellProfile,
                before: Some(ComponentValue::TellProfile(before)),
                after: ComponentValue::TellProfile(after),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_tell_profile(agent), Some(&after));
    }

    #[test]
    fn set_component_blocked_intent_memory_records_component_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before: BlockedIntentMemory = sample_blocked_intent_memory();
        let mut after = before.clone();
        after.intents[0].expires_tick = Tick(21);
        world
            .insert_component_blocked_intent_memory(agent, before.clone())
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_component_blocked_intent_memory(agent, after.clone())
            .unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Set {
                entity: agent,
                component_kind: ComponentKind::BlockedIntentMemory,
                before: Some(ComponentValue::BlockedIntentMemory(before)),
                after: ComponentValue::BlockedIntentMemory(after.clone()),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(
            world.get_component_blocked_intent_memory(agent),
            Some(&after)
        );
    }

    #[test]
    fn set_component_substitute_preferences_records_component_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before: SubstitutePreferences = sample_substitute_preferences();
        let mut after = before.clone();
        after
            .preferences
            .insert(crate::TradeCategory::Food, vec![CommodityKind::Water]);
        world
            .insert_component_substitute_preferences(agent, before.clone())
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_component_substitute_preferences(agent, after.clone())
            .unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Set {
                entity: agent,
                component_kind: ComponentKind::SubstitutePreferences,
                before: Some(ComponentValue::SubstitutePreferences(before)),
                after: ComponentValue::SubstitutePreferences(after.clone()),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(
            world.get_component_substitute_preferences(agent),
            Some(&after)
        );
    }

    #[test]
    fn set_component_workstation_marker_records_component_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let facility = world.create_entity(EntityKind::Facility, Tick(1));
        let before = crate::WorkstationMarker(crate::WorkstationTag::Mill);
        let after = crate::WorkstationMarker(crate::WorkstationTag::Forge);
        world
            .insert_component_workstation_marker(facility, before)
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_component_workstation_marker(facility, after)
            .unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Set {
                entity: facility,
                component_kind: ComponentKind::WorkstationMarker,
                before: Some(ComponentValue::WorkstationMarker(before)),
                after: ComponentValue::WorkstationMarker(after),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(
            world.get_component_workstation_marker(facility),
            Some(&after)
        );
    }

    #[test]
    fn set_component_in_transit_on_edge_records_component_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before = InTransitOnEdge {
            edge_id: TravelEdgeId(3),
            origin: entity(1),
            destination: entity(2),
            departure_tick: Tick(5),
            arrival_tick: Tick(11),
        };
        let after = InTransitOnEdge {
            edge_id: TravelEdgeId(4),
            origin: entity(2),
            destination: entity(3),
            departure_tick: Tick(12),
            arrival_tick: Tick(18),
        };
        world
            .insert_component_in_transit_on_edge(agent, before.clone())
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_component_in_transit_on_edge(agent, after.clone())
            .unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Set {
                entity: agent,
                component_kind: ComponentKind::InTransitOnEdge,
                before: Some(ComponentValue::InTransitOnEdge(before)),
                after: ComponentValue::InTransitOnEdge(after.clone()),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_in_transit_on_edge(agent), Some(&after));
    }

    #[test]
    fn set_component_production_job_records_component_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let facility = world.create_entity(EntityKind::Facility, Tick(1));
        let before = crate::ProductionJob {
            recipe_id: crate::RecipeId(3),
            worker: entity(9),
            staged_inputs_container: entity(10),
            progress_ticks: 4,
        };
        let after = crate::ProductionJob {
            recipe_id: crate::RecipeId(7),
            worker: entity(11),
            staged_inputs_container: entity(12),
            progress_ticks: 8,
        };
        world
            .insert_component_production_job(facility, before.clone())
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.set_component_production_job(facility, after.clone())
            .unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Set {
                entity: facility,
                component_kind: ComponentKind::ProductionJob,
                before: Some(ComponentValue::ProductionJob(before)),
                after: ComponentValue::ProductionJob(after.clone()),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_production_job(facility), Some(&after));
    }

    #[test]
    fn clear_component_carry_capacity_records_removed_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before = CarryCapacity(LoadUnits(12));
        world
            .insert_component_carry_capacity(agent, before)
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.clear_component_carry_capacity(agent).unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Removed {
                entity: agent,
                component_kind: ComponentKind::CarryCapacity,
                before: ComponentValue::CarryCapacity(before),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_carry_capacity(agent), None);
    }

    #[test]
    fn clear_component_known_recipes_records_removed_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before = KnownRecipes::with([crate::RecipeId(6), crate::RecipeId(1)]);
        world
            .insert_component_known_recipes(agent, before.clone())
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.clear_component_known_recipes(agent).unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Removed {
                entity: agent,
                component_kind: ComponentKind::KnownRecipes,
                before: ComponentValue::KnownRecipes(before),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_known_recipes(agent), None);
    }

    #[test]
    fn clear_component_utility_profile_records_removed_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before = sample_utility_profile();
        world
            .insert_component_utility_profile(agent, before.clone())
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.clear_component_utility_profile(agent).unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Removed {
                entity: agent,
                component_kind: ComponentKind::UtilityProfile,
                before: ComponentValue::UtilityProfile(before),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_utility_profile(agent), None);
    }

    #[test]
    fn clear_component_agent_belief_store_records_removed_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before = world
            .get_component_agent_belief_store(agent)
            .cloned()
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.clear_component_agent_belief_store(agent).unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Removed {
                entity: agent,
                component_kind: ComponentKind::AgentBeliefStore,
                before: ComponentValue::AgentBeliefStore(before),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_agent_belief_store(agent), None);
    }

    #[test]
    fn clear_component_perception_profile_records_removed_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before = world
            .get_component_perception_profile(agent)
            .copied()
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.clear_component_perception_profile(agent).unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Removed {
                entity: agent,
                component_kind: ComponentKind::PerceptionProfile,
                before: ComponentValue::PerceptionProfile(before),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_perception_profile(agent), None);
    }

    #[test]
    fn clear_component_tell_profile_records_removed_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before = world.get_component_tell_profile(agent).copied().unwrap();

        let mut txn = new_txn(&mut world);
        txn.clear_component_tell_profile(agent).unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Removed {
                entity: agent,
                component_kind: ComponentKind::TellProfile,
                before: ComponentValue::TellProfile(before),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_tell_profile(agent), None);
    }

    #[test]
    fn clear_component_blocked_intent_memory_records_removed_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before = sample_blocked_intent_memory();
        world
            .insert_component_blocked_intent_memory(agent, before.clone())
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.clear_component_blocked_intent_memory(agent).unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Removed {
                entity: agent,
                component_kind: ComponentKind::BlockedIntentMemory,
                before: ComponentValue::BlockedIntentMemory(before),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_blocked_intent_memory(agent), None);
    }

    #[test]
    fn clear_component_workstation_marker_records_removed_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let facility = world.create_entity(EntityKind::Facility, Tick(1));
        let before = crate::WorkstationMarker(crate::WorkstationTag::Forge);
        world
            .insert_component_workstation_marker(facility, before)
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.clear_component_workstation_marker(facility).unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Removed {
                entity: facility,
                component_kind: ComponentKind::WorkstationMarker,
                before: ComponentValue::WorkstationMarker(before),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_workstation_marker(facility), None);
    }

    #[test]
    fn clear_component_in_transit_on_edge_records_removed_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let before = InTransitOnEdge {
            edge_id: TravelEdgeId(3),
            origin: entity(1),
            destination: entity(2),
            departure_tick: Tick(5),
            arrival_tick: Tick(11),
        };
        world
            .insert_component_in_transit_on_edge(agent, before.clone())
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.clear_component_in_transit_on_edge(agent).unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Removed {
                entity: agent,
                component_kind: ComponentKind::InTransitOnEdge,
                before: ComponentValue::InTransitOnEdge(before),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_in_transit_on_edge(agent), None);
    }

    #[test]
    fn clear_component_production_job_records_removed_delta_and_updates_world_on_commit() {
        let mut world = World::new(test_topology()).unwrap();
        let facility = world.create_entity(EntityKind::Facility, Tick(1));
        let before = crate::ProductionJob {
            recipe_id: crate::RecipeId(5),
            worker: entity(13),
            staged_inputs_container: entity(14),
            progress_ticks: 6,
        };
        world
            .insert_component_production_job(facility, before.clone())
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.clear_component_production_job(facility).unwrap();

        assert_eq!(
            txn.deltas(),
            &[StateDelta::Component(ComponentDelta::Removed {
                entity: facility,
                component_kind: ComponentKind::ProductionJob,
                before: ComponentValue::ProductionJob(before),
            })]
        );

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas.len(), 1);
        assert_eq!(world.get_component_production_job(facility), None);
    }

    #[test]
    fn clear_component_in_transit_on_edge_is_noop_when_component_is_missing() {
        let mut world = World::new(test_topology()).unwrap();
        let agent = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();

        let mut txn = new_txn(&mut world);
        txn.clear_component_in_transit_on_edge(agent).unwrap();

        assert!(txn.deltas().is_empty());

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert!(record.state_deltas.is_empty());
        assert_eq!(world.get_component_in_transit_on_edge(agent), None);
    }

    #[test]
    fn builder_methods_accumulate_without_duplicates() {
        let mut world = World::new(test_topology()).unwrap();
        let mut txn = new_txn(&mut world);

        txn.add_target(entity(8))
            .add_target(entity(8))
            .add_target(entity(3))
            .add_tag(EventTag::WorldMutation)
            .add_tag(EventTag::WorldMutation)
            .add_tag(EventTag::System);

        assert_eq!(txn.target_ids(), &[entity(8), entity(3)]);
        assert_eq!(
            txn.tags().iter().copied().collect::<Vec<_>>(),
            vec![EventTag::WorldMutation, EventTag::System]
        );
    }

    #[test]
    fn commit_emits_record_with_canonical_targets_and_preserved_delta_order() {
        let mut world = World::new(test_topology()).unwrap();
        let mut txn = new_txn(&mut world);
        let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.add_target(entity(9))
            .add_target(agent)
            .add_target(entity(4))
            .add_tag(EventTag::WorldMutation)
            .add_tag(EventTag::Control);
        let expected_deltas = txn.deltas().to_vec();
        let expected_witness = txn.witness_data().clone();
        let expected_tags = txn.tags().clone();

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(event_id, crate::EventId(0));
        assert_eq!(record.event_id, event_id);
        assert_eq!(record.tick, Tick(9));
        assert_eq!(record.cause, CauseRef::Bootstrap);
        assert_eq!(record.actor_id, Some(entity(11)));
        assert_eq!(record.place_id, Some(entity(5)));
        assert_eq!(record.target_ids, vec![entity(4), agent, entity(9)]);
        assert_eq!(record.state_deltas, expected_deltas);
        assert_eq!(record.visibility, VisibilitySpec::SamePlace);
        assert_eq!(record.witness_data, expected_witness);
        assert_eq!(record.tags, expected_tags);
        assert_eq!(log.events_at_tick(Tick(9)), &[event_id]);
        assert_eq!(log.events_by_actor(entity(11)), &[event_id]);
        assert_eq!(log.events_by_place(entity(5)), &[event_id]);
        assert_eq!(log.events_by_tag(EventTag::WorldMutation), &[event_id]);
        assert_eq!(log.events_by_tag(EventTag::Control), &[event_id]);
    }

    #[test]
    fn commit_allows_empty_deltas_for_root_events() {
        let mut world = World::new(test_topology()).unwrap();
        let txn = new_txn(&mut world);
        let mut log = EventLog::new();

        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(event_id, crate::EventId(0));
        assert!(record.state_deltas.is_empty());
        assert!(record.tags.is_empty());
        assert!(record.target_ids.is_empty());
    }

    #[test]
    fn sequential_commits_receive_gapless_ids() {
        let mut world = World::new(test_topology()).unwrap();
        let mut first = new_txn(&mut world);
        first.add_tag(EventTag::WorldMutation);

        let mut log = EventLog::new();
        let first_id = first.commit(&mut log);
        let mut second = WorldTxn::new(
            &mut world,
            Tick(10),
            CauseRef::SystemTick(Tick(10)),
            None,
            Some(entity(2)),
            VisibilitySpec::PublicRecord,
            WitnessData::default(),
        );
        second.add_tag(EventTag::System);
        let second_id = second.commit(&mut log);

        assert_eq!(first_id, crate::EventId(0));
        assert_eq!(second_id, crate::EventId(1));
        assert_eq!(log.events_at_tick(Tick(9)), &[first_id]);
        assert_eq!(log.events_at_tick(Tick(10)), &[second_id]);
        assert_eq!(log.events_by_place(entity(2)), &[second_id]);
        assert_eq!(log.events_by_tag(EventTag::System), &[second_id]);
    }

    #[test]
    fn commit_preserves_archive_teardown_batch_without_reshaping_it() {
        let mut world = World::new(test_topology()).unwrap();
        let fixture = archive_teardown_fixture(&mut world);
        let mut txn = new_txn(&mut world);
        txn.add_target(fixture.archived)
            .add_tag(EventTag::WorldMutation);
        txn.archive_entity(fixture.archived).unwrap();
        let expected_deltas = txn.deltas().to_vec();

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.state_deltas, expected_deltas);
        assert!(matches!(
            record.state_deltas.first(),
            Some(StateDelta::Entity(EntityDelta::Archived { entity, .. })) if *entity == fixture.archived
        ));
    }

    #[test]
    fn mutation_errors_propagate_without_recording_partial_deltas() {
        let mut world = World::new(test_topology()).unwrap();
        let mut txn = new_txn(&mut world);

        let err = txn.create_container(open_container(0)).unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
        assert!(txn.deltas().is_empty());
    }
}
