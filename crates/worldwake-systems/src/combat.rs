use crate::inventory::{
    consume_one_unit_of_commodity, controlled_entity_load, move_entity_to_direct_possession,
    remaining_capacity,
};
use std::collections::BTreeSet;
use std::num::NonZeroU32;
use worldwake_core::{
    is_wound_load_fatal, load_per_unit, ActionDefId, BodyCostPerTick, BodyPart, CauseRef,
    CombatStance, CombatWeaponProfile, CombatWeaponRef, ComponentDelta, ComponentKind, DeadAt,
    DriveThresholds, EntityId, EntityKind, EventLog, EventTag, EvidenceRef, HomeostaticNeeds,
    Permille, Quantity, StateDelta, VisibilitySpec, WitnessData, WorldTxn, Wound, WoundCause,
    WoundList,
};
use worldwake_sim::{
    AbortReason, ActionAbortRequestReason, ActionDef, ActionDefRegistry, ActionDomain, ActionError,
    ActionHandler, ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionPayload,
    ActionProgress, ActionState, BeliefView, CombatActionPayload, CommitOutcome, Constraint,
    DeterministicRng, DurationExpr, Interruptibility, LootActionPayload, PayloadEntityRole,
    Precondition, SelfTargetActionKind, SystemError, SystemExecutionContext, TargetSpec,
};

const BODY_PARTS: [BodyPart; 6] = [
    BodyPart::Head,
    BodyPart::Torso,
    BodyPart::LeftArm,
    BodyPart::RightArm,
    BodyPart::LeftLeg,
    BodyPart::RightLeg,
];

pub fn register_attack_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(start_attack, tick_attack, commit_attack, abort_attack)
            .with_affordance_payloads(enumerate_attack_payloads),
    );
    defs.register(attack_action_def(ActionDefId(defs.len() as u32), handler))
}

pub fn register_defend_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(ActionHandler::new(
        start_defend,
        tick_defend,
        commit_defend,
        abort_defend,
    ));
    defs.register(defend_action_def(ActionDefId(defs.len() as u32), handler))
}

pub fn register_loot_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(start_loot, tick_loot, commit_loot, abort_loot)
            .with_affordance_payloads(enumerate_loot_payloads),
    );
    defs.register(loot_action_def(ActionDefId(defs.len() as u32), handler))
}

pub fn register_heal_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(ActionHandler::new(
        start_heal,
        tick_heal,
        commit_heal,
        abort_heal,
    ));
    defs.register(heal_action_def(ActionDefId(defs.len() as u32), handler))
}

pub fn combat_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let SystemExecutionContext {
        world,
        event_log,
        rng: _rng,
        active_actions,
        action_defs,
        tick,
        system_id: _system_id,
    } = ctx;
    apply_wound_progression(world, event_log, active_actions, action_defs, tick)?;
    let fatalities = collect_fatalities(world, event_log, tick);

    for fatality in fatalities {
        let place = world.effective_place(fatality.entity);
        let mut txn = WorldTxn::new(
            world,
            tick,
            fatality.cause,
            Some(fatality.entity),
            place,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        txn.add_tag(EventTag::System)
            .add_tag(EventTag::WorldMutation)
            .add_tag(EventTag::Combat)
            .add_target(fatality.entity);
        txn.set_component_dead_at(fatality.entity, DeadAt(tick))
            .map_err(|error| SystemError::new(error.to_string()))?;
        let pending = txn.into_pending_event().with_evidence(fatality.evidence);
        let _ = event_log.emit(pending);
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WoundProgressionUpdate {
    entity: worldwake_core::EntityId,
    wounds: WoundList,
}

fn apply_wound_progression(
    world: &mut worldwake_core::World,
    event_log: &mut EventLog,
    active_actions: &std::collections::BTreeMap<worldwake_sim::ActionInstanceId, ActionInstance>,
    action_defs: &ActionDefRegistry,
    tick: worldwake_core::Tick,
) -> Result<(), SystemError> {
    let updates = collect_wound_progression_updates(world, active_actions, action_defs);
    if updates.is_empty() {
        return Ok(());
    }

    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        None,
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.add_tag(EventTag::System)
        .add_tag(EventTag::WorldMutation);

    for update in updates {
        txn.set_component_wound_list(update.entity, update.wounds)
            .map_err(|error| SystemError::new(error.to_string()))?;
    }

    let _ = txn.commit(event_log);
    Ok(())
}

fn collect_wound_progression_updates(
    world: &worldwake_core::World,
    active_actions: &std::collections::BTreeMap<worldwake_sim::ActionInstanceId, ActionInstance>,
    action_defs: &ActionDefRegistry,
) -> Vec<WoundProgressionUpdate> {
    world
        .query_wound_list()
        .filter_map(|(entity, wounds)| {
            if world.get_component_dead_at(entity).is_some() || wounds.wounds.is_empty() {
                return None;
            }

            let profile = world.get_component_combat_profile(entity).copied()?;
            let needs = world.get_component_homeostatic_needs(entity).copied();
            let thresholds = world.get_component_drive_thresholds(entity).copied();
            let engaged_in_combat =
                entity_is_engaged_in_combat(entity, active_actions, action_defs);
            progress_wounds(wounds, profile, needs, thresholds, engaged_in_combat)
                .map(|wounds| WoundProgressionUpdate { entity, wounds })
        })
        .collect()
}

fn progress_wounds(
    current: &WoundList,
    profile: worldwake_core::CombatProfile,
    needs: Option<HomeostaticNeeds>,
    thresholds: Option<DriveThresholds>,
    engaged_in_combat: bool,
) -> Option<WoundList> {
    let can_recover = recovery_conditions_met(needs, thresholds, engaged_in_combat);
    let mut next = current.clone();
    let mut changed = false;

    for wound in &mut next.wounds {
        if wound.bleed_rate_per_tick.value() > 0 {
            let previous_severity = wound.severity;
            let previous_bleed_rate = wound.bleed_rate_per_tick;
            wound.severity = wound.severity.saturating_add(wound.bleed_rate_per_tick);
            wound.bleed_rate_per_tick = wound
                .bleed_rate_per_tick
                .saturating_sub(profile.natural_clot_resistance);
            changed |= wound.severity != previous_severity
                || wound.bleed_rate_per_tick != previous_bleed_rate;
            continue;
        }

        if can_recover && wound.severity.value() > 0 {
            let previous_severity = wound.severity;
            wound.severity = wound.severity.saturating_sub(profile.natural_recovery_rate);
            changed |= wound.severity != previous_severity;
        }
    }

    let previous_len = next.wounds.len();
    next.wounds.retain(|wound| wound.severity.value() > 0);
    changed |= next.wounds.len() != previous_len;

    changed.then_some(next)
}

fn recovery_conditions_met(
    needs: Option<HomeostaticNeeds>,
    thresholds: Option<DriveThresholds>,
    engaged_in_combat: bool,
) -> bool {
    if engaged_in_combat {
        return false;
    }

    let (Some(needs), Some(thresholds)) = (needs, thresholds) else {
        return false;
    };

    needs.hunger < thresholds.hunger.high()
        && needs.thirst < thresholds.thirst.high()
        && needs.fatigue < thresholds.fatigue.high()
}

fn entity_is_engaged_in_combat(
    entity: worldwake_core::EntityId,
    active_actions: &std::collections::BTreeMap<worldwake_sim::ActionInstanceId, ActionInstance>,
    action_defs: &ActionDefRegistry,
) -> bool {
    active_actions.values().any(|action| {
        let Some(def) = action_defs.get(action.def_id) else {
            return false;
        };

        def.domain.counts_as_combat_engagement()
            && (action.actor == entity || action.targets.contains(&entity))
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Fatality {
    entity: worldwake_core::EntityId,
    evidence: Vec<EvidenceRef>,
    cause: CauseRef,
}

fn collect_fatalities(
    world: &worldwake_core::World,
    event_log: &EventLog,
    tick: worldwake_core::Tick,
) -> Vec<Fatality> {
    world
        .query_wound_list()
        .filter_map(|(entity, wounds)| {
            let profile = world.get_component_combat_profile(entity)?;
            if world.get_component_dead_at(entity).is_some() {
                return None;
            }
            is_wound_load_fatal(wounds, profile).then_some(Fatality {
                entity,
                evidence: wounds
                    .wound_ids()
                    .into_iter()
                    .map(|wound_id| EvidenceRef::Wound { entity, wound_id })
                    .collect(),
                cause: latest_wound_event(event_log, entity)
                    .map_or(CauseRef::SystemTick(tick), CauseRef::Event),
            })
        })
        .collect()
}

fn latest_wound_event(
    event_log: &EventLog,
    entity: worldwake_core::EntityId,
) -> Option<worldwake_core::EventId> {
    (0..event_log.len())
        .rev()
        .map(|index| worldwake_core::EventId(index as u64))
        .find(|event_id| {
            event_log.get(*event_id).is_some_and(|record| {
                record.state_deltas.iter().any(|delta| {
                    matches!(
                        delta,
                        StateDelta::Component(ComponentDelta::Set {
                            entity: changed,
                            component_kind: ComponentKind::WoundList,
                            ..
                        }) if *changed == entity
                    )
                })
            })
        })
}

fn attack_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    let preconditions = vec![
        Precondition::ActorAlive,
        Precondition::TargetExists(0),
        Precondition::TargetAtActorPlace(0),
        Precondition::TargetAlive(0),
        Precondition::TargetIsAgent(0),
    ];

    ActionDef {
        id,
        name: "attack".to_string(),
        domain: ActionDomain::Combat,
        actor_constraints: vec![
            Constraint::ActorAlive,
            Constraint::ActorNotDead,
            Constraint::ActorNotIncapacitated,
            Constraint::ActorNotInTransit,
            Constraint::ActorHasControl,
        ],
        targets: vec![TargetSpec::EntityAtActorPlace {
            kind: worldwake_core::EntityKind::Agent,
        }],
        preconditions: preconditions.clone(),
        reservation_requirements: Vec::new(),
        duration: DurationExpr::CombatWeapon,
        body_cost_per_tick: BodyCostPerTick::zero(),
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: preconditions,
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::Combat, EventTag::WorldMutation]),
        payload: ActionPayload::None,
        handler,
    }
}

fn enumerate_attack_payloads(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    view: &dyn BeliefView,
) -> Vec<ActionPayload> {
    let Some(target) = targets.first().copied() else {
        return Vec::new();
    };

    let mut payloads = vec![ActionPayload::Combat(CombatActionPayload {
        target,
        weapon: CombatWeaponRef::Unarmed,
    })];
    for commodity in worldwake_core::CommodityKind::ALL {
        if commodity.spec().combat_weapon_profile.is_some()
            && view.commodity_quantity(actor, commodity) > Quantity(0)
        {
            payloads.push(ActionPayload::Combat(CombatActionPayload {
                target,
                weapon: CombatWeaponRef::Commodity(commodity),
            }));
        }
    }
    payloads.sort();
    payloads.dedup();
    payloads
}

fn defend_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "defend".to_string(),
        domain: ActionDomain::Combat,
        actor_constraints: vec![
            Constraint::ActorAlive,
            Constraint::ActorNotDead,
            Constraint::ActorNotIncapacitated,
            Constraint::ActorNotInTransit,
            Constraint::ActorHasControl,
        ],
        targets: Vec::new(),
        preconditions: vec![Precondition::ActorAlive],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::Indefinite,
        body_cost_per_tick: BodyCostPerTick::zero(),
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: vec![Precondition::ActorAlive],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::ActionStarted]),
        payload: ActionPayload::None,
        handler,
    }
}

