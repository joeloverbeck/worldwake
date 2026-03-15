//! Golden tests for social information transfer, relay degradation, and discovery correction.

mod golden_harness;

use golden_harness::*;
use worldwake_core::{
    belief_confidence, build_believed_entity_state, hash_event_log, hash_world,
    verify_authoritative_conservation, CommodityKind, EntityId, EventTag, EventView, EvidenceRef,
    HomeostaticNeeds, MismatchKind, PerceptionProfile, PerceptionSource, Quantity, ResourceSource,
    Seed, SocialObservationKind, TellProfile, Tick, UtilityProfile, WorkstationTag,
};

fn social_weighted_utility(weight: u16) -> UtilityProfile {
    UtilityProfile {
        social_weight: pm(weight),
        ..UtilityProfile::default()
    }
}

fn blind_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        memory_capacity: 16,
        memory_retention_ticks: 240,
        observation_fidelity: pm(0),
        confidence_policy: worldwake_core::BeliefConfidencePolicy::default(),
    }
}

fn keen_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        memory_capacity: 32,
        memory_retention_ticks: 240,
        observation_fidelity: pm(1000),
        confidence_policy: worldwake_core::BeliefConfidencePolicy::default(),
    }
}

fn accepting_tell_profile() -> TellProfile {
    TellProfile {
        max_tell_candidates: 3,
        max_relay_chain_len: 3,
        acceptance_fidelity: pm(1000),
    }
}

fn rejecting_tell_profile() -> TellProfile {
    TellProfile {
        max_tell_candidates: 3,
        max_relay_chain_len: 3,
        acceptance_fidelity: pm(0),
    }
}

fn focused_accepting_tell_profile() -> TellProfile {
    TellProfile {
        max_tell_candidates: 1,
        ..accepting_tell_profile()
    }
}

fn ensure_empty_belief_store(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
) {
    let mut txn = new_txn(world, 0);
    txn.set_component_agent_belief_store(agent, worldwake_core::AgentBeliefStore::new())
        .expect("golden social test should keep belief stores writable");
    commit_txn(txn, event_log);
}

fn saw_inventory_discovery(
    log: &worldwake_core::EventLog,
    observer: EntityId,
    subject: EntityId,
    commodity: CommodityKind,
) -> bool {
    log.events_by_tag(EventTag::Discovery).iter().any(|event_id| {
        log.get(*event_id).is_some_and(|event| {
            event.evidence().iter().any(|evidence| {
                matches!(
                    evidence,
                    EvidenceRef::Mismatch {
                        observer: evidence_observer,
                        subject: evidence_subject,
                        kind: MismatchKind::InventoryDiscrepancy { commodity: mismatch_commodity, .. }
                            | MismatchKind::ResourceSourceDiscrepancy {
                                commodity: mismatch_commodity,
                                ..
                            },
                    } if *evidence_observer == observer
                        && *evidence_subject == subject
                        && *mismatch_commodity == commodity
                )
            })
        })
    })
}

fn saw_entity_missing_discovery(
    log: &worldwake_core::EventLog,
    observer: EntityId,
    subject: EntityId,
) -> bool {
    log.events_by_tag(EventTag::Discovery).iter().any(|event_id| {
        log.get(*event_id).is_some_and(|event| {
            event.evidence().iter().any(|evidence| {
                matches!(
                    evidence,
                    EvidenceRef::Mismatch {
                        observer: evidence_observer,
                        subject: evidence_subject,
                        kind: MismatchKind::EntityMissing,
                    } if *evidence_observer == observer && *evidence_subject == subject
                )
            })
        })
    })
}

fn run_until(limit: usize, mut step: impl FnMut() -> bool) -> bool {
    for _ in 0..limit {
        if step() {
            return true;
        }
    }
    false
}

