use crate::facility_queue_actions::exclusive_facility_workstation_tag;
use std::collections::BTreeMap;
use std::num::NonZeroU32;
use worldwake_core::{
    ActionDomain, CauseRef, ContentionEventPayload, ContentionPolicy, ContentionQueue, EntityId,
    EntityKind, EventLog, EventTag, PlaceTag, ReliabilityRecord, SourceKey, Tick, VisibilitySpec,
    WitnessData, WorkstationTag, World, WorldTxn, build_contention_event_payload,
};
use worldwake_sim::{
    ActionDefRegistry, ActionInstance, ActionStatus, SystemError, SystemExecutionContext,
    validate_action_def_authoritatively,
};

struct QueueUpdateEffects<'a> {
    extra_tag: Option<EventTag>,
    extra_target: Option<EntityId>,
    extra_contention_event: Option<ContentionEventPayload>,
    cleared_waiters: &'a [worldwake_core::ContentionWaiter],
    wait_observation: Option<WaitObservation>,
}

#[derive(Clone, Copy)]
struct WaitObservation {
    actor: EntityId,
    wait_ticks: u32,
}

#[derive(Clone, Copy)]
enum PromotableContentionKind {
    FacilityExclusive(WorkstationTag),
    Corpse,
    Care,
    SelfCareWash,
    SelfCareLatrine,
}

pub fn contention_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let SystemExecutionContext {
        world,
        event_log,
        rng: _rng,
        active_actions,
        action_defs,
        politics_trace: _,
        perception_trace: _,
        tick,
        system_id: _system_id,
    } = ctx;

    sync_ground_unique_item_contention(world, event_log, tick)?;

    let contended_entities = world.all_entities().collect::<Vec<_>>();
    for facility in contended_entities {
        if world.get_component_contention_queue(facility).is_none() {
            continue;
        }

        prune_invalid_waiters(world, event_log, facility, tick)?;
        prune_patience_exceeded(world, event_log, facility, tick)?;
        expire_stale_grant(world, event_log, facility, tick)?;
        prune_structurally_invalid_heads(world, event_log, action_defs, facility, tick)?;
        promote_ready_head(
            world,
            event_log,
            active_actions,
            action_defs,
            facility,
            tick,
        )?;
    }

    Ok(())
}

fn unique_item_pickup_contention_policy() -> ContentionPolicy {
    ContentionPolicy {
        grant_hold_ticks: NonZeroU32::new(3).unwrap(),
        auto_promote: false,
        max_waiters: Some(0),
    }
}

fn unique_item_pickup_contention_eligible(world: &World, entity: EntityId) -> bool {
    world.entity_kind(entity) == Some(EntityKind::UniqueItem)
        && world.effective_place(entity).is_some()
        && world.direct_container(entity).is_none()
        && world.possessor_of(entity).is_none()
        && world.owner_of(entity).is_none()
}

fn sync_ground_unique_item_contention(
    world: &mut World,
    event_log: &mut EventLog,
    tick: Tick,
) -> Result<(), SystemError> {
    let unique_items = world
        .all_entities()
        .filter(|entity| world.entity_kind(*entity) == Some(EntityKind::UniqueItem))
        .collect::<Vec<_>>();

    for entity in unique_items {
        let eligible = unique_item_pickup_contention_eligible(world, entity);
        let has_policy = world.get_component_contention_policy(entity).is_some();
        let has_queue = world.get_component_contention_queue(entity).is_some();

        if eligible && !has_policy && !has_queue {
            let place = world.effective_place(entity);
            let mut txn = WorldTxn::new(
                world,
                tick,
                CauseRef::SystemTick(tick),
                None,
                place,
                VisibilitySpec::SamePlace,
                WitnessData::default(),
            );
            txn.add_tag(EventTag::System)
                .add_tag(EventTag::WorldMutation)
                .add_target(entity);
            txn.set_component_contention_policy(entity, unique_item_pickup_contention_policy())
                .map_err(|error| SystemError::new(error.to_string()))?;
            txn.set_component_contention_queue(entity, ContentionQueue::default())
                .map_err(|error| SystemError::new(error.to_string()))?;
            let _ = txn.commit(event_log);
            continue;
        }

        if !eligible && (has_policy || has_queue) {
            let place = world.effective_place(entity);
            let mut txn = WorldTxn::new(
                world,
                tick,
                CauseRef::SystemTick(tick),
                None,
                place,
                VisibilitySpec::SamePlace,
                WitnessData::default(),
            );
            txn.add_tag(EventTag::System)
                .add_tag(EventTag::WorldMutation)
                .add_target(entity);
            if has_queue {
                txn.clear_component_contention_queue(entity)
                    .map_err(|error| SystemError::new(error.to_string()))?;
            }
            if has_policy {
                txn.clear_component_contention_policy(entity)
                    .map_err(|error| SystemError::new(error.to_string()))?;
            }
            let _ = txn.commit(event_log);
        }
    }

    Ok(())
}

fn prune_invalid_waiters(
    world: &mut World,
    event_log: &mut EventLog,
    facility: EntityId,
    tick: Tick,
) -> Result<(), SystemError> {
    let facility_place = world.effective_place(facility);
    let Some(mut queue) = world.get_component_contention_queue(facility).cloned() else {
        return Ok(());
    };
    let removed_waiters = queue
        .waiting
        .values()
        .filter(|queued| {
            world.entity_kind(queued.actor).is_none()
                || world.get_component_dead_at(queued.actor).is_some()
                || world.effective_place(queued.actor) != facility_place
                || !actor_has_matching_contention_intent(
                    world,
                    queued.actor,
                    facility,
                    queued.intended_action,
                )
        })
        .cloned()
        .collect::<Vec<_>>();

    if !removed_waiters.is_empty() {
        queue.waiting.retain(|_, queued| {
            !removed_waiters.iter().any(|removed| {
                removed.actor == queued.actor
                    && removed.intended_action == queued.intended_action
                    && removed.queued_at == queued.queued_at
            })
        });
        commit_queue_update(
            world,
            event_log,
            facility,
            queue,
            tick,
            QueueUpdateEffects {
                extra_tag: None,
                extra_target: None,
                extra_contention_event: None,
                cleared_waiters: &removed_waiters,
                wait_observation: None,
            },
        )?;
    }

    Ok(())
}

