//! Golden tests for social information transfer, relay degradation, and discovery correction.

mod golden_harness;

use golden_harness::*;
use worldwake_ai::GoalTraceStatus;
use worldwake_core::{
    belief_confidence, build_believed_entity_state, hash_event_log, hash_world,
    verify_authoritative_conservation, CommodityKind, EntityId, EventTag, EventView, EvidenceRef,
    GoalKind, HomeostaticNeeds, MismatchKind, PerceptionProfile, PerceptionSource, Quantity,
    RecipientKnowledgeStatus, ResourceSource, Seed, SocialObservationKind, TellMemoryKey,
    TellProfile, Tick, UtilityProfile, WorkstationTag,
};
use worldwake_sim::ActionTraceKind;

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
        institutional_memory_capacity: 20,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
    }
}

fn keen_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        memory_capacity: 32,
        memory_retention_ticks: 240,
        observation_fidelity: pm(1000),
        confidence_policy: worldwake_core::BeliefConfidencePolicy::default(),
        institutional_memory_capacity: 20,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
    }
}

fn accepting_tell_profile() -> TellProfile {
    TellProfile {
        max_tell_candidates: 3,
        max_relay_chain_len: 3,
        acceptance_fidelity: pm(1000),
        ..TellProfile::default()
    }
}

