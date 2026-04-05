//! Golden tests for S06 commodity-opportunity integration.

mod golden_harness;

use golden_harness::*;
use std::num::NonZeroU8;
use worldwake_ai::{CommodityPurpose, DecisionOutcome, GoalKind};
use worldwake_core::{
    hash_event_log, hash_world, CommodityKind, CommodityValuationProfile, HomeostaticNeeds,
    KnownRecipes, MetabolismProfile, Quantity, Seed, Tick, UtilityProfile, WorkstationTag,
};
use worldwake_sim::{evaluate_trade_bundle, GoalBeliefView, PerAgentBeliefView, TradeAcceptance};

fn valuation_profile() -> CommodityValuationProfile {
    CommodityValuationProfile {
        recipe_opportunity_depth: NonZeroU8::new(3).unwrap(),
        recipe_place_horizon: 0,
        indirect_value_decay_per_step: pm(100),
    }
}

fn trace_has_recipe_input_candidate(
    h: &GoldenHarness,
    actor: worldwake_core::EntityId,
    commodity: CommodityKind,
    recipe_id: worldwake_core::RecipeId,
) -> bool {
    h.driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(actor, Tick(0))
        .is_some_and(|trace| match &trace.outcome {
            DecisionOutcome::Planning(planning) => {
                planning.candidates.generated.iter().any(|goal| {
                    matches!(
                        goal.goal_key.kind,
                        GoalKind::AcquireCommodity {
                            commodity: c,
                            purpose: CommodityPurpose::RecipeInput(r),
                        } if c == commodity && r == recipe_id
                    )
                })
            }
            _ => false,
        })
}

#[allow(clippy::type_complexity)]
fn run_recipe_input_snapshot_scenario(
    seed: Seed,
    mill_reachable: bool,
    knows_recipe: bool,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, build_multi_recipe_registry());
    let bread_recipe = h
        .recipes
        .recipe_by_name("Bake Bread")
        .map(|(id, _)| id)
        .expect("bake bread recipe should exist");

    let baker = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Baker",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        if knows_recipe {
            KnownRecipes::with([bread_recipe])
        } else {
            KnownRecipes::new()
        },
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        baker,
        VILLAGE_SQUARE,
        CommodityKind::Coin,
        Quantity(3),
    );
    let mut txn = new_txn(&mut h.world, 0);
    let firewood = txn
        .create_item_lot(CommodityKind::Firewood, Quantity(1))
        .expect("firewood lot should be creatable");
    txn.set_ground_location(firewood, VILLAGE_SQUARE)
        .expect("firewood should be placeable");
    commit_txn(txn, &mut h.event_log);
    if mill_reachable {
        place_workstation(
            &mut h.world,
            &mut h.event_log,
            VILLAGE_SQUARE,
            WorkstationTag::Mill,
            ProductionOutputOwner::Actor,
        );
    }

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_commodity_valuation_profile(baker, valuation_profile())
        .expect("golden scenario should keep valuation profiles writable");
    commit_txn(txn, &mut h.event_log);

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        baker,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );

    h.driver.enable_tracing();
    h.step_once();
    let ai_has_candidate =
        trace_has_recipe_input_candidate(&h, baker, CommodityKind::Firewood, bread_recipe);
    let belief_store = h
        .world
        .get_component_agent_belief_store(baker)
        .expect("baker should have a belief store")
        .clone();
    let view = PerAgentBeliefView::new_with_recipes(baker, &h.world, &h.recipes, &belief_store);
    let needs = GoalBeliefView::homeostatic_needs(&view, baker);
    let acceptance = evaluate_trade_bundle(
        baker,
        &view,
        needs.as_ref(),
        None,
        GoalBeliefView::commodity_quantity(&view, baker, CommodityKind::Coin),
        &[(CommodityKind::Coin, Quantity(1))],
        &[(CommodityKind::Firewood, Quantity(1))],
        &[],
        None,
    );

    assert_eq!(
        acceptance,
        TradeAcceptance::Reject {
            reason: worldwake_sim::TradeRejectionReason::NoNeed,
        },
        "local same-place setup should not over-claim a positive trade-side recipe-input contract"
    );
    assert_eq!(
        ai_has_candidate, mill_reachable && knows_recipe,
        "negative S06 scenarios should only emit a recipe-input candidate when both recipe knowledge and workstation reachability hold"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

// Scenario 89: Unreachable Workstation Suppresses Indirect Firewood Value
// ---------------------------------------------------------------------------
//
// Systems: AI ranking/candidate generation, belief-facing trade valuation
// GoalKinds: AcquireCommodity(RecipeInput)
// ActionDomains: Trade, Production
// Places: VillageSquare
//
// Setup: same hungry baker and known recipe, but no reachable mill is believed.
//
// Proves: AI does not generate the recipe-input firewood goal and trade
// valuation rejects receiving firewood as no-need.

#[test]
fn golden_unreachable_workstation_suppresses_recipe_input_value() {
    let _ = run_recipe_input_snapshot_scenario(Seed([92; 32]), false, true);
}

#[test]
fn golden_unreachable_workstation_suppresses_recipe_input_value_replays_deterministically() {
    let first = run_recipe_input_snapshot_scenario(Seed([93; 32]), false, true);
    let second = run_recipe_input_snapshot_scenario(Seed([93; 32]), false, true);
    assert_eq!(
        first, second,
        "unreachable-workstation recipe-input suppression should replay deterministically"
    );
}

// Scenario 90: No Known Recipe Prevents Indirect Firewood Motive
// ---------------------------------------------------------------------------
//
// Systems: AI ranking/candidate generation, belief-facing trade valuation
// GoalKinds: AcquireCommodity(RecipeInput)
// ActionDomains: Trade, Production
// Places: VillageSquare
//
// Setup: same hungry baker and reachable mill, but the baker does not know Bake Bread.
//
// Proves: AI does not generate the recipe-input firewood goal and trade
// valuation rejects receiving firewood as no-need.

#[test]
fn golden_no_known_recipe_suppresses_recipe_input_value() {
    let _ = run_recipe_input_snapshot_scenario(Seed([94; 32]), true, false);
}
