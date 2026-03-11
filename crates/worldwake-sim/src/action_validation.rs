use worldwake_core::{is_incapacitated, ControlSource, EntityId, EntityKind, World, WorldTxn};

use crate::{Constraint, ConsumableEffect, Precondition};

pub(crate) fn evaluate_constraint_authoritatively(
    world: &World,
    constraint: &Constraint,
    actor: EntityId,
) -> bool {
    match constraint {
        Constraint::ActorAlive => world.is_alive(actor),
        Constraint::ActorNotIncapacitated => world
            .get_component_wound_list(actor)
            .zip(world.get_component_combat_profile(actor))
            .is_none_or(|(wounds, profile)| !is_incapacitated(wounds, profile)),
        Constraint::ActorNotDead => world.get_component_dead_at(actor).is_none(),
        Constraint::ActorHasControl => has_control(world, actor),
        Constraint::ActorNotInTransit => !world.is_in_transit(actor),
        Constraint::ActorAtPlace(place) => world.effective_place(actor) == Some(*place),
        Constraint::ActorKnowsRecipe(recipe) => world
            .get_component_known_recipes(actor)
            .is_some_and(|known| known.recipes.contains(recipe)),
        Constraint::ActorHasUniqueItemKind { kind, min_count } => {
            world.controlled_unique_item_count(actor, *kind) >= *min_count
        }
        Constraint::ActorHasCommodity { kind, min_qty } => {
            world.controlled_commodity_quantity(actor, *kind) >= *min_qty
        }
        Constraint::ActorKind(kind) => world.entity_kind(actor) == Some(*kind),
    }
}

pub(crate) fn evaluate_precondition_authoritatively(
    world: &World,
    precondition: Precondition,
    actor: EntityId,
    targets: &[EntityId],
) -> bool {
    match precondition {
        Precondition::ActorAlive => world.is_alive(actor),
        Precondition::ActorCanControlTarget(index) => targets
            .get(usize::from(index))
            .is_some_and(|target| world.can_exercise_control(actor, *target).is_ok()),
        Precondition::TargetExists(index) => targets
            .get(usize::from(index))
            .is_some_and(|target| world.is_alive(*target)),
        Precondition::TargetAlive(index) => targets
            .get(usize::from(index))
            .is_some_and(|target| world.get_component_dead_at(*target).is_none()),
        Precondition::TargetDead(index) => targets
            .get(usize::from(index))
            .is_some_and(|target| world.get_component_dead_at(*target).is_some()),
        Precondition::TargetIsAgent(index) => targets
            .get(usize::from(index))
            .is_some_and(|target| world.entity_kind(*target) == Some(EntityKind::Agent)),
        Precondition::TargetAtActorPlace(index) => {
            let Some(target) = targets.get(usize::from(index)).copied() else {
                return false;
            };
            let Some(actor_place) = world.effective_place(actor) else {
                return false;
            };
            world.effective_place(target) == Some(actor_place)
        }
        Precondition::TargetAdjacentToActor(index) => {
            let Some(target) = targets.get(usize::from(index)).copied() else {
                return false;
            };
            let Some(actor_place) = world.effective_place(actor) else {
                return false;
            };
            world
                .topology()
                .unique_direct_edge(actor_place, target)
                .is_ok_and(|edge| edge.is_some())
        }
        Precondition::TargetKind { target_index, kind } => targets
            .get(usize::from(target_index))
            .is_some_and(|target| world.entity_kind(*target) == Some(kind)),
        Precondition::TargetCommodity { target_index, kind } => targets
            .get(usize::from(target_index))
            .and_then(|target| world.get_component_item_lot(*target))
            .is_some_and(|lot| lot.commodity == kind),
        Precondition::TargetHasWorkstationTag { target_index, tag } => targets
            .get(usize::from(target_index))
            .and_then(|target| world.get_component_workstation_marker(*target))
            .is_some_and(|marker| marker.0 == tag),
        Precondition::TargetHasResourceSource {
            target_index,
            commodity,
            min_available,
        } => targets
            .get(usize::from(target_index))
            .and_then(|target| world.get_component_resource_source(*target))
            .is_some_and(|source| {
                source.commodity == commodity && source.available_quantity >= min_available
            }),
        Precondition::TargetNotInContainer(target_index) => targets
            .get(usize::from(target_index))
            .is_some_and(|target| world.direct_container(*target).is_none()),
        Precondition::TargetUnpossessed(target_index) => targets
            .get(usize::from(target_index))
            .is_some_and(|target| world.possessor_of(*target).is_none()),
        Precondition::TargetDirectlyPossessedByActor(target_index) => targets
            .get(usize::from(target_index))
            .is_some_and(|target| world.possessor_of(*target) == Some(actor)),
        Precondition::TargetLacksProductionJob(target_index) => targets
            .get(usize::from(target_index))
            .is_some_and(|target| !world.has_component_production_job(*target)),
        Precondition::TargetHasConsumableEffect {
            target_index,
            effect,
        } => targets
            .get(usize::from(target_index))
            .and_then(|target| world.get_component_item_lot(*target))
            .and_then(|lot| lot.commodity.spec().consumable_profile)
            .is_some_and(|profile| match effect {
                ConsumableEffect::Hunger => profile.hunger_relief_per_unit.value() > 0,
                ConsumableEffect::Thirst => profile.thirst_relief_per_unit.value() > 0,
            }),
    }
}

