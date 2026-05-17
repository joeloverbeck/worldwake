use crate::numerics::Permille;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GoalPlanningBudget {
    pub max_depth: u8,
    pub max_node_expansions: u16,
    pub repair_budget_fraction: Permille,
    pub max_strategic_expansions: u16,
}

impl GoalPlanningBudget {
    pub const SELF_CARE: Self = Self {
        max_depth: 6,
        max_node_expansions: 96,
        repair_budget_fraction: Permille::new_unchecked(250),
        max_strategic_expansions: 12,
    };

    pub const TRAVEL_PURCHASE: Self = Self {
        max_depth: 10,
        max_node_expansions: 224,
        repair_budget_fraction: Permille::new_unchecked(300),
        max_strategic_expansions: 24,
    };

    pub const PRODUCTION: Self = Self {
        max_depth: 16,
        max_node_expansions: 384,
        repair_budget_fraction: Permille::new_unchecked(400),
        max_strategic_expansions: 48,
    };

    pub const INVESTIGATION: Self = Self {
        max_depth: 20,
        max_node_expansions: 512,
        repair_budget_fraction: Permille::new_unchecked(450),
        max_strategic_expansions: 64,
    };

    pub const BOUNTY_ESCORT: Self = Self {
        max_depth: 24,
        max_node_expansions: 768,
        repair_budget_fraction: Permille::new_unchecked(500),
        max_strategic_expansions: 96,
    };

    pub fn preset_name(&self) -> Option<&'static str> {
        if *self == Self::SELF_CARE {
            Some("SELF_CARE")
        } else if *self == Self::TRAVEL_PURCHASE {
            Some("TRAVEL_PURCHASE")
        } else if *self == Self::PRODUCTION {
            Some("PRODUCTION")
        } else if *self == Self::INVESTIGATION {
            Some("INVESTIGATION")
        } else if *self == Self::BOUNTY_ESCORT {
            Some("BOUNTY_ESCORT")
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_have_monotone_depth_ordering() {
        const {
            assert!(
                GoalPlanningBudget::SELF_CARE.max_depth
                    <= GoalPlanningBudget::TRAVEL_PURCHASE.max_depth
            );
            assert!(
                GoalPlanningBudget::TRAVEL_PURCHASE.max_depth
                    <= GoalPlanningBudget::PRODUCTION.max_depth
            );
            assert!(
                GoalPlanningBudget::PRODUCTION.max_depth
                    <= GoalPlanningBudget::INVESTIGATION.max_depth
            );
            assert!(
                GoalPlanningBudget::INVESTIGATION.max_depth
                    <= GoalPlanningBudget::BOUNTY_ESCORT.max_depth
            );
        }
    }

    #[test]
    fn presets_have_monotone_expansion_ordering() {
        const {
            assert!(
                GoalPlanningBudget::SELF_CARE.max_node_expansions
                    <= GoalPlanningBudget::TRAVEL_PURCHASE.max_node_expansions
            );
            assert!(
                GoalPlanningBudget::TRAVEL_PURCHASE.max_node_expansions
                    <= GoalPlanningBudget::PRODUCTION.max_node_expansions
            );
            assert!(
                GoalPlanningBudget::PRODUCTION.max_node_expansions
                    <= GoalPlanningBudget::INVESTIGATION.max_node_expansions
            );
            assert!(
                GoalPlanningBudget::INVESTIGATION.max_node_expansions
                    <= GoalPlanningBudget::BOUNTY_ESCORT.max_node_expansions
            );
        }
    }

    #[test]
    fn budget_roundtrips_through_bincode() {
        let budget = GoalPlanningBudget::BOUNTY_ESCORT;
        let bytes = bincode::serialize(&budget).unwrap();
        let roundtrip: GoalPlanningBudget = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, budget);
    }

    #[test]
    fn preset_name_returns_canonical_names() {
        assert_eq!(
            GoalPlanningBudget::SELF_CARE.preset_name(),
            Some("SELF_CARE")
        );
        assert_eq!(
            GoalPlanningBudget::TRAVEL_PURCHASE.preset_name(),
            Some("TRAVEL_PURCHASE")
        );
        assert_eq!(
            GoalPlanningBudget::PRODUCTION.preset_name(),
            Some("PRODUCTION")
        );
        assert_eq!(
            GoalPlanningBudget::INVESTIGATION.preset_name(),
            Some("INVESTIGATION")
        );
        assert_eq!(
            GoalPlanningBudget::BOUNTY_ESCORT.preset_name(),
            Some("BOUNTY_ESCORT")
        );

        let custom = GoalPlanningBudget {
            max_depth: 99,
            max_node_expansions: 999,
            repair_budget_fraction: Permille::new_unchecked(123),
            max_strategic_expansions: 17,
        };
        assert_eq!(custom.preset_name(), None);
    }
}
