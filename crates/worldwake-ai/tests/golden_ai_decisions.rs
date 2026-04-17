//! Golden tests for AI decision-making, goal switching, and needs-driven behavior.

mod golden_harness;

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use golden_harness::*;
use worldwake_ai::{
    AgentDecisionRuntime, CommodityPurpose, DecisionOutcome, ExhaustionBaseline, ExhaustionEntry,
    ExhaustionInvalidationCondition, ExhaustionRetryState, GoalKey, GoalKind, OpportunityAnchor,
    OpportunityKey, PlanTerminalKind,
};
use worldwake_core::{
    BeliefConfidencePolicy, CognitiveProfile, CommodityKind, FrameState, HomeostaticNeeds,
    IntentionDispositionProfile, MetabolismProfile, PerceptionProfile, PrototypePlace, Quantity,
    ResourceSource, Seed, Tick, UtilityProfile, WorkstationTag, prototype_place_entity,
    total_live_lot_quantity,
};
use worldwake_sim::{ActionTraceKind, SaveableRuntime};

// ---------------------------------------------------------------------------
// Scenario 1: Goal Invalidation by Another Agent
// ---------------------------------------------------------------------------
//
// Systems: Needs, Production, Travel, AI
// GoalKinds: ConsumeOwnedCommodity, AcquireCommodity(SelfConsume)
// ActionDomains: Needs, Travel, Production
// Places: VillageSquare, GeneralStore
//
// Setup: Two critically hungry agents at Village Square. Alice has 1 bread.
//   Orchard Farm has apples.
//
// Proves: Alice eats bread (ConsumeOwnedCommodity). Bob travels to Orchard
//   Farm to harvest (AcquireCommodity with travel sub-plan). Conservation:
//   bread lots never increase.
//
// Chain: Needs pressure -> goal generation -> plan search -> action execution
//   -> resource consumption.

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
        ProductionOutputOwner::Actor,
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
// Scenario 1b: Unrelated Commodity Change Preserves Frontier Exhaustion
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, Production
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Needs, Production, Travel
// Places: VillageSquare, OrchardFarm
// Principles: 3, 19, 25
//
// Setup: Alice and Bob are critically hungry at Village Square. Alice has
//   bread. Bob's runtime is seeded with a frontier-exhausted
//   AcquireCommodity(Apple, SelfConsume) entry whose invalidation conditions
//   are position change and apple-quantity change. Orchard Farm still has
//   apples, and Bob knows about the world.
//
// Proves: Alice consuming bread does not clear Bob's unrelated frontier-
//   exhausted apple-acquisition entry. Bob therefore never starts a travel or
//   harvest action merely because another agent consumed bread nearby.
//
// Chain: unrelated bread consumption -> runtime dirty observation ->
//   goal-aware invalidation check -> frontier exhaustion preserved.

#[derive(Debug, Eq, PartialEq)]
struct FrontierExhaustionIsolationObservation {
    alice_ate: bool,
    bob_started_action: bool,
    bob_exhaustion_retained: bool,
}

#[derive(Serialize, Deserialize)]
struct DriverStateMirror {
    runtime_by_agent: std::collections::BTreeMap<worldwake_core::EntityId, AgentDecisionRuntime>,
}

fn run_unrelated_commodity_change_preserves_frontier_exhaustion(
    seed: Seed,
) -> FrontierExhaustionIsolationObservation {
    let mut h = GoldenHarness::new(seed);

    let alice = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Alice",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let bob = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Bob",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        bob,
        PerceptionProfile {
            entity_activation_threshold: pm(64),
            claim_confidence_threshold: pm(50),
            observation_buffer_capacity: 64,
            observation_budget: 24,
            need_salience_boost: pm(500),
            need_salience_urgency_threshold: pm(500),
            observation_fidelity: pm(875),
            confidence_policy: BeliefConfidencePolicy::default(),
            institutional_memory_capacity: 20,
            consultation_speed_factor: pm(500),
            contradiction_tolerance: pm(300),
        },
    );

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        alice,
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

    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        bob,
        worldwake_core::Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let apple_goal = GoalKey::from(GoalKind::AcquireCommodity {
        commodity: CommodityKind::Apple,
        purpose: CommodityPurpose::SelfConsume,
    });
    let apple_opportunity = OpportunityKey {
        goal_key: apple_goal,
        anchor: OpportunityAnchor::Place(ORCHARD_FARM),
    };
    let mut runtime = AgentDecisionRuntime::default();
    runtime.exhaustion_cache.insert(
        apple_opportunity,
        ExhaustionEntry {
            retry_state: ExhaustionRetryState::FrontierExhausted,
            invalidation_conditions: vec![
                ExhaustionInvalidationCondition::PositionChanged,
                ExhaustionInvalidationCondition::CommodityChanged(CommodityKind::Apple),
            ],
            baseline: ExhaustionBaseline {
                position: Some(VILLAGE_SQUARE),
                commodity_quantities: vec![(CommodityKind::Apple, Quantity(0))],
                ..ExhaustionBaseline::default()
            },
            next_retry_tick: None,
            consecutive_failures: 0,
        },
    );
    let runtime_bytes = bincode::serialize(&DriverStateMirror {
        runtime_by_agent: std::collections::BTreeMap::from([(bob, runtime)]),
    })
    .expect("golden scenario should serialize seeded runtime");
    h.driver = worldwake_ai::AgentTickDriver::from_saved_runtime(&runtime_bytes, &h.world)
        .expect("golden scenario should restore seeded runtime");

    let mut alice_ate = false;
    let mut bob_started_action = false;
    for _ in 0..20 {
        h.step_once();
        alice_ate |= h.agent_commodity_qty(alice, CommodityKind::Bread) == Quantity(0);
        bob_started_action |= h.agent_has_active_action(bob);
    }

    let restored_runtime: DriverStateMirror = bincode::deserialize(
        &h.driver
            .save_runtime_state()
            .expect("runtime should serialize"),
    )
    .expect("runtime mirror should deserialize");
    let bob_exhaustion_retained = restored_runtime
        .runtime_by_agent
        .get(&bob)
        .is_some_and(|runtime| runtime.exhaustion_cache.contains_key(&apple_opportunity));

    FrontierExhaustionIsolationObservation {
        alice_ate,
        bob_started_action,
        bob_exhaustion_retained,
    }
}

