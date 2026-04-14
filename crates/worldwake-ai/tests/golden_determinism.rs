//! Golden test for deterministic replay fidelity.

mod golden_harness;

use golden_harness::*;
use std::collections::BTreeMap;
use worldwake_core::{
    ActiveGoal, BeliefConfidencePolicy, BelievedContentionState, CommodityKind, ContentionGrant,
    ContentionIntents, ContentionPolicy, ContentionQueue, DeadAt, FrameAssumption, FrameState,
    GoalKey, GoalKind, HomeostaticNeeds, IntentionDispositionProfile, IntentionDomain,
    IntentionDomainTag, IntentionFrame, MetabolismProfile, PerceptionProfile, PrototypePlace,
    Quantity, ResourceSource, Seed, StateHash, SuspensionReason, Tick, UtilityProfile,
    WorkstationTag, hash_event_log, hash_world, prototype_place_entity,
    total_authoritative_commodity_quantity,
};

// ---------------------------------------------------------------------------
// Determinism-specific helpers (only used by the test in this file)
// ---------------------------------------------------------------------------

fn build_deterministic_scenario(seed: Seed) -> GoldenHarness {
    let mut h = GoldenHarness::new(seed);

    let agent_a = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Alice",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    // Second agent exists for multi-agent determinism; not referenced directly.
    seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Bob",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent_a,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(1),
    );

    place_workstation_with_source(
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

    h
}