#[allow(clippy::too_many_lines)]
fn run_autonomous_tell_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, build_recipes());

    let speaker = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Speaker",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        social_weighted_utility(900),
    );
    let listener = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Listener",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let orchard = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(10),
            max_quantity: Quantity(10),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
    );

    ensure_empty_belief_store(&mut h.world, &mut h.event_log, speaker);
    ensure_empty_belief_store(&mut h.world, &mut h.event_log, listener);
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        speaker,
        focused_accepting_tell_profile(),
    );
    set_agent_tell_profile(&mut h.world, &mut h.event_log, listener, accepting_tell_profile());
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        speaker,
        blind_perception_profile(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        keen_perception_profile(),
    );

    let listener_belief = build_believed_entity_state(
        &h.world,
        listener,
        Tick(0),
        PerceptionSource::DirectObservation,
    )
    .expect("listener should be observable for tell targeting");
    seed_belief(&mut h.world, &mut h.event_log, speaker, listener, listener_belief);
    let orchard_belief = build_believed_entity_state(
        &h.world,
        orchard,
        Tick(1),
        PerceptionSource::DirectObservation,
    )
    .expect("orchard should be observable for belief seeding");
    seed_belief(&mut h.world, &mut h.event_log, speaker, orchard, orchard_belief);

    let mut saw_social_event = false;
    let mut saw_report_belief = false;
    let mut left_village = false;
    let mut reached_orchard = false;

    for _ in 0..120 {
        h.step_once();
        verify_authoritative_conservation(&h.world, CommodityKind::Apple, 10).unwrap();

        saw_social_event |= !h.event_log.events_by_tag(EventTag::Social).is_empty();
        if let Some(belief) = agent_belief_about(&h.world, listener, orchard) {
            saw_report_belief |= matches!(
                belief.source,
                PerceptionSource::Report {
                    from,
                    chain_len: 1
                } if from == speaker
            );
        }

        let place = h.world.effective_place(listener);
        left_village |= place != Some(VILLAGE_SQUARE) || h.world.is_in_transit(listener);
        reached_orchard |= place == Some(ORCHARD_FARM);

        if saw_social_event && saw_report_belief && left_village && reached_orchard {
            break;
        }
    }

    assert!(saw_social_event, "speaker should execute a social tell event");
    assert!(
        saw_report_belief,
        "listener should receive a reported belief about the orchard"
    );
    assert!(
        left_village,
        "listener should leave Village Square after learning about the remote orchard"
    );
    assert!(
        reached_orchard,
        "listener should replan toward Orchard Farm after receiving the told belief"
    );

    (hash_world(&h.world).unwrap(), hash_event_log(&h.event_log).unwrap())
}