#[test]
fn golden_unrelated_commodity_change_preserves_frontier_exhaustion() {
    let observation = run_unrelated_commodity_change_preserves_frontier_exhaustion(Seed([201; 32]));

    assert!(
        observation.alice_ate,
        "Alice should consume her local bread so the scenario exercises an unrelated commodity change"
    );
    assert!(
        observation.bob_exhaustion_retained,
        "Bob's frontier-exhausted apple acquisition entry should remain cached after Alice consumes bread"
    );
    assert!(
        !observation.bob_started_action,
        "Bob should not start travel or harvest solely because another agent consumed bread"
    );
}

// ---------------------------------------------------------------------------
// Scenario 2: Priority-Based Interrupt
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI
// GoalKinds: ConsumeOwnedCommodity, Sleep
// ActionDomains: Needs
// Places: VillageSquare
// Principles: none specific
//
// Setup: Agent with high fatigue (pm(800)), low hunger (pm(300)), fast hunger
//   metabolism (pm(50)/tick). Has 2 bread.
//
// Proves: Agent starts sleeping. Metabolism drives hunger past critical
//   (pm(900)). AI interrupts sleep and switches to eating.
//
// Chain: Metabolism tick -> need escalation -> interrupt evaluation -> goal
//   switch -> action termination -> new action start.

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
        pm(0),
        pm(0),
        pm(0),
        pm(0),
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
//
// Systems: Production, AI
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Production
// Places: OrchardFarm
//
// Setup: Agent at Orchard Farm, critically hungry. ResourceSource depleted but
//   regenerates at 1/5 ticks.
//
// Proves: Depleted source blocks harvest. Resource regeneration restores
//   apples over time. Agent eventually harvests.
//
// Chain: Depleted resource -> failed plan -> resource regeneration ticks
//   -> successful harvest.

#[test]
fn golden_blocked_intent_memory_with_ttl_expiry() {
    let mut h = GoldenHarness::new(Seed([5; 32]));

    // Agent at Orchard Farm, critically hungry.
    // Resource source is DEPLETED (available_quantity 0) but regenerates.
    // Seed local beliefs explicitly so the scenario stays about regeneration
    // and blocker clearing rather than co-location discovery timing.
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
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        PerceptionProfile {
            entity_activation_threshold: pm(125),
            claim_confidence_threshold: pm(50),
            observation_buffer_capacity: 64,
            observation_budget: 24,
            need_salience_boost: pm(500),
            need_salience_urgency_threshold: pm(500),
            observation_fidelity: pm(1000),
            confidence_policy: BeliefConfidencePolicy::default(),
            institutional_memory_capacity: 20,
            consultation_speed_factor: pm(500),
            contradiction_tolerance: pm(300),
        },
    );

    let orchard_source = place_workstation_with_source(
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
        ProductionOutputOwner::Actor,
    );
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );

    let mut saw_blocker = false;
    let mut eventually_harvested = false;

    for _ in 0..200 {
        h.step_once();

        // Check for blocked intent memory.
        if let Some(blocked) = h.world.get_component_blocked_intent_memory(agent)
            && !blocked.intents.is_empty()
        {
            saw_blocker = true;
        }

        // Harvest drops apple lots on the ground at the workstation.
        let total_apple_lots = total_live_lot_quantity(&h.world, CommodityKind::Apple);
        if total_apple_lots > 0 {
            eventually_harvested = true;
            break;
        }
    }

    assert!(
        saw_blocker,
        "Agent should record a blocked intent when the depleted orchard source blocks harvest"
    );

    // After resource regeneration (10+ ticks to reach Quantity(2)),
    // the agent should eventually harvest, creating apple lots.
    assert!(
        eventually_harvested,
        "Agent should eventually harvest apples after resource regeneration; blocked={:?}; authoritative_source={:?}; believed_source={:?}",
        h.world.get_component_blocked_intent_memory(agent),
        h.world.get_component_resource_source(orchard_source),
        h.world
            .get_component_agent_belief_store(agent)
            .and_then(|store| store.get_entity(&orchard_source))
    );
}