fn prune_patience_exceeded(
    world: &mut World,
    event_log: &mut EventLog,
    facility: EntityId,
    tick: Tick,
) -> Result<(), SystemError> {
    let Some(mut queue) = world.get_component_contention_queue(facility).cloned() else {
        return Ok(());
    };
    let removed_waiters = queue
        .waiting
        .values()
        .filter(|queued| {
            world
                .get_component_contention_disposition_profile(queued.actor)
                .and_then(|profile| profile.queue_patience_ticks)
                .is_some_and(|limit| tick >= queued.queued_at + u64::from(limit.get()))
        })
        .cloned()
        .collect::<Vec<_>>();

    if !removed_waiters.is_empty() {
        queue.waiting.retain(|_, queued| {
            !removed_waiters.iter().any(|removed| {
                removed.actor == queued.actor
                    && removed.intended_action == queued.intended_action
                    && removed.queued_at == queued.queued_at
            })
        });
        commit_queue_update(
            world,
            event_log,
            facility,
            queue,
            tick,
            QueueUpdateEffects {
                extra_tag: None,
                extra_target: None,
                extra_contention_event: None,
                cleared_waiters: &removed_waiters,
                wait_observation: None,
            },
        )?;
    }

    Ok(())
}

fn expire_stale_grant(
    world: &mut World,
    event_log: &mut EventLog,
    facility: EntityId,
    tick: Tick,
) -> Result<(), SystemError> {
    let Some(mut queue) = world.get_component_contention_queue(facility).cloned() else {
        return Ok(());
    };
    let Some(granted) = queue.granted.clone() else {
        return Ok(());
    };
    if !queue.grant_expired(tick) {
        return Ok(());
    }

    queue.clear_grant();
    commit_queue_update(
        world,
        event_log,
        facility,
        queue,
        tick,
        QueueUpdateEffects {
            extra_tag: Some(EventTag::QueueGrantExpired),
            extra_target: Some(granted.actor),
            extra_contention_event: None,
            cleared_waiters: &[],
            wait_observation: None,
        },
    )
}

fn prune_structurally_invalid_heads(
    world: &mut World,
    event_log: &mut EventLog,
    action_defs: &ActionDefRegistry,
    facility: EntityId,
    tick: Tick,
) -> Result<(), SystemError> {
    loop {
        let Some(queue) = world.get_component_contention_queue(facility).cloned() else {
            return Ok(());
        };
        let Some((&ordinal, queued)) = queue.waiting.iter().next() else {
            return Ok(());
        };
        let queued = queued.clone();
        let queued_actor = queued.actor;
        let intended_action = queued.intended_action;

        if !head_is_structurally_invalid(world, action_defs, facility, intended_action) {
            return Ok(());
        }

        let mut next_queue = queue;
        next_queue.waiting.remove(&ordinal);
        commit_queue_update(
            world,
            event_log,
            facility,
            next_queue,
            tick,
            QueueUpdateEffects {
                extra_tag: Some(EventTag::QueueHeadFailed),
                extra_target: Some(queued_actor),
                extra_contention_event: None,
                cleared_waiters: std::slice::from_ref(&queued),
                wait_observation: None,
            },
        )?;
    }
}

fn promote_ready_head(
    world: &mut World,
    event_log: &mut EventLog,
    active_actions: &BTreeMap<worldwake_sim::ActionInstanceId, ActionInstance>,
    action_defs: &ActionDefRegistry,
    facility: EntityId,
    tick: Tick,
) -> Result<(), SystemError> {
    let Some(policy) = world.get_component_contention_policy(facility).cloned() else {
        return Ok(());
    };
    let Some(mut queue) = world.get_component_contention_queue(facility).cloned() else {
        return Ok(());
    };
    if !policy.auto_promote {
        return Ok(());
    }
    if queue.granted.is_some()
        || active_exclusive_action_on_entity(active_actions, action_defs, facility)
    {
        return Ok(());
    }
    let Some(queued) = queue.waiting.values().next() else {
        return Ok(());
    };
    let queued_actor = queued.actor;
    let queued_at = queued.queued_at;
    if !head_is_ready_to_start(
        world,
        active_actions,
        action_defs,
        facility,
        queued.actor,
        queued.intended_action,
    ) {
        return Ok(());
    }
    let wait_ticks = tick
        .0
        .saturating_sub(queued_at.0)
        .try_into()
        .unwrap_or(u32::MAX);
    let queue_snapshot = queue.clone();
    let facility_place = world
        .effective_place(facility)
        .ok_or_else(|| SystemError::new("contention facility has no effective place"))?;

    let granted = queue
        .promote_head(tick, policy.grant_hold_ticks)
        .cloned()
        .expect("queue head exists when promotion is attempted");
    let contention_event = build_contention_event_payload(
        &queue_snapshot,
        facility,
        facility_place,
        granted.intended_action,
        worldwake_core::ContentionResolutionRule::ArrivalTime,
        Some(granted.actor),
        tick,
    );
    commit_queue_update(
        world,
        event_log,
        facility,
        queue,
        tick,
        QueueUpdateEffects {
            extra_tag: Some(EventTag::QueueGrantPromoted),
            extra_target: Some(granted.actor),
            extra_contention_event: Some(contention_event),
            cleared_waiters: &[],
            wait_observation: Some(WaitObservation {
                actor: queued_actor,
                wait_ticks,
            }),
        },
    )
}

