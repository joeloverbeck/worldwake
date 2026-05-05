use std::collections::BTreeSet;
use worldwake_core::{
    ActionDefId, ActionDomain, BanditCamp, BanditFactionPolicy, BodyCostPerTick, Container,
    EntityId, EntityKind, EventTag, ExpectationId, LoadUnits, PlaceTag, Quantity, VisibilitySpec,
    World, WorldTxn, WoundCause, current_container_load, load_of_entity,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerId,
    ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress, CommitOutcome,
    Constraint, DeterministicRng, DurationExpr, EffectEvaluationContext, EffectMode,
    EffectPrecondition, EffectSchema, EffectSink, EffectStep, EstablishCampActionPayload,
    Interruptibility, Precondition, RuntimeBeliefView, TargetSpec, apply_effects_with_context,
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
        attention_cost: worldwake_core::Permille::new_unchecked(200),
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
        binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
        guard_template: None,
        expectation_template: vec![],
        effect_schema: establish_camp_effect_schema(),
    }
}

fn establish_camp_effect_schema() -> EffectSchema {
    EffectSchema {
        preconditions: Vec::new(),
        steps: vec![EffectStep::EstablishBanditCamp],
    }
}

#[derive(Clone)]
struct ValidatedEstablishCamp {
    place: EntityId,
    faction: EntityId,
    supply_lots: Vec<EntityId>,
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

fn faction_policy(world: &World, faction: EntityId) -> Result<BanditFactionPolicy, ActionError> {
    world
        .get_component_bandit_faction_policy(faction)
        .cloned()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!("faction {faction} lacks BanditFactionPolicy"))
        })
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

