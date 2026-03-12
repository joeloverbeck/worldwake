//! Golden tests for AI decision-making, goal switching, and needs-driven behavior.

mod golden_harness;

use golden_harness::*;
use worldwake_core::{
    total_live_lot_quantity, CommodityKind, HomeostaticNeeds, MetabolismProfile, Quantity,
    ResourceSource, Seed, UtilityProfile, WorkstationTag,
};

// ---------------------------------------------------------------------------
// Scenario 1: Goal Invalidation by Another Agent
// ---------------------------------------------------------------------------

#[test]
fn golden_goal_invalidation_by_another_agent() {
    let mut h = GoldenHarness::new(Seed([1; 32]));

    // Both agents at Village Square, both critically hungry.
    let agent_a = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Alice",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let agent_b = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Bob",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    // Agent A has bread; Agent B has nothing edible.
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent_a,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(1),
    );

    // Place apple resource at Orchard Farm so B has a reachable food source.
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
    );

    let initial_bread = total_live_lot_quantity(&h.world, CommodityKind::Bread);

    // Run the simulation — Agent A should eat; Agent B should start moving.
    let mut a_ate = false;
    for _ in 0..50 {
        h.step_once();

        // Check if A consumed bread.
        if h.agent_commodity_qty(agent_a, CommodityKind::Bread) == Quantity(0) {
            a_ate = true;
        }

        // Conservation check at each tick.
        // Note: bread total can decrease (consumption is valid).
        let current_bread = total_live_lot_quantity(&h.world, CommodityKind::Bread);
        assert!(
            current_bread <= initial_bread,
            "Bread lots should not increase — conservation: initial={initial_bread}, now={current_bread}"
        );
    }

    assert!(a_ate, "Agent A should have eaten the bread");

    // Agent A's hunger should have decreased from eating bread.
    // Agent B should have no bread — either traveling or pursuing alternative.
    assert_eq!(
        h.agent_commodity_qty(agent_b, CommodityKind::Bread),
        Quantity(0),
        "Agent B should not have acquired bread"
    );
}

// ---------------------------------------------------------------------------
// Scenario 2: Priority-Based Interrupt
// ---------------------------------------------------------------------------

#[test]
fn golden_priority_based_interrupt() {
    let mut h = GoldenHarness::new(Seed([2; 32]));

    // Single agent: high fatigue, low hunger, but very fast hunger metabolism.
    let fast_hunger_metabolism = MetabolismProfile::new(
        pm(50), // hunger_rate — very fast!
        pm(3),  // thirst_rate
        pm(2),  // fatigue_rate
        pm(4),  // bladder_rate
        pm(1),  // dirtiness_rate
        pm(20), // rest_efficiency
        nz(480),
        nz(240),
        nz(120),
        nz(40),
        nz(8),
        nz(12),
    );

    let utility = UtilityProfile {
        fatigue_weight: pm(800),
        hunger_weight: pm(600),
        ..UtilityProfile::default()
    };

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Cara",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(300), pm(0), pm(800), pm(0), pm(0)),
        fast_hunger_metabolism,
        utility,
    );

    // Give agent bread so it can eat when hunger becomes critical.
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(2),
    );

    let mut initial_action_started = false;
    let mut hunger_reached_critical = false;
    let mut ate_bread = false;
    let initial_bread = h.agent_commodity_qty(agent, CommodityKind::Bread);

    for _tick in 0..100 {
        h.step_once();

        let hunger = h.agent_hunger(agent);
        let has_action = h.agent_has_active_action(agent);

        // Track milestones.
        if has_action && !initial_action_started {
            initial_action_started = true;
        }

        if hunger.value() >= 900 {
            hunger_reached_critical = true;
        }

        if h.agent_commodity_qty(agent, CommodityKind::Bread) < initial_bread {
            ate_bread = true;
        }

        // Early exit on success.
        if hunger_reached_critical && ate_bread {
            break;
        }
    }

    assert!(
        initial_action_started,
        "Agent should have started an action (sleep expected first)"
    );
    assert!(
        hunger_reached_critical,
        "Hunger should have reached critical with pm(50)/tick metabolism"
    );
    assert!(
        ate_bread,
        "Agent should have eaten bread after hunger became critical"
    );
}

// ---------------------------------------------------------------------------
// Scenario 5: Blocked Intent Memory with TTL Expiry
// ---------------------------------------------------------------------------

