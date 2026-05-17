use std::collections::{BTreeMap, BTreeSet};

use crate::scenario::types::{AgentDef, ScenarioDef};
use serde::Deserialize;
use worldwake_core::{ControlSource, Permille, Quantity};

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
    TreasuryAuthoredWithMissingSeat,
    TreasuryAuthoredWithZeroQuantity,
}

pub fn run_lints(scenario: &ScenarioDef) -> LintReport {
    let mut report = LintReport::default();
    check_profile_homogeneity(scenario, &mut report);
    check_unreachable_exploration_drive(scenario, &mut report);
    check_office_treasuries(scenario, &mut report);
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
        || utility_profile_motive_class_weight_varies(&ai_agents)
        || option_field_varies(&ai_agents, |agent| agent.perception_profile.as_ref())
        || option_field_varies(&ai_agents, |agent| agent.exploration_profile.as_ref())
        || option_field_varies(&ai_agents, |agent| agent.diversification_profile.as_ref())
        || option_field_varies(&ai_agents, |agent| agent.epistemic_disposition.as_ref())
        || option_field_varies(&ai_agents, |agent| agent.intention_disposition.as_ref())
        || option_field_varies(&ai_agents, |agent| agent.last_seen_memory.as_ref())
        || agent_schema_context_profile_varies(&ai_agents);

    if varies {
        return;
    }

    report.failures.push(LintFailure {
        rule: LintRule::ProfileHomogeneity,
        affected_agents: ai_agents.iter().map(|agent| agent.name.clone()).collect(),
        detail:
            "AI agent population shares profiles across all checked fields, including agent_schema_context_profile.disabled_extractors, agent_schema_context_profile.budget_overrides, and agent_schema_context_profile.disabled_methods; FND-22 requires concrete per-agent variation"
                .into(),
    });
}

fn agent_schema_context_profile_varies(agents: &[&AgentDef]) -> bool {
    option_field_varies(agents, |agent| {
        agent
            .agent_schema_context_profile
            .as_ref()
            .map(|profile| &profile.disabled_extractors)
    }) || option_field_varies(agents, |agent| {
        agent
            .agent_schema_context_profile
            .as_ref()
            .map(|profile| &profile.budget_overrides)
    }) || option_field_varies(agents, |agent| {
        agent
            .agent_schema_context_profile
            .as_ref()
            .map(|profile| &profile.disabled_methods)
    })
}