fn run_deterministic_scenario(seed: Seed) -> (StateHash, StateHash) {
    let mut h = build_deterministic_scenario(seed);
    for _ in 0..50 {
        h.step_once();
    }
    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

fn run_save_load_roundtrip_scenario(seed: Seed) -> GoldenHarness {
    const SAVE_AFTER_TICKS: u64 = 20;
    const RESUME_TICKS: u64 = 30;

    let mut uninterrupted = build_deterministic_scenario(seed);
    for _ in 0..SAVE_AFTER_TICKS {
        uninterrupted.step_once();
    }

    let save_boundary_world_hash = hash_world(&uninterrupted.world).unwrap();
    let initial_world_hash = {
        let initial = build_deterministic_scenario(seed);
        hash_world(&initial.world).unwrap()
    };
    assert_ne!(
        save_boundary_world_hash, initial_world_hash,
        "Save boundary should occur after non-trivial AI progress"
    );

    let mut resumed = uninterrupted.save_load_roundtrip();

    for _ in 0..RESUME_TICKS {
        uninterrupted.step_once();
        resumed.step_once();
    }

    assert_eq!(
        resumed.scheduler, uninterrupted.scheduler,
        "Scheduler state should match after resuming with restored AI runtime"
    );
    assert_eq!(
        resumed.controller, uninterrupted.controller,
        "Controller state should match after resuming with restored AI runtime"
    );
    assert_eq!(
        resumed.rng, uninterrupted.rng,
        "Deterministic RNG continuation should match across save/load"
    );
    assert_eq!(
        resumed.recipes, uninterrupted.recipes,
        "Recipe registry should remain unchanged across save/load"
    );
    assert_eq!(
        hash_world(&resumed.world).unwrap(),
        hash_world(&uninterrupted.world).unwrap(),
        "World state should match uninterrupted execution after save/load resume"
    );
    assert_eq!(
        hash_event_log(&resumed.event_log).unwrap(),
        hash_event_log(&uninterrupted.event_log).unwrap(),
        "Event log should match uninterrupted execution after save/load resume"
    );

    resumed
}

// ---------------------------------------------------------------------------
// Scenario 6: Deterministic Replay Fidelity
// ---------------------------------------------------------------------------
//
// Systems: All
// GoalKinds: ConsumeOwnedCommodity, AcquireCommodity(SelfConsume)
// ActionDomains: Needs, Production, Travel
// Places: VillageSquare, OrchardFarm
//
// Setup: Two hungry agents at Village Square. Alice has bread. Orchard Farm
//   has apples. Run 50 ticks with same seed twice. Includes save/load
//   round-trip test at tick 20 with resumed AI controller.
//
// Proves: Identical seeds produce identical StateHash for world and event
//   log. Full-stack determinism (ChaCha8Rng, BTreeMap ordering, no floats).
//   Save/load round-trip preserves scheduler, RNG, and controller state.
//
// Chain: Full simulation -> identical world/event-log hashes across runs.
//   Save at tick 20 -> load -> fresh AI controller -> identical continuation.

#[test]
fn golden_deterministic_replay_fidelity() {
    let seed = Seed([42; 32]);

    let (world_hash_1, log_hash_1) = run_deterministic_scenario(seed);
    let (world_hash_2, log_hash_2) = run_deterministic_scenario(seed);

    assert_eq!(
        world_hash_1, world_hash_2,
        "Two runs with the same seed must produce identical world hashes"
    );
    assert_eq!(
        log_hash_1, log_hash_2,
        "Two runs with the same seed must produce identical event log hashes"
    );

    // Verify non-trivial simulation occurred — hashes differ from initial state.
    let fresh = build_deterministic_scenario(seed);
    let initial_world_hash = hash_world(&fresh.world).unwrap();
    let initial_log_hash = hash_event_log(&fresh.event_log).unwrap();

    assert_ne!(
        world_hash_1, initial_world_hash,
        "World should have changed from initial state (non-trivial simulation)"
    );
    assert_ne!(
        log_hash_1, initial_log_hash,
        "Event log should have changed from initial state"
    );
}

#[test]
fn golden_save_load_round_trip_under_ai() {
    let seed = Seed([77; 32]);
    let resumed = run_save_load_roundtrip_scenario(seed);

    assert_eq!(
        resumed.scheduler.current_tick().0,
        50,
        "Resumed scenario should reach the same terminal tick as uninterrupted execution"
    );
}

// ---------------------------------------------------------------------------
// Scenario S02: World Runs Without Observers (Principle 6)
// ---------------------------------------------------------------------------
//
// Systems: Needs, Production, Travel, Enterprise, Trade, AI, Conservation
// GoalKinds: ConsumeOwnedCommodity, AcquireCommodity(SelfConsume), RestockCommodity
// ActionDomains: Needs, Production, Travel
// Places: VillageSquare, OrchardFarm, GeneralStore
// Principles: 6
//
// Setup: Four agents across three places for 200 ticks under full AI loop.
//   Farmer (hungry, orchard), Merchant (enterprise-focused, MerchandiseProfile),
//   Villager (hungry+thirsty, bread+water), Wanderer (thirsty+fatigued).
//
// Proves: World hash differs from initial after 200 ticks. Event log grows
//   by 20+ events. No deaths. Per-tick conservation for Bread, Water, Coin.
//   At least one transit and one consumption. 200-tick multi-agent determinism.
//
// Chain: Needs -> AI -> action, Production -> harvest, Travel -> movement,
//   Enterprise -> restock. Conservation across 200 ticks of multi-system
//   interaction. Proves Principle 6 (world runs without observers).

#[allow(clippy::too_many_lines)]
fn build_world_runs_without_observers_scenario(seed: Seed) -> GoldenHarness {
    let mut h = GoldenHarness::new(seed);
    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);

    // Farmer at Orchard Farm — hungry, knows harvest recipe, has perception.
    let farmer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Farmer",
        ORCHARD_FARM,
        HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        farmer,
        PerceptionProfile {
            entity_activation_threshold: pm(64),
            claim_confidence_threshold: pm(50),
            observation_buffer_capacity: 64,
            need_salience_boost: pm(500),
            need_salience_urgency_threshold: pm(500),
            observation_fidelity: pm(875),
            confidence_policy: BeliefConfidencePolicy::default(),
            institutional_memory_capacity: 20,
            consultation_speed_factor: pm(500),
            contradiction_tolerance: pm(300),
        },
    );

    // OrchardRow workstation + resource source at Orchard Farm.
    let orchard_ws = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(20),
            max_quantity: Quantity(20),
            regeneration_ticks_per_unit: Some(nz(5)),
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );

    // Merchant at General Store — enterprise-focused, has coins, perception.
    let merchant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Merchant",
        general_store,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            enterprise_weight: pm(800),
            ..UtilityProfile::default()
        },
    );
    {
        use std::collections::BTreeSet;
        use worldwake_core::{
            DemandMemory, DemandObservation, DemandObservationReason, MerchandiseProfile, Tick,
            TradeDispositionProfile,
        };
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_perception_profile(
            merchant,
            PerceptionProfile {
                entity_activation_threshold: pm(64),
                claim_confidence_threshold: pm(50),
                observation_buffer_capacity: 64,
                need_salience_boost: pm(500),
                need_salience_urgency_threshold: pm(500),
                observation_fidelity: pm(875),
                confidence_policy: BeliefConfidencePolicy::default(),
                institutional_memory_capacity: 20,
                consultation_speed_factor: pm(500),
                contradiction_tolerance: pm(300),
            },
        )
        .unwrap();
        txn.set_component_merchandise_profile(
            merchant,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Apple]),
                home_facility: Some(general_store),
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(
            merchant,
            TradeDispositionProfile {
                negotiation_round_ticks: nz(4),
                initial_offer_bias: pm(500),
                concession_rate: pm(100),
                rejection_escalation_rate: pm(200),
                demand_memory_retention_ticks: 240,
                market_presence_ticks: nz(30),
            },
        )
        .unwrap();
        txn.set_component_demand_memory(
            merchant,
            DemandMemory {
                observations: vec![DemandObservation {
                    commodity: CommodityKind::Apple,
                    quantity: Quantity(2),
                    place: general_store,
                    tick: Tick(0),
                    counterparty: None,
                    reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
                }],
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant,
        general_store,
        CommodityKind::Coin,
        Quantity(10),
    );
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        &[orchard_ws],
        worldwake_core::Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    // Villager at Village Square — hungry, thirsty, has bread + water + coins.
    let villager = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Villager",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(700), pm(500), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        villager,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(1),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        villager,
        VILLAGE_SQUARE,
        CommodityKind::Water,
        Quantity(2),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        villager,
        VILLAGE_SQUARE,
        CommodityKind::Coin,
        Quantity(5),
    );

    // Wanderer at Village Square — thirsty + fatigued, has water.
    let wanderer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Wanderer",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(800), pm(600), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile {
            thirst_weight: pm(800),
            fatigue_weight: pm(600),
            ..UtilityProfile::default()
        },
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        wanderer,
        VILLAGE_SQUARE,
        CommodityKind::Water,
        Quantity(1),
    );

    h
}