pub(crate) fn evaluate_txn_precondition_authoritatively(
    txn: &WorldTxn<'_>,
    precondition: Precondition,
    actor: EntityId,
    targets: &[EntityId],
) -> bool {
    evaluate_precondition_authoritatively(txn, precondition, actor, targets)
}

fn has_control(world: &World, entity: EntityId) -> bool {
    world
        .get_component_agent_data(entity)
        .is_some_and(|agent_data| agent_data.control_source != ControlSource::None)
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_constraint_authoritatively, evaluate_precondition_authoritatively,
        evaluate_txn_precondition_authoritatively,
    };
    use crate::{Constraint, ConsumableEffect, Precondition};
    use worldwake_core::{
        build_prototype_world, CauseRef, CommodityKind, ControlSource, EntityKind, EventLog,
        Permille, Quantity, RecipeId, ResourceSource, Tick, VisibilitySpec, WitnessData,
        WorkstationMarker, WorkstationTag, World, WorldTxn,
    };

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        )
    }

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    #[test]
    fn authoritative_constraint_checks_read_world_state_directly() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            commit_txn(txn);
            actor
        };

        assert!(evaluate_constraint_authoritatively(
            &world,
            &Constraint::ActorAlive,
            actor,
        ));
        assert!(evaluate_constraint_authoritatively(
            &world,
            &Constraint::ActorHasControl,
            actor,
        ));
        assert!(evaluate_constraint_authoritatively(
            &world,
            &Constraint::ActorKind(EntityKind::Agent),
            actor,
        ));
        assert!(evaluate_constraint_authoritatively(
            &world,
            &Constraint::ActorNotInTransit,
            actor,
        ));
        assert!(!evaluate_constraint_authoritatively(
            &world,
            &Constraint::ActorHasCommodity {
                kind: CommodityKind::Bread,
                min_qty: Quantity(1),
            },
            actor,
        ));
    }

    #[test]
    fn authoritative_combat_liveness_checks_cover_dead_and_incapacitated_variants() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (healthy, incapacitated, dead) = {
            let mut txn = new_txn(&mut world, 1);
            let healthy = txn.create_agent("Healthy", ControlSource::Ai).unwrap();
            let incapacitated = txn.create_agent("Incapacitated", ControlSource::Ai).unwrap();
            let dead = txn.create_agent("Dead", ControlSource::Ai).unwrap();
            for entity in [healthy, incapacitated, dead] {
                txn.set_ground_location(entity, place).unwrap();
                txn.set_component_combat_profile(
                    entity,
                    worldwake_core::CombatProfile::new(
                        Permille::new(1000).unwrap(),
                        Permille::new(700).unwrap(),
                        Permille::new(600).unwrap(),
                        Permille::new(550).unwrap(),
                        Permille::new(75).unwrap(),
                        Permille::new(20).unwrap(),
                        Permille::new(15).unwrap(),
                        Permille::new(120).unwrap(),
                        Permille::new(30).unwrap(),
                        std::num::NonZeroU32::new(6).unwrap(),
                    ),
                )
                .unwrap();
            }
            txn.set_component_wound_list(healthy, worldwake_core::WoundList::default())
                .unwrap();
            txn.set_component_wound_list(
                incapacitated,
                worldwake_core::WoundList {
                    wounds: vec![worldwake_core::Wound {
                        body_part: worldwake_core::BodyPart::Torso,
                        cause: worldwake_core::WoundCause::Deprivation(
                            worldwake_core::DeprivationKind::Starvation,
                        ),
                        severity: Permille::new(700).unwrap(),
                        inflicted_at: Tick(1),
                        bleed_rate_per_tick: Permille::new(0).unwrap(),
                    }],
                },
            )
            .unwrap();
            txn.set_component_wound_list(dead, worldwake_core::WoundList::default())
                .unwrap();
            txn.set_component_dead_at(dead, worldwake_core::DeadAt(Tick(2)))
                .unwrap();
            commit_txn(txn);
            (healthy, incapacitated, dead)
        };

        assert!(evaluate_constraint_authoritatively(
            &world,
            &Constraint::ActorNotDead,
            healthy,
        ));
        assert!(evaluate_constraint_authoritatively(
            &world,
            &Constraint::ActorNotIncapacitated,
            healthy,
        ));
        assert!(!evaluate_constraint_authoritatively(
            &world,
            &Constraint::ActorNotIncapacitated,
            incapacitated,
        ));
        assert!(!evaluate_constraint_authoritatively(
            &world,
            &Constraint::ActorNotDead,
            dead,
        ));
    }

    #[test]
    fn authoritative_preconditions_validate_control_and_consumable_requirements() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (actor, bag, bread, medicine) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let bag = txn
                .create_container(worldwake_core::Container {
                    capacity: worldwake_core::LoadUnits(10),
                    allowed_commodities: None,
                    allows_unique_items: true,
                    allows_nested_containers: true,
                })
                .unwrap();
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            let medicine = txn
                .create_item_lot(CommodityKind::Medicine, Quantity(1))
                .unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(bag, place).unwrap();
            txn.set_possessor(bag, actor).unwrap();
            txn.put_into_container(bread, bag).unwrap();
            txn.put_into_container(medicine, bag).unwrap();
            commit_txn(txn);
            (actor, bag, bread, medicine)
        };

        assert!(evaluate_precondition_authoritatively(
            &world,
            Precondition::ActorCanControlTarget(0),
            actor,
            &[bread],
        ));
        assert!(evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetCommodity {
                target_index: 0,
                kind: CommodityKind::Bread,
            },
            actor,
            &[bread],
        ));
        assert!(evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetHasConsumableEffect {
                target_index: 0,
                effect: ConsumableEffect::Hunger,
            },
            actor,
            &[bread],
        ));
        assert!(!evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetHasConsumableEffect {
                target_index: 0,
                effect: ConsumableEffect::Thirst,
            },
            actor,
            &[medicine],
        ));
        assert!(evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetDirectlyPossessedByActor(0),
            actor,
            &[bag],
        ));
        assert!(evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetUnpossessed(0),
            actor,
            &[bread],
        ));
        assert!(!evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetNotInContainer(0),
            actor,
            &[bread],
        ));
        assert!(evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetNotInContainer(0),
            actor,
            &[bag],
        ));
        assert!(!evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetUnpossessed(0),
            actor,
            &[bag],
        ));
        assert!(world.can_exercise_control(actor, bag).is_ok());
    }

    #[test]
    fn authoritative_target_liveness_and_agent_preconditions_check_entity_state() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (actor, living_agent, dead_agent, facility) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Actor", ControlSource::Ai).unwrap();
            let living_agent = txn.create_agent("Living", ControlSource::Ai).unwrap();
            let dead_agent = txn.create_agent("Dead", ControlSource::Ai).unwrap();
            let facility = txn.create_entity(EntityKind::Facility);
            for entity in [actor, living_agent, dead_agent, facility] {
                txn.set_ground_location(entity, place).unwrap();
            }
            txn.set_component_dead_at(dead_agent, worldwake_core::DeadAt(Tick(2)))
                .unwrap();
            commit_txn(txn);
            (actor, living_agent, dead_agent, facility)
        };

        assert!(evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetAlive(0),
            actor,
            &[living_agent],
        ));
        assert!(!evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetAlive(0),
            actor,
            &[dead_agent],
        ));
        assert!(evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetDead(0),
            actor,
            &[dead_agent],
        ));
        assert!(!evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetDead(0),
            actor,
            &[living_agent],
        ));
        assert!(evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetIsAgent(0),
            actor,
            &[living_agent],
        ));
        assert!(!evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetIsAgent(0),
            actor,
            &[facility],
        ));
    }

    #[test]
    fn authoritative_txn_precondition_reads_staged_world_state() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, target, place_a, place_b) = {
            let places = world.topology().place_ids().collect::<Vec<_>>();
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let target = txn
                .create_container(worldwake_core::Container {
                    capacity: worldwake_core::LoadUnits(10),
                    allowed_commodities: None,
                    allows_unique_items: true,
                    allows_nested_containers: true,
                })
                .unwrap();
            txn.set_ground_location(actor, places[0]).unwrap();
            txn.set_ground_location(target, places[0]).unwrap();
            commit_txn(txn);
            (actor, target, places[0], places[1])
        };

        assert!(evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetAtActorPlace(0),
            actor,
            &[target],
        ));
        assert_eq!(world.effective_place(actor), Some(place_a));
        assert_eq!(world.effective_place(target), Some(place_a));

        let mut txn = new_txn(&mut world, 2);
        txn.set_ground_location(target, place_b).unwrap();
        assert!(!evaluate_txn_precondition_authoritatively(
            &txn,
            Precondition::TargetAtActorPlace(0),
            actor,
            &[target],
        ));
    }

    #[test]
    fn authoritative_travel_checks_cover_adjacency_and_transit_state() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, places[0]).unwrap();
            commit_txn(txn);
            actor
        };
        let destination = world.topology().neighbors(places[0])[0];

        assert!(evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetAdjacentToActor(0),
            actor,
            &[destination],
        ));

        let mut txn = new_txn(&mut world, 2);
        txn.set_in_transit(actor).unwrap();
        assert!(!evaluate_txn_precondition_authoritatively(
            &txn,
            Precondition::TargetAdjacentToActor(0),
            actor,
            &[destination],
        ));
        assert!(!evaluate_constraint_authoritatively(
            &txn,
            &Constraint::ActorNotInTransit,
            actor,
        ));
    }

    #[test]
    fn authoritative_harvest_semantics_cover_recipe_workstation_and_source_checks() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (actor, workstation) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let workstation = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(workstation, place).unwrap();
            txn.set_component_known_recipes(
                actor,
                worldwake_core::KnownRecipes::with([RecipeId(7)]),
            )
            .unwrap();
            txn.set_component_workstation_marker(
                workstation,
                WorkstationMarker(WorkstationTag::OrchardRow),
            )
            .unwrap();
            txn.set_component_resource_source(
                workstation,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(3),
                    max_quantity: Quantity(6),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                },
            )
            .unwrap();
            commit_txn(txn);
            (actor, workstation)
        };

        assert!(evaluate_constraint_authoritatively(
            &world,
            &Constraint::ActorKnowsRecipe(RecipeId(7)),
            actor,
        ));
        assert!(evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetHasWorkstationTag {
                target_index: 0,
                tag: WorkstationTag::OrchardRow,
            },
            actor,
            &[workstation],
        ));
        assert!(evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetHasResourceSource {
                target_index: 0,
                commodity: CommodityKind::Apple,
                min_available: Quantity(2),
            },
            actor,
            &[workstation],
        ));
        assert!(!evaluate_precondition_authoritatively(
            &world,
            Precondition::TargetHasResourceSource {
                target_index: 0,
                commodity: CommodityKind::Apple,
                min_available: Quantity(4),
            },
            actor,
            &[workstation],
        ));
    }
}
