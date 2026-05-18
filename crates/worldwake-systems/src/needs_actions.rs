use crate::inventory::consume_one_unit;
use std::collections::BTreeSet;
use std::num::NonZeroU32;
use worldwake_core::{
    ActionDefId, CommodityKind, DecisionEventPayload, Discrepancy, EntityId, EventTag,
    FrameAssumption, HomeostaticNeedId, HomeostaticNeeds, ItemLot, MetabolismProfile,
    OUTDOOR_RELIEF_TAGS, Permille, PlaceTag, Quantity, SleepEpisode, SleepEpisodeEndedPayload,
    SleepEpisodeStartedPayload, SleepRecoveryModifier, Tick, VisibilitySpec, WakeCondition,
    WakeReason, WashFacilityUsedPayload, WasteCreatedPayload, WasteSource, WorkstationTag,
    WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerId,
    ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress, ActionState,
    CommitOutcome, Constraint, ConsumableEffect, DeterministicRng, DurationExpr, EffectEntityRef,
    EffectEvaluationContext, EffectMode, EffectPrecondition, EffectSchema, EffectSink, EffectStep,
    Interruptibility, MetabolismDurationKind, Precondition, TargetSpec, apply_effects_with_context,
};

use crate::evidence_support::emit_evidence;

pub fn register_needs_actions(defs: &mut ActionDefRegistry, handlers: &mut ActionHandlerRegistry) {
    let eat_handler = handlers.register(ActionHandler::new(
        start_noop,
        tick_continue,
        commit_eat,
        abort_noop,
    ));
    let drink_handler = handlers.register(ActionHandler::new(
        start_noop,
        tick_continue,
        commit_drink,
        abort_noop,
    ));
    let sleep_handler = handlers.register(ActionHandler::new(
        start_sleep_episode,
        tick_sleep,
        commit_sleep_episode,
        abort_sleep_episode,
    ));
    let toilet_handler = handlers.register(ActionHandler::new(
        start_noop,
        tick_continue,
        commit_toilet,
        abort_noop,
    ));
    let wash_handler = handlers.register(ActionHandler::new(
        start_noop,
        tick_continue,
        commit_wash,
        abort_noop,
    ));
    let relieve_wilderness_handler = handlers.register(ActionHandler::new(
        start_noop,
        tick_continue,
        commit_relieve_wilderness,
        abort_noop,
    ));

    register_def(
        defs,
        "eat",
        eat_handler,
        eat_preconditions(),
        DurationExpr::TargetConsumable { target_index: 0 },
    );
    register_def(
        defs,
        "drink",
        drink_handler,
        drink_preconditions(),
        DurationExpr::TargetConsumable { target_index: 0 },
    );
    register_def(
        defs,
        "sleep",
        sleep_handler,
        vec![Precondition::ActorAlive],
        DurationExpr::Variable {
            min: NonZeroU32::new(1).unwrap(),
            max: NonZeroU32::new(64).unwrap(),
        },
    );
    register_def(
        defs,
        "toilet",
        toilet_handler,
        vec![Precondition::ActorAlive],
        DurationExpr::ActorMetabolism {
            kind: MetabolismDurationKind::Toilet,
        },
    );
    register_def(
        defs,
        "wash",
        wash_handler,
        wash_preconditions(),
        DurationExpr::ActorMetabolism {
            kind: MetabolismDurationKind::Wash,
        },
    );

    // relieve_wilderness: registered directly because it needs SamePlace visibility
    // and WildernessRelief event tag, unlike the other needs actions.
    let rw_id = ActionDefId(defs.len() as u32);
    defs.register(ActionDef {
        id: rw_id,
        name: "relieve_wilderness".to_string(),
        domain: worldwake_core::ActionDomain::Needs,
        actor_constraints: vec![
            Constraint::ActorAlive,
            Constraint::ActorNotIncapacitated,
            Constraint::ActorNotInTransit,
            Constraint::ActorAtPlaceWithAnyTag(OUTDOOR_RELIEF_TAGS),
        ],
        targets: Vec::new(),
        preconditions: vec![Precondition::ActorAlive],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::ActorMetabolism {
            kind: MetabolismDurationKind::Toilet,
        },
        body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
        attention_cost: Permille::ZERO,
        interruptibility: Interruptibility::InterruptibleWithPenalty,
        commit_conditions: vec![Precondition::ActorAlive],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([
            EventTag::WorldMutation,
            EventTag::WildernessRelief,
            EventTag::WasteCreated,
        ]),
        payload: ActionPayload::None,
        handler: relieve_wilderness_handler,
        binding_strictness: worldwake_sim::BindingStrictness::AnyLegalTarget,
        guard_template: None,
        expectation_template: vec![],
        effect_schema: needs_effect_schema("relieve_wilderness"),
    });
}

fn register_def(
    defs: &mut ActionDefRegistry,
    name: &str,
    handler: ActionHandlerId,
    preconditions: Vec<Precondition>,
    duration: DurationExpr,
) -> ActionDefId {
    let id = ActionDefId(defs.len() as u32);
    defs.register(ActionDef {
        id,
        name: name.to_string(),
        domain: worldwake_core::ActionDomain::Needs,
        actor_constraints: match name {
            "toilet" => vec![
                Constraint::ActorAlive,
                Constraint::ActorAtPlaceTag(PlaceTag::Latrine),
            ],
            _ => vec![Constraint::ActorAlive],
        },
        targets: match name {
            "eat" | "drink" => vec![TargetSpec::EntityAtActorPlace {
                kind: worldwake_core::EntityKind::ItemLot,
            }],
            "wash" => vec![TargetSpec::EntityAtActorPlace {
                kind: worldwake_core::EntityKind::Facility,
            }],
            _ => Vec::new(),
        },
        preconditions: preconditions.clone(),
        reservation_requirements: Vec::new(),
        duration,
        body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
        attention_cost: Permille::ZERO,
        interruptibility: match name {
            "sleep" => Interruptibility::FreelyInterruptible,
            _ => Interruptibility::InterruptibleWithPenalty,
        },
        commit_conditions: preconditions,
        visibility: VisibilitySpec::ParticipantsOnly,
        causal_event_tags: match name {
            "wash" => BTreeSet::from([EventTag::WorldMutation, EventTag::WashFacilityUsed]),
            _ => BTreeSet::from([EventTag::WorldMutation]),
        },
        payload: ActionPayload::None,
        handler,
        binding_strictness: match name {
            "eat" | "drink" => worldwake_sim::BindingStrictness::FungibleEquivalentCommodity,
            "sleep" => worldwake_sim::BindingStrictness::AnyLegalTarget,
            "toilet" | "wash" => {
                worldwake_sim::BindingStrictness::EquivalentWorkstationTagAtSamePlace
            }
            other => panic!("unexpected needs action {other}"),
        },
        guard_template: None,
        expectation_template: vec![],
        effect_schema: needs_effect_schema(name),
    })
}

fn needs_effect_schema(name: &str) -> EffectSchema {
    match name {
        "eat" => consumable_effect_schema(ConsumableEffect::Hunger),
        "drink" => consumable_effect_schema(ConsumableEffect::Thirst),
        "sleep" => EffectSchema {
            preconditions: Vec::new(),
            steps: vec![EffectStep::EndSleepEpisode],
        },
        "toilet" => EffectSchema {
            preconditions: Vec::new(),
            steps: vec![EffectStep::UseToilet],
        },
        "relieve_wilderness" => EffectSchema {
            preconditions: Vec::new(),
            steps: vec![EffectStep::RelieveWilderness],
        },
        "wash" => EffectSchema {
            preconditions: vec![EffectPrecondition::CoLocated {
                actor: EffectEntityRef::Actor,
                target: EffectEntityRef::Target { index: 0 },
            }],
            steps: vec![EffectStep::UseWashBasin {
                basin: EffectEntityRef::Target { index: 0 },
            }],
        },
        other => panic!("unexpected needs action {other}"),
    }
}

fn consumable_effect_schema(effect: ConsumableEffect) -> EffectSchema {
    EffectSchema {
        preconditions: Vec::new(),
        steps: vec![EffectStep::ConsumeTargetConsumable {
            target: EffectEntityRef::Target { index: 0 },
            effect,
        }],
    }
}

fn eat_preconditions() -> Vec<Precondition> {
    vec![
        Precondition::ActorAlive,
        Precondition::TargetExists(0),
        Precondition::TargetKind {
            target_index: 0,
            kind: worldwake_core::EntityKind::ItemLot,
        },
        Precondition::TargetActorControls(0),
        Precondition::TargetHasConsumableEffect {
            target_index: 0,
            effect: ConsumableEffect::Hunger,
        },
    ]
}

fn drink_preconditions() -> Vec<Precondition> {
    vec![
        Precondition::ActorAlive,
        Precondition::TargetExists(0),
        Precondition::TargetKind {
            target_index: 0,
            kind: worldwake_core::EntityKind::ItemLot,
        },
        Precondition::TargetActorControls(0),
        Precondition::TargetHasConsumableEffect {
            target_index: 0,
            effect: ConsumableEffect::Thirst,
        },
    ]
}

fn wash_preconditions() -> Vec<Precondition> {
    vec![
        Precondition::ActorAlive,
        Precondition::TargetExists(0),
        Precondition::TargetAtActorPlace(0),
        Precondition::TargetKind {
            target_index: 0,
            kind: worldwake_core::EntityKind::Facility,
        },
        Precondition::TargetHasWorkstationTag {
            target_index: 0,
            tag: WorkstationTag::WashBasin,
        },
        Precondition::TargetHasWashBasinClean {
            target_index: 0,
            min: 1,
        },
    ]
}

