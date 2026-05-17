use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use worldwake_ai::{
    CandidateSuppressionCategory, PlanTerminalKind, ScenarioDiagnosticsReport, SlotKind,
    scenario_diagnostics::{
        BeliefMetrics, CoordinationMetrics, GoalPressureMetrics, MethodUsageCounts,
        PerformanceMetrics, PlanningMetrics, RevalidationRepairMetrics,
    },
};
use worldwake_core::{Discrepancy, GoalKind, MethodSchemaId, PercentileBucket, Permille, Tick};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct DiagnosticsMapEntry<K> {
    key: K,
    count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct MethodUsageDiagnosticsEntry {
    method_id: Option<MethodSchemaId>,
    counts: MethodUsageCounts,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct ScenarioDiagnosticsJson {
    tick_range: (Tick, Tick),
    goal_pressure: GoalPressureDiagnosticsJson,
    planning: PlanningDiagnosticsJson,
    revalidation_repair: RevalidationRepairDiagnosticsJson,
    belief: BeliefMetrics,
    coordination: CoordinationMetrics,
    performance: PerformanceMetrics,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct GoalPressureDiagnosticsJson {
    candidates_emitted_by_kind: Vec<DiagnosticsMapEntry<GoalKind>>,
    candidates_emitted_by_slot: Vec<DiagnosticsMapEntry<SlotKind>>,
    candidates_suppressed_by_category: Vec<DiagnosticsMapEntry<CandidateSuppressionCategory>>,
    top_k_not_planned: Vec<DiagnosticsMapEntry<GoalKind>>,
    active_intention_continuation_rate: Permille,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct PlanningDiagnosticsJson {
    plan_attempts: u64,
    plan_attempts_by_kind: Vec<DiagnosticsMapEntry<GoalKind>>,
    budget_exhaustion_count: u64,
    budget_exhaustion_rate: Permille,
    frontier_exhaustion_count: u64,
    frontier_exhaustion_rate: Permille,
    beam_truncation_ratio: Permille,
    plan_depth: PercentileBucket,
    terminal_kind_distribution: Vec<DiagnosticsMapEntry<PlanTerminalKind>>,
    heuristic_helpful_action_hit_rate: Permille,
    #[serde(default)]
    method_usage: Vec<MethodUsageDiagnosticsEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct RevalidationRepairDiagnosticsJson {
    invalidation_reasons: Vec<DiagnosticsMapEntry<Discrepancy>>,
    repair_attempts: u64,
    repair_succeeded: u64,
    repair_failed: u64,
    repair_success_rate: Permille,
    repair_budget_consumed: PercentileBucket,
    full_replan_count: u64,
}

fn diagnostics_map_entries<K: Copy + Ord>(map: &BTreeMap<K, u64>) -> Vec<DiagnosticsMapEntry<K>> {
    map.iter()
        .map(|(key, count)| DiagnosticsMapEntry {
            key: *key,
            count: *count,
        })
        .collect()
}

fn diagnostics_map_from_entries<K: Ord>(entries: Vec<DiagnosticsMapEntry<K>>) -> BTreeMap<K, u64> {
    entries
        .into_iter()
        .map(|entry| (entry.key, entry.count))
        .collect()
}

fn method_usage_entries(
    map: &BTreeMap<Option<MethodSchemaId>, MethodUsageCounts>,
) -> Vec<MethodUsageDiagnosticsEntry> {
    map.iter()
        .map(|(method_id, counts)| MethodUsageDiagnosticsEntry {
            method_id: *method_id,
            counts: counts.clone(),
        })
        .collect()
}

fn method_usage_from_entries(
    entries: Vec<MethodUsageDiagnosticsEntry>,
) -> BTreeMap<Option<MethodSchemaId>, MethodUsageCounts> {
    entries
        .into_iter()
        .map(|entry| (entry.method_id, entry.counts))
        .collect()
}

impl From<&ScenarioDiagnosticsReport> for ScenarioDiagnosticsJson {
    fn from(report: &ScenarioDiagnosticsReport) -> Self {
        Self {
            tick_range: report.tick_range,
            goal_pressure: GoalPressureDiagnosticsJson {
                candidates_emitted_by_kind: diagnostics_map_entries(
                    &report.goal_pressure.candidates_emitted_by_kind,
                ),
                candidates_emitted_by_slot: diagnostics_map_entries(
                    &report.goal_pressure.candidates_emitted_by_slot,
                ),
                candidates_suppressed_by_category: diagnostics_map_entries(
                    &report.goal_pressure.candidates_suppressed_by_category,
                ),
                top_k_not_planned: diagnostics_map_entries(&report.goal_pressure.top_k_not_planned),
                active_intention_continuation_rate: report
                    .goal_pressure
                    .active_intention_continuation_rate,
            },
            planning: PlanningDiagnosticsJson {
                plan_attempts: report.planning.plan_attempts,
                plan_attempts_by_kind: diagnostics_map_entries(
                    &report.planning.plan_attempts_by_kind,
                ),
                budget_exhaustion_count: report.planning.budget_exhaustion_count,
                budget_exhaustion_rate: report.planning.budget_exhaustion_rate,
                frontier_exhaustion_count: report.planning.frontier_exhaustion_count,
                frontier_exhaustion_rate: report.planning.frontier_exhaustion_rate,
                beam_truncation_ratio: report.planning.beam_truncation_ratio,
                plan_depth: report.planning.plan_depth.clone(),
                terminal_kind_distribution: diagnostics_map_entries(
                    &report.planning.terminal_kind_distribution,
                ),
                heuristic_helpful_action_hit_rate: report
                    .planning
                    .heuristic_helpful_action_hit_rate,
                method_usage: method_usage_entries(&report.planning.method_usage),
            },
            revalidation_repair: RevalidationRepairDiagnosticsJson {
                invalidation_reasons: diagnostics_map_entries(
                    &report.revalidation_repair.invalidation_reasons,
                ),
                repair_attempts: report.revalidation_repair.repair_attempts,
                repair_succeeded: report.revalidation_repair.repair_succeeded,
                repair_failed: report.revalidation_repair.repair_failed,
                repair_success_rate: report.revalidation_repair.repair_success_rate,
                repair_budget_consumed: report.revalidation_repair.repair_budget_consumed.clone(),
                full_replan_count: report.revalidation_repair.full_replan_count,
            },
            belief: report.belief.clone(),
            coordination: report.coordination.clone(),
            performance: report.performance.clone(),
        }
    }
}

impl From<ScenarioDiagnosticsJson> for ScenarioDiagnosticsReport {
    fn from(report: ScenarioDiagnosticsJson) -> Self {
        Self {
            tick_range: report.tick_range,
            goal_pressure: GoalPressureMetrics {
                candidates_emitted_by_kind: diagnostics_map_from_entries(
                    report.goal_pressure.candidates_emitted_by_kind,
                ),
                candidates_emitted_by_slot: diagnostics_map_from_entries(
                    report.goal_pressure.candidates_emitted_by_slot,
                ),
                candidates_suppressed_by_category: diagnostics_map_from_entries(
                    report.goal_pressure.candidates_suppressed_by_category,
                ),
                top_k_not_planned: diagnostics_map_from_entries(
                    report.goal_pressure.top_k_not_planned,
                ),
                active_intention_continuation_rate: report
                    .goal_pressure
                    .active_intention_continuation_rate,
            },
            planning: PlanningMetrics {
                plan_attempts: report.planning.plan_attempts,
                plan_attempts_by_kind: diagnostics_map_from_entries(
                    report.planning.plan_attempts_by_kind,
                ),
                budget_exhaustion_count: report.planning.budget_exhaustion_count,
                budget_exhaustion_rate: report.planning.budget_exhaustion_rate,
                frontier_exhaustion_count: report.planning.frontier_exhaustion_count,
                frontier_exhaustion_rate: report.planning.frontier_exhaustion_rate,
                beam_truncation_ratio: report.planning.beam_truncation_ratio,
                plan_depth: report.planning.plan_depth,
                terminal_kind_distribution: diagnostics_map_from_entries(
                    report.planning.terminal_kind_distribution,
                ),
                heuristic_helpful_action_hit_rate: report
                    .planning
                    .heuristic_helpful_action_hit_rate,
                method_usage: method_usage_from_entries(report.planning.method_usage),
            },
            revalidation_repair: RevalidationRepairMetrics {
                invalidation_reasons: diagnostics_map_from_entries(
                    report.revalidation_repair.invalidation_reasons,
                ),
                repair_attempts: report.revalidation_repair.repair_attempts,
                repair_succeeded: report.revalidation_repair.repair_succeeded,
                repair_failed: report.revalidation_repair.repair_failed,
                repair_success_rate: report.revalidation_repair.repair_success_rate,
                repair_budget_consumed: report.revalidation_repair.repair_budget_consumed,
                full_replan_count: report.revalidation_repair.full_replan_count,
            },
            belief: report.belief,
            coordination: report.coordination,
            performance: report.performance,
        }
    }
}

pub fn scenario_diagnostics_report_to_json_pretty(
    report: &ScenarioDiagnosticsReport,
) -> serde_json::Result<String> {
    serde_json::to_string_pretty(&ScenarioDiagnosticsJson::from(report))
}

pub fn scenario_diagnostics_report_from_json(
    json: &str,
) -> serde_json::Result<ScenarioDiagnosticsReport> {
    serde_json::from_str::<ScenarioDiagnosticsJson>(json).map(ScenarioDiagnosticsReport::from)
}
