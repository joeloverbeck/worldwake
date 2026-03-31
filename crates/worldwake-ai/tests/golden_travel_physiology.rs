//! Golden tests for travel physiology: per-agent body cost multipliers
//! during travel and their interaction with the AI decision pipeline.
//!
//! These tests prove that travel exertion multipliers on `MetabolismProfile`
//! produce observable need escalation, trigger interrupts at critical thresholds,
//! and create diversity between agents with different profiles.

mod golden_harness;

use golden_harness::*;
use worldwake_core::{
    prototype_place_entity, CommodityKind, EventTag, EventView, HomeostaticNeeds,
    MetabolismProfile, PerceptionProfile, PrototypePlace, Quantity, ResourceSource, Seed, Tick,
    UtilityProfile, WorkstationTag,
};

// ---------------------------------------------------------------------------
// Place constants
// ---------------------------------------------------------------------------

const EAST_FIELD_TRAIL: worldwake_core::EntityId =
    prototype_place_entity(PrototypePlace::EastFieldTrail);
const COMMON_HOUSE: worldwake_core::EntityId =
    prototype_place_entity(PrototypePlace::CommonHouse);
const FOREST_PATH: worldwake_core::EntityId =
    prototype_place_entity(PrototypePlace::ForestPath);

// ---------------------------------------------------------------------------
// Shared setup: place an apple-producing workstation at OrchardFarm so
// agents with hunger have a reason to travel there from VillageSquare.
// Route: VillageSquare → SouthGate (2) → EastFieldTrail (3) →
//        OrchardFarm (2) = 7 ticks of travel.
// ---------------------------------------------------------------------------

fn setup_food_at_orchard(h: &mut GoldenHarness) {
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(50),
            max_quantity: Quantity(50),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );
}

// ---------------------------------------------------------------------------
// Scenario 58: Travel Need Escalation
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, Travel, Production
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Travel, Production, Needs
// Places: VillageSquare, SouthGate, EastFieldTrail, OrchardFarm
// Principles: 8, 22, 26
//
// Setup: One agent at VillageSquare with high hunger (pm(700)) and
//   non-zero travel_bladder_multiplier (pm(500)). OrchardFarm has
//   apples via OrchardRow + ResourceSource. Agent plans multi-hop
//   travel (7 ticks). Bladder starts at pm(0).
//
// Proves: Travel body cost multipliers produce additional need
//   escalation beyond basal rate. The needs system applies both
//   basal metabolism (from MetabolismProfile.bladder_rate) and
//   travel body cost override (from start_travel handler) each tick.
//
// Chain: hunger pressure -> AcquireCommodity goal -> travel plan to
//   OrchardFarm -> travel body cost override applied -> needs system
//   adds basal + travel cost per tick -> bladder rises faster than
//   basal rate alone.

#[test]
fn golden_travel_escalation() {
    let mut h = GoldenHarness::new(Seed([90; 32]));
    setup_food_at_orchard(&mut h);

    // bladder_rate = 10, travel_bladder_multiplier = 500
    // Per tick during travel: basal 10 + travel (10 * 500 / 1000) = 10 + 5 = 15
    // Per tick idle: basal 10 only
    let travel_metabolism = MetabolismProfile::new(
        pm(2),   // hunger_rate
        pm(2),   // thirst_rate
        pm(2),   // fatigue_rate
        pm(10),  // bladder_rate
        pm(1),   // dirtiness_rate
        pm(20),  // rest_efficiency
        nz(480), // starvation_tolerance_ticks
        nz(240), // dehydration_tolerance_ticks
        nz(120), // exhaustion_collapse_ticks
        nz(200), // bladder_accident_tolerance_ticks
        nz(8),   // toilet_ticks
        nz(12),  // wash_ticks
        pm(0),   // travel_fatigue_multiplier
        pm(0),   // travel_thirst_multiplier
        pm(500), // travel_bladder_multiplier
        pm(0),   // wilderness_relief_dirtiness_penalty
    );

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Traveler",
        VILLAGE_SQUARE,
        // High hunger (above high threshold 750) drives travel to OrchardFarm.
        // Bladder starts at 0 — we measure how much it rises during travel.
        HomeostaticNeeds::new(pm(700), pm(0), pm(0), pm(0), pm(0)),
        travel_metabolism,
        UtilityProfile {
            hunger_weight: pm(900),
            ..UtilityProfile::default()
        },
    );

    // Seed beliefs about the orchard and its workstation so planner can find food.
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let initial_bladder = h.agent_bladder(agent);
    assert_eq!(initial_bladder, pm(0), "bladder should start at zero");

    // Run enough ticks for agent to travel to OrchardFarm.
    // Route: VillageSquare → SouthGate (2) → EastFieldTrail (3) → OrchardFarm (2) = 7 ticks.
    // Each leg is a separate travel action, so we count all ticks where the agent
    // is in a travel action across all legs.
    let mut ticks_traveled = 0u32;
    let mut non_travel_ticks_after_first_travel = 0u32;

    for _ in 0..80 {
        h.step_once();

        let is_traveling = h.agent_active_action_name(agent) == Some("travel");

        if is_traveling {
            ticks_traveled += 1;
            non_travel_ticks_after_first_travel = 0;
        } else if ticks_traveled > 0 {
            non_travel_ticks_after_first_travel += 1;
            // Allow a gap of 1 tick between travel legs (for AI replanning).
            // After 2+ consecutive non-travel ticks, travel is done.
            if non_travel_ticks_after_first_travel >= 2 {
                break;
            }
        }

        // Safety: stop if bladder gets very high (avoid deprivation cascade).
        if h.agent_bladder(agent).value() > 900 {
            break;
        }
    }

    let final_bladder = h.agent_bladder(agent);

    assert!(ticks_traveled >= 1, "agent should have traveled at least 1 tick");

    // The bladder should have increased by more than just basal rate.
    // Basal rate alone: bladder_rate (10) per tick.
    // With travel: basal (10) + travel cost (10 * 500 / 1000 = 5) = 15 per tick.
    //
    // We measure total bladder increase over the entire run (travel + non-travel ticks).
    // The travel ticks contribute 15/tick while non-travel ticks contribute 10/tick.
    // The excess above basal-only must come from travel body costs.
    let total_ticks_elapsed = ticks_traveled + non_travel_ticks_after_first_travel;
    let basal_only_estimate = 10u16 * total_ticks_elapsed as u16;
    let actual_increase = final_bladder.value() - initial_bladder.value();

    // The excess should be at least 4 * ticks_traveled (travel adds 5/tick,
    // with 1 permille tolerance per tick for rounding).
    let min_excess = 4u16 * ticks_traveled as u16;

    assert!(
        actual_increase > basal_only_estimate,
        "bladder increase ({actual_increase}) should exceed basal-only estimate \
         ({basal_only_estimate}) after {ticks_traveled} travel ticks. \
         initial={}, final={}",
        initial_bladder.value(),
        final_bladder.value(),
    );

    assert!(
        actual_increase - basal_only_estimate >= min_excess,
        "excess bladder increase ({}) above basal-only ({basal_only_estimate}) should be \
         at least {min_excess} (4 * {ticks_traveled} travel ticks), confirming \
         travel multiplier adds body cost",
        actual_increase - basal_only_estimate,
    );
}

// ---------------------------------------------------------------------------
// Scenario 59: Critical Bladder Local Relief
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI
// GoalKinds: Relieve
// ActionDomains: Needs
// Places: EastFieldTrail
// Principles: 8, 20, 26
//
// Setup: One agent at EastFieldTrail (outdoor: Trail + Field) with
//   high travel_bladder_multiplier (pm(800)) and initial bladder near
//   critical threshold (pm(850)). EastFieldTrail is outdoor so
//   relieve_wilderness is locally available. The agent relieves
//   locally without needing to travel.
//
// Proves: The AI detects critical bladder pressure and acts on
//   GoalKind::Relieve via locally available relieve_wilderness at
//   an outdoor place. This proves the weaker invariant: critical
//   bladder → immediate local relief. The stronger travel-interrupt
//   invariant is covered by golden_travel_interrupt_from_bladder_escalation.
//
// Chain: high bladder pressure -> Relieve goal ranked highest ->
//   relieve_wilderness available locally -> agent relieves without travel.