fn loot_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    let preconditions = vec![
        Precondition::ActorAlive,
        Precondition::TargetExists(0),
        Precondition::TargetAtActorPlace(0),
        Precondition::TargetDead(0),
        Precondition::TargetIsAgent(0),
    ];

    ActionDef {
        id,
        name: "loot".to_string(),
        domain: ActionDomain::Loot,
        actor_constraints: vec![
            Constraint::ActorAlive,
            Constraint::ActorNotDead,
            Constraint::ActorNotIncapacitated,
            Constraint::ActorNotInTransit,
            Constraint::ActorHasControl,
        ],
        targets: vec![TargetSpec::EntityAtActorPlace {
            kind: EntityKind::Agent,
        }],
        preconditions: preconditions.clone(),
        reservation_requirements: Vec::new(),
        duration: DurationExpr::Fixed(NonZeroU32::MIN),
        body_cost_per_tick: BodyCostPerTick::zero(),
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: preconditions,
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([
            EventTag::WorldMutation,
            EventTag::Inventory,
            EventTag::Transfer,
        ]),
        payload: ActionPayload::None,
        handler,
    }
}

fn enumerate_loot_payloads(
    _def: &ActionDef,
    _actor: EntityId,
    targets: &[EntityId],
    _view: &dyn BeliefView,
) -> Vec<ActionPayload> {
    targets
        .first()
        .copied()
        .map(|target| ActionPayload::Loot(LootActionPayload { target }))
        .into_iter()
        .collect()
}

fn heal_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    let preconditions = vec![
        Precondition::ActorAlive,
        Precondition::TargetExists(0),
        Precondition::TargetAtActorPlace(0),
        Precondition::TargetAlive(0),
        Precondition::TargetIsAgent(0),
        Precondition::TargetHasWounds(0),
    ];

    ActionDef {
        id,
        name: "heal".to_string(),
        domain: ActionDomain::Care,
        actor_constraints: vec![
            Constraint::ActorAlive,
            Constraint::ActorNotDead,
            Constraint::ActorNotIncapacitated,
            Constraint::ActorNotInTransit,
            Constraint::ActorHasControl,
            Constraint::ActorHasCommodity {
                kind: worldwake_core::CommodityKind::Medicine,
                min_qty: Quantity(1),
            },
        ],
        targets: vec![TargetSpec::EntityAtActorPlace {
            kind: EntityKind::Agent,
        }],
        preconditions: preconditions.clone(),
        reservation_requirements: Vec::new(),
        duration: DurationExpr::TargetTreatment {
            target_index: 0,
            commodity: worldwake_core::CommodityKind::Medicine,
        },
        body_cost_per_tick: BodyCostPerTick::zero(),
        interruptibility: Interruptibility::InterruptibleWithPenalty,
        commit_conditions: vec![
            Precondition::TargetAlive(0),
            Precondition::TargetAtActorPlace(0),
            Precondition::TargetIsAgent(0),
        ],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::WorldMutation, EventTag::Inventory]),
        payload: ActionPayload::None,
        handler,
    }
}

#[allow(clippy::unnecessary_wraps)]
fn start_defend(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    if txn.get_component_combat_stance(instance.actor).is_some() {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::ActorAlreadyHasCombatStance {
                actor: instance.actor,
            },
        ));
    }

    txn.set_component_combat_stance(instance.actor, CombatStance::Defending)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    Ok(None)
}

#[allow(clippy::unnecessary_wraps)]
fn tick_defend(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

#[allow(clippy::unnecessary_wraps)]
fn commit_defend(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    txn.clear_component_combat_stance(instance.actor)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_defend(
    _def: &ActionDef,
    instance: &ActionInstance,
    _reason: &AbortReason,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    txn.clear_component_combat_stance(instance.actor)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    Ok(())
}

fn combat_payload<'a>(
    def: &ActionDef,
    instance: &'a ActionInstance,
) -> Result<&'a CombatActionPayload, ActionError> {
    instance.payload.as_combat().ok_or_else(|| {
        ActionError::InternalError(format!(
            "action instance for def {} is missing combat payload",
            def.id
        ))
    })
}

fn loot_payload(instance: &ActionInstance) -> Option<&LootActionPayload> {
    instance.payload.as_loot()
}

fn validate_loot_context(
    txn: &WorldTxn<'_>,
    instance: &ActionInstance,
) -> Result<(EntityId, EntityId), ActionError> {
    let target = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    if let Some(payload) = loot_payload(instance) {
        if payload.target != target {
            return Err(ActionError::AbortRequested(
                ActionAbortRequestReason::PayloadEntityMismatch {
                    role: PayloadEntityRole::Target,
                    expected: target,
                    actual: payload.target,
                },
            ));
        }
    }
    let place = txn.effective_place(instance.actor).ok_or({
        ActionError::AbortRequested(ActionAbortRequestReason::ActorNotPlaced {
            actor: instance.actor,
        })
    })?;
    if txn.entity_kind(target) != Some(EntityKind::Agent) {
        return Err(ActionError::InvalidTarget(target));
    }
    if txn.effective_place(target) != Some(place) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetNotColocated {
                actor: instance.actor,
                target,
            },
        ));
    }
    if txn.get_component_dead_at(target).is_none() {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetNotDead { target },
        ));
    }
    Ok((target, place))
}

fn validate_attack_context(
    txn: &WorldTxn<'_>,
    instance: &ActionInstance,
    payload: &CombatActionPayload,
) -> Result<worldwake_core::EntityId, ActionError> {
    let target = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    if target != payload.target {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::PayloadEntityMismatch {
                role: PayloadEntityRole::Target,
                expected: target,
                actual: payload.target,
            },
        ));
    }
    if target == instance.actor {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::SelfTargetForbidden {
                actor: instance.actor,
                action: SelfTargetActionKind::Attack,
            },
        ));
    }
    let place = txn.effective_place(instance.actor).ok_or({
        ActionError::AbortRequested(ActionAbortRequestReason::ActorNotPlaced {
            actor: instance.actor,
        })
    })?;
    if txn.effective_place(target) != Some(place) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetNotColocated {
                actor: instance.actor,
                target,
            },
        ));
    }
    Ok(target)
}

fn validate_selected_weapon(
    txn: &WorldTxn<'_>,
    actor: worldwake_core::EntityId,
    weapon: CombatWeaponRef,
) -> Result<(), ActionError> {
    match weapon {
        CombatWeaponRef::Unarmed => Ok(()),
        CombatWeaponRef::Commodity(kind) => {
            if txn.controlled_commodity_quantity(actor, kind).0 > 0 {
                Ok(())
            } else {
                Err(ActionError::AbortRequested(
                    ActionAbortRequestReason::ActorMissingWeaponCommodity {
                        actor,
                        commodity: kind,
                    },
                ))
            }
        }
    }
}

fn validate_heal_context(
    txn: &WorldTxn<'_>,
    instance: &ActionInstance,
) -> Result<worldwake_core::EntityId, ActionError> {
    let target = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    if target == instance.actor {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::SelfTargetForbidden {
                actor: instance.actor,
                action: SelfTargetActionKind::Heal,
            },
        ));
    }
    if txn.entity_kind(target) != Some(EntityKind::Agent) {
        return Err(ActionError::InvalidTarget(target));
    }
    let place = txn.effective_place(instance.actor).ok_or({
        ActionError::AbortRequested(ActionAbortRequestReason::ActorNotPlaced {
            actor: instance.actor,
        })
    })?;
    if txn.effective_place(target) != Some(place) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetNotColocated {
                actor: instance.actor,
                target,
            },
        ));
    }
    if txn.get_component_dead_at(target).is_some() {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetNotAlive { target },
        ));
    }
    let wounds = txn
        .get_component_wound_list(target)
        .ok_or(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetLacksWounds { target },
        ))?;
    if wounds.wounds.is_empty() {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetHasNoWounds { target },
        ));
    }
    Ok(target)
}