fn lot_profile(
    txn: &WorldTxn<'_>,
    lot_id: EntityId,
) -> Result<worldwake_core::CommodityConsumableProfile, ActionError> {
    let lot = lot(txn, lot_id)?;
    lot.commodity
        .spec()
        .consumable_profile
        .ok_or_else(|| ActionError::PreconditionFailed(format!("lot {lot_id} is not consumable")))
}

fn lot(txn: &WorldTxn<'_>, lot_id: EntityId) -> Result<ItemLot, ActionError> {
    txn.get_component_item_lot(lot_id)
        .cloned()
        .ok_or(ActionError::InvalidTarget(lot_id))
}

fn actor_needs(txn: &WorldTxn<'_>, actor: EntityId) -> Result<HomeostaticNeeds, ActionError> {
    txn.get_component_homeostatic_needs(actor)
        .copied()
        .ok_or_else(|| ActionError::InternalError(format!("actor {actor} lacks needs component")))
}

fn actor_profile(txn: &WorldTxn<'_>, actor: EntityId) -> Result<MetabolismProfile, ActionError> {
    txn.get_component_metabolism_profile(actor)
        .copied()
        .ok_or_else(|| {
            ActionError::InternalError(format!("actor {actor} lacks metabolism profile"))
        })
}

fn set_actor_needs(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    needs: HomeostaticNeeds,
) -> Result<(), ActionError> {
    txn.set_component_homeostatic_needs(actor, needs)
        .map_err(|err| ActionError::InternalError(err.to_string()))
}

#[allow(clippy::unnecessary_wraps)]
fn start_noop(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    Ok(None)
}