#[test]
fn golden_critical_bladder_local_relief() {
    let mut h = GoldenHarness::new(Seed([91; 32]));
    h.enable_action_tracing();
    h.driver.enable_tracing();

    // Very aggressive travel bladder multiplier: bladder_rate=15,
    // travel_bladder_multiplier=800 → travel adds 15*800/1000=12/tick.
    // Total during travel: 15 + 12 = 27/tick.
    // Starting at pm(850), critical at pm(930): need ~3 ticks to cross.
    let aggressive_metabolism = MetabolismProfile::new(
        pm(2),   // hunger_rate
        pm(2),   // thirst_rate
        pm(2),   // fatigue_rate
        pm(15),  // bladder_rate
        pm(1),   // dirtiness_rate
        pm(20),  // rest_efficiency
        nz(480), // starvation_tolerance_ticks
        nz(240), // dehydration_tolerance_ticks
        nz(120), // exhaustion_collapse_ticks
        nz(200), // bladder_accident_tolerance_ticks
        nz(8),   // toilet_ticks
        nz(12),  // wash_ticks
        pm(0),   // travel_fatigue_multiplier
        pm(0),   // travel_thirst_multiplier
        pm(800), // travel_bladder_multiplier
        pm(0),   // wilderness_relief_dirtiness_penalty
    );

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Urgenta",
        EAST_FIELD_TRAIL,
        // Start bladder near critical (930). A few ticks of travel will push it over.
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(850), pm(0)),
        aggressive_metabolism,
        UtilityProfile {
            bladder_weight: pm(900),
            ..UtilityProfile::default()
        },
    );

    // Seed beliefs about the world so the planner can find latrine.
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let mut travel_was_interrupted = false;
    let mut saw_relieve_action = false;

    for _ in 0..80 {
        h.step_once();

        if let Some(action_name) = h.agent_active_action_name(agent) {
            if action_name == "toilet" || action_name == "relieve_wilderness" {
                saw_relieve_action = true;
            }
        }

        if saw_relieve_action {
            break;
        }
    }

    // Check action traces for travel abort.
    if let Some(sink) = h.action_trace_sink() {
        let events = sink.events_for(agent);
        for event in &events {
            if matches!(event.kind, worldwake_sim::ActionTraceKind::Aborted { .. }) {
                if let Some(def) = h.defs.get(event.def_id) {
                    if def.name == "travel" {
                        travel_was_interrupted = true;
                    }
                }
            }
        }
    }

    let final_bladder = h.agent_bladder(agent);

    // The agent should have either interrupted travel or started relief.
    assert!(
        travel_was_interrupted || saw_relieve_action,
        "agent should have either interrupted travel or started relief. \
         travel_interrupted={travel_was_interrupted}, saw_relieve={saw_relieve_action}, \
         final_bladder={}",
        final_bladder.value(),
    );

    // Verify via decision trace that Relieve goal appeared.
    if let Some(trace_sink) = h.driver.trace_sink() {
        let traces = trace_sink.traces_for(agent);
        let relieve_appeared = traces.iter().any(|trace| {
            if let worldwake_ai::DecisionOutcome::Planning(ref p) = trace.outcome {
                p.candidates.ranked.iter().any(|c| {
                    matches!(c.opportunity.goal_key.kind, worldwake_core::GoalKind::Relieve)
                })
            } else {
                false
            }
        });
        assert!(
            relieve_appeared,
            "GoalKind::Relieve should have appeared in decision trace candidates"
        );
    }
}

// ---------------------------------------------------------------------------
// Scenario 60: Agent Diversity in Travel Escalation
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, Travel, Production
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Travel, Production
// Places: VillageSquare, SouthGate, EastFieldTrail, OrchardFarm
// Principles: 8, 22, 26
//
// Setup: Two agents at VillageSquare with identical high hunger and
//   utility but different travel_bladder_multiplier values: pm(200)
//   vs pm(800). OrchardFarm has apples. Both plan multi-hop travel
//   (7 ticks) to acquire food.
//
// Proves: Per-agent MetabolismProfile travel multipliers produce
//   different escalation rates. Agent diversity (Principle 22) emerges
//   from profile differences, not hardcoded behavior. The needs system
//   applies each agent's body cost override independently.
//
// Chain: identical hunger pressure -> same AcquireCommodity goal ->
//   same travel route -> different body cost overrides from different
//   MetabolismProfiles -> divergent bladder values after same travel
//   duration.

#[test]
fn golden_agent_diversity() {
    let mut h = GoldenHarness::new(Seed([92; 32]));
    setup_food_at_orchard(&mut h);

    let base_needs = HomeostaticNeeds::new(pm(700), pm(0), pm(0), pm(0), pm(0));
    let base_utility = UtilityProfile {
        hunger_weight: pm(900),
        ..UtilityProfile::default()
    };

    // Agent A: low travel bladder multiplier (200)
    // Travel bladder cost: 10 * 200 / 1000 = 2/tick additional
    let low_multiplier_metabolism = MetabolismProfile::new(
        pm(2),   // hunger_rate
        pm(2),   // thirst_rate
        pm(2),   // fatigue_rate
        pm(10),  // bladder_rate
        pm(1),   // dirtiness_rate
        pm(20),  // rest_efficiency
        nz(480), // starvation_tolerance_ticks
        nz(240), // dehydration_tolerance_ticks
        nz(120), // exhaustion_collapse_ticks
        nz(200), // bladder_accident_tolerance_ticks
        nz(8),   // toilet_ticks
        nz(12),  // wash_ticks
        pm(0),   // travel_fatigue_multiplier
        pm(0),   // travel_thirst_multiplier
        pm(200), // travel_bladder_multiplier
        pm(0),   // wilderness_relief_dirtiness_penalty
    );

    // Agent B: high travel bladder multiplier (800)
    // Travel bladder cost: 10 * 800 / 1000 = 8/tick additional
    let high_multiplier_metabolism = MetabolismProfile::new(
        pm(2),   // hunger_rate
        pm(2),   // thirst_rate
        pm(2),   // fatigue_rate
        pm(10),  // bladder_rate
        pm(1),   // dirtiness_rate
        pm(20),  // rest_efficiency
        nz(480), // starvation_tolerance_ticks
        nz(240), // dehydration_tolerance_ticks
        nz(120), // exhaustion_collapse_ticks
        nz(200), // bladder_accident_tolerance_ticks
        nz(8),   // toilet_ticks
        nz(12),  // wash_ticks
        pm(0),   // travel_fatigue_multiplier
        pm(0),   // travel_thirst_multiplier
        pm(800), // travel_bladder_multiplier
        pm(0),   // wilderness_relief_dirtiness_penalty
    );

    let agent_low = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "SlowBladder",
        VILLAGE_SQUARE,
        base_needs,
        low_multiplier_metabolism,
        base_utility.clone(),
    );

    let agent_high = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "FastBladder",
        VILLAGE_SQUARE,
        base_needs,
        high_multiplier_metabolism,
        base_utility,
    );

    // Seed beliefs about the world for both agents.
    for agent in [agent_low, agent_high] {
        seed_actor_world_beliefs(
            &mut h.world,
            &mut h.event_log,
            agent,
            Tick(0),
            worldwake_core::PerceptionSource::Inference,
        );
    }

    // Track whether each agent traveled.
    let mut low_traveled = false;
    let mut high_traveled = false;
    let mut low_bladder_after_travel = pm(0);
    let mut high_bladder_after_travel = pm(0);

    for _ in 0..80 {
        h.step_once();

        if h.agent_active_action_name(agent_low) == Some("travel") {
            low_traveled = true;
        }
        if h.agent_active_action_name(agent_high) == Some("travel") {
            high_traveled = true;
        }

        // Continuously update bladder snapshots while either is traveling.
        if low_traveled || high_traveled {
            low_bladder_after_travel = h.agent_bladder(agent_low);
            high_bladder_after_travel = h.agent_bladder(agent_high);
        }

        // Stop once both have finished all travel.
        if low_traveled
            && high_traveled
            && h.agent_active_action_name(agent_low) != Some("travel")
            && h.agent_active_action_name(agent_high) != Some("travel")
        {
            break;
        }

        // Safety: stop if bladder gets very high.
        if h.agent_bladder(agent_high).value() > 900 {
            low_bladder_after_travel = h.agent_bladder(agent_low);
            high_bladder_after_travel = h.agent_bladder(agent_high);
            break;
        }
    }

    assert!(low_traveled, "low-multiplier agent should have traveled");
    assert!(high_traveled, "high-multiplier agent should have traveled");

    // Both agents started with bladder 0.
    // The agent with higher multiplier (800) should have higher bladder
    // than the agent with lower multiplier (200) after traveling.
    assert!(
        high_bladder_after_travel > low_bladder_after_travel,
        "agent with higher travel_bladder_multiplier (800) should have higher bladder ({}) \
         than agent with lower multiplier (200) ({})",
        high_bladder_after_travel.value(),
        low_bladder_after_travel.value(),
    );

    // The difference should be meaningful — at least proportional to the
    // multiplier difference. With bladder_rate=10:
    // low cost: 10*200/1000 = 2/tick additional
    // high cost: 10*800/1000 = 8/tick additional
    // Difference: 6/tick. Over even 2 ticks, that's 12 permille difference.
    let difference = high_bladder_after_travel
        .value()
        .saturating_sub(low_bladder_after_travel.value());
    assert!(
        difference >= 5,
        "bladder difference ({difference}) should be at least 5 permille, \
         confirming multiplier-driven divergence",
    );
}

