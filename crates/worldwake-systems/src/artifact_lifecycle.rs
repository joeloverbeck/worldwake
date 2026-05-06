use crate::reward_encumbrance_support::release_bounty_reward;
use worldwake_core::{
    ArtifactActionability, ArtifactAxisValue, ArtifactCredibility, ArtifactKind,
    ArtifactLegalEffect, ArtifactTransitionPayload, AxisName, CauseRef, CloseCause, EntityKind,
    EventId, EventLog, EventTag, EventView, RewardSource, Tick, VisibilitySpec, WitnessData, World,
    WorldTxn,
};
use worldwake_sim::{SystemError, SystemExecutionContext};

pub fn artifact_lifecycle_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let SystemExecutionContext {
        world,
        event_log,
        rng: _rng,
        active_actions: _active_actions,
        action_defs: _action_defs,
        politics_trace: _,
        perception_trace: _,
        tick,
        system_id: _system_id,
    } = ctx;

    // Fixed artifact-axis stage order:
    // existence -> legal_effect -> credibility -> visibility -> actionability.
    // Only stages with live transition sources are active today; later S140 slices
    // can add sources inside the same ordered shell.
    existence_stage();
    legal_effect_stage(world, event_log, tick)?;
    credibility_stage();
    visibility_stage();
    actionability_stage(world, event_log, tick)?;

    Ok(())
}

fn existence_stage() {}

