use worldwake_core::{ControlSource, EntityId, World, WorldTxn};

use crate::{ConsumableEffect, Constraint, Precondition};

pub(crate) fn evaluate_constraint_authoritatively(
    world: &World,
    constraint: &Constraint,
    actor: EntityId,
) -> bool {
    match constraint {
        Constraint::ActorAlive => world.is_alive(actor),
        Constraint::ActorHasControl => has_control(world, actor),
        Constraint::ActorAtPlace(place) => world.effective_place(actor) == Some(*place),
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
        Precondition::TargetAtActorPlace(index) => {
            let Some(target) = targets.get(usize::from(index)).copied() else {
                return false;
            };
            let Some(actor_place) = world.effective_place(actor) else {
                return false;
            };
            world.effective_place(target) == Some(actor_place)
        }
        Precondition::TargetKind { target_index, kind } => targets
            .get(usize::from(target_index))
            .is_some_and(|target| world.entity_kind(*target) == Some(kind)),
        Precondition::TargetCommodity { target_index, kind } => targets
            .get(usize::from(target_index))
            .and_then(|target| world.get_component_item_lot(*target))
            .is_some_and(|lot| lot.commodity == kind),
        Precondition::TargetHasConsumableEffect { target_index, effect } => targets
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
    use crate::{ConsumableEffect, Constraint, Precondition};
    use worldwake_core::{
        build_prototype_world, CauseRef, CommodityKind, ControlSource, EntityKind, EventLog,
        Quantity, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
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
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
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
        assert!(world.can_exercise_control(actor, bag).is_ok());
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
}