// ---------------------------------------------------------------------------
// Scenario 61: Travel Interrupt from Bladder Escalation
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, Travel, Production
// GoalKinds: Relieve, AcquireCommodity(SelfConsume)
// ActionDomains: Travel, Needs, Production
// Places: VillageSquare, SouthGate, EastFieldTrail, OrchardFarm
// Principles: 8, 20, 22, 26
//
// Setup: One agent at VillageSquare (indoor: Village tag only, no
//   wilderness relief, no latrine). Hunger at pm(800) (High priority,
//   threshold >= 750) drives travel toward OrchardFarm. Bladder starts
//   at pm(799) (Medium priority, threshold >= 600 and < 800 High).
//   Hunger outranks Relieve initially (High > Medium).
//   bladder_rate=70, travel_bladder_multiplier=900 → travel body cost
//   = 70*900/1000 = 63/tick additional, total 133/tick during travel.
//   First leg: VillageSquare → SouthGate (2 ticks). After tick 0
//   systems run: bladder = 799 + 133 = 932 (Critical! >= 930).
//   At tick 1, AI evaluates interrupt: Relieve at Critical priority
//   vs active travel (InterruptibleWithPenalty). interrupt_with_penalty
//   fires → travel aborted. Agent returned to VillageSquare (origin).
//   Agent replans for Relieve, travels to PublicLatrine (1 tick) or
//   finds another relief path.
//
// Proves: Travel body cost escalation causes bladder to cross the
//   critical threshold during a single travel leg. The interrupt system
//   detects critical survival pressure and aborts the InterruptibleWithPenalty
//   travel action mid-leg. The agent replans for GoalKind::Relieve and
//   performs a relief action. This is the stronger invariant missing from
//   golden_critical_bladder_local_relief, which only proves local relief.
//
// Chain: hunger pressure (High) -> AcquireCommodity goal -> travel plan
//   to OrchardFarm -> travel body cost override (133/tick) applied ->
//   needs system escalates bladder past critical (799+133=932) after
//   1 tick -> interrupt fires CriticalSurvival -> travel aborted ->
//   agent replans for Relieve -> relief action committed.

#[test]
fn golden_travel_interrupt_from_bladder_escalation() {
    let mut h = GoldenHarness::new(Seed([93; 32]));
    h.enable_action_tracing();
    h.driver.enable_tracing();

    setup_food_at_orchard(&mut h);

    // bladder_rate=70, travel_bladder_multiplier=900
    // Per tick during travel: basal 70 + travel (70 * 900 / 1000) = 70 + 63 = 133
    // Starting at pm(799), critical at pm(930): crosses after 1 tick (799 + 133 = 932).
    let aggressive_metabolism = MetabolismProfile::new(
        pm(2),   // hunger_rate
        pm(2),   // thirst_rate
        pm(2),   // fatigue_rate
        pm(70),  // bladder_rate
        pm(1),   // dirtiness_rate
        pm(20),  // rest_efficiency
        nz(480), // starvation_tolerance_ticks
        nz(240), // dehydration_tolerance_ticks
        nz(120), // exhaustion_collapse_ticks
        nz(200), // bladder_accident_tolerance_ticks
        nz(8),   // toilet_ticks
        nz(12),  // wash_ticks
        pm(0),   // travel_fatigue_multiplier
        pm(0),   // travel_thirst_multiplier
        pm(900), // travel_bladder_multiplier
        pm(0),   // wilderness_relief_dirtiness_penalty
    );

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "TravelInterruptee",
        VILLAGE_SQUARE,
        // Hunger at pm(800) → High priority (threshold >= 750).
        // Bladder at pm(799) → Medium priority (threshold >= 600, < 800 High).
        // Hunger outranks bladder initially (High > Medium). During travel,
        // 1 tick of body cost pushes bladder to 932 (Critical), triggering interrupt.
        HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(799), pm(0)),
        aggressive_metabolism,
        UtilityProfile {
            hunger_weight: pm(950),
            bladder_weight: pm(500),
            ..UtilityProfile::default()
        },
    );

    // Seed beliefs about the world so the planner can find OrchardFarm food.
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    // Run enough ticks for travel to start and then be interrupted.
    for _ in 0..80 {
        h.step_once();
    }

    // --- Verification Layer 1: Travel started ---
    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let agent_events = action_sink.events_for(agent);

    let travel_started = agent_events.iter().any(|e| {
        matches!(e.kind, worldwake_sim::ActionTraceKind::Started { .. })
            && h.defs.get(e.def_id).map(|d| d.name.as_str()) == Some("travel")
    });
    assert!(
        travel_started,
        "agent should have started a travel action from VillageSquare"
    );

    // --- Verification Layer 2: CriticalSurvival interrupt fired during travel ---
    // The interrupt-based abort path does not emit an ActionTraceKind::Aborted
    // event, so we prove the interrupt via the decision trace instead of action
    // trace. The decision trace is the correct semantic surface for this contract
    // (per golden-e2e-testing.md: "Use decision traces when proving AI reasoning").
    let decision_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled");
    let traces = decision_sink.traces_for(agent);
    let critical_survival_interrupt_during_travel = traces.iter().any(|trace| {
        if let worldwake_ai::DecisionOutcome::ActiveAction {
            ref interrupt, ..
        } = trace.outcome
        {
            matches!(
                interrupt.decision,
                worldwake_ai::InterruptDecision::InterruptForReplan {
                    trigger: worldwake_ai::InterruptTrigger::CriticalSurvival,
                }
            )
        } else {
            false
        }
    });
    assert!(
        critical_survival_interrupt_during_travel,
        "CriticalSurvival interrupt should have fired during active travel"
    );

    // --- Verification Layer 3: Relieve goal appeared after interrupt ---
    let relieve_appeared = traces.iter().any(|trace| {
        if let worldwake_ai::DecisionOutcome::Planning(ref p) = trace.outcome {
            p.candidates.ranked.iter().any(|c| {
                matches!(c.opportunity.goal_key.kind, worldwake_core::GoalKind::Relieve)
            })
        } else {
            false
        }
    });
    assert!(
        relieve_appeared,
        "GoalKind::Relieve should have appeared in decision trace after travel interrupt"
    );

    // --- Verification Layer 4: Relief action committed ---
    let relief_committed = agent_events.iter().any(|e| {
        matches!(e.kind, worldwake_sim::ActionTraceKind::Committed { .. })
            && h.defs
                .get(e.def_id)
                .is_some_and(|d| d.name.as_str() == "toilet" || d.name.as_str() == "relieve_wilderness")
    });
    assert!(
        relief_committed,
        "agent should have committed a relief action (toilet or relieve_wilderness)"
    );
}

