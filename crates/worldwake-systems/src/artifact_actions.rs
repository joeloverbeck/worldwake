use std::collections::BTreeSet;
use std::num::NonZeroU32;

use worldwake_core::{
    ArtifactHeader, ArtifactKind, ArtifactState, BodyCostPerTick, BountyTarget, BountyTerms,
    ContentionPolicy, ContentionQueue, EntityId, EntityKind, EventLog, EventTag, NoticeContent,
    NoticeTopic, Quantity, RewardSource, Tick, VisibilitySpec, World, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionAbortRequestReason, ActionDef, ActionDefRegistry, ActionError,
    ActionExecutionContext, ActionHandler, ActionHandlerId, ActionHandlerRegistry, ActionInstance,
    ActionPayload, ActionProgress, ActionState, CommitOutcome, Constraint, DeterministicRng,
    DurationExpr, Interruptibility, PostBountyActionPayload, PostNoticeActionPayload, Precondition,
    RuntimeBeliefView, TargetSpec,
};

pub fn register_artifact_actions(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> [worldwake_core::ActionDefId; 3] {
    let post_bounty_id = register_post_bounty_action(defs, handlers);
    let post_notice_id = register_post_notice_action(defs, handlers);
    let claim_bounty_id = register_claim_bounty_action(defs, handlers);
    [post_bounty_id, post_notice_id, claim_bounty_id]
}

pub fn register_post_bounty_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> worldwake_core::ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(
            start_post_bounty,
            tick_post_bounty,
            commit_post_bounty,
            abort_post_bounty,
        )
        .with_affordance_payloads(|_, _, _, _| Vec::new())
        .with_payload_override_validator(validate_post_bounty_payload_override)
        .with_authoritative_payload_validator(validate_post_bounty_payload_authoritatively),
    );
    let id = worldwake_core::ActionDefId(defs.len() as u32);
    defs.register(post_bounty_action_def(id, handler))
}

pub fn register_post_notice_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> worldwake_core::ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(
            start_post_notice,
            tick_post_notice,
            commit_post_notice,
            abort_post_notice,
        )
        .with_affordance_payloads(|_, _, _, _| Vec::new())
        .with_payload_override_validator(validate_post_notice_payload_override)
        .with_authoritative_payload_validator(validate_post_notice_payload_authoritatively),
    );
    let id = worldwake_core::ActionDefId(defs.len() as u32);
    defs.register(post_notice_action_def(id, handler))
}

pub fn register_claim_bounty_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> worldwake_core::ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(
            start_claim_bounty,
            tick_claim_bounty,
            commit_claim_bounty,
            abort_claim_bounty,
        )
        .with_affordance_targets(enumerate_claim_bounty_targets),
    );
    let id = worldwake_core::ActionDefId(defs.len() as u32);
    defs.register(claim_bounty_action_def(id, handler))
}

fn post_bounty_action_def(id: worldwake_core::ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "post_bounty".to_string(),
        domain: worldwake_core::ActionDomain::Social,
        actor_constraints: vec![
            Constraint::ActorAlive,
            Constraint::ActorHasControl,
            Constraint::ActorNotInTransit,
        ],
        targets: vec![TargetSpec::ActorPlace],
        preconditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Place,
            },
        ],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::Fixed(NonZeroU32::new(2).unwrap()),
        body_cost_per_tick: BodyCostPerTick::zero(),
        attention_cost: worldwake_core::Permille::ZERO,
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Place,
            },
        ],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::Social, EventTag::WorldMutation]),
        payload: ActionPayload::None,
        handler,
    }
}

fn post_notice_action_def(id: worldwake_core::ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "post_notice".to_string(),
        domain: worldwake_core::ActionDomain::Social,
        actor_constraints: vec![
            Constraint::ActorAlive,
            Constraint::ActorHasControl,
            Constraint::ActorNotInTransit,
        ],
        targets: vec![TargetSpec::ActorPlace],
        preconditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Place,
            },
        ],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::Fixed(NonZeroU32::MIN),
        body_cost_per_tick: BodyCostPerTick::zero(),
        attention_cost: worldwake_core::Permille::ZERO,
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Place,
            },
        ],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::Social, EventTag::WorldMutation]),
        payload: ActionPayload::None,
        handler,
    }
}

fn claim_bounty_action_def(id: worldwake_core::ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "claim_bounty".to_string(),
        domain: worldwake_core::ActionDomain::Social,
        actor_constraints: vec![
            Constraint::ActorAlive,
            Constraint::ActorHasControl,
            Constraint::ActorNotInTransit,
        ],
        targets: vec![TargetSpec::SpecificEntity(EntityId {
            slot: 0,
            generation: 0,
        })],
        preconditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::SocialArtifact,
            },
        ],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::Fixed(NonZeroU32::new(2).unwrap()),
        body_cost_per_tick: BodyCostPerTick::zero(),
        attention_cost: worldwake_core::Permille::ZERO,
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::SocialArtifact,
            },
        ],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([
            EventTag::Social,
            EventTag::Transfer,
            EventTag::WorldMutation,
        ]),
        payload: ActionPayload::None,
        handler,
    }
}

fn bounty_claim_contention_policy() -> ContentionPolicy {
    ContentionPolicy {
        grant_hold_ticks: NonZeroU32::new(3).unwrap(),
        auto_promote: false,
        max_waiters: Some(0),
    }
}

fn post_bounty_payload<'a>(
    def: &ActionDef,
    payload: &'a ActionPayload,
) -> Result<&'a PostBountyActionPayload, ActionError> {
    payload.as_post_bounty().ok_or_else(|| {
        ActionError::PreconditionFailed(format!(
            "action def {} requires PostBounty payload",
            def.id
        ))
    })
}

fn post_notice_payload<'a>(
    def: &ActionDef,
    payload: &'a ActionPayload,
) -> Result<&'a PostNoticeActionPayload, ActionError> {
    payload.as_post_notice().ok_or_else(|| {
        ActionError::PreconditionFailed(format!(
            "action def {} requires PostNotice payload",
            def.id
        ))
    })
}

fn posting_place_from_instance(instance: &ActionInstance) -> Result<EntityId, ActionError> {
    instance
        .targets
        .first()
        .copied()
        .ok_or(ActionError::InvalidTarget(instance.actor))
}

fn artifact_target_from_instance(instance: &ActionInstance) -> Result<EntityId, ActionError> {
    instance
        .targets
        .first()
        .copied()
        .ok_or(ActionError::InvalidTarget(instance.actor))
}

fn validate_target_place(world: &World, place: EntityId) -> Result<(), ActionError> {
    if world.entity_kind(place) != Some(EntityKind::Place) {
        return Err(ActionError::PreconditionFailed(format!(
            "target place {place} is not a place"
        )));
    }
    Ok(())
}

