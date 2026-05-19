use std::collections::BTreeSet;

use worldwake_ai::htn::{MethodSchema, PayloadTemplate, SubgoalTemplate, build_method_registry};
use worldwake_ai::planner_ops::PlannerOpKind;
use worldwake_core::{GoalKindDiscriminant, MethodSchemaId};

#[test]
fn every_method_goal_kind_resolves() {
    let registry = build_method_registry();
    let goal_kinds: BTreeSet<_> = GoalKindDiscriminant::ALL.iter().copied().collect();

    for method in registry.all_methods() {
        assert!(
            goal_kinds.contains(&method.goal_kind),
            "method {:?} has unknown goal kind {:?}",
            method.id,
            method.goal_kind
        );
    }
}

#[test]
fn every_subgoal_action_op_resolves() {
    let registry = build_method_registry();

    for method in registry.all_methods() {
        for subgoal in &method.subgoals {
            if let SubgoalTemplate::PerformAction(op, _) = subgoal {
                assert!(
                    planner_op_is_known(*op),
                    "method {:?} references unknown planner op {:?}",
                    method.id,
                    op
                );
            }
        }
    }
}

#[test]
fn method_schema_ids_are_unique() {
    let registry = build_method_registry();
    let ids: BTreeSet<MethodSchemaId> = registry.all_method_ids().collect();

    assert_eq!(ids.len(), registry.len());
}

#[test]
fn every_method_declares_at_least_one_failure_mode() {
    let registry = build_method_registry();

    for method in registry.all_methods() {
        assert!(
            !method.failure_modes.is_empty(),
            "method {:?} must declare at least one failure mode",
            method.id
        );
    }
}

#[test]
fn motive_bias_weights_are_in_permille_bounds() {
    let registry = build_method_registry();

    for method in registry.all_methods() {
        for bias in &method.motive_bias {
            assert!(
                bias.weight.value() <= 1000,
                "method {:?} has out-of-range motive bias {:?}",
                method.id,
                bias
            );
        }
    }
}

#[test]
fn every_method_has_at_least_one_action_or_non_action_subgoal() {
    let registry = build_method_registry();

    for method in registry.all_methods() {
        assert!(
            !method.subgoals.is_empty(),
            "method {:?} must declare at least one subgoal",
            method.id
        );
        assert_payload_templates_are_constructed(method);
    }
}

fn assert_payload_templates_are_constructed(method: &MethodSchema) {
    for subgoal in &method.subgoals {
        if let SubgoalTemplate::PerformAction(_, payload) = subgoal {
            match payload {
                PayloadTemplate::FromContext | PayloadTemplate::Explicit(_) => {}
            }
        }
    }
}

fn planner_op_is_known(op: PlannerOpKind) -> bool {
    match op {
        PlannerOpKind::Travel
        | PlannerOpKind::Patrol
        | PlannerOpKind::Consume
        | PlannerOpKind::Sleep
        | PlannerOpKind::Relieve
        | PlannerOpKind::Wash
        | PlannerOpKind::EstablishCamp
        | PlannerOpKind::Trade
        | PlannerOpKind::QueueForFacilityUse
        | PlannerOpKind::Harvest
        | PlannerOpKind::Craft
        | PlannerOpKind::MoveCargo
        | PlannerOpKind::DropItem
        | PlannerOpKind::Heal
        | PlannerOpKind::Loot
        | PlannerOpKind::Bury
        | PlannerOpKind::ClaimBounty
        | PlannerOpKind::WithdrawBounty
        | PlannerOpKind::PostBounty
        | PlannerOpKind::PostNotice
        | PlannerOpKind::Tell
        | PlannerOpKind::ConsultRecord
        | PlannerOpKind::Attack
        | PlannerOpKind::Defend
        | PlannerOpKind::Bribe
        | PlannerOpKind::Threaten
        | PlannerOpKind::Accuse
        | PlannerOpKind::Fine
        | PlannerOpKind::Exile
        | PlannerOpKind::DeclareSupport
        | PlannerOpKind::PressForceClaim
        | PlannerOpKind::YieldForceClaim
        | PlannerOpKind::Investigate
        | PlannerOpKind::AskWitness
        | PlannerOpKind::SearchPlace
        | PlannerOpKind::AskAboutPerson
        | PlannerOpKind::ReportMissing
        | PlannerOpKind::EscortToSafety
        | PlannerOpKind::ReportFound
        | PlannerOpKind::StaffMarket
        | PlannerOpKind::StockManagement => true,
    }
}