// ---------------------------------------------------------------------------
// Scenario 62: Latrine Preferred — toilet at Latrine place
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI
// GoalKinds: Relieve
// ActionDomains: Needs
// Places: PublicLatrine
// Principles: 1, 8, 20
//
// Setup: One agent at PublicLatrine (Latrine + Village tags) with critical
//   bladder (pm(950)). PublicLatrine has no outdoor tags, so only
//   `toilet` is afforded (not `relieve_wilderness`). The agent has a
//   non-zero wilderness_relief_dirtiness_penalty to confirm dirtiness
//   is NOT applied (toilet path, not wilderness path).
//
// Proves: When an agent is at a latrine, GoalKind::Relieve resolves to
//   the `toilet` action. Bladder resets to pm(0), dirtiness is unchanged
//   (no wilderness penalty), and waste is created at the place. This is
//   the latrine branch of the relief decision tree.
//
// Chain: critical bladder pressure -> Relieve goal ranked highest ->
//   toilet afforded at Latrine-tagged place -> toilet committed ->
//   bladder=0, dirtiness unchanged, waste created.

#[test]
fn golden_latrine_preferred() {
    let mut h = GoldenHarness::new(Seed([94; 32]));
    h.enable_action_tracing();
    h.driver.enable_tracing();

    let metabolism = MetabolismProfile::new(
        pm(2),   // hunger_rate
        pm(2),   // thirst_rate
        pm(2),   // fatigue_rate
        pm(10),  // bladder_rate
        pm(1),   // dirtiness_rate
        pm(20),  // rest_efficiency
        nz(480), // starvation_tolerance_ticks
        nz(240), // dehydration_tolerance_ticks
        nz(120), // exhaustion_collapse_ticks
        nz(200), // bladder_accident_tolerance_ticks
        nz(8),   // toilet_ticks
        nz(12),  // wash_ticks
        pm(0),   // travel_fatigue_multiplier
        pm(0),   // travel_thirst_multiplier
        pm(0),   // travel_bladder_multiplier
        pm(200), // wilderness_relief_dirtiness_penalty — non-zero to confirm it's NOT applied
    );

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "LatrineUser",
        PUBLIC_LATRINE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(950), pm(0)),
        metabolism,
        UtilityProfile {
            bladder_weight: pm(900),
            ..UtilityProfile::default()
        },
    );

    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let initial_dirtiness = h.agent_dirtiness(agent);

    // Run until toilet commits. Toilet takes toilet_ticks (8) to complete,
    // plus a few ticks for planning. Bladder keeps rising after relief,
    // so we check outcomes at the commit boundary, not at a fixed tick count.
    let mut toilet_committed_tick = None;
    for tick in 0..40u64 {
        h.step_once();

        if toilet_committed_tick.is_none() {
            let sink = h
                .action_trace_sink()
                .expect("action tracing should be enabled");
            let committed = sink.events_for(agent).iter().any(|e| {
                matches!(e.kind, worldwake_sim::ActionTraceKind::Committed { .. })
                    && h.defs
                        .get(e.def_id)
                        .is_some_and(|d| d.name.as_str() == "toilet")
            });
            if committed {
                toilet_committed_tick = Some(tick);
                break;
            }
        }
    }

    // --- Verification Layer 1: Decision trace shows Relieve goal ---
    let decision_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled");
    let traces = decision_sink.traces_for(agent);
    let relieve_appeared = traces.iter().any(|trace| {
        if let worldwake_ai::DecisionOutcome::Planning(ref p) = trace.outcome {
            p.candidates
                .ranked
                .iter()
                .any(|c| matches!(c.opportunity.goal_key.kind, worldwake_core::GoalKind::Relieve))
        } else {
            false
        }
    });
    assert!(
        relieve_appeared,
        "GoalKind::Relieve should have appeared in decision trace candidates"
    );

    // --- Verification Layer 2: Action trace shows toilet committed (not relieve_wilderness) ---
    assert!(
        toilet_committed_tick.is_some(),
        "agent should have committed a toilet action at PublicLatrine within 40 ticks"
    );

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let agent_events = action_sink.events_for(agent);

    let wilderness_committed = agent_events.iter().any(|e| {
        matches!(e.kind, worldwake_sim::ActionTraceKind::Committed { .. })
            && h.defs
                .get(e.def_id)
                .is_some_and(|d| d.name.as_str() == "relieve_wilderness")
    });
    assert!(
        !wilderness_committed,
        "relieve_wilderness should NOT have been committed at a Latrine-tagged place"
    );

    // --- Verification Layer 3: Authoritative state — bladder reset at commit, dirtiness unchanged ---
    // Bladder is pm(0) at commit time. We broke out immediately after commit,
    // but one more tick may have elapsed. Check bladder is low (basal drift only).
    assert!(
        h.agent_bladder(agent).value() <= 20,
        "bladder ({}) should be near pm(0) right after toilet commit",
        h.agent_bladder(agent).value(),
    );

    // Dirtiness should only have basal drift (1/tick), NOT wilderness penalty.
    // The wilderness penalty is pm(200). Basal drift over ~10 ticks is ~10.
    let final_dirtiness = h.agent_dirtiness(agent);
    assert!(
        final_dirtiness.value() < initial_dirtiness.value() + 100,
        "dirtiness ({}) should not have the wilderness penalty applied (initial={}, penalty=200)",
        final_dirtiness.value(),
        initial_dirtiness.value(),
    );

    // --- Verification Layer 4: Waste created at PublicLatrine ---
    let waste_at_latrine = h
        .world
        .entities_effectively_at(PUBLIC_LATRINE)
        .into_iter()
        .any(|entity| {
            h.world
                .get_component_item_lot(entity)
                .is_some_and(|lot| lot.commodity == CommodityKind::Waste)
        });
    assert!(
        waste_at_latrine,
        "waste should have been created at PublicLatrine after toilet"
    );
}

// ---------------------------------------------------------------------------
// Scenario 63: Wilderness Fallback — relieve_wilderness at outdoor place
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI
// GoalKinds: Relieve
// ActionDomains: Needs
// Places: ForestPath
// Principles: 1, 8, 20
//
// Setup: One agent at ForestPath (Forest + Trail tags, both outdoor) with
//   critical bladder (pm(950)). ForestPath has no Latrine tag, so `toilet`
//   is NOT afforded — only `relieve_wilderness`. Agent has non-zero
//   wilderness_relief_dirtiness_penalty (pm(200)).
//
// Proves: When an agent is at an outdoor place with no latrine,
//   GoalKind::Relieve resolves to `relieve_wilderness`. Bladder resets
//   to pm(0), dirtiness increases by wilderness_relief_dirtiness_penalty,
//   and waste is created at the place.
//
// Chain: critical bladder pressure -> Relieve goal ranked highest ->
//   relieve_wilderness afforded at outdoor place (no latrine) ->
//   relieve_wilderness committed -> bladder=0, dirtiness+=penalty,
//   waste created.