fn apply_treatment(
    current: &WoundList,
    profile: worldwake_core::CommodityTreatmentProfile,
) -> Option<WoundList> {
    let mut next = current.clone();
    let mut changed = false;
    let mut bleed_budget = profile.bleed_reduction_per_tick;
    let mut severity_budget = profile.severity_reduction_per_tick;

    for wound in next
        .wounds
        .iter_mut()
        .filter(|wound| wound.bleed_rate_per_tick.value() > 0)
    {
        if bleed_budget.value() == 0 {
            break;
        }
        let previous = wound.bleed_rate_per_tick;
        let reduction = bleed_budget.min(wound.bleed_rate_per_tick);
        wound.bleed_rate_per_tick = wound.bleed_rate_per_tick.saturating_sub(reduction);
        bleed_budget = bleed_budget.saturating_sub(reduction);
        changed |= wound.bleed_rate_per_tick != previous;
    }

    for wound in &mut next.wounds {
        if severity_budget.value() == 0 {
            break;
        }
        let previous = wound.severity;
        let reduction = severity_budget.min(wound.severity);
        wound.severity = wound.severity.saturating_sub(reduction);
        severity_budget = severity_budget.saturating_sub(reduction);
        changed |= wound.severity != previous;
    }

    let previous_len = next.wounds.len();
    next.wounds.retain(|wound| wound.severity.value() > 0);
    changed |= previous_len != next.wounds.len();

    changed.then_some(next)
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct AttackWeaponStats {
    severity: Permille,
    bleed_rate: Permille,
}

fn weapon_stats(
    profile: worldwake_core::CombatProfile,
    weapon: CombatWeaponRef,
) -> Result<AttackWeaponStats, ActionError> {
    match weapon {
        CombatWeaponRef::Unarmed => Ok(AttackWeaponStats {
            severity: profile.unarmed_wound_severity,
            bleed_rate: profile.unarmed_bleed_rate,
        }),
        CombatWeaponRef::Commodity(kind) => {
            let CombatWeaponProfile {
                base_wound_severity,
                base_bleed_rate,
                ..
            } = kind.spec().combat_weapon_profile.ok_or({
                ActionError::AbortRequested(ActionAbortRequestReason::CommodityNotCombatWeapon {
                    commodity: kind,
                })
            })?;
            Ok(AttackWeaponStats {
                severity: base_wound_severity,
                bleed_rate: base_bleed_rate,
            })
        }
    }
}

fn wound_penalty(wounds: Option<&WoundList>) -> Permille {
    let penalty = wounds
        .map_or(0, |wounds| wounds.wound_load().min(u32::from(u16::MAX)))
        .min(1000) as u16;
    Permille::new(penalty).unwrap()
}

fn fatigue_penalty(needs: Option<&HomeostaticNeeds>) -> Permille {
    needs.map_or(Permille::new_unchecked(0), |needs| needs.fatigue)
}

fn effective_attack_skill(
    profile: worldwake_core::CombatProfile,
    needs: Option<&HomeostaticNeeds>,
    wounds: Option<&WoundList>,
) -> Permille {
    profile
        .attack_skill
        .saturating_sub(fatigue_penalty(needs))
        .saturating_sub(wound_penalty(wounds))
}

fn effective_guard_skill(
    profile: worldwake_core::CombatProfile,
    stance: Option<CombatStance>,
    needs: Option<&HomeostaticNeeds>,
    wounds: Option<&WoundList>,
) -> Permille {
    let boosted = if stance == Some(CombatStance::Defending) {
        profile.guard_skill.saturating_add(profile.defend_bonus)
    } else {
        profile.guard_skill
    };
    boosted
        .saturating_sub(fatigue_penalty(needs))
        .saturating_sub(wound_penalty(wounds))
}

#[derive(Clone, Copy)]
struct AttackResolutionActor<'a> {
    entity: worldwake_core::EntityId,
    profile: worldwake_core::CombatProfile,
    needs: Option<&'a HomeostaticNeeds>,
    wounds: Option<&'a WoundList>,
}

#[derive(Clone, Copy)]
struct AttackResolutionTarget<'a> {
    profile: worldwake_core::CombatProfile,
    stance: Option<CombatStance>,
    needs: Option<&'a HomeostaticNeeds>,
    wounds: &'a WoundList,
}

#[derive(Clone, Copy)]
struct AttackResolutionContext {
    weapon: CombatWeaponRef,
    tick: worldwake_core::Tick,
}

fn resolve_attack_wound(
    attacker: AttackResolutionActor<'_>,
    target: AttackResolutionTarget<'_>,
    context: AttackResolutionContext,
    rng: &mut DeterministicRng,
) -> Result<Option<Wound>, ActionError> {
    let attack_skill = effective_attack_skill(attacker.profile, attacker.needs, attacker.wounds);
    let guard_skill = effective_guard_skill(
        target.profile,
        target.stance,
        target.needs,
        Some(target.wounds),
    );
    let attack_roll = rng.next_range(0, u32::from(attack_skill.value()) + 1);
    let guard_roll = rng.next_range(0, u32::from(guard_skill.value()) + 1);
    if attack_roll < guard_roll {
        return Ok(None);
    }

    let body_part = BODY_PARTS[rng.next_range(0, BODY_PARTS.len() as u32) as usize];
    let weapon_stats = weapon_stats(attacker.profile, context.weapon)?;

    Ok(Some(Wound {
        id: target.wounds.next_wound_id(),
        body_part,
        cause: WoundCause::Combat {
            attacker: attacker.entity,
            weapon: context.weapon,
        },
        severity: weapon_stats.severity,
        inflicted_at: context.tick,
        bleed_rate_per_tick: weapon_stats.bleed_rate,
    }))
}

fn start_attack(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let payload = combat_payload(def, instance)?;
    let _ = validate_attack_context(txn, instance, payload)?;
    validate_selected_weapon(txn, instance.actor, payload.weapon)?;
    Ok(None)
}

#[allow(clippy::unnecessary_wraps)]
fn start_heal(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let _ = validate_heal_context(txn, instance)?;
    let _ = consume_one_unit_of_commodity(
        txn,
        instance.actor,
        worldwake_core::CommodityKind::Medicine,
    )?;
    Ok(None)
}

fn direct_loot_entities(txn: &WorldTxn<'_>, corpse: EntityId) -> Vec<EntityId> {
    let mut entities = txn.possessions_of(corpse);
    entities.sort();
    entities.dedup();
    entities
}