// ---------------------------------------------------------------------------
// Scenario 7: Deprivation Cascade
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI
// GoalKinds: ConsumeOwnedCommodity
// ActionDomains: Needs
// Places: VillageSquare
//
// Setup: Agent starts pm(0) hunger, fast metabolism (pm(20)/tick), has 1 bread.
//
// Proves: Metabolism pushes hunger upward. When crossing low threshold
//   (pm(250)), AI generates consume goal. Agent eats.
//
// Chain: Metabolism system -> state change -> AI threshold detection -> goal
//   generation -> plan -> action.

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
        pm(0),
        pm(0),
        pm(0),
        pm(0),
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
        pm(0),
        pm(0),
        pm(0),
        pm(0),
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

#[test]
fn golden_wash_action() {
    let mut h = GoldenHarness::new(Seed([79; 32]));

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Sana",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(800)),
        MetabolismProfile::default(),
        UtilityProfile {
            dirtiness_weight: pm(900),
            ..UtilityProfile::default()
        },
    );

    let _wash_basin = place_workstation(
        &mut h.world,
        &mut h.event_log,
        VILLAGE_SQUARE,
        WorkstationTag::WashBasin,
        ProductionOutputOwner::Actor,
    );
    let water_source = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        VILLAGE_SQUARE,
        WorkstationTag::Well,
        ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(2),
            max_quantity: Quantity(2),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );

    let initial_dirtiness = h.agent_dirtiness(agent);
    let initial_source = h
        .world
        .get_component_resource_source(water_source)
        .expect("wash scenario should have a local water source")
        .available_quantity;
    let mut washed = false;

    for _tick in 0..80 {
        h.step_once();

        let current_source = h
            .world
            .get_component_resource_source(water_source)
            .expect("wash scenario should retain its local water source")
            .available_quantity;

        assert!(
            current_source <= initial_source,
            "Local wash water should not increase during washing: initial={initial_source}, now={current_source}"
        );

        if current_source < initial_source && h.agent_dirtiness(agent) < initial_dirtiness {
            washed = true;
            break;
        }
    }

    assert!(
        washed,
        "Agent should wash when dirtiness is high and a local basin plus water source are available; initial_dirtiness={initial_dirtiness}, final_dirtiness={}, initial_source={initial_source}, final_source={}",
        h.agent_dirtiness(agent),
        h.world
            .get_component_resource_source(water_source)
            .expect("wash scenario should retain its local water source")
            .available_quantity
    );
}

#[test]
fn golden_three_way_need_competition() {
    let mut scenario = TripleNeedScenario::new();

    let milestones = scenario.run();

    assert_eq!(
        milestones
            .first_action_name
            .expect("Agent should start a local self-care action under triple pressure"),
        "eat",
        "The first started self-care action should follow the highest-weight hunger path"
    );

    let bread_tick = milestones
        .bread_tick
        .expect("Agent should consume bread under simultaneous hunger pressure");
    let water_tick = milestones
        .water_tick
        .expect("Agent should consume water during the same local scenario");
    let fatigue_relief_tick = milestones
        .fatigue_relief_tick
        .expect("Agent should eventually reduce fatigue after hunger and thirst are addressed");

    assert!(
        fatigue_relief_tick > water_tick,
        "Fatigue relief should occur after thirst has been handled in the local scenario; bread_tick={bread_tick}, water_tick={water_tick}, fatigue_relief_tick={fatigue_relief_tick}"
    );
}

struct TripleNeedScenario {
    harness: GoldenHarness,
    agent: worldwake_core::EntityId,
    initial_bread: Quantity,
    initial_water: Quantity,
    initial_fatigue: worldwake_core::Permille,
    initial_bread_total: u64,
    initial_water_total: u64,
}

#[derive(Default)]
struct TripleNeedMilestones {
    first_action_name: Option<String>,
    bread_tick: Option<u32>,
    water_tick: Option<u32>,
    fatigue_relief_tick: Option<u32>,
}

impl TripleNeedMilestones {
    fn is_complete(&self) -> bool {
        self.first_action_name.is_some()
            && self.bread_tick.is_some()
            && self.water_tick.is_some()
            && self.fatigue_relief_tick.is_some()
    }
}

