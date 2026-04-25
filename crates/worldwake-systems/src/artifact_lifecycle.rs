use crate::reward_encumbrance_support::release_bounty_reward;
use worldwake_core::{
    ArtifactKind, ArtifactState, CauseRef, EntityKind, EventTag, RewardSource, VisibilitySpec,
    WitnessData, WorldTxn,
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

    let expiring = world
        .query_artifact_header()
        .filter_map(|(artifact, header)| {
            (header.state == ArtifactState::Active
                && header
                    .expires_at
                    .is_some_and(|expires_at| tick >= expires_at))
            .then_some((artifact, *header))
        })
        .collect::<Vec<_>>();

    for (artifact, mut header) in expiring {
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
        header.state = ArtifactState::Expired;
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

#[cfg(test)]
mod tests {
    use super::artifact_lifecycle_system;
    use worldwake_core::{
        ArtifactHeader, ArtifactKind, ArtifactState, BountyTarget, BountyTerms, CauseRef,
        CommodityKind, ControlSource, EntityKind, EventLog, EventTag, NoticeContent, NoticeTopic,
        ProofRequirement, PrototypePlace, Quantity, RewardEncumbrance, RewardReservation,
        RewardSource, Seed, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
        build_prototype_world, prototype_place_entity,
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
            ArtifactHeader {
                kind: ArtifactKind::Notice,
                issuer,
                issuing_authority: None,
                created_at: Tick(tick),
                expires_at,
                state: ArtifactState::Active,
                jurisdiction: None,
            },
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
            ArtifactHeader {
                kind: ArtifactKind::Bounty,
                issuer,
                issuing_authority: Some(office),
                created_at: Tick(tick),
                expires_at,
                state: ArtifactState::Active,
                jurisdiction: Some(place),
            },
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
            world.get_component_artifact_header(artifact).unwrap().state,
            ArtifactState::Expired
        );
        assert_eq!(log.events_by_tag(EventTag::WorldMutation).len(), 1);
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
            world.get_component_artifact_header(artifact).unwrap().state,
            ArtifactState::Active
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
            world.get_component_artifact_header(artifact).unwrap().state,
            ArtifactState::Active
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
            world.get_component_artifact_header(bounty).unwrap().state,
            ArtifactState::Expired
        );
        assert!(!world.has_component_reward_encumbrance(office));
        assert_eq!(log.events_by_tag(EventTag::WorldMutation).len(), 1);
    }
}