fn utility_profile_motive_class_weight_varies(agents: &[&AgentDef]) -> bool {
    option_field_varies(agents, |agent| {
        agent
            .utility_profile
            .as_ref()
            .map(|profile| &profile.office_duty_weight)
    }) || option_field_varies(agents, |agent| {
        agent
            .utility_profile
            .as_ref()
            .map(|profile| &profile.loyalty_weight)
    }) || option_field_varies(agents, |agent| {
        agent
            .utility_profile
            .as_ref()
            .map(|profile| &profile.greed_weight)
    }) || option_field_varies(agents, |agent| {
        agent
            .utility_profile
            .as_ref()
            .map(|profile| &profile.shame_weight)
    }) || option_field_varies(agents, |agent| {
        agent
            .utility_profile
            .as_ref()
            .map(|profile| &profile.revenge_weight)
    })
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

fn check_office_treasuries(scenario: &ScenarioDef, report: &mut LintReport) {
    let place_names: BTreeSet<&str> = scenario
        .places
        .iter()
        .map(|place| place.name.as_str())
        .collect();

    for office in scenario
        .offices
        .iter()
        .filter(|office| office.treasury.is_some())
    {
        if !place_names.contains(office.seat.as_str()) {
            report.failures.push(LintFailure {
                rule: LintRule::TreasuryAuthoredWithMissingSeat,
                affected_agents: Vec::new(),
                detail: format!(
                    "office '{}' authors a treasury but seat '{}' does not resolve to a place",
                    office.name, office.seat
                ),
            });
        }

        let treasury = office
            .treasury
            .as_ref()
            .expect("filtered to treasury-bearing offices");
        if treasury.quantity == Quantity(0) {
            report.failures.push(LintFailure {
                rule: LintRule::TreasuryAuthoredWithZeroQuantity,
                affected_agents: Vec::new(),
                detail: format!(
                    "office '{}' authors a treasury with zero quantity for {:?}",
                    office.name, treasury.commodity
                ),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LintRule, filter_overrides, run_lints};
    use crate::scenario::types::{
        AgentDef, ExplorationProfileDef, OfficeDef, PlaceDef, ScenarioDef, TreasuryDef,
    };
    use crate::scenario::{ScenarioError, spawn_scenario};
    use std::collections::{BTreeMap, BTreeSet};
    use worldwake_core::{
        AgentSchemaContextProfile, CognitiveProfile, CommodityKind, ControlSource,
        DiversificationProfile, EpistemicDispositionProfile, IntentionDispositionProfile,
        MethodSchemaId, PerceptionProfile, Permille, PlaceTag, Quantity, SuccessionLaw,
        UtilityProfile,
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
            agent_schema_context_profile: None,
            risk_weight_profile: None,
            law_abiding_profile: None,
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
            testimony_trust_profile: None,
            route_preference_profile: None,
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
                sleep_quality: None,
                place_dirtiness: None,
                latrine_fullness: None,
            }],
            edges: vec![],
            agents,
            bandit_camps: Vec::new(),
            offices: vec![],
            artifacts: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
        }
    }

    fn scenario_with_office(office: OfficeDef) -> ScenarioDef {
        ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Town".into(),
                tags: vec![PlaceTag::Village],
                visibility_profile: None,
                sleep_quality: None,
                place_dirtiness: None,
                latrine_fullness: None,
            }],
            edges: vec![],
            agents: vec![],
            bandit_camps: Vec::new(),
            offices: vec![office],
            artifacts: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
        }
    }

    fn office_with_treasury(seat: &str, quantity: Quantity) -> OfficeDef {
        OfficeDef {
            name: "Market Warden".into(),
            seat: seat.into(),
            succession_law: SuccessionLaw::Force,
            succession_period_ticks: 2,
            initial_holder: None,
            eligibility_rules: Vec::new(),
            treasury: Some(TreasuryDef {
                commodity: CommodityKind::Coin,
                quantity,
                container_name: None,
            }),
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
            negative_survey_damping_window: 200,
            negative_survey_damping_strength: Permille::new_unchecked(800),
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
    fn profile_homogeneity_passes_when_each_motive_class_weight_varies() {
        for mutate in [
            |profile: &mut UtilityProfile| {
                profile.office_duty_weight = Permille::new_unchecked(625);
            },
            |profile: &mut UtilityProfile| {
                profile.loyalty_weight = Permille::new_unchecked(625);
            },
            |profile: &mut UtilityProfile| {
                profile.greed_weight = Permille::new_unchecked(625);
            },
            |profile: &mut UtilityProfile| {
                profile.shame_weight = Permille::new_unchecked(625);
            },
            |profile: &mut UtilityProfile| {
                profile.revenge_weight = Permille::new_unchecked(625);
            },
        ] {
            let mut varied = fully_profiled_ai("Cara");
            let mut utility = UtilityProfile::default();
            mutate(&mut utility);
            varied.utility_profile = Some(utility);
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
    }

    #[test]
    fn homogeneous_schema_context_methods_are_reported() {
        let schema_context = AgentSchemaContextProfile {
            disabled_methods: BTreeSet::from([MethodSchemaId(3)]),
            ..AgentSchemaContextProfile::default()
        };
        let agents = ["Alice", "Bob", "Cara"].map(|name| AgentDef {
            agent_schema_context_profile: Some(schema_context.clone()),
            ..fully_profiled_ai(name)
        });
        let scenario = scenario_with_agents(agents.into());

        let report = run_lints(&scenario);
        let failure = report
            .failures
            .iter()
            .find(|failure| failure.rule == LintRule::ProfileHomogeneity)
            .expect("homogeneous method denylist should be reported");

        assert!(
            failure
                .detail
                .contains("agent_schema_context_profile.disabled_methods")
        );
    }

    #[test]
    fn schema_context_method_variation_satisfies_profile_homogeneity() {
        let varied_schema_context = AgentSchemaContextProfile {
            disabled_methods: BTreeSet::from([MethodSchemaId(3)]),
            ..AgentSchemaContextProfile::default()
        };
        let mut varied = fully_profiled_ai("Cara");
        varied.agent_schema_context_profile = Some(varied_schema_context);
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
    fn lint_rejects_treasury_with_zero_quantity() {
        let scenario = scenario_with_office(office_with_treasury("Town", Quantity(0)));

        let report = run_lints(&scenario);

        assert!(report.failures.iter().any(|failure| {
            failure.rule == LintRule::TreasuryAuthoredWithZeroQuantity
                && failure.detail.contains("Market Warden")
        }));
    }

    #[test]
    fn lint_rejects_treasury_when_office_seat_missing() {
        let scenario = scenario_with_office(office_with_treasury("Missing", Quantity(5)));

        let report = run_lints(&scenario);

        assert!(report.failures.iter().any(|failure| {
            failure.rule == LintRule::TreasuryAuthoredWithMissingSeat
                && failure.detail.contains("Missing")
        }));
    }

    #[test]
    fn treasury_lint_override_suppresses_failure() {
        let mut scenario = scenario_with_office(office_with_treasury("Town", Quantity(0)));
        scenario.scenario_lint_overrides.insert(
            LintRule::TreasuryAuthoredWithZeroQuantity,
            "negative test keeps invalid treasury quantity".into(),
        );

        let report =
            filter_overrides(run_lints(&scenario), &scenario.scenario_lint_overrides).unwrap();

        assert!(
            !report
                .failures
                .iter()
                .any(|failure| failure.rule == LintRule::TreasuryAuthoredWithZeroQuantity)
        );
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
