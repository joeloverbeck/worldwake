//! Golden tests covering observer-identified simulation gaps.

mod golden_harness;

use std::collections::BTreeSet;

use golden_harness::*;
use worldwake_ai::{
    CommodityPurpose, DecisionOutcome, GoalKey, GoalKind, PlannerOpKind, SelectedPlanSource,
};
use worldwake_core::{
    BeliefConfidencePolicy, CognitiveProfile, CommodityKind, ComponentKind, ComponentValue,
    DeathCause, EntityId, EventTag, EventView, HomeostaticNeedId, HomeostaticNeeds, KnownRecipes,
    MetabolismProfile, PerceptionProfile, Quantity, ResourceSource, Seed, StateHash, Tick,
    UtilityProfile, WorkstationTag, hash_event_log, hash_world, verify_authoritative_conservation,
    verify_live_lot_conservation,
};
use worldwake_sim::ActionTraceKind;

fn remote_resource_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        entity_activation_threshold: pm(125),
        claim_confidence_threshold: pm(50),
        observation_buffer_capacity: 64,
        need_salience_boost: pm(500),
        need_salience_urgency_threshold: pm(500),
        observation_fidelity: pm(875),
        confidence_policy: BeliefConfidencePolicy::default(),
        institutional_memory_capacity: 20,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
    }
}

fn configure_remote_resource_agent(h: &mut GoldenHarness, agent: EntityId) {
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        remote_resource_perception_profile(),
    );
    set_agent_cognitive_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        CognitiveProfile::default(),
    );
}

