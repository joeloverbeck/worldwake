use crate::golden_harness::*;
use worldwake_ai::{
    ActionTraceSnapshot, GoalKind, GoalPriorityClass, SelectedPlanSource, SurvivalForensicExtractor,
};
use worldwake_core::{DriveThresholds, EntityId, GoalKey, HomeostaticNeeds, Tick};

#[test]
fn forensic_determinism() {
    let first = run_sequence();
    let second = run_sequence();
    expect_deterministic_reports(&first, &second);
}

fn run_sequence() -> Vec<worldwake_ai::CriticalWindowReport> {
    let agent = EntityId {
        slot: 3,
        generation: 0,
    };
    let sleep_goal = GoalKey::from(GoalKind::Sleep);
    let thresholds = DriveThresholds::default();
    let local_state = sample_local_survival_state_summary();
    let mut extractor = SurvivalForensicExtractor::new(agent);

    for tick in 1..=100 {
        let trace = synthetic_planning_trace(
            agent,
            Tick(tick),
            sleep_goal,
            vec![synthetic_ranked_goal_summary(
                sleep_goal,
                GoalPriorityClass::Critical,
                975,
            )],
            Some(SelectedPlanSource::SearchSelection),
            worldwake_ai::PlanSearchOutcome::Found {
                steps: Vec::new(),
                terminal_kind: worldwake_ai::PlanTerminalKind::GoalSatisfied,
            },
        );
        extractor.observe(
            Tick(tick),
            &HomeostaticNeeds::new(pm(0), pm(0), pm(940), pm(0), pm(0)),
            &thresholds,
            Some(&trace),
            &ActionTraceSnapshot::empty(),
            &local_state,
            false,
        );
    }

    extractor.finalize()
}
