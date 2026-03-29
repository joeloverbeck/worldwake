use crate::inventory::carried_entities;
use std::collections::BTreeSet;
use worldwake_core::{
    current_container_load, load_of_entity, ActionDefId, ActionDomain, BanditCamp,
    BanditCampProfile, BodyCostPerTick, Container, EntityId, EntityKind, EventTag, LoadUnits,
    PlaceTag, VisibilitySpec, World, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerId,
    ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress, CommitOutcome,
    Constraint, DeterministicRng, DurationExpr, EstablishCampActionPayload, Interruptibility,
    Precondition, RuntimeBeliefView, TargetSpec,
};

pub fn register_establish_camp_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(
            start_establish_camp,
            tick_establish_camp,
            commit_establish_camp,
            abort_establish_camp,
        )
        .with_affordance_payloads(enumerate_establish_camp_payloads)
        .with_payload_override_validator(validate_establish_camp_payload_override)
        .with_authoritative_payload_validator(validate_establish_camp_payload_authoritatively),
    );
    let id = ActionDefId(defs.len() as u32);
    defs.register(establish_camp_action_def(id, handler))
}

fn establish_camp_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "establish_camp".to_string(),
        domain: ActionDomain::Generic,
        actor_constraints: vec![
            Constraint::ActorAlive,
            Constraint::ActorNotIncapacitated,
            Constraint::ActorHasControl,
            Constraint::ActorNotInTransit,
        ],
        targets: vec![TargetSpec::ActorPlace],
        preconditions: vec![
            Precondition::TargetExists(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Place,
            },
        ],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::BanditCampEstablishmentProfile,
        body_cost_per_tick: BodyCostPerTick::zero(),
        interruptibility: Interruptibility::InterruptibleWithPenalty,
        commit_conditions: vec![
            Precondition::TargetExists(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Place,
            },
        ],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::WorldMutation]),
        payload: ActionPayload::None,
        handler,
    }
}

#[derive(Clone)]
struct ValidatedEstablishCamp {
    place: EntityId,
    faction: EntityId,
    profile_origin: EntityId,
    profile: BanditCampProfile,
    carried_food_lots: Vec<EntityId>,
    existing_place_camp: Option<BanditCamp>,
}

fn payload_faction(payload: &ActionPayload) -> Result<EntityId, ActionError> {
    payload
        .as_establish_camp()
        .map(|payload| payload.faction)
        .ok_or_else(|| {
            ActionError::PreconditionFailed(
                "establish_camp requires ActionPayload::EstablishCamp".to_string(),
            )
        })
}

fn canonical_bandit_camp_profile(
    world: &World,
    faction: EntityId,
) -> Result<(EntityId, BanditCampProfile), ActionError> {
    let mut profiles = world
        .query_bandit_camp_profile()
        .filter(|(_, profile)| profile.faction == faction)
        .map(|(place, profile)| (place, profile.clone()))
        .collect::<Vec<_>>();
    profiles.sort_by_key(|(place, _)| *place);
    match profiles.as_slice() {
        [] => Err(ActionError::PreconditionFailed(format!(
            "no BanditCampProfile exists for faction {faction}"
        ))),
        [(place, profile)] => Ok((*place, profile.clone())),
        _ => Err(ActionError::PreconditionFailed(format!(
            "multiple BanditCampProfile components exist for faction {faction}"
        ))),
    }
}

fn active_camps_for_faction(world: &World, faction: EntityId) -> Vec<(EntityId, BanditCamp)> {
    let mut camps = world
        .query_bandit_camp()
        .filter(|(_, camp)| camp.faction == faction)
        .map(|(place, camp)| (place, camp.clone()))
        .collect::<Vec<_>>();
    camps.sort_by_key(|(place, _)| *place);
    camps
}

fn carried_edible_lots(txn: &WorldTxn<'_>, actor: EntityId) -> Vec<EntityId> {
    let mut lots = carried_entities(txn, actor)
        .into_iter()
        .filter(|entity| {
            txn.get_component_item_lot(*entity).is_some_and(|lot| {
                lot.commodity
                    .spec()
                    .consumable_profile
                    .is_some_and(|profile| profile.hunger_relief_per_unit.value() > 0)
            })
        })
        .collect::<Vec<_>>();
    lots.sort();
    lots
}

fn living_faction_members_at_place(txn: &WorldTxn<'_>, faction: EntityId, place: EntityId) -> usize {
    txn.members_of(faction)
        .into_iter()
        .filter(|member| txn.is_alive(*member))
        .filter(|member| txn.effective_place(*member) == Some(place))
        .count()
}