impl TripleNeedScenario {
    fn new() -> Self {
        let mut harness = GoldenHarness::new(Seed([81; 32]));

        let agent = seed_agent(
            &mut harness.world,
            &mut harness.event_log,
            "Nia",
            VILLAGE_SQUARE,
            HomeostaticNeeds::new(pm(900), pm(900), pm(320), pm(0), pm(0)),
            MetabolismProfile::default(),
            UtilityProfile {
                hunger_weight: pm(800),
                thirst_weight: pm(600),
                fatigue_weight: pm(400),
                ..UtilityProfile::default()
            },
        );

        give_commodity(
            &mut harness.world,
            &mut harness.event_log,
            agent,
            VILLAGE_SQUARE,
            CommodityKind::Bread,
            Quantity(2),
        );
        give_commodity(
            &mut harness.world,
            &mut harness.event_log,
            agent,
            VILLAGE_SQUARE,
            CommodityKind::Water,
            Quantity(2),
        );

        let initial_bread = harness.agent_commodity_qty(agent, CommodityKind::Bread);
        let initial_water = harness.agent_commodity_qty(agent, CommodityKind::Water);
        let initial_fatigue = harness
            .world
            .get_component_homeostatic_needs(agent)
            .map(|needs| needs.fatigue)
            .expect("seeded agent should have homeostatic needs");
        let initial_bread_total = total_live_lot_quantity(&harness.world, CommodityKind::Bread);
        let initial_water_total = total_live_lot_quantity(&harness.world, CommodityKind::Water);

        Self {
            harness,
            agent,
            initial_bread,
            initial_water,
            initial_fatigue,
            initial_bread_total,
            initial_water_total,
        }
    }

    fn run(&mut self) -> TripleNeedMilestones {
        let mut milestones = TripleNeedMilestones::default();

        for tick in 0..200 {
            self.harness.step_once();
            self.capture_first_action(&mut milestones);
            self.assert_local_supply_conservation();
            self.capture_consumption_ticks(tick, &mut milestones);
            self.capture_fatigue_relief_tick(tick, &mut milestones);

            if milestones.is_complete() {
                break;
            }
        }

        milestones
    }

    fn capture_first_action(&self, milestones: &mut TripleNeedMilestones) {
        if milestones.first_action_name.is_none() {
            milestones.first_action_name = self
                .harness
                .scheduler
                .active_actions()
                .values()
                .find(|instance| instance.actor == self.agent)
                .and_then(|instance| self.harness.defs.get(instance.def_id))
                .map(|def| def.name.clone());
        }
    }

    fn assert_local_supply_conservation(&self) {
        let current_bread_total =
            total_live_lot_quantity(&self.harness.world, CommodityKind::Bread);
        let current_water_total =
            total_live_lot_quantity(&self.harness.world, CommodityKind::Water);

        assert!(
            current_bread_total <= self.initial_bread_total,
            "Bread lots should not increase during multi-need competition: initial={}, now={current_bread_total}",
            self.initial_bread_total
        );
        assert!(
            current_water_total <= self.initial_water_total,
            "Water lots should not increase during multi-need competition: initial={}, now={current_water_total}",
            self.initial_water_total
        );
    }

    fn capture_consumption_ticks(&self, tick: u32, milestones: &mut TripleNeedMilestones) {
        if milestones.bread_tick.is_none()
            && self
                .harness
                .agent_commodity_qty(self.agent, CommodityKind::Bread)
                < self.initial_bread
        {
            milestones.bread_tick = Some(tick);
        }

        if milestones.water_tick.is_none()
            && self
                .harness
                .agent_commodity_qty(self.agent, CommodityKind::Water)
                < self.initial_water
        {
            milestones.water_tick = Some(tick);
        }
    }

    fn capture_fatigue_relief_tick(&self, tick: u32, milestones: &mut TripleNeedMilestones) {
        let fatigue = self
            .harness
            .world
            .get_component_homeostatic_needs(self.agent)
            .map(|needs| needs.fatigue)
            .expect("seeded agent should retain homeostatic needs");

        if milestones.bread_tick.is_some()
            && milestones.water_tick.is_some()
            && milestones.fatigue_relief_tick.is_none()
            && fatigue < self.initial_fatigue
        {
            milestones.fatigue_relief_tick = Some(tick);
        }
    }
}

