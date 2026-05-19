//! Golden tests for S140 artifact lifecycle axes.

use std::collections::BTreeMap;

use crate::golden_harness::{commit_txn, new_txn, seed_belief_from_world};
use worldwake_ai::generate_candidates;
use worldwake_core::social_artifact::SuspensionReason;
use worldwake_core::{
    ArtifactActionability, ArtifactAxisValue, ArtifactCredibility, ArtifactHeader, ArtifactKind,
    ArtifactLegalEffect, ArtifactTransitionPayload, AxisName, BlockerMemory, BountyTarget,
    BountyTerms, CauseRef, CloseCause, CommodityKind, ComponentKind, ComponentValue,
    ContentionStatus, ControlSource, DeadAt, EntityId, EntityKind, EventLog, EventTag, EventView,
    GoalKind, InstitutionalClaim, PerceptionSource, ProofRequirement, PrototypePlace, Quantity,
    RecordData, RecordKind, RewardSource, Seed, Tick, World, build_prototype_world,
    prototype_place_entity,
};
use worldwake_sim::{
    ActionDefRegistry, ActionError, ActionExecutionAuthority, ActionExecutionContext,
    ActionHandlerRegistry, ActionInstance, ActionInstanceId, Affordance, DeterministicRng,
    PerAgentBeliefView, RecipeRegistry, SystemExecutionContext, SystemId, TickOutcome,
    start_action, tick_action,
};
use worldwake_systems::{artifact_lifecycle_system, build_full_action_registries};

const SQUARE: EntityId = prototype_place_entity(PrototypePlace::VillageSquare);

fn actor(world: &mut World, name: &str, place: EntityId) -> EntityId {
    let mut txn = new_txn(world, 1);
    let actor = txn.create_agent(name, ControlSource::Human).unwrap();
    txn.set_ground_location(actor, place).unwrap();
    commit_txn(txn, &mut EventLog::new());
    actor
}

fn grant_funds(
    world: &mut World,
    owner: EntityId,
    place: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
) {
    let mut txn = new_txn(world, 2);
    let lot = txn.create_item_lot(commodity, quantity).unwrap();
    txn.set_ground_location(lot, place).unwrap();
    txn.set_owner(lot, owner).unwrap();
    commit_txn(txn, &mut EventLog::new());
}

fn kill(world: &mut World, target: EntityId, tick: Tick) {
    let mut txn = new_txn(world, tick.0);
    txn.set_component_dead_at(
        target,
        DeadAt {
            tick,
            cause: worldwake_core::DeathCause::CombatWounds,
        },
    )
    .unwrap();
    commit_txn(txn, &mut EventLog::new());
}

fn bounty(
    world: &mut World,
    issuer: EntityId,
    target: EntityId,
    expires_at: Option<Tick>,
) -> EntityId {
    let mut txn = new_txn(world, 3);
    let artifact = txn.create_entity(EntityKind::SocialArtifact);
    txn.set_component_artifact_header(
        artifact,
        ArtifactHeader::posted_active(
            ArtifactKind::Bounty,
            issuer,
            None,
            Tick(3),
            expires_at,
            None,
            SQUARE,
        ),
    )
    .unwrap();
    txn.set_component_bounty_terms(
        artifact,
        BountyTerms {
            target: BountyTarget::EliminateEntity { target },
            proof_requirement: ProofRequirement::PhysicalEvidence,
            reward_commodity: CommodityKind::Coin,
            reward_quantity: Quantity(4),
            reward_source: RewardSource::PersonalFunds { issuer },
            claim_place: SQUARE,
        },
    )
    .unwrap();
    txn.set_ground_location(artifact, SQUARE).unwrap();
    commit_txn(txn, &mut EventLog::new());
    artifact
}

fn office(world: &mut World, name: &str) -> EntityId {
    let mut txn = new_txn(world, 1);
    let office = txn.create_office(name).unwrap();
    commit_txn(txn, &mut EventLog::new());
    office
}

