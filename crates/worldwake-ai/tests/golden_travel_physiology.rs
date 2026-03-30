//! Golden tests for travel physiology: per-agent body cost multipliers
//! during travel and their interaction with the AI decision pipeline.
//!
//! These tests prove that travel exertion multipliers on `MetabolismProfile`
//! produce observable need escalation, trigger interrupts at critical thresholds,
//! and create diversity between agents with different profiles.

mod golden_harness;

use golden_harness::*;
use worldwake_core::{
    prototype_place_entity, CommodityKind, HomeostaticNeeds, MetabolismProfile, PrototypePlace,
    Quantity, ResourceSource, Seed, Tick, UtilityProfile, WorkstationTag,
};

// ---------------------------------------------------------------------------
// Place constants
// ---------------------------------------------------------------------------

const EAST_FIELD_TRAIL: worldwake_core::EntityId =
    prototype_place_entity(PrototypePlace::EastFieldTrail);

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
// Scenario 1: Travel Need Escalation
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
// Scenario 2: Critical Bladder Local Relief
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
// Scenario 3: Agent Diversity in Travel Escalation
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
// Scenario 4: Travel Interrupt from Bladder Escalation
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
                .map(|d| d.name.as_str() == "toilet" || d.name.as_str() == "relieve_wilderness")
                .unwrap_or(false)
    });
    assert!(
        relief_committed,
        "agent should have committed a relief action (toilet or relieve_wilderness)"
    );
}