fn controlled_edible_lots_at_place(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    place: EntityId,
) -> Vec<EntityId> {
    let mut lots = txn
        .entities_effectively_at(place)
        .into_iter()
        .filter(|entity| {
            txn.can_exercise_control(actor, *entity).is_ok()
                && txn.effective_place(*entity) == Some(place)
        })
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

fn living_faction_members_at_place(
    txn: &WorldTxn<'_>,
    faction: EntityId,
    place: EntityId,
) -> usize {
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
    if let Some(camp) = &existing_place_camp
        && camp.faction != faction
    {
        return Err(ActionError::PreconditionFailed(format!(
            "place {place} already hosts bandit camp for faction {}",
            camp.faction
        )));
    }
    let policy = faction_policy(txn, faction)?;
    let member_count = living_faction_members_at_place(txn, faction, place);
    if member_count < usize::from(policy.min_regroup_count) {
        return Err(ActionError::PreconditionFailed(format!(
            "faction {faction} has only {member_count} living members at place {place}, needs {}",
            policy.min_regroup_count
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

    let supply_lots = controlled_edible_lots_at_place(txn, actor, place);
    if supply_lots.is_empty() {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} controls no edible supplies at place {place}"
        )));
    }

    Ok(ValidatedEstablishCamp {
        place,
        faction,
        supply_lots,
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
            .ok_or_else(|| {
                ActionError::InternalError("camp transfer load overflowed".to_string())
            })?;
    }
    Ok(LoadUnits(total))
}

fn ensure_supply_container(
    txn: &mut WorldTxn<'_>,
    validated: &ValidatedEstablishCamp,
) -> Result<EntityId, ActionError> {
    let transfer_load = transferred_load(txn, &validated.supply_lots)?;
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
            let replacement = txn
                .create_container(Container {
                    capacity: LoadUnits(required_capacity),
                    allowed_commodities: current.allowed_commodities,
                    allows_unique_items: current.allows_unique_items,
                    allows_nested_containers: current.allows_nested_containers,
                })
                .map_err(|err| ActionError::InternalError(err.to_string()))?;
            txn.set_ground_location(replacement, validated.place)
                .map_err(|err| ActionError::InternalError(err.to_string()))?;
            txn.set_owner(replacement, validated.faction)
                .map_err(|err| ActionError::InternalError(err.to_string()))?;
            for item in txn.direct_contents_of(container) {
                txn.remove_from_container(item)
                    .map_err(|err| ActionError::InternalError(err.to_string()))?;
                txn.put_into_container(item, replacement)
                    .map_err(|err| ActionError::InternalError(err.to_string()))?;
            }
            txn.set_component_bandit_camp(
                validated.place,
                BanditCamp {
                    faction: validated.faction,
                    supplies: replacement,
                    empty_since_tick: camp.empty_since_tick,
                },
            )
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
            txn.archive_entity(container)
                .map_err(|err| ActionError::InternalError(err.to_string()))?;
            return Ok(replacement);
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

fn transfer_supply_lots_to_camp(
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

fn start_establish_camp(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
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
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn apply_establish_camp(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    place: EntityId,
    payload: &ActionPayload,
) -> Result<CommitOutcome, ActionError> {
    let faction = payload_faction(payload)?;
    let validated = validate_establish_camp(txn, actor, place, faction)?;
    let container = ensure_supply_container(txn, &validated)?;

    if validated.existing_place_camp.is_none() {
        txn.set_component_bandit_camp(
            validated.place,
            BanditCamp {
                faction: validated.faction,
                supplies: container,
                empty_since_tick: None,
            },
        )
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    transfer_supply_lots_to_camp(
        txn,
        validated.faction,
        validated.place,
        container,
        &validated.supply_lots,
    )?;
    Ok(CommitOutcome::empty())
}

struct EstablishCampEffectSink<'txn, 'world> {
    txn: &'txn mut WorldTxn<'world>,
    action_error: Option<ActionError>,
}

impl EstablishCampEffectSink<'_, '_> {
    fn record_error(&mut self, error: ActionError) -> worldwake_core::Discrepancy {
        self.action_error = Some(error);
        worldwake_core::Discrepancy::PartialExecutionDrift
    }

    fn take_error(self, discrepancy: worldwake_core::Discrepancy) -> ActionError {
        self.action_error.unwrap_or_else(|| {
            ActionError::PreconditionFailed(format!("effect schema failed: {discrepancy:?}"))
        })
    }
}

impl EffectSink for EstablishCampEffectSink<'_, '_> {
    fn check_precondition(
        &self,
        _precondition: &EffectPrecondition,
        _actor: EntityId,
        _targets: &[EntityId],
    ) -> Result<(), worldwake_core::Discrepancy> {
        Ok(())
    }

    fn checkpoint(&mut self) -> usize {
        0
    }

    fn restore(&mut self, _checkpoint: usize) -> Result<(), worldwake_core::Discrepancy> {
        Err(worldwake_core::Discrepancy::ImproperPlanningState)
    }

    fn write_transfer(
        &mut self,
        _source: EntityId,
        _dest: EntityId,
        _commodity: worldwake_core::CommodityKind,
        _quantity: Quantity,
    ) -> Result<(), worldwake_core::Discrepancy> {
        Err(worldwake_core::Discrepancy::ImproperPlanningState)
    }

    fn write_consume(
        &mut self,
        _source: EntityId,
        _commodity: worldwake_core::CommodityKind,
        _quantity: Quantity,
    ) -> Result<(), worldwake_core::Discrepancy> {
        Err(worldwake_core::Discrepancy::ImproperPlanningState)
    }

    fn write_produce(
        &mut self,
        _sink: EntityId,
        _commodity: worldwake_core::CommodityKind,
        _quantity: Quantity,
    ) -> Result<(), worldwake_core::Discrepancy> {
        Err(worldwake_core::Discrepancy::ImproperPlanningState)
    }

    fn write_wound(
        &mut self,
        _target: EntityId,
        _cause: WoundCause,
    ) -> Result<(), worldwake_core::Discrepancy> {
        Err(worldwake_core::Discrepancy::ImproperPlanningState)
    }

    fn write_event(&mut self, _tag: EventTag) -> Result<(), worldwake_core::Discrepancy> {
        Err(worldwake_core::Discrepancy::ImproperPlanningState)
    }

    fn assert_expectation_fulfilled(
        &mut self,
        _expectation: ExpectationId,
    ) -> Result<(), worldwake_core::Discrepancy> {
        Err(worldwake_core::Discrepancy::ImproperPlanningState)
    }

    fn consume_grant(&mut self, _grant: EntityId) -> Result<(), worldwake_core::Discrepancy> {
        Err(worldwake_core::Discrepancy::ImproperPlanningState)
    }

    fn establish_bandit_camp(
        &mut self,
        actor: EntityId,
        target: EntityId,
        payload: &ActionPayload,
    ) -> Result<(), worldwake_core::Discrepancy> {
        apply_establish_camp(self.txn, actor, target, payload)
            .map(|_| ())
            .map_err(|err| self.record_error(err))
    }
}

fn apply_establish_camp_effect_schema(
    def: &ActionDef,
    instance: &ActionInstance,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let mut sink = EstablishCampEffectSink {
        txn,
        action_error: None,
    };
    match apply_effects_with_context(
        &def.effect_schema,
        EffectEvaluationContext {
            actor: instance.actor,
            targets: &instance.targets,
            payload: &instance.payload,
            action_def_id: def.id,
        },
        &mut sink,
        EffectMode::Authoritative,
    ) {
        Ok(_) => Ok(CommitOutcome::empty()),
        Err(discrepancy) => Err(sink.take_error(discrepancy)),
    }
}

fn commit_establish_camp(
    def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    apply_establish_camp_effect_schema(def, instance, txn)
}

#[allow(clippy::unnecessary_wraps)]
fn abort_establish_camp(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _reason: &AbortReason,
    _event_log: &worldwake_core::EventLog,
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
    let _ = faction_policy(world, faction)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::register_establish_camp_action;
    use std::collections::BTreeMap;
    use worldwake_core::{
        BanditCamp, BanditFactionPolicy, CauseRef, CommodityKind, Container, ControlSource,
        EntityId, EventLog, LoadUnits, PrototypePlace, Quantity, Seed, Tick, VisibilitySpec, World,
        WorldTxn, build_prototype_world, prototype_place_entity, verify_live_lot_conservation,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionError, ActionExecutionAuthority, ActionHandlerRegistry,
        ActionInstanceId, ActionPayload, DeterministicRng, DurationExpr, EffectStep,
        EstablishCampActionPayload, PerAgentBeliefView, TargetSpec, TickOutcome, abort_action,
        get_affordances, start_action, tick_action,
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
                let bread = txn
                    .create_item_lot(CommodityKind::Bread, Quantity(2))
                    .unwrap();
                txn.set_ground_location(bread, rally_place).unwrap();
                txn.set_owner(bread, actor).unwrap();
                txn.set_possessor(bread, actor).unwrap();
                txn.set_component_bandit_faction_policy(
                    faction,
                    BanditFactionPolicy {
                        min_regroup_count: 3,
                        establishment_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                        abandonment_grace_ticks: std::num::NonZeroU32::new(2).unwrap(),
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
                worldwake_sim::ActionExecutionContext::without_recipes(
                    CauseRef::Bootstrap,
                    Tick(10),
                ),
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
                worldwake_sim::ActionExecutionContext::without_recipes(
                    CauseRef::Bootstrap,
                    Tick(11),
                ),
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
    fn register_establish_camp_action_creates_schema_driven_definition() {
        let harness = Harness::new();
        let def = harness
            .defs
            .iter()
            .find(|def| def.name == "establish_camp")
            .unwrap();

        assert_eq!(def.targets, vec![TargetSpec::ActorPlace]);
        assert_eq!(def.duration, DurationExpr::BanditCampEstablishmentProfile);
        assert_eq!(
            def.effect_schema.steps,
            vec![EffectStep::EstablishBanditCamp]
        );
    }

    #[test]
    fn establish_camp_start_and_commit_create_camp_and_transfer_supplies() {
        let mut harness = Harness::new();

        let action_id = harness.start().unwrap();
        let outcome = harness.tick(action_id);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));

        let camp = harness
            .world
            .get_component_bandit_camp(harness.rally_place)
            .unwrap();
        assert_eq!(camp.faction, harness.faction);
        assert_eq!(
            harness.world.effective_place(camp.supplies),
            Some(harness.rally_place)
        );
        assert_eq!(harness.world.possessor_of(harness.bread), None);
        assert_eq!(
            harness.world.direct_container(harness.bread),
            Some(camp.supplies)
        );
        assert_eq!(harness.world.owner_of(harness.bread), Some(harness.faction));
        assert_eq!(
            harness
                .world
                .get_component_bandit_faction_policy(harness.faction),
            Some(&BanditFactionPolicy {
                min_regroup_count: 3,
                establishment_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                abandonment_grace_ticks: std::num::NonZeroU32::new(2).unwrap(),
                flee_wound_threshold: worldwake_core::Permille::new(650).unwrap(),
                rally_place: Some(harness.rally_place),
            })
        );
        verify_live_lot_conservation(&harness.world, CommodityKind::Bread, 2).unwrap();
    }

    #[test]
    fn establish_camp_accepts_ground_supplies_under_local_control() {
        let mut harness = Harness::new();
        {
            let mut txn = new_txn(&mut harness.world, 2);
            txn.clear_possessor(harness.bread).unwrap();
            txn.commit(&mut harness.log);
        }

        let action_id = harness.start().unwrap();
        let outcome = harness.tick(action_id);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));

        let camp = harness
            .world
            .get_component_bandit_camp(harness.rally_place)
            .unwrap();
        assert_eq!(harness.world.possessor_of(harness.bread), None);
        assert_eq!(
            harness.world.direct_container(harness.bread),
            Some(camp.supplies)
        );
        assert_eq!(harness.world.owner_of(harness.bread), Some(harness.faction));
    }

    #[test]
    fn establish_camp_accepts_faction_owned_ground_supplies() {
        let mut harness = Harness::new();
        {
            let mut txn = new_txn(&mut harness.world, 2);
            txn.clear_possessor(harness.bread).unwrap();
            txn.set_owner(harness.bread, harness.faction).unwrap();
            txn.commit(&mut harness.log);
        }

        let action_id = harness.start().unwrap();
        let outcome = harness.tick(action_id);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));

        let camp = harness
            .world
            .get_component_bandit_camp(harness.rally_place)
            .unwrap();
        assert_eq!(harness.world.possessor_of(harness.bread), None);
        assert_eq!(
            harness.world.direct_container(harness.bread),
            Some(camp.supplies)
        );
        assert_eq!(harness.world.owner_of(harness.bread), Some(harness.faction));
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
            txn.set_ground_location(container, harness.rally_place)
                .unwrap();
            txn.set_owner(container, harness.faction).unwrap();
            txn.set_component_bandit_camp(
                harness.rally_place,
                BanditCamp {
                    faction: harness.faction,
                    supplies: container,
                    empty_since_tick: None,
                },
            )
            .unwrap();
            txn.commit(&mut harness.log);
            container
        };

        let action_id = harness.start().unwrap();
        let outcome = harness.tick(action_id);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));

        let camp = harness
            .world
            .get_component_bandit_camp(harness.rally_place)
            .unwrap();
        assert_eq!(camp.supplies, existing_container);
        assert_eq!(
            harness.world.direct_container(harness.bread),
            Some(existing_container)
        );
    }

    #[test]
    fn establish_camp_reuse_expands_same_place_camp_capacity_for_new_supplies() {
        let mut harness = Harness::new();
        let existing_container = {
            let mut txn = new_txn(&mut harness.world, 2);
            let container = txn
                .create_container(Container {
                    capacity: LoadUnits(1),
                    allowed_commodities: None,
                    allows_unique_items: false,
                    allows_nested_containers: false,
                })
                .unwrap();
            txn.set_ground_location(container, harness.rally_place)
                .unwrap();
            txn.set_owner(container, harness.faction).unwrap();
            txn.set_component_bandit_camp(
                harness.rally_place,
                BanditCamp {
                    faction: harness.faction,
                    supplies: container,
                    empty_since_tick: None,
                },
            )
            .unwrap();
            txn.commit(&mut harness.log);
            container
        };

        let action_id = harness.start().unwrap();
        let outcome = harness.tick(action_id);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));

        let camp = harness
            .world
            .get_component_bandit_camp(harness.rally_place)
            .unwrap();
        assert_ne!(camp.supplies, existing_container);
        assert!(
            harness
                .world
                .get_component_container(existing_container)
                .is_none(),
            "resized reuse should archive the undersized container"
        );
        assert_eq!(
            harness
                .world
                .get_component_container(camp.supplies)
                .expect("replacement camp should keep a supply container")
                .capacity,
            LoadUnits(2)
        );
        assert_eq!(
            harness.world.direct_container(harness.bread),
            Some(camp.supplies)
        );
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
            txn.set_ground_location(container, harness.rally_place)
                .unwrap();
            txn.set_owner(container, harness.rival_faction).unwrap();
            txn.set_component_bandit_camp(
                harness.rally_place,
                BanditCamp {
                    faction: harness.rival_faction,
                    supplies: container,
                    empty_since_tick: None,
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
            txn.set_ground_location(container, harness.anchor_place)
                .unwrap();
            txn.set_owner(container, harness.faction).unwrap();
            txn.set_component_bandit_camp(
                harness.anchor_place,
                BanditCamp {
                    faction: harness.faction,
                    supplies: container,
                    empty_since_tick: None,
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(11)),
            worldwake_sim::ExternalAbortReason::CancelledByInput { sequence_no: 1 },
        )
        .unwrap();

        assert_eq!(
            harness.world.get_component_bandit_camp(harness.rally_place),
            None
        );
        assert_eq!(
            harness.world.possessor_of(harness.bread),
            Some(harness.actor)
        );
        assert_eq!(harness.world.direct_container(harness.bread), None);
        verify_live_lot_conservation(&harness.world, CommodityKind::Bread, 2).unwrap();
    }
}