fn rejecting_tell_profile() -> TellProfile {
    TellProfile {
        max_tell_candidates: 3,
        max_relay_chain_len: 3,
        acceptance_fidelity: pm(0),
        ..TellProfile::default()
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
    log.events_by_tag(EventTag::Discovery)
        .iter()
        .any(|event_id| {
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

struct SocialRetellFixture {
    h: GoldenHarness,
    speaker: EntityId,
    listener: EntityId,
    subject: EntityId,
}

fn listener_suppressed_social_utility() -> UtilityProfile {
    UtilityProfile {
        social_weight: pm(0),
        ..UtilityProfile::default()
    }
}

fn retell_speaker_profile(retention_ticks: u64) -> TellProfile {
    TellProfile {
        max_tell_candidates: 1,
        acceptance_fidelity: pm(1000),
        conversation_memory_retention_ticks: retention_ticks,
        ..TellProfile::default()
    }
}

fn build_social_retell_fixture(
    seed: Seed,
    speaker_tell_profile: TellProfile,
) -> SocialRetellFixture {
    let mut h = GoldenHarness::with_recipes(seed, build_recipes());
    h.enable_action_tracing();
    h.driver.enable_tracing();

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
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        listener_suppressed_social_utility(),
    );
    let subject = place_workstation_with_source(
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
        ProductionOutputOwner::Actor,
    );

    for agent in [speaker, listener] {
        ensure_empty_belief_store(&mut h.world, &mut h.event_log, agent);
    }
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        speaker,
        speaker_tell_profile,
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        accepting_tell_profile(),
    );
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

    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        speaker,
        listener,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        speaker,
        subject,
        Tick(1),
        PerceptionSource::DirectObservation,
    );

    SocialRetellFixture {
        h,
        speaker,
        listener,
        subject,
    }
}

fn share_goal(listener: EntityId, subject: EntityId) -> GoalKind {
    GoalKind::ShareBelief { listener, subject }
}

fn told_memory(
    world: &worldwake_core::World,
    speaker: EntityId,
    listener: EntityId,
    subject: EntityId,
) -> worldwake_core::ToldBeliefMemory {
    world
        .get_component_agent_belief_store(speaker)
        .and_then(|store| {
            store.told_beliefs.get(&TellMemoryKey {
                counterparty: listener,
                subject,
            })
        })
        .cloned()
        .expect("speaker should have told-memory for the listener and subject")
}

fn latest_goal_status(h: &GoldenHarness, speaker: EntityId, goal: &GoalKind) -> GoalTraceStatus {
    h.driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .traces_for(speaker)
        .into_iter()
        .last()
        .expect("speaker should have at least one decision trace")
        .goal_status(goal)
}

fn wait_for_initial_tell(fixture: &mut SocialRetellFixture) -> Tick {
    let learned = run_until(24, || {
        fixture.h.step_once();
        agent_belief_about(&fixture.h.world, fixture.listener, fixture.subject).is_some()
    });
    assert!(
        learned,
        "listener should learn the speaker's initial subject belief"
    );

    let memory = told_memory(
        &fixture.h.world,
        fixture.speaker,
        fixture.listener,
        fixture.subject,
    );
    memory.told_tick
}

fn seed_subject_belief_change(fixture: &mut SocialRetellFixture, available_quantity: Quantity) {
    let mut source = fixture
        .h
        .world
        .get_component_resource_source(fixture.subject)
        .cloned()
        .expect("subject workstation should have a resource source");
    source.available_quantity = available_quantity;

    let observed_tick = fixture.h.scheduler.current_tick();
    let mut txn = new_txn(&mut fixture.h.world, observed_tick.0);
    txn.set_component_resource_source(fixture.subject, source)
        .expect("subject resource source should remain writable");
    commit_txn(txn, &mut fixture.h.event_log);

    let changed_belief = build_believed_entity_state(
        &fixture.h.world,
        fixture.subject,
        observed_tick,
        PerceptionSource::DirectObservation,
    )
    .expect("changed subject should remain observable for belief seeding");
    seed_belief(
        &mut fixture.h.world,
        &mut fixture.h.event_log,
        fixture.speaker,
        fixture.subject,
        changed_belief,
    );
}

#[allow(clippy::too_many_lines)]
fn run_autonomous_tell_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, build_recipes());
    h.enable_action_tracing();

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
        ProductionOutputOwner::Actor,
    );

    ensure_empty_belief_store(&mut h.world, &mut h.event_log, speaker);
    ensure_empty_belief_store(&mut h.world, &mut h.event_log, listener);
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        speaker,
        focused_accepting_tell_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        accepting_tell_profile(),
    );
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

    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        speaker,
        listener,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        speaker,
        orchard,
        Tick(1),
        PerceptionSource::DirectObservation,
    );

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

    assert!(
        saw_social_event,
        "speaker should execute a social tell event"
    );
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

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[allow(clippy::too_many_lines)]
fn run_rumor_chain_scenario(seed: Seed) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
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

    let bob_belief =
        build_believed_entity_state(&h.world, bob, Tick(0), PerceptionSource::DirectObservation)
            .expect("Bob should be observable for relay targeting");
    seed_belief(&mut h.world, &mut h.event_log, alice, bob, bob_belief);
    let subject_belief = build_believed_entity_state(
        &h.world,
        subject,
        Tick(1),
        PerceptionSource::DirectObservation,
    )
    .expect("Subject should be observable for relay seeding");
    seed_belief(
        &mut h.world,
        &mut h.event_log,
        alice,
        subject,
        subject_belief,
    );

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
    assert_eq!(
        carol_belief.source,
        PerceptionSource::Rumor { chain_len: 2 }
    );

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

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[allow(clippy::too_many_lines)]
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
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        keen_perception_profile(),
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
        ProductionOutputOwner::Actor,
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

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[allow(clippy::too_many_lines)]
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
        ProductionOutputOwner::Actor,
    );

    ensure_empty_belief_store(&mut h.world, &mut h.event_log, speaker);
    ensure_empty_belief_store(&mut h.world, &mut h.event_log, listener);
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        speaker,
        focused_accepting_tell_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        rejecting_tell_profile(),
    );
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
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        speaker,
        listener,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        speaker,
        orchard,
        Tick(1),
        PerceptionSource::DirectObservation,
    );

    let mut saw_social_event = false;
    let mut listener_left_village = false;

    for _ in 0..80 {
        h.step_once();
        saw_social_event |= !h.event_log.events_by_tag(EventTag::Social).is_empty();
        listener_left_village |= h.world.effective_place(listener) != Some(VILLAGE_SQUARE)
            || h.world.is_in_transit(listener);
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

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
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
        ProductionOutputOwner::Actor,
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
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        accepting_tell_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        bystander,
        rejecting_tell_profile(),
    );

    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        speaker,
        listener,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        speaker,
        orchard,
        Tick(1),
        PerceptionSource::DirectObservation,
    );

    let scenario_completed = run_until(60, || {
        h.step_once();
        let bystander_store = h
            .world
            .get_component_agent_belief_store(bystander)
            .expect("bystander should keep a belief store");
        let witnessed_telling = bystander_store
            .social_observations
            .iter()
            .any(|observation| {
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
        bystander_store
            .social_observations
            .iter()
            .any(|observation| {
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

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
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

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[allow(clippy::too_many_lines)]
fn run_survival_needs_suppression_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, build_recipes());
    h.enable_action_tracing();

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
        ProductionOutputOwner::Actor,
    );

    ensure_empty_belief_store(&mut h.world, &mut h.event_log, listener);
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        speaker,
        focused_accepting_tell_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        accepting_tell_profile(),
    );
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
    seed_belief(
        &mut h.world,
        &mut h.event_log,
        speaker,
        orchard,
        orchard_belief,
    );

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
        if first_social_tick.is_none() {
            let action_sink = h
                .action_trace_sink()
                .expect("action tracing should be enabled for social suppression checks");
            first_social_tick = action_sink.events_for(speaker).iter().find_map(|event| {
                (event.action_name == "tell"
                    && matches!(event.kind, ActionTraceKind::Committed { .. }))
                .then_some(event.tick.0)
            });
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

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
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

#[allow(clippy::too_many_lines)]
fn run_unchanged_tell_suppression_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut fixture = build_social_retell_fixture(seed, retell_speaker_profile(48));
    let share_goal = share_goal(fixture.listener, fixture.subject);
    let initial_told_tick = wait_for_initial_tell(&mut fixture);
    let initial_memory = told_memory(
        &fixture.h.world,
        fixture.speaker,
        fixture.listener,
        fixture.subject,
    );

    let mut saw_resend_omission = false;
    for _ in 0..6 {
        fixture.h.step_once();
        saw_resend_omission |= latest_goal_status(&fixture.h, fixture.speaker, &share_goal)
            == GoalTraceStatus::OmittedSocial(
                RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief,
            );
    }

    let final_memory = told_memory(
        &fixture.h.world,
        fixture.speaker,
        fixture.listener,
        fixture.subject,
    );
    assert!(
        saw_resend_omission,
        "decision traces should expose unchanged resend suppression"
    );
    assert_eq!(
        final_memory.told_tick, initial_told_tick,
        "unchanged resend suppression should leave the original told-memory tick intact"
    );
    assert_eq!(
        final_memory.shared_state, initial_memory.shared_state,
        "unchanged resend suppression should preserve the original shared snapshot"
    );

    (
        hash_world(&fixture.h.world).unwrap(),
        hash_event_log(&fixture.h.event_log).unwrap(),
    )
}

#[test]
fn golden_agent_does_not_repeat_same_unchanged_tell_to_same_listener() {
    let first = run_unchanged_tell_suppression_scenario(Seed([100; 32]));
    let second = run_unchanged_tell_suppression_scenario(Seed([100; 32]));

    assert_eq!(
        first, second,
        "unchanged tell suppression scenario should replay deterministically"
    );
}

#[allow(clippy::too_many_lines)]
fn run_retell_after_subject_belief_change_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut fixture = build_social_retell_fixture(seed, retell_speaker_profile(48));
    let share_goal = share_goal(fixture.listener, fixture.subject);
    let initial_told_tick = wait_for_initial_tell(&mut fixture);

    seed_subject_belief_change(&mut fixture, Quantity(6));

    let mut saw_reenabled_share_goal = false;
    let retold = run_until(16, || {
        fixture.h.step_once();
        let status = latest_goal_status(&fixture.h, fixture.speaker, &share_goal);
        saw_reenabled_share_goal |= matches!(
            status,
            GoalTraceStatus::GeneratedOnly | GoalTraceStatus::Ranked { .. }
        );
        told_memory(
            &fixture.h.world,
            fixture.speaker,
            fixture.listener,
            fixture.subject,
        )
        .told_tick
            > initial_told_tick
    });

    assert!(
        retold,
        "belief-content change should trigger a lawful re-tell"
    );
    assert!(
        saw_reenabled_share_goal,
        "decision traces should show ShareBelief re-enabled after a material belief change"
    );

    let final_memory = told_memory(
        &fixture.h.world,
        fixture.speaker,
        fixture.listener,
        fixture.subject,
    );
    assert!(
        final_memory.told_tick > initial_told_tick,
        "re-tell after belief change should refresh the told-memory tick"
    );
    assert_eq!(
        final_memory
            .shared_state
            .resource_source
            .as_ref()
            .expect("shared subject snapshot should retain resource source")
            .available_quantity,
        Quantity(6),
        "re-tell should store the materially changed shared content"
    );
    assert_eq!(
        agent_belief_about(&fixture.h.world, fixture.listener, fixture.subject)
            .and_then(|belief| belief.resource_source.as_ref())
            .map(|source| source.available_quantity),
        Some(Quantity(6)),
        "listener should receive the updated belief content through the second tell"
    );

    (
        hash_world(&fixture.h.world).unwrap(),
        hash_event_log(&fixture.h.event_log).unwrap(),
    )
}

#[test]
fn golden_agent_retells_after_subject_belief_changes() {
    let first = run_retell_after_subject_belief_change_scenario(Seed([101; 32]));
    let second = run_retell_after_subject_belief_change_scenario(Seed([101; 32]));

    assert_eq!(
        first, second,
        "belief-change re-tell scenario should replay deterministically"
    );
}

#[allow(clippy::too_many_lines)]
fn run_retell_after_conversation_memory_expiry_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut fixture = build_social_retell_fixture(seed, retell_speaker_profile(2));
    let share_goal = share_goal(fixture.listener, fixture.subject);
    let initial_told_tick = wait_for_initial_tell(&mut fixture);

    let mut saw_resend_omission_before_expiry = false;
    let mut saw_reenabled_after_expiry = false;
    let retold = run_until(16, || {
        fixture.h.step_once();
        let status = latest_goal_status(&fixture.h, fixture.speaker, &share_goal);
        match status {
            GoalTraceStatus::OmittedSocial(
                RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief,
            ) => saw_resend_omission_before_expiry = true,
            GoalTraceStatus::GeneratedOnly | GoalTraceStatus::Ranked { .. } => {
                saw_reenabled_after_expiry = true;
            }
            _ => {}
        }

        told_memory(
            &fixture.h.world,
            fixture.speaker,
            fixture.listener,
            fixture.subject,
        )
        .told_tick
            > initial_told_tick
    });

    assert!(
        retold,
        "expired conversation memory should permit a lawful re-tell"
    );
    assert!(
        saw_resend_omission_before_expiry,
        "before expiry, decision traces should still show unchanged resend suppression"
    );
    assert!(
        saw_reenabled_after_expiry,
        "after expiry, decision traces should show ShareBelief re-enabled"
    );

    let final_memory = told_memory(
        &fixture.h.world,
        fixture.speaker,
        fixture.listener,
        fixture.subject,
    );
    assert!(
        final_memory.told_tick > initial_told_tick,
        "re-tell after expiry should refresh the speaker's told-memory tick"
    );

    (
        hash_world(&fixture.h.world).unwrap(),
        hash_event_log(&fixture.h.event_log).unwrap(),
    )
}

#[test]
fn golden_agent_retells_after_conversation_memory_expiry() {
    let first = run_retell_after_conversation_memory_expiry_scenario(Seed([102; 32]));
    let second = run_retell_after_conversation_memory_expiry_scenario(Seed([102; 32]));

    assert_eq!(
        first, second,
        "conversation-memory expiry re-tell scenario should replay deterministically"
    );
}

fn run_trace_reenabled_social_candidate_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut changed = build_social_retell_fixture(seed, retell_speaker_profile(48));
    let changed_goal = share_goal(changed.listener, changed.subject);
    wait_for_initial_tell(&mut changed);
    seed_subject_belief_change(&mut changed, Quantity(4));
    changed.h.step_once();

    let changed_status = latest_goal_status(&changed.h, changed.speaker, &changed_goal);
    assert!(
        matches!(
            changed_status,
            GoalTraceStatus::GeneratedOnly | GoalTraceStatus::Ranked { .. }
        ),
        "belief change should re-enable ShareBelief in the decision trace"
    );

    let mut expired = build_social_retell_fixture(seed, retell_speaker_profile(2));
    let expired_goal = share_goal(expired.listener, expired.subject);
    wait_for_initial_tell(&mut expired);
    expired.h.step_once();
    let pre_expiry_status = latest_goal_status(&expired.h, expired.speaker, &expired_goal);
    assert_eq!(
        pre_expiry_status,
        GoalTraceStatus::OmittedSocial(
            RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief,
        ),
        "before expiry, unchanged resend suppression should still appear in the decision trace"
    );

    expired.h.step_once();
    expired.h.step_once();
    let expired_status = latest_goal_status(&expired.h, expired.speaker, &expired_goal);
    assert!(
        matches!(
            expired_status,
            GoalTraceStatus::GeneratedOnly | GoalTraceStatus::Ranked { .. }
        ),
        "expired conversation memory should re-enable ShareBelief in the decision trace"
    );

    (
        hash_event_log(&changed.h.event_log).unwrap(),
        hash_event_log(&expired.h.event_log).unwrap(),
    )
}