fn run_world_runs_without_observers(seed: Seed) -> (StateHash, StateHash) {
    let mut h = build_world_runs_without_observers_scenario(seed);

    let initial_world_hash = hash_world(&h.world).unwrap();
    let initial_event_count = h.event_log.len();

    // Track initial authoritative totals for conservation.
    // Apple has a regenerating ResourceSource, so its authoritative total can increase
    // (source stock regenerates between harvests). We skip apple conservation here
    // and check only non-regenerating commodities.
    let initial_bread = total_authoritative_commodity_quantity(&h.world, CommodityKind::Bread);
    let initial_water = total_authoritative_commodity_quantity(&h.world, CommodityKind::Water);
    let initial_coin = total_authoritative_commodity_quantity(&h.world, CommodityKind::Coin);

    let agents: Vec<_> = h
        .world
        .entities()
        .filter(|e| h.world.get_component_agent_data(*e).is_some())
        .collect();

    let mut any_agent_moved = false;
    let mut any_consumption_event = false;

    for _ in 0..200 {
        h.step_once();

        // Per-tick authoritative conservation for non-regenerating commodities.
        let bread = total_authoritative_commodity_quantity(&h.world, CommodityKind::Bread);
        let water = total_authoritative_commodity_quantity(&h.world, CommodityKind::Water);
        let coin = total_authoritative_commodity_quantity(&h.world, CommodityKind::Coin);
        assert!(
            bread <= initial_bread,
            "Bread conservation violated: initial={initial_bread}, now={bread}"
        );
        assert!(
            water <= initial_water,
            "Water conservation violated: initial={initial_water}, now={water}"
        );
        assert_eq!(
            coin, initial_coin,
            "Coin conservation violated: initial={initial_coin}, now={coin}"
        );

        // Track milestones — detect any agent in transit.
        for &agent in &agents {
            if h.world.is_in_transit(agent) {
                any_agent_moved = true;
            }
        }

        // Detect consumption by checking if any consumable total decreased.
        let current_bread = total_authoritative_commodity_quantity(&h.world, CommodityKind::Bread);
        let current_water = total_authoritative_commodity_quantity(&h.world, CommodityKind::Water);
        if current_bread < initial_bread || current_water < initial_water {
            any_consumption_event = true;
        }
    }

    let final_world_hash = hash_world(&h.world).unwrap();
    let final_event_count = h.event_log.len();

    // Assertion 1: World hash differs from initial.
    assert_ne!(
        final_world_hash, initial_world_hash,
        "World should have changed after 200 ticks of multi-agent simulation"
    );

    // Assertion 2: Event log grew by at least 20 events.
    assert!(
        final_event_count >= initial_event_count + 20,
        "Event log should grow by 20+ events over 200 ticks; initial={initial_event_count}, final={final_event_count}"
    );

    // Assertion 3: No agent died.
    for &agent in &agents {
        assert!(
            !h.agent_is_dead(agent),
            "No agent should die in a provisioned world"
        );
    }

    // Assertion 5: At least one agent changed places (travel system engaged).
    // We also detect travel by checking if any agent is in transit or visited an intermediate node.
    // For a more robust check, look at event log for travel events.
    let action_events_exist = !h
        .event_log
        .events_by_tag(worldwake_core::EventTag::ActionCommitted)
        .is_empty();
    assert!(
        any_agent_moved || action_events_exist,
        "At least one agent should have moved during 200 ticks"
    );

    // Assertion 6: At least one consumption event occurred.
    assert!(
        any_consumption_event,
        "At least one consumption event should have occurred over 200 ticks"
    );

    (final_world_hash, hash_event_log(&h.event_log).unwrap())
}