fn validate_expiration_tick(
    current_tick: Tick,
    expires_at: Option<Tick>,
) -> Result<(), ActionError> {
    if expires_at.is_some_and(|expires_at| expires_at <= current_tick) {
        return Err(ActionError::PreconditionFailed(format!(
            "artifact expiration tick {expires_at:?} must be after current tick {current_tick}"
        )));
    }
    Ok(())
}

fn validate_bounty_target(world: &World, target: BountyTarget) -> Result<(), ActionError> {
    match target {
        BountyTarget::EliminateEntity { target } => {
            if world.entity_kind(target).is_none() {
                return Err(ActionError::PreconditionFailed(format!(
                    "bounty target entity {target} does not exist"
                )));
            }
        }
        BountyTarget::DeliverCommodity { destination, .. } => {
            validate_target_place(world, destination)?;
        }
    }
    Ok(())
}

fn validate_posting_places(
    world: &World,
    actor: EntityId,
    posting_place: EntityId,
    claim_place: Option<EntityId>,
) -> Result<(), ActionError> {
    validate_target_place(world, posting_place)?;
    if world.effective_place(actor) != Some(posting_place) {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} is not co-located with posting place {posting_place}"
        )));
    }
    if let Some(claim_place) = claim_place {
        validate_target_place(world, claim_place)?;
    }
    Ok(())
}

fn validate_institutional_authority(
    world: &World,
    actor: EntityId,
    treasury_entity: EntityId,
) -> Result<(), ActionError> {
    match world.entity_kind(treasury_entity) {
        Some(EntityKind::Office) => {
            if world.office_holder(treasury_entity) != Some(actor) {
                return Err(ActionError::PreconditionFailed(format!(
                    "actor {actor} is not the holder of office treasury {treasury_entity}"
                )));
            }
        }
        Some(EntityKind::Faction) => {
            if !world.factions_of(actor).contains(&treasury_entity) {
                return Err(ActionError::PreconditionFailed(format!(
                    "actor {actor} is not a member of faction treasury {treasury_entity}"
                )));
            }
        }
        Some(kind) => {
            return Err(ActionError::PreconditionFailed(format!(
                "institutional treasury entity {treasury_entity} has unsupported kind {kind:?}"
            )));
        }
        None => {
            return Err(ActionError::PreconditionFailed(format!(
                "institutional treasury entity {treasury_entity} does not exist"
            )));
        }
    }

    Ok(())
}

fn validate_reward_source(
    world: &World,
    actor: EntityId,
    payload: &PostBountyActionPayload,
) -> Result<(), ActionError> {
    if payload.reward_quantity == Quantity(0) {
        return Err(ActionError::PreconditionFailed(
            "bounty reward quantity must be greater than zero".to_string(),
        ));
    }

    match payload.reward_source {
        RewardSource::InstitutionalTreasury { treasury_entity } => {
            validate_institutional_authority(world, actor, treasury_entity)?;
            if world.controlled_commodity_quantity(treasury_entity, payload.reward_commodity)
                < payload.reward_quantity
            {
                return Err(ActionError::PreconditionFailed(format!(
                    "institutional treasury {treasury_entity} lacks {:?} x{}",
                    payload.reward_commodity, payload.reward_quantity.0
                )));
            }
        }
        RewardSource::PersonalFunds { issuer } => {
            if issuer != actor {
                return Err(ActionError::PreconditionFailed(format!(
                    "personal-funds bounty issuer {issuer} does not match actor {actor}"
                )));
            }
            if world.controlled_commodity_quantity(actor, payload.reward_commodity)
                < payload.reward_quantity
            {
                return Err(ActionError::PreconditionFailed(format!(
                    "actor {actor} lacks {:?} x{} for personal-funds bounty",
                    payload.reward_commodity, payload.reward_quantity.0
                )));
            }
        }
        RewardSource::ReservedLot { lot } => {
            let Some(item_lot) = world.get_component_item_lot(lot) else {
                return Err(ActionError::PreconditionFailed(format!(
                    "reserved reward lot {lot} is not an item lot"
                )));
            };
            if world.can_exercise_control(actor, lot).is_err() {
                return Err(ActionError::PreconditionFailed(format!(
                    "actor {actor} cannot control reserved reward lot {lot}"
                )));
            }
            if item_lot.commodity != payload.reward_commodity {
                return Err(ActionError::PreconditionFailed(format!(
                    "reserved reward lot {lot} commodity {:?} does not match promised {:?}",
                    item_lot.commodity, payload.reward_commodity
                )));
            }
            if item_lot.quantity < payload.reward_quantity {
                return Err(ActionError::PreconditionFailed(format!(
                    "reserved reward lot {lot} lacks quantity {}",
                    payload.reward_quantity.0
                )));
            }
        }
    }

    Ok(())
}

fn validate_post_bounty_context(
    world: &World,
    actor: EntityId,
    posting_place: EntityId,
    payload: &PostBountyActionPayload,
    current_tick: Tick,
) -> Result<(), ActionError> {
    if payload.posting_place != posting_place {
        return Err(ActionError::PreconditionFailed(format!(
            "payload posting_place {} does not match bound target {}",
            payload.posting_place, posting_place
        )));
    }
    validate_posting_places(world, actor, posting_place, Some(payload.claim_place))?;
    validate_expiration_tick(current_tick, payload.expires_at)?;
    validate_bounty_target(world, payload.target)?;
    validate_reward_source(world, actor, payload)?;
    if let Some(authority) = payload.issuing_authority
        && world.entity_kind(authority).is_none()
    {
        return Err(ActionError::PreconditionFailed(format!(
            "issuing authority {authority} does not exist"
        )));
    }
    if let Some(jurisdiction) = payload.jurisdiction
        && world.entity_kind(jurisdiction).is_none()
    {
        return Err(ActionError::PreconditionFailed(format!(
            "jurisdiction entity {jurisdiction} does not exist"
        )));
    }
    Ok(())
}

fn validate_post_notice_context(
    world: &World,
    actor: EntityId,
    posting_place: EntityId,
    payload: &PostNoticeActionPayload,
    current_tick: Tick,
) -> Result<(), ActionError> {
    if payload.posting_place != posting_place {
        return Err(ActionError::PreconditionFailed(format!(
            "payload posting_place {} does not match bound target {}",
            payload.posting_place, posting_place
        )));
    }
    validate_posting_places(world, actor, posting_place, None)?;
    validate_expiration_tick(current_tick, payload.expires_at)?;
    match payload.topic {
        NoticeTopic::ThreatWarning { place } | NoticeTopic::CommodityShortage { place, .. } => {
            validate_target_place(world, place)?;
        }
        NoticeTopic::OfficeVacancy { office } => {
            if world.entity_kind(office) != Some(EntityKind::Office) {
                return Err(ActionError::PreconditionFailed(format!(
                    "office vacancy target {office} is not an office"
                )));
            }
        }
        NoticeTopic::Institutional { .. } => {}
    }
    if let Some(authority) = payload.issuing_authority
        && world.entity_kind(authority).is_none()
    {
        return Err(ActionError::PreconditionFailed(format!(
            "issuing authority {authority} does not exist"
        )));
    }
    if let Some(jurisdiction) = payload.jurisdiction
        && world.entity_kind(jurisdiction).is_none()
    {
        return Err(ActionError::PreconditionFailed(format!(
            "jurisdiction entity {jurisdiction} does not exist"
        )));
    }
    Ok(())
}