fn head_is_structurally_invalid(
    world: &World,
    action_defs: &ActionDefRegistry,
    entity: EntityId,
    intended_action: worldwake_core::ActionDefId,
) -> bool {
    let Some(def) = action_defs.get(intended_action) else {
        return true;
    };
    let Some(kind) = promotable_contention_kind(def) else {
        return true;
    };
    !contention_target_matches_kind(world, entity, kind)
}

fn head_is_ready_to_start(
    world: &World,
    active_actions: &BTreeMap<worldwake_sim::ActionInstanceId, ActionInstance>,
    action_defs: &ActionDefRegistry,
    entity: EntityId,
    actor: EntityId,
    intended_action: worldwake_core::ActionDefId,
) -> bool {
    let Some(def) = action_defs.get(intended_action) else {
        return false;
    };
    let Some(kind) = promotable_contention_kind(def) else {
        return false;
    };
    if !contention_target_matches_kind(world, entity, kind) {
        return false;
    }
    let targets = [entity];

    validate_action_def_authoritatively(world, def, actor, &targets).is_ok()
        && !active_exclusive_action_on_entity(active_actions, action_defs, entity)
}

fn active_exclusive_action_on_entity(
    active_actions: &BTreeMap<worldwake_sim::ActionInstanceId, ActionInstance>,
    action_defs: &ActionDefRegistry,
    entity: EntityId,
) -> bool {
    active_actions.values().any(|instance| {
        instance.status == ActionStatus::Active
            && instance.targets.first().copied() == Some(entity)
            && action_defs
                .get(instance.def_id)
                .and_then(promotable_contention_kind)
                .is_some()
    })
}

fn promotable_contention_kind(def: &worldwake_sim::ActionDef) -> Option<PromotableContentionKind> {
    if let Some(tag) = exclusive_facility_workstation_tag(def) {
        return Some(PromotableContentionKind::FacilityExclusive(tag));
    }

    match (def.domain, def.name.as_str()) {
        (ActionDomain::Corpse, "loot" | "bury") => Some(PromotableContentionKind::Corpse),
        (ActionDomain::Care, "heal") => Some(PromotableContentionKind::Care),
        (ActionDomain::Needs, "wash") => Some(PromotableContentionKind::SelfCareWash),
        (ActionDomain::Needs, "toilet") => Some(PromotableContentionKind::SelfCareLatrine),
        _ => None,
    }
}

fn contention_target_matches_kind(
    world: &World,
    entity: EntityId,
    kind: PromotableContentionKind,
) -> bool {
    match kind {
        PromotableContentionKind::FacilityExclusive(required_tag) => {
            if world.entity_kind(entity) != Some(EntityKind::Facility) || !world.is_alive(entity) {
                return false;
            }

            world
                .get_component_workstation_marker(entity)
                .is_some_and(|marker| marker.0 == required_tag)
        }
        PromotableContentionKind::Corpse => {
            world.entity_kind(entity) == Some(EntityKind::Agent)
                && world.get_component_dead_at(entity).is_some()
        }
        PromotableContentionKind::Care => {
            world.entity_kind(entity) == Some(EntityKind::Agent)
                && world.get_component_dead_at(entity).is_none()
                && world
                    .get_component_wound_list(entity)
                    .is_some_and(|wounds| !wounds.wounds.is_empty())
        }
        PromotableContentionKind::SelfCareWash => {
            world.entity_kind(entity) == Some(EntityKind::Facility)
                && world.is_alive(entity)
                && world
                    .get_component_workstation_marker(entity)
                    .is_some_and(|marker| marker.0 == WorkstationTag::WashBasin)
        }
        PromotableContentionKind::SelfCareLatrine => {
            world.entity_kind(entity) == Some(EntityKind::Place)
                && world.place_has_tag(entity, PlaceTag::Latrine)
        }
    }
}

fn commit_queue_update(
    world: &mut World,
    event_log: &mut EventLog,
    facility: EntityId,
    queue: ContentionQueue,
    tick: Tick,
    effects: QueueUpdateEffects<'_>,
) -> Result<(), SystemError> {
    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        None,
        world.effective_place(facility),
        VisibilitySpec::SamePlace,
        WitnessData::default(),
    );
    txn.add_tag(EventTag::System)
        .add_tag(EventTag::WorldMutation)
        .add_target(facility);
    if let Some(tag) = effects.extra_tag {
        txn.add_tag(tag);
    }
    if let Some(payload) = effects.extra_contention_event {
        txn.set_contention_event_payload(payload);
    }
    if let Some(target) = effects.extra_target {
        txn.add_target(target);
    }
    for cleared in effects.cleared_waiters {
        let Some(mut intents) = txn.get_component_contention_intents(cleared.actor).cloned() else {
            continue;
        };
        let remove_entry = intents
            .intents
            .get(&facility)
            .is_some_and(|intent| intent.intended_action == cleared.intended_action);
        if !remove_entry {
            continue;
        }
        intents.intents.remove(&facility);
        if intents.intents.is_empty() {
            txn.clear_component_contention_intents(cleared.actor)
                .map_err(|error| SystemError::new(error.to_string()))?;
        } else {
            txn.set_component_contention_intents(cleared.actor, intents)
                .map_err(|error| SystemError::new(error.to_string()))?;
        }
    }
    txn.set_component_contention_queue(facility, queue)
        .map_err(|error| SystemError::new(error.to_string()))?;
    if let Some(observation) = effects.wait_observation
        && observation.wait_ticks > 0
        && let Some(source) = txn.get_component_resource_source(facility)
    {
        let key = SourceKey {
            entity: facility,
            commodity: source.commodity,
        };
        let mut reliability = txn
            .get_component_source_reliability(observation.actor)
            .cloned()
            .unwrap_or_default();
        reliability
            .sources
            .entry(key)
            .or_insert_with(|| ReliabilityRecord::new(tick))
            .observe_wait(observation.wait_ticks);
        if let Some(record) = reliability.sources.get_mut(&key) {
            record.push_provenance(event_log.next_id());
        }
        txn.set_component_source_reliability(observation.actor, reliability)
            .map_err(|error| SystemError::new(error.to_string()))?;
    }
    let _ = txn.commit(event_log);
    Ok(())
}