fn validate_establish_camp(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    place: EntityId,
    faction: EntityId,
) -> Result<ValidatedEstablishCamp, ActionError> {
    if !txn.factions_of(actor).contains(&faction) {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} is not a member of faction {faction}"
        )));
    }
    if !txn.place_has_tag(place, PlaceTag::Camp) && !txn.place_has_tag(place, PlaceTag::Forest) {
        return Err(ActionError::PreconditionFailed(format!(
            "place {place} lacks Camp/Forest tag"
        )));
    }

    let existing_place_camp = txn.get_component_bandit_camp(place).cloned();
    if let Some(camp) = &existing_place_camp {
        if camp.faction != faction {
            return Err(ActionError::PreconditionFailed(format!(
                "place {place} already hosts bandit camp for faction {}",
                camp.faction
            )));
        }
    }
    if let Some(profile) = txn.get_component_bandit_camp_profile(place) {
        if profile.faction != faction {
            return Err(ActionError::PreconditionFailed(format!(
                "place {place} already holds bandit camp profile for faction {}",
                profile.faction
            )));
        }
    }

    let (profile_origin, profile) = canonical_bandit_camp_profile(txn, faction)?;
    let member_count = living_faction_members_at_place(txn, faction, place);
    if member_count < usize::from(profile.min_regroup_count) {
        return Err(ActionError::PreconditionFailed(format!(
            "faction {faction} has only {member_count} living members at place {place}, needs {}",
            profile.min_regroup_count
        )));
    }

    let active_camps = active_camps_for_faction(txn, faction);
    match active_camps.as_slice() {
        [] => {}
        [(camp_place, _)] if *camp_place == place => {}
        [(camp_place, _)] => {
            return Err(ActionError::PreconditionFailed(format!(
                "faction {faction} already has active camp at place {camp_place}"
            )));
        }
        _ => {
            return Err(ActionError::InternalError(format!(
                "multiple active BanditCamp components exist for faction {faction}"
            )));
        }
    }

    let carried_food_lots = carried_edible_lots(txn, actor);
    if carried_food_lots.is_empty() {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} carries no edible supplies"
        )));
    }

    Ok(ValidatedEstablishCamp {
        place,
        faction,
        profile_origin,
        profile,
        carried_food_lots,
        existing_place_camp,
    })
}

fn transferred_load(txn: &WorldTxn<'_>, lot_ids: &[EntityId]) -> Result<LoadUnits, ActionError> {
    let mut total = 0_u32;
    for lot_id in lot_ids {
        total = total
            .checked_add(
                load_of_entity(txn, *lot_id)
                    .map_err(|err| ActionError::InternalError(err.to_string()))?
                    .0,
            )
            .ok_or_else(|| ActionError::InternalError("camp transfer load overflowed".to_string()))?;
    }
    Ok(LoadUnits(total))
}

fn ensure_supply_container(
    txn: &mut WorldTxn<'_>,
    validated: &ValidatedEstablishCamp,
) -> Result<EntityId, ActionError> {
    let transfer_load = transferred_load(txn, &validated.carried_food_lots)?;
    if let Some(camp) = &validated.existing_place_camp {
        let container = camp.supplies;
        let current = txn
            .get_component_container(container)
            .cloned()
            .ok_or_else(|| {
                ActionError::InternalError(format!(
                    "camp supply entity {container} lacks Container component"
                ))
            })?;
        let required_capacity =
            current_container_load(txn, container, txn.direct_contents_of(container))
                .map_err(|err| ActionError::InternalError(err.to_string()))?
                .0
                .checked_add(transfer_load.0)
                .ok_or_else(|| {
                    ActionError::InternalError("camp container capacity overflowed".to_string())
                })?;
        if current.capacity.0 < required_capacity {
            return Err(ActionError::PreconditionFailed(format!(
                "camp supply container {container} lacks capacity for added supplies"
            )));
        }
        txn.set_owner(container, validated.faction)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
        Ok(container)
    } else {
        let container = txn
            .create_container(Container {
                capacity: transfer_load,
                allowed_commodities: None,
                allows_unique_items: false,
                allows_nested_containers: false,
            })
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
        txn.set_ground_location(container, validated.place)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
        txn.set_owner(container, validated.faction)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
        Ok(container)
    }
}