fn enumerate_claim_bounty_targets(
    _def: &ActionDef,
    actor: EntityId,
    view: &dyn RuntimeBeliefView,
) -> Vec<Vec<EntityId>> {
    let Some(actor_place) = view.effective_place(actor) else {
        return Vec::new();
    };
    let mut targets = view
        .known_entity_beliefs(actor)
        .into_iter()
        .filter_map(|(entity, belief)| {
            let artifact = belief.believed_artifact?;
            let terms = artifact.bounty_terms?;
            (artifact.kind == ArtifactKind::Bounty
                && artifact.state == ArtifactState::Active
                && terms.claim_place == actor_place)
                .then_some(vec![entity])
        })
        .collect::<Vec<_>>();
    targets.sort();
    targets.dedup();
    targets
}

fn validate_bounty_claim_target(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    target: EntityId,
) -> Result<(EntityId, ArtifactHeader, BountyTerms), ActionError> {
    if txn.entity_kind(target) != Some(EntityKind::SocialArtifact) {
        return Err(ActionError::InvalidTarget(target));
    }
    let actor_place = txn.effective_place(actor).ok_or_else(|| {
        ActionError::PreconditionFailed(format!("actor {actor} has no effective place"))
    })?;
    let header = txn
        .get_component_artifact_header(target)
        .cloned()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!("artifact {target} lacks header"))
        })?;
    if header.kind != ArtifactKind::Bounty {
        return Err(ActionError::PreconditionFailed(format!(
            "artifact {target} is not a bounty"
        )));
    }
    if header.state != ArtifactState::Active {
        return Err(ActionError::PreconditionFailed(format!(
            "artifact {target} is not active"
        )));
    }
    let terms = txn
        .get_component_bounty_terms(target)
        .cloned()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!("artifact {target} lacks bounty terms"))
        })?;
    if actor_place != terms.claim_place {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} is not at claim place {} for bounty {target}",
            terms.claim_place
        )));
    }
    Ok((actor_place, header, terms))
}

fn validate_bounty_target_satisfied(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    terms: &BountyTerms,
) -> Result<(), ActionError> {
    match terms.target {
        BountyTarget::EliminateEntity { target } => {
            if txn.get_component_dead_at(target).is_none() {
                return Err(ActionError::PreconditionFailed(format!(
                    "bounty target {target} is not dead"
                )));
            }
        }
        BountyTarget::DeliverCommodity {
            commodity,
            quantity,
            destination,
        } => {
            if txn.controlled_commodity_quantity_at_place(actor, destination, commodity) < quantity
            {
                return Err(ActionError::PreconditionFailed(format!(
                    "actor {actor} has not delivered {:?} x{} to destination {destination}",
                    commodity, quantity.0
                )));
            }
        }
    }
    Ok(())
}

fn has_qualifying_bounty_testimony(txn: &WorldTxn<'_>, actor: EntityId, target: EntityId) -> bool {
    txn.get_component_agent_belief_store(actor)
        .is_some_and(|store| {
            store.social_observations.iter().any(|observation| {
                matches!(
                    observation.detail,
                    worldwake_core::SocialObservationDetail::WitnessedConflict {
                        actor: observed,
                        ..
                    } if observed == target
                ) || matches!(
                    observation.detail,
                    worldwake_core::SocialObservationDetail::WitnessedConflict {
                        target: observed,
                        ..
                    } if observed == target
                ) || matches!(
                    observation.detail,
                    worldwake_core::SocialObservationDetail::WitnessedAbsence {
                        missing_entity: observed,
                        ..
                    } if observed == target
                )
            })
        })
}

fn validate_bounty_claim_proof(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    actor_place: EntityId,
    terms: &BountyTerms,
) -> Result<(), ActionError> {
    match terms.proof_requirement {
        worldwake_core::ProofRequirement::SelfReport => Ok(()),
        worldwake_core::ProofRequirement::PhysicalEvidence => match terms.target {
            BountyTarget::EliminateEntity { target } => {
                if txn.effective_place(target) != Some(actor_place) {
                    return Err(ActionError::PreconditionFailed(format!(
                        "insufficient proof: target {target} is not present at claim place {actor_place}"
                    )));
                }
                Ok(())
            }
            BountyTarget::DeliverCommodity {
                commodity,
                quantity,
                destination,
            } => {
                if txn.controlled_commodity_quantity_at_place(actor, destination, commodity)
                    < quantity
                {
                    return Err(ActionError::PreconditionFailed(format!(
                        "insufficient proof: actor {actor} lacks delivered {:?} x{} at {destination}",
                        commodity, quantity.0
                    )));
                }
                Ok(())
            }
        },
        worldwake_core::ProofRequirement::WitnessTestimony => match terms.target {
            BountyTarget::EliminateEntity { target } => {
                if has_qualifying_bounty_testimony(txn, actor, target) {
                    Ok(())
                } else {
                    Err(ActionError::PreconditionFailed(
                        "insufficient proof".to_string(),
                    ))
                }
            }
            BountyTarget::DeliverCommodity { .. } => Err(ActionError::PreconditionFailed(
                "insufficient proof".to_string(),
            )),
        },
    }
}

fn ensure_bounty_claim_contention_components(
    txn: &mut WorldTxn<'_>,
    target: EntityId,
) -> Result<(), ActionError> {
    if txn.entity_kind(target) != Some(EntityKind::SocialArtifact) {
        return Err(ActionError::InvalidTarget(target));
    }
    match (
        txn.get_component_contention_policy(target),
        txn.get_component_contention_queue(target),
    ) {
        (Some(_), Some(_)) => Ok(()),
        (Some(_), None) => Err(ActionError::PreconditionFailed(format!(
            "bounty {target} lacks ContentionQueue"
        ))),
        (None, Some(_)) => Err(ActionError::PreconditionFailed(format!(
            "bounty {target} lacks ContentionPolicy"
        ))),
        (None, None) => {
            txn.set_component_contention_policy(target, bounty_claim_contention_policy())
                .map_err(|err| ActionError::InternalError(err.to_string()))?;
            txn.set_component_contention_queue(target, ContentionQueue::default())
                .map_err(|err| ActionError::InternalError(err.to_string()))
        }
    }
}

