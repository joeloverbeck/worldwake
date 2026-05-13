use crate::{PlannedPlan, PlannedStep};
use serde::{Deserialize, Serialize};
use worldwake_core::{
    BeliefRef, BreachSignature, CausalLink, CognitiveProfile, DiscrepancyClearing,
    DiscrepancyEntry, RepairKind, RepairMemory,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanRepairContext<'a> {
    pub failed_step: u16,
    pub broken_link: CausalLink,
    pub breach_signature: BreachSignature,
    pub preserved_prefix: &'a [PlannedStep],
    pub reusable_suffix: &'a [PlannedStep],
    pub new_evidence: &'a [BeliefRef],
    pub discrepancy_entry: &'a DiscrepancyEntry,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RepairOutcome {
    Repaired {
        kind: RepairKind,
        new_plan: Box<PlannedPlan>,
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

        let failure = if recently_failed(repair_memory, context.breach_signature, kind) {
            RepairFailure::RecentlyFailed
        } else {
            expansions = expansions.saturating_add(1);
            attempt_kind(context, kind)
        };
        tried.push((kind, failure));
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

fn attempt_kind(context: &PlanRepairContext<'_>, kind: RepairKind) -> RepairFailure {
    match kind {
        RepairKind::RebindTarget => RepairFailure::NoSiblingTargetFound,
        RepairKind::ReplaceProvider => RepairFailure::NoProviderReplacement,
        RepairKind::InsertVerification => RepairFailure::NoEpistemicSubstrate,
        RepairKind::DowngradeToProgressBarrier | RepairKind::Abandon => {
            if discrepancy_clearing_is_repair_search_visible(context.discrepancy_entry) {
                RepairFailure::NoProviderReplacement
            } else {
                RepairFailure::BudgetExhausted
            }
        }
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
        PlanRepairContext, RepairFailure, RepairOutcome, attempt_order, attempt_repair_then_replan,
        discrepancy_clearing_is_repair_search_visible, repair_budget,
    };
    use crate::{PlannedPlan, PlannedStep, PlannerOpKind};
    use worldwake_core::{
        ActionDefId, BreachSignature, CausalLink, CausalProvider, Discrepancy, DiscrepancyClearing,
        DiscrepancyEntry, EntityId, InvalidatorTag, Permille, PlanningFact, RepairEntry,
        RepairKind, RepairMemory, Tick,
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

    fn context<'a>(
        prefix: &'a [PlannedStep],
        suffix: &'a [PlannedStep],
        entry: &'a DiscrepancyEntry,
    ) -> PlanRepairContext<'a> {
        PlanRepairContext {
            failed_step: 1,
            broken_link: broken_link(),
            breach_signature: signature(),
            preserved_prefix: prefix,
            reusable_suffix: suffix,
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
        let cognitive = cognitive(20, Permille::new(1000).unwrap());
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
        let cognitive = cognitive(20, Permille::new(1000).unwrap());

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
        };

        assert_eq!(
            outcome,
            RepairOutcome::Repaired {
                kind: RepairKind::RebindTarget,
                new_plan: Box::new(plan),
            }
        );
    }

    #[test]
    fn repair_failure_roundtrips_through_bincode() {
        let bytes = bincode::serialize(&RepairFailure::NoEpistemicSubstrate).unwrap();
        let roundtrip: RepairFailure = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, RepairFailure::NoEpistemicSubstrate);
    }
}