#[test]
fn golden_decision_trace_explains_social_candidate_reenabled_after_belief_change_or_expiry() {
    let first = run_trace_reenabled_social_candidate_scenario(Seed([103; 32]));
    let second = run_trace_reenabled_social_candidate_scenario(Seed([103; 32]));

    assert_eq!(
        first, second,
        "trace-level social reenablement scenario should replay deterministically"
    );
}

// ===== T11: Chain length filtering stops gossip =====

#[allow(clippy::too_many_lines)]
fn run_chain_length_filtering_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, build_recipes());

    // 4 agents co-located at Village Square + subject at Orchard Farm.
    let alice = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Alice",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        social_weighted_utility(900),
    );
    let bob = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Bob",
        ORCHARD_FARM,
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
        social_weighted_utility(900),
    );
    let dave = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Dave",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        social_weighted_utility(900),
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

    // All agents: empty beliefs, blind perception (no passive observation).
    for agent in [alice, bob, carol, dave] {
        ensure_empty_belief_store(&mut h.world, &mut h.event_log, agent);
        set_agent_perception_profile(
            &mut h.world,
            &mut h.event_log,
            agent,
            blind_perception_profile(),
        );
    }

    // Alice, Bob: can relay chains up to 3.
    for agent in [alice, bob] {
        set_agent_tell_profile(
            &mut h.world,
            &mut h.event_log,
            agent,
            TellProfile {
                max_tell_candidates: 1,
                max_relay_chain_len: 3,
                acceptance_fidelity: pm(1000),
                ..TellProfile::default()
            },
        );
    }
    // Carol: max_relay_chain_len=1 — speaker-side filter blocks relay of chain_len=2 rumor.
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        carol,
        TellProfile {
            max_tell_candidates: 1,
            max_relay_chain_len: 1,
            acceptance_fidelity: pm(1000),
            ..TellProfile::default()
        },
    );
    // Dave: willing to relay up to 3, but never receives because Carol cannot relay.
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        dave,
        TellProfile {
            max_tell_candidates: 1,
            max_relay_chain_len: 3,
            acceptance_fidelity: pm(1000),
            ..TellProfile::default()
        },
    );

    // Seed beliefs: Alice knows subject (DirectObservation) and Bob (tell target).
    let subject_belief = build_believed_entity_state(
        &h.world,
        subject,
        Tick(1),
        PerceptionSource::DirectObservation,
    )
    .expect("subject should be observable");
    seed_belief(
        &mut h.world,
        &mut h.event_log,
        alice,
        subject,
        subject_belief,
    );

    let bob_belief =
        build_believed_entity_state(&h.world, bob, Tick(0), PerceptionSource::DirectObservation)
            .expect("bob should be observable");
    seed_belief(&mut h.world, &mut h.event_log, alice, bob, bob_belief);

    // Bob knows Carol (relay target).
    let carol_belief = build_believed_entity_state(
        &h.world,
        carol,
        Tick(0),
        PerceptionSource::DirectObservation,
    )
    .expect("carol should be observable");
    seed_belief(&mut h.world, &mut h.event_log, bob, carol, carol_belief);

    // Carol knows Dave (would-be relay target, but blocked by chain_len filter).
    let dave_belief =
        build_believed_entity_state(&h.world, dave, Tick(0), PerceptionSource::DirectObservation)
            .expect("dave should be observable");
    seed_belief(&mut h.world, &mut h.event_log, carol, dave, dave_belief);

    // Step 1: wait for Alice → Bob propagation.
    let bob_received = run_until(40, || {
        h.step_once();
        agent_belief_about(&h.world, bob, subject).is_some()
    });
    assert!(
        bob_received,
        "Bob should receive Alice's told belief about subject"
    );

    {
        let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
        txn.set_ground_location(bob, VILLAGE_SQUARE)
            .expect("golden chain-length scenario should be able to move Bob to Carol");
        txn.set_ground_location(dave, ORCHARD_FARM)
            .expect("golden chain-length scenario should be able to move Dave away from Bob");
        commit_txn(txn, &mut h.event_log);
    }

    // Step 2: wait for Bob → Carol propagation.
    let carol_received = run_until(40, || {
        h.step_once();
        agent_belief_about(&h.world, carol, subject).is_some()
    });
    assert!(
        carol_received,
        "Carol should receive Bob's relayed belief about subject"
    );

    {
        let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
        txn.set_ground_location(bob, ORCHARD_FARM)
            .expect("golden chain-length scenario should be able to move Bob away from Dave");
        txn.set_ground_location(dave, VILLAGE_SQUARE)
            .expect("golden chain-length scenario should be able to move Dave next to Carol");
        commit_txn(txn, &mut h.event_log);
    }

    // Step 3: give Dave enough time to potentially receive (he should not).
    for _ in 0..40 {
        h.step_once();
    }

    // Verify chain degradation.
    let bob_belief = agent_belief_about(&h.world, bob, subject).unwrap();
    assert!(
        matches!(
            bob_belief.source,
            PerceptionSource::Report { chain_len: 1, .. }
        ),
        "Bob should hold first-order hearsay about the subject"
    );

    let carol_belief = agent_belief_about(&h.world, carol, subject).unwrap();
    assert_eq!(
        carol_belief.source,
        PerceptionSource::Rumor { chain_len: 2 },
        "Carol should have Rumor with chain_len=2"
    );

    assert!(
        agent_belief_about(&h.world, dave, subject).is_none(),
        "Dave should have NO belief about subject — Carol's max_relay_chain_len=1 blocks relay of chain_len=2 rumor"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_chain_length_filtering_stops_gossip() {
    let first = run_chain_length_filtering_scenario(Seed([98; 32]));
    let second = run_chain_length_filtering_scenario(Seed([98; 32]));

    assert_eq!(
        first, second,
        "chain-length filtering scenario should replay deterministically"
    );
}

// ===== T12: Agent diversity in social behavior =====

#[allow(clippy::too_many_lines)]
fn run_agent_diversity_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, build_recipes());

    // Three speakers with different social weights.
    let gossip = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Gossip",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        social_weighted_utility(900),
    );
    let normal = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Normal",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        social_weighted_utility(200),
    );
    let loner = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Loner",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        social_weighted_utility(0),
    );

    // Common listener (social_weight=0 so it won't relay).
    let listener = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Listener",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        social_weighted_utility(0),
    );

    // Three unique subjects at Orchard Farm — one per speaker.
    let subject_g = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "SubjectG",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let subject_n = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "SubjectN",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let subject_l = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "SubjectL",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    // Configure speakers: blind perception, focused tell profile.
    for agent in [gossip, normal, loner] {
        ensure_empty_belief_store(&mut h.world, &mut h.event_log, agent);
        set_agent_perception_profile(
            &mut h.world,
            &mut h.event_log,
            agent,
            blind_perception_profile(),
        );
        set_agent_tell_profile(
            &mut h.world,
            &mut h.event_log,
            agent,
            focused_accepting_tell_profile(),
        );
    }

    // Listener: keen perception, accepting tell profile.
    ensure_empty_belief_store(&mut h.world, &mut h.event_log, listener);
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        keen_perception_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        accepting_tell_profile(),
    );

    // Seed beliefs: each speaker knows the listener + their unique subject.
    for speaker in [gossip, normal, loner] {
        seed_belief_from_world(
            &mut h.world,
            &mut h.event_log,
            speaker,
            listener,
            Tick(0),
            PerceptionSource::DirectObservation,
        );
    }

    for (speaker, subject, label) in [
        (gossip, subject_g, "subject_g"),
        (normal, subject_n, "subject_n"),
        (loner, subject_l, "subject_l"),
    ] {
        let belief = seed_belief_from_world(
            &mut h.world,
            &mut h.event_log,
            speaker,
            subject,
            Tick(1),
            PerceptionSource::DirectObservation,
        );
        assert_eq!(
            belief.observed_tick,
            Tick(1),
            "{label} belief should preserve the requested observed tick"
        );
    }

    // Run simulation for extended ticks.
    let mut gossip_told = false;
    let mut normal_told = false;

    for _ in 0..60 {
        h.step_once();
        gossip_told |= agent_belief_about(&h.world, listener, subject_g).is_some();
        normal_told |= agent_belief_about(&h.world, listener, subject_n).is_some();

        if gossip_told && normal_told {
            break;
        }
    }

    assert!(
        gossip_told,
        "Gossip (social_weight=900) should tell listener about subject_g"
    );
    assert!(
        normal_told,
        "Normal (social_weight=200) should tell listener about subject_n"
    );
    assert!(
        agent_belief_about(&h.world, listener, subject_l).is_none(),
        "Loner (social_weight=0) should never tell listener — zero-motive filter excludes ShareBelief from ranked list"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_agent_diversity_in_social_behavior() {
    let first = run_agent_diversity_scenario(Seed([99; 32]));
    let second = run_agent_diversity_scenario(Seed([99; 32]));

    assert_eq!(
        first, second,
        "agent-diversity scenario should replay deterministically"
    );
}

// ===== T13: Rumor leads to wasted trip then discovery =====

#[allow(clippy::too_many_lines)]
fn run_rumor_wasted_trip_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, build_recipes());

    // Informant: the original observer (used as the Report `from` field).
    let informant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Informant",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    // Speaker: has second-hand knowledge (Report from informant) about orchard.
    let speaker = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Speaker",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        social_weighted_utility(900),
    );

    // Agent: hungry, will act on received rumor.
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Forager",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    // Orchard starts with apples (for belief building).
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
        ProductionOutputOwner::Actor,
    );

    // Build speaker's belief about orchard as Report{from: informant, chain_len: 1}.
    // This captures the pre-depletion state (10 apples available).
    let orchard_report = build_believed_entity_state(
        &h.world,
        orchard,
        Tick(1),
        PerceptionSource::Report {
            from: informant,
            chain_len: 1,
        },
    )
    .expect("orchard should be observable for belief building");

    // Deplete the orchard — the real world now has 0 apples.
    let mut txn = new_txn(&mut h.world, 2);
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

    // Configure speaker and agent.
    ensure_empty_belief_store(&mut h.world, &mut h.event_log, speaker);
    ensure_empty_belief_store(&mut h.world, &mut h.event_log, agent);
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        speaker,
        focused_accepting_tell_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        accepting_tell_profile(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        speaker,
        blind_perception_profile(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        keen_perception_profile(),
    );

    // Seed speaker's beliefs: knows agent (tell target) and orchard (Report source).
    let agent_belief = build_believed_entity_state(
        &h.world,
        agent,
        Tick(0),
        PerceptionSource::DirectObservation,
    )
    .expect("agent should be observable for tell targeting");
    seed_belief(&mut h.world, &mut h.event_log, speaker, agent, agent_belief);
    seed_belief(
        &mut h.world,
        &mut h.event_log,
        speaker,
        orchard,
        orchard_report,
    );

    // Track the full information lifecycle.
    let mut received_rumor = false;
    let mut left_village = false;
    let mut reached_orchard = false;
    let mut saw_discovery = false;
    let mut corrected_belief = false;

    for _ in 0..120 {
        h.step_once();
        verify_authoritative_conservation(&h.world, CommodityKind::Apple, 0).unwrap();

        if let Some(belief) = agent_belief_about(&h.world, agent, orchard) {
            received_rumor |= matches!(belief.source, PerceptionSource::Rumor { chain_len: 2 });
        }

        let place = h.world.effective_place(agent);
        left_village |= place != Some(VILLAGE_SQUARE) || h.world.is_in_transit(agent);
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

        if received_rumor && left_village && reached_orchard && saw_discovery && corrected_belief {
            break;
        }
    }

    assert!(
        received_rumor,
        "agent should receive a Rumor(chain_len=2) about the orchard from the speaker's told Report"
    );
    assert!(left_village, "rumor should drive travel toward the orchard");
    assert!(
        reached_orchard,
        "agent should reach Orchard Farm before discovering depletion"
    );
    assert!(
        saw_discovery,
        "arrival should emit a resource-source discrepancy discovery event"
    );
    assert!(
        corrected_belief,
        "direct observation should replace the rumor-sourced belief with the actual depleted state"
    );

    let final_belief = agent_belief_about(&h.world, agent, orchard)
        .expect("agent should retain corrected belief about orchard");
    assert_eq!(
        final_belief.source,
        PerceptionSource::DirectObservation,
        "belief source should be upgraded from Rumor to DirectObservation after discovery"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_rumor_leads_to_wasted_trip_then_discovery() {
    let first = run_rumor_wasted_trip_scenario(Seed([100; 32]));
    let second = run_rumor_wasted_trip_scenario(Seed([100; 32]));

    assert_eq!(
        first, second,
        "rumor-wasted-trip-discovery scenario should replay deterministically"
    );
}
