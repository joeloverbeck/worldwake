//! Golden tests for S135 planner perception budget and observation omission.
//!
//! These tests exercise the live S135 contract at the strongest available
//! golden surfaces: perception writes bounded omission records, same-place
//! planning has no second entity cap, and planner/effect revalidation carries
//! typed omission reasons when an omitted anchor is absent from the snapshot.

mod golden_harness;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use golden_harness::*;
use worldwake_ai::{
    GoalOffer, HypotheticalEffectSink, PlannerOpKind, PlanningEntityRef, PlanningState,
    build_planning_snapshot, build_semantics_table, search_plan,
};
use worldwake_core::{
    AgentBeliefStore, AgentData, CommodityKind, CommunicationClass, ControlSource, Discrepancy,
    EntityId, EntityKind, GoalKey, GoalKind, HomeostaticNeeds, MetabolismProfile,
    ObservationOmission, ObservationOmissionLog, OmissionReason, OpportunityAnchor,
    PerceptionProfile, PrototypePlace, Quantity, Seed, TellTopic, Tick, prototype_place_entity,
};
use worldwake_sim::{EffectEntityRef, EffectPrecondition, EffectSink, PerAgentBeliefView};

const TEST_PLACE: EntityId = ORCHARD_FARM;
const FOREST_PATH: EntityId = prototype_place_entity(PrototypePlace::ForestPath);

fn stable_metabolism() -> MetabolismProfile {
    MetabolismProfile::new(
        pm(0),
        pm(0),
        pm(0),
        pm(0),
        pm(0),
        pm(20),
        nz(480),
        nz(240),
        nz(120),
        nz(200),
        nz(8),
        nz(12),
        nz(8),
        pm(0),
        pm(0),
        pm(0),
        pm(0),
    )
}

fn set_control_source(h: &mut GoldenHarness, agent: EntityId, control_source: ControlSource) {
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_agent_data(agent, AgentData { control_source })
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn place_ground_lots(
    h: &mut GoldenHarness,
    commodity: CommodityKind,
    quantity: Quantity,
    count: usize,
) -> Vec<EntityId> {
    let mut txn = new_txn(&mut h.world, 0);
    let mut lots = Vec::with_capacity(count);
    for _ in 0..count {
        let lot = txn.create_item_lot(commodity, quantity).unwrap();
        txn.set_ground_location(lot, TEST_PLACE).unwrap();
        lots.push(lot);
    }
    commit_txn(txn, &mut h.event_log);
    lots
}

fn observer_with_profile(
    h: &mut GoldenHarness,
    name: &str,
    profile: PerceptionProfile,
) -> EntityId {
    let observer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        name,
        TEST_PLACE,
        HomeostaticNeeds::new_sated(),
        stable_metabolism(),
        worldwake_core::UtilityProfile::default(),
    );
    set_control_source(h, observer, ControlSource::Human);
    set_agent_perception_profile(&mut h.world, &mut h.event_log, observer, profile);
    observer
}

fn omission_sets(
    store: &AgentBeliefStore,
) -> (BTreeSet<EntityId>, BTreeMap<EntityId, OmissionReason>) {
    let omitted = store
        .observation_omission_log
        .entries
        .iter()
        .map(|entry| (entry.omitted_entity, entry.reason))
        .collect::<BTreeMap<_, _>>();
    let known = store
        .known_entities
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    (known, omitted)
}

fn assert_known_and_omitted_are_disjoint(store: &AgentBeliefStore) {
    let (known, omitted) = omission_sets(store);
    assert!(
        known.is_disjoint(&omitted.keys().copied().collect()),
        "omission log and retained beliefs must not name the same entity; known={known:?} omitted={:?}",
        omitted.keys().collect::<Vec<_>>()
    );
}

