use crate::golden_harness::*;
use worldwake_ai::{
    ActionTraceSnapshot, GoalKind, GoalPriorityClass, PlanSearchOutcome, SelectedPlanSource,
    SurvivalForensicExtractor,
};
use worldwake_core::{DriveThresholds, EntityId, EventLog, GoalKey, HomeostaticNeeds, Tick};

#[test]
fn forensic_sleep_progress_barrier() {
    let agent = EntityId {
        slot: 1,
        generation: 0,
    };
    let sleep_goal = GoalKey::from(GoalKind::Sleep);
    let thresholds = DriveThresholds::default();
    let local_state = sample_local_survival_state_summary();
    let mut extractor = SurvivalForensicExtractor::new(agent);

    for tick in 1..=200 {
        let trace = synthetic_planning_trace(
            agent,
            Tick(tick),
            sleep_goal,
            vec![synthetic_ranked_goal_summary(
                sleep_goal,
                GoalPriorityClass::Critical,
                980,
            )],
            Some(SelectedPlanSource::SearchSelection),
            PlanSearchOutcome::FrontierExhausted {
                expansions_used: 21,
            },
        );
        extractor.observe(
            Tick(tick),
            &HomeostaticNeeds::new(pm(0), pm(0), pm(950), pm(0), pm(0)),
            &thresholds,
            Some(&trace),
            &ActionTraceSnapshot::empty(),
            &EventLog::new(),
            &local_state,
            false,
        );
    }

    let reports = extractor.finalize();
    expect_sleep_progress_barrier_window(&reports);
}
