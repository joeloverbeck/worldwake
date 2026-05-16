use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldwake_core::{Discrepancy, GoalKind, PercentileBucket, Permille, Tick};

use crate::{PlanTerminalKind, SlotKind};

pub mod aggregator;

pub use aggregator::build_scenario_diagnostics;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScenarioDiagnosticsReport {
    pub tick_range: (Tick, Tick),
    pub goal_pressure: GoalPressureMetrics,
    pub planning: PlanningMetrics,
    pub revalidation_repair: RevalidationRepairMetrics,
    pub belief: BeliefMetrics,
    pub coordination: CoordinationMetrics,
    pub performance: PerformanceMetrics,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GoalPressureMetrics {
    pub candidates_emitted_by_kind: BTreeMap<GoalKind, u64>,
    pub candidates_emitted_by_slot: BTreeMap<SlotKind, u64>,
    pub candidates_suppressed_by_category: BTreeMap<CandidateSuppressionCategory, u64>,
    pub top_k_not_planned: BTreeMap<GoalKind, u64>,
    pub active_intention_continuation_rate: Permille,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanningMetrics {
    pub plan_attempts: u64,
    pub plan_attempts_by_kind: BTreeMap<GoalKind, u64>,
    pub budget_exhaustion_count: u64,
    pub budget_exhaustion_rate: Permille,
    pub frontier_exhaustion_count: u64,
    pub frontier_exhaustion_rate: Permille,
    pub beam_truncation_ratio: Permille,
    pub plan_depth: PercentileBucket,
    pub terminal_kind_distribution: BTreeMap<PlanTerminalKind, u64>,
    pub heuristic_helpful_action_hit_rate: Permille,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RevalidationRepairMetrics {
    pub invalidation_reasons: BTreeMap<Discrepancy, u64>,
    pub repair_attempts: u64,
    pub repair_succeeded: u64,
    pub repair_failed: u64,
    pub repair_success_rate: Permille,
    pub repair_budget_consumed: PercentileBucket,
    pub full_replan_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeliefMetrics {
    pub stale_belief_actions: u64,
    pub contradicted_belief_actions: u64,
    pub source_reliability_changes: u64,
    pub false_rumor_propagation_count: u64,
    pub correction_latency: PercentileBucket,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CoordinationMetrics {
    pub queue_wait_ticks: PercentileBucket,
    pub reservation_conflict_count: u64,
    pub abandoned_grant_count: u64,
    pub dead_claimant_cleanup_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub opportunity_compiled_count: PercentileBucket,
    pub opportunity_salience_floored: PercentileBucket,
    pub opportunity_learned_memory_damped: PercentileBucket,
    pub opportunity_cap_truncated: PercentileBucket,
    pub search_expansions: PercentileBucket,
    pub cache_hit_count: u64,
    pub cache_miss_count: u64,
    pub cache_invalidation_count: u64,
    pub planning_state_cache_entities_at_hits: u64,
    pub planning_state_cache_entities_at_misses: u64,
    pub planning_state_cache_effective_place_hits: u64,
    pub planning_state_cache_effective_place_misses: u64,
    pub planning_state_cache_invalidations: u64,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CandidateSuppressionCategory {
    RejectedLowerMotive,
    RejectedFeasibilityProbeFailed,
    RejectedSuppressedByBlocker,
    RejectedSuppressedByDiscrepancy,
    RejectedSuppressedByStressPolicy,
    RejectedSuppressedByContentionPreempt,
    RejectedArbitrationLost,
    RejectedSwitchMarginInsufficient,
    ZeroMotive,
    SoftDamped,
    FullyBlockedDesire,
    SituationallySuppressed,
    OmittedPolitical,
    OmittedBandit,
    OmittedSocial,
    OmittedViolationDetection,
}

#[cfg(test)]
mod tests {
    use super::{
        BeliefMetrics, CandidateSuppressionCategory, CoordinationMetrics, GoalPressureMetrics,
        PerformanceMetrics, PlanningMetrics, RevalidationRepairMetrics, ScenarioDiagnosticsReport,
    };
    use crate::{PlanTerminalKind, SlotKind};
    use std::collections::BTreeMap;
    use worldwake_core::{Discrepancy, GoalKind, PercentileBucket, Permille, Tick};

    #[test]
    fn scenario_diagnostics_report_round_trips_through_serde() {
        let report = populated_report();

        let encoded = bincode::serialize(&report).unwrap();
        let decoded: ScenarioDiagnosticsReport = bincode::deserialize(&encoded).unwrap();

        assert_eq!(decoded, report);
    }

    #[test]
    fn scenario_diagnostics_report_round_trips_through_json_for_string_keyed_values() {
        let report = populated_report();

        let encoded = serde_json::to_string(&report).unwrap();
        let decoded: ScenarioDiagnosticsReport = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, report);
    }

    #[test]
    fn candidate_suppression_category_is_ordered_and_serde_ready() {
        let mut counts = BTreeMap::new();
        counts.insert(CandidateSuppressionCategory::SoftDamped, 2);
        counts.insert(CandidateSuppressionCategory::RejectedLowerMotive, 1);
        counts.insert(CandidateSuppressionCategory::OmittedSocial, 3);

        let encoded = serde_json::to_string(&counts).unwrap();
        let decoded: BTreeMap<CandidateSuppressionCategory, u64> =
            serde_json::from_str(&encoded).unwrap();

        let ordered_keys: Vec<_> = decoded.keys().copied().collect();
        assert_eq!(
            ordered_keys,
            vec![
                CandidateSuppressionCategory::RejectedLowerMotive,
                CandidateSuppressionCategory::SoftDamped,
                CandidateSuppressionCategory::OmittedSocial,
            ]
        );
        assert_eq!(decoded, counts);
    }

    fn populated_report() -> ScenarioDiagnosticsReport {
        ScenarioDiagnosticsReport {
            tick_range: (Tick(3), Tick(9)),
            goal_pressure: GoalPressureMetrics {
                candidates_emitted_by_kind: BTreeMap::from([(GoalKind::Sleep, 4)]),
                candidates_emitted_by_slot: BTreeMap::from([(SlotKind::Survival, 3)]),
                candidates_suppressed_by_category: BTreeMap::from([(
                    CandidateSuppressionCategory::RejectedFeasibilityProbeFailed,
                    2,
                )]),
                top_k_not_planned: BTreeMap::from([(GoalKind::Wash, 1)]),
                active_intention_continuation_rate: Permille::new_unchecked(750),
            },
            planning: PlanningMetrics {
                plan_attempts: 7,
                plan_attempts_by_kind: BTreeMap::from([(GoalKind::Sleep, 5)]),
                budget_exhaustion_count: 1,
                budget_exhaustion_rate: Permille::new_unchecked(142),
                frontier_exhaustion_count: 2,
                frontier_exhaustion_rate: Permille::new_unchecked(285),
                beam_truncation_ratio: Permille::new_unchecked(125),
                plan_depth: PercentileBucket::from_sorted(&[1, 2, 4, 8]),
                terminal_kind_distribution: BTreeMap::from([(PlanTerminalKind::GoalSatisfied, 6)]),
                heuristic_helpful_action_hit_rate: Permille::new_unchecked(625),
            },
            revalidation_repair: RevalidationRepairMetrics {
                invalidation_reasons: BTreeMap::from([(Discrepancy::BeliefStale, 2)]),
                repair_attempts: 3,
                repair_succeeded: 2,
                repair_failed: 1,
                repair_success_rate: Permille::new_unchecked(666),
                repair_budget_consumed: PercentileBucket::from_sorted(&[2, 3, 5]),
                full_replan_count: 1,
            },
            belief: BeliefMetrics {
                stale_belief_actions: 2,
                contradicted_belief_actions: 1,
                source_reliability_changes: 3,
                false_rumor_propagation_count: 0,
                correction_latency: PercentileBucket::from_sorted(&[4, 9]),
            },
            coordination: CoordinationMetrics {
                queue_wait_ticks: PercentileBucket::from_sorted(&[1, 1, 5]),
                reservation_conflict_count: 2,
                abandoned_grant_count: 1,
                dead_claimant_cleanup_count: 1,
            },
            performance: PerformanceMetrics {
                opportunity_compiled_count: PercentileBucket::from_sorted(&[8, 13]),
                opportunity_salience_floored: PercentileBucket::from_sorted(&[1, 2]),
                opportunity_learned_memory_damped: PercentileBucket::from_sorted(&[0, 1]),
                opportunity_cap_truncated: PercentileBucket::from_sorted(&[0, 2]),
                search_expansions: PercentileBucket::from_sorted(&[5, 9, 12]),
                cache_hit_count: 10,
                cache_miss_count: 4,
                cache_invalidation_count: 0,
                planning_state_cache_entities_at_hits: 3,
                planning_state_cache_entities_at_misses: 2,
                planning_state_cache_effective_place_hits: 5,
                planning_state_cache_effective_place_misses: 4,
                planning_state_cache_invalidations: 1,
            },
        }
    }
}