#[allow(clippy::too_many_lines)]
fn run_rumor_chain_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, build_recipes());

    let alice = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Alice",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        social_weighted_utility(900),
    );
    let bob = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Bob",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        social_weighted_utility(900),
    );
    let carol = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Carol",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let subject = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Subject",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    for agent in [alice, bob, carol] {
        ensure_empty_belief_store(&mut h.world, &mut h.event_log, agent);
        set_agent_tell_profile(
            &mut h.world,
            &mut h.event_log,
            agent,
            focused_accepting_tell_profile(),
        );
        set_agent_perception_profile(
            &mut h.world,
            &mut h.event_log,
            agent,
            blind_perception_profile(),
        );
    }

    let bob_belief = build_believed_entity_state(
        &h.world,
        bob,
        Tick(0),
        PerceptionSource::DirectObservation,
    )
    .expect("Bob should be observable for relay targeting");
    seed_belief(&mut h.world, &mut h.event_log, alice, bob, bob_belief);
    let subject_belief = build_believed_entity_state(
        &h.world,
        subject,
        Tick(1),
        PerceptionSource::DirectObservation,
    )
    .expect("Subject should be observable for relay seeding");
    seed_belief(&mut h.world, &mut h.event_log, alice, subject, subject_belief);

    let carol_belief = build_believed_entity_state(
        &h.world,
        carol,
        Tick(0),
        PerceptionSource::DirectObservation,
    )
    .expect("Carol should be observable for relay targeting");
    seed_belief(&mut h.world, &mut h.event_log, bob, carol, carol_belief);

    let bob_updated = run_until(40, || {
        h.step_once();
        agent_belief_about(&h.world, bob, subject).is_some()
    });
    assert!(bob_updated, "Bob should receive Alice's told belief");

    let carol_updated = run_until(40, || {
        h.step_once();
        agent_belief_about(&h.world, carol, subject).is_some()
    });
    assert!(carol_updated, "Carol should receive Bob's relayed belief");

    let alice_belief = agent_belief_about(&h.world, alice, subject).unwrap();
    let bob_belief = agent_belief_about(&h.world, bob, subject).unwrap();
    let carol_belief = agent_belief_about(&h.world, carol, subject).unwrap();

    assert_eq!(alice_belief.source, PerceptionSource::DirectObservation);
    assert_eq!(
        bob_belief.source,
        PerceptionSource::Report {
            from: alice,
            chain_len: 1
        }
    );
    assert_eq!(carol_belief.source, PerceptionSource::Rumor { chain_len: 2 });

    let policy = keen_perception_profile().confidence_policy;
    let current_tick = h.scheduler.current_tick();
    let alice_confidence = belief_confidence(
        &alice_belief.source,
        current_tick.0.saturating_sub(alice_belief.observed_tick.0),
        &policy,
    );
    let bob_confidence = belief_confidence(
        &bob_belief.source,
        current_tick.0.saturating_sub(bob_belief.observed_tick.0),
        &policy,
    );
    let carol_confidence = belief_confidence(
        &carol_belief.source,
        current_tick.0.saturating_sub(carol_belief.observed_tick.0),
        &policy,
    );

    assert!(
        alice_confidence > bob_confidence && bob_confidence > carol_confidence,
        "relay provenance should monotonically degrade confidence: alice={alice_confidence:?}, bob={bob_confidence:?}, carol={carol_confidence:?}"
    );

    (hash_world(&h.world).unwrap(), hash_event_log(&h.event_log).unwrap())
}

fn run_stale_belief_replan_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, build_recipes());

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Forager",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(&mut h.world, &mut h.event_log, agent, keen_perception_profile());

    let orchard = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(10),
            max_quantity: Quantity(10),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
    );

    let stale_belief = build_believed_entity_state(
        &h.world,
        orchard,
        Tick(0),
        PerceptionSource::DirectObservation,
    )
    .expect("orchard workstation should be observable");
    seed_belief(&mut h.world, &mut h.event_log, agent, orchard, stale_belief);

    let mut txn = new_txn(&mut h.world, 1);
    txn.set_component_resource_source(
        orchard,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(0),
            max_quantity: Quantity(10),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);

    let mut saw_travel = false;
    let mut reached_orchard = false;
    let mut saw_discovery = false;
    let mut corrected_belief = false;
    let mut started_harvest_after_discovery = false;

    for _ in 0..120 {
        h.step_once();
        verify_authoritative_conservation(&h.world, CommodityKind::Apple, 0).unwrap();

        let place = h.world.effective_place(agent);
        saw_travel |= place != Some(VILLAGE_SQUARE) || h.world.is_in_transit(agent);
        reached_orchard |= place == Some(ORCHARD_FARM);
        saw_discovery |=
            saw_inventory_discovery(&h.event_log, agent, orchard, CommodityKind::Apple);
        if let Some(belief) = agent_belief_about(&h.world, agent, orchard) {
            corrected_belief |= belief.source == PerceptionSource::DirectObservation
                && belief
                    .resource_source
                    .as_ref()
                    .is_some_and(|source| source.available_quantity == Quantity(0));
        }
        if saw_discovery && h.agent_active_action_name(agent) == Some("harvest") {
            started_harvest_after_discovery = true;
        }

        if saw_travel && reached_orchard && saw_discovery && corrected_belief {
            break;
        }
    }

    assert!(
        saw_travel,
        "stale belief should drive travel toward the depleted orchard"
    );
    assert!(
        reached_orchard,
        "agent should reach Orchard Farm before correcting the stale belief"
    );
    assert!(
        saw_discovery,
        "arrival should emit an inventory discrepancy discovery event"
    );
    assert!(
        corrected_belief,
        "direct observation should replace the stale orchard-stock belief"
    );
    assert!(
        !started_harvest_after_discovery,
        "agent should not continue into a harvest action after discovery invalidates the stale belief"
    );

    (hash_world(&h.world).unwrap(), hash_event_log(&h.event_log).unwrap())
}

