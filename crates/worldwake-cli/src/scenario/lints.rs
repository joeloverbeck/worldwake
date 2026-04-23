use std::collections::{BTreeMap, BTreeSet};

use crate::scenario::types::{AgentDef, ScenarioDef};
use serde::Deserialize;
use worldwake_core::{ControlSource, Permille};

#[derive(Clone, Debug, Default)]
pub struct LintReport {
    pub failures: Vec<LintFailure>,
    pub warnings: Vec<LintWarning>,
}

#[derive(Clone, Debug)]
pub struct LintFailure {
    pub rule: LintRule,
    pub affected_agents: Vec<String>,
    pub detail: String,
}

#[derive(Clone, Debug)]
pub struct LintWarning {
    pub rule: LintRule,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
pub enum LintRule {
    ProfileHomogeneity,
    UnreachableExplorationDrive,
    AuthoritativeHelperOnSnapshot,
}

pub fn run_lints(scenario: &ScenarioDef) -> LintReport {
    let mut report = LintReport::default();
    check_profile_homogeneity(scenario, &mut report);
    check_unreachable_exploration_drive(scenario, &mut report);
    report
}

pub fn filter_overrides(
    mut report: LintReport,
    overrides: &BTreeMap<LintRule, String>,
) -> Result<LintReport, super::ScenarioError> {
    for (rule, justification) in overrides {
        if justification.trim().is_empty() {
            return Err(super::ScenarioError::Validation(format!(
                "lint override for {rule:?} requires a non-empty justification string"
            )));
        }
    }

    let suppressed: BTreeSet<LintRule> = overrides.keys().copied().collect();
    report
        .failures
        .retain(|failure| !suppressed.contains(&failure.rule));
    Ok(report)
}

fn check_profile_homogeneity(scenario: &ScenarioDef, report: &mut LintReport) {
    let ai_agents: Vec<&AgentDef> = scenario
        .agents
        .iter()
        .filter(|agent| agent.control == ControlSource::Ai)
        .collect();

    if ai_agents.len() <= 2 {
        return;
    }

    let varies = option_field_varies(&ai_agents, |agent| agent.cognitive_profile.as_ref())
        || option_field_varies(&ai_agents, |agent| agent.utility_profile.as_ref())
        || option_field_varies(&ai_agents, |agent| agent.perception_profile.as_ref())
        || option_field_varies(&ai_agents, |agent| agent.exploration_profile.as_ref())
        || option_field_varies(&ai_agents, |agent| agent.diversification_profile.as_ref())
        || option_field_varies(&ai_agents, |agent| agent.epistemic_disposition.as_ref())
        || option_field_varies(&ai_agents, |agent| agent.intention_disposition.as_ref())
        || option_field_varies(&ai_agents, |agent| agent.last_seen_memory.as_ref());

    if varies {
        return;
    }

    report.failures.push(LintFailure {
        rule: LintRule::ProfileHomogeneity,
        affected_agents: ai_agents.iter().map(|agent| agent.name.clone()).collect(),
        detail:
            "AI agent population shares default profiles across all checked fields; FND-22 requires concrete per-agent variation"
                .into(),
    });
}

fn option_field_varies<T: PartialEq>(
    agents: &[&AgentDef],
    accessor: impl Fn(&AgentDef) -> Option<&T>,
) -> bool {
    for (idx, left) in agents.iter().enumerate() {
        let left_value = accessor(left);
        for right in &agents[(idx + 1)..] {
            let right_value = accessor(right);
            if left_value != right_value {
                return true;
            }
        }
    }
    false
}

fn check_unreachable_exploration_drive(scenario: &ScenarioDef, report: &mut LintReport) {
    let zero = Permille::new_unchecked(0);

    for agent in scenario
        .agents
        .iter()
        .filter(|agent| agent.control == ControlSource::Ai)
    {
        let Some(exploration) = agent.exploration_profile else {
            continue;
        };

        let exploration_zero = exploration.curiosity_weight == zero;
        let diversification_absent_or_zero = agent
            .diversification_profile
            .as_ref()
            .is_none_or(|profile| profile.base_curiosity == zero);

        if exploration_zero && diversification_absent_or_zero {
            report.failures.push(LintFailure {
                rule: LintRule::UnreachableExplorationDrive,
                affected_agents: vec![agent.name.clone()],
                detail:
                    "ExplorationProfileDef.curiosity_weight == 0 and no DiversificationProfile (or base_curiosity == 0); exploration drive can never fire"
                        .into(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LintRule, filter_overrides, run_lints};
    use crate::scenario::types::{AgentDef, ExplorationProfileDef, PlaceDef, ScenarioDef};
    use crate::scenario::{ScenarioError, spawn_scenario};
    use std::collections::BTreeMap;
    use worldwake_core::{
        CognitiveProfile, ControlSource, DiversificationProfile, EpistemicDispositionProfile,
        IntentionDispositionProfile, PerceptionProfile, Permille, PlaceTag, UtilityProfile,
    };

    fn minimal_agent(name: &str, control: ControlSource) -> AgentDef {
        AgentDef {
            name: name.into(),
            location: "Town".into(),
            control,
            needs: None,
            combat_profile: None,
            utility_profile: None,
            artifact_posting_profile: None,
            merchandise_profile: None,
            trade_disposition: None,
            perception_profile: None,
            tell_profile: None,
            cognitive_profile: None,
            agenda_profile: None,
            execution_budget: None,
            epistemic_disposition: None,
            intention_disposition: None,
            communication_profile: None,
            preference_profile: None,
            expectation_store: None,
            last_seen_memory: None,
            social_observations: None,
            obligation_satiation_profile: None,
            drive_thresholds: None,
            drive_escalation_profile: None,
            metabolism_profile: None,
            disposal_profile: None,
            exploration_profile: None,
            diversification_profile: None,
            carry_capacity: None,
            theft_disposition: None,
            justice_disposition: None,
            violation_disposition: None,
            patrol_profile: None,
            patrol_route: None,
            pursuit_profile: None,
            contention_disposition: None,
            commodity_valuation: None,
            substitute_preferences: None,
            known_recipes: None,
        }
    }

    fn scenario_with_agents(agents: Vec<AgentDef>) -> ScenarioDef {
        ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Town".into(),
                tags: vec![PlaceTag::Village],
                visibility_profile: None,
            }],
            edges: vec![],
            agents,
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
        }
    }

    fn default_exploration_profile() -> ExplorationProfileDef {
        ExplorationProfileDef {
            curiosity_weight: Permille::new_unchecked(500),
            need_activation_threshold: Permille::new_unchecked(500),
            frontier_depth: 2,
            acquisition_failure_threshold: 3,
            exploration_arrival_boost: Permille::new_unchecked(500),
            max_consecutive_explorations: 2,
            visit_lookback_ticks: 10,
        }
    }

    fn fully_profiled_ai(name: &str) -> AgentDef {
        AgentDef {
            cognitive_profile: Some(CognitiveProfile::default()),
            utility_profile: Some(UtilityProfile::default()),
            perception_profile: Some(PerceptionProfile::default()),
            exploration_profile: Some(default_exploration_profile()),
            diversification_profile: Some(DiversificationProfile::default()),
            epistemic_disposition: Some(EpistemicDispositionProfile::default()),
            intention_disposition: Some(IntentionDispositionProfile::default()),
            last_seen_memory: Some(crate::scenario::types::LastSeenMemoryDef::default()),
            ..minimal_agent(name, ControlSource::Ai)
        }
    }

    #[test]
    fn homogeneous_population_fails_lint() {
        let scenario = scenario_with_agents(vec![
            fully_profiled_ai("Alice"),
            fully_profiled_ai("Bob"),
            fully_profiled_ai("Cara"),
        ]);

        let report = run_lints(&scenario);
        let failure = report
            .failures
            .iter()
            .find(|failure| failure.rule == LintRule::ProfileHomogeneity)
            .expect("homogeneous AI population should fail lint");

        assert_eq!(
            failure.affected_agents,
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()]
        );
    }

    #[test]
    fn varied_population_passes_lint() {
        let mut varied = fully_profiled_ai("Cara");
        varied.cognitive_profile = Some(CognitiveProfile {
            max_plan_depth: 12,
            ..CognitiveProfile::default()
        });
        let scenario = scenario_with_agents(vec![
            fully_profiled_ai("Alice"),
            fully_profiled_ai("Bob"),
            varied,
        ]);

        let report = run_lints(&scenario);

        assert!(
            !report
                .failures
                .iter()
                .any(|failure| failure.rule == LintRule::ProfileHomogeneity)
        );
    }

    #[test]
    fn zero_curiosity_no_diversification_fails_lint() {
        let mut agent = minimal_agent("Scout", ControlSource::Ai);
        agent.exploration_profile = Some(ExplorationProfileDef {
            curiosity_weight: Permille::new_unchecked(0),
            ..default_exploration_profile()
        });
        let scenario = scenario_with_agents(vec![agent]);

        let report = run_lints(&scenario);

        assert!(report.failures.iter().any(|failure| {
            failure.rule == LintRule::UnreachableExplorationDrive
                && failure.affected_agents == vec!["Scout".to_string()]
        }));
    }

    #[test]
    fn population_under_three_exempt_from_homogeneity() {
        let scenario =
            scenario_with_agents(vec![fully_profiled_ai("Alice"), fully_profiled_ai("Bob")]);

        let report = run_lints(&scenario);

        assert!(
            !report
                .failures
                .iter()
                .any(|failure| failure.rule == LintRule::ProfileHomogeneity)
        );
    }

    #[test]
    fn human_only_population_exempt_from_homogeneity() {
        let scenario = scenario_with_agents(vec![
            fully_profiled_human("Alice"),
            fully_profiled_human("Bob"),
            fully_profiled_human("Cara"),
        ]);

        let report = run_lints(&scenario);

        assert!(
            !report
                .failures
                .iter()
                .any(|failure| failure.rule == LintRule::ProfileHomogeneity)
        );
        assert!(report.failures.is_empty());
    }

    #[test]
    fn lint_report_accumulates_failures_across_rules() {
        let scenario = scenario_with_agents(vec![
            unreachable_homogeneous_ai("Alice"),
            unreachable_homogeneous_ai("Bob"),
            unreachable_homogeneous_ai("Cara"),
        ]);

        let report = run_lints(&scenario);

        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.rule == LintRule::ProfileHomogeneity)
        );
        assert_eq!(
            report
                .failures
                .iter()
                .filter(|failure| failure.rule == LintRule::UnreachableExplorationDrive)
                .count(),
            3
        );
    }

    #[test]
    fn override_with_justification_suppresses_failure() {
        let mut scenario = scenario_with_agents(vec![
            fully_profiled_ai("Alice"),
            fully_profiled_ai("Bob"),
            fully_profiled_ai("Cara"),
        ]);
        scenario.scenario_lint_overrides.insert(
            LintRule::ProfileHomogeneity,
            "covers identical-twin regression".into(),
        );

        let report =
            filter_overrides(run_lints(&scenario), &scenario.scenario_lint_overrides).unwrap();

        assert!(
            !report
                .failures
                .iter()
                .any(|failure| failure.rule == LintRule::ProfileHomogeneity)
        );
    }

    #[test]
    fn override_with_empty_justification_returns_validation_error() {
        let mut scenario = scenario_with_agents(vec![
            fully_profiled_ai("Alice"),
            fully_profiled_ai("Bob"),
            fully_profiled_ai("Cara"),
        ]);
        scenario
            .scenario_lint_overrides
            .insert(LintRule::ProfileHomogeneity, String::new());

        let error =
            filter_overrides(run_lints(&scenario), &scenario.scenario_lint_overrides).unwrap_err();

        match error {
            ScenarioError::Validation(message) => {
                assert!(message.contains("ProfileHomogeneity"));
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn unsuppressed_failure_short_circuits_spawn() {
        let scenario = scenario_with_agents(vec![
            fully_profiled_ai("Alice"),
            fully_profiled_ai("Bob"),
            fully_profiled_ai("Cara"),
        ]);

        match spawn_scenario(&scenario) {
            Err(ScenarioError::LintFailure(report)) => {
                assert!(
                    report
                        .failures
                        .iter()
                        .any(|failure| failure.rule == LintRule::ProfileHomogeneity)
                );
            }
            Ok(_) => panic!("expected lint failure"),
            Err(other) => panic!("expected lint failure, got {other:?}"),
        }
    }

    fn fully_profiled_human(name: &str) -> AgentDef {
        AgentDef {
            cognitive_profile: Some(CognitiveProfile::default()),
            utility_profile: Some(UtilityProfile::default()),
            perception_profile: Some(PerceptionProfile::default()),
            exploration_profile: Some(default_exploration_profile()),
            diversification_profile: Some(DiversificationProfile::default()),
            epistemic_disposition: Some(EpistemicDispositionProfile::default()),
            intention_disposition: Some(IntentionDispositionProfile::default()),
            last_seen_memory: Some(crate::scenario::types::LastSeenMemoryDef::default()),
            ..minimal_agent(name, ControlSource::Human)
        }
    }

    fn unreachable_homogeneous_ai(name: &str) -> AgentDef {
        AgentDef {
            cognitive_profile: Some(CognitiveProfile::default()),
            utility_profile: Some(UtilityProfile::default()),
            perception_profile: Some(PerceptionProfile::default()),
            exploration_profile: Some(ExplorationProfileDef {
                curiosity_weight: Permille::new_unchecked(0),
                ..default_exploration_profile()
            }),
            epistemic_disposition: Some(EpistemicDispositionProfile::default()),
            intention_disposition: Some(IntentionDispositionProfile::default()),
            last_seen_memory: Some(crate::scenario::types::LastSeenMemoryDef::default()),
            ..minimal_agent(name, ControlSource::Ai)
        }
    }
}
