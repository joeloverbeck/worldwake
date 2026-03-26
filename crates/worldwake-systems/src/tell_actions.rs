use std::collections::BTreeSet;
use std::num::NonZeroU32;
use worldwake_core::{
    current_institutional_belief_topics, institutional_claim_subject_entity,
    institutional_claim_same_memory_lane, institutional_knowledge_chain_len,
    social_observation_is_redundant_for_listener, tell_subject_is_directly_observable_by_listener,
    ActionDefId, AgentBeliefStore, BelievedInstitutionalClaim, BodyCostPerTick, EntityId,
    EntityKind, EventTag, HeardBeliefDisposition, HeardBeliefMemory, InstitutionalBeliefKey,
    InstitutionalClaim, InstitutionalKnowledgeSource, PerceptionProfile, PerceptionSource,
    Permille, RecipientKnowledgeStatus, TellMemoryKey, TellProfile, TellTopic, ToldBeliefMemory,
    VisibilitySpec, World, WorldTxn,
};
use worldwake_sim::{
    belief_chain_len, listener_aware_tell_topic_selection, AbortReason, ActionAbortRequestReason,
    ActionDef, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerId,
    ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress, ActionState,
    CommitOutcome, CommitTraceData, Constraint, DeterministicRng, DurationExpr, Interruptibility,
    PayloadEntityRole, Precondition, TargetSpec, TellActionPayload, TellBeliefDeltaKind,
    TellCommitResult, TellCommitTrace,
};

pub fn register_tell_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(start_tell, tick_tell, commit_tell, abort_tell)
            .with_affordance_payloads(enumerate_tell_payloads)
            .with_payload_override_validator(validate_tell_payload_override)
            .with_authoritative_payload_validator(validate_tell_payload_authoritatively),
    );
    let id = ActionDefId(defs.len() as u32);
    defs.register(tell_action_def(id, handler))
}

fn tell_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "tell".to_string(),
        domain: worldwake_sim::ActionDomain::Social,
        actor_constraints: vec![Constraint::ActorAlive],
        targets: vec![TargetSpec::EntityAtActorPlace {
            kind: EntityKind::Agent,
        }],
        preconditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetAtActorPlace(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Agent,
            },
            Precondition::TargetAlive(0),
        ],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::Fixed(NonZeroU32::new(2).unwrap()),
        body_cost_per_tick: BodyCostPerTick::zero(),
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetAtActorPlace(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Agent,
            },
            Precondition::TargetAlive(0),
        ],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::Social, EventTag::WorldMutation]),
        payload: ActionPayload::None,
        handler,
    }
}

fn tell_payload<'a>(
    def: &ActionDef,
    payload: &'a ActionPayload,
) -> Result<&'a TellActionPayload, ActionError> {
    payload.as_tell().ok_or_else(|| {
        ActionError::PreconditionFailed(format!("action def {} requires Tell payload", def.id))
    })
}

fn degrade_source(speaker: EntityId, source: PerceptionSource) -> PerceptionSource {
    match source {
        PerceptionSource::DirectObservation => PerceptionSource::Report {
            from: speaker,
            chain_len: 1,
        },
        PerceptionSource::Report { chain_len, .. } | PerceptionSource::Rumor { chain_len } => {
            PerceptionSource::Rumor {
                chain_len: chain_len.saturating_add(1),
            }
        }
        PerceptionSource::Inference => PerceptionSource::Rumor { chain_len: 1 },
    }
}

fn degrade_institutional_source(
    speaker: EntityId,
    source: InstitutionalKnowledgeSource,
) -> InstitutionalKnowledgeSource {
    match source {
        InstitutionalKnowledgeSource::WitnessedEvent
        | InstitutionalKnowledgeSource::RecordConsultation { .. }
        | InstitutionalKnowledgeSource::SelfDeclaration => InstitutionalKnowledgeSource::Report {
            from: speaker,
            chain_len: 1,
        },
        InstitutionalKnowledgeSource::Report { chain_len, .. } => {
            InstitutionalKnowledgeSource::Report {
                from: speaker,
                chain_len: chain_len.saturating_add(1),
            }
        }
    }
}

fn institutional_belief_key(claim: InstitutionalClaim) -> InstitutionalBeliefKey {
    match claim {
        InstitutionalClaim::OfficeHolder { office, .. } => {
            InstitutionalBeliefKey::OfficeHolderOf { office }
        }
        InstitutionalClaim::ForceControl { office, .. } => {
            InstitutionalBeliefKey::ForceControllerOf { office }
        }
        InstitutionalClaim::FactionMembership { faction, .. } => {
            InstitutionalBeliefKey::FactionMembersOf { faction }
        }
        InstitutionalClaim::SupportDeclaration {
            supporter, office, ..
        } => InstitutionalBeliefKey::SupportFor { supporter, office },
    }
}

fn best_relayable_institutional_belief(
    beliefs: &AgentBeliefStore,
    claim: InstitutionalClaim,
    max_relay_chain_len: u8,
) -> Option<BelievedInstitutionalClaim> {
    beliefs
        .relayable_institutional_beliefs(max_relay_chain_len)
        .into_iter()
        .filter(|belief| institutional_claim_same_memory_lane(belief.claim, claim))
        .max_by_key(|belief| {
            (
                std::cmp::Reverse(institutional_knowledge_chain_len(belief.source)),
                belief.learned_tick,
                belief.learned_at,
            )
        })
}

fn listener_already_has_institutional_claim(
    store: &AgentBeliefStore,
    belief: &BelievedInstitutionalClaim,
) -> bool {
    store
        .institutional_beliefs
        .get(&institutional_belief_key(belief.claim))
        .is_some_and(|claims| claims.iter().any(|existing| existing.claim == belief.claim))
}

fn passes_acceptance_check(fidelity: u16, rng: &mut DeterministicRng) -> bool {
    match fidelity {
        0 => false,
        1000 => true,
        value => rng.next_range(0, 1000) < u32::from(value),
    }
}

fn validate_tell_context(
    txn: &WorldTxn<'_>,
    instance: &ActionInstance,
    payload: &TellActionPayload,
) -> Result<EntityId, ActionError> {
    let listener = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    if payload.listener != listener {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::PayloadEntityMismatch {
                role: PayloadEntityRole::Target,
                expected: listener,
                actual: payload.listener,
            },
        ));
    }

    let actor_place = txn.effective_place(instance.actor).ok_or({
        ActionError::AbortRequested(ActionAbortRequestReason::ActorNotPlaced {
            actor: instance.actor,
        })
    })?;
    if txn.effective_place(listener) != Some(actor_place) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetNotColocated {
                actor: instance.actor,
                target: listener,
            },
        ));
    }

    Ok(listener)
}

fn required_tell_profile_in_world(
    world: &World,
    entity: EntityId,
) -> Result<TellProfile, ActionError> {
    world
        .get_component_tell_profile(entity)
        .copied()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "live agent {entity} lacks required TellProfile"
            ))
        })
}

fn required_tell_profile(
    world: &WorldTxn<'_>,
    entity: EntityId,
) -> Result<TellProfile, ActionError> {
    world
        .get_component_tell_profile(entity)
        .copied()
        .ok_or_else(|| {
            ActionError::InternalError(format!("live agent {entity} lacks required TellProfile"))
        })
}

fn required_perception_profile(
    world: &WorldTxn<'_>,
    entity: EntityId,
) -> Result<PerceptionProfile, ActionError> {
    world
        .get_component_perception_profile(entity)
        .copied()
        .ok_or_else(|| {
            ActionError::InternalError(format!(
                "live agent {entity} lacks required PerceptionProfile"
            ))
        })
}

fn required_belief_store(
    world: &WorldTxn<'_>,
    entity: EntityId,
) -> Result<AgentBeliefStore, ActionError> {
    world
        .get_component_agent_belief_store(entity)
        .cloned()
        .ok_or_else(|| {
            ActionError::InternalError(format!(
                "live agent {entity} lacks required AgentBeliefStore"
            ))
        })
}

fn validate_tell_payload_override(
    _def: &ActionDef,
    _actor: EntityId,
    _targets: &[EntityId],
    payload: &ActionPayload,
    _view: &dyn worldwake_sim::RuntimeBeliefView,
) -> bool {
    payload.as_tell().is_some()
}

fn enumerate_tell_payloads(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    view: &dyn worldwake_sim::RuntimeBeliefView,
) -> Vec<ActionPayload> {
    let Some(listener) = targets.first().copied() else {
        return Vec::new();
    };
    if listener == actor {
        return Vec::new();
    }

    let Some(profile) = view.tell_profile(actor) else {
        return Vec::new();
    };
    let known_institutional_beliefs =
        current_institutional_belief_topics(view.known_institutional_beliefs(actor));
    let relayable_entity_beliefs = view
        .known_entity_beliefs(actor)
        .into_iter()
        .filter(|(subject, _)| {
            let subject_kind = view.entity_kind(*subject);
            let claim_first_subject = matches!(
                subject_kind,
                Some(EntityKind::Office | EntityKind::Record)
            ) && known_institutional_beliefs
                .iter()
                .any(|belief| institutional_claim_subject_entity(belief.claim) == *subject);
            if claim_first_subject {
                return false;
            }
            !subject_is_listener_observable_entity_belief(view, listener, *subject)
        })
        .collect::<Vec<_>>();
    let relayable_social_observations = view
        .known_social_observations(actor)
        .into_iter()
        .filter(|observation| !social_observation_is_redundant_for_listener(observation, listener))
        .collect::<Vec<_>>();
    let selection = listener_aware_tell_topic_selection(
        relayable_entity_beliefs,
        relayable_social_observations,
        known_institutional_beliefs,
        profile.max_relay_chain_len,
        profile.max_tell_candidates,
        |topic| {
            view.recipient_knowledge_status(actor, listener, topic)
                .unwrap_or(RecipientKnowledgeStatus::UnknownToSpeaker)
        },
    );

    selection
        .selected
        .into_iter()
        .map(|topic| ActionPayload::Tell(TellActionPayload { listener, topic }))
        .collect()
}