#[allow(clippy::too_many_lines)]
#[test]
fn golden_bladder_relief_with_travel() {
    let mut h = GoldenHarness::new(Seed([80; 32]));

    let fast_bladder_metabolism = MetabolismProfile::new(
        pm(2),
        pm(2),
        pm(2),
        pm(10), // bladder_rate
        pm(1),
        pm(20),
        nz(480),
        nz(240),
        nz(120),
        nz(200),
        nz(8),
        nz(12),
        pm(0),
        pm(0),
        pm(0),
        pm(0),
    );

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Mira",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(650), pm(0)),
        fast_bladder_metabolism,
        UtilityProfile {
            bladder_weight: pm(900),
            ..UtilityProfile::default()
        },
    );
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        &[PUBLIC_LATRINE],
        worldwake_core::Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let initial_bladder = h.agent_bladder(agent);
    let mut reached_latrine = false;
    let mut relieved_at_latrine = false;
    let mut waste_appeared_at_latrine = false;
    let mut waste_appeared_at_origin = false;
    let mut final_place = h.world.effective_place(agent);
    let mut visited_places = vec![VILLAGE_SQUARE];

    for _ in 0..100 {
        h.step_once();

        if let Some(place) = h.world.effective_place(agent) {
            final_place = Some(place);
            if !visited_places.contains(&place) {
                visited_places.push(place);
            }
            if place == PUBLIC_LATRINE {
                reached_latrine = true;
            }
        }

        waste_appeared_at_latrine = h
            .world
            .entities_effectively_at(PUBLIC_LATRINE)
            .into_iter()
            .any(|entity| {
                h.world
                    .get_component_item_lot(entity)
                    .is_some_and(|lot| lot.commodity == CommodityKind::Waste)
            });
        waste_appeared_at_origin = h
            .world
            .entities_effectively_at(VILLAGE_SQUARE)
            .into_iter()
            .any(|entity| {
                h.world
                    .get_component_item_lot(entity)
                    .is_some_and(|lot| lot.commodity == CommodityKind::Waste)
            });
        let dirtiness = h
            .world
            .get_component_homeostatic_needs(agent)
            .map_or(pm(0), |needs| needs.dirtiness);

        if reached_latrine
            && waste_appeared_at_latrine
            && h.agent_bladder(agent) < initial_bladder
            && dirtiness < pm(200)
        {
            relieved_at_latrine = true;
            break;
        }
    }

    assert!(
        visited_places.iter().any(|place| *place != VILLAGE_SQUARE),
        "Agent should leave Village Square to satisfy relief at a latrine; visited={visited_places:?}, final_place={final_place:?}"
    );
    assert!(
        reached_latrine,
        "Agent should reach the public latrine before relieving; visited={visited_places:?}, final_place={final_place:?}"
    );
    assert!(
        relieved_at_latrine,
        "Agent should complete relief at the latrine without taking the accident path; initial={initial_bladder}, final={}, visited={visited_places:?}, final_place={final_place:?}, waste_origin={waste_appeared_at_origin}, waste_latrine={waste_appeared_at_latrine}, blocked={:?}, active_actions={:?}",
        h.agent_bladder(agent),
        h.world.get_component_blocked_intent_memory(agent),
        h.scheduler
            .active_actions()
            .values()
            .map(|instance| {
                h.defs.get(instance.def_id).map_or_else(
                    || format!("def-{}", instance.def_id.0),
                    |def| def.name.clone(),
                )
            })
            .collect::<Vec<_>>()
    );
    assert!(
        waste_appeared_at_latrine,
        "Relief should materialize waste at the latrine rather than at the origin; visited={visited_places:?}, final_place={final_place:?}"
    );
}