fn record(world: &mut World, issuer: EntityId, place: EntityId, kind: RecordKind) -> EntityId {
    let mut txn = new_txn(world, 2);
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
    commit_txn(txn, &mut EventLog::new());
    record
}

fn office_bounty(
    world: &mut World,
    issuer: EntityId,
    office: EntityId,
    target: EntityId,
    expires_at: Option<Tick>,
) -> EntityId {
    let mut txn = new_txn(world, 3);
    let artifact = txn.create_entity(EntityKind::SocialArtifact);
    txn.set_component_artifact_header(
        artifact,
        ArtifactHeader::posted_active(
            ArtifactKind::Bounty,
            issuer,
            Some(office),
            Tick(3),
            expires_at,
            Some(office),
            SQUARE,
        ),
    )
    .unwrap();
    txn.set_component_bounty_terms(
        artifact,
        BountyTerms {
            target: BountyTarget::EliminateEntity { target },
            proof_requirement: ProofRequirement::PhysicalEvidence,
            reward_commodity: CommodityKind::Coin,
            reward_quantity: Quantity(4),
            reward_source: RewardSource::PersonalFunds { issuer },
            claim_place: SQUARE,
        },
    )
    .unwrap();
    txn.set_ground_location(artifact, SQUARE).unwrap();
    commit_txn(txn, &mut EventLog::new());
    artifact
}

fn force_control_source_event(
    world: &mut World,
    log: &mut EventLog,
    record: EntityId,
    office: EntityId,
    contested: bool,
    tick: Tick,
) -> worldwake_core::EventId {
    let mut txn = new_txn(world, tick.0);
    txn.append_record_entry(
        record,
        InstitutionalClaim::ForceControl {
            office,
            controller: None,
            contested,
            effective_tick: tick,
        },
    )
    .unwrap();
    txn.add_tag(EventTag::Social).add_tag(EventTag::Control);
    txn.commit(log)
}

fn artifact_refutation_source_event(
    world: &mut World,
    log: &mut EventLog,
    record: EntityId,
    artifact: EntityId,
    evidence: EntityId,
    tick: Tick,
) -> worldwake_core::EventId {
    let mut txn = new_txn(world, tick.0);
    txn.append_record_entry(
        record,
        InstitutionalClaim::ArtifactCredibilityRefutation {
            artifact,
            evidence,
            effective_tick: tick,
        },
    )
    .unwrap();
    txn.add_tag(EventTag::Social);
    txn.commit(log)
}

fn action_id(defs: &ActionDefRegistry, name: &str) -> worldwake_core::ActionDefId {
    defs.iter()
        .find(|def| def.name == name)
        .map_or_else(|| panic!("{name} action must be registered"), |def| def.id)
}

fn registries() -> (ActionDefRegistry, ActionHandlerRegistry) {
    let registries = build_full_action_registries(&RecipeRegistry::new()).unwrap();
    (registries.defs, registries.handlers)
}

#[allow(clippy::too_many_arguments)]
fn start_bound_action(
    world: &mut World,
    log: &mut EventLog,
    rng: &mut DeterministicRng,
    defs: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    next_id: &mut ActionInstanceId,
    actor: EntityId,
    target: EntityId,
    name: &str,
) -> Result<ActionInstanceId, ActionError> {
    start_action(
        &Affordance {
            def_id: action_id(defs, name),
            actor,
            bound_targets: vec![target],
            payload_override: None,
            explanation: None,
            contention_status: ContentionStatus::Available,
        },
        defs,
        handlers,
        ActionExecutionAuthority {
            active_actions: &mut BTreeMap::new(),
            world,
            event_log: log,
            rng,
        },
        next_id,
        ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
    )
}

#[allow(clippy::too_many_arguments)]
fn tick_to_commit(
    world: &mut World,
    log: &mut EventLog,
    rng: &mut DeterministicRng,
    defs: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    active: &mut BTreeMap<ActionInstanceId, ActionInstance>,
    action_id: ActionInstanceId,
    tick: Tick,
) -> Result<TickOutcome, ActionError> {
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
        ActionExecutionContext::without_recipes(CauseRef::Bootstrap, tick),
    )
}

