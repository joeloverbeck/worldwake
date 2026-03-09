use crate::{
    CauseRef, ComponentDelta, ComponentValue, EntityDelta, EventTag, RelationDelta, RelationKind,
    RelationValue, ReservationDelta, StateDelta, VisibilitySpec, WitnessData,
};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;
use worldwake_core::{
    ArchiveMutationSnapshot, CommodityKind, Container, ControlSource, EntityId, EntityKind, FactId,
    Permille, Quantity, ReservationId, Tick, TickRange, UniqueItemKind, World, WorldError,
};

pub struct WorldTxn<'w> {
    world: &'w mut World,
    tick: Tick,
    cause: CauseRef,
    actor_id: Option<EntityId>,
    place_id: Option<EntityId>,
    tags: BTreeSet<EventTag>,
    target_ids: Vec<EntityId>,
    visibility: VisibilitySpec,
    witness_data: WitnessData,
    deltas: Vec<StateDelta>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct PlacementSnapshot {
    located_in: Option<EntityId>,
    in_transit: bool,
    contained_by: Option<EntityId>,
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

    pub fn create_entity(&mut self, kind: EntityKind) -> EntityId {
        let entity = self.world.create_entity(kind, self.tick);
        self.record_created_entity(entity, kind);
        entity
    }

    pub fn create_agent(
        &mut self,
        name: &str,
        control_source: ControlSource,
    ) -> Result<EntityId, WorldError> {
        let entity = self.world.create_agent(name, control_source, self.tick)?;
        self.record_created_entity(entity, EntityKind::Agent);
        Ok(entity)
    }

    pub fn create_office(&mut self, name: &str) -> Result<EntityId, WorldError> {
        let entity = self.world.create_office(name, self.tick)?;
        self.record_created_entity(entity, EntityKind::Office);
        Ok(entity)
    }

    pub fn create_faction(&mut self, name: &str) -> Result<EntityId, WorldError> {
        let entity = self.world.create_faction(name, self.tick)?;
        self.record_created_entity(entity, EntityKind::Faction);
        Ok(entity)
    }

    pub fn create_item_lot(
        &mut self,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Result<EntityId, WorldError> {
        let entity = self.world.create_item_lot(commodity, quantity, self.tick)?;
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
            .world
            .create_unique_item(kind, name, metadata, self.tick)?;
        self.record_created_entity(entity, EntityKind::UniqueItem);
        Ok(entity)
    }

    pub fn create_container(&mut self, container: Container) -> Result<EntityId, WorldError> {
        let entity = self.world.create_container(container, self.tick)?;
        self.record_created_entity(entity, EntityKind::Container);
        Ok(entity)
    }

    pub fn archive_entity(&mut self, entity: EntityId) -> Result<(), WorldError> {
        let snapshot = self.world.archive_mutation_snapshot(entity)?;
        self.world.archive_entity(entity, self.tick)?;
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

    pub fn try_reserve(
        &mut self,
        entity: EntityId,
        reserver: EntityId,
        range: TickRange,
    ) -> Result<ReservationId, WorldError> {
        let reservation_id = self.world.try_reserve(entity, reserver, range)?;
        let reservation = self
            .world
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
            .world
            .reservation(reservation_id)
            .cloned()
            .ok_or_else(|| {
                WorldError::InvalidOperation(format!("reservation {reservation_id} does not exist"))
            })?;
        self.world.release_reservation(reservation_id)?;
        self.deltas
            .push(StateDelta::Reservation(ReservationDelta::Released {
                reservation,
            }));
        Ok(())
    }

    fn record_created_entity(&mut self, entity: EntityId, kind: EntityKind) {
        self.deltas
            .push(StateDelta::Entity(EntityDelta::Created { entity, kind }));
        self.deltas
            .extend(self.component_deltas_after_create(entity));
        if self.world.is_in_transit(entity) {
            self.deltas.push(StateDelta::Relation(RelationDelta::Added {
                relation_kind: RelationKind::InTransit,
                relation: RelationValue::InTransit { entity },
            }));
        }
    }

    fn component_deltas_after_create(&self, entity: EntityId) -> Vec<StateDelta> {
        let mut deltas = Vec::new();

        for value in [
            self.world
                .get_component_name(entity)
                .cloned()
                .map(ComponentValue::Name),
            self.world
                .get_component_agent_data(entity)
                .cloned()
                .map(ComponentValue::AgentData),
            self.world
                .get_component_item_lot(entity)
                .cloned()
                .map(ComponentValue::ItemLot),
            self.world
                .get_component_unique_item(entity)
                .cloned()
                .map(ComponentValue::UniqueItem),
            self.world
                .get_component_container(entity)
                .cloned()
                .map(ComponentValue::Container),
        ]
        .into_iter()
        .flatten()
        {
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

        mutate(self.world)?;

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
        if self.world.get_component_container(entity).is_some() {
            scope.extend(self.world.recursive_contents_of(entity));
        }
        scope
    }

    fn placement_snapshot(&self, entity: EntityId) -> PlacementSnapshot {
        PlacementSnapshot {
            located_in: self.world.effective_place(entity),
            in_transit: self.world.is_in_transit(entity),
            contained_by: self.world.direct_container(entity),
        }
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
        self.push_archive_removed_fact_knowledge(snapshot.entity, &snapshot.known_facts);
        self.push_archive_removed_fact_beliefs(snapshot.entity, &snapshot.believed_facts);
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

    fn push_archive_removed_fact_knowledge(&mut self, agent: EntityId, facts: &[FactId]) {
        for fact in facts {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::KnowsFact,
                    relation: RelationValue::KnowsFact { agent, fact: *fact },
                }));
        }
    }

    fn push_archive_removed_fact_beliefs(&mut self, agent: EntityId, facts: &[FactId]) {
        for fact in facts {
            self.deltas
                .push(StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::BelievesFact,
                    relation: RelationValue::BelievesFact { agent, fact: *fact },
                }));
        }
    }
}