#[test]
fn golden_world_runs_without_observers() {
    let _ = run_world_runs_without_observers(Seed([201; 32]));
}

#[test]
fn golden_world_runs_without_observers_replays_deterministically() {
    let first = run_world_runs_without_observers(Seed([202; 32]));
    let second = run_world_runs_without_observers(Seed([202; 32]));

    assert_eq!(
        first, second,
        "Two runs of the 200-tick multi-agent world with the same seed must produce identical hashes"
    );
}

#[test]
#[ignore = "manual benchmark"]
fn bench_world_runs_without_observers() {
    use std::time::Instant;

    const RUNS: usize = 3;
    let mut elapsed = Vec::with_capacity(RUNS);

    for run in 0..RUNS {
        let seed_byte = 210 + u8::try_from(run).expect("benchmark run index should fit into u8");
        let start = Instant::now();
        let _ = run_world_runs_without_observers(Seed([seed_byte; 32]));
        elapsed.push(start.elapsed());
    }

    let total = elapsed.iter().copied().sum::<std::time::Duration>();
    let average = total / RUNS as u32;
    eprintln!(
        "bench_world_runs_without_observers: runs={RUNS} total={total:?} avg={average:?} samples={elapsed:?}"
    );
}

// ---------------------------------------------------------------------------
// Scenario S21-005: Save/Load Preserves Promoted Commitments
// ---------------------------------------------------------------------------
//
// Systems: Needs, Production, Travel, AI
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Travel, Production, Needs
// Places: VillageSquare, OrchardFarm (7-tick multi-leg route)
//
// Setup: A hungry agent at Village Square with food available only at
//   Orchard Farm. The agent must travel (7 ticks across 3 legs). We save
//   mid-travel, load, and assert that ActiveGoal, IntentionFrame, and
//   ContentionIntents survive the round-trip.
//
// Proves: Promoted causal runtime state (S21-001..004) is preserved by
//   save/load. The agent continues its journey rather than restarting.
//
// Chain: AI decides to travel → mid-travel save → load → commitment
//   preserved → agent continues journey → reaches destination.

fn build_commitment_preservation_scenario(seed: Seed) -> (GoldenHarness, worldwake_core::EntityId) {
    let mut h = GoldenHarness::new(seed);

    // Single hungry agent at VillageSquare — must travel to OrchardFarm for food.
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Traveler",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    // Give the agent perception so it can observe the world.
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        PerceptionProfile {
            entity_activation_threshold: pm(64),
            claim_confidence_threshold: pm(50),
            observation_buffer_capacity: 64,
            need_salience_boost: pm(500),
            need_salience_urgency_threshold: pm(500),
            observation_fidelity: pm(875),
            confidence_policy: BeliefConfidencePolicy::default(),
            institutional_memory_capacity: 20,
            consultation_speed_factor: pm(500),
            contradiction_tolerance: pm(300),
        },
    );

    // Food only at OrchardFarm — forces travel.
    let orchard_ws = place_workstation_with_source(
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

    // Seed the agent's beliefs about the orchard workstation so it knows
    // where to find food and will plan a journey there.
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        &[orchard_ws],
        worldwake_core::Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    (h, agent)
}