fn run_skeptical_listener_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, build_recipes());

    let speaker = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Speaker",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        social_weighted_utility(900),
    );
    let listener = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Skeptic",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let orchard = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(10),
            max_quantity: Quantity(10),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
    );

    ensure_empty_belief_store(&mut h.world, &mut h.event_log, speaker);
    ensure_empty_belief_store(&mut h.world, &mut h.event_log, listener);
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        speaker,
        focused_accepting_tell_profile(),
    );
    set_agent_tell_profile(&mut h.world, &mut h.event_log, listener, rejecting_tell_profile());
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        speaker,
        blind_perception_profile(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        keen_perception_profile(),
    );
    let listener_belief = build_believed_entity_state(
        &h.world,
        listener,
        Tick(0),
        PerceptionSource::DirectObservation,
    )
    .expect("listener should be observable for tell targeting");
    seed_belief(&mut h.world, &mut h.event_log, speaker, listener, listener_belief);
    let orchard_belief = build_believed_entity_state(
        &h.world,
        orchard,
        Tick(1),
        PerceptionSource::DirectObservation,
    )
    .expect("orchard should be observable for belief seeding");
    seed_belief(&mut h.world, &mut h.event_log, speaker, orchard, orchard_belief);

    let mut saw_social_event = false;
    let mut listener_left_village = false;

    for _ in 0..80 {
        h.step_once();
        saw_social_event |= !h.event_log.events_by_tag(EventTag::Social).is_empty();
        listener_left_village |=
            h.world.effective_place(listener) != Some(VILLAGE_SQUARE) || h.world.is_in_transit(listener);
    }

    assert!(
        saw_social_event,
        "speaker should still attempt and execute tell actions"
    );
    assert!(
        agent_belief_about(&h.world, listener, orchard).is_none(),
        "skeptical listener should reject the transferred orchard belief"
    );
    assert!(
        !listener_left_village,
        "listener should not travel toward Orchard Farm after rejecting the told belief"
    );

    (hash_world(&h.world).unwrap(), hash_event_log(&h.event_log).unwrap())
}

#[allow(clippy::too_many_lines)]
fn run_bystander_witness_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, build_recipes());

    let speaker = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Speaker",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        social_weighted_utility(900),
    );
    let listener = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Listener",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let bystander = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Bystander",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let orchard = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(10),
            max_quantity: Quantity(10),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
    );

    for agent in [speaker, listener, bystander] {
        ensure_empty_belief_store(&mut h.world, &mut h.event_log, agent);
        set_agent_perception_profile(
            &mut h.world,
            &mut h.event_log,
            agent,
            keen_perception_profile(),
        );
    }
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        speaker,
        focused_accepting_tell_profile(),
    );
    set_agent_tell_profile(&mut h.world, &mut h.event_log, listener, accepting_tell_profile());
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        bystander,
        rejecting_tell_profile(),
    );

    let listener_belief = build_believed_entity_state(
        &h.world,
        listener,
        Tick(0),
        PerceptionSource::DirectObservation,
    )
    .expect("listener should be observable for tell targeting");
    seed_belief(&mut h.world, &mut h.event_log, speaker, listener, listener_belief);
    let orchard_belief = build_believed_entity_state(
        &h.world,
        orchard,
        Tick(1),
        PerceptionSource::DirectObservation,
    )
    .expect("orchard should be observable for belief seeding");
    seed_belief(&mut h.world, &mut h.event_log, speaker, orchard, orchard_belief);

    let scenario_completed = run_until(60, || {
        h.step_once();
        let bystander_store = h
            .world
            .get_component_agent_belief_store(bystander)
            .expect("bystander should keep a belief store");
        let witnessed_telling = bystander_store.social_observations.iter().any(|observation| {
            observation.kind == SocialObservationKind::WitnessedTelling
                && observation.subjects == (speaker, listener)
                && observation.place == VILLAGE_SQUARE
        });
        let listener_learned_orchard = agent_belief_about(&h.world, listener, orchard).is_some();
        witnessed_telling && listener_learned_orchard
    });

    assert!(
        scenario_completed,
        "speaker should tell the listener while the bystander witnesses the social act"
    );
    let bystander_store = h
        .world
        .get_component_agent_belief_store(bystander)
        .expect("bystander should keep a belief store");
    assert!(
        bystander_store.social_observations.iter().any(|observation| {
            observation.kind == SocialObservationKind::WitnessedTelling
                && observation.subjects == (speaker, listener)
                && observation.place == VILLAGE_SQUARE
        }),
        "bystander should record the witnessed telling event"
    );
    assert!(
        agent_belief_about(&h.world, bystander, orchard).is_none(),
        "bystander should not receive the orchard belief content merely by witnessing the tell"
    );
    assert_eq!(
        h.world.effective_place(bystander),
        Some(VILLAGE_SQUARE),
        "bystander should remain local instead of acting on an unreceived remote belief"
    );

    (hash_world(&h.world).unwrap(), hash_event_log(&h.event_log).unwrap())
}