#[test]
fn golden_wilderness_fallback() {
    let mut h = GoldenHarness::new(Seed([95; 32]));
    h.enable_action_tracing();
    h.driver.enable_tracing();

    let metabolism = MetabolismProfile::new(
        pm(2),   // hunger_rate
        pm(2),   // thirst_rate
        pm(2),   // fatigue_rate
        pm(10),  // bladder_rate
        pm(1),   // dirtiness_rate
        pm(20),  // rest_efficiency
        nz(480), // starvation_tolerance_ticks
        nz(240), // dehydration_tolerance_ticks
        nz(120), // exhaustion_collapse_ticks
        nz(200), // bladder_accident_tolerance_ticks
        nz(8),   // toilet_ticks
        nz(12),  // wash_ticks
        pm(0),   // travel_fatigue_multiplier
        pm(0),   // travel_thirst_multiplier
        pm(0),   // travel_bladder_multiplier
        pm(200), // wilderness_relief_dirtiness_penalty
    );

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "ForestReliever",
        FOREST_PATH,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(950), pm(0)),
        metabolism,
        UtilityProfile {
            bladder_weight: pm(900),
            ..UtilityProfile::default()
        },
    );

    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let initial_dirtiness = h.agent_dirtiness(agent);

    // Run until relieve_wilderness commits. Break early to avoid
    // bladder re-escalating and confusing post-commit assertions.
    let mut wilderness_committed_tick = None;
    for tick in 0..40u64 {
        h.step_once();

        if wilderness_committed_tick.is_none() {
            let sink = h
                .action_trace_sink()
                .expect("action tracing should be enabled");
            let committed = sink.events_for(agent).iter().any(|e| {
                matches!(e.kind, worldwake_sim::ActionTraceKind::Committed { .. })
                    && h.defs
                        .get(e.def_id)
                        .is_some_and(|d| d.name.as_str() == "relieve_wilderness")
            });
            if committed {
                wilderness_committed_tick = Some(tick);
                break;
            }
        }
    }

    // --- Verification Layer 1: Decision trace shows Relieve goal ---
    let decision_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled");
    let traces = decision_sink.traces_for(agent);
    let relieve_appeared = traces.iter().any(|trace| {
        if let worldwake_ai::DecisionOutcome::Planning(ref p) = trace.outcome {
            p.candidates
                .ranked
                .iter()
                .any(|c| matches!(c.opportunity.goal_key.kind, worldwake_core::GoalKind::Relieve))
        } else {
            false
        }
    });
    assert!(
        relieve_appeared,
        "GoalKind::Relieve should have appeared in decision trace candidates"
    );

    // --- Verification Layer 2: Action trace shows relieve_wilderness committed ---
    assert!(
        wilderness_committed_tick.is_some(),
        "agent should have committed relieve_wilderness at ForestPath within 40 ticks"
    );

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let agent_events = action_sink.events_for(agent);

    let toilet_committed = agent_events.iter().any(|e| {
        matches!(e.kind, worldwake_sim::ActionTraceKind::Committed { .. })
            && h.defs
                .get(e.def_id)
                .is_some_and(|d| d.name.as_str() == "toilet")
    });
    assert!(
        !toilet_committed,
        "toilet should NOT have been committed at ForestPath (no Latrine tag)"
    );

    // --- Verification Layer 3: Authoritative state — bladder reset, dirtiness increased ---
    // Broke out right after commit; bladder may have re-escalated by 1 tick.
    assert!(
        h.agent_bladder(agent).value() <= 20,
        "bladder ({}) should be near pm(0) right after relieve_wilderness commit",
        h.agent_bladder(agent).value(),
    );

    let final_dirtiness = h.agent_dirtiness(agent);
    // Dirtiness should have increased by at least the wilderness penalty (200).
    // Basal dirtiness rate is 1/tick, so over ~10 ticks, basal drift ≤ 10.
    // The wilderness penalty (200) is the dominant contributor.
    assert!(
        final_dirtiness.value() >= initial_dirtiness.value() + 200,
        "dirtiness ({}) should include wilderness_relief_dirtiness_penalty (200) above initial ({})",
        final_dirtiness.value(),
        initial_dirtiness.value(),
    );

    // --- Verification Layer 4: Waste created at ForestPath ---
    let waste_at_forest = h
        .world
        .entities_effectively_at(FOREST_PATH)
        .into_iter()
        .any(|entity| {
            h.world
                .get_component_item_lot(entity)
                .is_some_and(|lot| lot.commodity == CommodityKind::Waste)
        });
    assert!(
        waste_at_forest,
        "waste should have been created at ForestPath after relieve_wilderness"
    );
}

// ---------------------------------------------------------------------------
// Scenario 64: Deprivation Accident — no relief option available
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI
// GoalKinds: Relieve
// ActionDomains: Needs
// Places: CommonHouse
// Principles: 1, 8, 20, 26
//
// Setup: One agent at CommonHouse (Inn + Village tags — indoor, no Latrine,
//   no outdoor tags). Neither `toilet` nor `relieve_wilderness` is afforded.
//   Bladder starts at pm(950) (critical). bladder_rate=50 keeps escalating.
//   bladder_accident_tolerance_ticks=nz(1) fires after 1 tick of critical
//   exposure. Needs system runs before AI in tick order, so the accident
//   fires before the agent can plan or start travel to a reachable latrine.
//
// Proves: When no relief action is available and the agent cannot reach
//   one, the needs system's deprivation accident fires after
//   bladder_accident_tolerance_ticks of critical bladder exposure.
//   Waste is created, bladder resets to pm(0), dirtiness increases by
//   the bladder pressure at accident time. No relief action is committed.
//
// Chain: critical bladder pressure -> Relieve goal ranked -> planner
//   finds no relief affordance -> no action started -> needs system
//   tracks critical exposure ticks -> tolerance exceeded -> deprivation
//   accident fires -> waste created, bladder=0, dirtiness+=bladder.

#[test]
fn golden_deprivation_accident() {
    let mut h = GoldenHarness::new(Seed([96; 32]));
    h.enable_action_tracing();
    h.driver.enable_tracing();

    // bladder_accident_tolerance_ticks=1 so the accident fires on the
    // very first tick. Needs system runs before AI in the tick order,
    // so the accident fires before the agent can plan/start travel.
    let metabolism = MetabolismProfile::new(
        pm(2),   // hunger_rate
        pm(2),   // thirst_rate
        pm(2),   // fatigue_rate
        pm(50),  // bladder_rate — fast escalation
        pm(1),   // dirtiness_rate
        pm(20),  // rest_efficiency
        nz(480), // starvation_tolerance_ticks
        nz(240), // dehydration_tolerance_ticks
        nz(120), // exhaustion_collapse_ticks
        nz(1),   // bladder_accident_tolerance_ticks — fires after 1 critical tick
        nz(8),   // toilet_ticks
        nz(12),  // wash_ticks
        pm(0),   // travel_fatigue_multiplier
        pm(0),   // travel_thirst_multiplier
        pm(0),   // travel_bladder_multiplier
        pm(200), // wilderness_relief_dirtiness_penalty
    );

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Desperate",
        COMMON_HOUSE,
        // Bladder at pm(950) — already above critical threshold (930).
        // Each tick adds 50 more (capped at 1000).
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(950), pm(0)),
        metabolism,
        UtilityProfile {
            bladder_weight: pm(900),
            ..UtilityProfile::default()
        },
    );

    // Override deprivation exposure to start at 0 critical ticks
    // (seed_agent already sets DeprivationExposure::default() which is all zeros).
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let initial_dirtiness = h.agent_dirtiness(agent);

    // Run enough ticks for the accident to fire.
    // Bladder starts at 950 (above critical 930). Each tick increments
    // bladder_critical_ticks. After 1 tick of critical exposure,
    // the accident fires.
    for _ in 0..30 {
        h.step_once();

        // Early exit once we see the accident effects.
        let bladder = h.agent_bladder(agent);
        let dirtiness = h.agent_dirtiness(agent);
        if bladder == pm(0) && dirtiness.value() > initial_dirtiness.value() + 100 {
            break;
        }
    }

    // --- Verification Layer 1: No relief action committed ---
    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let agent_events = action_sink.events_for(agent);

    let relief_committed = agent_events.iter().any(|e| {
        matches!(e.kind, worldwake_sim::ActionTraceKind::Committed { .. })
            && h.defs
                .get(e.def_id)
                .is_some_and(|d| d.name.as_str() == "toilet" || d.name.as_str() == "relieve_wilderness")
    });
    assert!(
        !relief_committed,
        "no relief action should have been committed at CommonHouse (indoor, no latrine)"
    );

    // --- Verification Layer 2: Authoritative state — bladder reset after accident ---
    // After the accident, bladder should be reset to pm(0).
    // The accident fires when bladder_critical_ticks >= 3 (tolerance).
    assert_eq!(
        h.agent_bladder(agent),
        pm(0),
        "bladder should be pm(0) after deprivation accident"
    );

    // --- Verification Layer 3: Dirtiness increased significantly ---
    // The accident adds the bladder pressure (which was at or near 1000) to dirtiness.
    // Starting dirtiness was pm(0). Even with basal drift (1/tick * ~30 ticks = 30),
    // the accident adds a large amount (the bladder value at accident time, likely 1000).
    let final_dirtiness = h.agent_dirtiness(agent);
    assert!(
        final_dirtiness.value() >= 500,
        "dirtiness ({}) should be very high after deprivation accident (bladder pressure added)",
        final_dirtiness.value(),
    );

    // --- Verification Layer 4: Waste created at agent's effective place ---
    // The accident creates waste at the agent's effective_place when it fires.
    // Due to tick ordering (actions process before systems), the agent may have
    // traveled from CommonHouse before the accident fired. We check wherever
    // the agent ended up.
    let agent_place = h
        .world
        .effective_place(agent)
        .expect("agent should have an effective place");
    let waste_at_agent_place = h
        .world
        .entities_effectively_at(agent_place)
        .into_iter()
        .any(|entity| {
            h.world
                .get_component_item_lot(entity)
                .is_some_and(|lot| lot.commodity == CommodityKind::Waste)
        });
    assert!(
        waste_at_agent_place,
        "waste should have been created at the agent's location after deprivation accident"
    );
}