fn legal_effect_stage(
    world: &mut World,
    event_log: &mut EventLog,
    tick: Tick,
) -> Result<(), SystemError> {
    let expiring = world
        .query_artifact_header()
        .filter_map(|(artifact, header)| {
            (matches!(header.legal_effect, ArtifactLegalEffect::Active { .. })
                && header
                    .expires_at
                    .is_some_and(|expires_at| tick >= expires_at))
            .then_some((artifact, header.clone()))
        })
        .collect::<Vec<_>>();

    for (artifact, mut header) in expiring {
        let prior = header.legal_effect;
        let new = ArtifactLegalEffect::Expired { expired_at: tick };
        let place = world.effective_place(artifact);
        let mut txn = WorldTxn::new(
            world,
            tick,
            CauseRef::SystemTick(tick),
            None,
            place,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        header.legal_effect = new;
        set_transition_payload(
            &mut txn,
            ArtifactTransitionPayload {
                artifact,
                axis: AxisName::LegalEffect,
                prior: ArtifactAxisValue::LegalEffect(prior),
                new: ArtifactAxisValue::LegalEffect(new),
                cause_event: None,
                at: tick,
            },
        );
        txn.add_tag(EventTag::System)
            .add_tag(EventTag::Social)
            .add_tag(EventTag::WorldMutation)
            .add_target(artifact);
        if header.kind == ArtifactKind::Bounty
            && let Some(terms) = txn.get_component_bounty_terms(artifact).copied()
            && let RewardSource::InstitutionalTreasury { treasury_entity } = terms.reward_source
            && txn.entity_kind(treasury_entity) == Some(EntityKind::Office)
        {
            release_bounty_reward(&mut txn, treasury_entity, artifact)
                .map_err(|error| SystemError::new(error.to_string()))?;
        }
        txn.set_component_artifact_header(artifact, header)
            .map_err(|error| SystemError::new(error.to_string()))?;
        let _ = txn.commit(event_log);
    }

    Ok(())
}

fn credibility_stage() {}

fn visibility_stage() {}

fn actionability_stage(
    world: &mut World,
    event_log: &mut EventLog,
    tick: Tick,
) -> Result<(), SystemError> {
    let transitions = legal_or_credibility_transitions_at_tick(event_log, tick);

    for (cause_event, transition) in transitions {
        let Some(close_cause) = close_cause_for_transition(&transition) else {
            continue;
        };
        let artifact = transition.artifact;
        let Some(mut header) = world.get_component_artifact_header(artifact).cloned() else {
            continue;
        };
        if !matches!(header.actionability, ArtifactActionability::Actionable) {
            continue;
        }

        let prior = header.actionability;
        let new = ArtifactActionability::Closed {
            closed_at: tick,
            cause: close_cause,
        };
        header.actionability = new;

        let place = world.effective_place(artifact);
        let mut txn = WorldTxn::new(
            world,
            tick,
            CauseRef::Event(cause_event),
            None,
            place,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        set_transition_payload(
            &mut txn,
            ArtifactTransitionPayload {
                artifact,
                axis: AxisName::Actionability,
                prior: ArtifactAxisValue::Actionability(prior),
                new: ArtifactAxisValue::Actionability(new),
                cause_event: Some(cause_event),
                at: tick,
            },
        );
        txn.add_tag(EventTag::System)
            .add_tag(EventTag::Social)
            .add_tag(EventTag::WorldMutation)
            .add_target(artifact);
        txn.set_component_artifact_header(artifact, header)
            .map_err(|error| SystemError::new(error.to_string()))?;
        let _ = txn.commit(event_log);
    }

    Ok(())
}

fn set_transition_payload(txn: &mut WorldTxn<'_>, payload: ArtifactTransitionPayload) {
    txn.set_artifact_transition_payload(payload);
}

fn legal_or_credibility_transitions_at_tick(
    event_log: &EventLog,
    tick: Tick,
) -> Vec<(EventId, ArtifactTransitionPayload)> {
    event_log
        .events_at_tick(tick)
        .iter()
        .filter_map(|event_id| {
            let record = event_log.get(*event_id)?;
            let payload = record.artifact_transition_payload()?;
            matches!(payload.axis, AxisName::LegalEffect | AxisName::Credibility)
                .then(|| (*event_id, payload.clone()))
        })
        .collect()
}

fn close_cause_for_transition(payload: &ArtifactTransitionPayload) -> Option<CloseCause> {
    match &payload.new {
        ArtifactAxisValue::LegalEffect(ArtifactLegalEffect::Fulfilled { .. }) => {
            Some(CloseCause::BountyFulfilled)
        }
        ArtifactAxisValue::LegalEffect(ArtifactLegalEffect::Expired { .. }) => {
            Some(CloseCause::LegalEffectExpired)
        }
        ArtifactAxisValue::LegalEffect(ArtifactLegalEffect::Revoked { .. }) => {
            Some(CloseCause::Revoked)
        }
        ArtifactAxisValue::Credibility(ArtifactCredibility::Refuted { .. }) => {
            Some(CloseCause::Refuted)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::artifact_lifecycle_system;
    use worldwake_core::{
        ArtifactActionability, ArtifactAxisValue, ArtifactHeader, ArtifactKind,
        ArtifactLegalEffect, ArtifactTransitionPayload, AxisName, BountyTarget, BountyTerms,
        CauseRef, CloseCause, CommodityKind, ControlSource, EntityKind, EventLog, EventTag,
        EventView, NoticeContent, NoticeTopic, ProofRequirement, PrototypePlace, Quantity,
        RewardEncumbrance, RewardReservation, RewardSource, Seed, Tick, VisibilitySpec,
        WitnessData, World, WorldTxn, build_prototype_world, prototype_place_entity,
    };
    use worldwake_sim::{ActionDefRegistry, DeterministicRng, SystemExecutionContext, SystemId};

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

    fn transition_payloads(log: &EventLog) -> Vec<ArtifactTransitionPayload> {
        log.events_by_tag(EventTag::ArtifactTransition)
            .iter()
            .map(|event_id| {
                log.get(*event_id)
                    .and_then(EventView::artifact_transition_payload)
                    .cloned()
                    .expect("artifact transition event carries transition payload")
            })
            .collect()
    }

    fn spawn_agent_at(
        world: &mut World,
        slot: u32,
        place: worldwake_core::EntityId,
    ) -> worldwake_core::EntityId {
        let mut txn = new_txn(world, 1);
        let agent = txn
            .create_agent(&format!("agent-{slot}"), ControlSource::Ai)
            .unwrap();
        txn.set_ground_location(agent, place).unwrap();
        commit_txn(txn);
        agent
    }

    fn post_notice(
        world: &mut World,
        tick: u64,
        place: worldwake_core::EntityId,
        expires_at: Option<Tick>,
    ) -> worldwake_core::EntityId {
        let issuer = spawn_agent_at(world, 90 + tick as u32, place);
        let mut txn = new_txn(world, tick);
        let artifact = txn.create_entity(worldwake_core::EntityKind::SocialArtifact);
        txn.set_component_artifact_header(
            artifact,
            ArtifactHeader::posted_active(
                ArtifactKind::Notice,
                issuer,
                None,
                Tick(tick),
                expires_at,
                None,
                place,
            ),
        )
        .unwrap();
        txn.set_component_notice_content(
            artifact,
            NoticeContent {
                topic: NoticeTopic::ThreatWarning { place },
            },
        )
        .unwrap();
        txn.set_ground_location(artifact, place).unwrap();
        commit_txn(txn);
        artifact
    }

    fn post_institutional_bounty(
        world: &mut World,
        tick: u64,
        place: worldwake_core::EntityId,
        office: worldwake_core::EntityId,
        expires_at: Option<Tick>,
    ) -> worldwake_core::EntityId {
        let issuer = spawn_agent_at(world, 120 + tick as u32, place);
        let target = spawn_agent_at(world, 140 + tick as u32, place);
        let mut txn = new_txn(world, tick);
        let artifact = txn.create_entity(EntityKind::SocialArtifact);
        txn.set_component_artifact_header(
            artifact,
            ArtifactHeader::posted_active(
                ArtifactKind::Bounty,
                issuer,
                Some(office),
                Tick(tick),
                expires_at,
                Some(place),
                place,
            ),
        )
        .unwrap();
        txn.set_component_bounty_terms(
            artifact,
            BountyTerms {
                target: BountyTarget::EliminateEntity { target },
                proof_requirement: ProofRequirement::SelfReport,
                reward_commodity: CommodityKind::Coin,
                reward_quantity: Quantity(4),
                reward_source: RewardSource::InstitutionalTreasury {
                    treasury_entity: office,
                },
                claim_place: place,
            },
        )
        .unwrap();
        txn.set_ground_location(artifact, place).unwrap();
        txn.set_component_reward_encumbrance(
            office,
            RewardEncumbrance::from_reservation(RewardReservation {
                bounty_artifact: artifact,
                commodity: CommodityKind::Coin,
                quantity: Quantity(4),
            }),
        )
        .unwrap();
        commit_txn(txn);
        artifact
    }

    #[test]
    fn artifact_lifecycle_system_expires_active_artifact_at_expiration_tick() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let artifact = post_notice(&mut world, 2, square, Some(Tick(5)));
        let mut log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([7; 32]));

        artifact_lifecycle_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut log,
            rng: &mut rng,
            active_actions: &std::collections::BTreeMap::new(),
            action_defs: &ActionDefRegistry::new(),
            politics_trace: None,
            perception_trace: None,
            tick: Tick(5),
            system_id: SystemId::ArtifactLifecycle,
        })
        .unwrap();

        assert_eq!(
            world
                .get_component_artifact_header(artifact)
                .unwrap()
                .legal_effect,
            ArtifactLegalEffect::Expired {
                expired_at: Tick(5)
            }
        );
        assert_eq!(
            world
                .get_component_artifact_header(artifact)
                .unwrap()
                .actionability,
            ArtifactActionability::Closed {
                closed_at: Tick(5),
                cause: CloseCause::LegalEffectExpired,
            }
        );
        let transition_ids = log.events_by_tag(EventTag::ArtifactTransition);
        assert_eq!(transition_ids.len(), 2);
        let transitions = transition_payloads(&log);
        assert_eq!(transitions[0].axis, AxisName::LegalEffect);
        assert_eq!(
            transitions[0].new,
            ArtifactAxisValue::LegalEffect(ArtifactLegalEffect::Expired {
                expired_at: Tick(5)
            })
        );
        assert_eq!(transitions[1].axis, AxisName::Actionability);
        assert_eq!(transitions[1].cause_event, Some(transition_ids[0]));
        assert_eq!(
            transitions[1].new,
            ArtifactAxisValue::Actionability(ArtifactActionability::Closed {
                closed_at: Tick(5),
                cause: CloseCause::LegalEffectExpired,
            })
        );
        assert_eq!(log.events_by_tag(EventTag::WorldMutation).len(), 2);
    }

    #[test]
    fn artifact_lifecycle_system_leaves_nonexpiring_artifact_active() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let artifact = post_notice(&mut world, 2, square, None);
        let mut log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([8; 32]));

        artifact_lifecycle_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut log,
            rng: &mut rng,
            active_actions: &std::collections::BTreeMap::new(),
            action_defs: &ActionDefRegistry::new(),
            politics_trace: None,
            perception_trace: None,
            tick: Tick(9),
            system_id: SystemId::ArtifactLifecycle,
        })
        .unwrap();

        assert_eq!(
            world
                .get_component_artifact_header(artifact)
                .unwrap()
                .actionability,
            ArtifactActionability::Actionable
        );
        assert!(log.is_empty());
    }

    #[test]
    fn artifact_lifecycle_system_does_not_expire_before_expiration_tick() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let artifact = post_notice(&mut world, 2, square, Some(Tick(8)));
        let mut log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([9; 32]));

        artifact_lifecycle_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut log,
            rng: &mut rng,
            active_actions: &std::collections::BTreeMap::new(),
            action_defs: &ActionDefRegistry::new(),
            politics_trace: None,
            perception_trace: None,
            tick: Tick(7),
            system_id: SystemId::ArtifactLifecycle,
        })
        .unwrap();

        assert_eq!(
            world
                .get_component_artifact_header(artifact)
                .unwrap()
                .actionability,
            ArtifactActionability::Actionable
        );
        assert!(log.is_empty());
    }

    #[test]
    fn bounty_ttl_expiry_releases_encumbrance() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let office = {
            let mut txn = new_txn(&mut world, 1);
            let office = txn.create_office("Market Warden").unwrap();
            commit_txn(txn);
            office
        };
        let bounty = post_institutional_bounty(&mut world, 2, square, office, Some(Tick(5)));
        assert!(world.has_component_reward_encumbrance(office));
        let mut log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([10; 32]));

        artifact_lifecycle_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut log,
            rng: &mut rng,
            active_actions: &std::collections::BTreeMap::new(),
            action_defs: &ActionDefRegistry::new(),
            politics_trace: None,
            perception_trace: None,
            tick: Tick(5),
            system_id: SystemId::ArtifactLifecycle,
        })
        .unwrap();

        assert_eq!(
            world
                .get_component_artifact_header(bounty)
                .unwrap()
                .legal_effect,
            ArtifactLegalEffect::Expired {
                expired_at: Tick(5)
            }
        );
        assert!(!world.has_component_reward_encumbrance(office));
        assert_eq!(log.events_by_tag(EventTag::ArtifactTransition).len(), 2);
        assert_eq!(log.events_by_tag(EventTag::WorldMutation).len(), 2);
    }
}