fn run_lifecycle(world: &mut World, log: &mut EventLog, tick: Tick) {
    let mut rng = DeterministicRng::new(Seed([140; 32]));
    artifact_lifecycle_system(SystemExecutionContext {
        world,
        event_log: log,
        rng: &mut rng,
        active_actions: &BTreeMap::new(),
        action_defs: &ActionDefRegistry::new(),
        politics_trace: None,
        perception_trace: None,
        tick,
        system_id: SystemId::ArtifactLifecycle,
    })
    .unwrap();
}

fn transition_payloads(
    log: &EventLog,
) -> Vec<(worldwake_core::EventId, ArtifactTransitionPayload)> {
    log.events_by_tag(EventTag::ArtifactTransition)
        .iter()
        .map(|event_id| {
            (
                *event_id,
                log.get(*event_id)
                    .and_then(EventView::artifact_transition_payload)
                    .cloned()
                    .expect("artifact transition event should carry payload"),
            )
        })
        .collect()
}

fn assert_artifact_header_event(log: &EventLog, artifact: EntityId) {
    assert!(
        log.events_by_tag(EventTag::WorldMutation)
            .iter()
            .filter_map(|event_id| log.get(*event_id))
            .any(|record| record.state_deltas().iter().any(|delta| {
                matches!(
                    delta,
                    worldwake_core::StateDelta::Component(worldwake_core::ComponentDelta::Set {
                        entity,
                        component_kind: ComponentKind::ArtifactHeader,
                        after: ComponentValue::ArtifactHeader(_),
                        ..
                    }) if *entity == artifact
                )
            })),
        "event log should retain the authoritative ArtifactHeader mutation"
    );
}

// Scenario 388: S140 Bounty Fulfillment Closes Actionability
// Systems: ArtifactActions, ArtifactLifecycle, EventLog
// GoalKinds: FulfillBounty
// ActionDomains: Artifact
// Places: VillageSquare
// Principles: 4, 8, 12, 18
// Setup: programmatic fixture isolates a single posted bounty, one claimant,
//   personal reward funds, and a dead target. Rival revocation and expiry
//   branches are excluded so the only legal-effect transition is fulfillment.
// Proves: `claim_bounty` commits the Fulfilled legal-effect transition, the
//   lifecycle pass observes that transition in the same tick, and actionability
//   closes through a second append-only artifact transition.
// Chain: claim_bounty commit -> ArtifactTransition(LegalEffect::Fulfilled) ->
//   artifact_lifecycle_system -> ArtifactTransition(Actionability::Closed).
#[test]
fn bounty_fulfilled_emits_legal_effect_and_actionability_cascade() {
    let mut world = World::new(build_prototype_world()).unwrap();
    let issuer = actor(&mut world, "issuer", SQUARE);
    let claimant = actor(&mut world, "claimant", SQUARE);
    let target = actor(&mut world, "target", SQUARE);
    grant_funds(&mut world, issuer, SQUARE, CommodityKind::Coin, Quantity(4));
    kill(&mut world, target, Tick(2));
    let bounty = bounty(&mut world, issuer, target, None);
    let (defs, handlers) = registries();
    let mut log = EventLog::new();
    let mut rng = DeterministicRng::new(Seed([1; 32]));
    let mut active = BTreeMap::new();
    let mut next_id = ActionInstanceId(0);

    let action = start_bound_action(
        &mut world,
        &mut log,
        &mut rng,
        &defs,
        &handlers,
        &mut next_id,
        claimant,
        bounty,
        "claim_bounty",
    )
    .unwrap();
    let instance = worldwake_sim::ActionInstance {
        instance_id: action,
        def_id: action_id(&defs, "claim_bounty"),
        payload: defs
            .get(action_id(&defs, "claim_bounty"))
            .unwrap()
            .payload
            .clone(),
        actor: claimant,
        targets: vec![bounty],
        start_tick: Tick(5),
        remaining_duration: worldwake_sim::ActionDuration::new(1),
        status: worldwake_sim::ActionStatus::Active,
        reservation_ids: Vec::new(),
        local_state: Some(worldwake_sim::ActionState::Empty),
        body_cost_override: None,
    };
    active.insert(action, instance);
    assert!(matches!(
        tick_to_commit(
            &mut world,
            &mut log,
            &mut rng,
            &defs,
            &handlers,
            &mut active,
            action,
            Tick(6),
        )
        .unwrap(),
        TickOutcome::Committed { .. }
    ));
    run_lifecycle(&mut world, &mut log, Tick(6));

    let header = world.get_component_artifact_header(bounty).unwrap();
    assert!(matches!(
        header.legal_effect,
        ArtifactLegalEffect::Fulfilled {
            by,
            evidence,
            fulfilled_at: Tick(6),
        } if by == claimant && evidence == bounty
    ));
    assert_eq!(
        header.actionability,
        ArtifactActionability::Closed {
            closed_at: Tick(6),
            cause: CloseCause::BountyFulfilled,
        }
    );
    let transitions = transition_payloads(&log);
    assert_eq!(transitions.len(), 2);
    assert_eq!(transitions[0].1.axis, AxisName::LegalEffect);
    assert_eq!(transitions[1].1.axis, AxisName::Actionability);
    assert_eq!(transitions[1].1.cause_event, Some(transitions[0].0));
    assert_artifact_header_event(&log, bounty);
}

