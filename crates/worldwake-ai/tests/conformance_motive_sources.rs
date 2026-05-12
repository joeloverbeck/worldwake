//! Conformance checks for the S141 motive-source validation surface.
//!
//! The production candidate-generation seam must emit explicit motive-source
//! references. Synthetic unit fixtures can still construct empty vectors, but
//! the public generation path exercised here cannot.

mod golden_harness;

use golden_harness::*;
use worldwake_ai::generate_candidates;
use worldwake_core::{
    BlockerMemory, CommodityKind, HomeostaticNeeds, MetabolismProfile, Permille, Quantity, Seed,
    Tick, UtilityProfile,
};
use worldwake_sim::PerAgentBeliefView;

#[test]
fn every_goal_offer_has_motive_sources() {
    let mut harness = GoldenHarness::new(Seed([141; 32]));
    let agent = seed_agent(
        &mut harness.world,
        &mut harness.event_log,
        "Alice",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    give_commodity(
        &mut harness.world,
        &mut harness.event_log,
        agent,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(1),
    );
    seed_actor_local_beliefs(
        &mut harness.world,
        &mut harness.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );
    let beliefs = harness
        .world
        .get_component_agent_belief_store(agent)
        .expect("agent should have a belief store");
    let view = PerAgentBeliefView::new_at_tick_with_recipes(
        agent,
        Tick(0),
        &harness.world,
        Some(&harness.recipes),
        beliefs,
    );

    let candidates = generate_candidates(
        &view,
        agent,
        &BlockerMemory::default(),
        &harness.recipes,
        Tick(0),
    );

    assert!(
        !candidates.is_empty(),
        "representative setup should emit at least one production candidate"
    );
    let empty_sources = candidates
        .iter()
        .filter(|candidate| candidate.motive_sources.is_empty())
        .map(|candidate| candidate.key)
        .collect::<Vec<_>>();
    assert!(
        empty_sources.is_empty(),
        "production candidates without motive sources: {empty_sources:?}"
    );
}

#[test]
fn utility_profile_default_for_motive_class() {
    let profile = UtilityProfile::default();
    let fields = [
        ("office_duty_weight", profile.office_duty_weight),
        ("loyalty_weight", profile.loyalty_weight),
        ("greed_weight", profile.greed_weight),
        ("shame_weight", profile.shame_weight),
        ("revenge_weight", profile.revenge_weight),
    ];

    for (name, value) in fields {
        assert!(
            value > Permille::ZERO,
            "{name} should have a non-zero serde/default value"
        );
    }
    assert_eq!(profile.office_duty_weight.value(), 500);
    assert_eq!(profile.loyalty_weight.value(), 500);
    assert_eq!(profile.greed_weight.value(), 500);
    assert_eq!(profile.shame_weight.value(), 400);
    assert_eq!(profile.revenge_weight.value(), 400);
}
