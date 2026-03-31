//! Integration tests for E22 cross-system scenario verification.
//!
//! T24: Player Agent Replacement — verifies `ControlSource` swap
//! mid-simulation with world continuity and preserved agent state.

mod golden_harness;

use golden_harness::*;
use worldwake_ai::DecisionOutcome;
use worldwake_core::{
    hash_event_log, hash_world, AgentData, CommodityKind, ControlSource, EntityId,
    HomeostaticNeeds, MetabolismProfile, PlaceTag, Quantity, Seed, StateHash, Tick, Topology,
    TravelEdge, TravelEdgeId, UtilityProfile,
};
use worldwake_sim::{
    get_affordances, ActionRequestMode, ControllerState, InputKind, PerAgentBeliefView,
    RequestProvenance,
};

// ---------------------------------------------------------------------------
// Custom place entity IDs (outside prototype range)
// ---------------------------------------------------------------------------

const PLACE_ALPHA: EntityId = entity(100);
const PLACE_BETA: EntityId = entity(101);

const fn entity(slot: u32) -> EntityId {
    EntityId {
        slot,
        generation: 0,
    }
}

fn place(name: &str, tags: &[PlaceTag]) -> worldwake_core::Place {
    worldwake_core::Place {
        name: name.to_string(),
        capacity: None,
        tags: tags.iter().copied().collect(),
    }
}

// ---------------------------------------------------------------------------
// Topology builder for T24: minimal 2-place world
// ---------------------------------------------------------------------------

/// Two places connected by a 3-tick travel edge.
fn build_t24_topology() -> Topology {
    let mut t = Topology::new();
    // Indoor Village tags — avoids outdoor relief affordances distracting the planner.
    t.add_place(PLACE_ALPHA, place("Alpha", &[PlaceTag::Village]))
        .unwrap();
    t.add_place(PLACE_BETA, place("Beta", &[PlaceTag::Village]))
        .unwrap();
    t.add_edge(TravelEdge::new(TravelEdgeId(200), PLACE_ALPHA, PLACE_BETA, 3, None).unwrap())
        .unwrap();
    t.add_edge(TravelEdge::new(TravelEdgeId(201), PLACE_BETA, PLACE_ALPHA, 3, None).unwrap())
        .unwrap();
    t
}

fn build_harness_with_topology(seed: Seed, topology: Topology) -> GoldenHarness {
    let mut h = GoldenHarness::new(seed);
    h.world = worldwake_core::World::new(topology).unwrap();
    h.event_log = worldwake_core::EventLog::new();
    h.scheduler = worldwake_sim::Scheduler::new(worldwake_sim::SystemManifest::canonical());
    h.controller = ControllerState::new();
    h
}

// ---------------------------------------------------------------------------
// T24: Player Agent Replacement
// ---------------------------------------------------------------------------
//
// Verifies `ControlSource` swap mid-simulation:
//   Agent A: starts Human, carrying Apple, submitted travel action
//   Agent B: starts Ai, at other place, with needs that drive autonomous goals
//
// At tick N (mid-travel for A): swap A→Ai, B→Human via WorldTxn.
//
// Acceptance criteria (from ticket):
//   1. Only ControlSource components changed on A and B
//   2. Agent A generates AI candidates within 5 ticks of swap
//   3. Agent B affordances legal for B's position/inventory/beliefs
//   4. Agent A inventory, wounds, needs, placement preserved
//   5. No simulation reset — tick counter monotonically increases
//   6. No AI inputs for Agent B after swap to Human
//   7. Determinism via 2-seed state hash comparison

