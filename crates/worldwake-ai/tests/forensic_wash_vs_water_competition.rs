mod golden_harness;

use golden_harness::*;
use worldwake_ai::{
    ActionTraceSnapshot, GoalPriorityClass, SelectedPlanSource, SurvivalForensicExtractor,
};
use worldwake_core::{
    AcquisitionQuantity, CommodityKind, CommodityPurpose, DriveThresholds, EntityId, GoalKey,
    GoalKind, HomeostaticNeeds, Tick,
};

#[test]
fn forensic_wash_vs_water_competition() {
    let agent = EntityId {
        slot: 2,
        generation: 0,
    };
    let wash_goal = GoalKey::from(GoalKind::Wash);
    let water_goal = GoalKey::from(GoalKind::AcquireCommodity {
        commodity: CommodityKind::Water,
        purpose: CommodityPurpose::SelfConsume,
        quantity: AcquisitionQuantity::single(),
    });
    let sleep_goal = GoalKey::from(GoalKind::Sleep);
    let thresholds = DriveThresholds::default();
    let local_state = sample_local_survival_state_summary();
    let mut extractor = SurvivalForensicExtractor::new(agent);

    for tick in 1..=150 {
        let selected_goal = if tick % 2 == 0 { wash_goal } else { water_goal };
        let ranked = if selected_goal == wash_goal {
            vec![
                synthetic_ranked_goal_summary(wash_goal, GoalPriorityClass::Critical, 990),
                synthetic_ranked_goal_summary(water_goal, GoalPriorityClass::Critical, 980),
                synthetic_ranked_goal_summary(sleep_goal, GoalPriorityClass::High, 850),
            ]
        } else {
            vec![
                synthetic_ranked_goal_summary(water_goal, GoalPriorityClass::Critical, 990),
                synthetic_ranked_goal_summary(wash_goal, GoalPriorityClass::Critical, 980),
                synthetic_ranked_goal_summary(sleep_goal, GoalPriorityClass::High, 850),
            ]
        };
        let trace = synthetic_planning_trace(
            agent,
            Tick(tick),
            selected_goal,
            ranked,
            Some(SelectedPlanSource::SearchSelection),
            worldwake_ai::PlanSearchOutcome::Found {
                steps: Vec::new(),
                terminal_kind: worldwake_ai::PlanTerminalKind::GoalSatisfied,
            },
        );
        extractor.observe(
            Tick(tick),
            &HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(950)),
            &thresholds,
            Some(&trace),
            &ActionTraceSnapshot::empty(),
            &local_state,
        );
    }

    let reports = extractor.finalize();
    expect_wash_vs_water_competition_window(&reports);
}
