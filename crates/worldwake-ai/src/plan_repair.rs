use crate::{PlanTerminalKind, PlannedPlan, PlannedStep};
use serde::{Deserialize, Serialize};
use worldwake_core::{
    BeliefClaimKey, BeliefRef, BreachSignature, CausalLink, CausalProvider, CognitiveProfile,
    DiscrepancyClearing, DiscrepancyEntry, EntityBeliefAspect, OpportunityKey, PlanningFact,
    RecordTopic, RepairKind, RepairMemory,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanRepairContext<'a> {
    pub opportunity: OpportunityKey,
    pub failed_step: u16,
    pub broken_link: CausalLink,
    pub breach_signature: BreachSignature,
    pub preserved_prefix: &'a [PlannedStep],
    pub reusable_suffix: &'a [PlannedStep],
    pub replacement_candidates: &'a [RepairPlanCandidate],
    pub new_evidence: &'a [BeliefRef],
    pub discrepancy_entry: &'a DiscrepancyEntry,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepairPlanCandidate {
    pub kind: RepairKind,
    pub provider: CausalProvider,
    pub fact: PlanningFact,
    pub step: PlannedStep,
    pub reusable_suffix_index: Option<u16>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RepairOutcome {
    Repaired {
        kind: RepairKind,
        new_plan: Box<PlannedPlan>,
        rejected: Vec<(RepairKind, RepairFailure)>,
    },
    Failed {
        tried: Vec<(RepairKind, RepairFailure)>,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum RepairFailure {
    NoSiblingTargetFound,
    NoProviderReplacement,
    NoEpistemicSubstrate,
    BudgetExhausted,
    RecentlyFailed,
    NoPreservedPrefix,
}

#[must_use]
pub fn repair_budget(cognitive: &CognitiveProfile) -> u16 {
    let scaled = (u32::from(cognitive.max_node_expansions)
        * u32::from(cognitive.repair_budget_fraction.value()))
        / 1000;
    u16::try_from(scaled).unwrap_or(u16::MAX)
}

#[must_use]
pub fn attempt_order() -> [RepairKind; 5] {
    [
        RepairKind::RebindTarget,
        RepairKind::ReplaceProvider,
        RepairKind::InsertVerification,
        RepairKind::DowngradeToProgressBarrier,
        RepairKind::Abandon,
    ]
}

#[must_use]
pub fn attempt_repair_then_replan(
    context: &PlanRepairContext<'_>,
    cognitive: &CognitiveProfile,
    repair_memory: &RepairMemory,
) -> RepairOutcome {
    let budget = repair_budget(cognitive);
    let mut expansions = 0u16;
    let mut tried = Vec::new();

    for kind in attempt_order() {
        if expansions >= budget {
            tried.push((kind, RepairFailure::BudgetExhausted));
            break;
        }

        if recently_failed(repair_memory, context.breach_signature, kind) {
            tried.push((kind, RepairFailure::RecentlyFailed));
        } else {
            expansions = expansions.saturating_add(1);
            match attempt_kind(context, kind) {
                Ok(new_plan) => {
                    return RepairOutcome::Repaired {
                        kind,
                        new_plan: Box::new(new_plan),
                        rejected: tried,
                    };
                }
                Err(failure) => tried.push((kind, failure)),
            }
        }
    }

    RepairOutcome::Failed { tried }
}

fn recently_failed(
    repair_memory: &RepairMemory,
    signature: BreachSignature,
    kind: RepairKind,
) -> bool {
    repair_memory
        .repairs
        .get(&signature)
        .is_some_and(|entry| entry.kind == kind && !entry.succeeded)
}

fn attempt_kind(
    context: &PlanRepairContext<'_>,
    kind: RepairKind,
) -> Result<PlannedPlan, RepairFailure> {
    match kind {
        RepairKind::RebindTarget => {
            attempt_candidate_repair(context, kind).ok_or(RepairFailure::NoSiblingTargetFound)
        }
        RepairKind::ReplaceProvider => {
            attempt_candidate_repair(context, kind).ok_or(RepairFailure::NoProviderReplacement)
        }
        RepairKind::InsertVerification => Err(RepairFailure::NoEpistemicSubstrate),
        RepairKind::DowngradeToProgressBarrier => downgrade_to_progress_barrier(context),
        RepairKind::Abandon => Ok(PlannedPlan::new(
            context.opportunity,
            context.breach_signature.goal_key,
            Vec::new(),
            PlanTerminalKind::ProgressBarrier,
        )),
    }
}

fn attempt_candidate_repair(
    context: &PlanRepairContext<'_>,
    kind: RepairKind,
) -> Option<PlannedPlan> {
    let candidate = context.replacement_candidates.iter().find(|candidate| {
        candidate.kind == kind
            && candidate.fact == context.broken_link.fact
            && provider_supports_fact(candidate.provider, candidate.fact)
    })?;
    Some(plan_from_parts(
        context,
        Some(candidate.step.clone()),
        candidate.reusable_suffix_index,
        PlanTerminalKind::GoalSatisfied,
        true,
    ))
}

fn downgrade_to_progress_barrier(
    context: &PlanRepairContext<'_>,
) -> Result<PlannedPlan, RepairFailure> {
    if !discrepancy_clearing_is_repair_search_visible(context.discrepancy_entry) {
        return Err(RepairFailure::BudgetExhausted);
    }
    if context.preserved_prefix.is_empty() {
        return Err(RepairFailure::NoPreservedPrefix);
    }
    Ok(plan_from_parts(
        context,
        None,
        None,
        PlanTerminalKind::ProgressBarrier,
        false,
    ))
}

fn plan_from_parts(
    context: &PlanRepairContext<'_>,
    replacement_step: Option<PlannedStep>,
    replaced_reusable_suffix_index: Option<u16>,
    terminal_kind: PlanTerminalKind,
    include_reusable_suffix: bool,
) -> PlannedPlan {
    let reusable_suffix_len = if include_reusable_suffix {
        context.reusable_suffix.len()
    } else {
        0
    };
    let mut steps = Vec::with_capacity(
        context.preserved_prefix.len()
            + usize::from(replacement_step.is_some())
            + reusable_suffix_len,
    );
    steps.extend_from_slice(context.preserved_prefix);
    if let Some(step) = replacement_step {
        steps.push(step);
    }
    if include_reusable_suffix {
        steps.extend(
            context
                .reusable_suffix
                .iter()
                .enumerate()
                .filter(|(index, _)| replaced_reusable_suffix_index != u16::try_from(*index).ok())
                .map(|(_, step)| step.clone()),
        );
    }
    PlannedPlan::new(
        context.opportunity,
        context.breach_signature.goal_key,
        steps,
        terminal_kind,
    )
}

fn provider_supports_fact(provider: CausalProvider, fact: PlanningFact) -> bool {
    match (provider, fact) {
        (
            CausalProvider::Observation {
                observed_entity,
                aspect: EntityBeliefAspect::Location,
            },
            PlanningFact::TargetPresent { target, .. },
        ) => observed_entity == target,
        (
            CausalProvider::Belief {
                claim_key:
                    BeliefClaimKey {
                        subject,
                        aspect: EntityBeliefAspect::Location,
                    },
            },
            PlanningFact::TargetPresent { target, .. },
        ) => subject == target,
        (
            CausalProvider::Observation {
                observed_entity,
                aspect: EntityBeliefAspect::Inventory(observed_kind),
            },
            PlanningFact::CommodityAvailable { place, kind, .. },
        ) => observed_entity == place && observed_kind == kind,
        (
            CausalProvider::Belief {
                claim_key:
                    BeliefClaimKey {
                        subject,
                        aspect: EntityBeliefAspect::Location,
                    },
            },
            PlanningFact::RouteKnown { to, .. },
        ) => subject == to,
        (
            CausalProvider::Record {
                topic: RecordTopic::RouteSafety,
                ..
            },
            PlanningFact::RouteKnown { .. },
        ) => true,
        (
            CausalProvider::Observation {
                observed_entity,
                aspect: EntityBeliefAspect::Location,
            },
            PlanningFact::ResourceAccess { resource, .. },
        ) => observed_entity == resource,
        (
            CausalProvider::Belief {
                claim_key:
                    BeliefClaimKey {
                        subject,
                        aspect: EntityBeliefAspect::Location,
                    },
            },
            PlanningFact::ResourceAccess { resource, .. },
        ) => subject == resource,
        _ => false,
    }
}

#[must_use]
pub fn discrepancy_clearing_is_repair_search_visible(entry: &DiscrepancyEntry) -> bool {
    match entry.clearing_condition {
        DiscrepancyClearing::TtlExpiry
        | DiscrepancyClearing::ReobservationOf { .. }
        | DiscrepancyClearing::BeliefUpdate { .. }
        | DiscrepancyClearing::CommodityAvailabilityChanged { .. }
        | DiscrepancyClearing::WorldStructureChange => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PlanRepairContext, RepairFailure, RepairOutcome, RepairPlanCandidate, attempt_order,
        attempt_repair_then_replan, discrepancy_clearing_is_repair_search_visible, repair_budget,
    };
    use crate::{PlanTerminalKind, PlannedPlan, PlannedStep, PlannerOpKind};
    use worldwake_core::{
        ActionDefId, BreachSignature, CausalLink, CausalProvider, Discrepancy, DiscrepancyClearing,
        DiscrepancyEntry, EntityId, InvalidatorTag, OpportunityAnchor, OpportunityKey, Permille,
        PlanningFact, RepairEntry, RepairKind, RepairMemory, Tick,
        test_utils::{sample_blocker_key, sample_goal_key},
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn signature() -> BreachSignature {
        BreachSignature {
            goal_key: sample_goal_key(),
            invalidator: InvalidatorTag::TargetMoved,
            step_target: Some(entity(7)),
        }
    }

    fn broken_link() -> CausalLink {
        CausalLink {
            provider: CausalProvider::PriorStep { step_index: 0 },
            fact: PlanningFact::TargetPresent {
                target: entity(7),
                at_place: entity(8),
            },
            consumer_step_index: 1,
            source_tick: Tick(3),
            confidence: Permille::new(800).unwrap(),
        }
    }

    fn discrepancy_entry(clearing_condition: DiscrepancyClearing) -> DiscrepancyEntry {
        DiscrepancyEntry {
            blocker_key: sample_blocker_key(),
            discrepancy: Discrepancy::BeliefStale,
            observed_tick: Tick(5),
            expires_tick: Tick(25),
            clearing_condition,
        }
    }

    fn step() -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(1),
            targets: Vec::new(),
            target_place: None,
            payload_override: None,
            op_kind: PlannerOpKind::Travel,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        }
    }

    fn opportunity() -> OpportunityKey {
        OpportunityKey {
            goal_key: sample_goal_key(),
            anchor: OpportunityAnchor::None,
        }
    }

    fn context<'a>(
        prefix: &'a [PlannedStep],
        suffix: &'a [PlannedStep],
        entry: &'a DiscrepancyEntry,
    ) -> PlanRepairContext<'a> {
        PlanRepairContext {
            opportunity: opportunity(),
            failed_step: 1,
            broken_link: broken_link(),
            breach_signature: signature(),
            preserved_prefix: prefix,
            reusable_suffix: suffix,
            replacement_candidates: &[],
            new_evidence: &[],
            discrepancy_entry: entry,
        }
    }

    fn cognitive(
        max_node_expansions: u16,
        repair_budget_fraction: Permille,
    ) -> worldwake_core::CognitiveProfile {
        worldwake_core::CognitiveProfile {
            max_node_expansions,
            repair_budget_fraction,
            ..worldwake_core::CognitiveProfile::default()
        }
    }

    #[test]
    fn repair_search_terminates_within_budget() {
        let prefix = vec![step()];
        let suffix = vec![step()];
        let entry = discrepancy_entry(DiscrepancyClearing::TtlExpiry);
        let context = context(&prefix, &suffix, &entry);
        let cognitive = cognitive(8, Permille::new(250).unwrap());

        let outcome = attempt_repair_then_replan(&context, &cognitive, &RepairMemory::default());

        let RepairOutcome::Failed { tried } = outcome else {
            panic!("staged repair search should fail until ticket 007 wires replacement");
        };
        assert_eq!(repair_budget(&cognitive), 2);
        assert_eq!(tried.len(), 3);
        assert_eq!(
            tried[2],
            (
                RepairKind::InsertVerification,
                RepairFailure::BudgetExhausted
            )
        );
    }

    #[test]
    fn repair_kind_attempt_order_is_deterministic() {
        assert_eq!(
            attempt_order(),
            [
                RepairKind::RebindTarget,
                RepairKind::ReplaceProvider,
                RepairKind::InsertVerification,
                RepairKind::DowngradeToProgressBarrier,
                RepairKind::Abandon,
            ]
        );
    }

    #[test]
    fn repair_memory_skips_recently_failed_kinds() {
        let prefix = vec![step()];
        let suffix = vec![step()];
        let entry = discrepancy_entry(DiscrepancyClearing::TtlExpiry);
        let context = context(&prefix, &suffix, &entry);
        let cognitive = cognitive(1, Permille::new(1000).unwrap());
        let mut memory = RepairMemory::default();
        memory.record(RepairEntry {
            signature: signature(),
            kind: RepairKind::RebindTarget,
            succeeded: false,
            observed_tick: Tick(10),
            expires_tick: Tick(30),
            success_count: 0,
        });

        let outcome = attempt_repair_then_replan(&context, &cognitive, &memory);

        let RepairOutcome::Failed { tried } = outcome else {
            panic!("staged repair search should fail until ticket 007 wires replacement");
        };
        assert_eq!(
            tried[0],
            (RepairKind::RebindTarget, RepairFailure::RecentlyFailed)
        );
    }

    #[test]
    fn insert_verification_returns_no_epistemic_substrate_without_s139() {
        let prefix = vec![step()];
        let suffix = vec![step()];
        let entry = discrepancy_entry(DiscrepancyClearing::TtlExpiry);
        let context = context(&prefix, &suffix, &entry);
        let cognitive = cognitive(3, Permille::new(1000).unwrap());

        let outcome = attempt_repair_then_replan(&context, &cognitive, &RepairMemory::default());

        let RepairOutcome::Failed { tried } = outcome else {
            panic!("staged repair search should fail until ticket 007 wires replacement");
        };
        assert_eq!(
            tried[2],
            (
                RepairKind::InsertVerification,
                RepairFailure::NoEpistemicSubstrate
            )
        );
    }

    #[test]
    fn discrepancy_clearing_dispatch_covers_all_variants() {
        let target = entity(2);
        let claim_key = worldwake_core::BeliefClaimKey {
            subject: target,
            aspect: worldwake_core::EntityBeliefAspect::Location,
        };
        let variants = [
            DiscrepancyClearing::TtlExpiry,
            DiscrepancyClearing::ReobservationOf { target },
            DiscrepancyClearing::BeliefUpdate { claim_key },
            DiscrepancyClearing::CommodityAvailabilityChanged {
                commodity: worldwake_core::CommodityKind::Bread,
                place: entity(3),
            },
            DiscrepancyClearing::WorldStructureChange,
        ];

        for clearing in variants {
            let entry = discrepancy_entry(clearing);
            assert!(discrepancy_clearing_is_repair_search_visible(&entry));
        }
    }

    #[test]
    fn repair_outcome_can_carry_repaired_plan_shape() {
        let plan = PlannedPlan::new(
            worldwake_core::OpportunityKey {
                goal_key: sample_goal_key(),
                anchor: worldwake_core::OpportunityAnchor::None,
            },
            sample_goal_key(),
            vec![step()],
            crate::PlanTerminalKind::GoalSatisfied,
        );

        let outcome = RepairOutcome::Repaired {
            kind: RepairKind::RebindTarget,
            new_plan: Box::new(plan.clone()),
            rejected: Vec::new(),
        };

        assert_eq!(
            outcome,
            RepairOutcome::Repaired {
                kind: RepairKind::RebindTarget,
                new_plan: Box::new(plan),
                rejected: Vec::new(),
            }
        );
    }

    #[test]
    fn repair_failure_roundtrips_through_bincode() {
        let bytes = bincode::serialize(&RepairFailure::NoEpistemicSubstrate).unwrap();
        let roundtrip: RepairFailure = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, RepairFailure::NoEpistemicSubstrate);
    }

    #[test]
    fn rebind_target_returns_repaired_plan_with_prefix_candidate_and_suffix() {
        let prefix = vec![step()];
        let suffix = vec![step()];
        let entry = discrepancy_entry(DiscrepancyClearing::TtlExpiry);
        let replacement = PlannedStep {
            targets: vec![crate::PlanningEntityRef::Authoritative(entity(17))],
            target_place: Some(entity(8)),
            ..step()
        };
        let candidates = vec![RepairPlanCandidate {
            kind: RepairKind::RebindTarget,
            provider: CausalProvider::Observation {
                observed_entity: entity(7),
                aspect: worldwake_core::EntityBeliefAspect::Location,
            },
            fact: broken_link().fact,
            step: replacement.clone(),
            reusable_suffix_index: None,
        }];
        let mut context = context(&prefix, &suffix, &entry);
        context.replacement_candidates = &candidates;
        let cognitive = cognitive(20, Permille::new(1000).unwrap());

        let outcome = attempt_repair_then_replan(&context, &cognitive, &RepairMemory::default());

        let RepairOutcome::Repaired { kind, new_plan, .. } = outcome else {
            panic!("rebind candidate should repair plan");
        };
        assert_eq!(kind, RepairKind::RebindTarget);
        assert_eq!(new_plan.terminal_kind, PlanTerminalKind::GoalSatisfied);
        assert_eq!(
            new_plan.steps,
            vec![prefix[0].clone(), replacement, suffix[0].clone()]
        );
    }

    #[test]
    fn suffix_sourced_candidate_is_promoted_without_duplication() {
        let prefix = vec![step()];
        let promoted = PlannedStep {
            targets: vec![crate::PlanningEntityRef::Authoritative(entity(17))],
            target_place: Some(entity(8)),
            ..step()
        };
        let trailing = PlannedStep {
            targets: vec![crate::PlanningEntityRef::Authoritative(entity(18))],
            target_place: Some(entity(8)),
            ..step()
        };
        let suffix = vec![promoted.clone(), trailing.clone()];
        let entry = discrepancy_entry(DiscrepancyClearing::TtlExpiry);
        let candidates = vec![RepairPlanCandidate {
            kind: RepairKind::RebindTarget,
            provider: CausalProvider::Observation {
                observed_entity: entity(7),
                aspect: worldwake_core::EntityBeliefAspect::Location,
            },
            fact: broken_link().fact,
            step: promoted.clone(),
            reusable_suffix_index: Some(0),
        }];
        let mut context = context(&prefix, &suffix, &entry);
        context.replacement_candidates = &candidates;
        let cognitive = cognitive(20, Permille::new(1000).unwrap());

        let outcome = attempt_repair_then_replan(&context, &cognitive, &RepairMemory::default());

        let RepairOutcome::Repaired { kind, new_plan, .. } = outcome else {
            panic!("suffix-sourced candidate should repair plan");
        };
        assert_eq!(kind, RepairKind::RebindTarget);
        assert_eq!(new_plan.steps, vec![prefix[0].clone(), promoted, trailing]);
    }

    #[test]
    fn replace_provider_rejects_provider_that_does_not_support_fact() {
        let prefix = vec![step()];
        let suffix = vec![step()];
        let entry = discrepancy_entry(DiscrepancyClearing::TtlExpiry);
        let candidates = vec![RepairPlanCandidate {
            kind: RepairKind::ReplaceProvider,
            provider: CausalProvider::Observation {
                observed_entity: entity(99),
                aspect: worldwake_core::EntityBeliefAspect::Location,
            },
            fact: broken_link().fact,
            step: step(),
            reusable_suffix_index: None,
        }];
        let mut context = context(&prefix, &suffix, &entry);
        context.replacement_candidates = &candidates;
        let cognitive = cognitive(3, Permille::new(1000).unwrap());

        let outcome = attempt_repair_then_replan(&context, &cognitive, &RepairMemory::default());

        let RepairOutcome::Failed { tried } = outcome else {
            panic!("unsupported provider should not repair plan");
        };
        assert_eq!(
            tried[1],
            (
                RepairKind::ReplaceProvider,
                RepairFailure::NoProviderReplacement
            )
        );
        assert_eq!(
            tried[2],
            (
                RepairKind::InsertVerification,
                RepairFailure::NoEpistemicSubstrate
            )
        );
    }

    #[test]
    fn replace_provider_returns_repaired_plan_for_lawful_route_provider() {
        let prefix = vec![step()];
        let suffix = vec![step()];
        let entry = discrepancy_entry(DiscrepancyClearing::TtlExpiry);
        let route_fact = PlanningFact::RouteKnown {
            from: entity(1),
            to: entity(2),
        };
        let candidates = vec![RepairPlanCandidate {
            kind: RepairKind::ReplaceProvider,
            provider: CausalProvider::Record {
                record_entity: entity(30),
                topic: worldwake_core::RecordTopic::RouteSafety,
            },
            fact: route_fact,
            step: step(),
            reusable_suffix_index: None,
        }];
        let mut context = context(&prefix, &suffix, &entry);
        context.broken_link.fact = route_fact;
        context.replacement_candidates = &candidates;
        let cognitive = cognitive(20, Permille::new(1000).unwrap());

        let outcome = attempt_repair_then_replan(&context, &cognitive, &RepairMemory::default());

        let RepairOutcome::Repaired {
            kind,
            new_plan,
            rejected,
        } = outcome
        else {
            panic!("route record provider should repair plan");
        };
        assert_eq!(kind, RepairKind::ReplaceProvider);
        assert_eq!(
            rejected,
            vec![(
                RepairKind::RebindTarget,
                RepairFailure::NoSiblingTargetFound
            )]
        );
        assert_eq!(new_plan.steps.len(), 3);
        assert_eq!(new_plan.terminal_kind, PlanTerminalKind::GoalSatisfied);
    }

    #[test]
    fn downgrade_to_progress_barrier_preserves_committed_prefix_only() {
        let prefix = vec![step()];
        let suffix = vec![step()];
        let entry = discrepancy_entry(DiscrepancyClearing::TtlExpiry);
        let context = context(&prefix, &suffix, &entry);
        let cognitive = cognitive(4, Permille::new(1000).unwrap());

        let outcome = attempt_repair_then_replan(&context, &cognitive, &RepairMemory::default());

        let RepairOutcome::Repaired { kind, new_plan, .. } = outcome else {
            panic!("visible discrepancy should downgrade to a progress barrier");
        };
        assert_eq!(kind, RepairKind::DowngradeToProgressBarrier);
        assert_eq!(new_plan.terminal_kind, PlanTerminalKind::ProgressBarrier);
        assert_eq!(new_plan.steps, prefix);
    }

    #[test]
    fn abandon_returns_empty_progress_barrier_after_prior_strategies_fail() {
        let prefix = Vec::new();
        let suffix = vec![step()];
        let entry = discrepancy_entry(DiscrepancyClearing::TtlExpiry);
        let context = context(&prefix, &suffix, &entry);
        let cognitive = cognitive(5, Permille::new(1000).unwrap());

        let outcome = attempt_repair_then_replan(&context, &cognitive, &RepairMemory::default());

        let RepairOutcome::Repaired { kind, new_plan, .. } = outcome else {
            panic!("abandon should be the final local outcome after earlier attempts fail");
        };
        assert_eq!(kind, RepairKind::Abandon);
        assert_eq!(new_plan.terminal_kind, PlanTerminalKind::ProgressBarrier);
        assert!(new_plan.steps.is_empty());
    }
}