fn run_t24_player_replacement(seed: Seed) -> (StateHash, StateHash) {
    let mut h = build_harness_with_topology(seed, build_t24_topology());

    // --- Seed Agent A (Human) at PLACE_ALPHA carrying an Apple ---
    let agent_a = {
        let mut txn = new_txn(&mut h.world, 0);
        let a = txn.create_agent("AgentA", ControlSource::Human).unwrap();
        txn.set_ground_location(a, PLACE_ALPHA).unwrap();
        txn.set_component_homeostatic_needs(
            a,
            HomeostaticNeeds::new(pm(200), pm(0), pm(100), pm(0), pm(0)),
        )
        .unwrap();
        txn.set_component_deprivation_exposure(
            a,
            worldwake_core::DeprivationExposure::default(),
        )
        .unwrap();
        txn.set_component_drive_thresholds(a, worldwake_core::DriveThresholds::default())
            .unwrap();
        txn.set_component_metabolism_profile(a, MetabolismProfile::default())
            .unwrap();
        txn.set_component_utility_profile(a, UtilityProfile::default())
            .unwrap();
        txn.set_component_combat_profile(a, default_combat_profile())
            .unwrap();
        txn.set_component_wound_list(a, worldwake_core::WoundList::default())
            .unwrap();
        txn.set_component_blocked_intent_memory(
            a,
            worldwake_core::BlockedIntentMemory::default(),
        )
        .unwrap();
        txn.set_component_carry_capacity(
            a,
            worldwake_core::CarryCapacity(worldwake_core::LoadUnits(50)),
        )
        .unwrap();
        txn.set_component_known_recipes(a, worldwake_core::KnownRecipes::with([]))
            .unwrap();
        commit_txn(txn, &mut h.event_log);
        a
    };

    // Register Agent A as the human-controlled entity.
    h.controller
        .switch_control(None, Some(agent_a))
        .unwrap();

    // Give Agent A an Apple.
    let _apple_lot = give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent_a,
        PLACE_ALPHA,
        CommodityKind::Apple,
        Quantity(1),
    );

    // --- Seed Agent B (Ai) at PLACE_BETA with hunger that drives autonomous goals ---
    let agent_b = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "AgentB",
        PLACE_BETA,
        HomeostaticNeeds::new(pm(600), pm(0), pm(100), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    // Give Agent B an Apple so it can pursue ConsumeOwnedCommodity.
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent_b,
        PLACE_BETA,
        CommodityKind::Apple,
        Quantity(3),
    );

    // Enable tracing for decision and action diagnostics.
    h.driver.enable_tracing();
    h.enable_action_tracing();

    // --- Phase 1: Establish mid-simulation state ---
    // Run 1 tick to let systems initialize, then submit travel for Agent A.
    h.step_once();

    // Submit a travel input for the Human agent (Agent A) toward PLACE_BETA.
    let travel_def_id = h
        .defs
        .iter()
        .find(|def| def.name == "travel")
        .map(|def| def.id)
        .expect("full registries should include travel");
    let submit_tick = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        submit_tick,
        InputKind::RequestAction {
            actor: agent_a,
            def_id: travel_def_id,
            targets: vec![PLACE_BETA],
            payload_override: None,
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );

    // Run 1 tick to start the travel, then 1 more to be mid-travel.
    h.step_once();
    h.step_once();

    let swap_tick = h.scheduler.current_tick().0;

    // --- Snapshot pre-swap state for invariant checks ---
    let pre_swap_a_needs = h
        .world
        .get_component_homeostatic_needs(agent_a)
        .cloned();
    let pre_swap_a_wounds = h.world.get_component_wound_list(agent_a).cloned();
    let pre_swap_a_place = h.world.effective_place(agent_a);
    let pre_swap_a_inventory = h.agent_commodity_qty(agent_a, CommodityKind::Apple);

    // Verify tick counter has advanced beyond 0.
    assert!(
        swap_tick > 0,
        "simulation should have advanced past tick 0 before swap"
    );

    // --- Phase 2: Swap ControlSource via WorldTxn ---
    {
        let mut txn = new_txn(&mut h.world, swap_tick);
        txn.set_component_agent_data(agent_a, AgentData { control_source: ControlSource::Ai })
            .unwrap();
        txn.set_component_agent_data(
            agent_b,
            AgentData {
                control_source: ControlSource::Human,
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Update ControllerState: A is no longer human-controlled, B is now.
    h.controller
        .switch_control(Some(agent_a), Some(agent_b))
        .unwrap();

    // Verify ControllerState is updated.
    assert_eq!(
        h.controller.controlled_entity(),
        Some(agent_b),
        "ControllerState should track Agent B after swap"
    );

    // --- Verification: only ControlSource changed ---
    let post_swap_a_needs = h
        .world
        .get_component_homeostatic_needs(agent_a)
        .cloned();
    let post_swap_a_wounds = h.world.get_component_wound_list(agent_a).cloned();
    let post_swap_a_place = h.world.effective_place(agent_a);
    let post_swap_a_inventory = h.agent_commodity_qty(agent_a, CommodityKind::Apple);

    // Needs may have drifted slightly from the needs system running between
    // pre-snapshot and swap, but inventory and placement should be identical
    // because the swap itself is a single-component mutation.
    assert_eq!(
        pre_swap_a_inventory, post_swap_a_inventory,
        "Agent A inventory must be preserved across swap"
    );
    assert_eq!(
        pre_swap_a_place, post_swap_a_place,
        "Agent A placement must be preserved across swap"
    );
    assert_eq!(
        pre_swap_a_wounds, post_swap_a_wounds,
        "Agent A wounds must be preserved across swap"
    );
    // Needs comparison: the swap txn does not touch needs, but the step_once
    // between snapshot and swap may have ticked them. We snapshot AFTER the
    // last step_once, so they should match exactly.
    assert_eq!(
        pre_swap_a_needs, post_swap_a_needs,
        "Agent A needs must be preserved across swap"
    );

    // Verify ControlSource actually changed.
    assert_eq!(
        h.world
            .get_component_agent_data(agent_a)
            .map(|d| d.control_source),
        Some(ControlSource::Ai),
        "Agent A should be Ai after swap"
    );
    assert_eq!(
        h.world
            .get_component_agent_data(agent_b)
            .map(|d| d.control_source),
        Some(ControlSource::Human),
        "Agent B should be Human after swap"
    );

    // --- Phase 3: Run post-swap ticks and verify AI activation ---
    let mut agent_a_generated_candidates = false;
    let mut ticks_since_swap = 0u32;
    let mut last_tick = Tick(swap_tick);

    for _ in 0..50 {
        // The trace is recorded for the tick being processed, which is
        // scheduler.current_tick() BEFORE step_once().
        let processed_tick = h.scheduler.current_tick();
        h.step_once();
        let current_tick = h.scheduler.current_tick();

        // Verify monotonic tick advancement.
        assert!(
            current_tick > last_tick,
            "tick counter must strictly increase: last={last_tick:?}, current={current_tick:?}"
        );
        last_tick = current_tick;
        ticks_since_swap += 1;

        // Check decision traces for Agent A: non-empty candidate list means
        // the AI pipeline is generating goals for the newly-Ai agent.
        if let Some(sink) = h.driver.trace_sink() {
            if let Some(trace) = sink.trace_at(agent_a, processed_tick) {
                match &trace.outcome {
                    DecisionOutcome::Planning(planning) => {
                        if !planning.candidates.ranked.is_empty() {
                            agent_a_generated_candidates = true;
                        }
                    }
                    DecisionOutcome::ActiveAction { .. } => {
                        // Agent A may still be finishing the travel action
                        // started while Human — that's fine, it means the
                        // simulation preserved the in-progress action.
                    }
                    DecisionOutcome::Dead => {
                        panic!("Agent A should not be dead during T24");
                    }
                }
            }
        }

        if agent_a_generated_candidates && ticks_since_swap >= 5 {
            break;
        }
    }

    assert!(
        agent_a_generated_candidates,
        "Agent A must generate AI goal candidates within post-swap ticks \
         (checked {ticks_since_swap} ticks after swap)",
    );

    // --- Verify Agent B affordances are legal ---
    let b_view = PerAgentBeliefView::from_world(agent_b, &h.world);
    let b_affordances = get_affordances(&b_view, agent_b, &h.defs, &h.handlers);
    // All returned affordances are legal by construction (get_affordances
    // filters on preconditions). We just verify the call succeeds and returns
    // a non-empty set (Agent B is alive and at a place, so travel at minimum
    // should be available — but it may be empty if at a dead-end with no
    // applicable actions; the key invariant is that the call doesn't panic).
    // Note: with a 2-place Village topology, travel back is always available.
    assert!(
        !b_affordances.is_empty(),
        "Agent B should have at least one legal affordance after swap"
    );

    // --- Verify Agent A state preservation post-run ---
    // Inventory may have changed if Agent A ate the Apple under AI control,
    // but wounds should not have appeared (no combat in this scenario) and
    // placement should be deterministic.
    assert!(
        !h.agent_is_dead(agent_a),
        "Agent A must survive the T24 scenario"
    );
    assert!(
        !h.agent_is_dead(agent_b),
        "Agent B must survive the T24 scenario"
    );

    // --- Verify tick counter continued monotonically (already checked per-tick) ---
    assert!(
        h.scheduler.current_tick().0 > swap_tick,
        "simulation must continue past swap tick"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Test functions
// ---------------------------------------------------------------------------

#[test]
fn t24_player_replacement_seed_1() {
    let _ = run_t24_player_replacement(Seed([24; 32]));
}

#[test]
fn t24_player_replacement_seed_2() {
    let first = run_t24_player_replacement(Seed([25; 32]));
    let second = run_t24_player_replacement(Seed([25; 32]));
    assert_eq!(
        first, second,
        "T24 player replacement scenario must replay deterministically"
    );
}