// Scenario 389: S140 Revoked Bounty Blocks Planner Emission
// Systems: ArtifactActions, ArtifactLifecycle, AI
// GoalKinds: FulfillBounty
// ActionDomains: Artifact
// Places: VillageSquare
// Principles: 14, 18, 20
// Setup: an issuer withdraws a single bounty after a claimant has observed it;
//   the claimant's belief is refreshed from the post-revocation artifact.
// Proves: revocation emits the legal-effect/actionability cascade, and the
//   public AI candidate surface no longer emits FulfillBounty for the closed
//   artifact while the artifact itself remains visible and inspectable.
// Chain: withdraw_bounty commit -> ArtifactTransition(LegalEffect::Revoked) ->
//   artifact_lifecycle_system -> closed believed artifact -> no FulfillBounty.
#[test]
fn warrant_revoked_blocks_subsequent_planner_emission() {
    let mut world = World::new(build_prototype_world()).unwrap();
    let issuer = actor(&mut world, "issuer", SQUARE);
    let claimant = actor(&mut world, "claimant", SQUARE);
    let target = actor(&mut world, "target", SQUARE);
    let bounty = bounty(&mut world, issuer, target, None);
    seed_belief_from_world(
        &mut world,
        &mut EventLog::new(),
        claimant,
        bounty,
        Tick(4),
        PerceptionSource::DirectObservation,
    );
    let (defs, handlers) = registries();
    let mut log = EventLog::new();
    let mut rng = DeterministicRng::new(Seed([2; 32]));
    let mut next_id = ActionInstanceId(0);
    let mut active = BTreeMap::new();
    let action = start_bound_action(
        &mut world,
        &mut log,
        &mut rng,
        &defs,
        &handlers,
        &mut next_id,
        issuer,
        bounty,
        "withdraw_bounty",
    )
    .unwrap();
    active.insert(
        action,
        ActionInstance {
            instance_id: action,
            def_id: action_id(&defs, "withdraw_bounty"),
            payload: defs
                .get(action_id(&defs, "withdraw_bounty"))
                .unwrap()
                .payload
                .clone(),
            actor: issuer,
            targets: vec![bounty],
            start_tick: Tick(5),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: worldwake_sim::ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: Some(worldwake_sim::ActionState::Empty),
            body_cost_override: None,
        },
    );
    assert!(matches!(
        tick_to_commit(
            &mut world,
            &mut log,
            &mut rng,
            &defs,
            &handlers,
            &mut active,
            action,
            Tick(6),
        )
        .unwrap(),
        TickOutcome::Committed { .. }
    ));
    run_lifecycle(&mut world, &mut log, Tick(6));
    seed_belief_from_world(
        &mut world,
        &mut log,
        claimant,
        bounty,
        Tick(7),
        PerceptionSource::DirectObservation,
    );

    let view = PerAgentBeliefView::from_world(claimant, &world);
    let offers = generate_candidates(
        &view,
        claimant,
        &BlockerMemory::default(),
        &RecipeRegistry::new(),
        Tick(7),
    );
    assert!(
        offers
            .iter()
            .all(|offer| offer.key.kind != GoalKind::FulfillBounty { bounty }),
        "closed bounty should not emit FulfillBounty after revocation"
    );
    assert_eq!(
        world
            .get_component_artifact_header(bounty)
            .unwrap()
            .actionability,
        ArtifactActionability::Closed {
            closed_at: Tick(6),
            cause: CloseCause::Revoked,
        }
    );
    let transitions = transition_payloads(&log);
    assert_eq!(transitions[0].1.axis, AxisName::LegalEffect);
    assert_eq!(transitions[1].1.axis, AxisName::Actionability);
}

