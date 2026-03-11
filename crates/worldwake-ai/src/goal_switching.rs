use crate::{GoalPriorityClass, PlanningBudget};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum GoalSwitchKind {
    HigherPriorityGoal,
    SameClassMargin,
}

pub(crate) fn compare_goal_switch(
    current_class: GoalPriorityClass,
    current_motive: Option<u32>,
    challenger_class: GoalPriorityClass,
    challenger_motive: u32,
    budget: &PlanningBudget,
) -> Option<GoalSwitchKind> {
    if challenger_class > current_class {
        return Some(GoalSwitchKind::HigherPriorityGoal);
    }
    if challenger_class < current_class {
        return None;
    }

    let current_motive = current_motive?;
    clears_switch_margin(challenger_motive, current_motive, budget)
        .then_some(GoalSwitchKind::SameClassMargin)
}

fn clears_switch_margin(new_score: u32, current_score: u32, budget: &PlanningBudget) -> bool {
    if new_score <= current_score {
        return false;
    }
    if current_score == 0 {
        return true;
    }
    let required_increase = (u64::from(current_score) * u64::from(budget.switch_margin_permille.value()))
        .div_ceil(1000);
    u64::from(new_score) >= u64::from(current_score) + required_increase
}

#[cfg(test)]
mod tests {
    use super::{compare_goal_switch, GoalSwitchKind};
    use crate::{GoalPriorityClass, PlanningBudget};

    #[test]
    fn challenger_with_higher_priority_always_switches() {
        let decision = compare_goal_switch(
            GoalPriorityClass::Low,
            Some(1),
            GoalPriorityClass::High,
            1,
            &PlanningBudget::default(),
        );

        assert_eq!(decision, Some(GoalSwitchKind::HigherPriorityGoal));
    }

    #[test]
    fn same_class_switch_requires_margin() {
        let budget = PlanningBudget::default();

        assert_eq!(
            compare_goal_switch(
                GoalPriorityClass::Medium,
                Some(1000),
                GoalPriorityClass::Medium,
                1099,
                &budget,
            ),
            None
        );
        assert_eq!(
            compare_goal_switch(
                GoalPriorityClass::Medium,
                Some(1000),
                GoalPriorityClass::Medium,
                1100,
                &budget,
            ),
            Some(GoalSwitchKind::SameClassMargin)
        );
    }

    #[test]
    fn same_class_switch_without_current_motive_is_disallowed() {
        let decision = compare_goal_switch(
            GoalPriorityClass::Medium,
            None,
            GoalPriorityClass::Medium,
            1200,
            &PlanningBudget::default(),
        );

        assert_eq!(decision, None);
    }
}
