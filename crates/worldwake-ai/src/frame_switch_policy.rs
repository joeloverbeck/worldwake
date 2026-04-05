use crate::{
    FramePlanRelation, GoalPriorityClass,
    goal_switching::{GoalSwitchKind, compare_goal_switch},
};
use worldwake_core::Permille;

#[must_use]
pub(crate) fn switch_margin_for_relation(
    relation: FramePlanRelation,
    default_switch_margin: Permille,
    frame_switch_margin: Permille,
) -> Permille {
    match relation {
        FramePlanRelation::AbandonsFrame => frame_switch_margin,
        FramePlanRelation::NoFrame
        | FramePlanRelation::RefreshesFrame
        | FramePlanRelation::SuspendsFrame => default_switch_margin,
    }
}

#[must_use]
pub(crate) fn compare_relation_aware_goal_switch(
    current_class: GoalPriorityClass,
    current_motive: Option<u32>,
    challenger_class: GoalPriorityClass,
    challenger_motive: u32,
    relation: FramePlanRelation,
    default_switch_margin: Permille,
    frame_switch_margin: Permille,
) -> Option<GoalSwitchKind> {
    compare_goal_switch(
        current_class,
        current_motive,
        challenger_class,
        challenger_motive,
        switch_margin_for_relation(relation, default_switch_margin, frame_switch_margin),
    )
}

#[cfg(test)]
mod tests {
    use super::{compare_relation_aware_goal_switch, switch_margin_for_relation};
    use crate::{FramePlanRelation, GoalPriorityClass};
    use worldwake_core::Permille;

    fn default_switch_margin() -> Permille {
        Permille::new(100).unwrap()
    }

    fn frame_switch_margin() -> Permille {
        Permille::new(300).unwrap()
    }

    #[test]
    fn abandons_frame_uses_frame_margin() {
        assert_eq!(
            switch_margin_for_relation(
                FramePlanRelation::AbandonsFrame,
                default_switch_margin(),
                frame_switch_margin(),
            ),
            frame_switch_margin()
        );
    }

    #[test]
    fn suspended_frame_uses_default_margin() {
        assert_eq!(
            switch_margin_for_relation(
                FramePlanRelation::SuspendsFrame,
                default_switch_margin(),
                frame_switch_margin(),
            ),
            default_switch_margin()
        );
    }

    #[test]
    fn relation_aware_switch_applies_relation_specific_margin() {
        assert_eq!(
            compare_relation_aware_goal_switch(
                GoalPriorityClass::High,
                Some(1_000),
                GoalPriorityClass::High,
                1_350,
                FramePlanRelation::AbandonsFrame,
                default_switch_margin(),
                Permille::new(400).unwrap(),
            ),
            None
        );
        assert_eq!(
            compare_relation_aware_goal_switch(
                GoalPriorityClass::High,
                Some(1_000),
                GoalPriorityClass::High,
                1_350,
                FramePlanRelation::SuspendsFrame,
                default_switch_margin(),
                Permille::new(400).unwrap(),
            ),
            Some(crate::goal_switching::GoalSwitchKind::SameClassMargin)
        );
    }
}
