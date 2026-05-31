//! S177 focused golden: muddy basin refill raises basin dirtiness and weakens a
//! later wash.

use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario};
use worldwake_core::{EntityId, HomeostaticNeeds, Tick};
use worldwake_sim::ActionTraceKind;

const TICKS: u32 = 24;

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/survival-muddy-basin-refill.ron")
}

fn named_entity(h: &GoldenHarness, target_name: &str) -> EntityId {
    h.world
        .query_name()
        .find(|(_, name)| name.0 == target_name)
        .map_or_else(
            || panic!("scenario should include named entity {target_name}"),
            |(entity, _)| entity,
        )
}

fn named_agent(h: &GoldenHarness, target_name: &str) -> EntityId {
    h.world
        .query_name_and_agent_data()
        .find(|(_, name, _)| name.0 == target_name)
        .map_or_else(
            || panic!("scenario should include named agent {target_name}"),
            |(entity, _, _)| entity,
        )
}

fn load_harness() -> GoldenHarness {
    let def = load_scenario_file(&scenario_path()).expect("scenario should parse");
    let spawned = spawn_scenario(&def).expect("scenario should spawn");
    let mut h = GoldenHarness::from_simulation_state(&spawned.state);
    h.driver.enable_tracing();
    h.enable_action_tracing();
    h
}

struct RefillObservation {
    initial_dirtiness: u16,
    post_refill_dirtiness: u16,
    pre_wash_needs: HomeostaticNeeds,
    post_wash_needs: HomeostaticNeeds,
    wash_committed: bool,
}

fn run() -> RefillObservation {
    let mut h = load_harness();
    let basin = named_entity(&h, "Muddy Wash Basin");
    let agent = named_agent(&h, "Muddy Washer");
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );

    let initial_dirtiness = h
        .world
        .get_component_wash_basin_state(basin)
        .expect("basin should have state")
        .dirtiness_level
        .value();

    h.step_once();
    let post_refill_dirtiness = h
        .world
        .get_component_wash_basin_state(basin)
        .expect("basin should have state")
        .dirtiness_level
        .value();

    let mut pre_wash_needs = h
        .world
        .get_component_homeostatic_needs(agent)
        .copied()
        .expect("agent should have needs");
    let mut post_wash_needs = pre_wash_needs;
    let mut wash_committed = false;

    for _ in 1..TICKS {
        pre_wash_needs = h
            .world
            .get_component_homeostatic_needs(agent)
            .copied()
            .expect("agent should have needs");
        h.step_once();
        let action_sink = h.action_trace_sink().expect("action tracing enabled");
        if action_sink.events_for(agent).iter().any(|event| {
            event.action_name == "wash" && matches!(event.kind, ActionTraceKind::Committed { .. })
        }) {
            post_wash_needs = h
                .world
                .get_component_homeostatic_needs(agent)
                .copied()
                .expect("agent should have needs");
            wash_committed = true;
            break;
        }
    }

    RefillObservation {
        initial_dirtiness,
        post_refill_dirtiness,
        pre_wash_needs,
        post_wash_needs,
        wash_committed,
    }
}

// Scenario 494: Muddy Basin Refill Raises Dirtiness Level
// ---------------------------------------------------------------------------
//
// Systems: ItemDecay, Dirtiness
// GoalKinds: Wash
// ActionDomains: Needs
// Places: Muddy Wash Camp
// Principles: 3, 4, 26, 31
//
// Setup: an empty basin is colocated with only a muddy water source.
//
// Proves: the item-decay refill transfers water and raises the basin's concrete dirtiness level by the authored muddy-water penalty.
#[test]
fn golden_muddy_basin_refill_raises_dirtiness_level() {
    let obs = run();
    assert_eq!(obs.initial_dirtiness, 0);
    assert_eq!(obs.post_refill_dirtiness, 80);
}

// Scenario 495: Muddy Basin Refill Degrades Wash Effectiveness
// ---------------------------------------------------------------------------
//
// Systems: ItemDecay, Needs
// GoalKinds: Wash
// ActionDomains: Needs
// Places: Muddy Wash Camp
// Principles: 3, 10, 26, 31
//
// Setup: after muddy-water refill raises basin dirtiness, the agent washes from that now-degraded basin.
//
// Proves: wash commits and reduces dirtiness by less than a full clean-basin reset would have from the sampled pre-wash need.
#[test]
fn golden_muddy_basin_refill_degrades_wash_effectiveness() {
    let obs = run();
    assert!(obs.wash_committed, "agent should commit a wash action");
    assert!(
        obs.post_wash_needs.dirtiness > worldwake_core::Permille::ZERO,
        "muddy-refilled basin should not fully reset dirtiness; pre={:?}, post={:?}",
        obs.pre_wash_needs,
        obs.post_wash_needs
    );
    assert!(
        obs.post_wash_needs.dirtiness < obs.pre_wash_needs.dirtiness,
        "wash should still provide partial relief"
    );
}