// Scenario 390: S140 Expired Bounty Remains Posted But Closed
// Systems: ArtifactLifecycle, EventLog
// GoalKinds: FulfillBounty
// ActionDomains: Artifact
// Places: VillageSquare
// Principles: 7, 18, 20
// Setup: one posted bounty reaches its explicit expiration tick. No claimant,
//   withdrawal, or fulfillment path is present.
// Proves: expiry closes legal effect and actionability while preserving posted
//   visibility, so the record remains inspectable as a closed artifact.
// Chain: expiration tick -> ArtifactTransition(LegalEffect::Expired) ->
//   ArtifactTransition(Actionability::Closed) with visibility unchanged.
#[test]
fn expired_bounty_retains_posted_visibility_with_closed_actionability() {
    let mut world = World::new(build_prototype_world()).unwrap();
    let issuer = actor(&mut world, "issuer", SQUARE);
    let target = actor(&mut world, "target", SQUARE);
    let bounty = bounty(&mut world, issuer, target, Some(Tick(8)));
    let mut log = EventLog::new();

    run_lifecycle(&mut world, &mut log, Tick(8));

    let header = world.get_component_artifact_header(bounty).unwrap();
    assert_eq!(
        header.legal_effect,
        ArtifactLegalEffect::Expired {
            expired_at: Tick(8)
        }
    );
    assert!(matches!(
        header.visibility,
        worldwake_core::ArtifactVisibility::Posted { place } if place == SQUARE
    ));
    assert_eq!(
        header.actionability,
        ArtifactActionability::Closed {
            closed_at: Tick(8),
            cause: CloseCause::LegalEffectExpired,
        }
    );
    let transitions = transition_payloads(&log);
    assert_eq!(transitions.len(), 2);
    assert_eq!(transitions[1].1.cause_event, Some(transitions[0].0));
}