#[allow(clippy::too_many_lines)]
#[test]
fn golden_goal_switching_during_multi_leg_travel() {
    let mut h = GoldenHarness::new(Seed([81; 32]));
    let bandit_camp = prototype_place_entity(PrototypePlace::BanditCamp);

    let thirst_escalates_after_first_leg = MetabolismProfile::new(
        pm(2),   // hunger_rate
        pm(180), // thirst_rate — rises through medium/high before crossing critical
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
        pm(0),
        pm(0),
        pm(0),
        pm(0),
    );

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Vale",
        bandit_camp,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        thirst_escalates_after_first_leg,
        UtilityProfile {
            hunger_weight: pm(500),
            thirst_weight: pm(1000),
            ..UtilityProfile::default()
        },
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_intention_disposition_profile(
            agent,
            IntentionDispositionProfile {
                commitment_switch_margin: pm(0),
                domain_patience: std::collections::BTreeMap::new(),
                default_patience_ticks: nz(4),
            },
        )
        .unwrap();
        txn.set_component_cognitive_profile(
            agent,
            CognitiveProfile {
                planning_switch_margin: pm(0),
                ..CognitiveProfile::default()
            },
        )
        .unwrap();
        txn.set_component_perception_profile(
            agent,
            PerceptionProfile {
                entity_activation_threshold: pm(125),
                claim_confidence_threshold: pm(50),
                observation_buffer_capacity: 64,
                observation_budget: 24,
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
        commit_txn(txn, &mut h.event_log);
    }

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent,
        bandit_camp,
        CommodityKind::Water,
        Quantity(1),
    );

    let _orchard_source = place_workstation_with_source(
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
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        worldwake_core::Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let initial_water_total = total_live_lot_quantity(&h.world, CommodityKind::Water);
    let initial_water = h.agent_commodity_qty(agent, CommodityKind::Water);
    let initial_hunger = h.agent_hunger(agent);
    let thirst_thresholds = h
        .world
        .get_component_drive_thresholds(agent)
        .expect("seeded agent should have drive thresholds")
        .thirst;
    let mut left_bandit_camp = false;
    let mut saw_travel_with_medium_thirst = false;
    let mut saw_travel_with_high_thirst = false;
    let mut thirst_reached_critical_before_drink = false;
    let mut drank_before_critical = false;
    let mut drink_place = None;
    let mut resumed_to_orchard_after_drink = false;
    let mut hunger_relieved_after_drink = false;
    let mut saw_active_commitment_to_orchard = false;
    let mut saw_suspended_commitment_to_orchard = false;
    let mut saw_reactivated_commitment_to_orchard = false;
    let mut saw_progress_tick_recorded = false;
    let mut visited_places = vec![bandit_camp];

    for _ in 0..150 {
        h.step_once();

        let thirst = h.agent_thirst(agent);
        let active_action_name = h
            .scheduler
            .active_actions()
            .values()
            .find(|instance| instance.actor == agent)
            .and_then(|instance| h.defs.get(instance.def_id))
            .map(|def| def.name.as_str());
        let frame_snapshot = h
            .driver
            .frame_snapshot(&h.world, agent)
            .expect("golden harness should retain runtime state for the seeded AI agent");

        if frame_snapshot.runtime.committed_destination == Some(ORCHARD_FARM) {
            if frame_snapshot.runtime.frame_state == Some(FrameState::Active) {
                saw_active_commitment_to_orchard = true;
                if saw_suspended_commitment_to_orchard {
                    saw_reactivated_commitment_to_orchard = true;
                }
            }
            if matches!(
                frame_snapshot.runtime.frame_state,
                Some(FrameState::Suspended { .. })
            ) {
                saw_suspended_commitment_to_orchard = true;
            }
        }
        saw_progress_tick_recorded |= frame_snapshot.runtime.last_progress_tick.is_some();

        if h.world.is_in_transit(agent) || h.world.effective_place(agent) != Some(bandit_camp) {
            left_bandit_camp = true;
        }

        if h.agent_commodity_qty(agent, CommodityKind::Water) == initial_water {
            if thirst >= thirst_thresholds.medium() && thirst < thirst_thresholds.critical() {
                saw_travel_with_medium_thirst |= active_action_name == Some("travel");
            }
            if thirst >= thirst_thresholds.high() && thirst < thirst_thresholds.critical() {
                saw_travel_with_high_thirst |= active_action_name == Some("travel");
            }
            if thirst >= thirst_thresholds.critical() {
                thirst_reached_critical_before_drink = true;
            }
        } else if !thirst_reached_critical_before_drink {
            drank_before_critical = true;
        }

        if let Some(place) = h.world.effective_place(agent) {
            if !visited_places.contains(&place) {
                visited_places.push(place);
            }
            if h.agent_commodity_qty(agent, CommodityKind::Water) < initial_water {
                drink_place = Some(place);
            }
            if drink_place.is_some() && place == ORCHARD_FARM {
                resumed_to_orchard_after_drink = true;
            }
        }

        if drink_place.is_some() && h.agent_hunger(agent) < initial_hunger {
            hunger_relieved_after_drink = true;
            break;
        }

        let current_water_total = total_live_lot_quantity(&h.world, CommodityKind::Water);
        assert!(
            current_water_total <= initial_water_total,
            "Water lots should not increase — conservation: initial={initial_water_total}, now={current_water_total}"
        );
    }

    assert!(
        left_bandit_camp,
        "Agent should begin the distant food journey from Bandit Camp"
    );
    assert!(
        saw_active_commitment_to_orchard,
        "The runtime should establish an active journey commitment to Orchard Farm before the detour"
    );
    assert!(
        saw_progress_tick_recorded,
        "The runtime should record journey progress after completing at least one travel leg"
    );
    assert!(
        saw_travel_with_medium_thirst,
        "The penalty-interruptible travel action should continue after thirst reaches the medium band"
    );
    assert!(
        saw_travel_with_high_thirst,
        "The penalty-interruptible travel action should continue after thirst reaches the high band"
    );
    assert!(
        !drank_before_critical,
        "The agent should not interrupt penalty travel for a subcritical thirst challenger"
    );
    assert!(
        thirst_reached_critical_before_drink,
        "The water detour should only happen after thirst reaches the critical band"
    );
    assert!(
        saw_suspended_commitment_to_orchard,
        "The runtime should preserve but suspend the Orchard Farm commitment while the local thirst detour is active"
    );
    let drink_place = drink_place
        .expect("Agent should consume carried water after departing on the food journey");
    assert_ne!(
        drink_place, bandit_camp,
        "Water-driven goal switch should occur after departure, not at the origin"
    );
    assert!(
        resumed_to_orchard_after_drink || drink_place == ORCHARD_FARM,
        "Agent should either resume the original food journey after drinking or already be at the destination when the local thirst detour resolves; visited={visited_places:?}, drink_place={drink_place:?}"
    );
    assert!(
        saw_reactivated_commitment_to_orchard || drink_place == ORCHARD_FARM,
        "The runtime should reactivate the original Orchard Farm commitment after the detour, unless the detour resolves at Orchard Farm itself; visited={visited_places:?}, drink_place={drink_place:?}"
    );
    assert!(
        hunger_relieved_after_drink,
        "Agent should complete the resumed food journey after the detour; visited={visited_places:?}, drink_place={drink_place:?}"
    );
}

// ---------------------------------------------------------------------------
// Scenario S02b: Utility Weight Diversity in Need Selection (Principle 20)
// ---------------------------------------------------------------------------
//
// Systems: Needs, Enterprise, AI, Travel, Production
// GoalKinds: ConsumeOwnedCommodity, RestockCommodity
// ActionDomains: Needs, Travel, Production
// Places: VillageSquare, OrchardFarm
// Principles: 20
//
// Setup: Two agents with identical initial conditions but divergent
//   UtilityProfile weights. HungerDriven: critically hungry,
//   hunger_weight=pm(800), enterprise_weight=pm(100), has bread.
//   EnterpriseDriven: no hunger, hunger_weight=pm(100),
//   enterprise_weight=pm(900), MerchandiseProfile(Apple).
//
// Proves: HungerDriven eats locally. EnterpriseDriven leaves to pursue
//   enterprise restock. Same world, different first choices from
//   UtilityProfile weight divergence (Principle 20).
//
// Chain: UtilityProfile weights -> divergent candidate ranking ->
//   HungerDriven selects ConsumeOwnedCommodity vs EnterpriseDriven
//   selects RestockCommodity.

#[test]
#[allow(clippy::too_many_lines)]
fn golden_utility_weight_diversity_in_need_selection() {
    let mut h = GoldenHarness::new(Seed([200; 32]));

    // Two agents at Village Square with identical critical hunger, but
    // divergent UtilityProfile weights. Only one agent has food locally;
    // the other has no food and must travel to Orchard Farm.
    //
    // HungerDriven has hunger_weight=1000 and bread → eats locally.
    // EnterpriseDriven has enterprise_weight=900 and a restock signal →
    //   pursues enterprise restocking rather than hunger relief,
    //   because enterprise motive outweighs the hunger-driven goal.
    //
    // This proves Principle 20: "two agents with the same role should
    // sometimes choose differently" — different UtilityProfile weights
    // produce divergent goal selection under identical environmental
    // conditions.
    let hunger_driven = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "HungerDriven",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile {
            hunger_weight: pm(800),
            enterprise_weight: pm(100),
            ..UtilityProfile::default()
        },
    );

    let enterprise_driven = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "EnterpriseDriven",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile {
            hunger_weight: pm(100),
            enterprise_weight: pm(900),
            ..UtilityProfile::default()
        },
    );

    // HungerDriven has bread to eat locally.
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        hunger_driven,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(2),
    );

    // EnterpriseDriven is a merchant with restock signal.
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
    {
        use worldwake_core::{
            DemandMemory, DemandObservation, DemandObservationReason, MerchandiseProfile, Tick,
            TradeDispositionProfile,
        };

        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_perception_profile(
            enterprise_driven,
            PerceptionProfile {
                entity_activation_threshold: pm(64),
                claim_confidence_threshold: pm(50),
                observation_buffer_capacity: 64,
                observation_budget: 24,
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
            enterprise_driven,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Apple]),
                home_facility: Some(VILLAGE_SQUARE),
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(
            enterprise_driven,
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
            enterprise_driven,
            DemandMemory {
                observations: vec![DemandObservation {
                    commodity: CommodityKind::Apple,
                    quantity: Quantity(2),
                    place: VILLAGE_SQUARE,
                    tick: Tick(0),
                    counterparty: None,
                    reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
                }],
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        enterprise_driven,
        &[orchard_ws],
        worldwake_core::Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let initial_hunger = h.agent_hunger(hunger_driven);
    let initial_bread = h.agent_commodity_qty(hunger_driven, CommodityKind::Bread);

    let mut hunger_driven_ate = false;
    let mut enterprise_driven_left = false;

    for _ in 0..50 {
        h.step_once();

        // Track HungerDriven eating locally.
        if h.agent_commodity_qty(hunger_driven, CommodityKind::Bread) < initial_bread {
            hunger_driven_ate = true;
        }

        // Track EnterpriseDriven leaving for restock.
        if h.world.is_in_transit(enterprise_driven)
            || h.world.effective_place(enterprise_driven) != Some(VILLAGE_SQUARE)
        {
            enterprise_driven_left = true;
        }

        if hunger_driven_ate && enterprise_driven_left {
            break;
        }
    }

    // HungerDriven (hunger_weight=800) should eat locally.
    assert!(
        hunger_driven_ate,
        "HungerDriven should eat bread locally under hunger pressure"
    );
    assert!(
        h.agent_hunger(hunger_driven) < initial_hunger,
        "HungerDriven's hunger should decrease after eating"
    );

    // EnterpriseDriven (enterprise_weight=900) should pursue restock travel.
    assert!(
        enterprise_driven_left,
        "EnterpriseDriven should leave Village Square to pursue enterprise restock goal"
    );
}

// ---------------------------------------------------------------------------
// Trace-Enabled Golden Test: Decision Trace End-to-End
// ---------------------------------------------------------------------------

/// Validates the trace system end-to-end: enable → step → query → dump.
///
/// Creates a simple scenario with one hungry agent that has bread, runs 5 ticks
/// with tracing enabled, then asserts traces were collected and contain at least
/// one Planning outcome with non-empty candidates.
#[test]
fn golden_trace_enabled_scenario() {
    let mut h = GoldenHarness::new(Seed([99; 32]));

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "TracedAgent",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    // Give the agent bread to eat.
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
    );

    // Seed local beliefs so the agent can observe its surroundings.
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        worldwake_core::Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );

    // Enable tracing before stepping.
    h.driver.enable_tracing();

    // Run 5 ticks.
    for _ in 0..5 {
        h.step_once();
    }

    // Query traces — must have been collected.
    let sink = h.driver.trace_sink().expect("tracing was enabled");
    assert!(
        !sink.traces().is_empty(),
        "traces should be non-empty after 5 ticks"
    );

    // All traces should be for our agent.
    let agent_traces = sink.traces_for(agent);
    assert_eq!(
        agent_traces.len(),
        sink.traces().len(),
        "all traces should be for the single agent"
    );

    // At least one Planning outcome with non-empty candidates.
    let has_planning_with_candidates = agent_traces.iter().any(|t| {
        if let DecisionOutcome::Planning(ref planning) = t.outcome {
            !planning.candidates.ranked.is_empty()
        } else {
            false
        }
    });
    assert!(
        has_planning_with_candidates,
        "at least one tick should have a Planning outcome with non-empty ranked candidates"
    );

    // summary() should produce non-empty strings for all traces.
    for trace in sink.traces() {
        let summary = trace.outcome.summary();
        assert!(!summary.is_empty(), "summary should never be empty");
    }

    // dump_agent must not panic (output goes to stderr, not asserted).
    sink.dump_agent(agent, &h.defs);
}