fn actor_has_matching_contention_intent(
    world: &World,
    actor: EntityId,
    facility: EntityId,
    intended_action: worldwake_core::ActionDefId,
) -> bool {
    let Some(intents) = world.get_component_contention_intents(actor) else {
        return true;
    };

    intents
        .intents
        .get(&facility)
        .is_some_and(|intent| intent.intended_action == intended_action)
}

#[cfg(test)]
mod tests {
    use super::{
        PromotableContentionKind, contention_system, contention_target_matches_kind,
        promotable_contention_kind,
    };
    use crate::{
        register_craft_actions, register_harvest_actions, register_heal_action,
        register_loot_action, register_needs_actions,
    };
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        ActionDefId, BodyPart, CauseRef, ClaimantOutcome, CommodityKind,
        ContentionDispositionProfile, ContentionIntents, ContentionPolicy, ContentionQueue,
        ContentionResolutionRule, ControlSource, DeadAt, DeprivationKind, EntityId, EntityKind,
        EventLog, EventTag, EventView, GoalKey, GoalKind, KnownRecipes, PlaceTag, ProductionJob,
        ProductionOutputOwner, ProductionOutputOwnershipPolicy, Quantity, QueuedContentionIntent,
        ResourceSource, Seed, SourceKey, Tick, UniqueItemKind, VisibilitySpec, WitnessData,
        WorkstationMarker, WorkstationTag, World, WorldTxn, Wound, WoundCause, WoundId, WoundList,
        build_prototype_world,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionDuration, ActionHandlerRegistry, ActionInstance, ActionInstanceId,
        ActionPayload, ActionState, ActionStatus, DeterministicRng, RecipeDefinition,
        RecipeRegistry, SystemExecutionContext, SystemId,
    };

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn pm(value: u16) -> worldwake_core::Permille {
        worldwake_core::Permille::new(value).unwrap()
    }

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

    fn add_possessed_lot(
        world: &mut World,
        actor: EntityId,
        place: EntityId,
        commodity: CommodityKind,
        quantity: u32,
    ) {
        let mut txn = new_txn(world, 2);
        let lot = txn.create_item_lot(commodity, Quantity(quantity)).unwrap();
        txn.set_ground_location(lot, place).unwrap();
        txn.set_possessor(lot, actor).unwrap();
        commit_txn(txn);
    }

    fn build_recipe_registry() -> RecipeRegistry {
        let mut recipes = RecipeRegistry::new();
        let _ = recipes.register(RecipeDefinition {
            name: "Harvest Apples".to_string(),
            inputs: Vec::new(),
            outputs: vec![(CommodityKind::Apple, Quantity(2))],
            work_ticks: nz(2),
            required_workstation_tag: Some(WorkstationTag::OrchardRow),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
        });
        let _ = recipes.register(RecipeDefinition {
            name: "Craft Bread".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(1))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: nz(3),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
        });
        recipes
    }

    fn setup_registries(
        recipes: &RecipeRegistry,
    ) -> (
        ActionDefRegistry,
        ActionHandlerRegistry,
        ActionDefId,
        ActionDefId,
    ) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let harvest_id = register_harvest_actions(&mut defs, &mut handlers, recipes)[0];
        let craft_id = register_craft_actions(&mut defs, &mut handlers, recipes)[0];
        (defs, handlers, harvest_id, craft_id)
    }

    fn setup_world(tag: WorkstationTag, stock: u32) -> (World, EntityId, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let facility = {
            let mut txn = new_txn(&mut world, 1);
            let facility = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(facility, place).unwrap();
            txn.set_component_workstation_marker(facility, WorkstationMarker(tag))
                .unwrap();
            txn.set_component_contention_policy(
                facility,
                ContentionPolicy {
                    grant_hold_ticks: nz(3),
                    auto_promote: true,
                    max_waiters: None,
                },
            )
            .unwrap();
            txn.set_component_contention_queue(facility, ContentionQueue::default())
                .unwrap();
            txn.set_component_production_output_ownership_policy(
                facility,
                ProductionOutputOwnershipPolicy {
                    output_owner: ProductionOutputOwner::Actor,
                },
            )
            .unwrap();
            if tag == WorkstationTag::OrchardRow {
                txn.set_component_resource_source(
                    facility,
                    ResourceSource {
                        commodity: CommodityKind::Apple,
                        available_quantity: Quantity(stock),
                        max_quantity: Quantity(8),
                        regeneration_ticks_per_unit: None,
                        last_regeneration_tick: None,
                        extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                        extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                    },
                )
                .unwrap();
            }
            commit_txn(txn);
            facility
        };
        (world, place, facility)
    }

    fn add_actor(
        world: &mut World,
        place: EntityId,
        recipe_id: worldwake_core::RecipeId,
    ) -> EntityId {
        let mut txn = new_txn(world, 1);
        let actor = txn.create_agent("Queue Actor", ControlSource::Ai).unwrap();
        txn.set_ground_location(actor, place).unwrap();
        txn.set_component_known_recipes(actor, KnownRecipes::with([recipe_id]))
            .unwrap();
        commit_txn(txn);
        actor
    }

    fn enqueue(
        world: &mut World,
        facility: EntityId,
        actor: EntityId,
        intended_action: ActionDefId,
    ) {
        enqueue_at(world, facility, actor, intended_action, 2);
    }

    fn enqueue_at(
        world: &mut World,
        facility: EntityId,
        actor: EntityId,
        intended_action: ActionDefId,
        tick: u64,
    ) {
        let mut txn = new_txn(world, tick);
        let mut queue = txn
            .get_component_contention_queue(facility)
            .cloned()
            .unwrap();
        queue
            .enqueue(actor, intended_action, Tick(tick), None)
            .unwrap();
        txn.set_component_contention_queue(facility, queue).unwrap();
        if txn.entity_kind(actor).is_some() {
            txn.set_component_contention_intents(
                actor,
                ContentionIntents {
                    intents: BTreeMap::from([(
                        facility,
                        QueuedContentionIntent {
                            goal_key: GoalKey::new(GoalKind::Sleep),
                            intended_action,
                        },
                    )]),
                },
            )
            .unwrap();
        }
        commit_txn(txn);
    }

    fn set_auto_promote(world: &mut World, facility: EntityId, auto_promote: bool) {
        let mut txn = new_txn(world, 2);
        let mut policy = txn
            .get_component_contention_policy(facility)
            .cloned()
            .unwrap();
        policy.auto_promote = auto_promote;
        txn.set_component_contention_policy(facility, policy)
            .unwrap();
        commit_txn(txn);
    }

    fn set_queue_patience(world: &mut World, actor: EntityId, ticks: u32) {
        let mut txn = new_txn(world, 2);
        txn.set_component_contention_disposition_profile(
            actor,
            ContentionDispositionProfile {
                queue_patience_ticks: Some(nz(ticks)),
            },
        )
        .unwrap();
        commit_txn(txn);
    }

    fn run_system(
        world: &mut World,
        log: &mut EventLog,
        defs: &ActionDefRegistry,
        active_actions: &BTreeMap<ActionInstanceId, ActionInstance>,
        tick: u64,
    ) {
        let mut rng = DeterministicRng::new(Seed([0xAB; 32]));
        contention_system(SystemExecutionContext {
            world,
            event_log: log,
            rng: &mut rng,
            active_actions,
            action_defs: defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(tick),
            system_id: SystemId::Contention,
        })
        .unwrap();
    }

    fn active_action(
        def_id: ActionDefId,
        payload: ActionPayload,
        actor: EntityId,
        facility: EntityId,
    ) -> ActionInstance {
        ActionInstance {
            instance_id: ActionInstanceId(1),
            def_id,
            payload,
            actor,
            targets: vec![facility],
            start_tick: Tick(5),
            remaining_duration: ActionDuration::new(2),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: Some(ActionState::Empty),
            body_cost_override: None,
        }
    }

    fn find_action_id(defs: &ActionDefRegistry, name: &str) -> ActionDefId {
        defs.iter()
            .find_map(|def| (def.name == name).then_some(def.id))
            .unwrap_or_else(|| panic!("expected action {name} to be registered"))
    }

    #[test]
    fn promotable_contention_kind_classifies_wash_action_as_self_care_wash() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_needs_actions(&mut defs, &mut handlers);
        let wash = defs.get(find_action_id(&defs, "wash")).unwrap();

        assert!(matches!(
            promotable_contention_kind(wash),
            Some(PromotableContentionKind::SelfCareWash)
        ));
    }

    #[test]
    fn promotable_contention_kind_classifies_toilet_action_as_self_care_latrine() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_needs_actions(&mut defs, &mut handlers);
        let toilet = defs.get(find_action_id(&defs, "toilet")).unwrap();

        assert!(matches!(
            promotable_contention_kind(toilet),
            Some(PromotableContentionKind::SelfCareLatrine)
        ));
    }

    #[test]
    fn promotable_contention_kind_unchanged_for_existing_actions() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, craft_id) = setup_registries(&recipes);

        assert!(matches!(
            promotable_contention_kind(defs.get(harvest_id).unwrap()),
            Some(PromotableContentionKind::FacilityExclusive(
                WorkstationTag::OrchardRow
            ))
        ));
        assert!(matches!(
            promotable_contention_kind(defs.get(craft_id).unwrap()),
            Some(PromotableContentionKind::FacilityExclusive(
                WorkstationTag::Mill
            ))
        ));

        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let loot_id = register_loot_action(&mut defs, &mut handlers);
        let heal_id = register_heal_action(&mut defs, &mut handlers);

        assert!(matches!(
            promotable_contention_kind(defs.get(loot_id).unwrap()),
            Some(PromotableContentionKind::Corpse)
        ));
        assert!(matches!(
            promotable_contention_kind(defs.get(heal_id).unwrap()),
            Some(PromotableContentionKind::Care)
        ));
    }

    #[test]
    fn self_care_contention_kinds_match_only_their_eligible_targets() {
        let (world, place, wash_basin) = setup_world(WorkstationTag::WashBasin, 0);
        let non_latrine_place = world
            .topology()
            .place_ids()
            .find(|candidate| !world.place_has_tag(*candidate, PlaceTag::Latrine))
            .unwrap();
        let latrine = world
            .topology()
            .place_ids()
            .find(|candidate| world.place_has_tag(*candidate, PlaceTag::Latrine))
            .unwrap();

        assert!(contention_target_matches_kind(
            &world,
            wash_basin,
            PromotableContentionKind::SelfCareWash
        ));
        assert!(!contention_target_matches_kind(
            &world,
            place,
            PromotableContentionKind::SelfCareWash
        ));
        assert!(contention_target_matches_kind(
            &world,
            latrine,
            PromotableContentionKind::SelfCareLatrine
        ));
        assert!(!contention_target_matches_kind(
            &world,
            non_latrine_place,
            PromotableContentionKind::SelfCareLatrine
        ));
    }

    fn assert_single_contention_grant_event(
        log: &EventLog,
        facility: EntityId,
        place: EntityId,
        action: ActionDefId,
        tick: Tick,
        expected_claimants: &[(EntityId, Tick, ClaimantOutcome)],
        winner: EntityId,
    ) {
        assert_eq!(log.events_by_tag(EventTag::ContentionResolved).len(), 1);
        let contention_event_id = log.events_by_tag(EventTag::ContentionResolved)[0];
        assert!(
            log.events_by_tag(EventTag::QueueGrantPromoted)
                .contains(&contention_event_id),
            "contention payload should be attached to the promoted-grant event"
        );
        let record = log.get(contention_event_id).unwrap();
        let payload = record
            .contention_event_payload()
            .expect("contention event carries typed payload");
        assert_eq!(payload.contested_affordance.facility, facility);
        assert_eq!(payload.contested_affordance.action, action);
        assert_eq!(payload.place, place);
        assert_eq!(
            payload.resolution_rule,
            ContentionResolutionRule::ArrivalTime
        );
        assert_eq!(payload.winner, Some(winner));
        assert_eq!(payload.at_tick, tick);
        assert_eq!(payload.total_claimants, expected_claimants.len() as u16);
        assert_eq!(payload.claimants.len(), expected_claimants.len());
        for (index, (claimant, arrived_tick, outcome)) in expected_claimants.iter().enumerate() {
            let emitted = &payload.claimants[index];
            assert_eq!(emitted.agent, *claimant);
            assert_eq!(emitted.arrived_tick, *arrived_tick);
            assert_eq!(emitted.queue_position, (index + 1) as u16);
            assert_eq!(emitted.outcome, *outcome);
        }
    }

    #[test]
    fn dead_actor_is_pruned_from_queue() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, harvest_id);
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_dead_at(
                actor,
                worldwake_core::DeadAt {
                    tick: Tick(3),
                    cause: worldwake_core::DeathCause::CombatWounds,
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        run_system(&mut world, &mut EventLog::new(), &defs, &BTreeMap::new(), 3);

        assert!(
            world
                .get_component_contention_queue(facility)
                .unwrap()
                .waiting
                .is_empty()
        );
    }

    #[test]
    fn departed_actor_is_pruned_from_queue() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        let other_place = world
            .topology()
            .place_ids()
            .find(|candidate| *candidate != place)
            .unwrap();
        enqueue(&mut world, facility, actor, harvest_id);
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_ground_location(actor, other_place).unwrap();
            commit_txn(txn);
        }

        run_system(&mut world, &mut EventLog::new(), &defs, &BTreeMap::new(), 3);

        assert!(
            world
                .get_component_contention_queue(facility)
                .unwrap()
                .waiting
                .is_empty()
        );
    }

    #[test]
    fn deallocated_actor_is_pruned_from_queue() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let _ = place;
        let actor = EntityId {
            slot: 999,
            generation: 1,
        };
        enqueue(&mut world, facility, actor, harvest_id);

        run_system(&mut world, &mut EventLog::new(), &defs, &BTreeMap::new(), 3);

        assert!(
            world
                .get_component_contention_queue(facility)
                .unwrap()
                .waiting
                .is_empty()
        );
    }

    #[test]
    fn expired_grant_is_cleared_and_emits_event() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, harvest_id);
        {
            let mut txn = new_txn(&mut world, 3);
            let mut queue = txn
                .get_component_contention_queue(facility)
                .cloned()
                .unwrap();
            queue.promote_head(Tick(3), nz(2));
            txn.set_component_contention_queue(facility, queue).unwrap();
            commit_txn(txn);
        }
        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 5);

        let queue = world.get_component_contention_queue(facility).unwrap();
        assert!(queue.granted.is_none());
        assert_eq!(log.events_by_tag(EventTag::QueueGrantExpired).len(), 1);
        let record = log
            .get(log.events_by_tag(EventTag::QueueGrantExpired)[0])
            .unwrap();
        assert_eq!(record.visibility(), VisibilitySpec::SamePlace);
        assert!(log.events_by_tag(EventTag::ContentionResolved).is_empty());
    }

    #[test]
    fn expired_grant_auto_promotes_head_when_enabled() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let first = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        let second = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, first, harvest_id);
        enqueue(&mut world, facility, second, harvest_id);
        {
            let mut txn = new_txn(&mut world, 3);
            let mut queue = txn
                .get_component_contention_queue(facility)
                .cloned()
                .unwrap();
            queue.promote_head(Tick(3), nz(2));
            txn.set_component_contention_queue(facility, queue).unwrap();
            commit_txn(txn);
        }
        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 5);

        let queue = world.get_component_contention_queue(facility).unwrap();
        assert_eq!(
            queue.granted.as_ref().map(|grant| grant.actor),
            Some(second)
        );
        assert_eq!(log.events_by_tag(EventTag::QueueGrantExpired).len(), 1);
        assert_eq!(log.events_by_tag(EventTag::QueueGrantPromoted).len(), 1);
        assert_single_contention_grant_event(
            &log,
            facility,
            place,
            harvest_id,
            Tick(5),
            &[(second, Tick(2), ClaimantOutcome::Granted)],
            second,
        );
    }

    #[test]
    fn structurally_invalid_head_is_pruned_and_emits_event() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, harvest_id);
        {
            let mut txn = new_txn(&mut world, 3);
            txn.clear_component_workstation_marker(facility).unwrap();
            commit_txn(txn);
        }
        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 3);

        assert!(
            world
                .get_component_contention_queue(facility)
                .unwrap()
                .waiting
                .is_empty()
        );
        assert_eq!(log.events_by_tag(EventTag::QueueHeadFailed).len(), 1);
        assert!(log.events_by_tag(EventTag::ContentionResolved).is_empty());
    }

    #[test]
    fn missing_intended_action_head_is_pruned_and_emits_event() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, _harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, ActionDefId(999));
        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 3);

        assert!(
            world
                .get_component_contention_queue(facility)
                .unwrap()
                .waiting
                .is_empty()
        );
        assert_eq!(log.events_by_tag(EventTag::QueueHeadFailed).len(), 1);
        assert!(log.events_by_tag(EventTag::ContentionResolved).is_empty());
    }

    #[test]
    fn waiter_with_mismatched_contention_intent_is_pruned() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, harvest_id);
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_contention_intents(
                actor,
                ContentionIntents {
                    intents: BTreeMap::from([(
                        facility,
                        QueuedContentionIntent {
                            goal_key: GoalKey::new(GoalKind::Sleep),
                            intended_action: ActionDefId(999),
                        },
                    )]),
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        run_system(&mut world, &mut EventLog::new(), &defs, &BTreeMap::new(), 3);

        let queue = world.get_component_contention_queue(facility).unwrap();
        assert!(queue.waiting.is_empty());
    }

    #[test]
    fn patience_exceeded_waiter_is_pruned() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, harvest_id);
        set_queue_patience(&mut world, actor, 1);

        run_system(&mut world, &mut EventLog::new(), &defs, &BTreeMap::new(), 3);

        let queue = world.get_component_contention_queue(facility).unwrap();
        assert!(queue.waiting.is_empty());
        assert!(world.get_component_contention_intents(actor).is_none());
    }

    #[test]
    fn depleted_stock_stalls_queue_without_pruning_or_promotion() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 0);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, harvest_id);
        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 3);

        let queue = world.get_component_contention_queue(facility).unwrap();
        assert_eq!(queue.position_of(actor), Some(0));
        assert!(queue.granted.is_none());
        assert!(log.events_by_tag(EventTag::QueueHeadFailed).is_empty());
        assert!(log.events_by_tag(EventTag::QueueGrantPromoted).is_empty());
    }

    #[test]
    fn occupied_craft_workstation_stalls_queue_without_pruning_or_promotion() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, _harvest_id, craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::Mill, 0);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(1));
        add_possessed_lot(&mut world, actor, place, CommodityKind::Grain, 1);
        enqueue(&mut world, facility, actor, craft_id);
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_production_job(
                facility,
                ProductionJob {
                    recipe_id: worldwake_core::RecipeId(1),
                    worker: actor,
                    staged_inputs_container: facility,
                    progress_ticks: 1,
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        run_system(&mut world, &mut EventLog::new(), &defs, &BTreeMap::new(), 3);

        let queue = world.get_component_contention_queue(facility).unwrap();
        assert_eq!(queue.position_of(actor), Some(0));
        assert!(queue.granted.is_none());
    }

    #[test]
    fn ready_head_is_promoted_with_expected_expiry_and_event() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, harvest_id);
        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 3);

        let queue = world.get_component_contention_queue(facility).unwrap();
        let granted = queue.granted.as_ref().unwrap();
        assert_eq!(granted.actor, actor);
        assert_eq!(granted.expires_at, Tick(6));
        assert_eq!(log.events_by_tag(EventTag::QueueGrantPromoted).len(), 1);
        assert_single_contention_grant_event(
            &log,
            facility,
            place,
            harvest_id,
            Tick(3),
            &[(actor, Tick(2), ClaimantOutcome::Granted)],
            actor,
        );
    }

    #[test]
    fn promote_ready_head_records_wait_observation_for_resource_source_facility() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue_at(&mut world, facility, actor, harvest_id, 10);
        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 15);

        let reliability = world
            .get_component_source_reliability(actor)
            .expect("agents carry source reliability");
        let record = reliability
            .sources
            .get(&SourceKey {
                entity: facility,
                commodity: CommodityKind::Apple,
            })
            .expect("wait observation should be recorded for resource source");
        assert_eq!(record.average_wait_ticks, 5);
        assert_eq!(record.wait_observation_count, 1);
        assert_single_contention_grant_event(
            &log,
            facility,
            place,
            harvest_id,
            Tick(15),
            &[(actor, Tick(10), ClaimantOutcome::Granted)],
            actor,
        );
    }

    #[test]
    fn promote_ready_head_skips_wait_observation_when_facility_has_no_resource_source() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, _harvest_id, craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::Mill, 0);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(1));
        add_possessed_lot(&mut world, actor, place, CommodityKind::Grain, 1);
        enqueue_at(&mut world, facility, actor, craft_id, 10);

        run_system(
            &mut world,
            &mut EventLog::new(),
            &defs,
            &BTreeMap::new(),
            15,
        );

        assert!(
            world
                .get_component_source_reliability(actor)
                .is_none_or(|reliability| reliability.sources.is_empty())
        );
    }

    #[test]
    fn dead_agent_corpse_head_is_promoted() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (actor, corpse) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Looter", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            let corpse = txn.create_agent("Corpse", ControlSource::Ai).unwrap();
            txn.set_ground_location(corpse, place).unwrap();
            txn.set_component_dead_at(
                corpse,
                DeadAt {
                    tick: Tick(1),
                    cause: worldwake_core::DeathCause::CombatWounds,
                },
            )
            .unwrap();
            txn.set_component_contention_policy(
                corpse,
                ContentionPolicy {
                    grant_hold_ticks: nz(3),
                    auto_promote: true,
                    max_waiters: None,
                },
            )
            .unwrap();
            txn.set_component_contention_queue(corpse, ContentionQueue::default())
                .unwrap();
            commit_txn(txn);
            (actor, corpse)
        };

        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let loot_id = register_loot_action(&mut defs, &mut handlers);
        enqueue(&mut world, corpse, actor, loot_id);
        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 3);

        let queue = world.get_component_contention_queue(corpse).unwrap();
        let granted = queue.granted.as_ref().unwrap();
        assert_eq!(granted.actor, actor);
        assert_eq!(granted.intended_action, loot_id);
        assert_eq!(log.events_by_tag(EventTag::QueueGrantPromoted).len(), 1);
        assert_single_contention_grant_event(
            &log,
            corpse,
            place,
            loot_id,
            Tick(3),
            &[(actor, Tick(2), ClaimantOutcome::Granted)],
            actor,
        );
    }

    #[test]
    fn wounded_agent_care_head_is_promoted() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (actor, patient) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Healer", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            let patient = txn.create_agent("Patient", ControlSource::Ai).unwrap();
            txn.set_ground_location(patient, place).unwrap();
            txn.set_component_wound_list(
                patient,
                WoundList {
                    wounds: vec![Wound {
                        id: WoundId(1),
                        body_part: BodyPart::Torso,
                        cause: WoundCause::Deprivation(DeprivationKind::Starvation),
                        severity: pm(100),
                        inflicted_at: Tick(1),
                        bleed_rate_per_tick: pm(0),
                    }],
                },
            )
            .unwrap();
            txn.set_component_contention_policy(
                patient,
                ContentionPolicy {
                    grant_hold_ticks: nz(3),
                    auto_promote: true,
                    max_waiters: None,
                },
            )
            .unwrap();
            txn.set_component_contention_queue(patient, ContentionQueue::default())
                .unwrap();
            commit_txn(txn);
            (actor, patient)
        };

        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let heal_id = register_heal_action(&mut defs, &mut handlers);
        add_possessed_lot(&mut world, actor, place, CommodityKind::Medicine, 1);
        enqueue(&mut world, patient, actor, heal_id);
        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 3);

        let queue = world.get_component_contention_queue(patient).unwrap();
        let granted = queue.granted.as_ref().unwrap();
        assert_eq!(granted.actor, actor);
        assert_eq!(granted.intended_action, heal_id);
        assert_eq!(log.events_by_tag(EventTag::QueueGrantPromoted).len(), 1);
        assert_single_contention_grant_event(
            &log,
            patient,
            place,
            heal_id,
            Tick(3),
            &[(actor, Tick(2), ClaimantOutcome::Granted)],
            actor,
        );
    }

    #[test]
    fn auto_promote_false_leaves_ready_head_waiting() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, harvest_id);
        set_auto_promote(&mut world, facility, false);
        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 3);

        let queue = world.get_component_contention_queue(facility).unwrap();
        assert_eq!(queue.position_of(actor), Some(0));
        assert!(queue.granted.is_none());
        assert!(log.events_by_tag(EventTag::QueueGrantPromoted).is_empty());
        assert!(log.events_by_tag(EventTag::ContentionResolved).is_empty());
    }

    #[test]
    fn active_exclusive_action_blocks_promotion() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, harvest_id);
        let harvest_payload = defs.get(harvest_id).unwrap().payload.clone();
        let active = BTreeMap::from([(
            ActionInstanceId(9),
            active_action(harvest_id, harvest_payload, actor, facility),
        )]);

        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &active, 3);

        assert!(
            world
                .get_component_contention_queue(facility)
                .unwrap()
                .granted
                .is_none()
        );
        assert!(log.events_by_tag(EventTag::ContentionResolved).is_empty());
    }

    #[test]
    fn every_facility_grant_emits_contention_event() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let first = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        let second = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        let third = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue_at(&mut world, facility, first, harvest_id, 6);
        enqueue_at(&mut world, facility, second, harvest_id, 7);
        enqueue_at(&mut world, facility, third, harvest_id, 8);
        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 9);

        assert_eq!(
            world
                .get_component_contention_queue(facility)
                .unwrap()
                .granted
                .as_ref()
                .map(|grant| grant.actor),
            Some(first)
        );
        assert_eq!(log.events_by_tag(EventTag::QueueGrantPromoted).len(), 1);
        assert_single_contention_grant_event(
            &log,
            facility,
            place,
            harvest_id,
            Tick(9),
            &[
                (first, Tick(6), ClaimantOutcome::Granted),
                (second, Tick(7), ClaimantOutcome::QueuedBehind),
                (third, Tick(8), ClaimantOutcome::QueuedBehind),
            ],
            first,
        );
    }

    #[test]
    fn system_is_idempotent_within_same_tick() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, harvest_id);
        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 3);
        let first_queue = world
            .get_component_contention_queue(facility)
            .unwrap()
            .clone();
        let first_event_count = log.len();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 3);

        assert_eq!(
            world.get_component_contention_queue(facility).unwrap(),
            &first_queue
        );
        assert_eq!(log.len(), first_event_count);
    }

    #[test]
    fn ground_unique_item_receives_race_mode_contention_state() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, _harvest_id, _craft_id) = setup_registries(&recipes);
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = entity(1);
        let item = {
            let mut txn = new_txn(&mut world, 1);
            let item = txn
                .create_unique_item(UniqueItemKind::Artifact, Some("Seal"), BTreeMap::new())
                .unwrap();
            txn.set_ground_location(item, place).unwrap();
            commit_txn(txn);
            item
        };

        run_system(&mut world, &mut EventLog::new(), &defs, &BTreeMap::new(), 2);

        let policy = world
            .get_component_contention_policy(item)
            .expect("ground unique item should receive contention policy");
        assert_eq!(policy.max_waiters, Some(0));
        assert!(!policy.auto_promote);
        assert_eq!(
            world.get_component_contention_queue(item),
            Some(&ContentionQueue::default())
        );
    }

    #[test]
    fn unique_item_contention_state_is_removed_after_item_leaves_ground() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, _harvest_id, _craft_id) = setup_registries(&recipes);
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = entity(1);
        let (actor, item) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let item = txn
                .create_unique_item(UniqueItemKind::Misc, Some("Token"), BTreeMap::new())
                .unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(item, place).unwrap();
            commit_txn(txn);
            (actor, item)
        };

        run_system(&mut world, &mut EventLog::new(), &defs, &BTreeMap::new(), 2);
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_possessor(item, actor).unwrap();
            commit_txn(txn);
        }

        run_system(&mut world, &mut EventLog::new(), &defs, &BTreeMap::new(), 4);

        assert_eq!(world.get_component_contention_policy(item), None);
        assert_eq!(world.get_component_contention_queue(item), None);
    }
}