fn transfer_carried_food_to_camp(
    txn: &mut WorldTxn<'_>,
    faction: EntityId,
    place: EntityId,
    container: EntityId,
    lot_ids: &[EntityId],
) -> Result<(), ActionError> {
    for lot_id in lot_ids {
        let quantity = txn
            .get_component_item_lot(*lot_id)
            .map(|lot| lot.quantity)
            .ok_or_else(|| ActionError::InvalidTarget(*lot_id))?;
        if txn.direct_container(*lot_id).is_some() {
            txn.remove_from_container(*lot_id)
                .map_err(|err| ActionError::InternalError(err.to_string()))?;
        }
        if txn.possessor_of(*lot_id).is_some() {
            txn.clear_possessor(*lot_id)
                .map_err(|err| ActionError::InternalError(err.to_string()))?;
        }
        if txn.effective_place(*lot_id) != Some(place) {
            txn.set_ground_location(*lot_id, place)
                .map_err(|err| ActionError::InternalError(err.to_string()))?;
        }
        txn.set_owner(*lot_id, faction)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
        txn.put_into_container(*lot_id, container)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
        txn.append_transfer_provenance(*lot_id, quantity)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
        txn.add_target(*lot_id);
    }
    Ok(())
}

fn rehome_camp_profile(
    txn: &mut WorldTxn<'_>,
    validated: &ValidatedEstablishCamp,
) -> Result<(), ActionError> {
    if validated.profile_origin != validated.place {
        txn.clear_component_bandit_camp_profile(validated.profile_origin)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    if txn.get_component_bandit_camp_profile(validated.place) != Some(&validated.profile) {
        txn.set_component_bandit_camp_profile(validated.place, validated.profile.clone())
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    Ok(())
}

fn start_establish_camp(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<worldwake_sim::ActionState>, ActionError> {
    let place = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    let faction = payload_faction(&instance.payload)?;
    let _ = validate_establish_camp(txn, instance.actor, place, faction)?;
    Ok(None)
}

#[allow(clippy::unnecessary_wraps)]
fn tick_establish_camp(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_establish_camp(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let place = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    let faction = payload_faction(&instance.payload)?;
    let validated = validate_establish_camp(txn, instance.actor, place, faction)?;
    let container = ensure_supply_container(txn, &validated)?;

    if validated.existing_place_camp.is_none() {
        txn.set_component_bandit_camp(
            validated.place,
            BanditCamp {
                faction: validated.faction,
                supplies: container,
            },
        )
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }

    rehome_camp_profile(txn, &validated)?;
    transfer_carried_food_to_camp(
        txn,
        validated.faction,
        validated.place,
        container,
        &validated.carried_food_lots,
    )?;
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_establish_camp(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _reason: &AbortReason,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

fn enumerate_establish_camp_payloads(
    _def: &ActionDef,
    actor: EntityId,
    _targets: &[EntityId],
    view: &dyn RuntimeBeliefView,
) -> Vec<ActionPayload> {
    view.factions_of(actor)
        .into_iter()
        .map(|faction| ActionPayload::EstablishCamp(EstablishCampActionPayload { faction }))
        .collect()
}

fn validate_establish_camp_payload_override(
    _def: &ActionDef,
    actor: EntityId,
    _targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn RuntimeBeliefView,
) -> bool {
    payload
        .as_establish_camp()
        .is_some_and(|payload| view.factions_of(actor).contains(&payload.faction))
}

fn validate_establish_camp_payload_authoritatively(
    _def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    _targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let faction = payload_faction(payload)?;
    if !world.factions_of(actor).contains(&faction) {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} is not a member of faction {faction}"
        )));
    }
    let _ = canonical_bandit_camp_profile(world, faction)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::register_establish_camp_action;
    use std::collections::BTreeMap;
    use worldwake_core::{
        build_prototype_world, prototype_place_entity, verify_live_lot_conservation, BanditCamp,
        BanditCampProfile, CauseRef, CommodityKind, Container, ControlSource, EntityId, EventLog,
        LoadUnits, PrototypePlace, Quantity, Seed, Tick, VisibilitySpec, World, WorldTxn,
    };
    use worldwake_sim::{
        abort_action, get_affordances, start_action, tick_action, ActionExecutionAuthority,
        ActionDefRegistry, ActionError, ActionExecutionContext, ActionHandlerRegistry,
        ActionInstanceId, ActionPayload, DeterministicRng, EstablishCampActionPayload,
        PerAgentBeliefView, TickOutcome,
    };

    struct Harness {
        world: World,
        log: EventLog,
        defs: ActionDefRegistry,
        handlers: ActionHandlerRegistry,
        active: BTreeMap<ActionInstanceId, worldwake_sim::ActionInstance>,
        rng: DeterministicRng,
        next_id: ActionInstanceId,
        actor: EntityId,
        faction: EntityId,
        rival_faction: EntityId,
        rally_place: EntityId,
        anchor_place: EntityId,
        bread: EntityId,
    }

    impl Harness {
        fn new() -> Self {
            let mut world = World::new(build_prototype_world()).unwrap();
            let rally_place = prototype_place_entity(PrototypePlace::ForestPath);
            let anchor_place = prototype_place_entity(PrototypePlace::BanditCamp);
            let (actor, faction, rival_faction, bread) = {
                let mut txn = new_txn(&mut world, 1);
                let faction = txn.create_faction("Forest Bandits").unwrap();
                let rival_faction = txn.create_faction("Road Wolves").unwrap();
                let actor = txn.create_agent("Rook", ControlSource::Ai).unwrap();
                let wingman_a = txn.create_agent("Mora", ControlSource::Ai).unwrap();
                let wingman_b = txn.create_agent("Tarn", ControlSource::Ai).unwrap();
                for member in [actor, wingman_a, wingman_b] {
                    txn.add_member(member, faction).unwrap();
                    txn.set_ground_location(member, rally_place).unwrap();
                }
                let bread = txn.create_item_lot(CommodityKind::Bread, Quantity(2)).unwrap();
                txn.set_ground_location(bread, rally_place).unwrap();
                txn.set_owner(bread, actor).unwrap();
                txn.set_possessor(bread, actor).unwrap();
                txn.set_component_bandit_camp_profile(
                    anchor_place,
                    BanditCampProfile {
                        faction,
                        min_regroup_count: 3,
                        establishment_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                        flee_wound_threshold: worldwake_core::Permille::new(650).unwrap(),
                        rally_place: Some(rally_place),
                    },
                )
                .unwrap();
                txn.commit(&mut EventLog::new());
                (actor, faction, rival_faction, bread)
            };

            let mut defs = ActionDefRegistry::new();
            let mut handlers = ActionHandlerRegistry::new();
            register_establish_camp_action(&mut defs, &mut handlers);

            Self {
                world,
                log: EventLog::new(),
                defs,
                handlers,
                active: BTreeMap::new(),
                rng: DeterministicRng::new(Seed([7; 32])),
                next_id: ActionInstanceId(1),
                actor,
                faction,
                rival_faction,
                rally_place,
                anchor_place,
                bread,
            }
        }

        fn establish_affordance(&self) -> worldwake_sim::Affordance {
            let view = PerAgentBeliefView::from_world(self.actor, &self.world);
            get_affordances(&view, self.actor, &self.defs, &self.handlers)
                .into_iter()
                .find(|affordance| {
                    self.defs.get(affordance.def_id).unwrap().name == "establish_camp"
                        && affordance.payload_override
                            == Some(ActionPayload::EstablishCamp(EstablishCampActionPayload {
                                faction: self.faction,
                            }))
                })
                .unwrap()
        }

        fn start(&mut self) -> Result<ActionInstanceId, ActionError> {
            let affordance = self.establish_affordance();
            start_action(
                &affordance,
                &self.defs,
                &self.handlers,
                ActionExecutionAuthority {
                    active_actions: &mut self.active,
                    world: &mut self.world,
                    event_log: &mut self.log,
                    rng: &mut self.rng,
                },
                &mut self.next_id,
                ActionExecutionContext {
                    cause: CauseRef::Bootstrap,
                    tick: Tick(10),
                },
            )
        }

        fn tick(&mut self, instance_id: ActionInstanceId) -> TickOutcome {
            tick_action(
                instance_id,
                &self.defs,
                &self.handlers,
                ActionExecutionAuthority {
                    active_actions: &mut self.active,
                    world: &mut self.world,
                    event_log: &mut self.log,
                    rng: &mut self.rng,
                },
                ActionExecutionContext {
                    cause: CauseRef::Bootstrap,
                    tick: Tick(11),
                },
            )
            .unwrap()
        }
    }

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            worldwake_core::WitnessData::default(),
        )
    }

    #[test]
    fn establish_camp_start_and_commit_create_camp_and_transfer_supplies() {
        let mut harness = Harness::new();

        let action_id = harness.start().unwrap();
        let outcome = harness.tick(action_id);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));

        let camp = harness.world.get_component_bandit_camp(harness.rally_place).unwrap();
        assert_eq!(camp.faction, harness.faction);
        assert_eq!(
            harness.world.effective_place(camp.supplies),
            Some(harness.rally_place)
        );
        assert_eq!(harness.world.possessor_of(harness.bread), None);
        assert_eq!(harness.world.direct_container(harness.bread), Some(camp.supplies));
        assert_eq!(harness.world.owner_of(harness.bread), Some(harness.faction));
        assert_eq!(
            harness
                .world
                .get_component_bandit_camp_profile(harness.rally_place)
                .unwrap()
                .faction,
            harness.faction
        );
        assert_eq!(
            harness
                .world
                .get_component_bandit_camp_profile(harness.anchor_place),
            None
        );
        verify_live_lot_conservation(&harness.world, CommodityKind::Bread, 2).unwrap();
    }

    #[test]
    fn establish_camp_reuses_same_place_same_faction_camp() {
        let mut harness = Harness::new();
        let existing_container = {
            let mut txn = new_txn(&mut harness.world, 2);
            let container = txn
                .create_container(Container {
                    capacity: LoadUnits(10),
                    allowed_commodities: None,
                    allows_unique_items: false,
                    allows_nested_containers: false,
                })
                .unwrap();
            txn.set_ground_location(container, harness.rally_place).unwrap();
            txn.set_owner(container, harness.faction).unwrap();
            txn.set_component_bandit_camp(
                harness.rally_place,
                BanditCamp {
                    faction: harness.faction,
                    supplies: container,
                },
            )
            .unwrap();
            txn.commit(&mut harness.log);
            container
        };

        let action_id = harness.start().unwrap();
        let outcome = harness.tick(action_id);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));

        let camp = harness.world.get_component_bandit_camp(harness.rally_place).unwrap();
        assert_eq!(camp.supplies, existing_container);
        assert_eq!(harness.world.direct_container(harness.bread), Some(existing_container));
    }

    #[test]
    fn establish_camp_rejects_cross_faction_camp_reuse() {
        let mut harness = Harness::new();
        {
            let mut txn = new_txn(&mut harness.world, 2);
            let container = txn
                .create_container(Container {
                    capacity: LoadUnits(1),
                    allowed_commodities: None,
                    allows_unique_items: false,
                    allows_nested_containers: false,
                })
                .unwrap();
            txn.set_ground_location(container, harness.rally_place).unwrap();
            txn.set_owner(container, harness.rival_faction).unwrap();
            txn.set_component_bandit_camp(
                harness.rally_place,
                BanditCamp {
                    faction: harness.rival_faction,
                    supplies: container,
                },
            )
            .unwrap();
            txn.commit(&mut harness.log);
        }

        let err = harness.start().unwrap_err();
        assert_eq!(
            err,
            ActionError::PreconditionFailed(format!(
                "place {} already hosts bandit camp for faction {}",
                harness.rally_place, harness.rival_faction
            ))
        );
    }

    #[test]
    fn establish_camp_rejects_second_active_camp_for_same_faction() {
        let mut harness = Harness::new();
        {
            let mut txn = new_txn(&mut harness.world, 2);
            let container = txn
                .create_container(Container {
                    capacity: LoadUnits(1),
                    allowed_commodities: None,
                    allows_unique_items: false,
                    allows_nested_containers: false,
                })
                .unwrap();
            txn.set_ground_location(container, harness.anchor_place).unwrap();
            txn.set_owner(container, harness.faction).unwrap();
            txn.set_component_bandit_camp(
                harness.anchor_place,
                BanditCamp {
                    faction: harness.faction,
                    supplies: container,
                },
            )
            .unwrap();
            txn.commit(&mut harness.log);
        }

        let err = harness.start().unwrap_err();
        assert_eq!(
            err,
            ActionError::PreconditionFailed(format!(
                "faction {} already has active camp at place {}",
                harness.faction, harness.anchor_place
            ))
        );
    }

    #[test]
    fn establish_camp_abort_preserves_actor_supplies_and_creates_no_camp() {
        let mut harness = Harness::new();
        let action_id = harness.start().unwrap();

        abort_action(
            action_id,
            &harness.defs,
            &harness.handlers,
            ActionExecutionAuthority {
                active_actions: &mut harness.active,
                world: &mut harness.world,
                event_log: &mut harness.log,
                rng: &mut harness.rng,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
            worldwake_sim::ExternalAbortReason::CancelledByInput { sequence_no: 1 },
        )
        .unwrap();

        assert_eq!(harness.world.get_component_bandit_camp(harness.rally_place), None);
        assert_eq!(harness.world.possessor_of(harness.bread), Some(harness.actor));
        assert_eq!(harness.world.direct_container(harness.bread), None);
        verify_live_lot_conservation(&harness.world, CommodityKind::Bread, 2).unwrap();
    }
}
