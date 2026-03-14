//! Debug-time verification for event-log structure and journaled world state.

use crate::EventId;
use crate::{CauseRef, EventLog};
use std::collections::BTreeSet;

#[cfg(test)]
use crate::{
    ComponentDelta, ComponentKind, ComponentValue, EntityDelta, RelationDelta, RelationValue,
    ReservationDelta, StateDelta,
};
#[cfg(test)]
use crate::{EntityId, EntityKind, ReservationId, ReservationRecord, World};
#[cfg(test)]
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerificationError {
    OrphanEvent {
        event_id: EventId,
    },
    DanglingCauseRef {
        event_id: EventId,
        cause: CauseRef,
    },
    NonMonotonicId {
        event_id: EventId,
        expected: EventId,
    },
    GapInSequence {
        expected: EventId,
        found: EventId,
    },
    FutureCauseRef {
        event_id: EventId,
        cause: CauseRef,
    },
    WorldStateMismatch {
        detail: String,
    },
}

pub fn verify_completeness(event_log: &EventLog) -> Result<(), Vec<VerificationError>> {
    let mut errors = Vec::new();

    for index in 0..event_log.len() {
        let expected_id = EventId(u64::try_from(index).expect("event count exceeds u64"));
        let record = event_log
            .get(expected_id)
            .expect("event log must return a record for every in-bounds index");

        if record.event_id > expected_id {
            errors.push(VerificationError::GapInSequence {
                expected: expected_id,
                found: record.event_id,
            });
        } else if record.event_id < expected_id {
            errors.push(VerificationError::NonMonotonicId {
                event_id: record.event_id,
                expected: expected_id,
            });
        }

        if let CauseRef::Event(cause_id) = record.cause {
            if cause_id >= record.event_id {
                errors.push(VerificationError::FutureCauseRef {
                    event_id: record.event_id,
                    cause: record.cause,
                });
            }
            if event_log.get(cause_id).is_none() {
                errors.push(VerificationError::DanglingCauseRef {
                    event_id: record.event_id,
                    cause: record.cause,
                });
            }
        }

        if !cause_chain_reaches_explicit_root(event_log, record.event_id) {
            errors.push(VerificationError::OrphanEvent {
                event_id: record.event_id,
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn cause_chain_reaches_explicit_root(event_log: &EventLog, start_id: EventId) -> bool {
    let mut visited = BTreeSet::new();
    let mut current_id = start_id;

    loop {
        if !visited.insert(current_id) {
            return false;
        }

        let Some(record) = event_log.get(current_id) else {
            return false;
        };

        match record.cause {
            CauseRef::Bootstrap | CauseRef::SystemTick(_) | CauseRef::ExternalInput(_) => {
                return true;
            }
            CauseRef::Event(cause_id) => {
                current_id = cause_id;
            }
        }
    }
}

#[cfg(test)]
pub fn verify_event_covers_world_state(
    world: &World,
    event_log: &EventLog,
) -> Result<(), Vec<VerificationError>> {
    let mut errors = Vec::new();

    if let Err(mut structural_errors) = verify_completeness(event_log) {
        errors.append(&mut structural_errors);
    }

    let expected = ExpectedWorldState::from_event_log(event_log);
    let actual = ActualWorldState::from_world(world);

    diff_btree_map(
        "entity states",
        &expected.entity_states,
        &actual.entity_states,
        &mut errors,
    );
    diff_btree_map(
        "components",
        &expected.components,
        &actual.components,
        &mut errors,
    );
    diff_btree_set(
        "relations",
        &expected.relations,
        &actual.relations,
        &mut errors,
    );
    diff_btree_map(
        "reservations",
        &expected.reservations,
        &actual.reservations,
        &mut errors,
    );

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct ExpectedWorldState {
    entity_states: BTreeMap<EntityId, EntityKind>,
    components: BTreeMap<(EntityId, ComponentKind), ComponentValue>,
    relations: BTreeSet<RelationValue>,
    reservations: BTreeMap<ReservationId, ReservationRecord>,
}

#[cfg(test)]
impl ExpectedWorldState {
    fn from_event_log(event_log: &EventLog) -> Self {
        let mut entity_kinds = BTreeMap::<EntityId, EntityKind>::new();
        let mut archived = BTreeSet::<EntityId>::new();
        let mut components = BTreeMap::<(EntityId, ComponentKind), ComponentValue>::new();
        let mut relations = BTreeSet::<RelationValue>::new();
        let mut reservations = BTreeMap::<ReservationId, ReservationRecord>::new();

        for index in 0..event_log.len() {
            let expected_id = EventId(u64::try_from(index).expect("event count exceeds u64"));
            let record = event_log
                .get(expected_id)
                .expect("event log must return a record for every in-bounds index");

            for delta in &record.state_deltas {
                match delta {
                    StateDelta::Entity(EntityDelta::Created { entity, kind }) => {
                        entity_kinds.insert(*entity, *kind);
                        archived.remove(entity);
                    }
                    StateDelta::Entity(EntityDelta::Archived { entity, .. }) => {
                        archived.insert(*entity);
                    }
                    StateDelta::Component(ComponentDelta::Set {
                        entity,
                        component_kind,
                        after,
                        ..
                    }) => {
                        components.insert((*entity, *component_kind), after.clone());
                    }
                    StateDelta::Component(ComponentDelta::Removed {
                        entity,
                        component_kind,
                        ..
                    }) => {
                        components.remove(&(*entity, *component_kind));
                    }
                    StateDelta::Relation(RelationDelta::Added { relation, .. }) => {
                        relations.insert(relation.clone());
                    }
                    StateDelta::Relation(RelationDelta::Removed { relation, .. }) => {
                        relations.remove(relation);
                    }
                    StateDelta::Reservation(ReservationDelta::Created { reservation }) => {
                        reservations.insert(reservation.id, reservation.clone());
                    }
                    StateDelta::Reservation(ReservationDelta::Released { reservation }) => {
                        reservations.remove(&reservation.id);
                    }
                    StateDelta::Quantity(_) => {}
                }
            }
        }

        let live_entities = entity_kinds
            .into_iter()
            .filter(|(entity, _)| !archived.contains(entity))
            .collect::<BTreeMap<_, _>>();
        let live_ids = live_entities.keys().copied().collect::<BTreeSet<_>>();

        components.retain(|(entity, _), _| live_ids.contains(entity));
        relations.retain(|relation| relation_is_live(relation, &live_entities));
        reservations.retain(|_, reservation| {
            live_entities.contains_key(&reservation.entity)
                && live_entities.contains_key(&reservation.reserver)
        });

        Self {
            entity_states: live_entities,
            components,
            relations,
            reservations,
        }
    }
}

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct ActualWorldState {
    entity_states: BTreeMap<EntityId, EntityKind>,
    components: BTreeMap<(EntityId, ComponentKind), ComponentValue>,
    relations: BTreeSet<RelationValue>,
    reservations: BTreeMap<ReservationId, ReservationRecord>,
}

#[cfg(test)]
impl ActualWorldState {
    fn from_world(world: &World) -> Self {
        let mut entity_states = BTreeMap::new();
        let mut components = BTreeMap::new();
        let mut relations = BTreeSet::new();
        let mut reservations = BTreeMap::new();

        for entity in world.entities() {
            let kind = world
                .entity_kind(entity)
                .expect("live world entity must have a kind");
            if kind == EntityKind::Place {
                continue;
            }
            entity_states.insert(entity, kind);
            Self::collect_components(world, entity, &mut components);
            Self::collect_relations(world, entity, kind, &mut relations);
            Self::collect_reservations(world, entity, &mut reservations);
        }

        Self {
            entity_states,
            components,
            relations,
            reservations,
        }
    }

    fn collect_components(
        world: &World,
        entity: EntityId,
        components: &mut BTreeMap<(EntityId, ComponentKind), ComponentValue>,
    ) {
        for value in world.component_values(entity) {
            components.insert((entity, value.kind()), value);
        }
    }

    fn collect_relations(
        world: &World,
        entity: EntityId,
        kind: EntityKind,
        relations: &mut BTreeSet<RelationValue>,
    ) {
        if let Some(place) = world.effective_place(entity) {
            relations.insert(RelationValue::LocatedIn { entity, place });
        }
        if world.is_in_transit(entity) {
            relations.insert(RelationValue::InTransit { entity });
        }
        if let Some(container) = world.direct_container(entity) {
            relations.insert(RelationValue::ContainedBy { entity, container });
        }
        if let Some(holder) = world.possessor_of(entity) {
            relations.insert(RelationValue::PossessedBy { entity, holder });
        }
        if let Some(owner) = world.owner_of(entity) {
            relations.insert(RelationValue::OwnedBy { entity, owner });
        }
        for faction in world.factions_of(entity) {
            relations.insert(RelationValue::MemberOf {
                member: entity,
                faction,
            });
        }
        for (target, strength) in world.loyal_targets_of(entity) {
            relations.insert(RelationValue::LoyalTo {
                subject: entity,
                target,
                strength,
            });
        }
        for target in world.hostile_targets_of(entity) {
            relations.insert(RelationValue::HostileTo {
                subject: entity,
                target,
            });
        }

        if kind == EntityKind::Office {
            if let Some(holder) = world.office_holder(entity) {
                relations.insert(RelationValue::OfficeHolder {
                    office: entity,
                    holder,
                });
            }
        }
    }

    fn collect_reservations(
        world: &World,
        entity: EntityId,
        reservations: &mut BTreeMap<ReservationId, ReservationRecord>,
    ) {
        for reservation in world.reservations_for(entity) {
            reservations.insert(reservation.id, reservation);
        }
    }
}

#[cfg(test)]
fn relation_is_live(
    relation: &RelationValue,
    live_entities: &BTreeMap<EntityId, EntityKind>,
) -> bool {
    match relation {
        RelationValue::LocatedIn { entity, .. }
        | RelationValue::InTransit { entity }
        | RelationValue::ContainedBy { entity, .. }
        | RelationValue::PossessedBy { entity, .. }
        | RelationValue::OwnedBy { entity, .. } => live_entities.contains_key(entity),
        RelationValue::MemberOf { member, faction } => {
            live_entities.contains_key(member)
                && live_entities.get(faction) == Some(&EntityKind::Faction)
        }
        RelationValue::LoyalTo {
            subject, target, ..
        }
        | RelationValue::HostileTo { subject, target } => {
            live_entities.contains_key(subject) && live_entities.contains_key(target)
        }
        RelationValue::OfficeHolder { office, holder } => {
            live_entities.get(office) == Some(&EntityKind::Office)
                && live_entities.contains_key(holder)
        }
    }
}

#[cfg(test)]
fn diff_btree_map<K, V>(
    label: &str,
    expected: &BTreeMap<K, V>,
    actual: &BTreeMap<K, V>,
    errors: &mut Vec<VerificationError>,
) where
    K: Clone + Ord + std::fmt::Debug,
    V: Clone + Eq + std::fmt::Debug,
{
    for (key, expected_value) in expected {
        match actual.get(key) {
            Some(actual_value) if actual_value == expected_value => {}
            Some(actual_value) => errors.push(VerificationError::WorldStateMismatch {
                detail: format!(
                    "{label} mismatch for {key:?}: expected {expected_value:?}, found {actual_value:?}"
                ),
            }),
            None => errors.push(VerificationError::WorldStateMismatch {
                detail: format!("{label} missing expected entry {key:?} -> {expected_value:?}"),
            }),
        }
    }

    for (key, actual_value) in actual {
        if !expected.contains_key(key) {
            errors.push(VerificationError::WorldStateMismatch {
                detail: format!("{label} has unexpected entry {key:?} -> {actual_value:?}"),
            });
        }
    }
}

#[cfg(test)]
fn diff_btree_set<T>(
    label: &str,
    expected: &BTreeSet<T>,
    actual: &BTreeSet<T>,
    errors: &mut Vec<VerificationError>,
) where
    T: Clone + Ord + std::fmt::Debug,
{
    for value in expected {
        if !actual.contains(value) {
            errors.push(VerificationError::WorldStateMismatch {
                detail: format!("{label} missing expected entry {value:?}"),
            });
        }
    }

    for value in actual {
        if !expected.contains(value) {
            errors.push(VerificationError::WorldStateMismatch {
                detail: format!("{label} has unexpected entry {value:?}"),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{verify_completeness, verify_event_covers_world_state, VerificationError};
    use crate::{
        BodyPart, CauseRef, CommodityKind, Container, ControlSource, DeprivationKind, EntityId,
        EventId, EventLog, EventRecord, EventTag, LoadUnits, Quantity, Tick, TickRange, Topology,
        VisibilitySpec, WitnessData, World, WorldTxn, Wound, WoundCause, WoundList,
    };
    use std::collections::BTreeSet;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn empty_tags() -> BTreeSet<EventTag> {
        BTreeSet::from([EventTag::WorldMutation])
    }

    fn record(event_id: EventId, cause: CauseRef) -> EventRecord {
        EventRecord::new(
            event_id,
            Tick(event_id.0 + 1),
            cause,
            Some(entity(1)),
            vec![entity(2)],
            Some(entity(3)),
            Vec::new(),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
            empty_tags(),
        )
    }

    fn test_world() -> World {
        let mut topology = Topology::new();
        topology
            .add_place(
                entity(10),
                crate::Place {
                    name: "Square".to_string(),
                    capacity: None,
                    tags: BTreeSet::new(),
                },
            )
            .unwrap();
        topology
            .add_place(
                entity(11),
                crate::Place {
                    name: "Granary".to_string(),
                    capacity: None,
                    tags: BTreeSet::new(),
                },
            )
            .unwrap();
        World::new(topology).unwrap()
    }

    #[test]
    fn verify_completeness_accepts_an_empty_log() {
        let log = EventLog::new();

        assert_eq!(verify_completeness(&log), Ok(()));
    }

    #[test]
    fn verify_completeness_accepts_a_well_formed_log() {
        let log = EventLog::from_records_for_test(vec![
            record(EventId(0), CauseRef::Bootstrap),
            record(EventId(1), CauseRef::Event(EventId(0))),
            record(EventId(2), CauseRef::Event(EventId(1))),
        ]);

        assert_eq!(verify_completeness(&log), Ok(()));
    }

    #[test]
    fn verify_completeness_reports_a_dangling_cause_reference() {
        let log =
            EventLog::from_records_for_test(vec![record(EventId(0), CauseRef::Event(EventId(7)))]);

        let errors = verify_completeness(&log).unwrap_err();

        assert!(errors.contains(&VerificationError::DanglingCauseRef {
            event_id: EventId(0),
            cause: CauseRef::Event(EventId(7)),
        }));
        assert!(errors.contains(&VerificationError::FutureCauseRef {
            event_id: EventId(0),
            cause: CauseRef::Event(EventId(7)),
        }));
        assert!(errors.contains(&VerificationError::OrphanEvent {
            event_id: EventId(0),
        }));
    }

    #[test]
    fn verify_completeness_reports_a_non_monotonic_event_id() {
        let log = EventLog::from_records_for_test(vec![
            record(EventId(0), CauseRef::Bootstrap),
            record(EventId(0), CauseRef::Event(EventId(0))),
        ]);

        let errors = verify_completeness(&log).unwrap_err();

        assert!(errors.contains(&VerificationError::NonMonotonicId {
            event_id: EventId(0),
            expected: EventId(1),
        }));
    }

    #[test]
    fn verify_completeness_reports_a_gap_in_event_ids() {
        let log = EventLog::from_records_for_test(vec![
            record(EventId(0), CauseRef::Bootstrap),
            record(EventId(2), CauseRef::Event(EventId(0))),
        ]);

        let errors = verify_completeness(&log).unwrap_err();

        assert!(errors.contains(&VerificationError::GapInSequence {
            expected: EventId(1),
            found: EventId(2),
        }));
    }

    #[test]
    fn verify_completeness_reports_an_orphan_chain_even_without_other_errors() {
        let log = EventLog::from_records_for_test(vec![
            record(EventId(0), CauseRef::Bootstrap),
            record(EventId(1), CauseRef::Event(EventId(3))),
        ]);

        let errors = verify_completeness(&log).unwrap_err();

        assert!(errors.contains(&VerificationError::DanglingCauseRef {
            event_id: EventId(1),
            cause: CauseRef::Event(EventId(3)),
        }));
        assert!(errors.contains(&VerificationError::FutureCauseRef {
            event_id: EventId(1),
            cause: CauseRef::Event(EventId(3)),
        }));
        assert!(errors.contains(&VerificationError::OrphanEvent {
            event_id: EventId(1),
        }));
    }

    #[test]
    fn verify_completeness_reports_multiple_errors_without_short_circuiting() {
        let log = EventLog::from_records_for_test(vec![
            record(EventId(0), CauseRef::Bootstrap),
            record(EventId(3), CauseRef::Event(EventId(9))),
            record(EventId(1), CauseRef::Event(EventId(2))),
        ]);

        let errors = verify_completeness(&log).unwrap_err();

        assert!(errors.len() >= 5);
        assert!(errors.iter().any(|error| matches!(
            error,
            VerificationError::GapInSequence {
                expected: EventId(1),
                found: EventId(3)
            }
        )));
        assert!(errors.iter().any(|error| matches!(
            error,
            VerificationError::DanglingCauseRef {
                event_id: EventId(3),
                cause: CauseRef::Event(EventId(9))
            }
        )));
        assert!(errors.iter().any(|error| matches!(
            error,
            VerificationError::NonMonotonicId {
                event_id: EventId(1),
                expected: EventId(2)
            }
        )));
    }

    #[test]
    fn verify_event_covers_world_state_accepts_world_txn_mutations() {
        let mut world = test_world();
        let mut log = EventLog::new();

        let mut txn = WorldTxn::new(
            &mut world,
            Tick(5),
            CauseRef::Bootstrap,
            None,
            Some(entity(10)),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        txn.add_tag(EventTag::WorldMutation);
        let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        let item = txn
            .create_item_lot(CommodityKind::Bread, Quantity(3))
            .unwrap();
        let container = txn
            .create_container(Container {
                capacity: LoadUnits(10),
                allowed_commodities: None,
                allows_unique_items: true,
                allows_nested_containers: true,
            })
            .unwrap();
        txn.set_ground_location(agent, entity(10)).unwrap();
        txn.set_ground_location(container, entity(10)).unwrap();
        txn.put_into_container(item, container).unwrap();
        let reservation = txn
            .try_reserve(item, agent, TickRange::new(Tick(6), Tick(8)).unwrap())
            .unwrap();
        txn.release_reservation(reservation).unwrap();
        txn.commit(&mut log);

        assert_eq!(verify_event_covers_world_state(&world, &log), Ok(()));
    }

    #[test]
    fn verify_event_covers_world_state_detects_out_of_band_component_mutation() {
        let mut world = test_world();
        let mut log = EventLog::new();

        let mut txn = WorldTxn::new(
            &mut world,
            Tick(5),
            CauseRef::Bootstrap,
            None,
            Some(entity(10)),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        txn.add_tag(EventTag::WorldMutation);
        let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_ground_location(agent, entity(10)).unwrap();
        txn.commit(&mut log);

        world
            .get_component_name_mut(agent)
            .expect("agent name component should exist")
            .0 = "Bypass".to_string();

        let errors = verify_event_covers_world_state(&world, &log).unwrap_err();

        assert!(errors.iter().any(|error| matches!(
            error,
            VerificationError::WorldStateMismatch { detail }
                if detail.contains("components mismatch")
                    && detail.contains("Bypass")
        )));
    }

    #[test]
    fn verify_event_covers_world_state_detects_out_of_band_relation_mutation() {
        let mut world = test_world();
        let mut log = EventLog::new();

        let mut txn = WorldTxn::new(
            &mut world,
            Tick(5),
            CauseRef::Bootstrap,
            None,
            Some(entity(10)),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        txn.add_tag(EventTag::WorldMutation);
        let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        let faction = txn.create_faction("River Pact").unwrap();
        txn.set_ground_location(agent, entity(10)).unwrap();
        txn.commit(&mut log);

        world.add_member(agent, faction).unwrap();

        let errors = verify_event_covers_world_state(&world, &log).unwrap_err();

        assert!(errors.iter().any(|error| matches!(
            error,
            VerificationError::WorldStateMismatch { detail }
                if detail.contains("relations has unexpected entry")
        )));
    }

    #[test]
    fn verify_event_covers_world_state_detects_out_of_band_wound_component_mutation() {
        let mut world = test_world();
        let mut log = EventLog::new();

        let mut txn = WorldTxn::new(
            &mut world,
            Tick(5),
            CauseRef::Bootstrap,
            None,
            Some(entity(10)),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        txn.add_tag(EventTag::WorldMutation);
        let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_ground_location(agent, entity(10)).unwrap();
        txn.commit(&mut log);

        world
            .insert_component_wound_list(
                agent,
                WoundList {
                    wounds: vec![Wound {
                        id: crate::WoundId(1),
                        body_part: BodyPart::Torso,
                        cause: WoundCause::Deprivation(DeprivationKind::Starvation),
                        severity: crate::Permille::new(600).unwrap(),
                        inflicted_at: Tick(8),
                        bleed_rate_per_tick: crate::Permille::new(0).unwrap(),
                    }],
                },
            )
            .unwrap();

        let errors = verify_event_covers_world_state(&world, &log).unwrap_err();

        assert!(errors.iter().any(|error| matches!(
            error,
            VerificationError::WorldStateMismatch { detail }
                if detail.contains("components has unexpected entry")
                    && detail.contains("WoundList")
        )));
    }
}
