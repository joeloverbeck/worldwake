use std::collections::BTreeSet;
use std::num::NonZeroU32;

use worldwake_core::{
    ArtifactHeader, ArtifactKind, ArtifactState, BountyTarget, BountyTerms, BodyCostPerTick,
    ContentionPolicy, ContentionQueue, EntityId, EntityKind, EventLog, EventTag, NoticeContent,
    NoticeTopic, Quantity, RewardSource, Tick, VisibilitySpec, World, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefRegistry, ActionError, ActionExecutionContext,
    ActionHandler, ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionPayload,
    ActionProgress, ActionState, CommitOutcome, Constraint, DeterministicRng, DurationExpr,
    Interruptibility,
    PostBountyActionPayload, PostNoticeActionPayload, Precondition, RuntimeBeliefView, TargetSpec,
};

pub fn register_artifact_actions(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> [worldwake_core::ActionDefId; 2] {
    let post_bounty_id = register_post_bounty_action(defs, handlers);
    let post_notice_id = register_post_notice_action(defs, handlers);
    [post_bounty_id, post_notice_id]
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

fn validate_target_place(world: &World, place: EntityId) -> Result<(), ActionError> {
    if world.entity_kind(place) != Some(EntityKind::Place) {
        return Err(ActionError::PreconditionFailed(format!(
            "target place {place} is not a place"
        )));
    }
    Ok(())
}

fn validate_expiration_tick(current_tick: Tick, expires_at: Option<Tick>) -> Result<(), ActionError> {
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
        BountyTarget::DeliverCommodity { destination, .. } => validate_target_place(world, destination)?,
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
    if let Some(authority) = payload.issuing_authority {
        if world.entity_kind(authority).is_none() {
            return Err(ActionError::PreconditionFailed(format!(
                "issuing authority {authority} does not exist"
            )));
        }
    }
    if let Some(jurisdiction) = payload.jurisdiction {
        if world.entity_kind(jurisdiction).is_none() {
            return Err(ActionError::PreconditionFailed(format!(
                "jurisdiction entity {jurisdiction} does not exist"
            )));
        }
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
    if let Some(authority) = payload.issuing_authority {
        if world.entity_kind(authority).is_none() {
            return Err(ActionError::PreconditionFailed(format!(
                "issuing authority {authority} does not exist"
            )));
        }
    }
    if let Some(jurisdiction) = payload.jurisdiction {
        if world.entity_kind(jurisdiction).is_none() {
            return Err(ActionError::PreconditionFailed(format!(
                "jurisdiction entity {jurisdiction} does not exist"
            )));
        }
    }
    Ok(())
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
    txn.set_component_notice_content(artifact, NoticeContent { topic: payload.topic })
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    txn.set_ground_location(artifact, posting_place)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    txn.add_target(artifact).add_target(posting_place);

    Ok(CommitOutcome::empty())
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
        build_prototype_world, prototype_place_entity, CauseRef, CommodityKind, ContentionQueue,
        ControlSource, EventLog, EventTag, ProofRequirement, PrototypePlace, Quantity, Seed, Tick,
        VisibilitySpec, WitnessData, World, WorldTxn,
    };
    use worldwake_sim::{
        start_action, tick_action, ActionDefRegistry, ActionError, ActionExecutionAuthority,
        ActionExecutionContext, ActionHandlerRegistry, ActionInstanceId, ActionPayload,
        Affordance, DeterministicRng, PostBountyActionPayload, PostNoticeActionPayload,
        TickOutcome,
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

        assert_eq!(handlers.len(), 2);
        assert_eq!(ids.len(), 2);
        assert_eq!(defs.get(ids[0]).unwrap().name, "post_bounty");
        assert_eq!(defs.get(ids[1]).unwrap().name, "post_notice");
        assert_eq!(defs.get(ids[0]).unwrap().targets, vec![TargetSpec::ActorPlace]);
        assert_eq!(defs.get(ids[1]).unwrap().targets, vec![TargetSpec::ActorPlace]);
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
            world.get_component_bounty_terms(artifact).unwrap().reward_quantity,
            Quantity(4)
        );
        assert_eq!(
            world.get_component_contention_policy(artifact).unwrap().max_waiters,
            Some(0)
        );
        assert_eq!(
            world.get_component_contention_queue(artifact),
            Some(&ContentionQueue::default())
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
}