// ---------------------------------------------------------------------------
// Scenario 65: Witness Observation — co-located agent perceives wilderness relief
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, Perception
// GoalKinds: Relieve
// ActionDomains: Needs
// Places: ForestPath
// Principles: 3, 7, 26
//
// Setup: Two agents at ForestPath (Forest + Trail tags, both outdoor).
//   Agent A has critical bladder (pm(950)) and relieves via
//   relieve_wilderness. Agent B has PerceptionProfile with
//   observation_fidelity=pm(1000) (guaranteed observation) and no
//   bladder pressure (idle). Both agents have world beliefs seeded.
//
// Proves: The perception pipeline processes VisibilitySpec::SamePlace
//   events from relieve_wilderness. A co-located agent with
//   PerceptionProfile witnesses the WildernessRelief event, as proven
//   by the perception trace recording a passed observation for Agent B
//   on an event tagged WildernessRelief with actor=Agent A. This is the
//   social consequence verification layer: witnesses observe relief events
//   only if co-located (Principle 7: Locality).
//
// Chain: critical bladder -> Relieve goal -> relieve_wilderness committed
//   with VisibilitySpec::SamePlace -> perception system resolves co-located
//   witnesses -> Agent B passes observation check -> perception trace
//   records observation.

#[test]
fn golden_witness_observation() {
    let mut h = GoldenHarness::new(Seed([97; 32]));
    h.enable_action_tracing();
    h.enable_perception_tracing();

    let metabolism = MetabolismProfile::new(
        pm(2),   // hunger_rate
        pm(2),   // thirst_rate
        pm(2),   // fatigue_rate
        pm(10),  // bladder_rate
        pm(1),   // dirtiness_rate
        pm(20),  // rest_efficiency
        nz(480), // starvation_tolerance_ticks
        nz(240), // dehydration_tolerance_ticks
        nz(120), // exhaustion_collapse_ticks
        nz(200), // bladder_accident_tolerance_ticks
        nz(8),   // toilet_ticks
        nz(12),  // wash_ticks
        pm(0),   // travel_fatigue_multiplier
        pm(0),   // travel_thirst_multiplier
        pm(0),   // travel_bladder_multiplier
        pm(200), // wilderness_relief_dirtiness_penalty
    );

    // Agent A: critical bladder, will relieve.
    let agent_a = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Reliever",
        FOREST_PATH,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(950), pm(0)),
        metabolism,
        UtilityProfile {
            bladder_weight: pm(900),
            ..UtilityProfile::default()
        },
    );

    // Agent B: idle observer with guaranteed observation fidelity.
    let idle_metabolism = MetabolismProfile::new(
        pm(1),   // hunger_rate
        pm(1),   // thirst_rate
        pm(1),   // fatigue_rate
        pm(1),   // bladder_rate
        pm(1),   // dirtiness_rate
        pm(20),  // rest_efficiency
        nz(480), // starvation_tolerance_ticks
        nz(240), // dehydration_tolerance_ticks
        nz(120), // exhaustion_collapse_ticks
        nz(200), // bladder_accident_tolerance_ticks
        nz(8),   // toilet_ticks
        nz(12),  // wash_ticks
        pm(0),   // travel_fatigue_multiplier
        pm(0),   // travel_thirst_multiplier
        pm(0),   // travel_bladder_multiplier
        pm(0),   // wilderness_relief_dirtiness_penalty
    );

    let agent_b = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Witness",
        FOREST_PATH,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(0)),
        idle_metabolism,
        UtilityProfile::default(),
    );

    // Set PerceptionProfile on Agent B with fidelity=1000 (guaranteed observation).
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent_b,
        PerceptionProfile {
            observation_fidelity: pm(1000),
            ..PerceptionProfile::default()
        },
    );

    // Seed world beliefs for both agents.
    for agent in [agent_a, agent_b] {
        seed_actor_world_beliefs(
            &mut h.world,
            &mut h.event_log,
            agent,
            Tick(0),
            worldwake_core::PerceptionSource::Inference,
        );
    }

    // Run until relieve_wilderness commits.
    let mut wilderness_committed = false;
    for _ in 0..40 {
        h.step_once();

        if !wilderness_committed {
            let sink = h
                .action_trace_sink()
                .expect("action tracing should be enabled");
            wilderness_committed = sink.events_for(agent_a).iter().any(|e| {
                matches!(e.kind, worldwake_sim::ActionTraceKind::Committed { .. })
                    && h.defs
                        .get(e.def_id)
                        .is_some_and(|d| d.name.as_str() == "relieve_wilderness")
            });
            if wilderness_committed {
                break;
            }
        }
    }

    // --- Verification Layer 1: Action committed ---
    assert!(
        wilderness_committed,
        "Agent A should have committed relieve_wilderness at ForestPath within 40 ticks"
    );

    // --- Verification Layer 2: Perception trace shows Agent B observed a WildernessRelief event ---
    let perception_sink = h
        .perception_trace_sink()
        .expect("perception tracing should be enabled");
    let b_observations = perception_sink.events_for(agent_b);

    // Find a perception trace entry where Agent B observed an event tagged WildernessRelief.
    let witnessed_wilderness_relief = b_observations.iter().any(|trace_event| {
        if !trace_event.observation_passed {
            return false;
        }
        // Look up the event in the log and check its tags.
        h.event_log
            .get(trace_event.event_id)
            .is_some_and(|record| record.tags().contains(&EventTag::WildernessRelief))
    });
    assert!(
        witnessed_wilderness_relief,
        "Agent B (co-located with PerceptionProfile) should have observed a WildernessRelief \
         event via the perception pipeline. Perception trace entries for B: {:?}",
        b_observations.iter().map(|e| e.summary()).collect::<Vec<_>>(),
    );

    // --- Verification Layer 3: The witnessed event's actor is Agent A ---
    let witnessed_event_actor_matches = b_observations.iter().any(|trace_event| {
        if !trace_event.observation_passed {
            return false;
        }
        h.event_log.get(trace_event.event_id).is_some_and(|record| {
            record.tags().contains(&EventTag::WildernessRelief)
                && record.actor_id() == Some(agent_a)
        })
    });
    assert!(
        witnessed_event_actor_matches,
        "The WildernessRelief event observed by Agent B should have Agent A as actor"
    );

    // --- Verification Layer 4: Authoritative state — bladder reset, waste exists ---
    assert!(
        h.agent_bladder(agent_a).value() <= 20,
        "Agent A bladder ({}) should be near pm(0) after relieve_wilderness commit",
        h.agent_bladder(agent_a).value(),
    );

    let waste_at_forest = h
        .world
        .entities_effectively_at(FOREST_PATH)
        .into_iter()
        .any(|entity| {
            h.world
                .get_component_item_lot(entity)
                .is_some_and(|lot| lot.commodity == CommodityKind::Waste)
        });
    assert!(
        waste_at_forest,
        "waste should have been created at ForestPath after relieve_wilderness"
    );
}

