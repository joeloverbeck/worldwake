use worldwake_ai::{
    FeasibilityStrategy, FreeInterruptRole, FrontierExhaustionStrategy, GoalDispatchKeySchemaExt,
    GoalFamilyPolicy, GoalSchema, InvalidationStrategy, PlannerOpKind,
    goal_policy::{PenaltyInterruptEligibility, SuppressionRule},
};
use worldwake_core::{GoalDispatchKey, GoalPlanningBudget, MethodSchemaId};

#[test]
fn iteration_order_preserved() {
    let schema = GoalSchema {
        trace_label: "Fixture",
        provenance_family: None,
        relevant_ops: &[PlannerOpKind::Travel],
        invalidation_strategy: InvalidationStrategy::NoOpinion,
        feasibility_strategy: FeasibilityStrategy::NoOpinion,
        frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
        family_policy: GoalFamilyPolicy {
            suppression: SuppressionRule::Never,
            penalty_interrupt: PenaltyInterruptEligibility::Never,
            free_interrupt: FreeInterruptRole::Normal,
        },
        progress_barrier_ops: &[],
        candidate_extractors: &[],
        planning_budget: GoalPlanningBudget::TRAVEL_PURCHASE,
        methods: &[MethodSchemaId(1), MethodSchemaId(2)],
    };

    assert_eq!(schema.methods, &[MethodSchemaId(1), MethodSchemaId(2)]);
}

#[test]
fn all_dispatch_declarations_expose_empty_method_anchors() {
    for key in GoalDispatchKey::all() {
        assert!(
            key.declaration().methods.is_empty(),
            "GoalDispatchKey::{key:?} should leave method assignment to the method registry"
        );
    }
}