fn transferable_loot_entity(
    txn: &mut WorldTxn<'_>,
    looter: EntityId,
    corpse: EntityId,
    entity: EntityId,
    place: EntityId,
) -> Result<Option<EntityId>, ActionError> {
    let remaining = remaining_capacity(txn, looter)?.0;
    let subtree_load = controlled_entity_load(txn, entity)?.0;
    if subtree_load <= remaining {
        move_entity_to_direct_possession(txn, entity, looter, place)?;
        return Ok(Some(entity));
    }

    let Some(lot) = txn.get_component_item_lot(entity).cloned() else {
        return Ok(None);
    };
    let per_unit = load_per_unit(lot.commodity).0;
    if per_unit == 0 || remaining < per_unit {
        return Ok(None);
    }

    let max_quantity = remaining / per_unit;
    if max_quantity == 0 || max_quantity >= lot.quantity.0 {
        return Ok(None);
    }

    let (_, split_off) = txn
        .split_lot(entity, Quantity(max_quantity))
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    if let Some(owner) = txn.owner_of(entity) {
        txn.set_owner(split_off, owner)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    txn.set_ground_location(split_off, place)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    move_entity_to_direct_possession(txn, split_off, looter, place)?;

    if txn.possessor_of(entity) != Some(corpse) {
        return Err(ActionError::InternalError(format!(
            "corpse {corpse} lost possession of source lot {entity} during loot split"
        )));
    }

    Ok(Some(split_off))
}

#[allow(clippy::unnecessary_wraps)]
fn start_loot(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let _ = validate_loot_context(txn, instance)?;
    let _ = remaining_capacity(txn, instance.actor)?;
    Ok(None)
}

#[allow(clippy::unnecessary_wraps)]
fn tick_loot(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn tick_heal(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    let target = validate_heal_context(txn, instance)?;
    let wounds =
        txn.get_component_wound_list(target)
            .cloned()
            .ok_or(ActionError::AbortRequested(
                ActionAbortRequestReason::TargetLacksWounds { target },
            ))?;
    let profile = worldwake_core::CommodityKind::Medicine
        .spec()
        .treatment_profile
        .ok_or_else(|| {
            ActionError::InternalError("medicine lacks treatment profile".to_string())
        })?;

    if let Some(next) = apply_treatment(&wounds, profile) {
        let completed = next.wounds.is_empty();
        txn.set_component_wound_list(target, next)
            .map_err(|error| ActionError::InternalError(error.to_string()))?;
        return Ok(if completed {
            ActionProgress::Complete
        } else {
            ActionProgress::Continue
        });
    }

    Ok(ActionProgress::Complete)
}

fn commit_loot(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let (corpse, place) = validate_loot_context(txn, instance)?;
    for entity in direct_loot_entities(txn, corpse) {
        let _ = transferable_loot_entity(txn, instance.actor, corpse, entity, place)?;
    }
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn tick_attack(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_attack(
    def: &ActionDef,
    instance: &ActionInstance,
    rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = combat_payload(def, instance)?;
    let target = validate_attack_context(txn, instance, payload)?;
    validate_selected_weapon(txn, instance.actor, payload.weapon)?;

    let attacker_profile = txn
        .get_component_combat_profile(instance.actor)
        .copied()
        .ok_or(ActionError::AbortRequested(
            ActionAbortRequestReason::ActorMissingCombatProfile {
                actor: instance.actor,
            },
        ))?;
    let target_profile =
        txn.get_component_combat_profile(target)
            .copied()
            .ok_or(ActionError::AbortRequested(
                ActionAbortRequestReason::TargetMissingCombatProfile { target },
            ))?;
    let attacker_needs = txn.get_component_homeostatic_needs(instance.actor);
    let target_needs = txn.get_component_homeostatic_needs(target);
    let attacker_wounds = txn.get_component_wound_list(instance.actor);
    let target_wounds = txn
        .get_component_wound_list(target)
        .cloned()
        .unwrap_or_default();
    let target_stance = txn.get_component_combat_stance(target).copied();

    let Some(wound) = resolve_attack_wound(
        AttackResolutionActor {
            entity: instance.actor,
            profile: attacker_profile,
            needs: attacker_needs,
            wounds: attacker_wounds,
        },
        AttackResolutionTarget {
            profile: target_profile,
            stance: target_stance,
            needs: target_needs,
            wounds: &target_wounds,
        },
        AttackResolutionContext {
            weapon: payload.weapon,
            tick: txn.tick(),
        },
        rng,
    )?
    else {
        return Ok(CommitOutcome::empty());
    };

    let mut next_wounds = target_wounds;
    next_wounds.wounds.push(wound);
    txn.set_component_wound_list(target, next_wounds)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_attack(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _reason: &AbortReason,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_loot(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _reason: &AbortReason,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn commit_heal(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_heal(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _reason: &AbortReason,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        combat_system, effective_guard_skill, register_attack_action, register_defend_action,
        register_heal_action, register_loot_action, resolve_attack_wound, AttackResolutionActor,
        AttackResolutionContext, AttackResolutionTarget,
    };
    use crate::dispatch_table;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, BodyPart, CarryCapacity, CauseRef, CombatProfile, CombatStance,
        CombatWeaponRef, CommodityKind, Container, ControlSource, DeadAt, DeprivationKind,
        DriveThresholds, EventLog, EventTag, EvidenceRef, HomeostaticNeeds, LoadUnits, Permille,
        Quantity, Seed, Tick, VisibilitySpec, WitnessData, World, WorldTxn, Wound, WoundCause,
        WoundId, WoundList,
    };
    use worldwake_sim::{
        abort_action, get_affordances, start_action, tick_action, ActionDuration, ActionError,
        ActionExecutionAuthority, ActionExecutionContext, ActionHandlerRegistry, ActionInstanceId,
        ActionPayload, ActionStatus, Affordance, CombatActionPayload, DeterministicRng,
        DurationExpr, Interruptibility, OmniscientBeliefView, SystemExecutionContext, SystemId,
        TickOutcome,
    };

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
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

    fn test_rng(seed: u8) -> DeterministicRng {
        DeterministicRng::new(Seed([seed; 32]))
    }

    fn defend_profile() -> CombatProfile {
        CombatProfile::new(
            pm(1000),
            pm(700),
            pm(600),
            pm(550),
            pm(75),
            pm(20),
            pm(15),
            pm(120),
            pm(30),
            nz(6),
        )
    }

    fn spawn_guard(
        world: &mut World,
        tick: u64,
        control: ControlSource,
    ) -> worldwake_core::EntityId {
        let place = world.topology().place_ids().next().unwrap();
        let actor = {
            let mut txn = new_txn(world, tick);
            let actor = txn.create_agent("Guard", control).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_component_combat_profile(actor, defend_profile())
                .unwrap();
            txn.set_component_wound_list(actor, WoundList::default())
                .unwrap();
            commit_txn(txn);
            actor
        };
        actor
    }

    fn spawn_guard_with_profile(
        world: &mut World,
        tick: u64,
        control: ControlSource,
        profile: CombatProfile,
    ) -> worldwake_core::EntityId {
        let place = world.topology().place_ids().next().unwrap();
        let actor = {
            let mut txn = new_txn(world, tick);
            let actor = txn.create_agent("Guard", control).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_component_combat_profile(actor, profile).unwrap();
            txn.set_component_wound_list(actor, WoundList::default())
                .unwrap();
            commit_txn(txn);
            actor
        };
        actor
    }

    fn arm_actor(
        world: &mut World,
        actor: worldwake_core::EntityId,
        tick: u64,
        kind: CommodityKind,
        quantity: u32,
    ) {
        let place = world.effective_place(actor).unwrap();
        let mut txn = new_txn(world, tick);
        let lot = txn.create_item_lot(kind, Quantity(quantity)).unwrap();
        txn.set_ground_location(lot, place).unwrap();
        txn.set_possessor(lot, actor).unwrap();
        commit_txn(txn);
    }

    fn set_carry_capacity(
        world: &mut World,
        actor: worldwake_core::EntityId,
        tick: u64,
        load: u32,
    ) {
        let mut txn = new_txn(world, tick);
        txn.set_component_carry_capacity(actor, CarryCapacity(LoadUnits(load)))
            .unwrap();
        commit_txn(txn);
    }

    fn add_carried_lot(
        world: &mut World,
        actor: worldwake_core::EntityId,
        tick: u64,
        commodity: CommodityKind,
        quantity: u32,
    ) -> worldwake_core::EntityId {
        let place = world.effective_place(actor).unwrap();
        let mut txn = new_txn(world, tick);
        let lot = txn.create_item_lot(commodity, Quantity(quantity)).unwrap();
        txn.set_ground_location(lot, place).unwrap();
        txn.set_possessor(lot, actor).unwrap();
        commit_txn(txn);
        lot
    }

    fn add_carried_container_with_lot(
        world: &mut World,
        actor: worldwake_core::EntityId,
        tick: u64,
        commodity: CommodityKind,
        quantity: u32,
    ) -> (worldwake_core::EntityId, worldwake_core::EntityId) {
        let place = world.effective_place(actor).unwrap();
        let mut txn = new_txn(world, tick);
        let container = txn
            .create_container(Container {
                capacity: LoadUnits(20),
                allowed_commodities: None,
                allows_unique_items: true,
                allows_nested_containers: true,
            })
            .unwrap();
        let lot = txn.create_item_lot(commodity, Quantity(quantity)).unwrap();
        txn.set_ground_location(container, place).unwrap();
        txn.set_possessor(container, actor).unwrap();
        txn.put_into_container(lot, container).unwrap();
        commit_txn(txn);
        (container, lot)
    }

    fn set_recovery_state(
        world: &mut World,
        entity: worldwake_core::EntityId,
        tick: u64,
        needs: HomeostaticNeeds,
    ) {
        let mut txn = new_txn(world, tick);
        txn.set_component_homeostatic_needs(entity, needs).unwrap();
        txn.set_component_drive_thresholds(entity, DriveThresholds::default())
            .unwrap();
        commit_txn(txn);
    }

    fn deprivation_wound(id: u64, severity: u16, bleed_rate: u16, inflicted_at: u64) -> Wound {
        Wound {
            id: WoundId(id),
            body_part: BodyPart::Torso,
            cause: WoundCause::Deprivation(DeprivationKind::Starvation),
            severity: pm(severity),
            inflicted_at: Tick(inflicted_at),
            bleed_rate_per_tick: pm(bleed_rate),
        }
    }

    #[test]
    fn register_defend_action_creates_indefinite_public_defend_definition() {
        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let defend_id = register_defend_action(&mut defs, &mut handlers);
        let defend = defs.get(defend_id).unwrap();

        assert_eq!(defend.name, "defend");
        assert_eq!(defend.duration, DurationExpr::Indefinite);
        assert_eq!(
            defend.interruptibility,
            Interruptibility::FreelyInterruptible
        );
        assert_eq!(defend.visibility, VisibilitySpec::SamePlace);
        assert_eq!(defend.payload, ActionPayload::None);
        assert!(defend.targets.is_empty());
    }

    #[test]
    fn register_attack_action_creates_public_combat_definition() {
        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let attack_id = register_attack_action(&mut defs, &mut handlers);
        let attack = defs.get(attack_id).unwrap();

        assert_eq!(attack.name, "attack");
        assert_eq!(attack.duration, DurationExpr::CombatWeapon);
        assert_eq!(
            attack.interruptibility,
            Interruptibility::FreelyInterruptible
        );
        assert_eq!(attack.visibility, VisibilitySpec::SamePlace);
        assert_eq!(attack.payload, ActionPayload::None);
        assert_eq!(attack.targets.len(), 1);
        assert!(attack
            .preconditions
            .contains(&worldwake_sim::Precondition::TargetAlive(0)));
        assert!(attack
            .actor_constraints
            .contains(&worldwake_sim::Constraint::ActorNotIncapacitated));
    }

    #[test]
    fn register_loot_action_creates_public_loot_definition() {
        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let loot_id = register_loot_action(&mut defs, &mut handlers);
        let loot = defs.get(loot_id).unwrap();

        assert_eq!(loot.name, "loot");
        assert_eq!(loot.domain, worldwake_sim::ActionDomain::Loot);
        assert_eq!(loot.duration, DurationExpr::Fixed(NonZeroU32::MIN));
        assert_eq!(loot.interruptibility, Interruptibility::FreelyInterruptible);
        assert_eq!(loot.visibility, VisibilitySpec::SamePlace);
        assert_eq!(loot.payload, ActionPayload::None);
        assert!(loot
            .actor_constraints
            .contains(&worldwake_sim::Constraint::ActorNotIncapacitated));
        assert!(loot
            .preconditions
            .contains(&worldwake_sim::Precondition::TargetDead(0)));
        assert!(loot
            .preconditions
            .contains(&worldwake_sim::Precondition::TargetIsAgent(0)));
        assert!(loot.causal_event_tags.contains(&EventTag::Inventory));
        assert!(loot.causal_event_tags.contains(&EventTag::Transfer));
    }

    #[test]
    fn register_heal_action_creates_public_care_definition() {
        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let heal_id = register_heal_action(&mut defs, &mut handlers);
        let heal = defs.get(heal_id).unwrap();

        assert_eq!(heal.name, "heal");
        assert_eq!(heal.domain, worldwake_sim::ActionDomain::Care);
        assert_eq!(
            heal.duration,
            DurationExpr::TargetTreatment {
                target_index: 0,
                commodity: CommodityKind::Medicine,
            }
        );
        assert_eq!(
            heal.interruptibility,
            Interruptibility::InterruptibleWithPenalty
        );
        assert_eq!(heal.visibility, VisibilitySpec::SamePlace);
        assert_eq!(heal.payload, ActionPayload::None);
        assert!(heal
            .actor_constraints
            .contains(&worldwake_sim::Constraint::ActorHasCommodity {
                kind: CommodityKind::Medicine,
                min_qty: Quantity(1),
            }));
        assert!(heal
            .preconditions
            .contains(&worldwake_sim::Precondition::TargetHasWounds(0)));
    }

    #[test]
    fn heal_affordance_only_targets_wounded_agents() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let healer = spawn_guard(&mut world, 1, ControlSource::Ai);
        let wounded = spawn_guard(&mut world, 2, ControlSource::Ai);
        let healthy = spawn_guard(&mut world, 3, ControlSource::Ai);
        arm_actor(&mut world, healer, 4, CommodityKind::Medicine, 1);
        {
            let mut txn = new_txn(&mut world, 5);
            txn.set_component_wound_list(
                wounded,
                WoundList {
                    wounds: vec![deprivation_wound(1, 240, 30, 5)],
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let heal_id = register_heal_action(&mut defs, &mut handlers);

        let affordances =
            get_affordances(&OmniscientBeliefView::new(&world), healer, &defs, &handlers);
        let heal_targets = affordances
            .into_iter()
            .filter(|affordance| affordance.def_id == heal_id)
            .map(|affordance| affordance.bound_targets)
            .collect::<Vec<_>>();

        assert_eq!(heal_targets, vec![vec![wounded]]);
        assert!(!heal_targets.contains(&vec![healthy]));
    }

    #[test]
    fn heal_lifecycle_consumes_medicine_and_reduces_bleeding_and_severity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let healer = spawn_guard(&mut world, 1, ControlSource::Ai);
        let patient = spawn_guard(&mut world, 2, ControlSource::Ai);
        arm_actor(&mut world, healer, 3, CommodityKind::Medicine, 1);
        {
            let mut txn = new_txn(&mut world, 4);
            txn.set_component_wound_list(
                patient,
                WoundList {
                    wounds: vec![deprivation_wound(1, 360, 90, 4)],
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let heal_id = register_heal_action(&mut defs, &mut handlers);
        let affordance =
            get_affordances(&OmniscientBeliefView::new(&world), healer, &defs, &handlers)
                .into_iter()
                .find(|affordance| {
                    affordance.def_id == heal_id && affordance.bound_targets == vec![patient]
                })
                .unwrap();
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = test_rng(0x31);

        let action_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(10),
            },
        )
        .unwrap();

        assert_eq!(
            world.controlled_commodity_quantity(healer, CommodityKind::Medicine),
            Quantity(0)
        );

        let outcome = tick_action(
            action_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
        )
        .unwrap();

        assert_eq!(outcome, TickOutcome::Continuing);
        let wound = &world.get_component_wound_list(patient).unwrap().wounds[0];
        assert_eq!(wound.bleed_rate_per_tick, pm(30));
        assert_eq!(wound.severity, pm(240));
        let mut final_outcome = outcome;
        for tick in 12..15 {
            final_outcome = tick_action(
                action_id,
                &defs,
                &handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active,
                    world: &mut world,
                    event_log: &mut log,
                    rng: &mut rng,
                },
                ActionExecutionContext {
                    cause: CauseRef::Bootstrap,
                    tick: Tick(tick),
                },
            )
            .unwrap();
            if matches!(final_outcome, TickOutcome::Committed { .. }) {
                break;
            }
        }

        assert!(matches!(final_outcome, TickOutcome::Committed { .. }));
        assert!(log
            .events_by_tag(EventTag::Inventory)
            .iter()
            .any(|event_id| log.get(*event_id).is_some()));
    }

    #[test]
    fn heal_removes_fully_healed_wounds() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let healer = spawn_guard(&mut world, 1, ControlSource::Ai);
        let patient = spawn_guard(&mut world, 2, ControlSource::Ai);
        arm_actor(&mut world, healer, 3, CommodityKind::Medicine, 1);
        {
            let mut txn = new_txn(&mut world, 4);
            txn.set_component_wound_list(
                patient,
                WoundList {
                    wounds: vec![deprivation_wound(1, 100, 0, 4)],
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let heal_id = register_heal_action(&mut defs, &mut handlers);
        let affordance =
            get_affordances(&OmniscientBeliefView::new(&world), healer, &defs, &handlers)
                .into_iter()
                .find(|affordance| {
                    affordance.def_id == heal_id && affordance.bound_targets == vec![patient]
                })
                .unwrap();
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = test_rng(0x33);

        let action_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(10),
            },
        )
        .unwrap();

        let outcome = tick_action(
            action_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(
            world.get_component_wound_list(patient),
            Some(&WoundList::default())
        );
    }

    #[test]
    fn heal_requires_medicine_and_living_same_place_wounded_target() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let healer = spawn_guard(&mut world, 1, ControlSource::Ai);
        let patient = spawn_guard(&mut world, 2, ControlSource::Ai);
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_wound_list(
                patient,
                WoundList {
                    wounds: vec![deprivation_wound(1, 180, 20, 3)],
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let heal_id = register_heal_action(&mut defs, &mut handlers);

        let affordances =
            get_affordances(&OmniscientBeliefView::new(&world), healer, &defs, &handlers);
        assert!(!affordances
            .iter()
            .any(|affordance| affordance.def_id == heal_id));

        arm_actor(&mut world, healer, 4, CommodityKind::Medicine, 1);
        {
            let mut txn = new_txn(&mut world, 5);
            txn.set_component_dead_at(patient, DeadAt(Tick(5))).unwrap();
            commit_txn(txn);
        }
        let affordances =
            get_affordances(&OmniscientBeliefView::new(&world), healer, &defs, &handlers);
        assert!(!affordances
            .iter()
            .any(|affordance| affordance.def_id == heal_id));
    }

    #[test]
    fn loot_transfers_corpse_possessions_and_emits_public_inventory_event() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let looter = spawn_guard(&mut world, 1, ControlSource::Ai);
        let corpse = spawn_guard(&mut world, 2, ControlSource::Ai);
        set_carry_capacity(&mut world, looter, 3, 10);
        let bread = add_carried_lot(&mut world, corpse, 4, CommodityKind::Bread, 3);
        {
            let mut txn = new_txn(&mut world, 5);
            txn.set_component_dead_at(corpse, DeadAt(Tick(5))).unwrap();
            commit_txn(txn);
        }

        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let loot_id = register_loot_action(&mut defs, &mut handlers);
        let affordance =
            get_affordances(&OmniscientBeliefView::new(&world), looter, &defs, &handlers)
                .into_iter()
                .find(|affordance| {
                    affordance.def_id == loot_id && affordance.bound_targets == vec![corpse]
                })
                .unwrap();
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = test_rng(0x21);

        let action_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(6),
            },
        )
        .unwrap();

        let outcome = tick_action(
            action_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(7),
            },
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(world.possessor_of(bread), Some(looter));
        assert_eq!(
            world.controlled_commodity_quantity(looter, CommodityKind::Bread),
            Quantity(3)
        );
        assert_eq!(
            world.controlled_commodity_quantity(corpse, CommodityKind::Bread),
            Quantity(0)
        );
        assert_eq!(world.get_component_dead_at(corpse), Some(&DeadAt(Tick(5))));

        let record = log
            .get(*log.events_by_tag(EventTag::ActionCommitted).last().unwrap())
            .unwrap();
        assert_eq!(record.visibility, VisibilitySpec::SamePlace);
        assert!(record.tags.contains(&EventTag::Inventory));
        assert!(record.tags.contains(&EventTag::Transfer));
        assert!(record.target_ids.contains(&bread));
    }

    #[test]
    fn loot_splits_lot_when_only_partial_quantity_fits() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let looter = spawn_guard(&mut world, 1, ControlSource::Ai);
        let corpse = spawn_guard(&mut world, 2, ControlSource::Ai);
        set_carry_capacity(&mut world, looter, 3, 2);
        let bread = add_carried_lot(&mut world, corpse, 4, CommodityKind::Bread, 3);
        {
            let mut txn = new_txn(&mut world, 5);
            txn.set_component_dead_at(corpse, DeadAt(Tick(5))).unwrap();
            commit_txn(txn);
        }

        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let loot_id = register_loot_action(&mut defs, &mut handlers);
        let affordance =
            get_affordances(&OmniscientBeliefView::new(&world), looter, &defs, &handlers)
                .into_iter()
                .find(|affordance| {
                    affordance.def_id == loot_id && affordance.bound_targets == vec![corpse]
                })
                .unwrap();
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = test_rng(0x22);

        let action_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(6),
            },
        )
        .unwrap();

        let outcome = tick_action(
            action_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(7),
            },
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(
            world.get_component_item_lot(bread).unwrap().quantity,
            Quantity(1)
        );
        assert_eq!(world.possessor_of(bread), Some(corpse));
        assert_eq!(
            world.controlled_commodity_quantity(looter, CommodityKind::Bread),
            Quantity(2)
        );
        assert_eq!(
            world.controlled_commodity_quantity(corpse, CommodityKind::Bread),
            Quantity(1)
        );

        let transferred = world
            .possessions_of(looter)
            .into_iter()
            .find(|entity| *entity != bread)
            .unwrap();
        assert_eq!(
            world.get_component_item_lot(transferred).unwrap().quantity,
            Quantity(2)
        );
        assert_eq!(
            world.effective_place(transferred),
            world.effective_place(looter)
        );
    }

    #[test]
    fn loot_transfers_possessed_container_with_nested_contents() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let looter = spawn_guard(&mut world, 1, ControlSource::Ai);
        let corpse = spawn_guard(&mut world, 2, ControlSource::Ai);
        set_carry_capacity(&mut world, looter, 3, 10);
        let (satchel, bread) =
            add_carried_container_with_lot(&mut world, corpse, 4, CommodityKind::Bread, 2);
        {
            let mut txn = new_txn(&mut world, 5);
            txn.set_component_dead_at(corpse, DeadAt(Tick(5))).unwrap();
            commit_txn(txn);
        }

        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let loot_id = register_loot_action(&mut defs, &mut handlers);
        let affordance =
            get_affordances(&OmniscientBeliefView::new(&world), looter, &defs, &handlers)
                .into_iter()
                .find(|affordance| {
                    affordance.def_id == loot_id && affordance.bound_targets == vec![corpse]
                })
                .unwrap();
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = test_rng(0x23);

        let action_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(6),
            },
        )
        .unwrap();
        let outcome = tick_action(
            action_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(7),
            },
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(world.possessor_of(satchel), Some(looter));
        assert_eq!(world.direct_container(bread), Some(satchel));
        assert_eq!(
            world.controlled_commodity_quantity(looter, CommodityKind::Bread),
            Quantity(2)
        );
        assert_eq!(
            world.controlled_commodity_quantity(corpse, CommodityKind::Bread),
            Quantity(0)
        );
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn loot_requires_dead_colocated_target_and_capable_looter() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let looter = spawn_guard(&mut world, 1, ControlSource::Ai);
        let incapacitated_looter = spawn_guard(&mut world, 2, ControlSource::Ai);
        let alive_target = spawn_guard(&mut world, 3, ControlSource::Ai);
        let dead_target = spawn_guard(&mut world, 4, ControlSource::Ai);
        let place = world.effective_place(looter).unwrap();
        let other_place = world.topology().place_ids().nth(1).unwrap();
        set_carry_capacity(&mut world, looter, 5, 10);
        set_carry_capacity(&mut world, incapacitated_looter, 6, 10);
        {
            let mut txn = new_txn(&mut world, 7);
            txn.set_component_dead_at(dead_target, DeadAt(Tick(7)))
                .unwrap();
            txn.set_ground_location(incapacitated_looter, place)
                .unwrap();
            txn.set_component_wound_list(
                incapacitated_looter,
                WoundList {
                    wounds: vec![deprivation_wound(77, 700, 0, 7)],
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let loot_id = register_loot_action(&mut defs, &mut handlers);

        let alive_affordances =
            get_affordances(&OmniscientBeliefView::new(&world), looter, &defs, &handlers);
        assert!(!alive_affordances.iter().any(|affordance| {
            affordance.def_id == loot_id && affordance.bound_targets == vec![alive_target]
        }));

        {
            let mut txn = new_txn(&mut world, 8);
            txn.set_ground_location(dead_target, other_place).unwrap();
            commit_txn(txn);
        }
        let moved_affordance = Affordance {
            def_id: loot_id,
            actor: looter,
            bound_targets: vec![dead_target],
            payload_override: None,
            explanation: None,
        };
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = test_rng(0x24);
        let moved_err = start_action(
            &moved_affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(9),
            },
        )
        .unwrap_err();
        assert_eq!(
            moved_err,
            ActionError::PreconditionFailed("TargetAtActorPlace(0)".to_string())
        );

        {
            let mut txn = new_txn(&mut world, 10);
            txn.set_ground_location(dead_target, place).unwrap();
            txn.set_component_dead_at(looter, DeadAt(Tick(10))).unwrap();
            commit_txn(txn);
        }
        let dead_actor_err = start_action(
            &Affordance {
                def_id: loot_id,
                actor: looter,
                bound_targets: vec![dead_target],
                payload_override: None,
                explanation: None,
            },
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
        )
        .unwrap_err();
        assert_eq!(
            dead_actor_err,
            ActionError::ConstraintFailed("ActorNotDead".to_string())
        );

        let incapacitated_err = start_action(
            &Affordance {
                def_id: loot_id,
                actor: incapacitated_looter,
                bound_targets: vec![dead_target],
                payload_override: None,
                explanation: None,
            },
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(12),
            },
        )
        .unwrap_err();
        assert_eq!(
            incapacitated_err,
            ActionError::ConstraintFailed("ActorNotIncapacitated".to_string())
        );
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn attack_lifecycle_applies_wound_and_emits_same_place_combat_event() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let attacker = spawn_guard_with_profile(
            &mut world,
            1,
            ControlSource::Ai,
            CombatProfile::new(
                pm(1000),
                pm(700),
                pm(1000),
                pm(0),
                pm(0),
                pm(0),
                pm(0),
                pm(120),
                pm(30),
                nz(6),
            ),
        );
        let target = spawn_guard_with_profile(
            &mut world,
            2,
            ControlSource::Ai,
            CombatProfile::new(
                pm(1000),
                pm(700),
                pm(0),
                pm(0),
                pm(0),
                pm(0),
                pm(0),
                pm(80),
                pm(10),
                nz(6),
            ),
        );
        arm_actor(&mut world, attacker, 3, CommodityKind::Sword, 1);

        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let attack_id = register_attack_action(&mut defs, &mut handlers);
        let affordance = get_affordances(
            &OmniscientBeliefView::new(&world),
            attacker,
            &defs,
            &handlers,
        )
        .into_iter()
        .find(|affordance| {
            affordance.def_id == attack_id && affordance.bound_targets == vec![target]
        })
        .unwrap();
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = test_rng(0);

        let action_id = start_action(
            &Affordance {
                payload_override: Some(ActionPayload::Combat(CombatActionPayload {
                    target,
                    weapon: CombatWeaponRef::Commodity(CommodityKind::Sword),
                })),
                ..affordance
            },
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(5),
            },
        )
        .unwrap();

        assert_eq!(
            active.get(&action_id).unwrap().remaining_duration,
            ActionDuration::Finite(4)
        );

        let mut outcome = TickOutcome::Continuing;
        for tick in 6..=10 {
            outcome = tick_action(
                action_id,
                &defs,
                &handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active,
                    world: &mut world,
                    event_log: &mut log,
                    rng: &mut rng,
                },
                ActionExecutionContext {
                    cause: CauseRef::Bootstrap,
                    tick: Tick(tick),
                },
            )
            .unwrap();
            if outcome != TickOutcome::Continuing {
                break;
            }
        }

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        let wounds = world.get_component_wound_list(target).unwrap();
        assert_eq!(wounds.wounds.len(), 1);
        assert_eq!(
            wounds.wounds[0].cause,
            WoundCause::Combat {
                attacker,
                weapon: CombatWeaponRef::Commodity(CommodityKind::Sword),
            }
        );
        assert_eq!(wounds.wounds[0].inflicted_at, Tick(9));

        let record = log
            .get(*log.events_by_tag(EventTag::ActionCommitted).last().unwrap())
            .unwrap();
        assert_eq!(record.visibility, VisibilitySpec::SamePlace);
        assert!(record.tags.contains(&EventTag::Combat));
        assert!(record.tags.contains(&EventTag::WorldMutation));
        assert!(record.target_ids.contains(&target));
    }

    #[test]
    fn defend_affordance_starts_and_stays_active_until_cancelled() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let actor = spawn_guard(&mut world, 1, ControlSource::Ai);
        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let defend_id = register_defend_action(&mut defs, &mut handlers);
        let view = OmniscientBeliefView::new(&world);
        let affordance = get_affordances(&view, actor, &defs, &handlers)
            .into_iter()
            .find(|affordance| affordance.def_id == defend_id)
            .unwrap();
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = test_rng(0x11);

        let action_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(5),
            },
        )
        .unwrap();

        assert_eq!(
            active.get(&action_id).unwrap().remaining_duration,
            ActionDuration::Indefinite
        );
        assert_eq!(active.get(&action_id).unwrap().status, ActionStatus::Active);
        assert_eq!(
            world.get_component_combat_stance(actor),
            Some(&CombatStance::Defending)
        );

        let outcome = tick_action(
            action_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(6),
            },
        )
        .unwrap();

        assert_eq!(outcome, TickOutcome::Continuing);
        assert_eq!(
            active.get(&action_id).unwrap().remaining_duration,
            ActionDuration::Indefinite
        );

        let replan = abort_action(
            action_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(7),
            },
            worldwake_sim::ExternalAbortReason::Other,
        )
        .unwrap();

        assert_eq!(replan.agent, actor);
        assert!(!active.contains_key(&action_id));
        assert_eq!(world.get_component_combat_stance(actor), None);
    }

    #[test]
    fn effective_guard_skill_applies_defend_bonus_through_authoritative_stance() {
        let profile = CombatProfile::new(
            pm(1000),
            pm(700),
            pm(500),
            pm(200),
            pm(300),
            pm(0),
            pm(0),
            pm(50),
            pm(10),
            nz(1),
        );

        assert_eq!(effective_guard_skill(profile, None, None, None), pm(200));
        assert_eq!(
            effective_guard_skill(profile, Some(CombatStance::Defending), None, None),
            pm(500)
        );
    }

    #[test]
    fn resolve_attack_wound_changes_outcome_when_target_is_defending() {
        let attacker_profile = CombatProfile::new(
            pm(1000),
            pm(700),
            pm(500),
            pm(0),
            pm(0),
            pm(0),
            pm(0),
            pm(120),
            pm(30),
            nz(1),
        );
        let defending_target_profile = CombatProfile::new(
            pm(1000),
            pm(700),
            pm(0),
            pm(0),
            pm(1000),
            pm(0),
            pm(0),
            pm(80),
            pm(10),
            nz(1),
        );
        let wounds = WoundList::default();

        let mut undefended_rng = DeterministicRng::new(Seed([3; 32]));
        let undefended = resolve_attack_wound(
            AttackResolutionActor {
                entity: worldwake_core::EntityId {
                    slot: 1,
                    generation: 1,
                },
                profile: attacker_profile,
                needs: None,
                wounds: None,
            },
            AttackResolutionTarget {
                profile: defending_target_profile,
                stance: None,
                needs: None,
                wounds: &wounds,
            },
            AttackResolutionContext {
                weapon: CombatWeaponRef::Unarmed,
                tick: Tick(9),
            },
            &mut undefended_rng,
        )
        .unwrap();

        let mut defending_rng = DeterministicRng::new(Seed([3; 32]));
        let defending = resolve_attack_wound(
            AttackResolutionActor {
                entity: worldwake_core::EntityId {
                    slot: 1,
                    generation: 1,
                },
                profile: attacker_profile,
                needs: None,
                wounds: None,
            },
            AttackResolutionTarget {
                profile: defending_target_profile,
                stance: Some(CombatStance::Defending),
                needs: None,
                wounds: &wounds,
            },
            AttackResolutionContext {
                weapon: CombatWeaponRef::Unarmed,
                tick: Tick(9),
            },
            &mut defending_rng,
        )
        .unwrap();

        assert!(undefended.is_some());
        assert_eq!(defending, None);
    }

    #[test]
    fn resolve_attack_wound_is_deterministic_for_same_seed_and_inputs() {
        let profile = defend_profile();
        let wounds = WoundList::default();

        let mut left_rng = DeterministicRng::new(Seed([19; 32]));
        let left = resolve_attack_wound(
            AttackResolutionActor {
                entity: worldwake_core::EntityId {
                    slot: 1,
                    generation: 1,
                },
                profile,
                needs: None,
                wounds: None,
            },
            AttackResolutionTarget {
                profile,
                stance: None,
                needs: None,
                wounds: &wounds,
            },
            AttackResolutionContext {
                weapon: CombatWeaponRef::Commodity(CommodityKind::Sword),
                tick: Tick(11),
            },
            &mut left_rng,
        )
        .unwrap();

        let mut right_rng = DeterministicRng::new(Seed([19; 32]));
        let right = resolve_attack_wound(
            AttackResolutionActor {
                entity: worldwake_core::EntityId {
                    slot: 1,
                    generation: 1,
                },
                profile,
                needs: None,
                wounds: None,
            },
            AttackResolutionTarget {
                profile,
                stance: None,
                needs: None,
                wounds: &wounds,
            },
            AttackResolutionContext {
                weapon: CombatWeaponRef::Commodity(CommodityKind::Sword),
                tick: Tick(11),
            },
            &mut right_rng,
        )
        .unwrap();

        assert_eq!(left, right);
    }

    #[test]
    fn resolve_attack_wound_uses_weapon_profiles_for_severity_and_bleed_rate() {
        let attacker_profile = CombatProfile::new(
            pm(1000),
            pm(700),
            pm(1000),
            pm(0),
            pm(0),
            pm(0),
            pm(0),
            pm(90),
            pm(12),
            nz(1),
        );
        let target_profile = CombatProfile::new(
            pm(1000),
            pm(700),
            pm(0),
            pm(0),
            pm(0),
            pm(0),
            pm(0),
            pm(10),
            pm(0),
            nz(1),
        );
        let wounds = WoundList::default();

        let mut sword_rng = DeterministicRng::new(Seed([0; 32]));
        let sword = resolve_attack_wound(
            AttackResolutionActor {
                entity: worldwake_core::EntityId {
                    slot: 1,
                    generation: 1,
                },
                profile: attacker_profile,
                needs: None,
                wounds: None,
            },
            AttackResolutionTarget {
                profile: target_profile,
                stance: None,
                needs: None,
                wounds: &wounds,
            },
            AttackResolutionContext {
                weapon: CombatWeaponRef::Commodity(CommodityKind::Sword),
                tick: Tick(3),
            },
            &mut sword_rng,
        )
        .unwrap()
        .unwrap();

        let mut unarmed_rng = DeterministicRng::new(Seed([0; 32]));
        let unarmed = resolve_attack_wound(
            AttackResolutionActor {
                entity: worldwake_core::EntityId {
                    slot: 1,
                    generation: 1,
                },
                profile: attacker_profile,
                needs: None,
                wounds: None,
            },
            AttackResolutionTarget {
                profile: target_profile,
                stance: None,
                needs: None,
                wounds: &wounds,
            },
            AttackResolutionContext {
                weapon: CombatWeaponRef::Unarmed,
                tick: Tick(3),
            },
            &mut unarmed_rng,
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            sword.severity,
            CommodityKind::Sword
                .spec()
                .combat_weapon_profile
                .unwrap()
                .base_wound_severity
        );
        assert_eq!(
            sword.bleed_rate_per_tick,
            CommodityKind::Sword
                .spec()
                .combat_weapon_profile
                .unwrap()
                .base_bleed_rate
        );
        assert_eq!(unarmed.severity, attacker_profile.unarmed_wound_severity);
        assert_eq!(
            unarmed.bleed_rate_per_tick,
            attacker_profile.unarmed_bleed_rate
        );
    }

    #[test]
    fn defend_is_not_offered_to_dead_or_incapacitated_agents() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let dead = spawn_guard(&mut world, 1, ControlSource::Ai);
        let incapacitated = spawn_guard(&mut world, 2, ControlSource::Ai);
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_dead_at(dead, DeadAt(Tick(3))).unwrap();
            txn.set_component_wound_list(
                incapacitated,
                WoundList {
                    wounds: vec![worldwake_core::Wound {
                        id: WoundId(1),
                        body_part: worldwake_core::BodyPart::Torso,
                        cause: worldwake_core::WoundCause::Deprivation(
                            worldwake_core::DeprivationKind::Starvation,
                        ),
                        severity: pm(700),
                        inflicted_at: Tick(3),
                        bleed_rate_per_tick: pm(0),
                    }],
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let defend_id = register_defend_action(&mut defs, &mut handlers);

        let dead_affordances =
            get_affordances(&OmniscientBeliefView::new(&world), dead, &defs, &handlers);
        let incapacitated_affordances = get_affordances(
            &OmniscientBeliefView::new(&world),
            incapacitated,
            &defs,
            &handlers,
        );

        assert!(!dead_affordances
            .iter()
            .any(|affordance| affordance.def_id == defend_id));
        assert!(!incapacitated_affordances
            .iter()
            .any(|affordance| affordance.def_id == defend_id));
    }

    #[test]
    fn defend_start_gate_rejects_dead_and_incapacitated_actors_authoritatively() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let dead = spawn_guard(&mut world, 1, ControlSource::Ai);
        let incapacitated = spawn_guard(&mut world, 2, ControlSource::Ai);
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_dead_at(dead, DeadAt(Tick(3))).unwrap();
            txn.set_component_wound_list(
                incapacitated,
                WoundList {
                    wounds: vec![worldwake_core::Wound {
                        id: WoundId(1),
                        body_part: worldwake_core::BodyPart::Torso,
                        cause: worldwake_core::WoundCause::Deprivation(
                            worldwake_core::DeprivationKind::Starvation,
                        ),
                        severity: pm(700),
                        inflicted_at: Tick(3),
                        bleed_rate_per_tick: pm(0),
                    }],
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let defend_id = register_defend_action(&mut defs, &mut handlers);
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = test_rng(0x12);

        let dead_err = start_action(
            &Affordance {
                def_id: defend_id,
                actor: dead,
                bound_targets: Vec::new(),
                payload_override: None,
                explanation: None,
            },
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(5),
            },
        )
        .unwrap_err();
        assert_eq!(
            dead_err,
            ActionError::ConstraintFailed("ActorNotDead".to_string())
        );

        let incap_err = start_action(
            &Affordance {
                def_id: defend_id,
                actor: incapacitated,
                bound_targets: Vec::new(),
                payload_override: None,
                explanation: None,
            },
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(5),
            },
        )
        .unwrap_err();
        assert_eq!(
            incap_err,
            ActionError::ConstraintFailed("ActorNotIncapacitated".to_string())
        );
    }

    #[test]
    fn combat_system_attaches_dead_at_and_emits_combat_event_for_fatal_wounds() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = spawn_guard(&mut world, 1, ControlSource::Ai);
        let place = world.topology().place_ids().next().unwrap();
        let mut log = EventLog::new();
        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_component_wound_list(
                guard,
                WoundList {
                    wounds: vec![worldwake_core::Wound {
                        id: WoundId(7),
                        body_part: worldwake_core::BodyPart::Head,
                        cause: worldwake_core::WoundCause::Deprivation(
                            worldwake_core::DeprivationKind::Starvation,
                        ),
                        severity: pm(1000),
                        inflicted_at: Tick(2),
                        bleed_rate_per_tick: pm(0),
                    }],
                },
            )
            .unwrap();
            let bread = txn
                .create_item_lot(worldwake_core::CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(bread, place).unwrap();
            txn.set_possessor(bread, guard).unwrap();
            let _ = txn.commit(&mut log);
        }
        let place_before = world.effective_place(guard);
        let mut rng = DeterministicRng::new(Seed([7; 32]));
        let active_actions = BTreeMap::new();
        let defs = worldwake_sim::ActionDefRegistry::new();

        combat_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &defs,
            tick: Tick(5),
            system_id: SystemId::Combat,
        })
        .unwrap();

        assert_eq!(world.get_component_dead_at(guard), Some(&DeadAt(Tick(5))));
        assert_eq!(world.effective_place(guard), place_before);
        assert_eq!(
            world.controlled_commodity_quantity(guard, worldwake_core::CommodityKind::Bread),
            Quantity(1)
        );
        assert!(!world.is_archived(guard));
        assert_eq!(log.events_by_tag(EventTag::Combat).len(), 1);
        let record = log.get(log.events_by_tag(EventTag::Combat)[0]).unwrap();
        assert_eq!(record.actor_id, Some(guard));
        assert!(matches!(record.cause, CauseRef::Event(_)));
        assert_eq!(
            record.evidence,
            vec![EvidenceRef::Wound {
                entity: guard,
                wound_id: WoundId(7),
            }]
        );
        assert!(record.tags.contains(&EventTag::System));
        assert!(record.tags.contains(&EventTag::WorldMutation));
    }

    #[test]
    fn combat_system_does_not_reemit_death_for_already_dead_agents() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = spawn_guard(&mut world, 1, ControlSource::Ai);
        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_component_wound_list(
                guard,
                WoundList {
                    wounds: vec![worldwake_core::Wound {
                        id: WoundId(8),
                        body_part: worldwake_core::BodyPart::Head,
                        cause: worldwake_core::WoundCause::Deprivation(
                            worldwake_core::DeprivationKind::Starvation,
                        ),
                        severity: pm(1000),
                        inflicted_at: Tick(2),
                        bleed_rate_per_tick: pm(0),
                    }],
                },
            )
            .unwrap();
            txn.set_component_dead_at(guard, DeadAt(Tick(3))).unwrap();
            commit_txn(txn);
        }
        let mut log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([5; 32]));
        let active_actions = BTreeMap::new();
        let defs = worldwake_sim::ActionDefRegistry::new();

        combat_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &defs,
            tick: Tick(6),
            system_id: SystemId::Combat,
        })
        .unwrap();

        assert_eq!(world.get_component_dead_at(guard), Some(&DeadAt(Tick(3))));
        assert!(log.is_empty());
    }

    #[test]
    fn dispatch_table_uses_combat_system_for_combat_slot() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = spawn_guard(&mut world, 1, ControlSource::Ai);
        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_component_wound_list(
                guard,
                WoundList {
                    wounds: vec![worldwake_core::Wound {
                        id: WoundId(9),
                        body_part: worldwake_core::BodyPart::Head,
                        cause: worldwake_core::WoundCause::Deprivation(
                            worldwake_core::DeprivationKind::Starvation,
                        ),
                        severity: pm(1000),
                        inflicted_at: Tick(2),
                        bleed_rate_per_tick: pm(0),
                    }],
                },
            )
            .unwrap();
            commit_txn(txn);
        }
        let systems = dispatch_table();
        let mut log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([11; 32]));
        let active_actions = BTreeMap::new();
        let defs = worldwake_sim::ActionDefRegistry::new();

        systems.get(SystemId::Combat)(SystemExecutionContext {
            world: &mut world,
            event_log: &mut log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &defs,
            tick: Tick(7),
            system_id: SystemId::Combat,
        })
        .unwrap();

        assert_eq!(world.get_component_dead_at(guard), Some(&DeadAt(Tick(7))));
        assert_eq!(log.events_by_tag(EventTag::Combat).len(), 1);
    }

    #[test]
    fn combat_system_skips_wounded_agents_without_combat_profile() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = spawn_guard(&mut world, 1, ControlSource::Ai);
        {
            let mut txn = new_txn(&mut world, 2);
            txn.clear_component_combat_profile(guard).unwrap();
            txn.set_component_wound_list(
                guard,
                WoundList {
                    wounds: vec![deprivation_wound(1, 100, 35, 2)],
                },
            )
            .unwrap();
            commit_txn(txn);
        }
        let before = world.get_component_wound_list(guard).unwrap().clone();
        let mut log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([12; 32]));
        let active_actions = BTreeMap::new();
        let defs = worldwake_sim::ActionDefRegistry::new();

        combat_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &defs,
            tick: Tick(3),
            system_id: SystemId::Combat,
        })
        .unwrap();

        assert_eq!(world.get_component_combat_profile(guard), None);
        assert_eq!(world.get_component_wound_list(guard), Some(&before));
        assert!(world.get_component_dead_at(guard).is_none());
        assert!(log.is_empty());
    }

    #[test]
    fn combat_system_progresses_bleeding_wounds_and_applies_clotting() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = spawn_guard(&mut world, 1, ControlSource::Ai);
        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_component_wound_list(
                guard,
                WoundList {
                    wounds: vec![deprivation_wound(1, 100, 35, 2)],
                },
            )
            .unwrap();
            commit_txn(txn);
        }
        let mut log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([13; 32]));
        let active_actions = BTreeMap::new();
        let defs = worldwake_sim::ActionDefRegistry::new();

        combat_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &defs,
            tick: Tick(3),
            system_id: SystemId::Combat,
        })
        .unwrap();

        let wounds = world.get_component_wound_list(guard).unwrap();
        assert_eq!(wounds.wounds[0].severity, pm(135));
        assert_eq!(wounds.wounds[0].bleed_rate_per_tick, pm(15));
    }

    #[test]
    fn higher_clot_resistance_stabilizes_faster() {
        let base = WoundList {
            wounds: vec![deprivation_wound(1, 100, 35, 2)],
        };
        let slow = CombatProfile::new(
            pm(1000),
            pm(700),
            pm(600),
            pm(550),
            pm(75),
            pm(5),
            pm(15),
            pm(120),
            pm(30),
            nz(6),
        );
        let fast = CombatProfile::new(
            pm(1000),
            pm(700),
            pm(600),
            pm(550),
            pm(75),
            pm(20),
            pm(15),
            pm(120),
            pm(30),
            nz(6),
        );

        let slow_next = super::progress_wounds(
            &base,
            slow,
            Some(HomeostaticNeeds::new_sated()),
            Some(DriveThresholds::default()),
            false,
        )
        .unwrap();
        let fast_next = super::progress_wounds(
            &base,
            fast,
            Some(HomeostaticNeeds::new_sated()),
            Some(DriveThresholds::default()),
            false,
        )
        .unwrap();

        assert_eq!(slow_next.wounds[0].bleed_rate_per_tick, pm(30));
        assert_eq!(fast_next.wounds[0].bleed_rate_per_tick, pm(15));
    }

    #[test]
    fn non_bleeding_wounds_recover_when_physiology_is_tolerable() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = spawn_guard(&mut world, 1, ControlSource::Ai);
        set_recovery_state(&mut world, guard, 2, HomeostaticNeeds::new_sated());
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_wound_list(
                guard,
                WoundList {
                    wounds: vec![deprivation_wound(1, 100, 0, 3)],
                },
            )
            .unwrap();
            commit_txn(txn);
        }
        let mut log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([17; 32]));
        let active_actions = BTreeMap::new();
        let defs = worldwake_sim::ActionDefRegistry::new();

        combat_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &defs,
            tick: Tick(4),
            system_id: SystemId::Combat,
        })
        .unwrap();

        let wounds = world.get_component_wound_list(guard).unwrap();
        assert_eq!(wounds.wounds[0].severity, pm(85));
        assert_eq!(wounds.wounds[0].bleed_rate_per_tick, pm(0));
    }

    #[test]
    fn recovery_is_blocked_during_active_combat_domain_actions() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = spawn_guard(&mut world, 1, ControlSource::Ai);
        set_recovery_state(&mut world, guard, 2, HomeostaticNeeds::new_sated());
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_wound_list(
                guard,
                WoundList {
                    wounds: vec![deprivation_wound(1, 100, 0, 3)],
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let defend_id = register_defend_action(&mut defs, &mut handlers);
        let affordance =
            get_affordances(&OmniscientBeliefView::new(&world), guard, &defs, &handlers)
                .into_iter()
                .find(|affordance| affordance.def_id == defend_id)
                .unwrap();
        let mut active_actions = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let mut action_rng = test_rng(0x13);

        let _ = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut action_rng,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(4),
            },
        )
        .unwrap();

        let mut rng = DeterministicRng::new(Seed([18; 32]));
        combat_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &defs,
            tick: Tick(5),
            system_id: SystemId::Combat,
        })
        .unwrap();

        let wounds = world.get_component_wound_list(guard).unwrap();
        assert_eq!(wounds.wounds[0].severity, pm(100));
    }

    #[test]
    fn recovery_is_blocked_when_physiology_exceeds_tolerable_thresholds() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = spawn_guard(&mut world, 1, ControlSource::Ai);
        set_recovery_state(
            &mut world,
            guard,
            2,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_wound_list(
                guard,
                WoundList {
                    wounds: vec![deprivation_wound(1, 100, 0, 3)],
                },
            )
            .unwrap();
            commit_txn(txn);
        }
        let mut log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([19; 32]));
        let active_actions = BTreeMap::new();
        let defs = worldwake_sim::ActionDefRegistry::new();

        combat_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &defs,
            tick: Tick(4),
            system_id: SystemId::Combat,
        })
        .unwrap();

        let wounds = world.get_component_wound_list(guard).unwrap();
        assert_eq!(wounds.wounds[0].severity, pm(100));
    }

    #[test]
    fn healed_wounds_are_removed_from_wound_list() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = spawn_guard(&mut world, 1, ControlSource::Ai);
        set_recovery_state(&mut world, guard, 2, HomeostaticNeeds::new_sated());
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_wound_list(
                guard,
                WoundList {
                    wounds: vec![deprivation_wound(1, 10, 0, 3)],
                },
            )
            .unwrap();
            commit_txn(txn);
        }
        let mut log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([23; 32]));
        let active_actions = BTreeMap::new();
        let defs = worldwake_sim::ActionDefRegistry::new();

        combat_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &defs,
            tick: Tick(4),
            system_id: SystemId::Combat,
        })
        .unwrap();

        let wounds = world.get_component_wound_list(guard).unwrap();
        assert!(wounds.wounds.is_empty());
    }

    #[test]
    fn progression_can_trigger_same_tick_fatality() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = spawn_guard(&mut world, 1, ControlSource::Ai);
        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_component_wound_list(
                guard,
                WoundList {
                    wounds: vec![deprivation_wound(1, 980, 30, 2)],
                },
            )
            .unwrap();
            commit_txn(txn);
        }
        let mut log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([29; 32]));
        let active_actions = BTreeMap::new();
        let defs = worldwake_sim::ActionDefRegistry::new();

        combat_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &defs,
            tick: Tick(3),
            system_id: SystemId::Combat,
        })
        .unwrap();

        assert_eq!(world.get_component_dead_at(guard), Some(&DeadAt(Tick(3))));
        let record = log.get(log.events_by_tag(EventTag::Combat)[0]).unwrap();
        assert!(matches!(record.cause, CauseRef::Event(_)));
    }
}
