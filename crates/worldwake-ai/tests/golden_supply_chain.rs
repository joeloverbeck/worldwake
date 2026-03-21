//! Golden E2E tests for the multi-role supply chain (S02c).
//!
//! These tests exercise the supply chain in two proven segments:
//!
//! 1. **Merchant restock cycle**: merchant travels to orchard, harvests apples,
//!    returns to General Store with stock. Validates multi-hop travel + production
//!    with decision trace assertions.
//!
//! 2. **Consumer co-located trade**: consumer trades with a merchant who already
//!    has apples, then eats to reduce hunger. Validates trade + consumption with
//!    decision trace assertions.
//!
//! The full end-to-end chain (restock → trade → consumption in one simulation)
//! is blocked on `specs/S10-bilateral-trade-negotiation.md`: the trade system
//! lacks price negotiation, so the consumer's fixed 1-coin offer is rejected by
//! the enterprise-focused merchant. S09 (A* heuristic + travel pruning) resolved
//! the earlier plan search budget exhaustion; perception (E14) correctly updates
//! beliefs. The remaining blocker is purely trade valuation mechanics.

mod golden_harness;

use golden_harness::*;
use std::collections::BTreeSet;
use worldwake_ai::{
    AgentTickDriver, CandidateEvidenceExclusionReason, CandidateEvidenceKind, DecisionOutcome,
    PlannerOpKind, PlanningBudget, SelectedPlanReplacementKind, SelectedPlanSource,
};
use worldwake_core::{
    hash_event_log, hash_world, prototype_place_entity, total_authoritative_commodity_quantity,
    total_live_lot_quantity, verify_authoritative_conservation, verify_live_lot_conservation,
    BeliefConfidencePolicy, BodyCostPerTick, CommodityKind, DemandMemory, DemandObservation,
    DemandObservationReason, GoalKind, HomeostaticNeeds, KnownRecipes, MerchandiseProfile,
    MetabolismProfile, PerceptionProfile, PerceptionSource, PrototypePlace, Quantity,
    ResourceSource, Seed, StateHash, Tick, TradeDispositionProfile, UtilityProfile, WorkstationTag,
};
use worldwake_sim::{ActionTraceKind, RecipeDefinition, RecipeRegistry};

fn default_trade_disposition() -> TradeDispositionProfile {
    TradeDispositionProfile {
        negotiation_round_ticks: nz(4),
        initial_offer_bias: pm(500),
        concession_rate: pm(100),
        demand_memory_retention_ticks: 48,
    }
}

fn enterprise_trade_disposition() -> TradeDispositionProfile {
    TradeDispositionProfile {
        demand_memory_retention_ticks: 240,
        ..default_trade_disposition()
    }
}

fn build_craft_restock_recipe_registry() -> RecipeRegistry {
    let mut recipes = RecipeRegistry::new();
    recipes.register(RecipeDefinition {
        name: "Harvest Firewood".to_string(),
        inputs: vec![],
        outputs: vec![(CommodityKind::Firewood, Quantity(1))],
        work_ticks: nz(3),
        required_workstation_tag: Some(WorkstationTag::ChoppingBlock),
        required_tool_kinds: vec![],
        body_cost_per_tick: BodyCostPerTick::new(pm(3), pm(2), pm(5), pm(1)),
    });
    recipes.register(build_bake_bread_recipe());
    recipes
}