fn run_entity_missing_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, build_recipes());

    let observer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Observer",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let missing_subject = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "MissingSubject",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    ensure_empty_belief_store(&mut h.world, &mut h.event_log, observer);
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        observer,
        keen_perception_profile(),
    );

    let mut stale_belief = build_believed_entity_state(
        &h.world,
        missing_subject,
        Tick(0),
        PerceptionSource::DirectObservation,
    )
    .expect("missing subject should be observable for belief seeding");
    stale_belief.last_known_place = Some(VILLAGE_SQUARE);
    seed_belief(
        &mut h.world,
        &mut h.event_log,
        observer,
        missing_subject,
        stale_belief,
    );

    let observed_missing = run_until(8, || {
        h.step_once();
        saw_entity_missing_discovery(&h.event_log, observer, missing_subject)
    });

    assert!(
        observed_missing,
        "passive local observation should emit an EntityMissing discovery for a violated place expectation"
    );
    let belief = agent_belief_about(&h.world, observer, missing_subject)
        .expect("observer should retain the prior belief snapshot");
    assert_eq!(
        belief.last_known_place,
        Some(VILLAGE_SQUARE),
        "entity-missing discovery should not silently teleport the subject to a new place"
    );

    (hash_world(&h.world).unwrap(), hash_event_log(&h.event_log).unwrap())
}