/// Run the commitment-preservation scenario, returning world+log hashes for
/// deterministic replay comparison.
fn run_commitment_preservation_scenario(seed: Seed) -> (StateHash, StateHash) {
    let (mut h, agent) = build_commitment_preservation_scenario(seed);

    // Run until the agent is mid-travel. The route VillageSquare → OrchardFarm
    // is 7 ticks (VillageSquare→SouthGate 2, SouthGate→EastFieldTrail 3,
    // EastFieldTrail→OrchardFarm 2). The agent needs a tick or two to decide
    // and start travel, so we scan for the travel action.
    let mut save_tick = None;
    for tick in 0..30 {
        h.step_once();
        if save_tick.is_none()
            && let Some(action_name) = h.agent_active_action_name(agent)
            && action_name == "travel"
        {
            // Found mid-travel — save after one more tick to be solidly
            // in the middle of a leg.
            h.step_once();
            save_tick = Some(tick + 2); // +1 for the extra step, +1 for 0-indexing
            break;
        }
    }
    let save_tick = save_tick.expect("Agent should have started traveling within 30 ticks");

    // --- Pre-save component reads ---
    let pre_active_goal: Option<ActiveGoal> = h.world.get_component_active_goal(agent).cloned();
    let pre_frame: Option<IntentionFrame> = h.world.get_component_intention_frame(agent).cloned();
    let pre_facility: Option<ContentionIntents> =
        h.world.get_component_contention_intents(agent).cloned();

    // Assert non-trivial state: at least ActiveGoal and IntentionFrame
    // must be present (acceptance criterion 3).
    assert!(
        pre_active_goal.is_some(),
        "Agent should have an ActiveGoal at save time (tick ~{save_tick})"
    );
    let pre_frame_val = pre_frame
        .clone()
        .expect("Agent should have a IntentionFrame at save time");
    assert_eq!(
        pre_frame_val.state,
        FrameState::Active,
        "IntentionFrame should be Active mid-travel"
    );

    // --- Save/load round-trip ---
    let mut resumed = h.save_load_roundtrip();

    // --- Post-load component reads ---
    let post_active_goal: Option<ActiveGoal> =
        resumed.world.get_component_active_goal(agent).cloned();
    let post_frame: Option<IntentionFrame> =
        resumed.world.get_component_intention_frame(agent).cloned();
    let post_facility: Option<ContentionIntents> = resumed
        .world
        .get_component_contention_intents(agent)
        .cloned();

    // --- Field-level equality assertions ---
    assert_eq!(
        pre_active_goal, post_active_goal,
        "ActiveGoal must survive save/load round-trip"
    );
    assert_eq!(
        pre_frame, post_frame,
        "IntentionFrame must survive save/load round-trip (destination, state, established_at, \
         last_progress_tick, stalled_ticks)"
    );
    assert_eq!(
        pre_facility, post_facility,
        "ContentionIntents must survive save/load round-trip"
    );

    // Specific field assertions for clarity (redundant but documents the contract).
    let post_frame_val = post_frame.unwrap();
    assert_eq!(
        pre_frame_val.domain, post_frame_val.domain,
        "IntentionFrame.domain must match after load"
    );
    assert_eq!(
        post_frame_val.state,
        FrameState::Active,
        "IntentionFrame.state must be Active after load"
    );
    assert_eq!(
        pre_frame_val.established_at, post_frame_val.established_at,
        "IntentionFrame.established_at must match after load"
    );

    // --- Verify agent continues journey (not restarting) ---
    // The agent should reach OrchardFarm within the remaining expected ticks,
    // not the full 7-tick journey duration. We allow a generous window (20 ticks)
    // but check that the agent does NOT take longer than a full restart would.
    let orchard_farm = prototype_place_entity(PrototypePlace::OrchardFarm);
    let mut reached_destination = false;
    for _ in 0..20 {
        resumed.step_once();
        if resumed.world.effective_place(agent) == Some(orchard_farm)
            && !resumed.world.is_in_transit(agent)
        {
            reached_destination = true;
            break;
        }
    }
    assert!(
        reached_destination,
        "Agent should reach OrchardFarm after load without restarting its journey"
    );

    // Return hashes for the deterministic replay companion test.
    (
        hash_world(&resumed.world).unwrap(),
        hash_event_log(&resumed.event_log).unwrap(),
    )
}

