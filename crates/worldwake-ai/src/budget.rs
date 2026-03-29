use serde::{Deserialize, Serialize};
use worldwake_core::Permille;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanningBudget {
    pub max_candidates_to_plan: u8,
    pub max_plan_depth: u8,
    pub snapshot_travel_horizon: u8,
    pub max_prerequisite_locations: u8,
    pub max_node_expansions: u16,
    pub beam_width: u8,
    pub switch_margin_permille: Permille,
    pub transient_block_ticks: u32,
    pub unknown_block_ticks: u32,
    pub structural_block_ticks: u32,
    pub initial_cooldown_ticks: u32,
    pub max_cooldown_ticks: u32,
}

impl Default for PlanningBudget {
    fn default() -> Self {
        Self {
            max_candidates_to_plan: 2,
            max_plan_depth: 8,
            snapshot_travel_horizon: 6,
            max_prerequisite_locations: 3,
            max_node_expansions: 224,
            beam_width: 8,
            switch_margin_permille: Permille::new_unchecked(100),
            transient_block_ticks: 20,
            unknown_block_ticks: 5,
            structural_block_ticks: 200,
            initial_cooldown_ticks: 4,
            max_cooldown_ticks: 64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PlanningBudget;
    use worldwake_core::Permille;

    #[test]
    fn planning_budget_default_matches_ticket_values() {
        let budget = PlanningBudget::default();

        assert_eq!(budget.max_candidates_to_plan, 2);
        assert_eq!(budget.max_plan_depth, 8);
        assert_eq!(budget.snapshot_travel_horizon, 6);
        assert_eq!(budget.max_prerequisite_locations, 3);
        assert_eq!(budget.max_node_expansions, 224);
        assert_eq!(budget.beam_width, 8);
        assert_eq!(budget.switch_margin_permille, Permille::new(100).unwrap());
        assert_eq!(budget.transient_block_ticks, 20);
        assert_eq!(budget.unknown_block_ticks, 5);
        assert_eq!(budget.structural_block_ticks, 200);
        assert_eq!(budget.initial_cooldown_ticks, 4);
        assert_eq!(budget.max_cooldown_ticks, 64);
    }

    #[test]
    fn planning_budget_roundtrips_through_bincode() {
        let budget = PlanningBudget::default();

        let bytes = bincode::serialize(&budget).unwrap();
        let roundtrip: PlanningBudget = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, budget);
    }
}