fn validate_tell_payload_authoritatively(
    def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let payload = tell_payload(def, payload)?;
    let listener = *targets.first().ok_or(ActionError::InvalidTarget(actor))?;

    if payload.listener != listener {
        return Err(ActionError::PreconditionFailed(format!(
            "tell payload listener {} does not match bound target {}",
            payload.listener, listener
        )));
    }
    if listener == actor {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} cannot tell themselves"
        )));
    }

    let beliefs = world
        .get_component_agent_belief_store(actor)
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!("actor {actor} lacks AgentBeliefStore"))
        })?;
    let relay_limit = required_tell_profile_in_world(world, actor)?.max_relay_chain_len;
    let chain_len = match payload.topic {
        TellTopic::EntityBelief { subject } => {
            if subject_is_listener_observable_entity_belief_in_world(world, listener, subject) {
                return Err(ActionError::PreconditionFailed(format!(
                    "listener {listener} can already directly observe subject {subject}"
                )));
            }
            let belief = beliefs.get_entity(&subject).ok_or_else(|| {
                ActionError::PreconditionFailed(format!(
                    "actor {actor} lacks belief about subject {subject}"
                ))
            })?;
            belief_chain_len(belief.source)
        }
        TellTopic::SocialObservation { observation } => {
            if social_observation_is_redundant_for_listener(&observation, listener) {
                return Err(ActionError::PreconditionFailed(format!(
                    "listener {listener} already directly knows social observation topic"
                )));
            }
            if !beliefs.social_observations.contains(&observation) {
                return Err(ActionError::PreconditionFailed(format!(
                    "actor {actor} lacks social observation topic"
                )));
            }
            belief_chain_len(observation.source)
        }
        TellTopic::InstitutionalClaim { claim } => {
            let belief =
                best_relayable_institutional_belief(beliefs, claim, relay_limit).ok_or_else(|| {
                    ActionError::PreconditionFailed(format!(
                        "actor {actor} lacks institutional claim topic {claim:?}"
                    ))
                })?;
            institutional_knowledge_chain_len(belief.source)
        }
    };
    if chain_len > relay_limit {
        return Err(ActionError::PreconditionFailed(format!(
            "tell topic chain length {chain_len} exceeds actor {actor} relay limit {relay_limit}"
        )));
    }

    Ok(())
}

fn subject_is_listener_observable_entity_belief(
    view: &dyn worldwake_sim::RuntimeBeliefView,
    listener: EntityId,
    subject: EntityId,
) -> bool {
    tell_subject_is_directly_observable_by_listener(
        subject,
        view.entity_kind(subject),
        view.effective_place(subject),
        listener,
        view.effective_place(listener),
        view.observation_fidelity(listener),
    )
}

fn subject_is_listener_observable_entity_belief_in_world(
    world: &World,
    listener: EntityId,
    subject: EntityId,
) -> bool {
    tell_subject_is_directly_observable_by_listener(
        subject,
        world.entity_kind(subject),
        world.effective_place(subject),
        listener,
        world.effective_place(listener),
        world
            .get_component_perception_profile(listener)
            .map_or(Permille::new_unchecked(1000), |profile| {
                profile.observation_fidelity
            }),
    )
}