// ---------------------------------------------------------------------------
// GT-1: Planner Falls Back to Addressable Needs When Top Need Unsatisfiable
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI
// GoalKinds: ConsumeOwnedCommodity
// ActionDomains: Needs
// Principles: P20 (resource-bounded practical reasoning)
//
// Setup: Single AI agent with high thirst (700) and moderate hunger (400).
//   No Water items anywhere (thirst unsatisfiable). Apple available at location.
//
// Proves: when top-priority need (thirst) has no viable plan, agent falls back
//   to next-best addressable need (hunger) instead of going idle indefinitely.
//
// Chain: unsatisfiable top need -> planner exhausts thirst-relief candidates ->
//   falls back to hunger-relief -> agent eats or sleeps.

#[test]
fn golden_fallback_to_addressable_need_when_top_need_unsatisfiable() {
    let mut h = GoldenHarness::new(Seed([100; 32]));

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Parched",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(400), pm(700), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    // Give food but NO water — thirst is unsatisfiable
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent,
        VILLAGE_SQUARE,
        CommodityKind::Apple,
        Quantity(5),
    );

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );

    h.driver.enable_tracing();

    let initial_apples = h.agent_commodity_qty(agent, CommodityKind::Apple);
    let mut ate = false;
    let mut slept = false;
    let mut max_idle = 0u32;
    let mut consecutive_idle = 0u32;

    for _tick in 0..200 {
        h.step_once();

        let had_action = if let Some(sink) = h.action_trace_sink() {
            sink.events_for(agent).iter().any(|e| {
                if matches!(e.kind, ActionTraceKind::Committed { .. }) {
                    match e.action_name.as_str() {
                        "eat" => ate = true,
                        "sleep" => slept = true,
                        _ => {}
                    }
                    true
                } else {
                    false
                }
            })
        } else {
            false
        };

        if h.agent_commodity_qty(agent, CommodityKind::Apple) < initial_apples {
            ate = true;
        }
        if h.agent_active_action_name(agent) == Some("sleep") {
            slept = true;
        }

        if had_action {
            consecutive_idle = 0;
        } else {
            consecutive_idle += 1;
            max_idle = max_idle.max(consecutive_idle);
        }

        if ate || slept {
            break;
        }
    }

    let decision_summaries = h
        .driver
        .trace_sink()
        .map(|sink| {
            sink.traces_for(agent)
                .into_iter()
                .map(|trace| format!("{:?}: {}", trace.tick, trace.outcome.summary()))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    let zero_step_self_care = h
        .driver
        .trace_sink()
        .map(|sink| {
            sink.traces_for(agent)
                .into_iter()
                .filter_map(|trace| {
                    let DecisionOutcome::Planning(planning) = &trace.outcome else {
                        return None;
                    };
                    let selected_goal = planning.selection.selected_goal()?;
                    let selected_plan = planning.selection.selected_plan.as_ref()?;
                    (matches!(selected_goal.kind, GoalKind::Sleep | GoalKind::Relieve)
                        && selected_plan.terminal_kind == PlanTerminalKind::GoalSatisfied
                        && selected_plan.steps.is_empty())
                    .then(|| format!("{:?}: {}", trace.tick, trace.outcome.summary()))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    assert!(
        ate || slept,
        "Agent with unsatisfiable thirst should still eat or sleep — got neither in 200 ticks (max idle streak: {max_idle})\n{decision_summaries}"
    );
    assert!(
        max_idle < 100,
        "Agent should not be idle for 100+ consecutive ticks before falling back to an addressable need, was idle for {max_idle}\n{decision_summaries}"
    );
    assert!(
        zero_step_self_care.is_empty(),
        "low-band Sleep/Relieve goals must not collapse into zero-step GoalSatisfied selections:\n{}",
        zero_step_self_care.join("\n")
    );
}