impl Deref for WorldTxn<'_> {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        self.world
    }
}

#[cfg(test)]
mod tests {
    use super::WorldTxn;
    use crate::{
        CauseRef, ComponentDelta, ComponentKind, ComponentValue, EntityDelta, EventTag,
        RelationDelta, RelationKind, RelationValue, ReservationDelta, StateDelta, VisibilitySpec,
        WitnessData,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use worldwake_core::{
        CommodityKind, Container, ControlSource, EntityId, EntityKind, FactId, LoadUnits, Name,
        Permille, Place, PlaceTag, Quantity, ReservationId, ReservationRecord, Tick, TickRange,
        Topology, UniqueItemKind, World, WorldError,
    };

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

    struct ArchiveTeardownFixture {
        archived: EntityId,
        owner: EntityId,
        holder: EntityId,
        faction: EntityId,
        loyal_target: EntityId,
        hostile_target: EntityId,
        reserved_target: EntityId,
        fact_known: FactId,
        fact_believed: FactId,
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
        let fact_known = FactId(11);
        let fact_believed = FactId(12);
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
        world.add_known_fact(archived, fact_known).unwrap();
        world.add_believed_fact(archived, fact_believed).unwrap();
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
            fact_known,
            fact_believed,
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
                    after: ComponentValue::AgentData(worldwake_core::AgentData {
                        control_source: ControlSource::Human,
                    }),
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
                StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::KnowsFact,
                    relation: RelationValue::KnowsFact {
                        agent: fx.archived,
                        fact: fx.fact_known,
                    },
                }),
                StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::BelievesFact,
                    relation: RelationValue::BelievesFact {
                        agent: fx.archived,
                        fact: fx.fact_believed,
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
    fn mutation_errors_propagate_without_recording_partial_deltas() {
        let mut world = World::new(test_topology()).unwrap();
        let mut txn = new_txn(&mut world);

        let err = txn.create_container(open_container(0)).unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
        assert!(txn.deltas().is_empty());
    }
}
