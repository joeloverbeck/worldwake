use serde::{Deserialize, Serialize};
use worldwake_core::Permille;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanningBudget {
    pub max_candidates_to_plan: u8,
    pub max_plan_depth: u8,
    pub snapshot_travel_horizon: u8,
    pub max_node_expansions: u16,
    pub beam_width: u8,
    pub switch_margin_permille: Permille,
    pub transient_block_ticks: u32,
    pub structural_block_ticks: u32,
}

impl Default for PlanningBudget {
    fn default() -> Self {
        Self {
            max_candidates_to_plan: 4,
            max_plan_depth: 6,
            snapshot_travel_horizon: 6,
            max_node_expansions: 512,
            beam_width: 8,
            switch_margin_permille: Permille::new_unchecked(100),
            transient_block_ticks: 20,
            structural_block_ticks: 200,
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

        assert_eq!(budget.max_candidates_to_plan, 4);
        assert_eq!(budget.max_plan_depth, 6);
        assert_eq!(budget.snapshot_travel_horizon, 6);
        assert_eq!(budget.max_node_expansions, 512);
        assert_eq!(budget.beam_width, 8);
        assert_eq!(budget.switch_margin_permille, Permille::new(100).unwrap());
        assert_eq!(budget.transient_block_ticks, 20);
        assert_eq!(budget.structural_block_ticks, 200);
    }

    #[test]
    fn planning_budget_roundtrips_through_bincode() {
        let budget = PlanningBudget::default();

        let bytes = bincode::serialize(&budget).unwrap();
        let roundtrip: PlanningBudget = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, budget);
    }
}
