mod golden_harness;

use std::path::PathBuf;

use golden_harness::GoldenHarness;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario};
use worldwake_core::{EventTag, EventView, Tick};

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-baseline.ron")
}

#[test]
fn survival_baseline_emits_goal_commit_and_plan_adoption_in_order() {
    let scenario = load_scenario_file(&scenario_path()).expect("survival-baseline should parse");
    let spawned = spawn_scenario(&scenario).expect("survival-baseline should spawn");
    let mut harness = GoldenHarness::from_simulation_state(&spawned.state);

    for _ in 0..40 {
        harness.step_once();
    }

    let committed = harness.event_log.events_by_tag(EventTag::GoalCommitted);
    let adopted = harness.event_log.events_by_tag(EventTag::PlanAdopted);
    assert!(
        !committed.is_empty(),
        "scenario should emit at least one GoalCommitted event by tick {}",
        harness.scheduler.current_tick().0
    );
    assert!(
        !adopted.is_empty(),
        "scenario should emit at least one PlanAdopted event by tick {}",
        harness.scheduler.current_tick().0
    );

    let found_same_tick_ordering = (0..=harness.scheduler.current_tick().0).any(|tick| {
        let tick = Tick(tick);
        let events = harness.event_log.events_at_tick(tick);
        let commit_pos = events.iter().position(|id| {
            harness
                .event_log
                .get(*id)
                .is_some_and(|event| event.tags().contains(&EventTag::GoalCommitted))
        });
        let adopt_pos = events.iter().position(|id| {
            harness
                .event_log
                .get(*id)
                .is_some_and(|event| event.tags().contains(&EventTag::PlanAdopted))
        });
        matches!((commit_pos, adopt_pos), (Some(commit), Some(adopt)) if commit < adopt)
    });

    assert!(
        found_same_tick_ordering,
        "expected at least one tick with GoalCommitted before PlanAdopted"
    );
}