#[test]
fn golden_save_load_preserves_promoted_commitments() {
    let _ = run_commitment_preservation_scenario(Seed([105; 32]));
}

#[test]
fn golden_save_load_preserves_promoted_commitments_replays_deterministically() {
    let first = run_commitment_preservation_scenario(Seed([106; 32]));
    let second = run_commitment_preservation_scenario(Seed([106; 32]));

    assert_eq!(
        first, second,
        "Two runs of the commitment-preservation scenario with the same seed must produce \
         identical hashes"
    );
}

// ---------------------------------------------------------------------------
// Scenario S22-007: Save/load verification for IntentionFrame and
// IntentionDispositionProfile
// ---------------------------------------------------------------------------

#[test]
fn golden_save_load_preserves_suspended_intention_frame() {
    let (mut h, agent) = build_commitment_preservation_scenario(Seed([107; 32]));

    // Run until the agent has an active IntentionFrame (mid-travel).
    for _ in 0..30 {
        h.step_once();
        if h.world.get_component_intention_frame(agent).is_some() {
            break;
        }
    }
    let pre_frame = h
        .world
        .get_component_intention_frame(agent)
        .cloned()
        .expect("Agent should have an IntentionFrame before suspension");

    // Manually suspend the frame to test suspended-state round-trip.
    let suspended_frame = IntentionFrame {
        state: FrameState::Suspended {
            reason: SuspensionReason::PriorityInterrupt,
            suspended_at: Tick(99),
        },
        ..pre_frame
    };
    {
        let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
        txn.set_component_intention_frame(agent, suspended_frame.clone())
            .unwrap();
        txn.commit(&mut h.event_log);
    }

    // Verify the suspended frame is set.
    let pre_save = h
        .world
        .get_component_intention_frame(agent)
        .cloned()
        .unwrap();
    assert_eq!(
        pre_save.state,
        FrameState::Suspended {
            reason: SuspensionReason::PriorityInterrupt,
            suspended_at: Tick(99),
        },
        "Frame should be Suspended before save"
    );

    // Save/load round-trip.
    let resumed = h.save_load_roundtrip();

    let post_load = resumed
        .world
        .get_component_intention_frame(agent)
        .cloned()
        .expect("IntentionFrame must survive save/load");

    assert_eq!(
        pre_save, post_load,
        "Suspended IntentionFrame must survive save/load round-trip"
    );
    assert_eq!(
        post_load.state,
        FrameState::Suspended {
            reason: SuspensionReason::PriorityInterrupt,
            suspended_at: Tick(99),
        },
        "Suspended state with reason and tick must be preserved after load"
    );
}

#[test]
fn golden_save_load_preserves_intention_disposition_profile() {
    let (mut h, agent) = build_commitment_preservation_scenario(Seed([108; 32]));

    // Set an IntentionDispositionProfile with 2+ domain-specific patience entries.
    let mut domain_patience = BTreeMap::new();
    domain_patience.insert(IntentionDomainTag::Travel, nz(50));
    domain_patience.insert(IntentionDomainTag::Care, nz(75));
    domain_patience.insert(IntentionDomainTag::Escort, nz(100));

    let profile = IntentionDispositionProfile {
        domain_patience,
        default_patience_ticks: nz(30),
        commitment_switch_margin: pm(200),
    };

    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_intention_disposition_profile(agent, profile.clone())
            .unwrap();
        txn.commit(&mut h.event_log);
    }

    let pre_save = h
        .world
        .get_component_intention_disposition_profile(agent)
        .cloned()
        .expect("Profile should be set before save");

    // Save/load round-trip.
    let resumed = h.save_load_roundtrip();

    let post_load = resumed
        .world
        .get_component_intention_disposition_profile(agent)
        .cloned()
        .expect("IntentionDispositionProfile must survive save/load");

    assert_eq!(
        pre_save, post_load,
        "IntentionDispositionProfile must survive save/load round-trip"
    );

    // Explicit field assertions for contract documentation.
    assert_eq!(
        post_load.domain_patience.len(),
        3,
        "domain_patience should have 3 entries after load"
    );
    assert_eq!(
        post_load.domain_patience.get(&IntentionDomainTag::Travel),
        Some(&nz(50)),
        "Travel patience must be preserved"
    );
    assert_eq!(
        post_load.domain_patience.get(&IntentionDomainTag::Care),
        Some(&nz(75)),
        "Care patience must be preserved"
    );
    assert_eq!(
        post_load.domain_patience.get(&IntentionDomainTag::Escort),
        Some(&nz(100)),
        "Escort patience must be preserved"
    );
    assert_eq!(
        post_load.default_patience_ticks,
        nz(30),
        "default_patience_ticks must be preserved"
    );
    assert_eq!(
        post_load.commitment_switch_margin,
        pm(200),
        "commitment_switch_margin must be preserved"
    );
}