// ---------------------------------------------------------------------------
// Segment 1: Merchant Restock Cycle (with traces)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn run_merchant_restock_with_traces(seed: Seed) -> (StateHash, StateHash) {
    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);
    let mut h = GoldenHarness::new(seed);

    // OrchardRow workstation + ResourceSource at Orchard Farm.
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

    // Producer at Orchard Farm — low needs, knows harvest, sells apples.
    let producer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Producer",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        producer,
        PerceptionProfile {
            memory_capacity: 64,
            memory_retention_ticks: 240,
            observation_fidelity: pm(875),
            confidence_policy: BeliefConfidencePolicy::default(),
        },
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_merchandise_profile(
            producer,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Apple]),
                home_market: Some(ORCHARD_FARM),
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(producer, default_trade_disposition())
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        producer,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    // Merchant at General Store — enterprise-focused, has coins, restock demand.
    let merchant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Merchant",
        general_store,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            enterprise_weight: pm(900),
            social_weight: pm(0),
            care_weight: pm(0),
            ..UtilityProfile::default()
        },
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant,
        general_store,
        CommodityKind::Coin,
        Quantity(5),
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_perception_profile(
            merchant,
            PerceptionProfile {
                memory_capacity: 64,
                memory_retention_ticks: 240,
                observation_fidelity: pm(875),
                confidence_policy: BeliefConfidencePolicy::default(),
            },
        )
        .unwrap();
        txn.set_component_merchandise_profile(
            merchant,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Apple]),
                home_market: Some(general_store),
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(merchant, enterprise_trade_disposition())
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
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        &[orchard_ws, producer],
        Tick(0),
        PerceptionSource::Inference,
    );

    // Plan continuation (SUPPLYCHAINFIX-001) allows default budget in multi-agent
    // scenarios by skipping expensive plan search when the current goal is still
    // top-ranked and the next step revalidates.
    h.driver = AgentTickDriver::new(PlanningBudget::default());

    // Enable tracing and run.
    h.driver.enable_tracing();

    let initial_authoritative_apples =
        total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);

    let mut merchant_left_home = false;
    let mut merchant_acquired_apples = false;
    let mut merchant_returned_with_apples = false;

    for _ in 0..220 {
        h.step_once();

        let merchant_place = h.world.effective_place(merchant);
        let merchant_apples = h.agent_commodity_qty(merchant, CommodityKind::Apple);
        let authoritative_apples =
            total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);

        merchant_left_home |=
            h.world.is_in_transit(merchant) || merchant_place != Some(general_store);
        merchant_acquired_apples |= merchant_apples > Quantity(0);
        merchant_returned_with_apples |=
            merchant_place == Some(general_store) && merchant_apples > Quantity(0);

        assert!(
            authoritative_apples <= initial_authoritative_apples,
            "authoritative apples should never increase: initial={initial_authoritative_apples}, now={authoritative_apples}"
        );

        if merchant_left_home && merchant_acquired_apples && merchant_returned_with_apples {
            break;
        }
    }

    // Supply chain assertions.
    assert!(
        merchant_left_home,
        "Merchant should leave General Store to restock"
    );
    assert!(
        merchant_acquired_apples,
        "Merchant should acquire apples (via harvest at Orchard Farm)"
    );
    assert!(
        merchant_returned_with_apples,
        "Merchant should return to General Store with apples"
    );
    assert!(!h.agent_is_dead(producer), "Producer must stay alive");
    assert!(!h.agent_is_dead(merchant), "Merchant must stay alive");

    // Trace assertion: Merchant generated RestockCommodity(Apple) in the first 20 ticks.
    let sink = h.driver.trace_sink().expect("tracing should be enabled");
    let merchant_early_restock = (0u64..=20).any(|t| {
        sink.trace_at(merchant, Tick(t))
            .is_some_and(|trace| match &trace.outcome {
                DecisionOutcome::Planning(planning) => {
                    planning.candidates.generated.iter().any(|g| {
                        matches!(
                            g.kind,
                            GoalKind::RestockCommodity {
                                commodity: CommodityKind::Apple
                            }
                        )
                    })
                }
                _ => false,
            })
    });
    assert!(
        merchant_early_restock,
        "Merchant should generate RestockCommodity(Apple) in the first 20 ticks. \
         Use `sink.dump_agent(merchant, &h.defs)` to diagnose."
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Segment 2: Consumer Co-located Trade (with traces)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn run_merchant_restocks_via_prerequisite_aware_craft(seed: Seed) -> (StateHash, StateHash) {
    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);
    let mut h = GoldenHarness::with_recipes(seed, build_craft_restock_recipe_registry());
    h.driver = AgentTickDriver::new(PlanningBudget {
        max_plan_depth: 12,
        ..PlanningBudget::default()
    });
    let harvest_firewood_recipe = h
        .recipes
        .recipe_by_name("Harvest Firewood")
        .map(|(id, _)| id)
        .expect("harvest firewood recipe should exist");
    let bake_bread_recipe = h
        .recipes
        .recipe_by_name("Bake Bread")
        .map(|(id, _)| id)
        .expect("bake bread recipe should exist");

    let remote_firewood_source = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::ChoppingBlock,
        ResourceSource {
            commodity: CommodityKind::Firewood,
            available_quantity: Quantity(1),
            max_quantity: Quantity(1),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );
    place_workstation(
        &mut h.world,
        &mut h.event_log,
        general_store,
        WorkstationTag::Mill,
        ProductionOutputOwner::Actor,
    );

    let merchant = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Merchant",
        general_store,
        HomeostaticNeeds::default(),
        MetabolismProfile {
            hunger_rate: pm(0),
            thirst_rate: pm(0),
            fatigue_rate: pm(0),
            bladder_rate: pm(0),
            dirtiness_rate: pm(0),
            ..MetabolismProfile::default()
        },
        UtilityProfile {
            enterprise_weight: pm(900),
            ..UtilityProfile::default()
        },
        KnownRecipes::with([harvest_firewood_recipe, bake_bread_recipe]),
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_perception_profile(
            merchant,
            PerceptionProfile {
                memory_capacity: 64,
                memory_retention_ticks: 240,
                observation_fidelity: pm(875),
                confidence_policy: BeliefConfidencePolicy::default(),
            },
        )
        .unwrap();
        txn.set_component_merchandise_profile(
            merchant,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(general_store),
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(merchant, enterprise_trade_disposition())
            .unwrap();
        txn.set_component_demand_memory(
            merchant,
            DemandMemory {
                observations: vec![DemandObservation {
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(1),
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
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        &[remote_firewood_source],
        Tick(0),
        PerceptionSource::Inference,
    );

    h.driver.enable_tracing();
    h.enable_action_tracing();

    let initial_firewood_authority =
        total_authoritative_commodity_quantity(&h.world, CommodityKind::Firewood);
    let initial_bread_authority =
        total_authoritative_commodity_quantity(&h.world, CommodityKind::Bread);
    let initial_combined_authority = initial_firewood_authority + initial_bread_authority;

    let mut merchant_visited_orchard = false;
    let mut merchant_acquired_firewood = false;
    let mut bread_restocked_at_home_market = false;

    for _ in 0..200 {
        h.step_once();

        let firewood_authority =
            total_authoritative_commodity_quantity(&h.world, CommodityKind::Firewood);
        let bread_authority =
            total_authoritative_commodity_quantity(&h.world, CommodityKind::Bread);
        let live_firewood = total_live_lot_quantity(&h.world, CommodityKind::Firewood);
        let live_bread = total_live_lot_quantity(&h.world, CommodityKind::Bread);

        merchant_visited_orchard |= h.world.effective_place(merchant) == Some(ORCHARD_FARM);
        merchant_acquired_firewood |=
            h.agent_commodity_qty(merchant, CommodityKind::Firewood) > Quantity(0);
        bread_restocked_at_home_market |= h
            .world
            .entities_effectively_at(general_store)
            .into_iter()
            .any(|entity| {
                h.world
                    .get_component_item_lot(entity)
                    .is_some_and(|lot| lot.commodity == CommodityKind::Bread)
            });

        assert_eq!(
            firewood_authority + bread_authority,
            initial_combined_authority,
            "firewood-to-bread transformation should preserve combined authoritative quantity"
        );
        assert!(
            live_firewood + live_bread <= initial_combined_authority,
            "live lots should stay bounded by the single remote prerequisite chain"
        );
        verify_authoritative_conservation(&h.world, CommodityKind::Firewood, firewood_authority)
            .unwrap();
        verify_authoritative_conservation(&h.world, CommodityKind::Bread, bread_authority).unwrap();
        verify_live_lot_conservation(&h.world, CommodityKind::Firewood, live_firewood).unwrap();
        verify_live_lot_conservation(&h.world, CommodityKind::Bread, live_bread).unwrap();

        let merchant_events = h
            .action_trace_sink()
            .expect("action tracing should be enabled for craft restock")
            .events_for(merchant);
        let craft_committed = merchant_events.iter().any(|event| {
            event.action_name == "craft:Bake Bread"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        });

        if merchant_visited_orchard
            && merchant_acquired_firewood
            && bread_restocked_at_home_market
            && craft_committed
        {
            break;
        }
    }

    let decision_trace = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled for craft restock");
    let tick_zero_trace = decision_trace
        .trace_at(merchant, Tick(0))
        .expect("merchant should have a tick 0 trace")
        .clone();
    let tick_zero_planning = match &tick_zero_trace.outcome {
        DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected planning trace at tick 0, got {other:?}"),
    };
    let selected_plan = tick_zero_planning
        .selection
        .selected_plan
        .as_ref()
        .expect("merchant should select a craft-restock plan at tick 0");
    let next_step = selected_plan
        .next_step
        .as_ref()
        .expect("selected craft-restock plan should expose a next step");
    assert_eq!(
        tick_zero_planning.selection.selected_plan_source,
        Some(SelectedPlanSource::SearchSelection),
        "craft-restock scenario should start from a fresh search result"
    );
    assert_eq!(
        next_step.op_kind,
        PlannerOpKind::Travel,
        "craft-restock plan should begin by traveling toward the remote prerequisite source"
    );
    assert!(
        selected_plan
            .steps
            .iter()
            .any(|step| step.op_kind == PlannerOpKind::Travel && step.targets == vec![ORCHARD_FARM]),
        "selected craft-restock plan should include travel to Orchard Farm"
    );

    let merchant_generated_and_selected_restock = (0u64..=20).any(|tick| {
        decision_trace
            .trace_at(merchant, Tick(tick))
            .is_some_and(|trace| match &trace.outcome {
                DecisionOutcome::Planning(planning) => {
                    let generated = planning.candidates.generated.iter().any(|goal| {
                        matches!(
                            goal.kind,
                            GoalKind::RestockCommodity {
                                commodity: CommodityKind::Bread
                            }
                        )
                    });
                    let selected = planning.selection.selected.as_ref().is_some_and(|goal| {
                        matches!(
                            goal.kind,
                            GoalKind::RestockCommodity {
                                commodity: CommodityKind::Bread
                            }
                        )
                    });
                    generated && selected
                }
                _ => false,
            })
    });
    assert!(
        merchant_generated_and_selected_restock,
        "merchant should generate and select RestockCommodity(Bread) in the first 20 ticks"
    );
    let saw_prerequisite_guidance = tick_zero_planning.planning.attempts.iter().any(|attempt| {
        attempt
            .expansion_summaries
            .iter()
            .any(|summary| summary.prerequisite_places_count > 0)
    });
    assert!(
        saw_prerequisite_guidance,
        "craft-restock search should record non-empty prerequisite places in expansion summaries"
    );

    let merchant_events = h
        .action_trace_sink()
        .expect("action tracing should be enabled for craft restock")
        .events_for(merchant);
    let mut travel_commits = merchant_events
        .iter()
        .filter_map(|event| {
            (event.action_name == "travel"
                && matches!(event.kind, ActionTraceKind::Committed { .. }))
            .then_some((event.tick, event.sequence_in_tick))
        })
        .collect::<Vec<_>>();
    travel_commits.sort_unstable();
    let harvest_commit = merchant_events
        .iter()
        .find_map(|event| {
            (event.action_name == "harvest:Harvest Firewood"
                && matches!(event.kind, ActionTraceKind::Committed { .. }))
            .then_some((event.tick, event.sequence_in_tick))
        })
        .expect("craft-restock scenario should commit Harvest Firewood");
    let craft_commit = merchant_events
        .iter()
        .find_map(|event| {
            (event.action_name == "craft:Bake Bread"
                && matches!(event.kind, ActionTraceKind::Committed { .. }))
            .then_some((event.tick, event.sequence_in_tick))
        })
        .expect("craft-restock scenario should commit Bake Bread");
    assert!(
        travel_commits.len() >= 2,
        "craft-restock scenario should commit outbound and return travel; events={merchant_events:?}"
    );
    assert!(
        travel_commits.iter().any(|commit| *commit < harvest_commit),
        "merchant should travel before harvesting remote firewood; events={merchant_events:?}"
    );
    assert!(
        travel_commits.iter().any(|commit| *commit > harvest_commit && *commit < craft_commit),
        "merchant should return home after harvesting and before crafting; events={merchant_events:?}"
    );
    assert!(
        merchant_visited_orchard,
        "merchant should visit Orchard Farm before the restock craft completes"
    );
    assert!(
        merchant_acquired_firewood,
        "merchant should acquire firewood before crafting bread"
    );
    assert!(
        bread_restocked_at_home_market,
        "craft-restock scenario should leave bread stock at the home market; final_place={:?}, merchant_bread={:?}, live_bread={}, events={merchant_events:?}",
        h.world.effective_place(merchant),
        h.agent_commodity_qty(merchant, CommodityKind::Bread),
        total_live_lot_quantity(&h.world, CommodityKind::Bread),
    );
    assert_eq!(
        h.world
            .get_component_resource_source(remote_firewood_source)
            .expect("remote firewood source should remain present")
            .available_quantity,
        Quantity(0),
        "remote firewood source should be depleted by the successful craft-restock chain"
    );
    assert_eq!(
        h.world.effective_place(merchant),
        Some(general_store),
        "merchant should finish at the home market after restocking"
    );
    assert!(
        h.world
            .entities_effectively_at(general_store)
            .into_iter()
            .any(|entity| {
                h.world
                    .get_component_item_lot(entity)
                    .is_some_and(|lot| lot.commodity == CommodityKind::Bread)
            }),
        "bread should exist at the home market after the successful craft-restock chain"
    );
    assert_eq!(
        h.agent_commodity_qty(merchant, CommodityKind::Firewood),
        Quantity(0),
        "merchant should not retain the prerequisite input after crafting"
    );
    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[allow(clippy::too_many_lines)]
fn run_stale_prerequisite_belief_discovery_replan(seed: Seed) -> (StateHash, StateHash) {
    let home_market = prototype_place_entity(PrototypePlace::EastFieldTrail);
    let bandit_camp = prototype_place_entity(PrototypePlace::BanditCamp);
    let mut h = GoldenHarness::with_recipes(seed, build_craft_restock_recipe_registry());
    h.driver = AgentTickDriver::new(PlanningBudget {
        max_plan_depth: 12,
        max_node_expansions: 1024,
        ..PlanningBudget::default()
    });
    let harvest_firewood_recipe = h
        .recipes
        .recipe_by_name("Harvest Firewood")
        .map(|(id, _)| id)
        .expect("harvest firewood recipe should exist");
    let bake_bread_recipe = h
        .recipes
        .recipe_by_name("Bake Bread")
        .map(|(id, _)| id)
        .expect("bake bread recipe should exist");

    let orchard_source = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::ChoppingBlock,
        ResourceSource {
            commodity: CommodityKind::Firewood,
            available_quantity: Quantity(1),
            max_quantity: Quantity(1),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );
    let bandit_source = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        bandit_camp,
        WorkstationTag::ChoppingBlock,
        ResourceSource {
            commodity: CommodityKind::Firewood,
            available_quantity: Quantity(1),
            max_quantity: Quantity(1),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );
    place_workstation(
        &mut h.world,
        &mut h.event_log,
        home_market,
        WorkstationTag::Mill,
        ProductionOutputOwner::Actor,
    );

    let merchant = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Merchant",
        home_market,
        HomeostaticNeeds::default(),
        MetabolismProfile {
            hunger_rate: pm(0),
            thirst_rate: pm(0),
            fatigue_rate: pm(0),
            bladder_rate: pm(0),
            dirtiness_rate: pm(0),
            ..MetabolismProfile::default()
        },
        UtilityProfile {
            enterprise_weight: pm(900),
            ..UtilityProfile::default()
        },
        KnownRecipes::with([harvest_firewood_recipe, bake_bread_recipe]),
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_perception_profile(
            merchant,
            PerceptionProfile {
                memory_capacity: 64,
                memory_retention_ticks: 240,
                observation_fidelity: pm(875),
                confidence_policy: BeliefConfidencePolicy::default(),
            },
        )
        .unwrap();
        txn.set_component_merchandise_profile(
            merchant,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(home_market),
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(merchant, enterprise_trade_disposition())
            .unwrap();
        txn.set_component_demand_memory(
            merchant,
            DemandMemory {
                observations: vec![DemandObservation {
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(1),
                    place: home_market,
                    tick: Tick(0),
                    counterparty: None,
                    reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
                }],
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        merchant,
        orchard_source,
        Tick(0),
        PerceptionSource::Inference,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        merchant,
        bandit_source,
        Tick(0),
        PerceptionSource::Inference,
    );

    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_resource_source(
            orchard_source,
            ResourceSource {
                commodity: CommodityKind::Firewood,
                available_quantity: Quantity(0),
                max_quantity: Quantity(1),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    h.driver.enable_tracing();
    h.enable_action_tracing();

    let initial_belief = agent_belief_about(&h.world, merchant, orchard_source)
        .expect("merchant should retain a seeded stale belief about the orchard source");
    assert_eq!(
        initial_belief
            .resource_source
            .as_ref()
            .map(|source| source.available_quantity),
        Some(Quantity(1)),
        "seeded belief should still think the orchard source is stocked"
    );
    assert_eq!(
        h.world
            .get_component_resource_source(orchard_source)
            .expect("orchard source should remain present")
            .available_quantity,
        Quantity(0),
        "authoritative orchard source should start empty so the belief is stale"
    );

    let mut visited_orchard = false;
    let mut visited_bandit = false;
    let mut acquired_firewood = false;
    let mut craft_committed = false;
    let mut bread_restocked_at_home_market = false;

    for _ in 0..280 {
        h.step_once();

        let firewood_authority =
            total_authoritative_commodity_quantity(&h.world, CommodityKind::Firewood);
        let bread_authority =
            total_authoritative_commodity_quantity(&h.world, CommodityKind::Bread);
        let live_firewood = total_live_lot_quantity(&h.world, CommodityKind::Firewood);
        let live_bread = total_live_lot_quantity(&h.world, CommodityKind::Bread);

        visited_orchard |= h.world.effective_place(merchant) == Some(ORCHARD_FARM);
        visited_bandit |= h.world.effective_place(merchant) == Some(bandit_camp);
        acquired_firewood |= h.agent_commodity_qty(merchant, CommodityKind::Firewood) > Quantity(0);
        bread_restocked_at_home_market |= h
            .world
            .entities_effectively_at(home_market)
            .into_iter()
            .any(|entity| {
                h.world
                    .get_component_item_lot(entity)
                    .is_some_and(|lot| lot.commodity == CommodityKind::Bread)
            });

        verify_authoritative_conservation(&h.world, CommodityKind::Firewood, firewood_authority)
            .unwrap();
        verify_authoritative_conservation(&h.world, CommodityKind::Bread, bread_authority).unwrap();
        verify_live_lot_conservation(&h.world, CommodityKind::Firewood, live_firewood).unwrap();
        verify_live_lot_conservation(&h.world, CommodityKind::Bread, live_bread).unwrap();

        let merchant_events = h
            .action_trace_sink()
            .expect("action tracing should be enabled for stale-belief recovery")
            .events_for(merchant);
        craft_committed |= merchant_events.iter().any(|event| {
            event.action_name == "craft:Bake Bread"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        });

        if visited_orchard
            && visited_bandit
            && acquired_firewood
            && craft_committed
            && bread_restocked_at_home_market
        {
            break;
        }
    }

    let trace_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should remain enabled for stale-belief recovery");
    let tick_zero_trace = trace_sink
        .trace_at(merchant, Tick(0))
        .expect("merchant should have a tick 0 trace")
        .clone();
    let tick_zero_planning = match &tick_zero_trace.outcome {
        DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected planning trace at tick 0, got {other:?}"),
    };
    let selected_tick_zero_plan = tick_zero_planning
        .selection
        .selected_plan
        .as_ref()
        .expect("stale-belief scenario should select a plan at tick 0");
    assert_eq!(
        tick_zero_planning.selection.selected_plan_source,
        Some(SelectedPlanSource::SearchSelection),
        "stale-belief scenario should start from a fresh search result"
    );
    assert!(matches!(
        tick_zero_planning.selection.selected.as_ref().map(|goal| &goal.kind),
        Some(GoalKind::RestockCommodity { commodity }) if *commodity == CommodityKind::Bread
    ));
    assert_eq!(
        selected_tick_zero_plan
            .next_step
            .as_ref()
            .expect("selected plan should expose a first step")
            .op_kind,
        PlannerOpKind::Travel,
        "stale-belief scenario should begin by traveling toward the stale prerequisite source"
    );
    assert!(
        selected_tick_zero_plan.steps.iter().any(|step| {
            step.op_kind == PlannerOpKind::Travel && step.targets == vec![ORCHARD_FARM]
        }),
        "tick 0 plan should route through Orchard Farm from the stale prerequisite belief"
    );
    let initial_candidate_trace = tick_zero_planning
        .candidates
        .evidence
        .iter()
        .find(|trace| {
            matches!(
                trace.goal.kind,
                GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread
                }
            )
        })
        .expect("initial stale branch should record typed candidate evidence provenance");
    assert!(initial_candidate_trace
        .contributors
        .iter()
        .any(|contributor| {
            contributor.kind == CandidateEvidenceKind::ResourceSource
                && contributor.entity == orchard_source
                && contributor.place == ORCHARD_FARM
        }));

    let fallback_replan_trace =
        trace_sink
            .traces_for(merchant)
            .into_iter()
            .find(|trace| match &trace.outcome {
                DecisionOutcome::Planning(planning) => {
                    trace.tick > Tick(0)
                        && planning.selection.selected_plan_source
                            == Some(SelectedPlanSource::SearchSelection)
                        && planning.selection.selected.as_ref().is_some_and(|goal| {
                            matches!(
                                goal.kind,
                                GoalKind::RestockCommodity {
                                    commodity: CommodityKind::Bread
                                }
                            )
                        })
                        && planning
                            .selection
                            .selected_plan
                            .as_ref()
                            .is_some_and(|plan| {
                                plan.steps.iter().any(|step| {
                                    step.op_kind == PlannerOpKind::Travel
                                        && step.targets == vec![bandit_camp]
                                })
                            })
                }
                _ => false,
            });
    let replan_planning = match &fallback_replan_trace
        .expect("corrected local belief should trigger a fresh fallback replan toward Bandit Camp")
        .outcome
    {
        DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected fallback planning trace, got {other:?}"),
    };
    let replanned_plan = replan_planning
        .selection
        .selected_plan
        .as_ref()
        .expect("fallback planning should select a bandit-camp plan");
    assert!(matches!(
        replan_planning.selection.selected.as_ref().map(|goal| &goal.kind),
        Some(GoalKind::RestockCommodity { commodity }) if *commodity == CommodityKind::Bread
    ));
    let replacement = replan_planning
        .selection
        .plan_replacement
        .as_ref()
        .expect("fallback replan should expose same-goal branch replacement provenance");
    assert_eq!(
        replacement.kind,
        SelectedPlanReplacementKind::SameGoalBranchReplanned
    );
    assert_eq!(replacement.previous_goal, replacement.new_goal);
    assert_ne!(
        replacement
            .previous_next_step
            .as_ref()
            .expect("replacement should expose the invalidated branch step")
            .targets,
        replacement
            .new_next_step
            .as_ref()
            .expect("replacement should expose the fresh branch step")
            .targets
    );
    assert!(
        replanned_plan.steps.iter().any(|step| {
            step.op_kind == PlannerOpKind::Travel && step.targets == vec![bandit_camp]
        }),
        "post-failure plan should route through the fallback firewood source"
    );
    let fallback_candidate_trace = replan_planning
        .candidates
        .evidence
        .iter()
        .find(|trace| {
            matches!(
                trace.goal.kind,
                GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread
                }
            )
        })
        .expect("fallback replan should record typed candidate evidence provenance");
    assert!(fallback_candidate_trace
        .contributors
        .iter()
        .any(|contributor| {
            contributor.kind == CandidateEvidenceKind::ResourceSource
                && contributor.entity == bandit_source
                && contributor.place == bandit_camp
        }));
    assert!(fallback_candidate_trace.exclusions.iter().any(|exclusion| {
        exclusion.kind == CandidateEvidenceKind::ResourceSource
            && exclusion.entity == orchard_source
            && exclusion.place == ORCHARD_FARM
            && exclusion.reason == CandidateEvidenceExclusionReason::DepletedResourceSource
    }));
    let selected_attempt = replan_planning
        .planning
        .attempts
        .iter()
        .find(|attempt| {
            matches!(
                attempt.goal.kind,
                GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread
                }
            )
        })
        .expect("fallback replan should keep the selected goal's search attempt");
    let root_guidance = selected_attempt
        .expansion_summaries
        .first()
        .and_then(|summary| summary.prerequisite_guidance.as_ref())
        .expect("search trace should expose root prerequisite guidance");
    assert!(
        root_guidance.prerequisite_places.contains(&bandit_camp),
        "fallback guidance should keep the live Bandit Camp prerequisite place"
    );
    assert!(root_guidance.exclusions.iter().any(|exclusion| {
        exclusion.place == ORCHARD_FARM && exclusion.commodity == CommodityKind::Firewood
    }));

    assert!(
        agent_belief_about(&h.world, merchant, orchard_source)
            .and_then(|belief| belief.resource_source.as_ref())
            .is_some_and(|source| source.available_quantity == Quantity(0)),
        "merchant should correct the orchard-source belief to empty after local observation"
    );
    assert!(
        visited_orchard,
        "merchant should visit Orchard Farm from the stale prerequisite belief"
    );
    assert!(
        visited_bandit,
        "merchant should later visit the fallback source after the orchard branch fails"
    );
    assert!(
        acquired_firewood,
        "merchant should acquire fallback firewood before crafting bread"
    );
    assert!(
        bread_restocked_at_home_market,
        "merchant should restock bread at the home market after recovering from the stale source"
    );
    assert!(
        craft_committed,
        "stale-belief recovery should still commit craft:Bake Bread"
    );
    assert_eq!(
        h.world
            .get_component_resource_source(orchard_source)
            .expect("orchard source should remain present")
            .available_quantity,
        Quantity(0),
        "stale orchard source should remain depleted after the failed branch"
    );
    assert_eq!(
        h.world
            .get_component_resource_source(bandit_source)
            .expect("fallback source should remain present")
            .available_quantity,
        Quantity(0),
        "fallback source should be depleted by the successful recovery chain"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[allow(clippy::too_many_lines)]
fn run_consumer_trade_with_traces(seed: Seed) -> (StateHash, StateHash) {
    use worldwake_sim::RecipeRegistry;

    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);
    let mut h = GoldenHarness::with_recipes(seed, RecipeRegistry::new());

    // Merchant at General Store with apples ready to sell.
    let merchant = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Merchant",
        general_store,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::new(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant,
        general_store,
        CommodityKind::Apple,
        Quantity(3),
    );

    // Consumer at General Store — hungry, has coins.
    let consumer = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Consumer",
        general_store,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::new(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        consumer,
        general_store,
        CommodityKind::Coin,
        Quantity(3),
    );

    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_merchandise_profile(
            merchant,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Apple]),
                home_market: Some(general_store),
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(merchant, default_trade_disposition())
            .unwrap();
        txn.set_component_trade_disposition_profile(consumer, default_trade_disposition())
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Enable tracing and run.
    h.driver.enable_tracing();

    let initial_consumer_hunger = h.agent_hunger(consumer);
    let initial_total_coins = total_live_lot_quantity(&h.world, CommodityKind::Coin);

    let mut consumer_acquired_apples = false;
    let mut consumer_hunger_decreased = false;

    for _ in 0..80 {
        h.step_once();

        let consumer_apples = h.agent_commodity_qty(consumer, CommodityKind::Apple);
        consumer_acquired_apples |= consumer_apples > Quantity(0);
        consumer_hunger_decreased |= h.agent_hunger(consumer) < initial_consumer_hunger;

        // Coin conservation per tick.
        let current_coins = total_live_lot_quantity(&h.world, CommodityKind::Coin);
        assert_eq!(
            current_coins, initial_total_coins,
            "Coin conservation violated"
        );

        if consumer_acquired_apples && consumer_hunger_decreased {
            break;
        }
    }

    // Assertions.
    assert!(
        consumer_acquired_apples,
        "Consumer should acquire apples via trade with merchant"
    );
    assert!(
        consumer_hunger_decreased,
        "Consumer hunger should decrease after eating acquired apples"
    );
    assert!(!h.agent_is_dead(consumer), "Consumer must stay alive");
    assert!(!h.agent_is_dead(merchant), "Merchant must stay alive");

    // Trace assertion: Consumer generated AcquireCommodity(Apple) in the first 10 ticks.
    let sink = h.driver.trace_sink().expect("tracing should be enabled");
    let consumer_acquire_goal = (0u64..=10).any(|t| {
        sink.trace_at(consumer, Tick(t))
            .is_some_and(|trace| match &trace.outcome {
                DecisionOutcome::Planning(planning) => {
                    planning.candidates.generated.iter().any(|g| {
                        matches!(
                            g.kind,
                            GoalKind::AcquireCommodity {
                                commodity: CommodityKind::Apple,
                                ..
                            }
                        )
                    })
                }
                _ => false,
            })
    });
    assert!(
        consumer_acquire_goal,
        "Consumer should generate AcquireCommodity(Apple) in the first 10 ticks. \
         Use `sink.dump_agent(consumer, &h.defs)` to diagnose."
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Full combined: Merchant Restock → Consumer Trade (with traces)
// ---------------------------------------------------------------------------

/// Full end-to-end supply chain in a single simulation:
/// Producer at Orchard Farm, Merchant at General Store, Consumer at General Store.
/// Merchant restocks apples from orchard, returns, consumer buys and eats.
#[allow(clippy::too_many_lines)]
fn run_full_supply_chain(seed: Seed) -> (StateHash, StateHash) {
    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);
    let mut h = GoldenHarness::new(seed);

    // OrchardRow workstation + ResourceSource at Orchard Farm.
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

    // Producer at Orchard Farm — low needs, knows harvest, sells apples.
    let producer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Producer",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        producer,
        PerceptionProfile {
            memory_capacity: 64,
            memory_retention_ticks: 240,
            observation_fidelity: pm(875),
            confidence_policy: BeliefConfidencePolicy::default(),
        },
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_merchandise_profile(
            producer,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Apple]),
                home_market: Some(ORCHARD_FARM),
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(producer, default_trade_disposition())
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        producer,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    // Merchant at General Store — enterprise-focused, has coins, restock demand.
    // Minimal non-enterprise metabolism so the merchant stays at General Store
    // after returning with stock, ensuring co-location for consumer trade.
    let merchant_metabolism = MetabolismProfile {
        hunger_rate: pm(1),
        thirst_rate: pm(0),
        fatigue_rate: pm(0),
        bladder_rate: pm(0),
        dirtiness_rate: pm(0),
        ..MetabolismProfile::default()
    };
    let merchant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Merchant",
        general_store,
        HomeostaticNeeds::default(),
        merchant_metabolism,
        UtilityProfile {
            enterprise_weight: pm(900),
            ..UtilityProfile::default()
        },
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant,
        general_store,
        CommodityKind::Coin,
        Quantity(5),
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_perception_profile(
            merchant,
            PerceptionProfile {
                memory_capacity: 64,
                memory_retention_ticks: 240,
                observation_fidelity: pm(875),
                confidence_policy: BeliefConfidencePolicy::default(),
            },
        )
        .unwrap();
        txn.set_component_merchandise_profile(
            merchant,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Apple]),
                home_market: Some(general_store),
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(merchant, enterprise_trade_disposition())
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
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        &[orchard_ws, producer],
        Tick(0),
        PerceptionSource::Inference,
    );

    // Consumer at General Store — hungry, has coins, knows about merchant.
    // Hunger-only metabolism prevents the consumer from drifting away to
    // satisfy bladder/thirst during the ~150-tick wait for the merchant.
    let consumer_metabolism = MetabolismProfile {
        hunger_rate: pm(2),
        thirst_rate: pm(0),
        fatigue_rate: pm(0),
        bladder_rate: pm(0),
        dirtiness_rate: pm(0),
        ..MetabolismProfile::default()
    };
    let consumer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Consumer",
        general_store,
        HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        consumer_metabolism,
        UtilityProfile::default(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        consumer,
        general_store,
        CommodityKind::Coin,
        Quantity(5),
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_perception_profile(
            consumer,
            PerceptionProfile {
                memory_capacity: 64,
                memory_retention_ticks: 480,
                observation_fidelity: pm(875),
                confidence_policy: BeliefConfidencePolicy::default(),
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(consumer, default_trade_disposition())
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        consumer,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    // Increased budget for the 3-agent combined scenario.
    // The segment tests prove plan continuation works at default 512 budget.
    // The full chain needs 1024 for the return-trip plan search from the
    // remote OrchardFarm through the high-branching VillageSquare hub.
    h.driver = AgentTickDriver::new(PlanningBudget {
        max_node_expansions: 1024,
        ..PlanningBudget::default()
    });
    h.driver.enable_tracing();

    let initial_authoritative_apples =
        total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);
    let initial_consumer_hunger = h.agent_hunger(consumer);

    let mut merchant_left_home = false;
    let mut merchant_acquired_apples = false;
    let mut merchant_returned_with_apples = false;
    let mut consumer_acquired_apples = false;
    let mut consumer_hunger_decreased = false;

    for _ in 0..500 {
        h.step_once();

        let merchant_place = h.world.effective_place(merchant);
        let merchant_apples = h.agent_commodity_qty(merchant, CommodityKind::Apple);
        let authoritative_apples =
            total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);

        merchant_left_home |=
            h.world.is_in_transit(merchant) || merchant_place != Some(general_store);
        merchant_acquired_apples |= merchant_apples > Quantity(0);
        merchant_returned_with_apples |=
            merchant_place == Some(general_store) && merchant_apples > Quantity(0);

        let consumer_apples = h.agent_commodity_qty(consumer, CommodityKind::Apple);
        consumer_acquired_apples |= consumer_apples > Quantity(0);
        consumer_hunger_decreased |= h.agent_hunger(consumer) < initial_consumer_hunger;

        assert!(
            authoritative_apples <= initial_authoritative_apples,
            "authoritative apples should never increase: initial={initial_authoritative_apples}, now={authoritative_apples}"
        );

        // Exit early once the full chain completes.
        if merchant_returned_with_apples && consumer_acquired_apples && consumer_hunger_decreased {
            break;
        }
    }

    // Supply chain assertions.
    assert!(
        merchant_left_home,
        "Merchant should leave General Store to restock"
    );
    assert!(
        merchant_acquired_apples,
        "Merchant should acquire apples (via harvest at Orchard Farm)"
    );
    assert!(
        merchant_returned_with_apples,
        "Merchant should return to General Store with apples"
    );
    assert!(
        consumer_acquired_apples,
        "Consumer should acquire apples via trade with merchant"
    );
    assert!(
        consumer_hunger_decreased,
        "Consumer hunger should decrease after eating acquired apples"
    );
    assert!(!h.agent_is_dead(producer), "Producer must stay alive");
    assert!(!h.agent_is_dead(merchant), "Merchant must stay alive");
    assert!(!h.agent_is_dead(consumer), "Consumer must stay alive");

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Test entry points
// ---------------------------------------------------------------------------

#[test]
fn test_merchant_restock_with_traces() {
    let _ = run_merchant_restock_with_traces(Seed([100; 32]));
}

#[test]
fn test_merchant_restock_replay() {
    let first = run_merchant_restock_with_traces(Seed([101; 32]));
    let second = run_merchant_restock_with_traces(Seed([101; 32]));

    assert_eq!(
        first, second,
        "Merchant restock scenario should replay deterministically"
    );
}

#[test]
fn golden_merchant_restocks_via_prerequisite_aware_craft() {
    let _ = run_merchant_restocks_via_prerequisite_aware_craft(Seed([106; 32]));
}

#[test]
fn golden_merchant_restocks_via_prerequisite_aware_craft_replays_deterministically() {
    let first = run_merchant_restocks_via_prerequisite_aware_craft(Seed([107; 32]));
    let second = run_merchant_restocks_via_prerequisite_aware_craft(Seed([107; 32]));

    assert_eq!(
        first, second,
        "craft-restock scenario should replay deterministically"
    );
}

#[test]
fn golden_stale_prerequisite_belief_discovery_replan() {
    let _ = run_stale_prerequisite_belief_discovery_replan(Seed([108; 32]));
}

#[test]
fn golden_stale_prerequisite_belief_discovery_replan_replays_deterministically() {
    let first = run_stale_prerequisite_belief_discovery_replan(Seed([109; 32]));
    let second = run_stale_prerequisite_belief_discovery_replan(Seed([109; 32]));

    assert_eq!(
        first, second,
        "stale-belief prerequisite recovery scenario should replay deterministically"
    );
}

#[test]
fn test_consumer_trade_with_traces() {
    let _ = run_consumer_trade_with_traces(Seed([102; 32]));
}

#[test]
fn test_consumer_trade_replay() {
    let first = run_consumer_trade_with_traces(Seed([103; 32]));
    let second = run_consumer_trade_with_traces(Seed([103; 32]));

    assert_eq!(
        first, second,
        "Consumer trade scenario should replay deterministically"
    );
}

/// Full combined supply chain test (merchant restock → consumer trade in one sim).
///
/// Ignored: blocked on `specs/S10-bilateral-trade-negotiation.md`. The consumer
/// correctly observes the merchant's return with apples (perception works) and
/// generates a trade plan, but the merchant rejects the fixed 1-coin offer as
/// `InsufficientPayment` given its enterprise weight and scarce stock. The trade
/// system lacks price negotiation — every offer is hardcoded at 1:1. S10
/// introduces multi-round bilateral negotiation with variable pricing derived
/// from concrete agent state, which will unblock this test.
#[test]
#[ignore = "blocked on S10 bilateral trade negotiation pricing"]
fn test_full_supply_chain() {
    let _ = run_full_supply_chain(Seed([104; 32]));
}

#[test]
#[ignore = "blocked on S10 bilateral trade negotiation pricing"]
fn test_full_supply_chain_replay() {
    let first = run_full_supply_chain(Seed([105; 32]));
    let second = run_full_supply_chain(Seed([105; 32]));

    assert_eq!(
        first, second,
        "Full supply chain scenario should replay deterministically"
    );
}
