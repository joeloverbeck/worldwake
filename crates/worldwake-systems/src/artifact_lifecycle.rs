use crate::reward_encumbrance_support::release_bounty_reward;
use worldwake_core::social_artifact::SuspensionReason;
use worldwake_core::{
    ArtifactActionability, ArtifactAxisValue, ArtifactCredibility, ArtifactKind,
    ArtifactLegalEffect, ArtifactTransitionPayload, AxisName, CauseRef, CloseCause, ComponentDelta,
    ComponentValue, EntityId, EntityKind, EventId, EventLog, EventTag, EventView,
    InstitutionalClaim, RewardSource, StateDelta, Tick, VisibilitySpec, WitnessData, World,
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
    let source_events = force_control_source_events_at_tick(event_log, tick);
    for source in source_events {
        apply_force_control_source_event(world, event_log, tick, source)?;
    }

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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct ForceControlSourceEvent {
    event_id: EventId,
    office: EntityId,
    contested: bool,
}

fn force_control_source_events_at_tick(
    event_log: &EventLog,
    tick: Tick,
) -> Vec<ForceControlSourceEvent> {
    event_log
        .events_at_tick(tick)
        .iter()
        .filter_map(|event_id| event_log.get(*event_id).map(|record| (*event_id, record)))
        .flat_map(|(event_id, record)| {
            record
                .state_deltas()
                .iter()
                .flat_map(move |delta| new_force_control_claims(event_id, delta))
        })
        .collect()
}

fn new_force_control_claims(event_id: EventId, delta: &StateDelta) -> Vec<ForceControlSourceEvent> {
    let StateDelta::Component(ComponentDelta::Set {
        before,
        after: ComponentValue::RecordData(after),
        ..
    }) = delta
    else {
        return Vec::new();
    };
    let before_len = before
        .as_ref()
        .and_then(|value| match value {
            ComponentValue::RecordData(before) => Some(before.entries.len()),
            _ => None,
        })
        .unwrap_or(0);

    after
        .entries
        .iter()
        .skip(before_len)
        .filter_map(|entry| match entry.claim {
            InstitutionalClaim::ForceControl {
                office, contested, ..
            } => Some(ForceControlSourceEvent {
                event_id,
                office,
                contested,
            }),
            _ => None,
        })
        .collect()
}

fn apply_force_control_source_event(
    world: &mut World,
    event_log: &mut EventLog,
    tick: Tick,
    source: ForceControlSourceEvent,
) -> Result<(), SystemError> {
    let affected = world
        .query_artifact_header()
        .filter_map(|(artifact, header)| {
            artifact_authority_matches(header, source.office).then_some((artifact, header.clone()))
        })
        .collect::<Vec<_>>();

    for (artifact, mut header) in affected {
        let new = if source.contested {
            if !matches!(header.legal_effect, ArtifactLegalEffect::Active { .. }) {
                continue;
            }
            ArtifactLegalEffect::Suspended {
                reason: SuspensionReason::JurisdictionDispute,
                suspended_at: tick,
            }
        } else if matches!(
            header.legal_effect,
            ArtifactLegalEffect::Suspended {
                reason: SuspensionReason::JurisdictionDispute,
                ..
            }
        ) {
            ArtifactLegalEffect::Active {
                expires_at: header.expires_at,
            }
        } else {
            continue;
        };

        let prior = header.legal_effect;
        header.legal_effect = new;
        let place = world.effective_place(artifact);
        let mut txn = WorldTxn::new(
            world,
            tick,
            CauseRef::Event(source.event_id),
            None,
            place,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        set_transition_payload(
            &mut txn,
            ArtifactTransitionPayload {
                artifact,
                axis: AxisName::LegalEffect,
                prior: ArtifactAxisValue::LegalEffect(prior),
                new: ArtifactAxisValue::LegalEffect(new),
                cause_event: Some(source.event_id),
                at: tick,
            },
        );
        txn.add_tag(EventTag::System)
            .add_tag(EventTag::Social)
            .add_tag(EventTag::WorldMutation)
            .add_target(artifact)
            .add_target(source.office);
        txn.set_component_artifact_header(artifact, header)
            .map_err(|error| SystemError::new(error.to_string()))?;
        let _ = txn.commit(event_log);
    }

    Ok(())
}

fn artifact_authority_matches(header: &worldwake_core::ArtifactHeader, office: EntityId) -> bool {
    header.issuing_authority == Some(office) || header.jurisdiction == Some(office)
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
        EventView, InstitutionalClaim, NoticeContent, NoticeTopic, ProofRequirement,
        PrototypePlace, Quantity, RecordData, RecordKind, RewardEncumbrance, RewardReservation,
        RewardSource, Seed, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
        build_prototype_world, prototype_place_entity, social_artifact::SuspensionReason,
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

    fn create_record(
        world: &mut World,
        tick: u64,
        place: worldwake_core::EntityId,
        issuer: worldwake_core::EntityId,
        kind: RecordKind,
    ) -> worldwake_core::EntityId {
        let mut txn = new_txn(world, tick);
        let record = txn
            .create_record(RecordData {
                record_kind: kind,
                home_place: place,
                issuer,
                consultation_ticks: 1,
                max_entries_per_consult: 8,
                entries: Vec::new(),
                next_entry_id: 0,
            })
            .unwrap();
        commit_txn(txn);
        record
    }

    fn append_force_control_claim(
        world: &mut World,
        log: &mut EventLog,
        tick: u64,
        record: worldwake_core::EntityId,
        office: worldwake_core::EntityId,
        contested: bool,
    ) -> worldwake_core::EventId {
        let mut txn = new_txn(world, tick);
        txn.append_record_entry(
            record,
            InstitutionalClaim::ForceControl {
                office,
                controller: None,
                contested,
                effective_tick: Tick(tick),
            },
        )
        .unwrap();
        txn.add_tag(EventTag::Social).add_tag(EventTag::Control);
        txn.commit(log)
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
    fn force_control_contest_source_event_suspends_office_artifacts() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let issuer = spawn_agent_at(&mut world, 50, square);
        let office = {
            let mut txn = new_txn(&mut world, 1);
            let office = txn.create_office("Market Warden").unwrap();
            commit_txn(txn);
            office
        };
        let record = create_record(&mut world, 2, square, issuer, RecordKind::OfficeRegister);
        let artifact = post_institutional_bounty(&mut world, 3, square, office, None);
        let mut log = EventLog::new();
        let source_event =
            append_force_control_claim(&mut world, &mut log, 5, record, office, true);
        let mut rng = DeterministicRng::new(Seed([11; 32]));

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
            ArtifactLegalEffect::Suspended {
                reason: SuspensionReason::JurisdictionDispute,
                suspended_at: Tick(5),
            }
        );
        assert_eq!(
            world
                .get_component_artifact_header(artifact)
                .unwrap()
                .actionability,
            ArtifactActionability::Actionable
        );
        let transition_ids = log.events_by_tag(EventTag::ArtifactTransition);
        assert_eq!(transition_ids.len(), 1);
        let transitions = transition_payloads(&log);
        assert_eq!(transitions[0].axis, AxisName::LegalEffect);
        assert_eq!(transitions[0].cause_event, Some(source_event));
        assert_eq!(
            transitions[0].new,
            ArtifactAxisValue::LegalEffect(ArtifactLegalEffect::Suspended {
                reason: SuspensionReason::JurisdictionDispute,
                suspended_at: Tick(5),
            })
        );
    }

    #[test]
    fn force_control_resolution_source_event_restores_suspended_office_artifacts() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let issuer = spawn_agent_at(&mut world, 51, square);
        let office = {
            let mut txn = new_txn(&mut world, 1);
            let office = txn.create_office("Market Warden").unwrap();
            commit_txn(txn);
            office
        };
        let record = create_record(&mut world, 2, square, issuer, RecordKind::OfficeRegister);
        let artifact = post_institutional_bounty(&mut world, 3, square, office, Some(Tick(20)));
        let mut log = EventLog::new();
        append_force_control_claim(&mut world, &mut log, 5, record, office, true);
        let mut rng = DeterministicRng::new(Seed([12; 32]));

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
        let source_event =
            append_force_control_claim(&mut world, &mut log, 6, record, office, false);
        artifact_lifecycle_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut log,
            rng: &mut rng,
            active_actions: &std::collections::BTreeMap::new(),
            action_defs: &ActionDefRegistry::new(),
            politics_trace: None,
            perception_trace: None,
            tick: Tick(6),
            system_id: SystemId::ArtifactLifecycle,
        })
        .unwrap();

        assert_eq!(
            world
                .get_component_artifact_header(artifact)
                .unwrap()
                .legal_effect,
            ArtifactLegalEffect::Active {
                expires_at: Some(Tick(20)),
            }
        );
        assert_eq!(
            world
                .get_component_artifact_header(artifact)
                .unwrap()
                .actionability,
            ArtifactActionability::Actionable
        );
        let transitions = transition_payloads(&log);
        assert_eq!(transitions.len(), 2);
        assert_eq!(transitions[1].cause_event, Some(source_event));
        assert_eq!(
            transitions[1].new,
            ArtifactAxisValue::LegalEffect(ArtifactLegalEffect::Active {
                expires_at: Some(Tick(20)),
            })
        );
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