// Scenario 391: S140 Suspended Legal Effect Restores Without Closing
// Systems: ArtifactLifecycle, EventLog
// GoalKinds: FulfillBounty
// ActionDomains: Artifact
// Places: VillageSquare
// Principles: 18, 21, 26
// Setup: an office force-control record first becomes contested, then resolves.
//   The bounty is issued by that office so the record events are lawful source
//   carriers for jurisdiction suspension/restoration. No closure cause is
//   emitted.
// Proves: source-backed Suspended and restored Active legal-effect transitions
//   remain append-only and do not create a spurious actionability closure.
// Chain: ForceControl(contested) record event ->
//   artifact_lifecycle_system -> ArtifactTransition(LegalEffect::Suspended) ->
//   ForceControl(resolved) record event -> ArtifactTransition(LegalEffect::Active).
#[test]
fn suspended_legal_effect_restores_on_resolution_event() {
    let mut world = World::new(build_prototype_world()).unwrap();
    let issuer = actor(&mut world, "issuer", SQUARE);
    let office = office(&mut world, "Market Warden");
    let record = record(&mut world, issuer, SQUARE, RecordKind::OfficeRegister);
    let target = actor(&mut world, "target", SQUARE);
    let bounty = office_bounty(&mut world, issuer, office, target, None);
    let mut log = EventLog::new();

    let suspend_source =
        force_control_source_event(&mut world, &mut log, record, office, true, Tick(9));
    run_lifecycle(&mut world, &mut log, Tick(9));
    assert_eq!(
        world
            .get_component_artifact_header(bounty)
            .unwrap()
            .actionability,
        ArtifactActionability::Actionable
    );

    let restore_source =
        force_control_source_event(&mut world, &mut log, record, office, false, Tick(10));
    run_lifecycle(&mut world, &mut log, Tick(10));

    let header = world.get_component_artifact_header(bounty).unwrap();
    assert_eq!(
        header.legal_effect,
        ArtifactLegalEffect::Active { expires_at: None }
    );
    assert_eq!(header.actionability, ArtifactActionability::Actionable);
    assert_eq!(
        transition_payloads(&log)
            .iter()
            .filter(|(_, payload)| payload.axis == AxisName::LegalEffect)
            .count(),
        2
    );
    let legal_transitions = transition_payloads(&log)
        .into_iter()
        .filter(|(_, payload)| payload.axis == AxisName::LegalEffect)
        .collect::<Vec<_>>();
    assert_eq!(legal_transitions[0].1.cause_event, Some(suspend_source));
    assert_eq!(
        legal_transitions[0].1.new,
        ArtifactAxisValue::LegalEffect(ArtifactLegalEffect::Suspended {
            reason: SuspensionReason::JurisdictionDispute,
            suspended_at: Tick(9),
        })
    );
    assert_eq!(legal_transitions[1].1.cause_event, Some(restore_source));
}

// Scenario 392: S140 Refuted False Rumor Closes Actionability
// Systems: ArtifactLifecycle, EventLog
// GoalKinds: FulfillBounty
// ActionDomains: Artifact
// Places: VillageSquare
// Principles: 15, 16, 18
// Setup: a posted bounty receives an artifact-addressed credibility
//   refutation record entry with a concrete evidence entity. No legal-effect
//   closure is authored.
// Proves: the lifecycle credibility stage converts the record source into a
//   Credibility::Refuted transition, and actionability closes from that same
//   append-only transition.
// Chain: ArtifactCredibilityRefutation record event ->
//   ArtifactTransition(Credibility::Refuted) ->
//   ArtifactTransition(Actionability::Closed).
#[test]
fn refuted_false_rumor_cascades_to_closed_actionability_via_credibility_handler() {
    let mut world = World::new(build_prototype_world()).unwrap();
    let issuer = actor(&mut world, "issuer", SQUARE);
    let evidence = actor(&mut world, "evidence witness", SQUARE);
    let target = actor(&mut world, "target", SQUARE);
    let record = record(&mut world, issuer, SQUARE, RecordKind::CrimeRegister);
    let bounty = bounty(&mut world, issuer, target, None);
    let mut log = EventLog::new();

    let source_event =
        artifact_refutation_source_event(&mut world, &mut log, record, bounty, evidence, Tick(11));
    run_lifecycle(&mut world, &mut log, Tick(11));

    let header = world.get_component_artifact_header(bounty).unwrap();
    assert_eq!(
        header.credibility,
        ArtifactCredibility::Refuted {
            refuted_at: Tick(11),
            evidence,
        }
    );
    assert_eq!(
        header.actionability,
        ArtifactActionability::Closed {
            closed_at: Tick(11),
            cause: CloseCause::Refuted,
        }
    );
    let transitions = transition_payloads(&log);
    assert_eq!(transitions.len(), 2);
    assert_eq!(transitions[0].1.axis, AxisName::Credibility);
    assert_eq!(transitions[0].1.cause_event, Some(source_event));
    assert_eq!(transitions[1].1.axis, AxisName::Actionability);
    assert_eq!(transitions[1].1.cause_event, Some(transitions[0].0));
}