// Scenario 381: Perception Overbudget Writes Omission Records
//
// Systems: Perception, AI
// ActionDomains: N/A
// Places: OrchardFarm
// Principles: 7, 12, 14, 16
//
// Setup: A human-controlled observer with `observation_budget = 24` and `omission_log_capacity = 64` remains co-located with sixty equal-priority waste lots for one tick.
//
// Proves: Perception writes thirty-six deterministic `OverBudget` omission records, retained beliefs are disjoint from omitted entities, and the same-place planning snapshot has no second cap over co-located local entities.
//
// Chain: same-place passive perception -> priority sort -> observation-budget truncation -> omission ring-buffer write -> planning snapshot without a second local cap.
#[test]
fn golden_perception_omission_overbudget_writes() {
    let mut h = GoldenHarness::new(Seed([0x87; 32]));
    let lots = place_ground_lots(&mut h, CommodityKind::Waste, Quantity(1), 60);
    let observer = observer_with_profile(
        &mut h,
        "Observer",
        PerceptionProfile {
            observation_budget: 24,
            omission_log_capacity: 64,
            observation_fidelity: pm(1000),
            ..PerceptionProfile::default()
        },
    );

    h.step_once();

    let store = h
        .world
        .get_component_agent_belief_store(observer)
        .cloned()
        .unwrap_or_else(AgentBeliefStore::new);
    assert_eq!(store.observation_omission_log.entries.len(), 36);
    assert_known_and_omitted_are_disjoint(&store);

    let omitted_entities = store
        .observation_omission_log
        .entries
        .iter()
        .map(|entry| {
            assert_eq!(
                entry.reason,
                OmissionReason::OverBudget {
                    budget: 24,
                    candidates_seen: 60
                }
            );
            entry.omitted_entity
        })
        .collect::<Vec<_>>();
    let mut sorted_omitted = omitted_entities.clone();
    sorted_omitted.sort();
    assert_eq!(
        omitted_entities, sorted_omitted,
        "equal-priority omissions should be emitted in deterministic entity order"
    );

    let view = PerAgentBeliefView::new_at_tick(observer, Tick(1), &h.world, &store);
    let snapshot = build_planning_snapshot(&view, observer, &BTreeSet::new(), &BTreeSet::new(), 0);
    let state = PlanningState::new(&snapshot);
    let planner_visible_lots = lots
        .iter()
        .copied()
        .filter(|lot| {
            state.entity_kind_ref(PlanningEntityRef::Authoritative(*lot))
                == Some(EntityKind::ItemLot)
        })
        .count();
    assert_eq!(
        planner_visible_lots, 60,
        "same-place planning snapshot should not apply a second entity cap"
    );
}

// Scenario 382: Need-Weighted Perception Keeps Food Above Waste
//
// Systems: Perception, Needs
// ActionDomains: N/A
// Places: OrchardFarm
// Principles: 3, 7, 14, 20
//
// Setup: A hungry human-controlled observer with `observation_budget = 12` remains co-located with ten apple lots and twenty waste lots for one tick.
//
// Proves: Need-boosted item salience keeps all food lots in retained beliefs while low-priority waste fills the omission log with `OverBudget` records.
//
// Chain: hunger pressure -> item need salience boost -> priority sort -> observation-budget truncation -> retained food beliefs plus omitted waste.
#[test]
fn golden_perception_omission_need_weighted_priority() {
    let mut h = GoldenHarness::new(Seed([0x88; 32]));
    let food = place_ground_lots(&mut h, CommodityKind::Apple, Quantity(1), 10);
    let waste = place_ground_lots(&mut h, CommodityKind::Waste, Quantity(1), 20);
    let observer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Hungry Observer",
        TEST_PLACE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        stable_metabolism(),
        worldwake_core::UtilityProfile::default(),
    );
    set_control_source(&mut h, observer, ControlSource::Human);
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        observer,
        PerceptionProfile {
            observation_budget: 12,
            omission_log_capacity: 64,
            opportunity_floor_permille: worldwake_core::default_opportunity_floor_permille(),
            need_salience_boost: pm(500),
            need_salience_urgency_threshold: pm(500),
            observation_fidelity: pm(1000),
            ..PerceptionProfile::default()
        },
    );

    h.step_once();

    let store = h
        .world
        .get_component_agent_belief_store(observer)
        .cloned()
        .unwrap_or_else(AgentBeliefStore::new);
    assert_known_and_omitted_are_disjoint(&store);
    for lot in &food {
        assert!(
            store.known_entities.contains_key(lot),
            "all food lots should be retained under hunger salience; missing={lot:?}"
        );
    }

    let omitted = store
        .observation_omission_log
        .entries
        .iter()
        .map(|entry| entry.omitted_entity)
        .collect::<BTreeSet<_>>();
    let waste_set = waste.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(omitted.len(), 18);
    assert!(
        omitted.is_subset(&waste_set),
        "only lower-priority waste should be omitted; omitted={omitted:?}"
    );
    assert!(store.observation_omission_log.entries.iter().all(|entry| {
        entry.reason
            == OmissionReason::OverBudget {
                budget: 12,
                candidates_seen: 30,
            }
    }));
}