fn claim_or_require_bounty_grant(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    target: EntityId,
    action_def: worldwake_core::ActionDefId,
    claim_if_absent: bool,
) -> Result<(), ActionError> {
    ensure_bounty_claim_contention_components(txn, target)?;
    let policy = txn
        .get_component_contention_policy(target)
        .cloned()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!("bounty {target} lacks ContentionPolicy"))
        })?;
    let mut queue = txn
        .get_component_contention_queue(target)
        .cloned()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!("bounty {target} lacks ContentionQueue"))
        })?;

    match queue.granted.as_ref() {
        Some(granted) if granted.actor == actor && granted.intended_action == action_def => {
            return Ok(());
        }
        Some(_) => {
            return Err(ActionError::PreconditionFailed(
                "contention_rejected".to_string(),
            ));
        }
        None if !claim_if_absent => {
            return Err(ActionError::PreconditionFailed(
                "contention_rejected".to_string(),
            ));
        }
        None => {}
    }

    queue.granted = Some(worldwake_core::ContentionGrant {
        actor,
        intended_action: action_def,
        granted_at: txn.tick(),
        expires_at: txn.tick() + u64::from(policy.grant_hold_ticks.get()),
    });
    txn.set_component_contention_queue(target, queue)
        .map_err(|err| ActionError::InternalError(err.to_string()))
}

fn clear_bounty_grant(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    target: EntityId,
    action_def: worldwake_core::ActionDefId,
) -> Result<(), ActionError> {
    let Some(mut queue) = txn.get_component_contention_queue(target).cloned() else {
        return Ok(());
    };
    if queue
        .granted
        .as_ref()
        .is_some_and(|granted| granted.actor == actor && granted.intended_action == action_def)
    {
        queue.clear_grant();
        txn.set_component_contention_queue(target, queue)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    Ok(())
}

fn ensure_accessible_quantity(
    txn: &WorldTxn<'_>,
    holder: EntityId,
    commodity: worldwake_core::CommodityKind,
    quantity: Quantity,
) -> Result<(), ActionError> {
    let available = txn.controlled_commodity_quantity(holder, commodity);
    if available < quantity {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::HolderLacksAccessibleCommodity {
                holder,
                commodity,
                quantity,
            },
        ));
    }
    Ok(())
}

fn resolve_controlled_lots(
    txn: &mut WorldTxn<'_>,
    holder: EntityId,
    commodity: worldwake_core::CommodityKind,
    quantity: Quantity,
    place: EntityId,
) -> Result<Vec<(EntityId, Quantity)>, ActionError> {
    let mut remaining = quantity;
    let mut selected = Vec::new();
    let mut lots = txn
        .query_item_lot()
        .filter_map(|(entity, lot)| {
            (lot.commodity == commodity
                && txn.can_exercise_control(holder, entity).is_ok()
                && txn.effective_place(entity) == Some(place))
            .then_some((entity, lot.quantity))
        })
        .collect::<Vec<_>>();
    lots.sort_by_key(|(entity, _)| *entity);

    for (lot_id, available) in lots {
        if remaining == Quantity(0) {
            break;
        }
        if available > remaining {
            let (_, split_off) = txn
                .split_lot(lot_id, remaining)
                .map_err(|err| ActionError::InternalError(err.to_string()))?;
            selected.push((split_off, remaining));
            remaining = Quantity(0);
            break;
        }
        selected.push((lot_id, available));
        remaining = remaining.checked_sub(available).ok_or_else(|| {
            ActionError::InternalError("bounty reward accounting underflowed".to_string())
        })?;
    }

    if remaining != Quantity(0) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::HolderLacksAccessibleCommodity {
                holder,
                commodity,
                quantity,
            },
        ));
    }
    Ok(selected)
}