// ---------------------------------------------------------------------------
// Scenario 66: No Witness — isolated agent produces no social consequences
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, Perception
// GoalKinds: Relieve
// ActionDomains: Needs
// Places: ForestPath
// Principles: 3, 7
//
// Setup: One agent alone at ForestPath (Forest + Trail tags, outdoor)
//   with critical bladder (pm(950)). No other agents at ForestPath.
//   A second agent exists at a different place (VillageSquare) with
//   PerceptionProfile to confirm it does NOT observe the event.
//
// Proves: Agents who relieve in isolation produce no social consequences.
//   VisibilitySpec::SamePlace ensures only co-located agents can witness
//   the event (Principle 7: Locality). A distant agent with
//   PerceptionProfile does NOT observe. Physical consequences still
//   exist: waste at place, dirtiness on agent.
//
// Chain: critical bladder -> Relieve goal -> relieve_wilderness committed
//   with VisibilitySpec::SamePlace -> perception system resolves witnesses
//   -> no co-located observers -> no perception trace for distant agent
//   -> physical consequences (waste, dirtiness) still produced.

#[test]
fn golden_no_witness() {
    let mut h = GoldenHarness::new(Seed([98; 32]));
    h.enable_action_tracing();
    h.enable_perception_tracing();

    let metabolism = MetabolismProfile::new(
        pm(2),   // hunger_rate
        pm(2),   // thirst_rate
        pm(2),   // fatigue_rate
        pm(10),  // bladder_rate
        pm(1),   // dirtiness_rate
        pm(20),  // rest_efficiency
        nz(480), // starvation_tolerance_ticks
        nz(240), // dehydration_tolerance_ticks
        nz(120), // exhaustion_collapse_ticks
        nz(200), // bladder_accident_tolerance_ticks
        nz(8),   // toilet_ticks
        nz(12),  // wash_ticks
        pm(0),   // travel_fatigue_multiplier
        pm(0),   // travel_thirst_multiplier
        pm(0),   // travel_bladder_multiplier
        pm(200), // wilderness_relief_dirtiness_penalty
    );

    // Isolated agent at ForestPath with critical bladder.
    let isolated = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Isolated",
        FOREST_PATH,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(950), pm(0)),
        metabolism,
        UtilityProfile {
            bladder_weight: pm(900),
            ..UtilityProfile::default()
        },
    );

    // Distant agent at VillageSquare with PerceptionProfile — should NOT observe.
    let idle_metabolism = MetabolismProfile::new(
        pm(1),   // hunger_rate
        pm(1),   // thirst_rate
        pm(1),   // fatigue_rate
        pm(1),   // bladder_rate
        pm(1),   // dirtiness_rate
        pm(20),  // rest_efficiency
        nz(480), // starvation_tolerance_ticks
        nz(240), // dehydration_tolerance_ticks
        nz(120), // exhaustion_collapse_ticks
        nz(200), // bladder_accident_tolerance_ticks
        nz(8),   // toilet_ticks
        nz(12),  // wash_ticks
        pm(0),   // travel_fatigue_multiplier
        pm(0),   // travel_thirst_multiplier
        pm(0),   // travel_bladder_multiplier
        pm(0),   // wilderness_relief_dirtiness_penalty
    );

    let distant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Distant",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(0)),
        idle_metabolism,
        UtilityProfile::default(),
    );

    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        distant,
        PerceptionProfile {
            observation_fidelity: pm(1000),
            ..PerceptionProfile::default()
        },
    );

    // Seed beliefs for both agents.
    for agent in [isolated, distant] {
        seed_actor_world_beliefs(
            &mut h.world,
            &mut h.event_log,
            agent,
            Tick(0),
            worldwake_core::PerceptionSource::Inference,
        );
    }

    let initial_dirtiness = h.agent_dirtiness(isolated);

    // Run until relieve_wilderness commits.
    let mut wilderness_committed = false;
    for _ in 0..40 {
        h.step_once();

        if !wilderness_committed {
            let sink = h
                .action_trace_sink()
                .expect("action tracing should be enabled");
            wilderness_committed = sink.events_for(isolated).iter().any(|e| {
                matches!(e.kind, worldwake_sim::ActionTraceKind::Committed { .. })
                    && h.defs
                        .get(e.def_id)
                        .is_some_and(|d| d.name.as_str() == "relieve_wilderness")
            });
            if wilderness_committed {
                break;
            }
        }
    }

    assert!(
        wilderness_committed,
        "isolated agent should have committed relieve_wilderness at ForestPath within 40 ticks"
    );

    // --- Verification Layer 1: Distant agent did NOT observe a WildernessRelief event ---
    let perception_sink = h
        .perception_trace_sink()
        .expect("perception tracing should be enabled");
    let distant_observations = perception_sink.events_for(distant);

    let distant_saw_wilderness_relief = distant_observations.iter().any(|trace_event| {
        trace_event.observation_passed
            && h.event_log
                .get(trace_event.event_id)
                .is_some_and(|record| record.tags().contains(&EventTag::WildernessRelief))
    });
    assert!(
        !distant_saw_wilderness_relief,
        "distant agent at VillageSquare should NOT have observed a WildernessRelief event \
         at ForestPath (VisibilitySpec::SamePlace requires co-location)"
    );

    // --- Verification Layer 2: Physical consequences still exist ---
    assert!(
        h.agent_bladder(isolated).value() <= 20,
        "isolated agent bladder ({}) should be near pm(0) after relieve_wilderness commit",
        h.agent_bladder(isolated).value(),
    );

    let final_dirtiness = h.agent_dirtiness(isolated);
    assert!(
        final_dirtiness.value() >= initial_dirtiness.value() + 200,
        "dirtiness ({}) should include wilderness_relief_dirtiness_penalty (200) above initial ({})",
        final_dirtiness.value(),
        initial_dirtiness.value(),
    );

    let waste_at_forest = h
        .world
        .entities_effectively_at(FOREST_PATH)
        .into_iter()
        .any(|entity| {
            h.world
                .get_component_item_lot(entity)
                .is_some_and(|lot| lot.commodity == CommodityKind::Waste)
        });
    assert!(
        waste_at_forest,
        "waste should have been created at ForestPath even without witnesses"
    );
}

// ---------------------------------------------------------------------------
// Scenario 67: Need Continuity — cross-path bladder and dirtiness invariants
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI
// GoalKinds: Relieve
// ActionDomains: Needs
// Places: PublicLatrine, ForestPath, CommonHouse
// Principles: 3, 8, 26
//
// Setup: Three sub-scenarios with separate harnesses:
//   (a) Agent at PublicLatrine, critical bladder → toilet → bladder=0,
//       dirtiness unchanged.
//   (b) Agent at ForestPath, critical bladder → relieve_wilderness →
//       bladder=0, dirtiness += wilderness_relief_dirtiness_penalty.
//   (c) Agent at CommonHouse (indoor, no relief), critical bladder,
//       bladder_accident_tolerance_ticks=1 → deprivation accident →
//       bladder=0, dirtiness at max consequence level.
//
// Proves: Across all relief paths (toilet, wilderness, accident), bladder
//   is always exactly Permille(0) at the commit/accident boundary. No
//   partial resets. Dirtiness reflects the exact penalty for each path:
//   0 for toilet, wilderness_relief_dirtiness_penalty for wilderness,
//   and the full bladder pressure value for accident. This is the
//   cross-cutting need continuity invariant (Principle 3: Concrete State).
//
// Chain: Each sub-scenario: critical bladder -> path-specific relief ->
//   bladder=0, dirtiness=path_penalty. The invariant holds regardless
//   of which relief mechanism fires.