// Scenario 383: Omitted Anchor Reason Surfaces In Planning And Revalidation
//
// Systems: AI, PlanningSnapshot
// ActionDomains: Social, Inventory
// Places: OrchardFarm, ForestPath
// Principles: 12, 14, 16, 29
//
// Setup: An observer's belief store contains a current omission record for a remote listener that is absent from the planning snapshot. A `ShareBelief` goal is exact-bound to that listener, and a hypothetical co-location revalidation checks the same missing target.
//
// Proves: The planner annotates the root `Tell` candidate with the typed omission reason, and the hypothetical effect sink returns the matching `Discrepancy::Omission(reason)` instead of collapsing the failure into a generic missing observation.
//
// Chain: omission log entry -> planner root candidate trace -> hypothetical revalidation -> typed discrepancy preservation.
#[test]
fn golden_perception_omission_revalidation_typed_reason() {
    let mut h = GoldenHarness::new(Seed([0x89; 32]));
    let actor = observer_with_profile(&mut h, "Speaker", PerceptionProfile::default());
    let listener = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Omitted Listener",
        FOREST_PATH,
        HomeostaticNeeds::new_sated(),
        stable_metabolism(),
        worldwake_core::UtilityProfile::default(),
    );
    let subject = place_ground_lots(&mut h, CommodityKind::Apple, Quantity(1), 1)[0];
    let reason = OmissionReason::OverBudget {
        budget: 5,
        candidates_seen: 12,
    };
    let mut store = h
        .world
        .get_component_agent_belief_store(actor)
        .cloned()
        .unwrap_or_else(AgentBeliefStore::new);
    store.observation_omission_log = ObservationOmissionLog {
        entries: VecDeque::from([ObservationOmission {
            omitted_entity: listener,
            reason,
            observed_tick: Tick(7),
        }]),
    };
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_agent_belief_store(actor, store.clone())
        .unwrap();
    commit_txn(txn, &mut h.event_log);

    let view = PerAgentBeliefView::new_at_tick(actor, Tick(7), &h.world, &store);
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::new(),
        &BTreeSet::from([TEST_PLACE]),
        0,
    );
    let state = PlanningState::new(&snapshot);
    assert_eq!(
        state.entity_kind_ref(PlanningEntityRef::Authoritative(listener)),
        None,
        "listener must be absent from the snapshot for omission attribution to be meaningful"
    );

    let goal_kind = GoalKind::ShareBelief {
        listener,
        topic: TellTopic::EntityBelief { subject },
        communication_class: CommunicationClass::Gossip,
    };
    let goal = GoalOffer {
        anchor: OpportunityAnchor::Entity(listener),
        key: GoalKey::from(goal_kind),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([TEST_PLACE]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: worldwake_ai::motive_source_mapping::derive_default_motive_sources(
            &goal_kind,
            &OpportunityAnchor::Entity(listener),
            Tick(7),
        ),
        acquisition_quantity: None,
    };
    let mut expansions = Vec::new();
    let _ = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&h.defs),
        &h.defs,
        &h.handlers,
        &worldwake_core::CognitiveProfile::default(),
        &worldwake_core::ExecutionBudget::default(),
        &h.recipes,
        &worldwake_core::BlockerMemory::default(),
        Tick(7),
        None,
        Some(&mut expansions),
    );
    let root = expansions
        .iter()
        .find(|summary| summary.depth == 0)
        .expect("root expansion summary should be recorded");
    let tell_candidate = root
        .root_candidates
        .iter()
        .find(|candidate| {
            candidate.op_kind == Some(PlannerOpKind::Tell)
                && candidate.authoritative_targets == vec![listener]
        })
        .expect("synthesized tell candidate should be traced");
    assert_eq!(tell_candidate.omitted_anchor, Some(reason));

    let sink = HypotheticalEffectSink::new(state);
    let discrepancy = sink
        .check_precondition(
            &EffectPrecondition::CoLocated {
                actor: EffectEntityRef::Actor,
                target: EffectEntityRef::Entity(listener),
            },
            actor,
            &[listener],
        )
        .expect_err("missing omitted target should fail revalidation");
    assert_eq!(discrepancy, Discrepancy::Omission(reason));
    assert!(!store.known_entities.contains_key(&listener));
}