fn transfer_lot_to_holder(
    txn: &mut WorldTxn<'_>,
    lot_id: EntityId,
    new_holder: EntityId,
    place: EntityId,
    quantity: Quantity,
) -> Result<(), ActionError> {
    if txn.direct_container(lot_id).is_some() {
        txn.remove_from_container(lot_id)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    if txn.possessor_of(lot_id).is_some() {
        txn.clear_possessor(lot_id)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    if txn.effective_place(lot_id) != Some(place) {
        txn.set_ground_location(lot_id, place)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    txn.set_owner(lot_id, new_holder)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.set_possessor(lot_id, new_holder)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.append_transfer_provenance(lot_id, quantity)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.add_target(lot_id);
    Ok(())
}

fn transfer_controlled_commodity(
    txn: &mut WorldTxn<'_>,
    holder: EntityId,
    new_holder: EntityId,
    commodity: worldwake_core::CommodityKind,
    quantity: Quantity,
    place: EntityId,
) -> Result<(), ActionError> {
    ensure_accessible_quantity(txn, holder, commodity, quantity)?;
    for (lot_id, moved_quantity) in
        resolve_controlled_lots(txn, holder, commodity, quantity, place)?
    {
        transfer_lot_to_holder(txn, lot_id, new_holder, place, moved_quantity)?;
    }
    Ok(())
}

fn transfer_reserved_reward_lot(
    txn: &mut WorldTxn<'_>,
    lot: EntityId,
    new_holder: EntityId,
    place: EntityId,
    commodity: worldwake_core::CommodityKind,
    quantity: Quantity,
) -> Result<(), ActionError> {
    let item_lot = txn.get_component_item_lot(lot).cloned().ok_or_else(|| {
        ActionError::PreconditionFailed(format!("reserved reward lot {lot} is not an item lot"))
    })?;
    if item_lot.commodity != commodity {
        return Err(ActionError::PreconditionFailed(format!(
            "reserved reward lot {lot} commodity {:?} does not match promised {:?}",
            item_lot.commodity, commodity
        )));
    }
    if item_lot.quantity < quantity {
        return Err(ActionError::PreconditionFailed(format!(
            "reserved reward lot {lot} lacks quantity {}",
            quantity.0
        )));
    }
    let moved_lot = if item_lot.quantity > quantity {
        let (_, split_off) = txn
            .split_lot(lot, quantity)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
        split_off
    } else {
        lot
    };
    transfer_lot_to_holder(txn, moved_lot, new_holder, place, quantity)
}

fn validate_post_bounty_payload_override(
    _def: &ActionDef,
    _actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn RuntimeBeliefView,
) -> bool {
    let Some(payload) = payload.as_post_bounty() else {
        return false;
    };
    let Some(posting_place) = targets.first().copied() else {
        return false;
    };
    posting_place == payload.posting_place
        && view.entity_kind(posting_place) == Some(EntityKind::Place)
        && view.entity_kind(payload.claim_place) == Some(EntityKind::Place)
}

fn validate_post_notice_payload_override(
    _def: &ActionDef,
    _actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn RuntimeBeliefView,
) -> bool {
    let Some(payload) = payload.as_post_notice() else {
        return false;
    };
    let Some(posting_place) = targets.first().copied() else {
        return false;
    };
    posting_place == payload.posting_place
        && view.entity_kind(posting_place) == Some(EntityKind::Place)
}

fn validate_post_bounty_payload_authoritatively(
    def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let payload = post_bounty_payload(def, payload)?;
    let posting_place = *targets.first().ok_or(ActionError::InvalidTarget(actor))?;
    validate_post_bounty_context(world, actor, posting_place, payload, Tick(0))
}

fn validate_post_notice_payload_authoritatively(
    def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let payload = post_notice_payload(def, payload)?;
    let posting_place = *targets.first().ok_or(ActionError::InvalidTarget(actor))?;
    validate_post_notice_context(world, actor, posting_place, payload, Tick(0))
}

fn start_post_bounty(
    def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let payload = post_bounty_payload(def, &instance.payload)?;
    let posting_place = posting_place_from_instance(instance)?;
    validate_post_bounty_context(txn, instance.actor, posting_place, payload, txn.tick())?;
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_post_bounty(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _context: &ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_post_bounty(
    def: &ActionDef,
    instance: &ActionInstance,
    _context: &ActionExecutionContext<'_>,
    _event_log: &EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = post_bounty_payload(def, &instance.payload)?;
    let posting_place = posting_place_from_instance(instance)?;
    validate_post_bounty_context(txn, instance.actor, posting_place, payload, txn.tick())?;

    let artifact = txn.create_entity(EntityKind::SocialArtifact);
    txn.set_component_artifact_header(
        artifact,
        ArtifactHeader {
            kind: ArtifactKind::Bounty,
            issuer: instance.actor,
            issuing_authority: payload.issuing_authority,
            created_at: txn.tick(),
            expires_at: payload.expires_at,
            state: ArtifactState::Active,
            jurisdiction: payload.jurisdiction,
        },
    )
    .map_err(|error| ActionError::InternalError(error.to_string()))?;
    txn.set_component_bounty_terms(
        artifact,
        BountyTerms {
            target: payload.target,
            proof_requirement: payload.proof_requirement,
            reward_commodity: payload.reward_commodity,
            reward_quantity: payload.reward_quantity,
            reward_source: payload.reward_source,
            claim_place: payload.claim_place,
        },
    )
    .map_err(|error| ActionError::InternalError(error.to_string()))?;
    txn.set_ground_location(artifact, posting_place)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    txn.set_component_contention_policy(artifact, bounty_claim_contention_policy())
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    txn.set_component_contention_queue(artifact, ContentionQueue::default())
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    let mut tracker = txn
        .get_component_obligation_execution_tracker(instance.actor)
        .cloned()
        .unwrap_or_default();
    tracker.completion_ticks.push(txn.tick());
    txn.set_component_obligation_execution_tracker(instance.actor, tracker)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    txn.add_target(artifact).add_target(posting_place);

    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_post_bounty(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _context: &ActionExecutionContext<'_>,
    _reason: &AbortReason,
    _event_log: &EventLog,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

fn start_post_notice(
    def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let payload = post_notice_payload(def, &instance.payload)?;
    let posting_place = posting_place_from_instance(instance)?;
    validate_post_notice_context(txn, instance.actor, posting_place, payload, txn.tick())?;
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_post_notice(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _context: &ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_post_notice(
    def: &ActionDef,
    instance: &ActionInstance,
    _context: &ActionExecutionContext<'_>,
    _event_log: &EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = post_notice_payload(def, &instance.payload)?;
    let posting_place = posting_place_from_instance(instance)?;
    validate_post_notice_context(txn, instance.actor, posting_place, payload, txn.tick())?;

    let artifact = txn.create_entity(EntityKind::SocialArtifact);
    txn.set_component_artifact_header(
        artifact,
        ArtifactHeader {
            kind: ArtifactKind::Notice,
            issuer: instance.actor,
            issuing_authority: payload.issuing_authority,
            created_at: txn.tick(),
            expires_at: payload.expires_at,
            state: ArtifactState::Active,
            jurisdiction: payload.jurisdiction,
        },
    )
    .map_err(|error| ActionError::InternalError(error.to_string()))?;
    txn.set_component_notice_content(
        artifact,
        NoticeContent {
            topic: payload.topic,
        },
    )
    .map_err(|error| ActionError::InternalError(error.to_string()))?;
    txn.set_ground_location(artifact, posting_place)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    let mut tracker = txn
        .get_component_obligation_execution_tracker(instance.actor)
        .cloned()
        .unwrap_or_default();
    tracker.completion_ticks.push(txn.tick());
    txn.set_component_obligation_execution_tracker(instance.actor, tracker)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    txn.add_target(artifact).add_target(posting_place);

    Ok(CommitOutcome::empty())
}

fn start_claim_bounty(
    def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let target = artifact_target_from_instance(instance)?;
    let (actor_place, _header, terms) = validate_bounty_claim_target(txn, instance.actor, target)?;
    validate_bounty_target_satisfied(txn, instance.actor, &terms)?;
    validate_bounty_claim_proof(txn, instance.actor, actor_place, &terms)?;
    claim_or_require_bounty_grant(txn, instance.actor, target, def.id, true)?;
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_claim_bounty(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _context: &ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_claim_bounty(
    def: &ActionDef,
    instance: &ActionInstance,
    _context: &ActionExecutionContext<'_>,
    _event_log: &EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let target = artifact_target_from_instance(instance)?;
    let (actor_place, mut header, terms) =
        validate_bounty_claim_target(txn, instance.actor, target)?;
    validate_bounty_target_satisfied(txn, instance.actor, &terms)?;
    validate_bounty_claim_proof(txn, instance.actor, actor_place, &terms)?;
    claim_or_require_bounty_grant(txn, instance.actor, target, def.id, false)?;

    match terms.reward_source {
        RewardSource::InstitutionalTreasury { treasury_entity } => transfer_controlled_commodity(
            txn,
            treasury_entity,
            instance.actor,
            terms.reward_commodity,
            terms.reward_quantity,
            actor_place,
        )?,
        RewardSource::PersonalFunds { issuer } => transfer_controlled_commodity(
            txn,
            issuer,
            instance.actor,
            terms.reward_commodity,
            terms.reward_quantity,
            actor_place,
        )?,
        RewardSource::ReservedLot { lot } => transfer_reserved_reward_lot(
            txn,
            lot,
            instance.actor,
            actor_place,
            terms.reward_commodity,
            terms.reward_quantity,
        )?,
    }

    header.state = ArtifactState::Fulfilled;
    txn.set_component_artifact_header(target, header)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    clear_bounty_grant(txn, instance.actor, target, def.id)?;
    txn.add_target(target);
    Ok(CommitOutcome::empty())
}

fn abort_claim_bounty(
    def: &ActionDef,
    instance: &ActionInstance,
    _context: &ActionExecutionContext<'_>,
    _reason: &AbortReason,
    _event_log: &EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    if let Some(target) = instance.targets.first().copied() {
        clear_bounty_grant(txn, instance.actor, target, def.id)?;
    }
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_post_notice(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _context: &ActionExecutionContext<'_>,
    _reason: &AbortReason,
    _event_log: &EventLog,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{register_artifact_actions, register_post_bounty_action};
    use std::collections::BTreeMap;
    use worldwake_core::{
        AgentBeliefStore, BelievedArtifactState, BelievedBountyTerms, BelievedEntityState,
        CauseRef, CommodityKind, ContentionQueue, ControlSource, DeadAt, EventLog, EventTag,
        ObligationExecutionTracker, PerceptionSource, ProofRequirement, PrototypePlace, Quantity,
        Seed, Tick, VisibilitySpec, WitnessData, World, WorldTxn, build_prototype_world,
        prototype_place_entity,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionError, ActionExecutionAuthority, ActionExecutionContext,
        ActionHandlerRegistry, ActionInstanceId, ActionPayload, Affordance, DeterministicRng,
        PerAgentBeliefView, PostBountyActionPayload, PostNoticeActionPayload, TickOutcome,
        start_action, tick_action,
    };

    use super::*;

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

    fn spawn_agent_at(world: &mut World, name: &str, place: EntityId) -> EntityId {
        let mut txn = new_txn(world, 1);
        let actor = txn.create_agent(name, ControlSource::Ai).unwrap();
        txn.set_ground_location(actor, place).unwrap();
        commit_txn(txn);
        actor
    }

    fn grant_personal_funds(
        world: &mut World,
        owner: EntityId,
        place: EntityId,
        commodity: CommodityKind,
        quantity: u16,
    ) {
        let mut txn = new_txn(world, 2);
        let lot = txn
            .create_item_lot(commodity, Quantity(u32::from(quantity)))
            .expect("create funding lot");
        txn.set_ground_location(lot, place).unwrap();
        txn.set_owner(lot, owner).unwrap();
        commit_txn(txn);
    }

    fn kill_entity(world: &mut World, entity: EntityId, tick: u64) {
        let mut txn = new_txn(world, tick);
        txn.set_component_dead_at(
            entity,
            DeadAt {
                tick: Tick(tick),
                cause: worldwake_core::DeathCause::CombatWounds,
            },
        )
        .unwrap();
        commit_txn(txn);
    }

    fn create_bounty_artifact(
        world: &mut World,
        issuer: EntityId,
        posting_place: EntityId,
        claim_place: EntityId,
        target: EntityId,
        reward_source: RewardSource,
        proof_requirement: ProofRequirement,
    ) -> EntityId {
        let mut txn = new_txn(world, 3);
        let artifact = txn.create_entity(EntityKind::SocialArtifact);
        txn.set_component_artifact_header(
            artifact,
            ArtifactHeader {
                kind: ArtifactKind::Bounty,
                issuer,
                issuing_authority: None,
                created_at: Tick(3),
                expires_at: Some(Tick(12)),
                state: ArtifactState::Active,
                jurisdiction: None,
            },
        )
        .unwrap();
        txn.set_component_bounty_terms(
            artifact,
            BountyTerms {
                target: BountyTarget::EliminateEntity { target },
                proof_requirement,
                reward_commodity: CommodityKind::Coin,
                reward_quantity: Quantity(4),
                reward_source,
                claim_place,
            },
        )
        .unwrap();
        txn.set_ground_location(artifact, posting_place).unwrap();
        txn.set_component_contention_policy(artifact, bounty_claim_contention_policy())
            .unwrap();
        txn.set_component_contention_queue(artifact, ContentionQueue::default())
            .unwrap();
        commit_txn(txn);
        artifact
    }

    fn sum_commodity(world: &World, commodity: CommodityKind) -> Quantity {
        Quantity(
            world
                .query_item_lot()
                .filter_map(|(_, lot)| (lot.commodity == commodity).then_some(lot.quantity.0))
                .sum(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn tick_to_completion(
        world: &mut World,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        active: &mut BTreeMap<ActionInstanceId, worldwake_sim::ActionInstance>,
        log: &mut EventLog,
        rng: &mut DeterministicRng,
        action_id: ActionInstanceId,
        first_tick: u64,
    ) -> Result<TickOutcome, ActionError> {
        let first = tick_action(
            action_id,
            defs,
            handlers,
            ActionExecutionAuthority {
                active_actions: active,
                world,
                event_log: log,
                rng,
            },
            ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(first_tick)),
        )?;
        assert!(matches!(first, TickOutcome::Continuing));
        tick_action(
            action_id,
            defs,
            handlers,
            ActionExecutionAuthority {
                active_actions: active,
                world,
                event_log: log,
                rng,
            },
            ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(first_tick + 1)),
        )
    }

    fn post_bounty_payload(
        posting_place: EntityId,
        claim_place: EntityId,
        target: EntityId,
        issuer: EntityId,
    ) -> ActionPayload {
        ActionPayload::PostBounty(PostBountyActionPayload {
            posting_place,
            issuing_authority: None,
            expires_at: Some(Tick(12)),
            jurisdiction: None,
            target: BountyTarget::EliminateEntity { target },
            proof_requirement: ProofRequirement::PhysicalEvidence,
            reward_commodity: CommodityKind::Coin,
            reward_quantity: Quantity(4),
            reward_source: RewardSource::PersonalFunds { issuer },
            claim_place,
        })
    }

    fn post_notice_payload(posting_place: EntityId, topic_place: EntityId) -> ActionPayload {
        ActionPayload::PostNotice(PostNoticeActionPayload {
            posting_place,
            issuing_authority: None,
            expires_at: Some(Tick(14)),
            jurisdiction: None,
            topic: NoticeTopic::ThreatWarning { place: topic_place },
        })
    }

    #[test]
    fn register_artifact_actions_creates_expected_definitions() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let ids = register_artifact_actions(&mut defs, &mut handlers);

        assert_eq!(handlers.len(), 3);
        assert_eq!(ids.len(), 3);
        assert_eq!(defs.get(ids[0]).unwrap().name, "post_bounty");
        assert_eq!(defs.get(ids[1]).unwrap().name, "post_notice");
        assert_eq!(defs.get(ids[2]).unwrap().name, "claim_bounty");
        assert_eq!(
            defs.get(ids[0]).unwrap().targets,
            vec![TargetSpec::ActorPlace]
        );
        assert_eq!(
            defs.get(ids[1]).unwrap().targets,
            vec![TargetSpec::ActorPlace]
        );
        assert_eq!(
            defs.get(ids[2]).unwrap().targets,
            vec![TargetSpec::SpecificEntity(EntityId {
                slot: 0,
                generation: 0
            })]
        );
    }

    #[test]
    fn post_bounty_commits_social_artifact_with_contention_components() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let actor = spawn_agent_at(&mut world, "issuer", square);
        let target = spawn_agent_at(&mut world, "target", square);
        grant_personal_funds(&mut world, actor, square, CommodityKind::Coin, 4);

        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let post_bounty_id = register_post_bounty_action(&mut defs, &mut handlers);
        let affordance = Affordance {
            def_id: post_bounty_id,
            actor,
            bound_targets: vec![square],
            payload_override: Some(post_bounty_payload(square, square, target, actor)),
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = DeterministicRng::new(Seed([7; 32]));

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
            ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap();

        let first_tick = tick_action(
            action_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(6)),
        )
        .unwrap();
        assert!(matches!(first_tick, TickOutcome::Continuing));

        let second_tick = tick_action(
            action_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(7)),
        )
        .unwrap();

        assert!(matches!(second_tick, TickOutcome::Committed { .. }));
        let artifacts = world
            .all_entities()
            .filter(|entity| world.entity_kind(*entity) == Some(EntityKind::SocialArtifact))
            .collect::<Vec<_>>();
        assert_eq!(artifacts.len(), 1);
        let artifact = artifacts[0];
        assert_eq!(world.effective_place(artifact), Some(square));
        assert_eq!(
            world.get_component_artifact_header(artifact).unwrap().kind,
            ArtifactKind::Bounty
        );
        assert_eq!(
            world
                .get_component_bounty_terms(artifact)
                .unwrap()
                .reward_quantity,
            Quantity(4)
        );
        assert_eq!(
            world
                .get_component_contention_policy(artifact)
                .unwrap()
                .max_waiters,
            Some(0)
        );
        assert_eq!(
            world.get_component_contention_queue(artifact),
            Some(&ContentionQueue::default())
        );
        assert_eq!(
            world.get_component_obligation_execution_tracker(actor),
            Some(&ObligationExecutionTracker {
                completion_ticks: vec![Tick(7)],
            })
        );
        assert_eq!(log.events_by_tag(EventTag::ActionCommitted).len(), 1);
    }

    #[test]
    fn post_notice_commits_social_artifact_with_notice_content() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let route = prototype_place_entity(PrototypePlace::EastFieldTrail);
        let actor = spawn_agent_at(&mut world, "issuer", square);

        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let ids = register_artifact_actions(&mut defs, &mut handlers);
        let affordance = Affordance {
            def_id: ids[1],
            actor,
            bound_targets: vec![square],
            payload_override: Some(post_notice_payload(square, route)),
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = DeterministicRng::new(Seed([8; 32]));

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
            ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
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
            ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(6)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        let artifact = world
            .all_entities()
            .find(|entity| world.entity_kind(*entity) == Some(EntityKind::SocialArtifact))
            .unwrap();
        assert_eq!(
            world.get_component_artifact_header(artifact).unwrap().kind,
            ArtifactKind::Notice
        );
        assert_eq!(
            world.get_component_notice_content(artifact).unwrap().topic,
            NoticeTopic::ThreatWarning { place: route }
        );
        assert_eq!(
            world.get_component_obligation_execution_tracker(actor),
            Some(&ObligationExecutionTracker {
                completion_ticks: vec![Tick(6)],
            })
        );
    }

    #[test]
    fn post_bounty_fails_when_actor_is_not_colocated_with_posting_place() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let farm = prototype_place_entity(PrototypePlace::OrchardFarm);
        let actor = spawn_agent_at(&mut world, "issuer", square);
        let target = spawn_agent_at(&mut world, "target", square);
        grant_personal_funds(&mut world, actor, square, CommodityKind::Coin, 4);

        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let post_bounty_id = register_post_bounty_action(&mut defs, &mut handlers);
        let affordance = Affordance {
            def_id: post_bounty_id,
            actor,
            bound_targets: vec![farm],
            payload_override: Some(post_bounty_payload(farm, farm, target, actor)),
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = DeterministicRng::new(Seed([9; 32]));

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
            ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn claim_bounty_transfers_reward_and_fulfills_bounty() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let posting_place = prototype_place_entity(PrototypePlace::VillageSquare);
        let claim_place = prototype_place_entity(PrototypePlace::OrchardFarm);
        let issuer = spawn_agent_at(&mut world, "issuer", posting_place);
        let claimant = spawn_agent_at(&mut world, "claimant", claim_place);
        let target = spawn_agent_at(&mut world, "target", claim_place);
        kill_entity(&mut world, target, 2);
        grant_personal_funds(&mut world, issuer, claim_place, CommodityKind::Coin, 4);
        let bounty = create_bounty_artifact(
            &mut world,
            issuer,
            posting_place,
            claim_place,
            target,
            RewardSource::PersonalFunds { issuer },
            ProofRequirement::PhysicalEvidence,
        );

        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let ids = register_artifact_actions(&mut defs, &mut handlers);
        let affordance = Affordance {
            def_id: ids[2],
            actor: claimant,
            bound_targets: vec![bounty],
            payload_override: None,
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Available,
        };
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = DeterministicRng::new(Seed([10; 32]));
        let total_before = sum_commodity(&world, CommodityKind::Coin);

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
            ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap();

        let outcome = tick_to_completion(
            &mut world,
            &defs,
            &handlers,
            &mut active,
            &mut log,
            &mut rng,
            action_id,
            6,
        )
        .unwrap();
        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(
            world.get_component_artifact_header(bounty).unwrap().state,
            ArtifactState::Fulfilled
        );
        assert_eq!(
            world.controlled_commodity_quantity(claimant, CommodityKind::Coin),
            Quantity(4)
        );
        assert_eq!(
            world.controlled_commodity_quantity(issuer, CommodityKind::Coin),
            Quantity(0)
        );
        assert_eq!(sum_commodity(&world, CommodityKind::Coin), total_before);
        assert!(
            world
                .get_component_contention_queue(bounty)
                .unwrap()
                .granted
                .is_none()
        );
    }

    #[test]
    fn claim_bounty_rejects_second_claimant_in_race_mode() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let posting_place = prototype_place_entity(PrototypePlace::VillageSquare);
        let claim_place = prototype_place_entity(PrototypePlace::OrchardFarm);
        let issuer = spawn_agent_at(&mut world, "issuer", posting_place);
        let claimant_a = spawn_agent_at(&mut world, "claimant_a", claim_place);
        let claimant_b = spawn_agent_at(&mut world, "claimant_b", claim_place);
        let target = spawn_agent_at(&mut world, "target", claim_place);
        kill_entity(&mut world, target, 2);
        grant_personal_funds(&mut world, issuer, claim_place, CommodityKind::Coin, 4);
        let bounty = create_bounty_artifact(
            &mut world,
            issuer,
            posting_place,
            claim_place,
            target,
            RewardSource::PersonalFunds { issuer },
            ProofRequirement::PhysicalEvidence,
        );

        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let ids = register_artifact_actions(&mut defs, &mut handlers);
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = DeterministicRng::new(Seed([11; 32]));

        let first = Affordance {
            def_id: ids[2],
            actor: claimant_a,
            bound_targets: vec![bounty],
            payload_override: None,
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Available,
        };
        start_action(
            &first,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_id,
            ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap();

        let second = Affordance {
            def_id: ids[2],
            actor: claimant_b,
            bound_targets: vec![bounty],
            payload_override: None,
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Full,
        };
        let err = start_action(
            &second,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_id,
            ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap_err();

        assert!(matches!(
            err,
            ActionError::PreconditionFailed(message) if message == "contention_rejected"
        ));
    }

    #[test]
    fn claim_bounty_depleted_source_fails_and_bounty_stays_active() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let posting_place = prototype_place_entity(PrototypePlace::VillageSquare);
        let claim_place = prototype_place_entity(PrototypePlace::OrchardFarm);
        let issuer = spawn_agent_at(&mut world, "issuer", posting_place);
        let claimant = spawn_agent_at(&mut world, "claimant", claim_place);
        let target = spawn_agent_at(&mut world, "target", claim_place);
        kill_entity(&mut world, target, 2);
        let reward_lot = {
            let mut txn = new_txn(&mut world, 2);
            let lot = txn
                .create_item_lot(CommodityKind::Coin, Quantity(4))
                .unwrap();
            txn.set_ground_location(lot, claim_place).unwrap();
            txn.set_owner(lot, issuer).unwrap();
            commit_txn(txn);
            lot
        };
        let bounty = create_bounty_artifact(
            &mut world,
            issuer,
            posting_place,
            claim_place,
            target,
            RewardSource::PersonalFunds { issuer },
            ProofRequirement::PhysicalEvidence,
        );

        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let ids = register_artifact_actions(&mut defs, &mut handlers);
        let affordance = Affordance {
            def_id: ids[2],
            actor: claimant,
            bound_targets: vec![bounty],
            payload_override: None,
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Available,
        };
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = DeterministicRng::new(Seed([12; 32]));

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
            ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap();

        {
            let mut txn = new_txn(&mut world, 6);
            txn.clear_owner(reward_lot).unwrap();
            commit_txn(txn);
        }

        let first = tick_action(
            action_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(6)),
        )
        .unwrap();
        assert!(matches!(first, TickOutcome::Continuing));
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
            ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(7)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Aborted { .. }));
        assert_eq!(
            world.get_component_artifact_header(bounty).unwrap().state,
            ArtifactState::Active
        );
    }

    #[test]
    fn claim_bounty_rejects_when_proof_is_insufficient() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let posting_place = prototype_place_entity(PrototypePlace::VillageSquare);
        let claim_place = prototype_place_entity(PrototypePlace::OrchardFarm);
        let issuer = spawn_agent_at(&mut world, "issuer", posting_place);
        let claimant = spawn_agent_at(&mut world, "claimant", claim_place);
        let target = spawn_agent_at(&mut world, "target", claim_place);
        grant_personal_funds(&mut world, issuer, claim_place, CommodityKind::Coin, 4);
        let bounty = create_bounty_artifact(
            &mut world,
            issuer,
            posting_place,
            claim_place,
            target,
            RewardSource::PersonalFunds { issuer },
            ProofRequirement::PhysicalEvidence,
        );

        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let ids = register_artifact_actions(&mut defs, &mut handlers);
        let affordance = Affordance {
            def_id: ids[2],
            actor: claimant,
            bound_targets: vec![bounty],
            payload_override: None,
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Available,
        };
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = DeterministicRng::new(Seed([13; 32]));

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
            ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn claim_bounty_rejects_when_bounty_is_already_fulfilled() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let posting_place = prototype_place_entity(PrototypePlace::VillageSquare);
        let claim_place = prototype_place_entity(PrototypePlace::OrchardFarm);
        let issuer = spawn_agent_at(&mut world, "issuer", posting_place);
        let claimant = spawn_agent_at(&mut world, "claimant", claim_place);
        let target = spawn_agent_at(&mut world, "target", claim_place);
        kill_entity(&mut world, target, 2);
        grant_personal_funds(&mut world, issuer, claim_place, CommodityKind::Coin, 4);
        let bounty = create_bounty_artifact(
            &mut world,
            issuer,
            posting_place,
            claim_place,
            target,
            RewardSource::PersonalFunds { issuer },
            ProofRequirement::PhysicalEvidence,
        );
        {
            let mut txn = new_txn(&mut world, 4);
            let mut header = *txn.get_component_artifact_header(bounty).unwrap();
            header.state = ArtifactState::Fulfilled;
            txn.set_component_artifact_header(bounty, header).unwrap();
            commit_txn(txn);
        }

        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let ids = register_artifact_actions(&mut defs, &mut handlers);
        let affordance = Affordance {
            def_id: ids[2],
            actor: claimant,
            bound_targets: vec![bounty],
            payload_override: None,
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = DeterministicRng::new(Seed([14; 32]));

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
            ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn claim_bounty_affordance_targets_known_remote_bounty_by_identity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let posting_place = prototype_place_entity(PrototypePlace::VillageSquare);
        let claim_place = prototype_place_entity(PrototypePlace::OrchardFarm);
        let claimant = spawn_agent_at(&mut world, "claimant", claim_place);
        let issuer = spawn_agent_at(&mut world, "issuer", posting_place);
        let target = spawn_agent_at(&mut world, "target", claim_place);
        let bounty = create_bounty_artifact(
            &mut world,
            issuer,
            posting_place,
            claim_place,
            target,
            RewardSource::PersonalFunds { issuer },
            ProofRequirement::SelfReport,
        );
        {
            let mut txn = new_txn(&mut world, 4);
            let mut beliefs = txn
                .get_component_agent_belief_store(claimant)
                .cloned()
                .unwrap_or_else(AgentBeliefStore::new);
            beliefs.update_entity(
                bounty,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(posting_place),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: Some(BelievedArtifactState {
                        kind: ArtifactKind::Bounty,
                        state: ArtifactState::Active,
                        issuer,
                        expires_at: Some(Tick(12)),
                        bounty_terms: Some(BelievedBountyTerms {
                            target: BountyTarget::EliminateEntity { target },
                            reward_commodity: CommodityKind::Coin,
                            reward_quantity: Quantity(4),
                            claim_place,
                        }),
                        notice_topic: None,
                        observed_tick: Tick(4),
                    }),
                    believed_contention: None,
                    believed_evidence: None,
                    observed_tick: Tick(4),
                    source: PerceptionSource::DirectObservation,
                },
            );
            txn.set_component_agent_belief_store(claimant, beliefs)
                .unwrap();
            commit_txn(txn);
        }

        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let ids = register_artifact_actions(&mut defs, &mut handlers);
        let def = defs.get(ids[2]).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let view = PerAgentBeliefView::from_world_at_tick(claimant, Tick(5), &world);

        let targets = (handler.affordance_targets)(def, claimant, &view);

        assert_eq!(targets, vec![vec![bounty]]);
    }
}