fn place_ground_commodity(
    h: &mut GoldenHarness,
    place: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
) -> EntityId {
    let mut txn = new_txn(&mut h.world, 0);
    let lot = txn.create_item_lot(commodity, quantity).unwrap();
    txn.set_ground_location(lot, place).unwrap();
    commit_txn(txn, &mut h.event_log);
    lot
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RemoteTravelObservation {
    reached_orchard_farm: bool,
    ate_or_drank: bool,
    left_origin_tick: Option<u32>,
    stationary_origin_ticks: u32,
    committed_actions: BTreeSet<String>,
}

fn run_remote_travel_when_local_supply_exhausted(
    seed: Seed,
) -> (RemoteTravelObservation, StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.enable_action_tracing();

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "RemoteTraveler",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(500), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile {
            hunger_weight: pm(950),
            thirst_weight: pm(400),
            ..UtilityProfile::default()
        },
    );
    configure_remote_resource_agent(&mut h, agent);

    let orchard_source = place_workstation_with_source(
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

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        ORCHARD_FARM,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        orchard_source,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let mut observation = RemoteTravelObservation {
        reached_orchard_farm: false,
        ate_or_drank: false,
        left_origin_tick: None,
        stationary_origin_ticks: 0,
        committed_actions: BTreeSet::new(),
    };
    let mut seen_events = 0usize;

    for tick in 0..300u32 {
        h.step_once();

        if h.world.is_in_transit(agent) || h.world.effective_place(agent) != Some(VILLAGE_SQUARE) {
            observation.left_origin_tick.get_or_insert(tick);
        } else {
            observation.stationary_origin_ticks += 1;
        }

        if h.world.effective_place(agent) == Some(ORCHARD_FARM) {
            observation.reached_orchard_farm = true;
        }

        let events = h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(agent);
        for event in &events[seen_events..] {
            if matches!(event.kind, ActionTraceKind::Committed { .. })
                && let Some(def) = h.defs.get(event.def_id)
            {
                observation.committed_actions.insert(def.name.clone());
                if matches!(def.name.as_str(), "eat" | "drink") {
                    observation.ate_or_drank = true;
                }
            }
        }
        seen_events = events.len();

        if observation.reached_orchard_farm && observation.ate_or_drank {
            break;
        }
    }

    verify_live_lot_conservation(&h.world, CommodityKind::Apple, 1).unwrap();
    verify_authoritative_conservation(&h.world, CommodityKind::Apple, 9).unwrap();

    assert!(
        observation.reached_orchard_farm,
        "Agent should reach Orchard Farm when no local food exists; left_origin_tick={:?}, committed_actions={:?}",
        observation.left_origin_tick, observation.committed_actions
    );
    assert!(
        observation.ate_or_drank,
        "Agent should eat or drink after reaching remote resources; committed_actions={:?}",
        observation.committed_actions
    );
    assert!(
        observation.left_origin_tick.is_some_and(|tick| tick < 200),
        "Agent should not remain at VillageSquare for 200+ ticks under local scarcity; left_origin_tick={:?}, stationary_origin_ticks={}",
        observation.left_origin_tick,
        observation.stationary_origin_ticks
    );

    (
        observation,
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct IdleScarcityObservation {
    max_consecutive_idle: u32,
    committed_actions: BTreeSet<String>,
    reached_remote_resource_place: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AgentConvergenceObservation {
    name: String,
    max_consecutive_idle: u32,
    committed_actions: BTreeSet<String>,
    reached_remote_resource_place: bool,
    left_origin_tick: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MultiAgentConvergenceObservation {
    agents: Vec<AgentConvergenceObservation>,
    earliest_travel_start_tick: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DeathTraceabilityObservation {
    death_tick: Tick,
    death_cause: DeathCause,
    saw_death_event: bool,
    saw_post_death_dead_decision: bool,
    post_death_started_actions: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HarvestToConsumeObservation {
    water_selected_harvest_plan: bool,
    water_committed_actions: BTreeSet<String>,
    water_thirst_before: u16,
    water_thirst_after: u16,
    apple_selected_harvest_plan: bool,
    apple_committed_actions: BTreeSet<String>,
    apple_hunger_before: u16,
    apple_hunger_after: u16,
}

fn run_max_idle_under_remote_resource_scarcity(
    seed: Seed,
) -> (IdleScarcityObservation, StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.enable_action_tracing();

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "ScarcitySurvivor",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(500), pm(500), pm(500), pm(500), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile {
            hunger_weight: pm(800),
            thirst_weight: pm(800),
            fatigue_weight: pm(700),
            bladder_weight: pm(700),
            ..UtilityProfile::default()
        },
    );
    configure_remote_resource_agent(&mut h, agent);

    let orchard_source = place_workstation_with_source(
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
    let remote_water =
        place_ground_commodity(&mut h, ORCHARD_FARM, CommodityKind::Water, Quantity(2));

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );
    for entity in [ORCHARD_FARM, orchard_source, remote_water] {
        seed_belief_from_world(
            &mut h.world,
            &mut h.event_log,
            agent,
            entity,
            Tick(0),
            worldwake_core::PerceptionSource::Inference,
        );
    }

    let mut observation = IdleScarcityObservation {
        max_consecutive_idle: 0,
        committed_actions: BTreeSet::new(),
        reached_remote_resource_place: false,
    };
    let mut consecutive_idle = 0u32;
    let mut seen_events = 0usize;

    for _ in 0..300u32 {
        h.step_once();

        if h.world.effective_place(agent) == Some(ORCHARD_FARM) {
            observation.reached_remote_resource_place = true;
        }

        let events = h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(agent);
        let new_events = &events[seen_events..];
        let mut had_lifecycle_activity = false;
        for event in new_events {
            if let Some(def) = h.defs.get(event.def_id)
                && matches!(
                    event.kind,
                    ActionTraceKind::Started { .. }
                        | ActionTraceKind::Committed { .. }
                        | ActionTraceKind::Aborted { .. }
                        | ActionTraceKind::StartFailed { .. }
                )
            {
                had_lifecycle_activity = true;
                if matches!(event.kind, ActionTraceKind::Committed { .. }) {
                    observation.committed_actions.insert(def.name.clone());
                }
            }
        }
        seen_events = events.len();

        if had_lifecycle_activity || h.agent_has_active_action(agent) {
            consecutive_idle = 0;
        } else {
            consecutive_idle += 1;
            observation.max_consecutive_idle =
                observation.max_consecutive_idle.max(consecutive_idle);
        }
    }

    assert!(
        observation.max_consecutive_idle < 100,
        "Agent should not idle for 100+ consecutive ticks under remote resource scarcity; max_idle={}, committed_actions={:?}, reached_remote_resource_place={}",
        observation.max_consecutive_idle,
        observation.committed_actions,
        observation.reached_remote_resource_place
    );
    assert!(
        !observation.committed_actions.is_empty() || observation.reached_remote_resource_place,
        "Agent should eventually act under remote resource scarcity; max_idle={}",
        observation.max_consecutive_idle
    );

    (
        observation,
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Scenario 126: Remote Travel To Resource Under Local Scarcity
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, Travel, Production
// GoalKinds: AcquireCommodity, ConsumeOwnedCommodity
// ActionDomains: Travel, Needs, Production
// Places: VillageSquare, OrchardFarm
// Principles: 7, 14, 20
//
// Setup: One agent at VillageSquare with critical hunger and no local food.
//   OrchardFarm has an apple source, and the agent starts with seeded beliefs
//   about both the place and the remote source.
//
// Proves: The agent leaves the barren local start, reaches OrchardFarm, and
//   commits eat/drink within the tick budget instead of stalling in local
//   sleep/relieve loops.
//
// Chain: hunger pressure -> remote resource candidate generation -> travel
//   plan -> OrchardFarm arrival -> remote acquisition -> consumption.

#[test]
fn golden_remote_travel_when_local_supply_exhausted() {
    let _ = run_remote_travel_when_local_supply_exhausted(Seed([176; 32]));
}

// ---------------------------------------------------------------------------
// Scenario 127: Idle Cap Under Remote Resource Scarcity
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, Travel, Production
// GoalKinds: Sleep, Relieve, AcquireCommodity, ConsumeOwnedCommodity
// ActionDomains: Travel, Needs, Production
// Places: VillageSquare, OrchardFarm
// Principles: 8, 20, 22
//
// Setup: One agent at VillageSquare with moderate hunger, thirst, fatigue,
//   and bladder pressure. Local food and water are absent; OrchardFarm holds
//   both apples and remote water, while local self-care affordances remain
//   lawful. The agent starts with seeded beliefs about the remote resources.
//
// Proves: Under multi-need scarcity, the agent remains behaviorally active
//   instead of entering a prolonged idle streak.
//
// Chain: multiple active needs -> candidate generation over remote resources
//   plus local self-care -> planner fallback/execution -> bounded idle streak.

#[test]
fn golden_max_idle_under_remote_resource_scarcity() {
    let _ = run_max_idle_under_remote_resource_scarcity(Seed([177; 32]));
}

fn run_multi_agent_convergence(
    seed: Seed,
) -> (MultiAgentConvergenceObservation, StateHash, StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, build_multi_recipe_registry());
    h.enable_action_tracing();

    let apple_recipe = h
        .recipes
        .recipe_by_name("Harvest Apples")
        .map(|(id, _)| id)
        .unwrap();
    let water_recipe = h
        .recipes
        .recipe_by_name("Harvest Water")
        .map(|(id, _)| id)
        .unwrap();
    let known_recipes = worldwake_core::KnownRecipes::with([apple_recipe, water_recipe]);

    let agents = vec![
        (
            "Scout Alder",
            HomeostaticNeeds::new(pm(650), pm(600), pm(250), pm(150), pm(0)),
            UtilityProfile {
                hunger_weight: pm(900),
                thirst_weight: pm(875),
                fatigue_weight: pm(500),
                bladder_weight: pm(400),
                ..UtilityProfile::default()
            },
        ),
        (
            "Scout Brim",
            HomeostaticNeeds::new(pm(600), pm(650), pm(200), pm(200), pm(0)),
            UtilityProfile {
                hunger_weight: pm(860),
                thirst_weight: pm(920),
                fatigue_weight: pm(450),
                bladder_weight: pm(450),
                ..UtilityProfile::default()
            },
        ),
        (
            "Scout Cinder",
            HomeostaticNeeds::new(pm(625), pm(625), pm(225), pm(175), pm(0)),
            UtilityProfile {
                hunger_weight: pm(890),
                thirst_weight: pm(890),
                fatigue_weight: pm(475),
                bladder_weight: pm(425),
                ..UtilityProfile::default()
            },
        ),
    ]
    .into_iter()
    .map(|(name, needs, utility)| {
        let agent = seed_agent_with_recipes(
            &mut h.world,
            &mut h.event_log,
            name,
            VILLAGE_SQUARE,
            needs,
            MetabolismProfile::default(),
            utility,
            known_recipes.clone(),
        );
        configure_remote_resource_agent(&mut h, agent);
        seed_actor_local_beliefs(
            &mut h.world,
            &mut h.event_log,
            agent,
            Tick(0),
            worldwake_core::PerceptionSource::DirectObservation,
        );
        (name.to_string(), agent)
    })
    .collect::<Vec<_>>();

    let orchard_source = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(12),
            max_quantity: Quantity(12),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );
    let well_source = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::Well,
        ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(12),
            max_quantity: Quantity(12),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );

    for (_, agent) in &agents {
        for entity in [ORCHARD_FARM, orchard_source, well_source] {
            seed_belief_from_world(
                &mut h.world,
                &mut h.event_log,
                *agent,
                entity,
                Tick(0),
                worldwake_core::PerceptionSource::Inference,
            );
        }
    }

    let mut observation = MultiAgentConvergenceObservation {
        agents: agents
            .iter()
            .map(|(name, _)| AgentConvergenceObservation {
                name: name.clone(),
                max_consecutive_idle: 0,
                committed_actions: BTreeSet::new(),
                reached_remote_resource_place: false,
                left_origin_tick: None,
            })
            .collect(),
        earliest_travel_start_tick: None,
    };
    let mut consecutive_idle = vec![0u32; agents.len()];
    let mut seen_events = vec![0usize; agents.len()];

    for tick in 0..600u32 {
        h.step_once();
        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");

        for (index, (_, agent)) in agents.iter().enumerate() {
            let agent_observation = &mut observation.agents[index];
            if h.world.is_in_transit(*agent)
                || h.world.effective_place(*agent) != Some(VILLAGE_SQUARE)
            {
                agent_observation.left_origin_tick.get_or_insert(tick);
            }
            if h.world.effective_place(*agent) == Some(ORCHARD_FARM) {
                agent_observation.reached_remote_resource_place = true;
            }

            let events = action_sink.events_for(*agent);
            let new_events = &events[seen_events[index]..];
            let mut had_lifecycle_activity = false;
            for event in new_events {
                if matches!(
                    event.kind,
                    ActionTraceKind::Started { .. }
                        | ActionTraceKind::Committed { .. }
                        | ActionTraceKind::Aborted { .. }
                        | ActionTraceKind::StartFailed { .. }
                ) {
                    had_lifecycle_activity = true;
                }
                if matches!(event.kind, ActionTraceKind::Started { .. })
                    && event.action_name == "travel"
                {
                    observation.earliest_travel_start_tick.get_or_insert(tick);
                }
                if matches!(event.kind, ActionTraceKind::Committed { .. }) {
                    agent_observation
                        .committed_actions
                        .insert(event.action_name.clone());
                }
            }
            seen_events[index] = events.len();

            if had_lifecycle_activity || h.agent_has_active_action(*agent) {
                consecutive_idle[index] = 0;
            } else {
                consecutive_idle[index] += 1;
                agent_observation.max_consecutive_idle = agent_observation
                    .max_consecutive_idle
                    .max(consecutive_idle[index]);
            }
        }
    }

    assert!(
        observation
            .agents
            .iter()
            .all(|agent| agent.max_consecutive_idle <= 200),
        "No agent should exceed 200 consecutive ticks with no lifecycle activity; observation={observation:?}"
    );
    assert!(
        observation
            .earliest_travel_start_tick
            .is_some_and(|tick| tick < 300),
        "At least one agent should start travel within 300 ticks; observation={observation:?}"
    );
    assert!(
        observation
            .agents
            .iter()
            .any(|agent| agent.reached_remote_resource_place),
        "At least one agent should reach Orchard Farm by tick 600; observation={observation:?}"
    );

    (
        observation,
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Scenario 130: Multi-Agent Convergence Under Remote Resource Scarcity
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, Travel, Production
// GoalKinds: AcquireCommodity, ConsumeOwnedCommodity, Sleep, Relieve
// ActionDomains: Travel, Production, Needs
// Places: VillageSquare, OrchardFarm
// Principles: 7, 8, 20
//
// Setup: Three agents begin at VillageSquare with elevated hunger and thirst,
//   but no local food or water. OrchardFarm holds both apple and water harvest
//   sources, and each agent starts with explicit beliefs about the remote place
//   and both resource carriers plus the needed harvest recipes.
//
// Proves: multi-agent scarcity pressure still produces bounded behavioral
//   progress instead of collapsing into prolonged no-op stretches; at least one
//   agent starts travel toward the remote resources and reaches OrchardFarm.
//
// Chain: shared scarcity pressure -> remote resource candidate generation
//   across multiple agents -> lawful travel start -> OrchardFarm arrival.

#[test]
fn golden_multi_agent_convergence() {
    let _ = run_multi_agent_convergence(Seed([178; 32]));
}

fn starvation_traceability_metabolism() -> MetabolismProfile {
    MetabolismProfile {
        starvation_tolerance_ticks: nz(2),
        ..MetabolismProfile::default()
    }
}

fn run_death_traceability(seed: Seed) -> (DeathTraceabilityObservation, StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "DeprivationVictim",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(950), pm(0), pm(0), pm(0), pm(0)),
        starvation_traceability_metabolism(),
        UtilityProfile {
            hunger_weight: pm(950),
            thirst_weight: pm(200),
            ..UtilityProfile::default()
        },
        KnownRecipes::default(),
    );
    configure_remote_resource_agent(&mut h, agent);

    let death_tick = loop {
        h.step_once();
        if let Some(dead_at) = h.world.get_component_dead_at(agent) {
            break dead_at.tick;
        }
        assert!(
            h.scheduler.current_tick().0 < 600,
            "Agent should die from starvation before tick 600; hunger={}, wound_load={}",
            h.agent_hunger(agent).value(),
            h.agent_wound_load(agent),
        );
    };

    for _ in 0..5 {
        h.step_once();
    }

    let dead_at = *h
        .world
        .get_component_dead_at(agent)
        .expect("DeadAt should remain set after death");
    let death_event_id =
        first_tagged_event_id_matching(&h.event_log, EventTag::Death, |_, record| {
            record.target_ids().contains(&agent)
                && event_sets_component(record, agent, ComponentKind::DeadAt, |value| {
                    matches!(
                        value,
                        ComponentValue::DeadAt(dead_at)
                            if dead_at.tick == death_tick
                                && matches!(
                                    dead_at.cause,
                                    DeathCause::NeedDeprivation {
                                        need: HomeostaticNeedId::Hunger
                                    }
                                )
                    )
                })
        });

    let saw_post_death_dead_decision = h.driver.trace_sink().is_some_and(|sink| {
        ((death_tick.0 + 1)..=h.scheduler.current_tick().0).any(|tick| {
            sink.trace_at(agent, Tick(tick))
                .is_some_and(|trace| matches!(trace.outcome, DecisionOutcome::Dead))
        })
    });
    let post_death_started_actions = h
        .action_trace_sink()
        .map(|sink| {
            sink.events_for(agent)
                .iter()
                .filter(|event| {
                    event.tick.0 > death_tick.0
                        && matches!(event.kind, ActionTraceKind::Started { .. })
                })
                .map(|event| event.action_name.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let observation = DeathTraceabilityObservation {
        death_tick,
        death_cause: dead_at.cause,
        saw_death_event: death_event_id.is_some(),
        saw_post_death_dead_decision,
        post_death_started_actions,
    };

    assert_eq!(
        dead_at.cause,
        DeathCause::NeedDeprivation {
            need: HomeostaticNeedId::Hunger
        },
        "Scenario setup should make starvation the deterministic death cause; observation={observation:?}"
    );
    assert!(
        observation.saw_death_event,
        "Death event tagged EventTag::Death should target the dead agent and persist DeadAt; observation={observation:?}"
    );
    assert!(
        observation.saw_post_death_dead_decision,
        "At least one post-death AI tick should record DecisionOutcome::Dead; observation={observation:?}"
    );
    assert!(
        observation.post_death_started_actions.is_empty(),
        "Dead agent should not start actions after death; observation={observation:?}"
    );

    (
        observation,
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

fn selected_harvest_plan_within_ticks(
    h: &GoldenHarness,
    agent: EntityId,
    commodity: CommodityKind,
    max_tick: u32,
) -> bool {
    let expected_goal = GoalKey::from(GoalKind::AcquireCommodity {
        commodity,
        purpose: CommodityPurpose::SelfConsume,
    });

    (0..=max_tick).any(|tick| {
        h.driver
            .trace_sink()
            .and_then(|sink| sink.trace_at(agent, Tick(tick.into())))
            .is_some_and(|trace| match &trace.outcome {
                DecisionOutcome::Planning(planning) => {
                    planning.selection.selected_goal_is(expected_goal)
                        && planning.selection.selected_plan_source
                            == Some(SelectedPlanSource::SearchSelection)
                        && planning
                            .selection
                            .selected_plan
                            .as_ref()
                            .and_then(|plan| plan.next_step.as_ref())
                            .is_some_and(|step| step.op_kind == PlannerOpKind::Harvest)
                }
                _ => false,
            })
    })
}

fn committed_action_names(h: &GoldenHarness, agent: EntityId) -> BTreeSet<String> {
    h.action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for(agent)
        .iter()
        .filter(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
        .map(|event| event.action_name.clone())
        .collect()
}

fn action_lifecycle_summaries(h: &GoldenHarness, agent: EntityId) -> Vec<String> {
    h.action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for(agent)
        .iter()
        .map(|event| {
            format!(
                "tick {} {:?} {}",
                event.tick.0, event.kind, event.action_name
            )
        })
        .collect()
}

fn run_harvest_to_consume(seed: Seed) -> (HarvestToConsumeObservation, StateHash, StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, build_multi_recipe_registry());
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let apple_recipe = h
        .recipes
        .recipe_by_name("Harvest Apples")
        .map(|(id, _)| id)
        .expect("multi-recipe registry should include Harvest Apples");
    let water_recipe = h
        .recipes
        .recipe_by_name("Harvest Water")
        .map(|(id, _)| id)
        .expect("multi-recipe registry should include Harvest Water");

    let water_agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Wellwatch",
        ORCHARD_FARM,
        HomeostaticNeeds::new(pm(0), pm(900), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile {
            hunger_weight: pm(200),
            thirst_weight: pm(950),
            ..UtilityProfile::default()
        },
        KnownRecipes::with([water_recipe]),
    );
    let apple_agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Rowkeeper",
        ORCHARD_FARM,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile {
            hunger_weight: pm(950),
            thirst_weight: pm(200),
            ..UtilityProfile::default()
        },
        KnownRecipes::with([apple_recipe]),
    );
    for agent in [water_agent, apple_agent] {
        configure_remote_resource_agent(&mut h, agent);
    }

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
    let _well_source = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::Well,
        ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(10),
            max_quantity: Quantity(10),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );

    for agent in [water_agent, apple_agent] {
        seed_actor_local_beliefs(
            &mut h.world,
            &mut h.event_log,
            agent,
            Tick(0),
            worldwake_core::PerceptionSource::DirectObservation,
        );
    }

    let water_thirst_before = h.agent_thirst(water_agent).value();
    let apple_hunger_before = h.agent_hunger(apple_agent).value();

    for _ in 0..100u32 {
        h.step_once();
    }

    let observation = HarvestToConsumeObservation {
        water_selected_harvest_plan: selected_harvest_plan_within_ticks(
            &h,
            water_agent,
            CommodityKind::Water,
            8,
        ),
        water_committed_actions: committed_action_names(&h, water_agent),
        water_thirst_before,
        water_thirst_after: h.agent_thirst(water_agent).value(),
        apple_selected_harvest_plan: selected_harvest_plan_within_ticks(
            &h,
            apple_agent,
            CommodityKind::Apple,
            8,
        ),
        apple_committed_actions: committed_action_names(&h, apple_agent),
        apple_hunger_before,
        apple_hunger_after: h.agent_hunger(apple_agent).value(),
    };

    assert!(
        observation.water_selected_harvest_plan,
        "water agent should select an opening self-consume harvest plan with Harvest as the next op; observation={observation:?}"
    );
    assert!(
        observation
            .water_committed_actions
            .contains("harvest:Harvest Water"),
        "water agent should commit Harvest Water; observation={observation:?}; lifecycle={:?}",
        action_lifecycle_summaries(&h, water_agent)
    );
    assert!(
        observation.water_committed_actions.contains("drink"),
        "water agent should commit drink after harvesting water; observation={observation:?}"
    );
    assert!(
        observation.water_thirst_after < observation.water_thirst_before,
        "water agent thirst should decrease after the harvest-to-drink chain; observation={observation:?}"
    );
    assert!(
        observation.apple_selected_harvest_plan,
        "apple agent should select an opening self-consume harvest plan with Harvest as the next op; observation={observation:?}"
    );
    assert!(
        observation
            .apple_committed_actions
            .contains("harvest:Harvest Apples"),
        "apple agent should commit Harvest Apples; observation={observation:?}"
    );
    assert!(
        observation.apple_committed_actions.contains("eat"),
        "apple agent should commit eat after harvesting apples; observation={observation:?}"
    );
    assert!(
        observation.apple_hunger_after < observation.apple_hunger_before,
        "apple agent hunger should decrease after the harvest-to-eat chain; observation={observation:?}"
    );

    (
        observation,
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Scenario 131: Death Traceability Under Unmet Needs
// ---------------------------------------------------------------------------
//
// Systems: Needs, Wounds, AI
// GoalKinds: ConsumeOwnedCommodity, Sleep, Relieve
// ActionDomains: Needs
// Places: VillageSquare
// Principles: 4, 10, 20
//
// Setup: One agent begins at VillageSquare with critical hunger, no local food
//   or water, no remote resource beliefs, and no recipe knowledge. A shortened
//   starvation tolerance makes deprivation wounds accumulate to a fatal wound
//   load within the scenario budget.
//
// Proves: deprivation death becomes an explicit durable world-state transition
//   with a traceable cause and death-tagged event, and the AI enters the dead
//   decision path instead of starting new actions after death.
//
// Chain: sustained unmet hunger -> deprivation wound creation -> fatal wound
//   load -> DeadAt with cause plus EventTag::Death -> DecisionOutcome::Dead.

#[test]
fn golden_death_traceability() {
    let _ = run_death_traceability(Seed([179; 32]));
}

// ---------------------------------------------------------------------------
// Scenario 132: Harvest-To-Consume Chain At Resource Source Locations
// ---------------------------------------------------------------------------
//
// Systems: Production, Needs, AI
// GoalKinds: AcquireCommodity, ConsumeOwnedCommodity
// ActionDomains: Production, Needs
// Places: OrchardFarm
// Principles: 3, 8, 26
//
// Setup: Two agents begin co-located with an orchard row and a well at
//   OrchardFarm. One knows only Harvest Water and starts critically thirsty;
//   the other knows only Harvest Apples and starts critically hungry. Direct
//   local beliefs are seeded so the scenario isolates harvest-to-consume
//   execution rather than perception/discovery lag.
//
// Proves: The opening AI plan selects the self-consume harvest branch for each
//   agent, then the full colocated chain completes: harvest -> possession/
//   materialization -> drink/eat -> corresponding need decreases.
//
// Chain: local resource-source beliefs + recipe knowledge -> harvest plan ->
//   committed harvest action -> committed consume action -> lower thirst/hunger.

#[test]
fn golden_harvest_to_consume() {
    let _ = run_harvest_to_consume(Seed([180; 32]));
}