#[allow(clippy::unnecessary_wraps)]
fn tick_continue(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

#[allow(clippy::unnecessary_wraps)]
fn abort_noop(
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

fn start_sleep_episode(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let needs = actor_needs(txn, instance.actor)?;
    let profile = actor_profile(txn, instance.actor)?;
    let place = txn.effective_place(instance.actor).ok_or_else(|| {
        ActionError::InternalError(format!("actor {} has no place", instance.actor))
    })?;
    let quality = txn
        .get_component_sleep_quality_profile(place)
        .copied()
        .unwrap_or_default();
    let intended_min_ticks = profile.min_sleep_ticks;
    let intended_max_ticks = intended_sleep_ticks(needs.fatigue, profile, intended_min_ticks);
    let target_recovery = Permille::ZERO;
    let wake_conditions = crate::sleep_synthesis::synthesize_wake_conditions(
        instance.actor,
        intended_max_ticks,
        target_recovery,
        context.tick,
        txn,
    );
    let episode = SleepEpisode {
        place,
        start_tick: context.tick,
        intended_min_ticks,
        intended_max_ticks,
        target_recovery,
        accumulated_recovery: Permille::ZERO,
        recovery_modifier: quality.recovery_modifier,
        wake_conditions,
    };
    txn.set_component_sleep_episode(instance.actor, episode.clone())
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.add_tag(EventTag::SleepEpisodeStarted)
        .set_decision_payload(DecisionEventPayload::SleepEpisodeStarted(
            SleepEpisodeStartedPayload {
                sleeper: instance.actor,
                place,
                intended_min_ticks,
                intended_max_ticks,
                target_recovery,
                wake_conditions: episode.wake_conditions,
                recovery_modifier: episode.recovery_modifier,
            },
        ));
    Ok(Some(ActionState::Empty))
}

fn intended_sleep_ticks(
    fatigue: Permille,
    profile: MetabolismProfile,
    intended_min_ticks: NonZeroU32,
) -> NonZeroU32 {
    let hard_cap = 64_u32.max(intended_min_ticks.get());
    let rest_efficiency = u32::from(profile.rest_efficiency.value());
    let ticks_to_recover = if fatigue.value() == 0 {
        1
    } else if rest_efficiency == 0 {
        hard_cap
    } else {
        u32::from(fatigue.value()).div_ceil(rest_efficiency)
    };
    let bounded = ticks_to_recover
        .max(intended_min_ticks.get())
        .min(hard_cap)
        .max(1);
    NonZeroU32::new(bounded).unwrap()
}

fn tick_sleep(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    let mut episode = txn
        .get_component_sleep_episode(instance.actor)
        .cloned()
        .ok_or_else(|| {
            ActionError::InternalError(format!(
                "actor {} lacks active sleep episode",
                instance.actor
            ))
        })?;
    let needs = actor_needs(txn, instance.actor)?;
    let profile = actor_profile(txn, instance.actor)?;
    let recovery = sleep_recovery_delta(profile.rest_efficiency, episode.recovery_modifier);
    let next = HomeostaticNeeds::new(
        needs.hunger,
        needs.thirst,
        needs.fatigue.saturating_sub(recovery),
        needs.bladder,
        needs.dirtiness,
    );
    episode.accumulated_recovery = episode.accumulated_recovery.saturating_add(recovery);
    set_actor_needs(txn, instance.actor, next)?;
    txn.set_component_sleep_episode(instance.actor, episode.clone())
        .map_err(|err| ActionError::InternalError(err.to_string()))?;

    if sleep_wake_reason(txn, instance.actor, &episode, context.tick).is_some() {
        Ok(ActionProgress::Complete)
    } else {
        Ok(ActionProgress::Continue)
    }
}

fn sleep_recovery_delta(
    rest_efficiency: Permille,
    recovery_modifier: SleepRecoveryModifier,
) -> Permille {
    let scaled = (u32::from(rest_efficiency.value()) * u32::from(recovery_modifier.value())) / 1000;
    Permille::new(scaled.min(1000) as u16).unwrap()
}

fn sleep_wake_reason(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    episode: &SleepEpisode,
    current_tick: Tick,
) -> Option<WakeReason> {
    episode
        .wake_conditions
        .iter()
        .find_map(|condition| match *condition {
            WakeCondition::IntendedDurationReached => {
                (current_tick.0.saturating_sub(episode.start_tick.0)
                    >= u64::from(episode.intended_max_ticks.get()))
                .then_some(WakeReason::IntendedDuration)
            }
            WakeCondition::TargetRecoveryReached => {
                let required_recovery =
                    Permille::new_unchecked(1000).saturating_sub(episode.target_recovery);
                (episode.accumulated_recovery >= required_recovery)
                    .then_some(WakeReason::TargetRecovery)
            }
            WakeCondition::ProjectedNeedBreach { need } => {
                projected_need_breach_tick(txn, actor, need)
                    .filter(|until_tick| *until_tick <= current_tick)
                    .map(|projected_breach_tick| WakeReason::ProjectedNeedBreach {
                        need,
                        projected_breach_tick,
                    })
            }
            WakeCondition::ScheduledCommitmentDue { tick } => {
                (current_tick >= tick).then_some(WakeReason::ScheduledCommitment)
            }
            WakeCondition::LocalDisturbance => None,
        })
}

fn projected_need_breach_tick(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    need: HomeostaticNeedId,
) -> Option<Tick> {
    txn.get_component_intention_frame(actor).and_then(|frame| {
        frame
            .assumptions
            .iter()
            .find_map(|assumption| match *assumption {
                FrameAssumption::NeedSafeUntilTick {
                    need: assumed_need,
                    until_tick,
                } if assumed_need == need => Some(until_tick),
                _ => None,
            })
    })
}

fn commit_sleep_episode(
    def: &ActionDef,
    instance: &ActionInstance,
    context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    apply_needs_effect_schema(def, instance, context.tick, txn)
}

fn abort_sleep_episode(
    _def: &ActionDef,
    instance: &ActionInstance,
    context: &worldwake_sim::ActionExecutionContext<'_>,
    _reason: &AbortReason,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    end_sleep_episode(
        instance.actor,
        context.tick,
        Some(WakeReason::LocalDisturbance),
        txn,
    )
}

fn end_sleep_episode(
    actor: EntityId,
    current_tick: Tick,
    forced_reason: Option<WakeReason>,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    let episode = txn
        .get_component_sleep_episode(actor)
        .cloned()
        .ok_or_else(|| ActionError::InternalError(format!("actor {actor} lacks sleep episode")))?;
    let end_reason = forced_reason
        .or_else(|| sleep_wake_reason(txn, actor, &episode, current_tick))
        .unwrap_or(WakeReason::IntendedDuration);
    let final_fatigue = actor_needs(txn, actor)?.fatigue;

    txn.clear_component_sleep_episode(actor)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.add_tag(EventTag::SleepEpisodeEnded)
        .set_decision_payload(DecisionEventPayload::SleepEpisodeEnded(
            SleepEpisodeEndedPayload {
                sleeper: actor,
                place: episode.place,
                start_tick: episode.start_tick,
                end_tick: current_tick,
                end_reason,
                accumulated_recovery: episode.accumulated_recovery,
                final_fatigue,
            },
        ));
    Ok(())
}

struct NeedsEffectSink<'txn, 'world> {
    txn: &'txn mut WorldTxn<'world>,
    current_tick: Tick,
    action_error: Option<ActionError>,
}

impl<'txn, 'world> NeedsEffectSink<'txn, 'world> {
    fn new(txn: &'txn mut WorldTxn<'world>, current_tick: Tick) -> Self {
        Self {
            txn,
            current_tick,
            action_error: None,
        }
    }

    fn record_error(&mut self, error: ActionError) -> Discrepancy {
        self.action_error = Some(error);
        Discrepancy::PartialExecutionDrift
    }

    fn take_error(mut self, discrepancy: Discrepancy) -> ActionError {
        self.action_error.take().unwrap_or_else(|| {
            ActionError::PreconditionFailed(format!("effect schema failed: {discrepancy:?}"))
        })
    }
}

impl EffectSink for NeedsEffectSink<'_, '_> {
    fn check_precondition(
        &self,
        precondition: &EffectPrecondition,
        actor_entity: EntityId,
        targets: &[EntityId],
    ) -> Result<(), Discrepancy> {
        let ok = match precondition {
            EffectPrecondition::TargetMatchesSlot { .. }
            | EffectPrecondition::CapacityFloor { .. }
            | EffectPrecondition::BeliefHeld { .. }
            | EffectPrecondition::QuantityAvailable { .. }
            | EffectPrecondition::ContentionGrantHeld { .. } => true,
            EffectPrecondition::CoLocated { actor, target } => {
                let actor = resolve_needs_effect_entity_ref(*actor, actor_entity, targets)?;
                let target = resolve_needs_effect_entity_ref(*target, actor_entity, targets)?;
                self.txn.effective_place(actor) == self.txn.effective_place(target)
            }
        };

        ok.then_some(()).ok_or(Discrepancy::MissingObservation)
    }

    fn checkpoint(&mut self) -> usize {
        0
    }

    fn restore(&mut self, _checkpoint: usize) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn write_transfer(
        &mut self,
        _source: EntityId,
        _dest: EntityId,
        _commodity: CommodityKind,
        _quantity: Quantity,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn write_consume(
        &mut self,
        _source: EntityId,
        _commodity: CommodityKind,
        _quantity: Quantity,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn write_produce(
        &mut self,
        _sink: EntityId,
        _commodity: CommodityKind,
        _quantity: Quantity,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn write_wound(
        &mut self,
        _target: EntityId,
        _cause: worldwake_core::WoundCause,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn write_event(&mut self, tag: EventTag) -> Result<(), Discrepancy> {
        self.txn.add_tag(tag);
        Ok(())
    }

    fn assert_expectation_fulfilled(
        &mut self,
        _expectation: worldwake_core::ExpectationId,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn consume_grant(&mut self, _grant: EntityId) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn consume_target_consumable(
        &mut self,
        actor: EntityId,
        target: EntityId,
        effect: ConsumableEffect,
    ) -> Result<(), Discrepancy> {
        apply_consumable_effects(actor, target, effect, self.txn)
            .map_err(|err| self.record_error(err))
    }

    fn end_sleep_episode(&mut self, actor: EntityId) -> Result<(), Discrepancy> {
        end_sleep_episode(actor, self.current_tick, None, self.txn)
            .map_err(|err| self.record_error(err))
    }

    fn use_toilet(&mut self, actor: EntityId) -> Result<(), Discrepancy> {
        apply_toilet(actor, self.txn).map_err(|err| self.record_error(err))
    }

    fn relieve_wilderness(&mut self, actor: EntityId) -> Result<(), Discrepancy> {
        apply_relieve_wilderness(actor, self.txn).map_err(|err| self.record_error(err))
    }

    fn use_wash_basin(&mut self, actor: EntityId, basin: EntityId) -> Result<(), Discrepancy> {
        apply_wash(actor, basin, self.txn).map_err(|err| self.record_error(err))
    }
}

fn resolve_needs_effect_entity_ref(
    entity_ref: EffectEntityRef,
    actor: EntityId,
    targets: &[EntityId],
) -> Result<EntityId, Discrepancy> {
    match entity_ref {
        EffectEntityRef::Actor => Ok(actor),
        EffectEntityRef::Target { index } => targets
            .get(index)
            .copied()
            .ok_or(Discrepancy::NoLegalBinding),
        EffectEntityRef::Entity(entity) => Ok(entity),
    }
}

fn apply_needs_effect_schema(
    def: &ActionDef,
    instance: &ActionInstance,
    current_tick: Tick,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let mut sink = NeedsEffectSink::new(txn, current_tick);
    match apply_effects_with_context(
        &def.effect_schema,
        EffectEvaluationContext {
            actor: instance.actor,
            targets: &instance.targets,
            payload: &instance.payload,
            action_def_id: instance.def_id,
        },
        &mut sink,
        EffectMode::Authoritative,
    ) {
        Ok(_) => Ok(CommitOutcome::empty()),
        Err(discrepancy) => Err(sink.take_error(discrepancy)),
    }
}

fn commit_eat(
    def: &ActionDef,
    instance: &ActionInstance,
    context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    apply_needs_effect_schema(def, instance, context.tick, txn)
}

fn commit_drink(
    def: &ActionDef,
    instance: &ActionInstance,
    context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    apply_needs_effect_schema(def, instance, context.tick, txn)
}

fn apply_consumable_effects(
    actor: EntityId,
    target: EntityId,
    effect: ConsumableEffect,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    let profile = lot_profile(txn, target)?;
    match effect {
        ConsumableEffect::Hunger if profile.hunger_relief_per_unit.value() == 0 => {
            return Err(ActionError::PreconditionFailed(format!(
                "lot {target} has no hunger relief"
            )));
        }
        ConsumableEffect::Thirst if profile.thirst_relief_per_unit.value() == 0 => {
            return Err(ActionError::PreconditionFailed(format!(
                "lot {target} has no thirst relief"
            )));
        }
        ConsumableEffect::Hunger | ConsumableEffect::Thirst => {}
    }

    let needs = actor_needs(txn, actor)?;
    let next = HomeostaticNeeds::new(
        needs.hunger.saturating_sub(profile.hunger_relief_per_unit),
        needs.thirst.saturating_sub(profile.thirst_relief_per_unit),
        needs.fatigue,
        needs.bladder.saturating_add(profile.bladder_fill_per_unit),
        needs.dirtiness,
    );
    consume_one_unit(txn, target)?;
    set_actor_needs(txn, actor, next)
}

fn commit_toilet(
    def: &ActionDef,
    instance: &ActionInstance,
    context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    apply_needs_effect_schema(def, instance, context.tick, txn)
}

fn apply_toilet(actor: EntityId, txn: &mut WorldTxn<'_>) -> Result<(), ActionError> {
    let needs = actor_needs(txn, actor)?;
    let place = txn
        .effective_place(actor)
        .ok_or_else(|| ActionError::InternalError(format!("actor {actor} has no place")))?;
    let waste = txn
        .create_item_lot(CommodityKind::Waste, Quantity(1))
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.set_ground_location(waste, place)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    let mut latrine = txn
        .get_component_latrine_fullness(place)
        .copied()
        .unwrap_or_default();
    latrine.fill = latrine.fill.saturating_add(latrine.fill_per_use);
    txn.set_component_latrine_fullness(place, latrine)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;

    if latrine.fill >= latrine.critical_threshold {
        let mut place_dirtiness = txn
            .get_component_place_dirtiness(place)
            .copied()
            .unwrap_or_default();
        let previous_value = place_dirtiness.value;
        place_dirtiness.value = place_dirtiness
            .value
            .saturating_add(place_dirtiness.dirtiness_per_use);
        let place_dirtiness_delta = place_dirtiness.value.saturating_sub(previous_value);
        txn.set_component_place_dirtiness(place, place_dirtiness)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;

        txn.add_tag(EventTag::WasteCreated).set_decision_payload(
            DecisionEventPayload::WasteCreated(WasteCreatedPayload {
                creator: actor,
                place,
                waste_lot: waste,
                source: WasteSource::OvercapacityLatrine,
                place_dirtiness_delta,
            }),
        );
    }
    set_actor_needs(
        txn,
        actor,
        HomeostaticNeeds::new(
            needs.hunger,
            needs.thirst,
            needs.fatigue,
            pm(0),
            needs.dirtiness,
        ),
    )
}

fn commit_relieve_wilderness(
    def: &ActionDef,
    instance: &ActionInstance,
    context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    apply_needs_effect_schema(def, instance, context.tick, txn)
}

fn apply_relieve_wilderness(actor: EntityId, txn: &mut WorldTxn<'_>) -> Result<(), ActionError> {
    let needs = actor_needs(txn, actor)?;
    let profile = actor_profile(txn, actor)?;
    let place = txn
        .effective_place(actor)
        .ok_or_else(|| ActionError::InternalError(format!("actor {actor} has no place")))?;
    let waste = txn
        .create_item_lot(CommodityKind::Waste, Quantity(1))
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.set_ground_location(waste, place)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    let current_tick = txn.tick();
    emit_evidence(
        txn,
        place,
        worldwake_core::EvidenceKind::DisturbanceMarker {
            place,
            kind: worldwake_core::DisturbanceKind::WildernessRelief,
            created_at: current_tick,
        },
        50,
    )
    .map_err(|err| ActionError::InternalError(err.to_string()))?;
    set_actor_needs(
        txn,
        actor,
        HomeostaticNeeds::new(
            needs.hunger,
            needs.thirst,
            needs.fatigue,
            pm(0),
            needs
                .dirtiness
                .saturating_add(profile.wilderness_relief_dirtiness_penalty),
        ),
    )?;
    let mut place_dirtiness = txn
        .get_component_place_dirtiness(place)
        .copied()
        .unwrap_or_default();
    let previous_value = place_dirtiness.value;
    place_dirtiness.value = place_dirtiness
        .value
        .saturating_add(place_dirtiness.dirtiness_per_use);
    let place_dirtiness_delta = place_dirtiness.value.saturating_sub(previous_value);
    txn.set_component_place_dirtiness(place, place_dirtiness)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.add_tag(EventTag::WasteCreated)
        .set_decision_payload(DecisionEventPayload::WasteCreated(WasteCreatedPayload {
            creator: actor,
            place,
            waste_lot: waste,
            source: WasteSource::WildernessRelief,
            place_dirtiness_delta,
        }));
    Ok(())
}

fn commit_wash(
    def: &ActionDef,
    instance: &ActionInstance,
    context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    apply_needs_effect_schema(def, instance, context.tick, txn)
}

fn apply_wash(actor: EntityId, basin: EntityId, txn: &mut WorldTxn<'_>) -> Result<(), ActionError> {
    let needs = actor_needs(txn, actor)?;
    let mut basin_state = txn
        .get_component_wash_basin_state(basin)
        .cloned()
        .ok_or(ActionError::InvalidTarget(basin))?;
    if basin_state.clean_water_units == 0 {
        return Err(ActionError::PreconditionFailed(format!(
            "wash basin {basin} has no clean water"
        )));
    }
    if basin_state.units_per_full_wash == 0 {
        return Err(ActionError::InternalError(format!(
            "wash basin {basin} has zero units_per_full_wash"
        )));
    }

    let water_consumed = basin_state
        .clean_water_units
        .min(basin_state.units_per_full_wash);
    let partial = water_consumed < basin_state.units_per_full_wash;
    let agent_dirtiness_delta = proportional_permille(
        needs.dirtiness,
        water_consumed,
        basin_state.units_per_full_wash,
    )?;
    let basin_dirtiness_delta = proportional_permille(
        basin_state.dirtiness_per_use,
        water_consumed,
        basin_state.units_per_full_wash,
    )?;

    basin_state.clean_water_units -= water_consumed;
    basin_state.dirtiness_level = basin_state
        .dirtiness_level
        .saturating_add(basin_dirtiness_delta);
    txn.set_component_wash_basin_state(basin, basin_state)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    set_actor_needs(
        txn,
        actor,
        HomeostaticNeeds::new(
            needs.hunger,
            needs.thirst,
            needs.fatigue,
            needs.bladder,
            needs.dirtiness.saturating_sub(agent_dirtiness_delta),
        ),
    )?;
    txn.add_tag(EventTag::WashFacilityUsed)
        .set_decision_payload(DecisionEventPayload::WashFacilityUsed(
            WashFacilityUsedPayload {
                user: actor,
                basin,
                water_consumed,
                agent_dirtiness_delta,
                basin_dirtiness_delta,
                partial,
            },
        ));
    Ok(())
}

fn proportional_permille(
    value: Permille,
    numerator: u16,
    denominator: u16,
) -> Result<Permille, ActionError> {
    if denominator == 0 {
        return Err(ActionError::InternalError(
            "proportional permille denominator must be nonzero".to_string(),
        ));
    }
    let scaled = (u32::from(value.value()) * u32::from(numerator)) / u32::from(denominator);
    Permille::new(scaled as u16).map_err(|err| ActionError::InternalError(err.to_string()))
}

const fn pm(value: u16) -> Permille {
    Permille::new_unchecked(value)
}

#[cfg(test)]
mod tests {
    use super::register_needs_actions;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        ActionDefId, AgentBeliefStore, CauseRef, CommodityKind, ControlSource,
        DecisionEventPayload, DeprivationExposure, DisturbanceKind, DriveThresholds, EntityId,
        EntityKind, EventLog, EventTag, EventView, EvidenceKind, FrameAssumption, FrameState,
        GoalKey, GoalKind, HomeostaticNeedId, HomeostaticNeeds, IntentionDomain, IntentionFrame,
        LatrineFullness, MetabolismProfile, PerceptionSource, Permille, PlaceDirtiness,
        PrototypePlace, Quantity, ResourceSource, Seed, SleepEpisode, SleepRecoveryModifier, Tick,
        VisibilitySpec, WakeCondition, WakeReason, WashBasinState, WasteSource, WitnessData,
        WorkstationMarker, WorkstationTag, World, WorldTxn, build_believed_entity_state,
        build_prototype_world, prototype_place_entity,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionExecutionAuthority, ActionHandlerRegistry, ActionInstance,
        ActionInstanceId, BindingStrictness, DeterministicRng, EffectStep, PerAgentBeliefView,
        Precondition, TickOutcome, abort_action, get_affordances, start_action, tick_action,
    };

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
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

    fn test_rng() -> DeterministicRng {
        DeterministicRng::new(Seed([0x41; 32]))
    }

    fn setup_actor(world: &mut World) -> (EntityId, EntityId) {
        let place = world.topology().place_ids().next().unwrap();
        let mut txn = new_txn(world, 1);
        let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_ground_location(actor, place).unwrap();
        txn.set_component_homeostatic_needs(
            actor,
            HomeostaticNeeds::new(pm(700), pm(650), pm(400), pm(200), pm(350)),
        )
        .unwrap();
        txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
            .unwrap();
        txn.set_component_drive_thresholds(actor, DriveThresholds::default())
            .unwrap();
        txn.set_component_metabolism_profile(
            actor,
            MetabolismProfile::new(
                pm(1),
                pm(1),
                pm(1),
                pm(1),
                pm(1),
                pm(40),
                NonZeroU32::new(10).unwrap(),
                NonZeroU32::new(10).unwrap(),
                NonZeroU32::new(10).unwrap(),
                NonZeroU32::new(10).unwrap(),
                NonZeroU32::new(2).unwrap(),
                NonZeroU32::new(3).unwrap(),
                NonZeroU32::new(8).unwrap(),
                pm(0),
                pm(0),
                pm(0),
                pm(0),
            ),
        )
        .unwrap();
        commit_txn(txn);
        (actor, place)
    }

    fn setup_registries() -> (ActionDefRegistry, ActionHandlerRegistry) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_needs_actions(&mut defs, &mut handlers);
        (defs, handlers)
    }

    fn setup_wash_access(
        world: &mut World,
        place: EntityId,
        clean_water_units: u16,
    ) -> (EntityId, EntityId) {
        let mut txn = new_txn(world, 2);
        let basin = txn.create_entity(EntityKind::Facility);
        let source = txn.create_entity(EntityKind::Facility);
        txn.set_ground_location(basin, place).unwrap();
        txn.set_ground_location(source, place).unwrap();
        txn.set_component_workstation_marker(basin, WorkstationMarker(WorkstationTag::WashBasin))
            .unwrap();
        txn.set_component_wash_basin_state(
            basin,
            WashBasinState {
                clean_water_units,
                max_clean_water: 10,
                refill_per_tick: 1,
                units_per_full_wash: 2,
                dirtiness_level: pm(0),
                dirtiness_per_use: pm(50),
            },
        )
        .unwrap();
        txn.set_component_resource_source(
            source,
            ResourceSource {
                commodity: CommodityKind::Water,
                available_quantity: Quantity(10),
                max_quantity: Quantity(10),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        )
        .unwrap();
        commit_txn(txn);
        (basin, source)
    }

    fn set_actor_dirtiness(world: &mut World, actor: EntityId, dirtiness: Permille) {
        let current = *world.get_component_homeostatic_needs(actor).unwrap();
        let mut txn = new_txn(world, 2);
        txn.set_component_homeostatic_needs(
            actor,
            HomeostaticNeeds::new(
                current.hunger,
                current.thirst,
                current.fatigue,
                current.bladder,
                dirtiness,
            ),
        )
        .unwrap();
        commit_txn(txn);
    }

    fn test_belief_store(world: &World, actor: EntityId) -> AgentBeliefStore {
        let mut store = world
            .get_component_agent_belief_store(actor)
            .cloned()
            .unwrap_or_default();
        for entity in world.entities() {
            if entity == actor {
                continue;
            }
            if let Some(state) = build_believed_entity_state(
                world,
                entity,
                Tick(u64::MAX),
                PerceptionSource::DirectObservation,
            ) {
                store.update_entity(entity, state);
            }
        }
        store
    }

    fn affordances_for(
        world: &World,
        actor: EntityId,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
    ) -> Vec<worldwake_sim::Affordance> {
        let beliefs = test_belief_store(world, actor);
        let view = PerAgentBeliefView::new(actor, world, &beliefs);
        get_affordances(&view, actor, defs, handlers)
    }

    fn run_action_to_completion(
        actor: EntityId,
        affordance_index: usize,
        world: &mut World,
        log: &mut EventLog,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
    ) -> ActionInstanceId {
        let mut active = BTreeMap::<ActionInstanceId, ActionInstance>::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = test_rng();
        let affordances = affordances_for(world, actor, defs, handlers);
        let affordance = affordances[affordance_index].clone();
        let instance_id = start_action(
            &affordance,
            defs,
            handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world,
                event_log: log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();

        for tick in 11..40 {
            match tick_action(
                instance_id,
                defs,
                handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active,
                    world,
                    event_log: log,
                    rng: &mut rng,
                },
                worldwake_sim::ActionExecutionContext::without_recipes(
                    CauseRef::Bootstrap,
                    Tick(tick),
                ),
            )
            .unwrap()
            {
                TickOutcome::Continuing => {}
                TickOutcome::Committed { .. } => break,
                TickOutcome::Aborted { reason, .. } => panic!("unexpected abort: {reason:?}"),
            }
        }

        instance_id
    }

    #[test]
    fn register_needs_actions_adds_all_six_defs_and_handlers() {
        let (defs, handlers) = setup_registries();
        assert_eq!(defs.len(), 6);
        assert_eq!(handlers.len(), 6);
        assert_eq!(defs.get(ActionDefId(0)).unwrap().name, "eat");
        assert!(
            !defs
                .get(ActionDefId(0))
                .unwrap()
                .effect_schema
                .steps
                .is_empty()
        );
        assert_eq!(
            defs.get(ActionDefId(0)).unwrap().binding_strictness,
            BindingStrictness::FungibleEquivalentCommodity
        );
        let sleep = defs.get(ActionDefId(2)).unwrap();
        assert_eq!(sleep.name, "sleep");
        assert_eq!(sleep.effect_schema.steps, vec![EffectStep::EndSleepEpisode]);
        assert_eq!(sleep.binding_strictness, BindingStrictness::AnyLegalTarget);
        assert_eq!(
            sleep.duration,
            worldwake_sim::DurationExpr::Variable {
                min: NonZeroU32::new(1).unwrap(),
                max: NonZeroU32::new(64).unwrap(),
            }
        );
        assert_eq!(
            sleep.interruptibility,
            worldwake_sim::Interruptibility::FreelyInterruptible
        );
        assert_eq!(defs.get(ActionDefId(4)).unwrap().name, "wash");
        let wash = defs.get(ActionDefId(4)).unwrap();
        assert_eq!(
            wash.binding_strictness,
            BindingStrictness::EquivalentWorkstationTagAtSamePlace
        );
        assert!(
            matches!(
                wash.effect_schema.steps.as_slice(),
                [EffectStep::UseWashBasin { .. }]
            ),
            "wash should commit through the needs effect schema"
        );
        assert_eq!(wash.targets.len(), 1, "wash should target only the basin");
        assert!(
            wash.preconditions
                .contains(&Precondition::TargetHasWashBasinClean {
                    target_index: 0,
                    min: 1,
                }),
            "wash should gate on basin clean-water state"
        );
        assert!(
            !wash.preconditions.iter().any(|precondition| matches!(
                precondition,
                Precondition::TargetHasResourceSource { .. }
            )),
            "wash should no longer gate on a direct water-source target"
        );
        let relieve = defs.get(ActionDefId(5)).unwrap();
        assert_eq!(relieve.name, "relieve_wilderness");
        assert_eq!(
            relieve.effect_schema.steps,
            vec![EffectStep::RelieveWilderness]
        );
        assert_eq!(
            relieve.binding_strictness,
            BindingStrictness::AnyLegalTarget
        );
    }

    #[test]
    fn eat_consumes_one_unit_and_applies_consumable_effects() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        let bread = {
            let mut txn = new_txn(&mut world, 2);
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(2))
                .unwrap();
            txn.set_ground_location(bread, place).unwrap();
            txn.set_possessor(bread, actor).unwrap();
            commit_txn(txn);
            bread
        };
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        run_action_to_completion(actor, 0, &mut world, &mut log, &defs, &handlers);

        let needs = world.get_component_homeostatic_needs(actor).unwrap();
        let lot = world.get_component_item_lot(bread).unwrap();
        let profile = CommodityKind::Bread.spec().consumable_profile.unwrap();
        assert_eq!(lot.quantity, Quantity(1));
        assert_eq!(
            needs.hunger,
            pm(700).saturating_sub(profile.hunger_relief_per_unit)
        );
        assert_eq!(
            needs.thirst,
            pm(650).saturating_sub(profile.thirst_relief_per_unit)
        );
        assert_eq!(
            needs.bladder,
            pm(200).saturating_add(profile.bladder_fill_per_unit)
        );
    }

    #[test]
    fn drink_consumes_one_unit_and_applies_consumable_effects() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        let water = {
            let mut txn = new_txn(&mut world, 2);
            let water = txn
                .create_item_lot(CommodityKind::Water, Quantity(2))
                .unwrap();
            txn.set_ground_location(water, place).unwrap();
            txn.set_possessor(water, actor).unwrap();
            commit_txn(txn);
            water
        };
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let affordances = affordances_for(&world, actor, &defs, &handlers);
        let drink_index = affordances
            .iter()
            .position(|affordance| affordance.def_id == ActionDefId(1))
            .unwrap();
        run_action_to_completion(actor, drink_index, &mut world, &mut log, &defs, &handlers);

        let needs = world.get_component_homeostatic_needs(actor).unwrap();
        let lot = world.get_component_item_lot(water).unwrap();
        let profile = CommodityKind::Water.spec().consumable_profile.unwrap();
        assert_eq!(lot.quantity, Quantity(1));
        assert_eq!(
            needs.thirst,
            pm(650).saturating_sub(profile.thirst_relief_per_unit)
        );
        assert_eq!(
            needs.bladder,
            pm(200).saturating_add(profile.bladder_fill_per_unit)
        );
    }

    #[test]
    fn aborted_eat_does_not_consume_item() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        let bread = {
            let mut txn = new_txn(&mut world, 2);
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(bread, place).unwrap();
            txn.set_possessor(bread, actor).unwrap();
            commit_txn(txn);
            bread
        };
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();
        let mut active = BTreeMap::<ActionInstanceId, ActionInstance>::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = test_rng();
        let affordance = affordances_for(&world, actor, &defs, &handlers)[0].clone();
        let instance_id = start_action(
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();

        abort_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(11)),
            worldwake_sim::ExternalAbortReason::Other,
        )
        .unwrap();

        assert_eq!(
            world.get_component_item_lot(bread).unwrap().quantity,
            Quantity(1)
        );
    }

    #[test]
    fn sleep_episode_reduces_fatigue_at_default_place() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, _) = setup_actor(&mut world);
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();
        let mut active = BTreeMap::<ActionInstanceId, ActionInstance>::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = test_rng();

        let affordances = affordances_for(&world, actor, &defs, &handlers);
        let sleep_index = affordances
            .iter()
            .position(|affordance| affordance.def_id == ActionDefId(2))
            .unwrap();
        let instance_id = start_action(
            &affordances[sleep_index],
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();

        let started = log.events_by_tag(EventTag::SleepEpisodeStarted);
        assert_eq!(started.len(), 1);
        let DecisionEventPayload::SleepEpisodeStarted(payload) = log
            .get(started[0])
            .and_then(|record| record.decision_payload())
            .unwrap()
        else {
            panic!("expected SleepEpisodeStarted payload");
        };
        assert_eq!(payload.sleeper, actor);
        assert_eq!(payload.recovery_modifier, SleepRecoveryModifier::IDENTITY);
        assert!(world.get_component_sleep_episode(actor).is_some());

        let first_tick = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(11)),
        )
        .unwrap();
        assert_eq!(first_tick, TickOutcome::Continuing);
        assert_eq!(
            world
                .get_component_homeostatic_needs(actor)
                .unwrap()
                .fatigue,
            pm(400).saturating_sub(pm(40))
        );
        assert_eq!(
            world
                .get_component_sleep_episode(actor)
                .unwrap()
                .accumulated_recovery,
            pm(40)
        );

        for tick in 12..=20 {
            if let TickOutcome::Committed { .. } = tick_action(
                instance_id,
                &defs,
                &handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active,
                    world: &mut world,
                    event_log: &mut log,
                    rng: &mut rng,
                },
                worldwake_sim::ActionExecutionContext::without_recipes(
                    CauseRef::Bootstrap,
                    Tick(tick),
                ),
            )
            .unwrap()
            {
                break;
            }
        }

        assert_eq!(
            world
                .get_component_homeostatic_needs(actor)
                .unwrap()
                .fatigue,
            pm(0)
        );
        assert!(world.get_component_sleep_episode(actor).is_none());
        assert_eq!(log.events_by_tag(EventTag::ActionStarted).len(), 1);
        assert_eq!(log.events_by_tag(EventTag::ActionCommitted).len(), 1);
        let ended = log.events_by_tag(EventTag::SleepEpisodeEnded);
        assert_eq!(ended.len(), 1);
        let DecisionEventPayload::SleepEpisodeEnded(payload) = log
            .get(ended[0])
            .and_then(|record| record.decision_payload())
            .unwrap()
        else {
            panic!("expected SleepEpisodeEnded payload");
        };
        assert_eq!(payload.sleeper, actor);
        assert_eq!(payload.end_reason, WakeReason::IntendedDuration);
        assert_eq!(payload.accumulated_recovery, pm(400));
        assert_eq!(payload.final_fatigue, pm(0));
    }

    #[test]
    fn sleep_wake_reason_uses_first_matching_condition() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        let episode = SleepEpisode {
            place,
            start_tick: Tick(10),
            intended_min_ticks: NonZeroU32::new(1).unwrap(),
            intended_max_ticks: NonZeroU32::new(10).unwrap(),
            target_recovery: pm(960),
            accumulated_recovery: pm(40),
            recovery_modifier: SleepRecoveryModifier::IDENTITY,
            wake_conditions: vec![
                WakeCondition::TargetRecoveryReached,
                WakeCondition::IntendedDurationReached,
            ],
        };
        let txn = new_txn(&mut world, 11);

        assert_eq!(
            super::sleep_wake_reason(&txn, actor, &episode, Tick(20)),
            Some(WakeReason::TargetRecovery)
        );
    }

    #[test]
    fn sleep_wake_reason_maps_projected_need_and_scheduled_commitment() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_intention_frame(
                actor,
                IntentionFrame {
                    goal: GoalKey::from(GoalKind::Sleep),
                    domain: IntentionDomain::Generic,
                    assumptions: vec![FrameAssumption::NeedSafeUntilTick {
                        need: HomeostaticNeedId::Hunger,
                        until_tick: Tick(12),
                    }],
                    state: FrameState::Active,
                    established_at: Tick(3),
                    last_progress_tick: None,
                    stalled_ticks: 0,
                    patience_limit: 10,
                    motive_refs: Vec::new(),
                    resume_conditions: Vec::new(),
                    abandon_conditions: Vec::new(),
                    explicit_claims: Vec::new(),
                    causal_links: Vec::new(),
                },
            )
            .unwrap();
            commit_txn(txn);
        }
        let projected = SleepEpisode {
            place,
            start_tick: Tick(10),
            intended_min_ticks: NonZeroU32::new(1).unwrap(),
            intended_max_ticks: NonZeroU32::new(10).unwrap(),
            target_recovery: pm(1000),
            accumulated_recovery: pm(0),
            recovery_modifier: SleepRecoveryModifier::IDENTITY,
            wake_conditions: vec![WakeCondition::ProjectedNeedBreach {
                need: HomeostaticNeedId::Hunger,
            }],
        };
        let scheduled = SleepEpisode {
            wake_conditions: vec![WakeCondition::ScheduledCommitmentDue { tick: Tick(14) }],
            ..projected.clone()
        };
        let txn = new_txn(&mut world, 12);

        assert_eq!(
            super::sleep_wake_reason(&txn, actor, &projected, Tick(12)),
            Some(WakeReason::ProjectedNeedBreach {
                need: HomeostaticNeedId::Hunger,
                projected_breach_tick: Tick(12),
            })
        );
        assert_eq!(
            super::sleep_wake_reason(&txn, actor, &scheduled, Tick(14)),
            Some(WakeReason::ScheduledCommitment)
        );
    }

    #[test]
    fn toilet_reduces_bladder_and_creates_waste() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::PublicLatrine);
        let mut txn = new_txn(&mut world, 1);
        let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_ground_location(actor, place).unwrap();
        txn.set_component_homeostatic_needs(
            actor,
            HomeostaticNeeds::new(pm(700), pm(650), pm(400), pm(200), pm(350)),
        )
        .unwrap();
        txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
            .unwrap();
        txn.set_component_drive_thresholds(actor, DriveThresholds::default())
            .unwrap();
        txn.set_component_metabolism_profile(
            actor,
            MetabolismProfile::new(
                pm(1),
                pm(1),
                pm(1),
                pm(1),
                pm(1),
                pm(40),
                NonZeroU32::new(10).unwrap(),
                NonZeroU32::new(10).unwrap(),
                NonZeroU32::new(10).unwrap(),
                NonZeroU32::new(10).unwrap(),
                NonZeroU32::new(2).unwrap(),
                NonZeroU32::new(3).unwrap(),
                NonZeroU32::new(8).unwrap(),
                pm(0),
                pm(0),
                pm(0),
                pm(0),
            ),
        )
        .unwrap();
        txn.set_component_latrine_fullness(
            place,
            LatrineFullness {
                fill: pm(0),
                fill_per_use: pm(80),
                critical_threshold: pm(800),
            },
        )
        .unwrap();
        commit_txn(txn);
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let toilet_index = toilet_affordance_index(&world, actor, &defs, &handlers);
        run_action_to_completion(actor, toilet_index, &mut world, &mut log, &defs, &handlers);

        assert_eq!(
            world
                .get_component_homeostatic_needs(actor)
                .unwrap()
                .bladder,
            pm(0)
        );
        let waste_count = world
            .ground_entities_at(place)
            .into_iter()
            .filter(|entity| {
                world
                    .get_component_item_lot(*entity)
                    .is_some_and(|lot| lot.commodity == CommodityKind::Waste)
            })
            .count();
        assert_eq!(waste_count, 1);
        assert_eq!(
            world.get_component_latrine_fullness(place).unwrap().fill,
            pm(80)
        );
        assert!(
            log.events_by_tag(EventTag::WasteCreated).is_empty(),
            "under-threshold toilet use should not emit WasteCreated"
        );
    }

    #[test]
    fn toilet_overflow_emits_waste_created_with_overcapacity_source() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::PublicLatrine);
        let actor = setup_actor_at_place(&mut world, place);
        let mut txn = new_txn(&mut world, 2);
        txn.set_component_latrine_fullness(
            place,
            LatrineFullness {
                fill: pm(750),
                fill_per_use: pm(80),
                critical_threshold: pm(800),
            },
        )
        .unwrap();
        txn.set_component_place_dirtiness(
            place,
            PlaceDirtiness {
                value: pm(40),
                dirtiness_per_use: pm(80),
                ..PlaceDirtiness::default()
            },
        )
        .unwrap();
        commit_txn(txn);
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let toilet_index = toilet_affordance_index(&world, actor, &defs, &handlers);
        run_action_to_completion(actor, toilet_index, &mut world, &mut log, &defs, &handlers);

        assert_eq!(
            world.get_component_latrine_fullness(place).unwrap().fill,
            pm(830)
        );
        assert_eq!(
            world.get_component_place_dirtiness(place).unwrap().value,
            pm(120)
        );
        let waste_created = log.events_by_tag(EventTag::WasteCreated);
        assert_eq!(waste_created.len(), 1);
        let DecisionEventPayload::WasteCreated(payload) = log
            .get(waste_created[0])
            .and_then(|record| record.decision_payload())
            .expect("overflowing toilet should emit WasteCreated payload")
        else {
            panic!("overflowing toilet should emit WasteCreated payload");
        };
        assert_eq!(payload.creator, actor);
        assert_eq!(payload.place, place);
        assert_eq!(payload.source, WasteSource::OvercapacityLatrine);
        assert_eq!(payload.place_dirtiness_delta, pm(80));
        assert!(
            world
                .get_component_item_lot(payload.waste_lot)
                .is_some_and(|lot| lot.commodity == CommodityKind::Waste),
            "WasteCreated payload should reference the created waste lot"
        );
    }

    #[test]
    fn toilet_under_threshold_does_not_emit_waste_created() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::PublicLatrine);
        let actor = setup_actor_at_place(&mut world, place);
        let mut txn = new_txn(&mut world, 2);
        txn.set_component_latrine_fullness(
            place,
            LatrineFullness {
                fill: pm(0),
                fill_per_use: pm(80),
                critical_threshold: pm(800),
            },
        )
        .unwrap();
        commit_txn(txn);
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let toilet_index = toilet_affordance_index(&world, actor, &defs, &handlers);
        run_action_to_completion(actor, toilet_index, &mut world, &mut log, &defs, &handlers);

        assert_eq!(
            world.get_component_latrine_fullness(place).unwrap().fill,
            pm(80)
        );
        assert!(log.events_by_tag(EventTag::WasteCreated).is_empty());
    }

    #[test]
    fn toilet_already_over_threshold_emits_waste_created_each_tick() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::PublicLatrine);
        let actor = setup_actor_at_place(&mut world, place);
        let mut txn = new_txn(&mut world, 2);
        txn.set_component_latrine_fullness(
            place,
            LatrineFullness {
                fill: pm(900),
                fill_per_use: pm(80),
                critical_threshold: pm(800),
            },
        )
        .unwrap();
        commit_txn(txn);
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let toilet_index = toilet_affordance_index(&world, actor, &defs, &handlers);
        run_action_to_completion(actor, toilet_index, &mut world, &mut log, &defs, &handlers);
        let toilet_index = toilet_affordance_index(&world, actor, &defs, &handlers);
        run_action_to_completion(actor, toilet_index, &mut world, &mut log, &defs, &handlers);

        assert_eq!(
            world.get_component_latrine_fullness(place).unwrap().fill,
            pm(1000)
        );
        assert_eq!(
            world.get_component_place_dirtiness(place).unwrap().value,
            pm(160)
        );
        let waste_created = log.events_by_tag(EventTag::WasteCreated);
        assert_eq!(waste_created.len(), 2);
        for event in waste_created {
            let DecisionEventPayload::WasteCreated(payload) = log
                .get(*event)
                .and_then(|record| record.decision_payload())
                .expect("over-threshold toilet should emit WasteCreated payload")
            else {
                panic!("over-threshold toilet should emit WasteCreated payload");
            };
            assert_eq!(payload.source, WasteSource::OvercapacityLatrine);
            assert_eq!(payload.place_dirtiness_delta, pm(80));
        }
    }

    #[test]
    fn toilet_latrine_fullness_saturates_at_max() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::PublicLatrine);
        let actor = setup_actor_at_place(&mut world, place);
        let mut txn = new_txn(&mut world, 2);
        txn.set_component_latrine_fullness(
            place,
            LatrineFullness {
                fill: pm(950),
                fill_per_use: pm(80),
                critical_threshold: pm(800),
            },
        )
        .unwrap();
        commit_txn(txn);
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let toilet_index = toilet_affordance_index(&world, actor, &defs, &handlers);
        run_action_to_completion(actor, toilet_index, &mut world, &mut log, &defs, &handlers);

        assert_eq!(
            world.get_component_latrine_fullness(place).unwrap().fill,
            pm(1000)
        );
    }

    #[test]
    fn toilet_affordance_requires_latrine_tagged_place() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, village_square) = setup_actor(&mut world);
        let public_latrine = prototype_place_entity(PrototypePlace::PublicLatrine);
        let (defs, handlers) = setup_registries();

        let square_affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            square_affordances
                .iter()
                .all(|affordance| affordance.def_id != ActionDefId(3)),
            "toilet should not be available away from a latrine; actor_place={village_square}"
        );

        let mut txn = new_txn(&mut world, 2);
        txn.set_ground_location(actor, public_latrine).unwrap();
        commit_txn(txn);

        let latrine_affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            latrine_affordances
                .iter()
                .any(|affordance| affordance.def_id == ActionDefId(3)),
            "toilet should be available at the public latrine; actor_place={public_latrine}"
        );
    }

    #[test]
    fn wash_full_success_consumes_basin_water_and_clears_dirtiness() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        set_actor_dirtiness(&mut world, actor, pm(800));
        let (basin, source) = setup_wash_access(&mut world, place, 5);
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let affordances = affordances_for(&world, actor, &defs, &handlers);
        let wash_index = affordances
            .iter()
            .position(|affordance| affordance.def_id == ActionDefId(4))
            .unwrap();
        run_action_to_completion(actor, wash_index, &mut world, &mut log, &defs, &handlers);

        assert_eq!(
            world.get_component_wash_basin_state(basin).unwrap(),
            &WashBasinState {
                clean_water_units: 3,
                max_clean_water: 10,
                refill_per_tick: 1,
                units_per_full_wash: 2,
                dirtiness_level: pm(50),
                dirtiness_per_use: pm(50),
            }
        );
        assert_eq!(
            world
                .get_component_resource_source(source)
                .unwrap()
                .available_quantity,
            Quantity(10),
            "wash should not consume the co-located water source directly"
        );
        assert_eq!(
            world
                .get_component_homeostatic_needs(actor)
                .unwrap()
                .dirtiness,
            pm(0)
        );
        let wash_events = log.events_by_tag(EventTag::WashFacilityUsed);
        assert_eq!(wash_events.len(), 1);
        let DecisionEventPayload::WashFacilityUsed(payload) = log
            .get(wash_events[0])
            .and_then(|record| record.decision_payload())
            .expect("wash should emit WashFacilityUsed payload")
        else {
            panic!("wash should emit WashFacilityUsed payload");
        };
        assert_eq!(payload.user, actor);
        assert_eq!(payload.basin, basin);
        assert_eq!(payload.water_consumed, 2);
        assert_eq!(payload.agent_dirtiness_delta, pm(800));
        assert_eq!(payload.basin_dirtiness_delta, pm(50));
        assert!(!payload.partial);
    }

    #[test]
    fn wash_partial_success_when_basin_water_below_full_wash() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        set_actor_dirtiness(&mut world, actor, pm(800));
        let (basin, _source) = setup_wash_access(&mut world, place, 1);
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let affordances = affordances_for(&world, actor, &defs, &handlers);
        let wash_index = affordances
            .iter()
            .position(|affordance| affordance.def_id == ActionDefId(4))
            .unwrap();
        run_action_to_completion(actor, wash_index, &mut world, &mut log, &defs, &handlers);

        assert_eq!(
            world.get_component_wash_basin_state(basin).unwrap(),
            &WashBasinState {
                clean_water_units: 0,
                max_clean_water: 10,
                refill_per_tick: 1,
                units_per_full_wash: 2,
                dirtiness_level: pm(25),
                dirtiness_per_use: pm(50),
            }
        );
        assert_eq!(
            world
                .get_component_homeostatic_needs(actor)
                .unwrap()
                .dirtiness,
            pm(400)
        );
        let wash_events = log.events_by_tag(EventTag::WashFacilityUsed);
        assert_eq!(wash_events.len(), 1);
        let DecisionEventPayload::WashFacilityUsed(payload) = log
            .get(wash_events[0])
            .and_then(|record| record.decision_payload())
            .expect("wash should emit WashFacilityUsed payload")
        else {
            panic!("wash should emit WashFacilityUsed payload");
        };
        assert_eq!(payload.water_consumed, 1);
        assert_eq!(payload.agent_dirtiness_delta, pm(400));
        assert_eq!(payload.basin_dirtiness_delta, pm(25));
        assert!(payload.partial);
    }

    #[test]
    fn uncontrolled_ground_item_does_not_produce_eat_affordance() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        {
            let mut txn = new_txn(&mut world, 2);
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(bread, place).unwrap();
            commit_txn(txn);
        }
        let (defs, handlers) = setup_registries();

        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .all(|affordance| affordance.def_id != ActionDefId(0))
        );
    }

    // --- Physical access / control tests (S01PROOUTOWNCLA-010) ---

    fn eat_def_id() -> ActionDefId {
        ActionDefId(0)
    }
    fn drink_def_id() -> ActionDefId {
        ActionDefId(1)
    }
    fn wash_def_id() -> ActionDefId {
        ActionDefId(4)
    }

    #[test]
    fn eat_accepts_actor_owned_ground_lot() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        {
            let mut txn = new_txn(&mut world, 2);
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(bread, place).unwrap();
            txn.set_owner(bread, actor).unwrap();
            commit_txn(txn);
        }
        let (defs, handlers) = setup_registries();
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances.iter().any(|a| a.def_id == eat_def_id()),
            "eat should be offered for actor-controlled ground lot"
        );
    }

    #[test]
    fn eat_accepts_possessed_lot() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        {
            let mut txn = new_txn(&mut world, 2);
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(bread, place).unwrap();
            txn.set_possessor(bread, actor).unwrap();
            commit_txn(txn);
        }
        let (defs, handlers) = setup_registries();
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances.iter().any(|a| a.def_id == eat_def_id()),
            "eat should be offered for possessed lot"
        );
    }

    #[test]
    fn drink_accepts_actor_owned_ground_lot() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        {
            let mut txn = new_txn(&mut world, 2);
            let water = txn
                .create_item_lot(CommodityKind::Water, Quantity(1))
                .unwrap();
            txn.set_ground_location(water, place).unwrap();
            txn.set_owner(water, actor).unwrap();
            commit_txn(txn);
        }
        let (defs, handlers) = setup_registries();
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances.iter().any(|a| a.def_id == drink_def_id()),
            "drink should be offered for actor-controlled ground lot"
        );
    }

    #[test]
    fn drink_accepts_possessed_lot() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        {
            let mut txn = new_txn(&mut world, 2);
            let water = txn
                .create_item_lot(CommodityKind::Water, Quantity(1))
                .unwrap();
            txn.set_ground_location(water, place).unwrap();
            txn.set_possessor(water, actor).unwrap();
            commit_txn(txn);
        }
        let (defs, handlers) = setup_registries();
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances.iter().any(|a| a.def_id == drink_def_id()),
            "drink should be offered for possessed lot"
        );
    }

    #[test]
    fn wash_rejects_basin_with_zero_clean_water() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        setup_wash_access(&mut world, place, 0);
        let (defs, handlers) = setup_registries();
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances.iter().all(|a| a.def_id != wash_def_id()),
            "wash should not be offered for an empty local wash basin"
        );
    }

    #[test]
    fn wash_accepts_basin_with_sufficient_clean_water() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        let _ = setup_wash_access(&mut world, place, 1);
        let (defs, handlers) = setup_registries();
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances.iter().any(|a| a.def_id == wash_def_id()),
            "wash should be offered for a local basin with clean water"
        );
    }

    #[test]
    fn wash_race_condition_basin_emptied_between_affordance_and_start_returns_precondition_failed()
    {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        let (basin, _source) = setup_wash_access(&mut world, place, 1);
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();
        let affordance = affordances_for(&world, actor, &defs, &handlers)
            .into_iter()
            .find(|affordance| affordance.def_id == wash_def_id())
            .expect("wash affordance should exist before basin is drained");

        let mut txn = new_txn(&mut world, 2);
        let mut basin_state = txn.get_component_wash_basin_state(basin).copied().unwrap();
        basin_state.clean_water_units = 0;
        txn.set_component_wash_basin_state(basin, basin_state)
            .unwrap();
        commit_txn(txn);

        let mut active = BTreeMap::<ActionInstanceId, ActionInstance>::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = test_rng();
        let err = start_action(
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .expect_err("stale wash affordance should fail start revalidation");

        assert!(matches!(
            err,
            worldwake_sim::ActionError::PreconditionFailed(_)
        ));
        assert!(active.is_empty());
        assert!(log.events_by_tag(EventTag::ActionStarted).is_empty());
    }

    fn relieve_wilderness_def_id() -> ActionDefId {
        ActionDefId(5)
    }

    fn toilet_affordance_index(
        world: &World,
        actor: EntityId,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
    ) -> usize {
        affordances_for(world, actor, defs, handlers)
            .iter()
            .position(|affordance| affordance.def_id == ActionDefId(3))
            .expect("toilet affordance should exist at a latrine")
    }

    fn setup_actor_at_place(world: &mut World, place: EntityId) -> EntityId {
        let mut txn = new_txn(world, 1);
        let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_ground_location(actor, place).unwrap();
        txn.set_component_homeostatic_needs(
            actor,
            HomeostaticNeeds::new(pm(700), pm(650), pm(400), pm(800), pm(100)),
        )
        .unwrap();
        txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
            .unwrap();
        txn.set_component_drive_thresholds(actor, DriveThresholds::default())
            .unwrap();
        txn.set_component_metabolism_profile(
            actor,
            MetabolismProfile::new(
                pm(1),
                pm(1),
                pm(1),
                pm(1),
                pm(1),
                pm(40),
                NonZeroU32::new(10).unwrap(),
                NonZeroU32::new(10).unwrap(),
                NonZeroU32::new(10).unwrap(),
                NonZeroU32::new(10).unwrap(),
                NonZeroU32::new(2).unwrap(),
                NonZeroU32::new(3).unwrap(),
                NonZeroU32::new(8).unwrap(),
                pm(0),
                pm(0),
                pm(0),
                pm(150), // wilderness_relief_dirtiness_penalty
            ),
        )
        .unwrap();
        commit_txn(txn);
        actor
    }

    #[test]
    fn relieve_wilderness_accepts_outdoor_places() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (defs, handlers) = setup_registries();

        // ForestPath has tags Forest + Trail — both outdoor
        let forest_path = prototype_place_entity(PrototypePlace::ForestPath);
        let actor = setup_actor_at_place(&mut world, forest_path);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .any(|a| a.def_id == relieve_wilderness_def_id()),
            "relieve_wilderness should be available at ForestPath"
        );

        // EastFieldTrail has tags Trail + Field — both outdoor
        let east_field = prototype_place_entity(PrototypePlace::EastFieldTrail);
        let mut txn = new_txn(&mut world, 2);
        txn.set_ground_location(actor, east_field).unwrap();
        commit_txn(txn);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .any(|a| a.def_id == relieve_wilderness_def_id()),
            "relieve_wilderness should be available at EastFieldTrail"
        );

        // NorthCrossroads has tags Crossroads + Road — Road is outdoor
        let crossroads = prototype_place_entity(PrototypePlace::NorthCrossroads);
        let mut txn = new_txn(&mut world, 3);
        txn.set_ground_location(actor, crossroads).unwrap();
        commit_txn(txn);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .any(|a| a.def_id == relieve_wilderness_def_id()),
            "relieve_wilderness should be available at NorthCrossroads (has Road tag)"
        );

        // OrchardFarm has tags Farm + Field — both outdoor
        let farm = prototype_place_entity(PrototypePlace::OrchardFarm);
        let mut txn = new_txn(&mut world, 4);
        txn.set_ground_location(actor, farm).unwrap();
        commit_txn(txn);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .any(|a| a.def_id == relieve_wilderness_def_id()),
            "relieve_wilderness should be available at OrchardFarm"
        );
    }

    #[test]
    fn relieve_wilderness_rejects_indoor_places() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (defs, handlers) = setup_registries();

        // VillageSquare has tag Village — not outdoor
        let (actor, _) = setup_actor(&mut world);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .all(|a| a.def_id != relieve_wilderness_def_id()),
            "relieve_wilderness should not be available at VillageSquare"
        );

        // PublicLatrine has tags Latrine + Village — not outdoor
        let latrine = prototype_place_entity(PrototypePlace::PublicLatrine);
        let mut txn = new_txn(&mut world, 2);
        txn.set_ground_location(actor, latrine).unwrap();
        commit_txn(txn);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .all(|a| a.def_id != relieve_wilderness_def_id()),
            "relieve_wilderness should not be available at PublicLatrine"
        );

        // CommonHouse has tags Inn + Village — not outdoor
        let inn = prototype_place_entity(PrototypePlace::CommonHouse);
        let mut txn = new_txn(&mut world, 3);
        txn.set_ground_location(actor, inn).unwrap();
        commit_txn(txn);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .all(|a| a.def_id != relieve_wilderness_def_id()),
            "relieve_wilderness should not be available at CommonHouse"
        );

        // RulersHall has tags Hall + Village — not outdoor
        let hall = prototype_place_entity(PrototypePlace::RulersHall);
        let mut txn = new_txn(&mut world, 4);
        txn.set_ground_location(actor, hall).unwrap();
        commit_txn(txn);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .all(|a| a.def_id != relieve_wilderness_def_id()),
            "relieve_wilderness should not be available at RulersHall"
        );

        // GuardPost has tags Barracks + Village — not outdoor
        let barracks = prototype_place_entity(PrototypePlace::GuardPost);
        let mut txn = new_txn(&mut world, 5);
        txn.set_ground_location(actor, barracks).unwrap();
        commit_txn(txn);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .all(|a| a.def_id != relieve_wilderness_def_id()),
            "relieve_wilderness should not be available at GuardPost"
        );

        // GeneralStore has tags Store + Village — not outdoor
        let store = prototype_place_entity(PrototypePlace::GeneralStore);
        let mut txn = new_txn(&mut world, 6);
        txn.set_ground_location(actor, store).unwrap();
        commit_txn(txn);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .all(|a| a.def_id != relieve_wilderness_def_id()),
            "relieve_wilderness should not be available at GeneralStore"
        );
    }

    #[test]
    fn relieve_wilderness_commit_effects() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let forest_path = prototype_place_entity(PrototypePlace::ForestPath);
        let actor = setup_actor_at_place(&mut world, forest_path);
        let mut txn = new_txn(&mut world, 2);
        txn.set_component_place_dirtiness(
            forest_path,
            PlaceDirtiness {
                value: pm(0),
                dirtiness_per_use: pm(80),
                ..PlaceDirtiness::default()
            },
        )
        .unwrap();
        commit_txn(txn);
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let affordances = affordances_for(&world, actor, &defs, &handlers);
        let rw_index = affordances
            .iter()
            .position(|a| a.def_id == relieve_wilderness_def_id())
            .expect("relieve_wilderness affordance should exist at ForestPath");
        run_action_to_completion(actor, rw_index, &mut world, &mut log, &defs, &handlers);

        let needs = world.get_component_homeostatic_needs(actor).unwrap();
        // Bladder should be 0
        assert_eq!(needs.bladder, pm(0));
        // Dirtiness should be original (100) + penalty (150) = 250
        assert_eq!(needs.dirtiness, pm(250));

        // Waste entity should exist at the place
        let waste_count = world
            .ground_entities_at(forest_path)
            .into_iter()
            .filter(|entity| {
                world
                    .get_component_item_lot(*entity)
                    .is_some_and(|lot| lot.commodity == CommodityKind::Waste)
            })
            .count();
        assert_eq!(waste_count, 1);

        let place_dirtiness = world.get_component_place_dirtiness(forest_path).unwrap();
        assert_eq!(place_dirtiness.value, pm(80));

        let waste_created = log.events_by_tag(EventTag::WasteCreated);
        assert_eq!(waste_created.len(), 1);
        let DecisionEventPayload::WasteCreated(payload) = log
            .get(waste_created[0])
            .and_then(|record| record.decision_payload())
            .expect("relieve_wilderness should emit WasteCreated payload")
        else {
            panic!("relieve_wilderness should emit WasteCreated payload");
        };
        assert_eq!(payload.creator, actor);
        assert_eq!(payload.place, forest_path);
        assert_eq!(payload.source, WasteSource::WildernessRelief);
        assert_eq!(payload.place_dirtiness_delta, pm(80));
        assert!(
            world
                .get_component_item_lot(payload.waste_lot)
                .is_some_and(|lot| lot.commodity == CommodityKind::Waste),
            "WasteCreated payload should reference the created waste lot"
        );
    }

    #[test]
    fn relieve_wilderness_visibility_is_same_place() {
        let (defs, _) = setup_registries();
        let def = defs.get(relieve_wilderness_def_id()).unwrap();
        assert_eq!(def.visibility, VisibilitySpec::SamePlace);
    }

    #[test]
    fn relieve_wilderness_has_wilderness_relief_event_tag() {
        let (defs, _) = setup_registries();
        let def = defs.get(relieve_wilderness_def_id()).unwrap();
        assert!(
            def.causal_event_tags.contains(&EventTag::WildernessRelief),
            "relieve_wilderness should have WildernessRelief event tag"
        );
        assert!(
            def.causal_event_tags.contains(&EventTag::WasteCreated),
            "relieve_wilderness should have WasteCreated event tag"
        );
    }

    #[test]
    fn relieve_wilderness_place_dirtiness_saturates() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let forest_path = prototype_place_entity(PrototypePlace::ForestPath);
        let actor = setup_actor_at_place(&mut world, forest_path);
        let mut txn = new_txn(&mut world, 2);
        txn.set_component_place_dirtiness(
            forest_path,
            PlaceDirtiness {
                value: pm(950),
                dirtiness_per_use: pm(80),
                ..PlaceDirtiness::default()
            },
        )
        .unwrap();
        commit_txn(txn);
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let affordances = affordances_for(&world, actor, &defs, &handlers);
        let rw_index = affordances
            .iter()
            .position(|a| a.def_id == relieve_wilderness_def_id())
            .expect("relieve_wilderness affordance should exist at ForestPath");
        run_action_to_completion(actor, rw_index, &mut world, &mut log, &defs, &handlers);

        let place_dirtiness = world.get_component_place_dirtiness(forest_path).unwrap();
        assert_eq!(place_dirtiness.value, pm(1000));

        let waste_created = log.events_by_tag(EventTag::WasteCreated);
        assert_eq!(waste_created.len(), 1);
        let DecisionEventPayload::WasteCreated(payload) = log
            .get(waste_created[0])
            .and_then(|record| record.decision_payload())
            .expect("relieve_wilderness should emit WasteCreated payload")
        else {
            panic!("relieve_wilderness should emit WasteCreated payload");
        };
        assert_eq!(payload.source, WasteSource::WildernessRelief);
        assert_eq!(payload.place_dirtiness_delta, pm(50));
    }

    #[test]
    fn relieve_wilderness_commit_emits_scene_evidence() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let forest_path = prototype_place_entity(PrototypePlace::ForestPath);
        let actor = setup_actor_at_place(&mut world, forest_path);
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let affordances = affordances_for(&world, actor, &defs, &handlers);
        let rw_index = affordances
            .iter()
            .position(|a| a.def_id == relieve_wilderness_def_id())
            .expect("relieve_wilderness affordance should exist at ForestPath");
        run_action_to_completion(actor, rw_index, &mut world, &mut log, &defs, &handlers);

        let scene = world
            .get_component_scene_evidence(forest_path)
            .expect("wilderness relief should leave scene evidence");
        assert!(scene.evidence.iter().any(|entry| {
            matches!(
                entry.kind,
                EvidenceKind::DisturbanceMarker {
                    place,
                    kind: DisturbanceKind::WildernessRelief,
                    ..
                } if place == forest_path
            )
        }));
    }
}