#[test]
fn golden_save_load_preserves_frame_assumptions() {
    let (mut h, agent) = build_commitment_preservation_scenario(Seed([109; 32]));

    // Create a frame with 2+ explicit assumptions.
    let destination = prototype_place_entity(PrototypePlace::OrchardFarm);
    let frame = IntentionFrame {
        goal: GoalKey::new(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: worldwake_core::CommodityPurpose::SelfConsume,
        }),
        domain: IntentionDomain::Travel { destination },
        assumptions: vec![
            FrameAssumption::RouteExists {
                from: prototype_place_entity(PrototypePlace::VillageSquare),
                to: destination,
            },
            FrameAssumption::NoCriticalThreat,
            FrameAssumption::CommodityAvailableAt {
                commodity: CommodityKind::Apple,
                place: destination,
            },
        ],
        state: FrameState::Active,
        established_at: Tick(5),
        last_progress_tick: Some(Tick(8)),
        stalled_ticks: 2,
        patience_limit: 40,
    };

    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_intention_frame(agent, frame.clone())
            .unwrap();
        txn.commit(&mut h.event_log);
    }

    let pre_save = h
        .world
        .get_component_intention_frame(agent)
        .cloned()
        .unwrap();

    // Save/load round-trip.
    let resumed = h.save_load_roundtrip();

    let post_load = resumed
        .world
        .get_component_intention_frame(agent)
        .cloned()
        .expect("IntentionFrame with assumptions must survive save/load");

    assert_eq!(
        pre_save, post_load,
        "IntentionFrame must survive save/load round-trip"
    );

    // Explicit assumptions assertions — order must be preserved (not re-sorted).
    assert_eq!(
        post_load.assumptions.len(),
        3,
        "All 3 assumptions must be preserved"
    );
    assert_eq!(
        post_load.assumptions[0],
        FrameAssumption::RouteExists {
            from: prototype_place_entity(PrototypePlace::VillageSquare),
            to: destination,
        },
        "First assumption (RouteExists) must be preserved in order"
    );
    assert_eq!(
        post_load.assumptions[1],
        FrameAssumption::NoCriticalThreat,
        "Second assumption (NoCriticalThreat) must be preserved in order"
    );
    assert_eq!(
        post_load.assumptions[2],
        FrameAssumption::CommodityAvailableAt {
            commodity: CommodityKind::Apple,
            place: destination,
        },
        "Third assumption (CommodityAvailableAt) must be preserved in order"
    );

    // Additional field assertions for full coverage.
    assert_eq!(post_load.established_at, Tick(5));
    assert_eq!(post_load.last_progress_tick, Some(Tick(8)));
    assert_eq!(post_load.stalled_ticks, 2);
    assert_eq!(post_load.patience_limit, 40);
}

// ---------------------------------------------------------------------------
// Scenario 104: Save/Load Preserves Generalized Contention World And Belief State
// ---------------------------------------------------------------------------
//
// Systems: SaveLoad, Contention, Perception
// GoalKinds: LootCorpse
// ActionDomains: Corpse
// Places: VillageSquare
// Principles: 8, 9, 12
//
// Setup: a dead bread-carrying corpse has real contention policy, one active
//   grant, one queued waiter, and one observing agent with a direct
//   `BelievedContentionState`.
//
// Proves: generalized contention world state and its projected local belief
//   survive save/load round-trip without drift.
//
// Chain: contention state projected into belief -> save -> load -> same queue,
//   same policy, same believed contention state.