#[allow(clippy::unnecessary_wraps)]
fn start_tell(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let _ = tell_payload(def, &instance.payload)?;
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_tell(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn merge_tell_delta_kind(
    current: TellBeliefDeltaKind,
    next: TellBeliefDeltaKind,
) -> TellBeliefDeltaKind {
    match (current, next) {
        (TellBeliefDeltaKind::None, delta) | (delta, TellBeliefDeltaKind::None) => delta,
        (left, right) if left == right => left,
        _ => TellBeliefDeltaKind::Mixed,
    }
}

fn tell_trace(
    listener: EntityId,
    topic: TellTopic,
    result: TellCommitResult,
    heard_disposition: Option<HeardBeliefDisposition>,
    belief_delta: TellBeliefDeltaKind,
) -> CommitOutcome {
    CommitOutcome::empty().with_trace(CommitTraceData::Tell(TellCommitTrace {
        listener,
        topic,
        result,
        heard_disposition,
        belief_delta,
    }))
}

#[allow(clippy::too_many_lines, clippy::unnecessary_wraps)]
fn commit_tell(
    def: &ActionDef,
    instance: &ActionInstance,
    rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = tell_payload(def, &instance.payload)?;
    let listener = validate_tell_context(txn, instance, payload)?;
    let speaker = instance.actor;

    let mut speaker_beliefs = required_belief_store(txn, speaker)?;
    let speaker_profile = required_tell_profile(txn, speaker)?;
    let Some(shared_state) = speaker_beliefs
        .shared_tell_state_for_topic(&payload.topic, speaker_profile.max_relay_chain_len)
    else {
        return Ok(tell_trace(
            listener,
            payload.topic,
            TellCommitResult::SpeakerNoLongerKnowsTopic,
            None,
            TellBeliefDeltaKind::None,
        ));
    };

    let topic_chain_len = match payload.topic {
        TellTopic::EntityBelief { subject } => speaker_beliefs
            .get_entity(&subject)
            .map_or(u8::MAX, |belief| belief_chain_len(belief.source)),
        TellTopic::SocialObservation { observation } => belief_chain_len(observation.source),
        TellTopic::InstitutionalClaim { claim } => best_relayable_institutional_belief(
            &speaker_beliefs,
            claim,
            speaker_profile.max_relay_chain_len,
        )
        .map_or(u8::MAX, |belief| {
            institutional_knowledge_chain_len(belief.source)
        }),
    };
    if topic_chain_len > speaker_profile.max_relay_chain_len {
        return Ok(tell_trace(
            listener,
            payload.topic,
            TellCommitResult::RelayLimitExceeded,
            None,
            TellBeliefDeltaKind::None,
        ));
    }

    let told_key = TellMemoryKey {
        counterparty: listener,
        topic: payload.topic,
    };
    speaker_beliefs.record_told_belief(
        told_key,
        ToldBeliefMemory {
            shared_state: shared_state.clone(),
            told_tick: txn.tick(),
        },
    );
    speaker_beliefs.enforce_conversation_memory(&speaker_profile, txn.tick());

    let mut listener_beliefs = required_belief_store(txn, listener)?;
    let listener_profile = required_tell_profile(txn, listener)?;
    let heard_key = TellMemoryKey {
        counterparty: speaker,
        topic: payload.topic,
    };
    let (result, disposition, belief_delta) =
        if passes_acceptance_check(listener_profile.acceptance_fidelity.value(), rng) {
            let mut accepted_any = false;
            let mut belief_delta = TellBeliefDeltaKind::None;
            let listener_perception = required_perception_profile(txn, listener)?;
            match payload.topic {
                TellTopic::EntityBelief { subject } => {
                    let Some(speaker_belief) = speaker_beliefs.get_entity(&subject).cloned() else {
                        return Ok(tell_trace(
                            listener,
                            payload.topic,
                            TellCommitResult::SpeakerNoLongerKnowsTopic,
                            None,
                            TellBeliefDeltaKind::None,
                        ));
                    };
                    let mut transferred = speaker_belief.clone();
                    transferred.source = degrade_source(speaker, speaker_belief.source);
                    let should_update_entity =
                        listener_beliefs
                            .get_entity(&subject)
                            .is_none_or(|existing| {
                                existing.observed_tick < speaker_belief.observed_tick
                            });
                    if should_update_entity {
                        listener_beliefs.update_entity(subject, transferred);
                        listener_beliefs.enforce_capacity(&listener_perception, txn.tick());
                        accepted_any = true;
                        belief_delta =
                            merge_tell_delta_kind(belief_delta, TellBeliefDeltaKind::EntityBelief);
                    }
                }
                TellTopic::SocialObservation { observation } => {
                    let mut transferred = observation;
                    transferred.source = degrade_source(speaker, observation.source);
                    if !listener_beliefs.social_observations.contains(&transferred) {
                        listener_beliefs.record_social_observation(transferred);
                        listener_beliefs.enforce_capacity(&listener_perception, txn.tick());
                        accepted_any = true;
                        belief_delta = merge_tell_delta_kind(
                            belief_delta,
                            TellBeliefDeltaKind::SocialObservation,
                        );
                    }
                }
                TellTopic::InstitutionalClaim { claim } => {
                    let Some(belief) = best_relayable_institutional_belief(
                        &speaker_beliefs,
                        claim,
                        speaker_profile.max_relay_chain_len,
                    ) else {
                        return Ok(tell_trace(
                            listener,
                            payload.topic,
                            TellCommitResult::SpeakerNoLongerKnowsTopic,
                            Some(HeardBeliefDisposition::Rejected),
                            TellBeliefDeltaKind::None,
                        ));
                    };
                    let relayed = BelievedInstitutionalClaim {
                        claim: belief.claim,
                        source: degrade_institutional_source(speaker, belief.source),
                        learned_tick: txn.tick(),
                        learned_at: txn.effective_place(listener),
                    };
                    if !listener_already_has_institutional_claim(&listener_beliefs, &relayed) {
                        listener_beliefs.record_institutional_belief(
                            institutional_belief_key(relayed.claim),
                            relayed,
                            &listener_perception,
                        );
                        accepted_any = true;
                        belief_delta = merge_tell_delta_kind(
                            belief_delta,
                            TellBeliefDeltaKind::InstitutionalBelief,
                        );
                    }
                }
            }

            if accepted_any {
                (
                    TellCommitResult::Accepted,
                    HeardBeliefDisposition::Accepted,
                    belief_delta,
                )
            } else {
                (
                    TellCommitResult::AlreadyHeldEqualOrNewer,
                    HeardBeliefDisposition::AlreadyHeldEqualOrNewer,
                    TellBeliefDeltaKind::None,
                )
            }
        } else {
            (
                TellCommitResult::NotInternalized,
                HeardBeliefDisposition::NotInternalized,
                TellBeliefDeltaKind::None,
            )
        };

    listener_beliefs.record_heard_belief(
        heard_key,
        HeardBeliefMemory {
            heard_state: shared_state,
            heard_tick: txn.tick(),
            disposition,
        },
    );
    listener_beliefs.enforce_conversation_memory(&listener_profile, txn.tick());
    txn.set_component_agent_belief_store(speaker, speaker_beliefs)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    txn.set_component_agent_belief_store(listener, listener_beliefs)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    Ok(tell_trace(
        listener,
        payload.topic,
        result,
        Some(disposition),
        belief_delta,
    ))
}

#[allow(clippy::unnecessary_wraps)]
fn abort_tell(
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
    use super::{register_tell_action, validate_tell_payload_authoritatively};
    use std::collections::BTreeSet;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_believed_entity_state, build_prototype_world, to_shared_belief_snapshot, ActionDefId,
        AgentBeliefStore, BeliefConfidencePolicy, BelievedEntityState, BelievedInstitutionalClaim,
        BodyCostPerTick, CauseRef, CombatProfile, CommodityConsumableProfile, CommodityKind,
        ControlSource, DemandObservation, DriveThresholds, EntityId, EntityKind, EventLog,
        EventTag, EventView, HeardBeliefDisposition, HomeostaticNeeds, InTransitOnEdge,
        InstitutionalBeliefKey, InstitutionalClaim, InstitutionalKnowledgeSource,
        IntentionDispositionProfile, LoadUnits, MerchandiseProfile, MetabolismProfile, OfficeData,
        PerceptionProfile, PerceptionSource, Permille, Quantity, RecipeId,
        RecipientKnowledgeStatus, ResourceSource, Seed, SharedTellState, SuccessionLaw,
        TellMemoryKey, TellProfile, TellTopic, Tick, TickRange, TradeDispositionProfile,
        UniqueItemKind, VisibilitySpec, WitnessData, WorkstationTag, World, WorldTxn, Wound,
    };
    use worldwake_sim::{
        get_affordances, ActionDefRegistry, ActionError, ActionHandlerRegistry, ActionInstance,
        ActionPayload, ActionState, ActionStatus, CommitTraceData, DeterministicRng, DurationExpr,
        Interruptibility, Precondition, RuntimeBeliefView, TargetSpec, TellActionPayload,
        TellBeliefDeltaKind, TellCommitResult,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
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
            WitnessData::default(),
        )
    }

    fn new_action_txn(
        world: &mut World,
        actor: EntityId,
        visibility: VisibilitySpec,
        tick: u64,
    ) -> WorldTxn<'_> {
        let place = world.effective_place(actor);
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            Some(actor),
            place,
            visibility,
            WitnessData::default(),
        )
    }

    fn test_rng(seed: u8) -> DeterministicRng {
        DeterministicRng::new(Seed([seed; 32]))
    }

    fn world_with_speaker_listener_and_subject(
        source: PerceptionSource,
    ) -> (World, EntityId, EntityId, EntityId, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();

        let (speaker, listener, subject) = {
            let mut txn = new_txn(&mut world, 1);
            let speaker = txn.create_agent("Speaker", ControlSource::Ai).unwrap();
            let listener = txn.create_agent("Listener", ControlSource::Ai).unwrap();
            let subject = txn.create_agent("Subject", ControlSource::Ai).unwrap();
            txn.set_component_tell_profile(
                listener,
                TellProfile {
                    max_tell_candidates: 3,
                    max_relay_chain_len: 3,
                    acceptance_fidelity: Permille::new(1000).unwrap(),
                    ..TellProfile::default()
                },
            )
            .unwrap();
            for entity in [speaker, listener, subject] {
                txn.set_ground_location(entity, place).unwrap();
            }
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (speaker, listener, subject)
        };

        let belief = build_believed_entity_state(&world, subject, Tick(2), source).unwrap();
        let mut store = world
            .get_component_agent_belief_store(speaker)
            .cloned()
            .unwrap_or_else(AgentBeliefStore::new);
        store.update_entity(subject, belief);

        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_agent_belief_store(speaker, store)
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        (world, place, speaker, listener, subject)
    }

    fn tell_test_setup(
        source: PerceptionSource,
    ) -> (
        ActionDefRegistry,
        ActionHandlerRegistry,
        ActionDefId,
        World,
        EntityId,
        EntityId,
        EntityId,
        EntityId,
    ) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let (world, place, speaker, listener, subject) =
            world_with_speaker_listener_and_subject(source);
        (
            defs, handlers, tell_id, world, place, speaker, listener, subject,
        )
    }

    fn world_with_speaker_listener_and_office_subject(
        source: PerceptionSource,
    ) -> (World, EntityId, EntityId, EntityId, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();

        let (speaker, listener, office) = {
            let mut txn = new_txn(&mut world, 1);
            let speaker = txn.create_agent("Speaker", ControlSource::Ai).unwrap();
            let listener = txn.create_agent("Listener", ControlSource::Ai).unwrap();
            let office = txn.create_office("Village Elder").unwrap();
            txn.set_component_office_data(
                office,
                OfficeData {
                    title: "Village Elder".to_string(),
                    jurisdiction: place,
                    succession_law: SuccessionLaw::Support,
                    eligibility_rules: Vec::new(),
                    succession_period_ticks: 5,
                    vacancy_since: Some(Tick(1)),
                },
            )
            .unwrap();
            txn.set_component_tell_profile(
                listener,
                TellProfile {
                    max_tell_candidates: 3,
                    max_relay_chain_len: 3,
                    acceptance_fidelity: Permille::new(1000).unwrap(),
                    ..TellProfile::default()
                },
            )
            .unwrap();
            for entity in [speaker, listener, office] {
                txn.set_ground_location(entity, place).unwrap();
            }
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (speaker, listener, office)
        };

        let belief = build_believed_entity_state(&world, office, Tick(2), source).unwrap();
        let mut store = world
            .get_component_agent_belief_store(speaker)
            .cloned()
            .unwrap_or_else(AgentBeliefStore::new);
        store.update_entity(office, belief);

        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_agent_belief_store(speaker, store)
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        (world, place, speaker, listener, office)
    }

    fn tell_office_test_setup(
        source: PerceptionSource,
    ) -> (
        ActionDefRegistry,
        ActionHandlerRegistry,
        ActionDefId,
        World,
        EntityId,
        EntityId,
        EntityId,
        EntityId,
    ) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let (world, place, speaker, listener, office) =
            world_with_speaker_listener_and_office_subject(source);
        (
            defs, handlers, tell_id, world, place, speaker, listener, office,
        )
    }

    fn office_holder_claim(
        office: EntityId,
        holder: Option<EntityId>,
        tick: u64,
    ) -> InstitutionalClaim {
        InstitutionalClaim::OfficeHolder {
            office,
            holder,
            effective_tick: Tick(tick),
        }
    }

    fn office_holder_belief(
        office: EntityId,
        holder: Option<EntityId>,
        source: InstitutionalKnowledgeSource,
        learned_tick: u64,
        learned_at: Option<EntityId>,
    ) -> BelievedInstitutionalClaim {
        BelievedInstitutionalClaim {
            claim: office_holder_claim(office, holder, learned_tick),
            source,
            learned_tick: Tick(learned_tick),
            learned_at,
        }
    }

    fn force_control_belief(
        office: EntityId,
        controller: Option<EntityId>,
        contested: bool,
        source: InstitutionalKnowledgeSource,
        learned_tick: u64,
        learned_at: Option<EntityId>,
    ) -> BelievedInstitutionalClaim {
        BelievedInstitutionalClaim {
            claim: InstitutionalClaim::ForceControl {
                office,
                controller,
                contested,
                effective_tick: Tick(learned_tick),
            },
            source,
            learned_tick: Tick(learned_tick),
            learned_at,
        }
    }

    fn tell_instance(
        tell_id: ActionDefId,
        speaker: EntityId,
        listener: EntityId,
        subject: EntityId,
    ) -> ActionInstance {
        ActionInstance {
            instance_id: worldwake_sim::ActionInstanceId(0),
            def_id: tell_id,
            payload: ActionPayload::Tell(TellActionPayload {
                listener,
                topic: TellTopic::EntityBelief { subject },
            }),
            actor: speaker,
            targets: vec![listener],
            start_tick: Tick(5),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        }
    }

    fn commit_tell_and_finalize_event(
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        tell_id: ActionDefId,
        world: &mut World,
        instance: &ActionInstance,
        seed: u8,
        tick: u64,
    ) {
        let def = defs.get(tell_id).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let mut rng = test_rng(seed);
        let mut txn = new_action_txn(world, instance.actor, def.visibility, tick);

        (handler.on_commit)(def, instance, &mut rng, &mut txn).unwrap();
        txn.add_tag(EventTag::ActionCommitted);
        for tag in &def.causal_event_tags {
            txn.add_tag(*tag);
        }
        for target in &instance.targets {
            txn.add_target(*target);
        }

        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
        assert_eq!(log.len(), 1);
    }

    fn commit_tell_result(
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        tell_id: ActionDefId,
        world: &mut World,
        instance: &ActionInstance,
        seed: u8,
        tick: u64,
    ) -> Result<worldwake_sim::CommitOutcome, ActionError> {
        let def = defs.get(tell_id).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let mut rng = test_rng(seed);
        let mut txn = new_action_txn(world, instance.actor, def.visibility, tick);

        (handler.on_commit)(def, instance, &mut rng, &mut txn)
    }

    fn assert_tell_trace(
        outcome: &worldwake_sim::CommitOutcome,
        expected_result: TellCommitResult,
        expected_disposition: Option<HeardBeliefDisposition>,
        expected_delta: TellBeliefDeltaKind,
    ) {
        let trace = match outcome.trace.as_ref() {
            Some(CommitTraceData::Tell(trace)) => trace,
            other => panic!("expected tell trace, got {other:?}"),
        };
        assert_eq!(trace.result, expected_result);
        assert_eq!(trace.heard_disposition, expected_disposition);
        assert_eq!(trace.belief_delta, expected_delta);
        assert_eq!(
            trace.artifact_changed(),
            expected_delta != TellBeliefDeltaKind::None
        );
    }

    #[derive(Default)]
    struct StubTellBeliefView {
        alive: std::collections::BTreeMap<EntityId, bool>,
        kinds: std::collections::BTreeMap<EntityId, EntityKind>,
        places: std::collections::BTreeMap<EntityId, EntityId>,
        beliefs: std::collections::BTreeMap<EntityId, Vec<(EntityId, BelievedEntityState)>>,
        institutional_claims:
            std::collections::BTreeMap<EntityId, Vec<BelievedInstitutionalClaim>>,
        social_observations:
            std::collections::BTreeMap<EntityId, Vec<worldwake_core::SocialObservation>>,
        tell_profiles: std::collections::BTreeMap<EntityId, TellProfile>,
        recipient_statuses:
            std::collections::BTreeMap<(EntityId, EntityId, TellTopic), RecipientKnowledgeStatus>,
    }

    impl RuntimeBeliefView for StubTellBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.get(&entity).copied().unwrap_or(false)
        }

        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.kinds.get(&entity).copied()
        }

        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.places.get(&entity).copied()
        }

        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }

        fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
            let mut entities = self
                .places
                .iter()
                .filter_map(|(entity, entity_place)| (*entity_place == place).then_some(*entity))
                .collect::<Vec<_>>();
            entities.sort();
            entities
        }

        fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
            self.beliefs.get(&agent).cloned().unwrap_or_default()
        }

        fn known_social_observations(
            &self,
            agent: EntityId,
        ) -> Vec<worldwake_core::SocialObservation> {
            self.social_observations
                .get(&agent)
                .cloned()
                .unwrap_or_default()
        }

        fn known_institutional_beliefs(
            &self,
            agent: EntityId,
        ) -> Vec<BelievedInstitutionalClaim> {
            self.institutional_claims
                .get(&agent)
                .cloned()
                .unwrap_or_default()
        }

        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn adjacent_places(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool {
            false
        }

        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 {
            0
        }

        fn commodity_quantity(&self, _holder: EntityId, _kind: CommodityKind) -> Quantity {
            Quantity(0)
        }

        fn controlled_commodity_quantity_at_place(
            &self,
            _agent: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Quantity {
            Quantity(0)
        }

        fn local_controlled_lots_for(
            &self,
            _agent: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn item_lot_commodity(&self, _entity: EntityId) -> Option<CommodityKind> {
            None
        }

        fn item_lot_consumable_profile(
            &self,
            _entity: EntityId,
        ) -> Option<CommodityConsumableProfile> {
            None
        }

        fn direct_container(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn direct_possessor(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn believed_owner_of(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }

        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }

        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }

        fn has_control(&self, _entity: EntityId) -> bool {
            false
        }

        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
            false
        }

        fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> {
            Vec::new()
        }

        fn is_dead(&self, entity: EntityId) -> bool {
            !self.is_alive(entity)
        }

        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }

        fn has_wounds(&self, _entity: EntityId) -> bool {
            false
        }

        fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
            None
        }

        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
        }

        fn belief_confidence_policy(
            &self,
            _agent: EntityId,
        ) -> worldwake_core::BeliefConfidencePolicy {
            worldwake_core::BeliefConfidencePolicy::default()
        }

        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            None
        }

        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
            None
        }

        fn intention_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<IntentionDispositionProfile> {
            None
        }

        fn route_exists(&self, _from: EntityId, _to: EntityId) -> bool {
            false
        }

        fn tell_profile(&self, agent: EntityId) -> Option<TellProfile> {
            self.tell_profiles.get(&agent).copied()
        }

        fn recipient_knowledge_status(
            &self,
            actor: EntityId,
            counterparty: EntityId,
            topic: &TellTopic,
        ) -> Option<RecipientKnowledgeStatus> {
            self.recipient_statuses
                .get(&(actor, counterparty, *topic))
                .copied()
        }

        fn combat_profile(&self, _agent: EntityId) -> Option<CombatProfile> {
            None
        }

        fn wounds(&self, _agent: EntityId) -> Vec<Wound> {
            Vec::new()
        }

        fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn agents_selling_at(&self, _place: EntityId, _commodity: CommodityKind) -> Vec<EntityId> {
            Vec::new()
        }

        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
            Vec::new()
        }

        fn matching_workstations_at(
            &self,
            _place: EntityId,
            _tag: WorkstationTag,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn resource_sources_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            Vec::new()
        }

        fn merchandise_profile(&self, _agent: EntityId) -> Option<MerchandiseProfile> {
            None
        }

        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn in_transit_state(&self, _entity: EntityId) -> Option<InTransitOnEdge> {
            None
        }

        fn adjacent_places_with_travel_ticks(
            &self,
            _place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            Vec::new()
        }

        fn estimate_duration(
            &self,
            _actor: EntityId,
            _duration: &DurationExpr,
            _targets: &[EntityId],
            _payload: &ActionPayload,
        ) -> Option<worldwake_sim::ActionDuration> {
            None
        }
    }

    fn collect_tell_affordances_from_view(
        view: &dyn RuntimeBeliefView,
        speaker: EntityId,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
    ) -> Vec<(EntityId, TellTopic)> {
        get_affordances(view, speaker, defs, handlers)
            .into_iter()
            .filter_map(|affordance| {
                affordance
                    .payload_override
                    .and_then(|payload| payload.as_tell().cloned())
                    .map(|payload| (payload.listener, payload.topic))
            })
            .collect()
    }

    fn believed_state(
        observed_tick: u64,
        last_known_place: EntityId,
        source: PerceptionSource,
    ) -> BelievedEntityState {
        BelievedEntityState {
            last_known_place: Some(last_known_place),
            last_known_inventory: std::collections::BTreeMap::default(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            observed_tick: Tick(observed_tick),
            source,
        }
    }

    fn tell_memory_key(counterparty: EntityId, subject: EntityId) -> TellMemoryKey {
        TellMemoryKey {
            counterparty,
            topic: TellTopic::EntityBelief { subject },
        }
    }

    #[test]
    fn register_tell_action_creates_expected_definition() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();

        assert_eq!(tell.name, "tell");
        assert_eq!(tell.domain, worldwake_sim::ActionDomain::Social);
        assert_eq!(
            tell.targets,
            vec![TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }]
        );
        assert_eq!(
            tell.duration,
            DurationExpr::Fixed(NonZeroU32::new(2).unwrap())
        );
        assert_eq!(tell.body_cost_per_tick, BodyCostPerTick::zero());
        assert_eq!(tell.interruptibility, Interruptibility::FreelyInterruptible);
        assert_eq!(tell.visibility, VisibilitySpec::SamePlace);
        assert_eq!(
            tell.causal_event_tags,
            BTreeSet::from([EventTag::Social, EventTag::WorldMutation])
        );
        assert!(handlers.get(tell.handler).is_some());
        assert_eq!(tell.payload, ActionPayload::None);
        assert!(tell.preconditions.contains(&Precondition::TargetAlive(0)));
        assert!(tell
            .commit_conditions
            .contains(&Precondition::TargetAlive(0)));
    }

    #[test]
    fn tell_payload_validator_rejects_non_tell_payload() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (world, _place, speaker, listener, _subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);

        let err = validate_tell_payload_authoritatively(
            tell,
            &defs,
            speaker,
            &[listener],
            &ActionPayload::None,
            &world,
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn tell_payload_validator_rejects_listener_target_mismatch() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (world, _place, speaker, listener, subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);
        let other_listener = entity(999);

        let err = validate_tell_payload_authoritatively(
            tell,
            &defs,
            speaker,
            &[listener],
            &ActionPayload::Tell(TellActionPayload {
                listener: other_listener,
                topic: TellTopic::EntityBelief { subject },
            }),
            &world,
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn tell_payload_validator_rejects_self_targeting() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (world, _place, speaker, _listener, subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);

        let err = validate_tell_payload_authoritatively(
            tell,
            &defs,
            speaker,
            &[speaker],
            &ActionPayload::Tell(TellActionPayload {
                listener: speaker,
                topic: TellTopic::EntityBelief { subject },
            }),
            &world,
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn tell_payload_validator_rejects_unknown_subject_belief() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (world, _place, speaker, listener, _subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);

        let err = validate_tell_payload_authoritatively(
            tell,
            &defs,
            speaker,
            &[listener],
            &ActionPayload::Tell(TellActionPayload {
                listener,
                topic: TellTopic::EntityBelief {
                    subject: entity(404),
                },
            }),
            &world,
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn tell_payload_validator_rejects_unknown_institutional_claim_topic() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (world, _place, speaker, listener, office) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);
        let claim = InstitutionalClaim::OfficeHolder {
            office,
            holder: None,
            effective_tick: Tick(4),
        };

        let err = validate_tell_payload_authoritatively(
            tell,
            &defs,
            speaker,
            &[listener],
            &ActionPayload::Tell(TellActionPayload {
                listener,
                topic: TellTopic::InstitutionalClaim { claim },
            }),
            &world,
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn tell_payload_validator_rejects_subjects_beyond_relay_limit() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (mut world, _place, speaker, listener, subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::Rumor { chain_len: 4 });

        {
            let mut txn = new_txn(&mut world, 4);
            txn.set_component_tell_profile(
                speaker,
                TellProfile {
                    max_tell_candidates: 3,
                    max_relay_chain_len: 2,
                    acceptance_fidelity: Permille::new(800).unwrap(),
                    ..TellProfile::default()
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        let err = validate_tell_payload_authoritatively(
            tell,
            &defs,
            speaker,
            &[listener],
            &ActionPayload::Tell(TellActionPayload {
                listener,
                topic: TellTopic::EntityBelief { subject },
            }),
            &world,
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn tell_payload_validator_accepts_known_relayable_subject() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (mut world, place, speaker, listener, subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::Report {
                from: entity(77),
                chain_len: 2,
            });
        let remote_place = world
            .topology()
            .place_ids()
            .find(|candidate| *candidate != place)
            .unwrap();
        {
            let mut txn = new_txn(&mut world, 4);
            txn.set_ground_location(subject, remote_place).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        {
            let belief = build_believed_entity_state(
                &world,
                subject,
                Tick(4),
                PerceptionSource::Report {
                    from: entity(77),
                    chain_len: 2,
                },
            )
            .unwrap();
            let mut store = world
                .get_component_agent_belief_store(speaker)
                .cloned()
                .unwrap_or_else(AgentBeliefStore::new);
            store.update_entity(subject, belief);
            let mut txn = new_txn(&mut world, 4);
            txn.set_component_agent_belief_store(speaker, store)
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        assert_eq!(
            validate_tell_payload_authoritatively(
                tell,
                &defs,
                speaker,
                &[listener],
                &ActionPayload::Tell(TellActionPayload {
                    listener,
                    topic: TellTopic::EntityBelief { subject },
                }),
                &world,
            ),
            Ok(())
        );
    }

    #[test]
    fn tell_payload_validator_accepts_known_institutional_claim_topic() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (mut world, place, speaker, listener, office) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);
        let claim = InstitutionalClaim::OfficeHolder {
            office,
            holder: None,
            effective_tick: Tick(4),
        };
        {
            let mut store = world
                .get_component_agent_belief_store(speaker)
                .cloned()
                .unwrap_or_else(AgentBeliefStore::new);
            let profile = *world.get_component_perception_profile(speaker).unwrap();
            store.record_institutional_belief(
                InstitutionalBeliefKey::OfficeHolderOf { office },
                BelievedInstitutionalClaim {
                    claim,
                    source: InstitutionalKnowledgeSource::WitnessedEvent,
                    learned_tick: Tick(4),
                    learned_at: Some(place),
                },
                &profile,
            );
            let mut txn = new_txn(&mut world, 4);
            txn.set_component_agent_belief_store(speaker, store)
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        assert_eq!(
            validate_tell_payload_authoritatively(
                tell,
                &defs,
                speaker,
                &[listener],
                &ActionPayload::Tell(TellActionPayload {
                    listener,
                    topic: TellTopic::InstitutionalClaim { claim },
                }),
                &world,
            ),
            Ok(())
        );
    }

    #[test]
    fn tell_payload_validator_rejects_missing_speaker_tell_profile() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (mut world, _place, speaker, listener, subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);
        {
            let mut txn = new_txn(&mut world, 4);
            txn.clear_component_tell_profile(speaker).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        let err = validate_tell_payload_authoritatively(
            tell,
            &defs,
            speaker,
            &[listener],
            &ActionPayload::Tell(TellActionPayload {
                listener,
                topic: TellTopic::EntityBelief { subject },
            }),
            &world,
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
        assert!(format!("{err:?}").contains("TellProfile"));
    }

    #[test]
    fn tell_action_starts_with_tell_payload() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let handler = handlers.get(tell.handler).unwrap();
        let (mut world, _place, speaker, listener, subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);
        let instance = ActionInstance {
            instance_id: worldwake_sim::ActionInstanceId(0),
            def_id: tell_id,
            payload: ActionPayload::Tell(TellActionPayload {
                listener,
                topic: TellTopic::EntityBelief { subject },
            }),
            actor: speaker,
            targets: vec![listener],
            start_tick: Tick(5),
            remaining_duration: worldwake_sim::ActionDuration::new(2),
            status: worldwake_sim::ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };
        let mut rng = test_rng(1);
        let mut txn = new_txn(&mut world, 5);

        assert_eq!(
            (handler.on_start)(tell, &instance, &mut rng, &mut txn).unwrap(),
            Some(ActionState::Empty)
        );
    }

    #[test]
    fn tell_commit_transfers_direct_observation_as_report_and_preserves_tick() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        let transferred = listener_store.get_entity(&subject).unwrap();
        assert_eq!(transferred.observed_tick, Tick(2));
        assert_eq!(
            transferred.source,
            PerceptionSource::Report {
                from: speaker,
                chain_len: 1,
            }
        );
    }

    #[test]
    fn tell_commit_records_speaker_told_belief_memory() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let expected = {
            let speaker_store = world.get_component_agent_belief_store(speaker).unwrap();
            to_shared_belief_snapshot(speaker_store.get_entity(&subject).unwrap())
        };
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let speaker_store = world.get_component_agent_belief_store(speaker).unwrap();
        let memory = speaker_store
            .told_beliefs
            .get(&tell_memory_key(listener, subject))
            .unwrap();
        assert_eq!(memory.shared_state, SharedTellState::EntityBelief(expected));
        assert_eq!(memory.told_tick, Tick(8));
    }

    #[test]
    fn tell_commit_records_listener_heard_belief_with_accepted_disposition() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let expected = {
            let speaker_store = world.get_component_agent_belief_store(speaker).unwrap();
            to_shared_belief_snapshot(speaker_store.get_entity(&subject).unwrap())
        };
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        let memory = listener_store
            .heard_beliefs
            .get(&tell_memory_key(speaker, subject))
            .unwrap();
        assert_eq!(memory.heard_state, SharedTellState::EntityBelief(expected));
        assert_eq!(memory.heard_tick, Tick(8));
        assert_eq!(memory.disposition, HeardBeliefDisposition::Accepted);
    }

    #[test]
    fn tell_commit_trace_reports_accepted_entity_update() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let instance = tell_instance(tell_id, speaker, listener, subject);

        let outcome =
            commit_tell_result(&defs, &handlers, tell_id, &mut world, &instance, 1, 8).unwrap();

        assert_tell_trace(
            &outcome,
            TellCommitResult::Accepted,
            Some(HeardBeliefDisposition::Accepted),
            TellBeliefDeltaKind::EntityBelief,
        );
    }

    #[test]
    fn tell_commit_degrades_report_to_rumor() {
        let report_source = PerceptionSource::Report {
            from: entity(77),
            chain_len: 2,
        };
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(report_source);
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let transferred = world
            .get_component_agent_belief_store(listener)
            .unwrap()
            .get_entity(&subject)
            .unwrap();
        assert_eq!(transferred.source, PerceptionSource::Rumor { chain_len: 3 });
    }

    #[test]
    fn tell_commit_degrades_rumor_to_deeper_rumor() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::Rumor { chain_len: 3 });
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let transferred = world
            .get_component_agent_belief_store(listener)
            .unwrap()
            .get_entity(&subject)
            .unwrap();
        assert_eq!(transferred.source, PerceptionSource::Rumor { chain_len: 4 });
    }

    #[test]
    fn tell_commit_degrades_inference_to_first_hand_rumor() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::Inference);
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let transferred = world
            .get_component_agent_belief_store(listener)
            .unwrap()
            .get_entity(&subject)
            .unwrap();
        assert_eq!(transferred.source, PerceptionSource::Rumor { chain_len: 1 });
    }

    #[test]
    fn tell_commit_skips_when_speaker_no_longer_has_subject_belief() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        {
            let mut txn = new_txn(&mut world, 6);
            txn.set_component_agent_belief_store(speaker, AgentBeliefStore::new())
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        assert!(listener_store.get_entity(&subject).is_none());
    }

    #[test]
    fn tell_commit_fails_if_speaker_lacks_belief_store() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        {
            let mut txn = new_txn(&mut world, 6);
            txn.clear_component_agent_belief_store(speaker).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        let err =
            commit_tell_result(&defs, &handlers, tell_id, &mut world, &instance, 1, 8).unwrap_err();

        assert!(matches!(err, ActionError::InternalError(_)));
        assert!(format!("{err:?}").contains("AgentBeliefStore"));
        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        assert!(listener_store.get_entity(&subject).is_none());
    }

    #[test]
    fn tell_commit_fails_if_listener_lacks_belief_store() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        {
            let mut txn = new_txn(&mut world, 6);
            txn.clear_component_agent_belief_store(listener).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        let err =
            commit_tell_result(&defs, &handlers, tell_id, &mut world, &instance, 1, 8).unwrap_err();

        assert!(matches!(err, ActionError::InternalError(_)));
        assert!(format!("{err:?}").contains("AgentBeliefStore"));
    }

    #[test]
    fn tell_commit_fails_if_listener_lacks_tell_profile() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        {
            let mut txn = new_txn(&mut world, 6);
            txn.clear_component_tell_profile(listener).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        let err =
            commit_tell_result(&defs, &handlers, tell_id, &mut world, &instance, 1, 8).unwrap_err();

        assert!(matches!(err, ActionError::InternalError(_)));
        assert!(format!("{err:?}").contains("TellProfile"));
    }

    #[test]
    fn tell_commit_fails_if_listener_lacks_perception_profile() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        {
            let mut txn = new_txn(&mut world, 6);
            txn.clear_component_perception_profile(listener).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        let err =
            commit_tell_result(&defs, &handlers, tell_id, &mut world, &instance, 1, 8).unwrap_err();

        assert!(matches!(err, ActionError::InternalError(_)));
        assert!(format!("{err:?}").contains("PerceptionProfile"));
    }

    #[test]
    fn tell_commit_respects_listener_acceptance_fidelity() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let expected = {
            let speaker_store = world.get_component_agent_belief_store(speaker).unwrap();
            to_shared_belief_snapshot(speaker_store.get_entity(&subject).unwrap())
        };
        {
            let mut txn = new_txn(&mut world, 6);
            txn.set_component_tell_profile(
                listener,
                TellProfile {
                    max_tell_candidates: 3,
                    max_relay_chain_len: 3,
                    acceptance_fidelity: Permille::new(0).unwrap(),
                    ..TellProfile::default()
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        assert!(listener_store.get_entity(&subject).is_none());
        let heard = listener_store
            .heard_beliefs
            .get(&tell_memory_key(speaker, subject))
            .unwrap();
        assert_eq!(heard.heard_state, SharedTellState::EntityBelief(expected));
        assert_eq!(heard.disposition, HeardBeliefDisposition::NotInternalized);
        let speaker_store = world.get_component_agent_belief_store(speaker).unwrap();
        assert!(speaker_store
            .told_beliefs
            .contains_key(&tell_memory_key(listener, subject)));
    }

    #[test]
    fn tell_commit_keeps_listener_newer_belief() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let expected = {
            let speaker_store = world.get_component_agent_belief_store(speaker).unwrap();
            to_shared_belief_snapshot(speaker_store.get_entity(&subject).unwrap())
        };
        let newer = build_believed_entity_state(
            &world,
            subject,
            Tick(7),
            PerceptionSource::DirectObservation,
        )
        .unwrap();
        {
            let mut store = world
                .get_component_agent_belief_store(listener)
                .cloned()
                .unwrap_or_default();
            store.update_entity(subject, newer.clone());

            let mut txn = new_txn(&mut world, 7);
            txn.set_component_agent_belief_store(listener, store)
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let retained = world
            .get_component_agent_belief_store(listener)
            .unwrap()
            .get_entity(&subject)
            .unwrap();
        assert_eq!(retained, &newer);
        let heard = world
            .get_component_agent_belief_store(listener)
            .unwrap()
            .heard_beliefs
            .get(&tell_memory_key(speaker, subject))
            .unwrap();
        assert_eq!(heard.heard_state, SharedTellState::EntityBelief(expected));
        assert_eq!(
            heard.disposition,
            HeardBeliefDisposition::AlreadyHeldEqualOrNewer
        );
        assert!(world
            .get_component_agent_belief_store(speaker)
            .unwrap()
            .told_beliefs
            .contains_key(&tell_memory_key(listener, subject)));
    }

    #[test]
    fn tell_commit_records_listener_heard_belief_with_already_held_equal_or_newer() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let expected = {
            let speaker_store = world.get_component_agent_belief_store(speaker).unwrap();
            to_shared_belief_snapshot(speaker_store.get_entity(&subject).unwrap())
        };
        let existing = build_believed_entity_state(
            &world,
            subject,
            Tick(2),
            PerceptionSource::DirectObservation,
        )
        .unwrap();
        {
            let mut store = world
                .get_component_agent_belief_store(listener)
                .cloned()
                .unwrap_or_default();
            store.update_entity(subject, existing.clone());

            let mut txn = new_txn(&mut world, 7);
            txn.set_component_agent_belief_store(listener, store)
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        assert_eq!(listener_store.get_entity(&subject).unwrap(), &existing);
        let heard = listener_store
            .heard_beliefs
            .get(&tell_memory_key(speaker, subject))
            .unwrap();
        assert_eq!(heard.heard_state, SharedTellState::EntityBelief(expected));
        assert_eq!(
            heard.disposition,
            HeardBeliefDisposition::AlreadyHeldEqualOrNewer
        );
    }

    #[test]
    fn tell_commit_trace_reports_redundant_noop() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let existing = build_believed_entity_state(
            &world,
            subject,
            Tick(2),
            PerceptionSource::DirectObservation,
        )
        .unwrap();
        {
            let mut store = world
                .get_component_agent_belief_store(listener)
                .cloned()
                .unwrap_or_default();
            store.update_entity(subject, existing);

            let mut txn = new_txn(&mut world, 7);
            txn.set_component_agent_belief_store(listener, store)
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        let outcome =
            commit_tell_result(&defs, &handlers, tell_id, &mut world, &instance, 1, 8).unwrap();

        assert_tell_trace(
            &outcome,
            TellCommitResult::AlreadyHeldEqualOrNewer,
            Some(HeardBeliefDisposition::AlreadyHeldEqualOrNewer),
            TellBeliefDeltaKind::None,
        );
    }

    #[test]
    fn tell_commit_records_listener_heard_belief_with_not_internalized() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let expected = {
            let speaker_store = world.get_component_agent_belief_store(speaker).unwrap();
            to_shared_belief_snapshot(speaker_store.get_entity(&subject).unwrap())
        };
        {
            let mut txn = new_txn(&mut world, 6);
            txn.set_component_tell_profile(
                listener,
                TellProfile {
                    acceptance_fidelity: Permille::new(0).unwrap(),
                    ..TellProfile::default()
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        assert!(listener_store.get_entity(&subject).is_none());
        let heard = listener_store
            .heard_beliefs
            .get(&tell_memory_key(speaker, subject))
            .unwrap();
        assert_eq!(heard.heard_state, SharedTellState::EntityBelief(expected));
        assert_eq!(heard.disposition, HeardBeliefDisposition::NotInternalized);
    }

    #[test]
    fn tell_commit_trace_reports_not_internalized() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        {
            let mut txn = new_txn(&mut world, 6);
            txn.set_component_tell_profile(
                listener,
                TellProfile {
                    acceptance_fidelity: Permille::new(0).unwrap(),
                    ..TellProfile::default()
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        let outcome =
            commit_tell_result(&defs, &handlers, tell_id, &mut world, &instance, 1, 8).unwrap();

        assert_tell_trace(
            &outcome,
            TellCommitResult::NotInternalized,
            Some(HeardBeliefDisposition::NotInternalized),
            TellBeliefDeltaKind::None,
        );
    }

    #[test]
    fn tell_commit_enforces_listener_memory_capacity() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let older_subject = {
            let place = world.topology().place_ids().next().unwrap();
            let mut txn = new_txn(&mut world, 4);
            let older_subject = txn.create_agent("OlderSubject", ControlSource::Ai).unwrap();
            txn.set_ground_location(older_subject, place).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            older_subject
        };
        let older_belief = build_believed_entity_state(
            &world,
            older_subject,
            Tick(1),
            PerceptionSource::DirectObservation,
        )
        .unwrap();
        {
            let mut store = world
                .get_component_agent_belief_store(listener)
                .cloned()
                .unwrap_or_default();
            store.update_entity(older_subject, older_belief);

            let mut txn = new_txn(&mut world, 6);
            txn.set_component_agent_belief_store(listener, store)
                .unwrap();
            txn.set_component_perception_profile(
                listener,
                PerceptionProfile {
                    memory_capacity: 1,
                    memory_retention_ticks: 100,
                    observation_fidelity: Permille::new(1000).unwrap(),
                    confidence_policy: BeliefConfidencePolicy::default(),
                    institutional_memory_capacity: 20,
                    consultation_speed_factor: Permille::new(500).unwrap(),
                    contradiction_tolerance: Permille::new(300).unwrap(),
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        assert!(listener_store.get_entity(&older_subject).is_none());
        assert!(listener_store.get_entity(&subject).is_some());
        assert_eq!(listener_store.known_entities.len(), 1);
    }

    #[test]
    fn tell_commit_projects_institutional_claims_and_records_them_in_heard_memory() {
        let (defs, handlers, tell_id, mut world, place, speaker, listener, office) =
            tell_office_test_setup(PerceptionSource::DirectObservation);
        let vacancy = office_holder_belief(
            office,
            None,
            InstitutionalKnowledgeSource::WitnessedEvent,
            4,
            Some(place),
        );
        let vacancy_claim = vacancy.claim;
        {
            let mut store = world
                .get_component_agent_belief_store(speaker)
                .cloned()
                .unwrap_or_default();
            let profile = *world.get_component_perception_profile(speaker).unwrap();
            store.record_institutional_belief(
                InstitutionalBeliefKey::OfficeHolderOf { office },
                vacancy.clone(),
                &profile,
            );
            let mut txn = new_txn(&mut world, 5);
            txn.set_component_agent_belief_store(speaker, store)
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = ActionInstance {
            instance_id: worldwake_sim::ActionInstanceId(0),
            def_id: tell_id,
            actor: speaker,
            targets: vec![listener],
            payload: ActionPayload::Tell(TellActionPayload {
                listener,
                topic: TellTopic::InstitutionalClaim {
                    claim: vacancy_claim,
                },
            }),
            start_tick: Tick(5),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        let received = listener_store
            .institutional_beliefs
            .get(&InstitutionalBeliefKey::OfficeHolderOf { office })
            .unwrap();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].claim, vacancy.claim);
        assert_eq!(
            received[0].source,
            InstitutionalKnowledgeSource::Report {
                from: speaker,
                chain_len: 1,
            }
        );
        let heard = listener_store
            .heard_beliefs
            .get(&TellMemoryKey {
                counterparty: speaker,
                topic: TellTopic::InstitutionalClaim {
                    claim: vacancy.claim,
                },
            })
            .unwrap();
        assert_eq!(heard.disposition, HeardBeliefDisposition::Accepted);
        let SharedTellState::InstitutionalClaim(heard_state) = &heard.heard_state else {
            panic!("expected institutional-claim tell state");
        };
        assert_eq!(heard_state.claim, vacancy.claim);
        assert_eq!(heard_state.source, vacancy.source);
    }

    #[test]
    fn tell_commit_entity_belief_no_longer_relays_institutional_claim_sidecars() {
        let (defs, handlers, tell_id, mut world, place, speaker, listener, office) =
            tell_office_test_setup(PerceptionSource::DirectObservation);
        let vacancy = office_holder_belief(
            office,
            None,
            InstitutionalKnowledgeSource::WitnessedEvent,
            4,
            Some(place),
        );
        {
            let mut store = world
                .get_component_agent_belief_store(speaker)
                .cloned()
                .unwrap_or_default();
            let profile = *world.get_component_perception_profile(speaker).unwrap();
            store.record_institutional_belief(
                InstitutionalBeliefKey::OfficeHolderOf { office },
                vacancy,
                &profile,
            );
            let mut txn = new_txn(&mut world, 5);
            txn.set_component_agent_belief_store(speaker, store)
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, office);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        assert!(
            listener_store.institutional_beliefs.is_empty(),
            "entity belief tells should not piggyback institutional claims once institutional topics are first-class"
        );
        let heard = listener_store
            .heard_beliefs
            .get(&tell_memory_key(speaker, office))
            .unwrap();
        let SharedTellState::EntityBelief(heard_state) = &heard.heard_state else {
            panic!("expected entity-belief tell state");
        };
        assert!(
            heard_state.last_known_place.is_some(),
            "entity tell state should still carry only the entity snapshot"
        );
    }

    #[test]
    fn tell_commit_relays_institutional_claims_with_incremented_chain_length() {
        let (defs, handlers, tell_id, mut world, place, speaker, listener, office) =
            tell_office_test_setup(PerceptionSource::DirectObservation);
        let vacancy = office_holder_belief(
            office,
            None,
            InstitutionalKnowledgeSource::Report {
                from: entity(90),
                chain_len: 1,
            },
            4,
            Some(place),
        );
        let vacancy_claim = vacancy.claim;
        {
            let mut store = world
                .get_component_agent_belief_store(speaker)
                .cloned()
                .unwrap_or_default();
            let profile = *world.get_component_perception_profile(speaker).unwrap();
            store.record_institutional_belief(
                InstitutionalBeliefKey::OfficeHolderOf { office },
                vacancy,
                &profile,
            );
            let mut txn = new_txn(&mut world, 5);
            txn.set_component_agent_belief_store(speaker, store)
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = ActionInstance {
            instance_id: worldwake_sim::ActionInstanceId(0),
            def_id: tell_id,
            actor: speaker,
            targets: vec![listener],
            payload: ActionPayload::Tell(TellActionPayload {
                listener,
                topic: TellTopic::InstitutionalClaim {
                    claim: vacancy_claim,
                },
            }),
            start_tick: Tick(5),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        let received = listener_store
            .institutional_beliefs
            .get(&InstitutionalBeliefKey::OfficeHolderOf { office })
            .unwrap();
        assert_eq!(received.len(), 1);
        assert_eq!(
            received[0].source,
            InstitutionalKnowledgeSource::Report {
                from: speaker,
                chain_len: 2,
            }
        );
    }

    #[test]
    fn tell_commit_does_not_duplicate_identical_institutional_claims_on_repeat_tell() {
        let (defs, handlers, tell_id, mut world, place, speaker, listener, office) =
            tell_office_test_setup(PerceptionSource::DirectObservation);
        let vacancy = office_holder_belief(
            office,
            None,
            InstitutionalKnowledgeSource::WitnessedEvent,
            4,
            Some(place),
        );
        let vacancy_claim = vacancy.claim;
        {
            let mut store = world
                .get_component_agent_belief_store(speaker)
                .cloned()
                .unwrap_or_default();
            let profile = *world.get_component_perception_profile(speaker).unwrap();
            store.record_institutional_belief(
                InstitutionalBeliefKey::OfficeHolderOf { office },
                vacancy,
                &profile,
            );
            let mut txn = new_txn(&mut world, 5);
            txn.set_component_agent_belief_store(speaker, store)
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let first = ActionInstance {
            instance_id: worldwake_sim::ActionInstanceId(0),
            def_id: tell_id,
            actor: speaker,
            targets: vec![listener],
            payload: ActionPayload::Tell(TellActionPayload {
                listener,
                topic: TellTopic::InstitutionalClaim {
                    claim: vacancy_claim,
                },
            }),
            start_tick: Tick(5),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };
        let second = ActionInstance {
            instance_id: worldwake_sim::ActionInstanceId(1),
            def_id: tell_id,
            actor: speaker,
            targets: vec![listener],
            payload: ActionPayload::Tell(TellActionPayload {
                listener,
                topic: TellTopic::InstitutionalClaim {
                    claim: vacancy_claim,
                },
            }),
            start_tick: Tick(6),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &first, 1, 8);
        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &second, 2, 9);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        let received = listener_store
            .institutional_beliefs
            .get(&InstitutionalBeliefKey::OfficeHolderOf { office })
            .unwrap();
        assert_eq!(received.len(), 1);
    }

    #[test]
    fn tell_commit_relays_force_control_claims() {
        let (defs, handlers, tell_id, mut world, place, speaker, listener, office) =
            tell_office_test_setup(PerceptionSource::DirectObservation);
        let contested = force_control_belief(
            office,
            None,
            true,
            InstitutionalKnowledgeSource::WitnessedEvent,
            4,
            Some(place),
        );
        {
            let mut store = world
                .get_component_agent_belief_store(speaker)
                .cloned()
                .unwrap_or_default();
            let profile = *world.get_component_perception_profile(speaker).unwrap();
            store.record_institutional_belief(
                InstitutionalBeliefKey::ForceControllerOf { office },
                contested.clone(),
                &profile,
            );
            let mut txn = new_txn(&mut world, 5);
            txn.set_component_agent_belief_store(speaker, store)
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = ActionInstance {
            instance_id: worldwake_sim::ActionInstanceId(0),
            def_id: tell_id,
            actor: speaker,
            targets: vec![listener],
            payload: ActionPayload::Tell(TellActionPayload {
                listener,
                topic: TellTopic::InstitutionalClaim {
                    claim: contested.claim,
                },
            }),
            start_tick: Tick(5),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        let received = listener_store
            .institutional_beliefs
            .get(&InstitutionalBeliefKey::ForceControllerOf { office })
            .unwrap();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].claim, contested.claim);
        assert_eq!(
            received[0].source,
            InstitutionalKnowledgeSource::Report {
                from: speaker,
                chain_len: 1,
            }
        );
    }

    #[test]
    fn tell_commit_rechecks_relay_limit_against_current_belief() {
        let report_source = PerceptionSource::Report {
            from: entity(77),
            chain_len: 2,
        };
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(report_source);
        {
            let mut txn = new_txn(&mut world, 6);
            txn.set_component_tell_profile(
                speaker,
                TellProfile {
                    max_tell_candidates: 3,
                    max_relay_chain_len: 1,
                    acceptance_fidelity: Permille::new(800).unwrap(),
                    ..TellProfile::default()
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        assert!(listener_store.get_entity(&subject).is_none());
    }

    #[test]
    fn tell_commit_trace_reports_relay_limit_rejection() {
        let report_source = PerceptionSource::Report {
            from: entity(77),
            chain_len: 2,
        };
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(report_source);
        {
            let mut txn = new_txn(&mut world, 6);
            txn.set_component_tell_profile(
                speaker,
                TellProfile {
                    max_tell_candidates: 3,
                    max_relay_chain_len: 1,
                    acceptance_fidelity: Permille::new(800).unwrap(),
                    ..TellProfile::default()
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        let outcome =
            commit_tell_result(&defs, &handlers, tell_id, &mut world, &instance, 1, 8).unwrap();

        assert_tell_trace(
            &outcome,
            TellCommitResult::RelayLimitExceeded,
            None,
            TellBeliefDeltaKind::None,
        );
    }

    #[test]
    fn tell_commit_relies_on_scheduler_event_transaction_shape() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let def = defs.get(tell_id).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let instance = tell_instance(tell_id, speaker, listener, subject);
        let mut rng = test_rng(1);
        let mut txn = new_action_txn(&mut world, speaker, def.visibility, 8);

        (handler.on_commit)(def, &instance, &mut rng, &mut txn).unwrap();
        txn.add_tag(EventTag::ActionCommitted);
        for tag in &def.causal_event_tags {
            txn.add_tag(*tag);
        }
        for target in &instance.targets {
            txn.add_target(*target);
        }

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.actor_id(), Some(speaker));
        assert_eq!(record.target_ids(), vec![listener]);
        assert_eq!(record.visibility(), VisibilitySpec::SamePlace);
        assert!(record.tags().contains(&EventTag::ActionCommitted));
        assert!(record.tags().contains(&EventTag::Social));
        assert!(record.tags().contains(&EventTag::WorldMutation));
    }

    #[test]
    fn tell_affordances_expand_live_colocated_listeners_across_relayable_subjects() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_tell_action(&mut defs, &mut handlers);
        let speaker = entity(1);
        let listener_a = entity(2);
        let listener_b = entity(3);
        let dead_listener = entity(4);
        let subject_a = entity(10);
        let subject_b = entity(11);
        let subject_c = entity(12);
        let place = entity(20);
        let mut view = StubTellBeliefView::default();

        for entity in [speaker, listener_a, listener_b, dead_listener] {
            view.kinds.insert(entity, EntityKind::Agent);
            view.places.insert(entity, place);
        }
        view.alive.insert(speaker, true);
        view.alive.insert(listener_a, true);
        view.alive.insert(listener_b, true);
        view.alive.insert(dead_listener, false);
        view.tell_profiles.insert(
            speaker,
            TellProfile {
                max_tell_candidates: 3,
                max_relay_chain_len: 3,
                acceptance_fidelity: Permille::new(800).unwrap(),
                ..TellProfile::default()
            },
        );
        view.beliefs.insert(
            speaker,
            vec![
                (
                    subject_a,
                    BelievedEntityState {
                        last_known_place: Some(entity(30)),
                        last_known_inventory: std::collections::BTreeMap::default(),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        last_known_courage: None,
                        observed_tick: Tick(2),
                        source: PerceptionSource::DirectObservation,
                    },
                ),
                (
                    subject_b,
                    BelievedEntityState {
                        last_known_place: Some(entity(31)),
                        last_known_inventory: std::collections::BTreeMap::default(),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        last_known_courage: None,
                        observed_tick: Tick(4),
                        source: PerceptionSource::Report {
                            from: entity(77),
                            chain_len: 2,
                        },
                    },
                ),
                (
                    subject_c,
                    BelievedEntityState {
                        last_known_place: Some(entity(32)),
                        last_known_inventory: std::collections::BTreeMap::default(),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        last_known_courage: None,
                        observed_tick: Tick(6),
                        source: PerceptionSource::Inference,
                    },
                ),
            ],
        );

        let affordances = collect_tell_affordances_from_view(&view, speaker, &defs, &handlers);

        assert_eq!(
            affordances,
            vec![
                (listener_a, TellTopic::EntityBelief { subject: subject_a }),
                (listener_a, TellTopic::EntityBelief { subject: subject_b }),
                (listener_a, TellTopic::EntityBelief { subject: subject_c }),
                (listener_b, TellTopic::EntityBelief { subject: subject_a }),
                (listener_b, TellTopic::EntityBelief { subject: subject_b }),
                (listener_b, TellTopic::EntityBelief { subject: subject_c }),
            ]
        );
    }

    #[test]
    fn tell_affordances_skip_already_told_current_belief_for_listener() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_tell_action(&mut defs, &mut handlers);
        let speaker = entity(1);
        let listener = entity(2);
        let current_subject = entity(10);
        let untold_subject = entity(11);
        let place = entity(20);
        let mut view = StubTellBeliefView::default();

        for entity in [speaker, listener] {
            view.kinds.insert(entity, EntityKind::Agent);
            view.places.insert(entity, place);
            view.alive.insert(entity, true);
        }
        view.tell_profiles.insert(speaker, TellProfile::default());
        view.beliefs.insert(
            speaker,
            vec![
                (
                    current_subject,
                    believed_state(9, entity(30), PerceptionSource::DirectObservation),
                ),
                (
                    untold_subject,
                    believed_state(8, entity(31), PerceptionSource::DirectObservation),
                ),
            ],
        );
        view.recipient_statuses.insert(
            (
                speaker,
                listener,
                TellTopic::EntityBelief {
                    subject: current_subject,
                },
            ),
            RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief,
        );

        let affordances = collect_tell_affordances_from_view(&view, speaker, &defs, &handlers);

        assert_eq!(
            affordances,
            vec![(
                listener,
                TellTopic::EntityBelief {
                    subject: untold_subject,
                },
            )]
        );
    }

    #[test]
    fn tell_affordances_reinclude_subject_when_prior_tell_is_stale_or_changed() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_tell_action(&mut defs, &mut handlers);
        let speaker = entity(1);
        let listener = entity(2);
        let stale_subject = entity(10);
        let expired_subject = entity(11);
        let place = entity(20);
        let mut view = StubTellBeliefView::default();

        for entity in [speaker, listener] {
            view.kinds.insert(entity, EntityKind::Agent);
            view.places.insert(entity, place);
            view.alive.insert(entity, true);
        }
        view.tell_profiles.insert(speaker, TellProfile::default());
        view.beliefs.insert(
            speaker,
            vec![
                (
                    stale_subject,
                    believed_state(9, entity(30), PerceptionSource::DirectObservation),
                ),
                (
                    expired_subject,
                    believed_state(8, entity(31), PerceptionSource::DirectObservation),
                ),
            ],
        );
        view.recipient_statuses.insert(
            (
                speaker,
                listener,
                TellTopic::EntityBelief {
                    subject: stale_subject,
                },
            ),
            RecipientKnowledgeStatus::SpeakerHasOnlyToldStaleBelief,
        );
        view.recipient_statuses.insert(
            (
                speaker,
                listener,
                TellTopic::EntityBelief {
                    subject: expired_subject,
                },
            ),
            RecipientKnowledgeStatus::SpeakerPreviouslyToldButMemoryExpired,
        );

        let affordances = collect_tell_affordances_from_view(&view, speaker, &defs, &handlers);

        assert_eq!(
            affordances,
            vec![
                (
                    listener,
                    TellTopic::EntityBelief {
                        subject: stale_subject,
                    },
                ),
                (
                    listener,
                    TellTopic::EntityBelief {
                        subject: expired_subject,
                    },
                ),
            ]
        );
    }

    #[test]
    fn tell_affordances_listener_aware_filtering_happens_before_truncation() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_tell_action(&mut defs, &mut handlers);
        let speaker = entity(1);
        let listener = entity(2);
        let crowded_subject = entity(10);
        let untold_subject = entity(11);
        let place = entity(20);
        let mut view = StubTellBeliefView::default();

        for entity in [speaker, listener] {
            view.kinds.insert(entity, EntityKind::Agent);
            view.places.insert(entity, place);
            view.alive.insert(entity, true);
        }
        view.tell_profiles.insert(
            speaker,
            TellProfile {
                max_tell_candidates: 1,
                ..TellProfile::default()
            },
        );
        view.beliefs.insert(
            speaker,
            vec![
                (
                    crowded_subject,
                    believed_state(10, entity(30), PerceptionSource::DirectObservation),
                ),
                (
                    untold_subject,
                    believed_state(8, entity(31), PerceptionSource::DirectObservation),
                ),
            ],
        );
        view.recipient_statuses.insert(
            (
                speaker,
                listener,
                TellTopic::EntityBelief {
                    subject: crowded_subject,
                },
            ),
            RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief,
        );

        let affordances = collect_tell_affordances_from_view(&view, speaker, &defs, &handlers);

        assert_eq!(
            affordances,
            vec![(
                listener,
                TellTopic::EntityBelief {
                    subject: untold_subject,
                },
            )]
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn tell_affordances_filter_relay_depth_and_limit_subjects_by_recency() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_tell_action(&mut defs, &mut handlers);
        let speaker = entity(1);
        let listener = entity(2);
        let subject_a = entity(10);
        let subject_b = entity(11);
        let subject_c = entity(12);
        let subject_d = entity(13);
        let subject_e = entity(14);
        let place = entity(20);
        let mut view = StubTellBeliefView::default();

        for entity in [speaker, listener] {
            view.kinds.insert(entity, EntityKind::Agent);
            view.places.insert(entity, place);
            view.alive.insert(entity, true);
        }
        view.tell_profiles.insert(
            speaker,
            TellProfile {
                max_tell_candidates: 3,
                max_relay_chain_len: 2,
                acceptance_fidelity: Permille::new(800).unwrap(),
                ..TellProfile::default()
            },
        );
        view.beliefs.insert(
            speaker,
            vec![
                (
                    subject_a,
                    BelievedEntityState {
                        last_known_place: Some(entity(30)),
                        last_known_inventory: std::collections::BTreeMap::default(),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        last_known_courage: None,
                        observed_tick: Tick(3),
                        source: PerceptionSource::DirectObservation,
                    },
                ),
                (
                    subject_b,
                    BelievedEntityState {
                        last_known_place: Some(entity(31)),
                        last_known_inventory: std::collections::BTreeMap::default(),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        last_known_courage: None,
                        observed_tick: Tick(9),
                        source: PerceptionSource::Report {
                            from: entity(80),
                            chain_len: 2,
                        },
                    },
                ),
                (
                    subject_c,
                    BelievedEntityState {
                        last_known_place: Some(entity(32)),
                        last_known_inventory: std::collections::BTreeMap::default(),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        last_known_courage: None,
                        observed_tick: Tick(9),
                        source: PerceptionSource::Inference,
                    },
                ),
                (
                    subject_d,
                    BelievedEntityState {
                        last_known_place: Some(entity(33)),
                        last_known_inventory: std::collections::BTreeMap::default(),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        last_known_courage: None,
                        observed_tick: Tick(7),
                        source: PerceptionSource::Rumor { chain_len: 3 },
                    },
                ),
                (
                    subject_e,
                    BelievedEntityState {
                        last_known_place: Some(entity(34)),
                        last_known_inventory: std::collections::BTreeMap::default(),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        last_known_courage: None,
                        observed_tick: Tick(5),
                        source: PerceptionSource::Rumor { chain_len: 1 },
                    },
                ),
            ],
        );

        let affordances = collect_tell_affordances_from_view(&view, speaker, &defs, &handlers);

        assert_eq!(
            affordances,
            vec![
                (listener, TellTopic::EntityBelief { subject: subject_b }),
                (listener, TellTopic::EntityBelief { subject: subject_c }),
                (listener, TellTopic::EntityBelief { subject: subject_e })
            ]
        );
    }

    #[test]
    fn tell_affordances_require_speaker_tell_profile() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_tell_action(&mut defs, &mut handlers);
        let speaker = entity(1);
        let listener = entity(2);
        let subject = entity(10);
        let place = entity(20);
        let mut view = StubTellBeliefView::default();

        for entity in [speaker, listener] {
            view.kinds.insert(entity, EntityKind::Agent);
            view.places.insert(entity, place);
            view.alive.insert(entity, true);
        }
        view.beliefs.insert(
            speaker,
            vec![(
                subject,
                BelievedEntityState {
                    last_known_place: Some(entity(30)),
                    last_known_inventory: std::collections::BTreeMap::default(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    observed_tick: Tick(3),
                    source: PerceptionSource::DirectObservation,
                },
            )],
        );

        let affordances = collect_tell_affordances_from_view(&view, speaker, &defs, &handlers);

        assert!(affordances.is_empty());
    }

    #[test]
    fn tell_affordances_include_relayable_social_observation_topics() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_tell_action(&mut defs, &mut handlers);
        let speaker = entity(1);
        let listener = entity(2);
        let subject = entity(10);
        let place = entity(20);
        let observation = worldwake_core::SocialObservation {
            detail: worldwake_core::SocialObservationDetail::SuspectedTheft {
                missing_entity: subject,
                expected_place: place,
                suspect: Some(entity(99)),
            },
            place,
            observed_tick: Tick(9),
            source: PerceptionSource::DirectObservation,
        };
        let mut view = StubTellBeliefView::default();

        for entity in [speaker, listener] {
            view.kinds.insert(entity, EntityKind::Agent);
            view.places.insert(entity, place);
            view.alive.insert(entity, true);
        }
        view.tell_profiles.insert(speaker, TellProfile::default());
        view.social_observations.insert(speaker, vec![observation]);

        let affordances = collect_tell_affordances_from_view(&view, speaker, &defs, &handlers);

        assert_eq!(
            affordances,
            vec![(listener, TellTopic::SocialObservation { observation })]
        );
    }

    #[test]
    fn tell_affordances_allow_same_place_office_entity_topics_without_claim_sidecars() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_tell_action(&mut defs, &mut handlers);
        let speaker = entity(1);
        let listener = entity(2);
        let office = entity(10);
        let place = entity(20);
        let mut view = StubTellBeliefView::default();

        for entity in [speaker, listener] {
            view.kinds.insert(entity, EntityKind::Agent);
            view.places.insert(entity, place);
            view.alive.insert(entity, true);
        }
        view.kinds.insert(office, EntityKind::Office);
        view.places.insert(office, place);
        view.alive.insert(office, true);
        view.tell_profiles.insert(speaker, TellProfile::default());
        view.beliefs.insert(
            speaker,
            vec![(
                office,
                BelievedEntityState {
                    last_known_place: Some(place),
                    last_known_inventory: std::collections::BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    observed_tick: Tick(9),
                    source: PerceptionSource::DirectObservation,
                },
            )],
        );

        let affordances = collect_tell_affordances_from_view(&view, speaker, &defs, &handlers);

        assert_eq!(
            affordances,
            vec![(listener, TellTopic::EntityBelief { subject: office })],
            "same-place office entity beliefs can still be shared as plain entity state once institutional claims are no longer piggybacked onto them"
        );
    }

    #[test]
    fn tell_affordances_include_local_institutional_claim_topics_even_when_office_is_visible() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_tell_action(&mut defs, &mut handlers);
        let speaker = entity(1);
        let listener = entity(2);
        let office = entity(10);
        let place = entity(20);
        let claim = InstitutionalClaim::OfficeHolder {
            office,
            holder: None,
            effective_tick: Tick(9),
        };
        let mut view = StubTellBeliefView::default();

        for entity in [speaker, listener] {
            view.kinds.insert(entity, EntityKind::Agent);
            view.places.insert(entity, place);
            view.alive.insert(entity, true);
        }
        view.kinds.insert(office, EntityKind::Office);
        view.places.insert(office, place);
        view.alive.insert(office, true);
        view.tell_profiles.insert(speaker, TellProfile::default());
        view.institutional_claims.insert(
            speaker,
            vec![BelievedInstitutionalClaim {
                claim,
                source: InstitutionalKnowledgeSource::WitnessedEvent,
                learned_tick: Tick(9),
                learned_at: Some(place),
            }],
        );

        let affordances = collect_tell_affordances_from_view(&view, speaker, &defs, &handlers);

        assert_eq!(
            affordances,
            vec![(listener, TellTopic::InstitutionalClaim { claim })]
        );
    }

    #[test]
    fn tell_affordances_exclude_social_observations_listener_participated_in() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_tell_action(&mut defs, &mut handlers);
        let speaker = entity(1);
        let listener = entity(2);
        let place = entity(20);
        let observation = worldwake_core::SocialObservation {
            detail: worldwake_core::SocialObservationDetail::WitnessedObligation {
                actor: speaker,
                target: listener,
            },
            place,
            observed_tick: Tick(9),
            source: PerceptionSource::DirectObservation,
        };
        let mut view = StubTellBeliefView::default();

        for entity in [speaker, listener] {
            view.kinds.insert(entity, EntityKind::Agent);
            view.places.insert(entity, place);
            view.alive.insert(entity, true);
        }
        view.tell_profiles.insert(speaker, TellProfile::default());
        view.social_observations.insert(speaker, vec![observation]);

        let affordances = collect_tell_affordances_from_view(&view, speaker, &defs, &handlers);

        assert!(
            affordances.is_empty(),
            "listeners should not receive tells about social observations they were part of"
        );
    }

    #[test]
    fn tell_payload_validator_accepts_known_social_observation_topic() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let def = defs.get(tell_id).unwrap();
        let (mut world, place, speaker, listener, subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);
        let observation = worldwake_core::SocialObservation {
            detail: worldwake_core::SocialObservationDetail::SuspectedTheft {
                missing_entity: subject,
                expected_place: place,
                suspect: Some(listener),
            },
            place,
            observed_tick: Tick(7),
            source: PerceptionSource::DirectObservation,
        };
        {
            let mut store = world
                .get_component_agent_belief_store(speaker)
                .cloned()
                .unwrap();
            store.record_social_observation(observation);
            let mut txn = new_txn(&mut world, 8);
            txn.set_component_agent_belief_store(speaker, store)
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        assert!(validate_tell_payload_authoritatively(
            def,
            &defs,
            speaker,
            &[listener],
            &ActionPayload::Tell(TellActionPayload {
                listener,
                topic: TellTopic::SocialObservation { observation },
            }),
            &world,
        )
        .is_ok());
    }

    #[test]
    fn tell_commit_transfers_social_observation_with_degraded_provenance() {
        let (defs, handlers, tell_id, mut world, place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let observation = worldwake_core::SocialObservation {
            detail: worldwake_core::SocialObservationDetail::SuspectedTheft {
                missing_entity: subject,
                expected_place: place,
                suspect: Some(entity(77)),
            },
            place,
            observed_tick: Tick(7),
            source: PerceptionSource::DirectObservation,
        };
        {
            let mut store = world
                .get_component_agent_belief_store(speaker)
                .cloned()
                .unwrap();
            store.record_social_observation(observation);
            let mut txn = new_txn(&mut world, 8);
            txn.set_component_agent_belief_store(speaker, store)
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        let instance = ActionInstance {
            instance_id: worldwake_sim::ActionInstanceId(0),
            def_id: tell_id,
            payload: ActionPayload::Tell(TellActionPayload {
                listener,
                topic: TellTopic::SocialObservation { observation },
            }),
            actor: speaker,
            targets: vec![listener],
            start_tick: Tick(8),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let speaker_store = world.get_component_agent_belief_store(speaker).unwrap();
        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        let told = speaker_store
            .told_beliefs
            .get(&TellMemoryKey {
                counterparty: listener,
                topic: TellTopic::SocialObservation { observation },
            })
            .unwrap();
        let heard = listener_store
            .heard_beliefs
            .get(&TellMemoryKey {
                counterparty: speaker,
                topic: TellTopic::SocialObservation { observation },
            })
            .unwrap();
        assert_eq!(
            told.shared_state,
            SharedTellState::SocialObservation(observation)
        );
        assert_eq!(
            heard.heard_state,
            SharedTellState::SocialObservation(observation)
        );
        assert_eq!(heard.disposition, HeardBeliefDisposition::Accepted);
        assert!(listener_store
            .social_observations
            .contains(&worldwake_core::SocialObservation {
                source: PerceptionSource::Report {
                    from: speaker,
                    chain_len: 1,
                },
                ..observation
            }));
    }
}