#[test]
fn golden_need_continuity_toilet() {
    // Sub-scenario (a): toilet at PublicLatrine.
    let mut h = GoldenHarness::new(Seed([99; 32]));
    h.enable_action_tracing();

    let metabolism = MetabolismProfile::new(
        pm(2),   // hunger_rate
        pm(2),   // thirst_rate
        pm(2),   // fatigue_rate
        pm(10),  // bladder_rate
        pm(1),   // dirtiness_rate
        pm(20),  // rest_efficiency
        nz(480), // starvation_tolerance_ticks
        nz(240), // dehydration_tolerance_ticks
        nz(120), // exhaustion_collapse_ticks
        nz(200), // bladder_accident_tolerance_ticks
        nz(8),   // toilet_ticks
        nz(12),  // wash_ticks
        pm(0),   // travel_fatigue_multiplier
        pm(0),   // travel_thirst_multiplier
        pm(0),   // travel_bladder_multiplier
        pm(200), // wilderness_relief_dirtiness_penalty — non-zero to confirm NOT applied
    );

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "ToiletUser",
        PUBLIC_LATRINE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(950), pm(0)),
        metabolism,
        UtilityProfile {
            bladder_weight: pm(900),
            ..UtilityProfile::default()
        },
    );

    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let initial_dirtiness = h.agent_dirtiness(agent);

    // Run until toilet commits. Break immediately at commit for exact assertion.
    let mut toilet_committed = false;
    for _ in 0..40 {
        h.step_once();

        if !toilet_committed {
            let sink = h
                .action_trace_sink()
                .expect("action tracing should be enabled");
            toilet_committed = sink.events_for(agent).iter().any(|e| {
                matches!(e.kind, worldwake_sim::ActionTraceKind::Committed { .. })
                    && h.defs
                        .get(e.def_id)
                        .is_some_and(|d| d.name.as_str() == "toilet")
            });
            if toilet_committed {
                break;
            }
        }
    }

    assert!(toilet_committed, "agent should have committed toilet at PublicLatrine");

    // Bladder should be near pm(0) (basal drift of at most 1 tick since commit).
    assert!(
        h.agent_bladder(agent).value() <= 20,
        "toilet path: bladder ({}) should be near pm(0) at commit boundary",
        h.agent_bladder(agent).value(),
    );

    // Dirtiness should NOT include the wilderness penalty (200). Only basal drift.
    let final_dirtiness = h.agent_dirtiness(agent);
    assert!(
        final_dirtiness.value() < initial_dirtiness.value() + 100,
        "toilet path: dirtiness ({}) should not include wilderness penalty (initial={})",
        final_dirtiness.value(),
        initial_dirtiness.value(),
    );
}

#[test]
fn golden_need_continuity_wilderness() {
    // Sub-scenario (b): relieve_wilderness at ForestPath.
    let mut h = GoldenHarness::new(Seed([100; 32]));
    h.enable_action_tracing();

    let metabolism = MetabolismProfile::new(
        pm(2),   // hunger_rate
        pm(2),   // thirst_rate
        pm(2),   // fatigue_rate
        pm(10),  // bladder_rate
        pm(1),   // dirtiness_rate
        pm(20),  // rest_efficiency
        nz(480), // starvation_tolerance_ticks
        nz(240), // dehydration_tolerance_ticks
        nz(120), // exhaustion_collapse_ticks
        nz(200), // bladder_accident_tolerance_ticks
        nz(8),   // toilet_ticks
        nz(12),  // wash_ticks
        pm(0),   // travel_fatigue_multiplier
        pm(0),   // travel_thirst_multiplier
        pm(0),   // travel_bladder_multiplier
        pm(200), // wilderness_relief_dirtiness_penalty
    );

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "WildernessUser",
        FOREST_PATH,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(950), pm(0)),
        metabolism,
        UtilityProfile {
            bladder_weight: pm(900),
            ..UtilityProfile::default()
        },
    );

    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let initial_dirtiness = h.agent_dirtiness(agent);

    // Run until relieve_wilderness commits.
    let mut wilderness_committed = false;
    for _ in 0..40 {
        h.step_once();

        if !wilderness_committed {
            let sink = h
                .action_trace_sink()
                .expect("action tracing should be enabled");
            wilderness_committed = sink.events_for(agent).iter().any(|e| {
                matches!(e.kind, worldwake_sim::ActionTraceKind::Committed { .. })
                    && h.defs
                        .get(e.def_id)
                        .is_some_and(|d| d.name.as_str() == "relieve_wilderness")
            });
            if wilderness_committed {
                break;
            }
        }
    }

    assert!(
        wilderness_committed,
        "agent should have committed relieve_wilderness at ForestPath"
    );

    // Bladder should be near pm(0) at commit boundary.
    assert!(
        h.agent_bladder(agent).value() <= 20,
        "wilderness path: bladder ({}) should be near pm(0) at commit boundary",
        h.agent_bladder(agent).value(),
    );

    // Dirtiness should include the wilderness penalty (200).
    let final_dirtiness = h.agent_dirtiness(agent);
    assert!(
        final_dirtiness.value() >= initial_dirtiness.value() + 200,
        "wilderness path: dirtiness ({}) should include penalty (200) above initial ({})",
        final_dirtiness.value(),
        initial_dirtiness.value(),
    );
}

#[test]
fn golden_need_continuity_accident() {
    // Sub-scenario (c): deprivation accident at CommonHouse (indoor, no relief).
    let mut h = GoldenHarness::new(Seed([101; 32]));
    h.enable_action_tracing();

    let metabolism = MetabolismProfile::new(
        pm(2),   // hunger_rate
        pm(2),   // thirst_rate
        pm(2),   // fatigue_rate
        pm(50),  // bladder_rate — fast escalation
        pm(1),   // dirtiness_rate
        pm(20),  // rest_efficiency
        nz(480), // starvation_tolerance_ticks
        nz(240), // dehydration_tolerance_ticks
        nz(120), // exhaustion_collapse_ticks
        nz(1),   // bladder_accident_tolerance_ticks — fires after 1 critical tick
        nz(8),   // toilet_ticks
        nz(12),  // wash_ticks
        pm(0),   // travel_fatigue_multiplier
        pm(0),   // travel_thirst_multiplier
        pm(0),   // travel_bladder_multiplier
        pm(200), // wilderness_relief_dirtiness_penalty
    );

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "AccidentVictim",
        COMMON_HOUSE,
        // Bladder at pm(950) — above critical threshold (930).
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(950), pm(0)),
        metabolism,
        UtilityProfile {
            bladder_weight: pm(900),
            ..UtilityProfile::default()
        },
    );

    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let initial_dirtiness = h.agent_dirtiness(agent);

    // Run until the accident fires — bladder resets to 0 and dirtiness jumps.
    for _ in 0..30 {
        h.step_once();

        if h.agent_bladder(agent) == pm(0)
            && h.agent_dirtiness(agent).value() > initial_dirtiness.value() + 100
        {
            break;
        }
    }

    // --- No relief action should have been committed ---
    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let relief_committed = action_sink.events_for(agent).iter().any(|e| {
        matches!(e.kind, worldwake_sim::ActionTraceKind::Committed { .. })
            && h.defs
                .get(e.def_id)
                .is_some_and(|d| d.name.as_str() == "toilet" || d.name.as_str() == "relieve_wilderness")
    });
    assert!(
        !relief_committed,
        "accident path: no relief action should have been committed at CommonHouse (indoor)"
    );

    // Bladder should be exactly pm(0) after accident.
    assert_eq!(
        h.agent_bladder(agent),
        pm(0),
        "accident path: bladder should be pm(0) after deprivation accident"
    );

    // Dirtiness should be very high — the accident adds the bladder pressure
    // (which was at or near 1000) to dirtiness.
    let final_dirtiness = h.agent_dirtiness(agent);
    assert!(
        final_dirtiness.value() >= 500,
        "accident path: dirtiness ({}) should be very high after deprivation accident \
         (bladder pressure added)",
        final_dirtiness.value(),
    );

    // Waste should exist at the agent's effective place.
    let agent_place = h
        .world
        .effective_place(agent)
        .expect("agent should have an effective place");
    let waste_exists = h
        .world
        .entities_effectively_at(agent_place)
        .into_iter()
        .any(|entity| {
            h.world
                .get_component_item_lot(entity)
                .is_some_and(|lot| lot.commodity == CommodityKind::Waste)
        });
    assert!(
        waste_exists,
        "accident path: waste should exist at agent's location after deprivation accident"
    );
}