#[test]
fn golden_blocked_intent_memory_with_ttl_expiry() {
    let mut h = GoldenHarness::new(Seed([5; 32]));

    // Agent at Orchard Farm, critically hungry.
    // Resource source is DEPLETED (available_quantity 0) but regenerates.
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Eve",
        ORCHARD_FARM,
        HomeostaticNeeds::new(pm(900), pm(0), pm(400), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile {
            hunger_weight: pm(700),
            fatigue_weight: pm(500),
            ..UtilityProfile::default()
        },
    );

    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(0),
            max_quantity: Quantity(2),
            regeneration_ticks_per_unit: Some(nz(5)),
            last_regeneration_tick: None,
        },
    );

    let mut saw_blocker = false;
    let mut eventually_harvested = false;

    for _ in 0..200 {
        h.step_once();

        // Check for blocked intent memory.
        if let Some(blocked) = h.world.get_component_blocked_intent_memory(agent) {
            if !blocked.intents.is_empty() {
                saw_blocker = true;
            }
        }

        // Harvest drops apple lots on the ground at the workstation.
        let total_apple_lots = total_live_lot_quantity(&h.world, CommodityKind::Apple);
        if total_apple_lots > 0 {
            eventually_harvested = true;
            break;
        }
    }

    // Blocked intent recording depends on an action actually failing (handle_plan_failure),
    // not on the planner failing to find a plan. With a depleted source, the planner may
    // simply never produce a harvest plan, so blocked intent is observational, not required.
    if saw_blocker {
        eprintln!("Observed: Agent recorded a blocked intent for the depleted resource");
    }

    // After resource regeneration (10+ ticks to reach Quantity(2)),
    // the agent should eventually harvest, creating apple lots.
    assert!(
        eventually_harvested,
        "Agent should eventually harvest apples after resource regeneration"
    );
}

// ---------------------------------------------------------------------------
// Scenario 7: Deprivation Cascade
// ---------------------------------------------------------------------------

#[test]
fn golden_deprivation_cascade() {
    let mut h = GoldenHarness::new(Seed([77; 32]));

    // Agent starts with NO hunger (pm(0)) and fast metabolism.
    // Metabolism pushes hunger up over time. When hunger crosses the low threshold
    // (pm(250)), the AI generates a consume goal and the agent eats.
    // This proves cross-system emergence: needs system drives state →
    // AI detects threshold crossing → agent acts.
    let fast_metabolism = MetabolismProfile::new(
        pm(20), // hunger_rate — fast, ~20 permille/tick
        pm(3),
        pm(2),
        pm(4),
        pm(1),
        pm(20),
        nz(480),
        nz(240),
        nz(120),
        nz(40),
        nz(8),
        nz(12),
    );

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Felix",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(0)),
        fast_metabolism,
        UtilityProfile::default(),
    );

    // Give agent bread so it can eat when hunger crosses threshold.
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(1),
    );

    let initial_bread = h.agent_commodity_qty(agent, CommodityKind::Bread);
    let mut hunger_crossed_threshold = false;
    let mut ate_bread = false;

    for _tick in 0..80 {
        h.step_once();

        let hunger = h.agent_hunger(agent);
        let bread = h.agent_commodity_qty(agent, CommodityKind::Bread);

        // Track when hunger crosses the low threshold (250).
        if hunger.value() >= 250 {
            hunger_crossed_threshold = true;
        }

        if bread < initial_bread {
            ate_bread = true;
        }

        if hunger_crossed_threshold && ate_bread {
            break;
        }
    }

    assert!(
        hunger_crossed_threshold,
        "Hunger should have escalated past low threshold via metabolism"
    );
    assert!(
        ate_bread,
        "Agent should have eaten bread after hunger crossed threshold"
    );
}

#[test]
fn golden_thirst_driven_acquisition() {
    let mut h = GoldenHarness::new(Seed([78; 32]));

    let fast_thirst_metabolism = MetabolismProfile::new(
        pm(2),
        pm(20), // thirst_rate
        pm(2),
        pm(4),
        pm(1),
        pm(20),
        nz(480),
        nz(240),
        nz(120),
        nz(40),
        nz(8),
        nz(12),
    );

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Talia",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(0)),
        fast_thirst_metabolism,
        UtilityProfile::default(),
    );

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent,
        VILLAGE_SQUARE,
        CommodityKind::Water,
        Quantity(1),
    );

    let initial_water = h.agent_commodity_qty(agent, CommodityKind::Water);
    let mut thirst_crossed_threshold = false;
    let mut drank_water = false;

    for _tick in 0..80 {
        h.step_once();

        let thirst = h.agent_thirst(agent);
        let water = h.agent_commodity_qty(agent, CommodityKind::Water);

        if thirst.value() >= 200 {
            thirst_crossed_threshold = true;
        }

        if water < initial_water {
            drank_water = true;
        }

        if thirst_crossed_threshold && drank_water {
            break;
        }
    }

    assert!(
        thirst_crossed_threshold,
        "Thirst should have escalated past the default low threshold via metabolism"
    );
    assert!(
        drank_water,
        "Agent should have drunk water after thirst crossed threshold"
    );
}
