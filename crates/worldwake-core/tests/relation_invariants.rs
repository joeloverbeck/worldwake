use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{
    test_utils::deterministic_seed, CommodityKind, Container, ControlSource, EntityId, LoadUnits,
    Place, PlaceTag, Quantity, Tick, TickRange, Topology, UniqueItemKind, World, WorldError,
};

const SEED_COUNT: u8 = 5;
const ITERATIONS: usize = 250;

fn entity(slot: u32) -> EntityId {
    EntityId {
        slot,
        generation: 0,
    }
}

fn test_topology() -> Topology {
    let mut topology = Topology::new();
    for (slot, name, tag) in [
        (2, "Farm", PlaceTag::Farm),
        (5, "Square", PlaceTag::Village),
        (7, "Store", PlaceTag::Store),
        (11, "Camp", PlaceTag::Camp),
    ] {
        topology
            .add_place(
                entity(slot),
                Place {
                    name: name.to_string(),
                    capacity: None,
                    tags: BTreeSet::from([tag]),
                },
            )
            .unwrap();
    }
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

fn seeded_rng(offset: u8) -> ChaCha8Rng {
    let mut seed = deterministic_seed().0;
    seed[0] = offset;
    seed[31] = offset.wrapping_mul(17);
    ChaCha8Rng::from_seed(seed)
}

fn pick_index(rng: &mut ChaCha8Rng, len: usize) -> usize {
    (rng.next_u32() as usize) % len
}

fn pick_entity(rng: &mut ChaCha8Rng, entities: &[EntityId]) -> EntityId {
    entities[pick_index(rng, entities.len())]
}

fn pick_place(rng: &mut ChaCha8Rng, places: &[EntityId]) -> EntityId {
    places[pick_index(rng, places.len())]
}

fn random_range(rng: &mut ChaCha8Rng) -> TickRange {
    let start = u64::from(rng.next_u32() % 25);
    let len = 1 + u64::from(rng.next_u32() % 6);
    TickRange::new(Tick(start), Tick(start + len)).unwrap()
}

fn mark_entity_and_descendants_placed(
    world: &World,
    entity: EntityId,
    placed_entities: &mut BTreeSet<EntityId>,
) {
    placed_entities.insert(entity);
    placed_entities.extend(world.recursive_contents_of(entity));
}

fn assert_placement_invariants(
    world: &World,
    places: &[EntityId],
    containers: &[EntityId],
    entities: &[EntityId],
    expected_placed: &BTreeSet<EntityId>,
) {
    let mut seen_places = BTreeMap::new();

    for place in places {
        let effective_entities = world.entities_effectively_at(*place);
        let ground_entities = world.ground_entities_at(*place);
        let effective_set = effective_entities.iter().copied().collect::<BTreeSet<_>>();

        for entity in &ground_entities {
            assert!(
                effective_set.contains(entity),
                "ground entity {entity} missing from entities_effectively_at({place})"
            );
        }

        for entity in effective_entities {
            let prior = seen_places.insert(entity, *place);
            assert!(
                prior.is_none(),
                "entity {entity} appeared at both {prior:?} and {place:?}"
            );
            assert_eq!(world.effective_place(entity), Some(*place));
        }
    }

    for entity in expected_placed {
        let place = world.effective_place(*entity);
        assert!(
            place.is_some(),
            "expected placed entity {entity} lost its effective place"
        );
        assert!(
            !world.is_in_transit(*entity),
            "placed entity {entity} remained marked in transit"
        );
        assert_eq!(seen_places.get(entity), place.as_ref());
    }

    for entity in entities {
        if !expected_placed.contains(entity) {
            assert_eq!(world.effective_place(*entity), None);
            assert!(
                world.is_in_transit(*entity),
                "unplaced physical entity {entity} lost explicit transit state"
            );
        }
    }

    for container in containers {
        let Some(container_place) = world.effective_place(*container) else {
            continue;
        };

        for descendant in world.recursive_contents_of(*container) {
            assert_eq!(
                world.effective_place(descendant),
                Some(container_place),
                "descendant {descendant} diverged from container {container} place"
            );
        }
    }
}

fn assert_reservation_invariants(world: &World, reservable_entities: &[EntityId]) {
    for entity in reservable_entities {
        let reservations = world.reservations_for(*entity);

        for window in reservations.windows(2) {
            assert!(window[0].id < window[1].id);
        }

        for (index, reservation) in reservations.iter().enumerate() {
            for other in reservations.iter().skip(index + 1) {
                assert!(
                    !reservation.range.overlaps(&other.range),
                    "entity {entity} has overlapping reservations {} {} and {} {}",
                    reservation.id,
                    reservation.range,
                    other.id,
                    other.range
                );
            }
        }
    }
}

fn assert_containment_invariants(world: &World, containers: &[EntityId]) {
    for container in containers {
        let mut visited = BTreeSet::new();
        let mut current = *container;
        let mut depth = 0usize;

        while let Some(parent) = world.direct_container(current) {
            assert!(
                visited.insert(parent),
                "containment chain for {container} revisited {parent}"
            );
            depth += 1;
            assert!(
                depth <= containers.len(),
                "containment depth {} exceeded container count {}",
                depth,
                containers.len()
            );
            assert_eq!(
                world.effective_place(current),
                world.effective_place(parent),
                "contained entity {current} diverged from parent {parent} effective place"
            );
            current = parent;
        }
    }
}

#[test]
fn randomized_moves_preserve_unique_effective_locations_for_explicitly_placed_entities() {
    for seed_offset in 0..SEED_COUNT {
        let mut world = World::new(test_topology()).unwrap();
        let mut rng = seeded_rng(seed_offset);
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let containers = (0_u64..4)
            .map(|index| {
                world
                    .create_container(open_container(100), Tick(index + 1))
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let items = [
            world.create_agent("Aster", ControlSource::Ai, Tick(10)).unwrap(),
            world
                .create_item_lot(CommodityKind::Bread, Quantity(3), Tick(11))
                .unwrap(),
            world
                .create_item_lot(CommodityKind::Coin, Quantity(5), Tick(12))
                .unwrap(),
            world
                .create_unique_item(
                    UniqueItemKind::SimpleTool,
                    Some("Hammer"),
                    BTreeMap::new(),
                    Tick(13),
                )
                .unwrap(),
        ];
        let entities = containers
            .iter()
            .chain(items.iter())
            .copied()
            .collect::<Vec<_>>();
        let mut expected_placed = BTreeSet::new();

        for container in &containers {
            let place = pick_place(&mut rng, &places);
            world.set_ground_location(*container, place).unwrap();
            mark_entity_and_descendants_placed(&world, *container, &mut expected_placed);
        }

        assert_placement_invariants(&world, &places, &containers, &entities, &expected_placed);

        for _ in 0..ITERATIONS {
            match rng.next_u32() % 4 {
                0 => {
                    let entity = pick_entity(&mut rng, &entities);
                    let place = pick_place(&mut rng, &places);
                    world.set_ground_location(entity, place).unwrap();
                    mark_entity_and_descendants_placed(&world, entity, &mut expected_placed);
                }
                1 => {
                    let entity = pick_entity(&mut rng, &entities);
                    let container = pick_entity(&mut rng, &containers);
                    let would_cycle =
                        entity == container || world.recursive_contents_of(entity).contains(&container);

                    let result = world.put_into_container(entity, container);
                    if would_cycle {
                        assert!(matches!(
                            result,
                            Err(WorldError::ContainmentCycle {
                                entity: actual_entity,
                                container: actual_container,
                            }) if actual_entity == entity && actual_container == container
                        ));
                    } else {
                        result.unwrap();
                        mark_entity_and_descendants_placed(
                            &world,
                            entity,
                            &mut expected_placed,
                        );
                    }
                }
                2 => {
                    let container = pick_entity(&mut rng, &containers);
                    let place = pick_place(&mut rng, &places);
                    world.move_container_subtree(container, place).unwrap();
                    mark_entity_and_descendants_placed(&world, container, &mut expected_placed);
                }
                _ => {
                    let entity = pick_entity(&mut rng, &entities);
                    let was_contained = world.direct_container(entity).is_some();
                    let result = world.remove_from_container(entity);

                    if was_contained {
                        result.unwrap();
                    } else {
                        assert!(matches!(result, Err(WorldError::PreconditionFailed(_))));
                    }
                }
            }

            assert_placement_invariants(&world, &places, &containers, &entities, &expected_placed);
        }
    }
}

#[test]
fn randomized_reservations_preserve_exclusivity() {
    for seed_offset in 0..SEED_COUNT {
        let mut world = World::new(Topology::new()).unwrap();
        let mut rng = seeded_rng(seed_offset.wrapping_add(32));
        let reservers = vec![
            world.create_agent("Aster", ControlSource::Ai, Tick(1)).unwrap(),
            world.create_agent("Bram", ControlSource::Human, Tick(2)).unwrap(),
            world.create_agent("Cora", ControlSource::Ai, Tick(3)).unwrap(),
        ];
        let reservable_entities = vec![
            world
                .create_item_lot(CommodityKind::Medicine, Quantity(1), Tick(4))
                .unwrap(),
            world
                .create_item_lot(CommodityKind::Bread, Quantity(2), Tick(5))
                .unwrap(),
            world
                .create_unique_item(
                    UniqueItemKind::Contract,
                    Some("Lease"),
                    BTreeMap::new(),
                    Tick(6),
                )
                .unwrap(),
        ];

        for _ in 0..ITERATIONS {
            let mut active_reservations = reservable_entities
                .iter()
                .flat_map(|entity| world.reservations_for(*entity))
                .collect::<Vec<_>>();

            if !active_reservations.is_empty() && rng.next_u32().is_multiple_of(3) {
                let released = active_reservations.swap_remove(pick_index(&mut rng, active_reservations.len()));
                world.release_reservation(released.id).unwrap();
            } else {
                let entity = pick_entity(&mut rng, &reservable_entities);
                let reserver = pick_entity(&mut rng, &reservers);
                let range = random_range(&mut rng);
                let existing = world.reservations_for(entity);
                let overlaps_existing = existing
                    .iter()
                    .any(|reservation| reservation.range.overlaps(&range));

                let result = world.try_reserve(entity, reserver, range);
                if overlaps_existing {
                    assert!(matches!(
                        result,
                        Err(WorldError::ConflictingReservation { entity: actual }) if actual == entity
                    ));
                } else {
                    result.unwrap();
                }
            }

            assert_reservation_invariants(&world, &reservable_entities);
        }
    }
}

#[test]
fn randomized_container_nesting_preserves_acyclic_containment() {
    for seed_offset in 0..SEED_COUNT {
        let mut world = World::new(test_topology()).unwrap();
        let mut rng = seeded_rng(seed_offset.wrapping_add(64));
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let containers = (0_u64..6)
            .map(|index| {
                world
                    .create_container(open_container(100), Tick(index + 1))
                    .unwrap()
            })
            .collect::<Vec<_>>();

        for container in &containers {
            world
                .set_ground_location(*container, pick_place(&mut rng, &places))
                .unwrap();
        }

        assert_containment_invariants(&world, &containers);

        for _ in 0..ITERATIONS {
            match rng.next_u32() % 3 {
                0 => {
                    let entity = pick_entity(&mut rng, &containers);
                    let container = pick_entity(&mut rng, &containers);
                    let would_cycle =
                        entity == container || world.recursive_contents_of(entity).contains(&container);

                    let result = world.put_into_container(entity, container);
                    if would_cycle {
                        assert!(matches!(
                            result,
                            Err(WorldError::ContainmentCycle {
                                entity: actual_entity,
                                container: actual_container,
                            }) if actual_entity == entity && actual_container == container
                        ));
                    } else {
                        result.unwrap();
                    }
                }
                1 => {
                    let entity = pick_entity(&mut rng, &containers);
                    let was_contained = world.direct_container(entity).is_some();
                    let result = world.remove_from_container(entity);

                    if was_contained {
                        result.unwrap();
                    } else {
                        assert!(matches!(result, Err(WorldError::PreconditionFailed(_))));
                    }
                }
                _ => {
                    let container = pick_entity(&mut rng, &containers);
                    let place = pick_place(&mut rng, &places);
                    world.move_container_subtree(container, place).unwrap();
                }
            }

            assert_containment_invariants(&world, &containers);
        }
    }
}
