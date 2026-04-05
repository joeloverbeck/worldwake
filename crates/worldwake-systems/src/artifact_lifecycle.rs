use worldwake_core::{
    ArtifactState, CauseRef, EventTag, VisibilitySpec, WitnessData, WorldTxn,
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
                && header.expires_at.is_some_and(|expires_at| tick >= expires_at))
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
        build_prototype_world, prototype_place_entity, ArtifactHeader, ArtifactKind, ArtifactState,
        CauseRef, ControlSource, EventLog, EventTag, NoticeContent, NoticeTopic, PrototypePlace,
        Seed, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
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

    fn spawn_agent_at(world: &mut World, slot: u32, place: worldwake_core::EntityId) -> worldwake_core::EntityId {
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
}