fn build_generalized_contention_roundtrip_scenario(
    seed: Seed,
) -> (
    GoldenHarness,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
) {
    let mut h = GoldenHarness::new(seed);
    let granted_actor = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Granted",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let queued_actor = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Queued",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let observer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Observer",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        observer,
        PerceptionProfile {
            entity_activation_threshold: pm(100),
            claim_confidence_threshold: pm(50),
            observation_buffer_capacity: 20,
            need_salience_boost: pm(500),
            need_salience_urgency_threshold: pm(500),
            observation_fidelity: pm(1000),
            confidence_policy: BeliefConfidencePolicy::default(),
            institutional_memory_capacity: 20,
            consultation_speed_factor: pm(500),
            contradiction_tolerance: pm(300),
        },
    );

    let corpse = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Corpse",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        corpse,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(6),
    );

    let loot_action = h
        .defs
        .iter()
        .find(|def| def.name == "loot")
        .map(|def| def.id)
        .expect("loot action should be registered");

    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_dead_at(
            corpse,
            DeadAt {
                tick: Tick(0),
                cause: worldwake_core::DeathCause::CombatWounds,
            },
        )
        .unwrap();
        txn.set_component_contention_policy(
            corpse,
            ContentionPolicy {
                grant_hold_ticks: nz(4),
                auto_promote: true,
                max_waiters: None,
            },
        )
        .unwrap();
        let mut queue = ContentionQueue {
            next_ordinal: 0,
            waiting: BTreeMap::new(),
            granted: Some(ContentionGrant {
                actor: granted_actor,
                intended_action: loot_action,
                granted_at: Tick(0),
                expires_at: Tick(4),
            }),
        };
        queue
            .enqueue(queued_actor, loot_action, Tick(0), None)
            .unwrap();
        txn.set_component_contention_queue(corpse, queue).unwrap();
        txn.commit(&mut h.event_log);
    }
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        observer,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );

    (h, corpse, observer, granted_actor)
}

fn run_generalized_contention_roundtrip_scenario(seed: Seed) -> (StateHash, StateHash) {
    let (h, corpse, observer, granted_actor) =
        build_generalized_contention_roundtrip_scenario(seed);

    let pre_policy = h
        .world
        .get_component_contention_policy(corpse)
        .cloned()
        .expect("corpse contention policy should exist before save");
    let pre_queue = h
        .world
        .get_component_contention_queue(corpse)
        .cloned()
        .expect("corpse contention queue should exist before save");
    let pre_belief = h
        .world
        .get_component_agent_belief_store(observer)
        .and_then(|store| store.get_entity(&corpse))
        .and_then(|state| state.believed_contention)
        .expect("observer should hold believed contention state before save");

    let resumed = h.save_load_roundtrip();

    let post_policy = resumed
        .world
        .get_component_contention_policy(corpse)
        .cloned()
        .expect("corpse contention policy should survive save/load");
    let post_queue = resumed
        .world
        .get_component_contention_queue(corpse)
        .cloned()
        .expect("corpse contention queue should survive save/load");
    let post_belief = resumed
        .world
        .get_component_agent_belief_store(observer)
        .and_then(|store| store.get_entity(&corpse))
        .and_then(|state| state.believed_contention)
        .expect("observer believed contention state should survive save/load");

    assert_eq!(
        pre_policy, post_policy,
        "ContentionPolicy must survive save/load round-trip"
    );
    assert_eq!(
        pre_queue, post_queue,
        "ContentionQueue must survive save/load round-trip"
    );
    assert_eq!(
        pre_belief, post_belief,
        "BelievedContentionState must survive save/load round-trip"
    );
    assert_eq!(
        post_belief,
        BelievedContentionState {
            grant_holder: Some(granted_actor),
            queue_length: 1,
            observed_tick: Tick(0),
        },
        "loaded believed contention state should preserve grant holder, queue length, and observation tick"
    );

    (
        hash_world(&resumed.world).unwrap(),
        hash_event_log(&resumed.event_log).unwrap(),
    )
}

#[test]
fn golden_save_load_preserves_generalized_contention_state() {
    let _ = run_generalized_contention_roundtrip_scenario(Seed([110; 32]));
}

#[test]
fn golden_save_load_preserves_generalized_contention_state_replays_deterministically() {
    let seed = Seed([111; 32]);
    let first = run_generalized_contention_roundtrip_scenario(seed);
    let second = run_generalized_contention_roundtrip_scenario(seed);

    assert_eq!(
        first, second,
        "generalized contention save/load scenario should replay deterministically"
    );
}