#[allow(clippy::too_many_lines)]
fn run_survival_needs_suppression_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, build_recipes());

    let speaker = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "HungrySpeaker",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        worldwake_core::MetabolismProfile::default(),
        social_weighted_utility(900),
    );
    let listener = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Listener",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let orchard = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(10),
            max_quantity: Quantity(10),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
    );

    ensure_empty_belief_store(&mut h.world, &mut h.event_log, listener);
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        speaker,
        focused_accepting_tell_profile(),
    );
    set_agent_tell_profile(&mut h.world, &mut h.event_log, listener, accepting_tell_profile());
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        speaker,
        blind_perception_profile(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        keen_perception_profile(),
    );

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        speaker,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(1),
    );

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        speaker,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    let orchard_belief = build_believed_entity_state(
        &h.world,
        orchard,
        Tick(1),
        PerceptionSource::DirectObservation,
    )
    .expect("orchard should be observable for belief seeding");
    seed_belief(&mut h.world, &mut h.event_log, speaker, orchard, orchard_belief);

    let initial_hunger = h.agent_hunger(speaker);
    let initial_bread = h.agent_commodity_qty(speaker, CommodityKind::Bread);
    let mut first_relief_tick = None;
    let mut first_social_tick = None;
    let mut social_before_relief = false;
    let mut listener_learned_before_relief = false;
    let mut hunger_decreased = false;
    let mut bread_consumed = false;

    for _ in 0..120 {
        h.step_once();

        let current_tick = h.scheduler.current_tick().0;
        let current_bread = h.agent_commodity_qty(speaker, CommodityKind::Bread);
        let current_hunger = h.agent_hunger(speaker);
        hunger_decreased |= current_hunger < initial_hunger;
        bread_consumed |= current_bread < initial_bread;
        if first_relief_tick.is_none() && (hunger_decreased || bread_consumed) {
            first_relief_tick = Some(current_tick);
        }
        if first_social_tick.is_none() && !h.event_log.events_by_tag(EventTag::Social).is_empty() {
            first_social_tick = Some(current_tick);
        }

        let listener_knows_orchard = agent_belief_about(&h.world, listener, orchard).is_some();
        if first_relief_tick.is_none() {
            social_before_relief |= first_social_tick.is_some();
            listener_learned_before_relief |= listener_knows_orchard;
        }

        if first_relief_tick.is_some() && hunger_decreased && bread_consumed {
            break;
        }
    }

    assert!(
        first_relief_tick.is_some(),
        "critically hungry speaker should consume local bread before pursuing social goals"
    );
    assert!(
        hunger_decreased,
        "consuming owned bread should reduce the speaker's hunger"
    );
    assert!(
        bread_consumed,
        "speaker should consume the local bread rather than defer survival relief"
    );
    assert!(
        !social_before_relief,
        "no social tell event should fire before the hunger-driven food relief occurs"
    );
    assert!(
        !listener_learned_before_relief,
        "listener should not receive the told belief before hunger is addressed"
    );
    assert!(
        first_social_tick.is_none_or(|social_tick| {
            first_relief_tick.is_some_and(|relief_tick| social_tick > relief_tick)
        }),
        "if a social event occurs in this scenario, it must occur strictly after the first survival-relief tick"
    );

    (hash_world(&h.world).unwrap(), hash_event_log(&h.event_log).unwrap())
}

#[test]
fn golden_agent_autonomously_tells_colocated_peer() {
    let first = run_autonomous_tell_scenario(Seed([91; 32]));
    let second = run_autonomous_tell_scenario(Seed([91; 32]));

    assert_eq!(
        first, second,
        "autonomous tell scenario should replay deterministically"
    );
}

#[test]
fn golden_rumor_chain_degrades_through_three_agents() {
    let first = run_rumor_chain_scenario(Seed([92; 32]));
    let second = run_rumor_chain_scenario(Seed([92; 32]));

    assert_eq!(
        first, second,
        "rumor relay scenario should replay deterministically"
    );
}

#[test]
fn golden_stale_belief_travel_reobserve_replan() {
    let first = run_stale_belief_replan_scenario(Seed([93; 32]));
    let second = run_stale_belief_replan_scenario(Seed([93; 32]));

    assert_eq!(
        first, second,
        "stale-belief discovery scenario should replay deterministically"
    );
}

#[test]
fn golden_skeptical_listener_rejects_told_belief() {
    let first = run_skeptical_listener_scenario(Seed([94; 32]));
    let second = run_skeptical_listener_scenario(Seed([94; 32]));

    assert_eq!(
        first, second,
        "skeptical-listener scenario should replay deterministically"
    );
}

#[test]
fn golden_bystander_sees_telling_but_gets_no_belief() {
    let first = run_bystander_witness_scenario(Seed([95; 32]));
    let second = run_bystander_witness_scenario(Seed([95; 32]));

    assert_eq!(
        first, second,
        "bystander locality scenario should replay deterministically"
    );
}

#[test]
fn golden_entity_missing_discovery_does_not_teleport_belief() {
    let first = run_entity_missing_scenario(Seed([96; 32]));
    let second = run_entity_missing_scenario(Seed([96; 32]));

    assert_eq!(
        first, second,
        "entity-missing discovery scenario should replay deterministically"
    );
}

#[test]
fn golden_survival_needs_suppress_social_goals() {
    let first = run_survival_needs_suppression_scenario(Seed([97; 32]));
    let second = run_survival_needs_suppression_scenario(Seed([97; 32]));

    assert_eq!(
        first, second,
        "survival-needs suppression scenario should replay deterministically"
    );
}
