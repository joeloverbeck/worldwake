//! Goal-family decision policy — single authoritative surface for suppression,
//! penalty-interrupt eligibility, and free-interrupt role per `GoalKind`.

use crate::goal_model::GoalPriorityClass;
use crate::interrupts::InterruptTrigger;
use worldwake_core::{CommodityPurpose, GoalKind};

// ---------------------------------------------------------------------------
// DecisionContext
// ---------------------------------------------------------------------------

/// Shared pressure state used to evaluate goal policies.
/// Contains only the two priority-class summaries that ranking and interrupts
/// both need; no interrupt-specific parameters live here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecisionContext {
    pub max_self_care_class: GoalPriorityClass,
    pub danger_class: GoalPriorityClass,
}

impl DecisionContext {
    /// Returns `true` when *either* `max_self_care_class` or `danger_class`
    /// is at or above the given threshold.
    pub fn is_stressed_at_or_above(&self, threshold: GoalPriorityClass) -> bool {
        self.max_self_care_class >= threshold || self.danger_class >= threshold
    }
}

// ---------------------------------------------------------------------------
// Policy enums
// ---------------------------------------------------------------------------

/// Whether a goal family can be suppressed under stress.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SuppressionRule {
    /// Goal is never suppressed regardless of stress.
    Never,
    /// Goal is suppressed when either self-care or danger class is at or above
    /// the given threshold.
    WhenStressedAtOrAbove(GoalPriorityClass),
}

/// Whether a running goal is eligible for a penalty interrupt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PenaltyInterruptEligibility {
    /// Eligible for penalty interrupt when the given trigger fires.
    WhenCritical { trigger: InterruptTrigger },
    /// Never eligible for penalty interrupts.
    Never,
}

/// How a goal behaves with respect to free (non-penalty) interrupts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FreeInterruptRole {
    /// Reactive goals (self-care, danger reduction, healing) — can freely
    /// interrupt lower-priority work.
    Reactive,
    /// Opportunistic goals (e.g. looting) — can interrupt when the opportunity
    /// is available.
    Opportunistic,
    /// Normal goals — standard interrupt rules apply.
    Normal,
}

// ---------------------------------------------------------------------------
// GoalFamilyPolicy
// ---------------------------------------------------------------------------

/// Complete decision policy for a single goal family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GoalFamilyPolicy {
    pub suppression: SuppressionRule,
    pub penalty_interrupt: PenaltyInterruptEligibility,
    pub free_interrupt: FreeInterruptRole,
}

// ---------------------------------------------------------------------------
// GoalPolicyOutcome
// ---------------------------------------------------------------------------

/// Result of evaluating a goal's suppression rule against a `DecisionContext`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GoalPolicyOutcome {
    Available,
    Suppressed {
        threshold: GoalPriorityClass,
        max_self_care: GoalPriorityClass,
        danger: GoalPriorityClass,
    },
}

// ---------------------------------------------------------------------------
// Policy lookup
// ---------------------------------------------------------------------------

/// Returns the authoritative decision policy for a goal family.
///
/// The match is exhaustive over all `GoalKind` variants. Adding a new variant
/// without updating this function will produce a compile error.
///
/// `AcquireCommodity` is discriminated by `CommodityPurpose`: `SelfConsume`
/// is treated as self-care; all other purposes are enterprise goals.
pub fn goal_family_policy(kind: &GoalKind) -> GoalFamilyPolicy {
    match kind {
        // --- Self-care goals (survival needs) ---
        GoalKind::ConsumeOwnedCommodity { .. }
        | GoalKind::Sleep
        | GoalKind::Relieve
        | GoalKind::Wash => GoalFamilyPolicy {
            suppression: SuppressionRule::Never,
            penalty_interrupt: PenaltyInterruptEligibility::WhenCritical {
                trigger: InterruptTrigger::CriticalSurvival,
            },
            free_interrupt: FreeInterruptRole::Reactive,
        },

        // AcquireCommodity: self-care when SelfConsume, enterprise otherwise
        GoalKind::AcquireCommodity { purpose, .. } => match purpose {
            CommodityPurpose::SelfConsume => GoalFamilyPolicy {
                suppression: SuppressionRule::Never,
                penalty_interrupt: PenaltyInterruptEligibility::WhenCritical {
                    trigger: InterruptTrigger::CriticalSurvival,
                },
                free_interrupt: FreeInterruptRole::Reactive,
            },
            _ => GoalFamilyPolicy {
                suppression: SuppressionRule::Never,
                penalty_interrupt: PenaltyInterruptEligibility::Never,
                free_interrupt: FreeInterruptRole::Normal,
            },
        },

        // --- Danger goals ---
        GoalKind::ReduceDanger => GoalFamilyPolicy {
            suppression: SuppressionRule::Never,
            penalty_interrupt: PenaltyInterruptEligibility::WhenCritical {
                trigger: InterruptTrigger::CriticalDanger,
            },
            free_interrupt: FreeInterruptRole::Reactive,
        },

        // --- Care ---
        GoalKind::TreatWounds { .. } => GoalFamilyPolicy {
            suppression: SuppressionRule::Never,
            penalty_interrupt: PenaltyInterruptEligibility::Never,
            free_interrupt: FreeInterruptRole::Reactive,
        },

        // --- Combat + enterprise goals (no suppression, no penalty, normal interrupt) ---
        GoalKind::EngageHostile { .. }
        | GoalKind::ProduceCommodity { .. }
        | GoalKind::SellCommodity { .. }
        | GoalKind::RestockCommodity { .. }
        | GoalKind::MoveCargo { .. } => GoalFamilyPolicy {
            suppression: SuppressionRule::Never,
            penalty_interrupt: PenaltyInterruptEligibility::Never,
            free_interrupt: FreeInterruptRole::Normal,
        },

        // --- Corpse: loot is opportunistic ---
        GoalKind::LootCorpse { .. } => GoalFamilyPolicy {
            suppression: SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::High),
            penalty_interrupt: PenaltyInterruptEligibility::Never,
            free_interrupt: FreeInterruptRole::Opportunistic,
        },

        // --- Corpse / social / political goals (suppressed under stress, normal interrupt) ---
        GoalKind::BuryCorpse { .. }
        | GoalKind::ShareBelief { .. }
        | GoalKind::ClaimOffice { .. }
        | GoalKind::SupportCandidateForOffice { .. } => GoalFamilyPolicy {
            suppression: SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::High),
            penalty_interrupt: PenaltyInterruptEligibility::Never,
            free_interrupt: FreeInterruptRole::Normal,
        },
    }
}

// ---------------------------------------------------------------------------
// Suppression evaluation
// ---------------------------------------------------------------------------

/// Evaluates whether a goal is suppressed given the current decision context.
pub fn evaluate_suppression(kind: &GoalKind, context: &DecisionContext) -> GoalPolicyOutcome {
    let policy = goal_family_policy(kind);
    match policy.suppression {
        SuppressionRule::Never => GoalPolicyOutcome::Available,
        SuppressionRule::WhenStressedAtOrAbove(threshold) => {
            if context.is_stressed_at_or_above(threshold) {
                GoalPolicyOutcome::Suppressed {
                    threshold,
                    max_self_care: context.max_self_care_class,
                    danger: context.danger_class,
                }
            } else {
                GoalPolicyOutcome::Available
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use worldwake_core::{CommodityKind, CommodityPurpose, EntityId, GoalKind, RecipeId};

    // Helpers
    fn dummy_entity() -> EntityId {
        EntityId {
            slot: 0,
            generation: 0,
        }
    }

    fn dummy_recipe() -> RecipeId {
        RecipeId(0)
    }

    // -- Suppression rule tests --

    #[test]
    fn suppression_never_for_self_care_goals() {
        let self_care = [
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Apple,
            },
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
            },
            GoalKind::Sleep,
            GoalKind::Relieve,
            GoalKind::Wash,
        ];
        for kind in &self_care {
            assert_eq!(
                goal_family_policy(kind).suppression,
                SuppressionRule::Never,
                "Self-care goal {kind:?} should never be suppressed"
            );
        }
    }

    #[test]
    fn suppression_never_for_danger_combat_healing_enterprise() {
        let goals = [
            GoalKind::ReduceDanger,
            GoalKind::EngageHostile {
                target: dummy_entity(),
            },
            GoalKind::TreatWounds {
                patient: dummy_entity(),
            },
            GoalKind::ProduceCommodity {
                recipe_id: dummy_recipe(),
            },
            GoalKind::SellCommodity {
                commodity: CommodityKind::Apple,
            },
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Apple,
            },
            GoalKind::MoveCargo {
                commodity: CommodityKind::Apple,
                destination: dummy_entity(),
            },
        ];
        for kind in &goals {
            assert_eq!(
                goal_family_policy(kind).suppression,
                SuppressionRule::Never,
                "Goal {kind:?} should never be suppressed"
            );
        }
    }

    #[test]
    fn suppression_when_stressed_for_corpse_social_political() {
        let goals = [
            GoalKind::LootCorpse {
                corpse: dummy_entity(),
            },
            GoalKind::BuryCorpse {
                corpse: dummy_entity(),
                burial_site: dummy_entity(),
            },
            GoalKind::ShareBelief {
                listener: dummy_entity(),
                subject: dummy_entity(),
            },
            GoalKind::ClaimOffice {
                office: dummy_entity(),
            },
            GoalKind::SupportCandidateForOffice {
                office: dummy_entity(),
                candidate: dummy_entity(),
            },
        ];
        for kind in &goals {
            assert_eq!(
                goal_family_policy(kind).suppression,
                SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::High),
                "Goal {kind:?} should be suppressed when stressed at or above High"
            );
        }
    }

    // -- Penalty interrupt eligibility tests --

    #[test]
    fn penalty_critical_survival_for_self_care() {
        let self_care = [
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Apple,
            },
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
            },
            GoalKind::Sleep,
            GoalKind::Relieve,
            GoalKind::Wash,
        ];
        for kind in &self_care {
            assert_eq!(
                goal_family_policy(kind).penalty_interrupt,
                PenaltyInterruptEligibility::WhenCritical {
                    trigger: InterruptTrigger::CriticalSurvival
                },
                "Self-care goal {kind:?} should have CriticalSurvival penalty interrupt"
            );
        }
    }

    #[test]
    fn penalty_critical_danger_for_reduce_danger() {
        assert_eq!(
            goal_family_policy(&GoalKind::ReduceDanger).penalty_interrupt,
            PenaltyInterruptEligibility::WhenCritical {
                trigger: InterruptTrigger::CriticalDanger
            },
        );
    }

    #[test]
    fn penalty_never_for_heal_combat_enterprise_corpse_social_political() {
        let goals = [
            GoalKind::TreatWounds {
                patient: dummy_entity(),
            },
            GoalKind::EngageHostile {
                target: dummy_entity(),
            },
            GoalKind::ProduceCommodity {
                recipe_id: dummy_recipe(),
            },
            GoalKind::SellCommodity {
                commodity: CommodityKind::Apple,
            },
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Apple,
            },
            GoalKind::MoveCargo {
                commodity: CommodityKind::Apple,
                destination: dummy_entity(),
            },
            GoalKind::LootCorpse {
                corpse: dummy_entity(),
            },
            GoalKind::BuryCorpse {
                corpse: dummy_entity(),
                burial_site: dummy_entity(),
            },
            GoalKind::ShareBelief {
                listener: dummy_entity(),
                subject: dummy_entity(),
            },
            GoalKind::ClaimOffice {
                office: dummy_entity(),
            },
            GoalKind::SupportCandidateForOffice {
                office: dummy_entity(),
                candidate: dummy_entity(),
            },
        ];
        for kind in &goals {
            assert_eq!(
                goal_family_policy(kind).penalty_interrupt,
                PenaltyInterruptEligibility::Never,
                "Goal {kind:?} should have Never penalty interrupt"
            );
        }
    }

    // -- Free interrupt role tests --

    #[test]
    fn free_interrupt_reactive_for_self_care_danger_heal() {
        let reactive = [
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Apple,
            },
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
            },
            GoalKind::Sleep,
            GoalKind::Relieve,
            GoalKind::Wash,
            GoalKind::ReduceDanger,
            GoalKind::TreatWounds {
                patient: dummy_entity(),
            },
        ];
        for kind in &reactive {
            assert_eq!(
                goal_family_policy(kind).free_interrupt,
                FreeInterruptRole::Reactive,
                "Goal {kind:?} should have Reactive free interrupt role"
            );
        }
    }

    #[test]
    fn free_interrupt_opportunistic_for_loot_corpse() {
        assert_eq!(
            goal_family_policy(&GoalKind::LootCorpse {
                corpse: dummy_entity()
            })
            .free_interrupt,
            FreeInterruptRole::Opportunistic,
        );
    }

    #[test]
    fn free_interrupt_normal_for_combat_enterprise_corpse_social_political() {
        let normal = [
            GoalKind::EngageHostile {
                target: dummy_entity(),
            },
            GoalKind::ProduceCommodity {
                recipe_id: dummy_recipe(),
            },
            GoalKind::SellCommodity {
                commodity: CommodityKind::Apple,
            },
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Apple,
            },
            GoalKind::MoveCargo {
                commodity: CommodityKind::Apple,
                destination: dummy_entity(),
            },
            GoalKind::BuryCorpse {
                corpse: dummy_entity(),
                burial_site: dummy_entity(),
            },
            GoalKind::ShareBelief {
                listener: dummy_entity(),
                subject: dummy_entity(),
            },
            GoalKind::ClaimOffice {
                office: dummy_entity(),
            },
            GoalKind::SupportCandidateForOffice {
                office: dummy_entity(),
                candidate: dummy_entity(),
            },
        ];
        for kind in &normal {
            assert_eq!(
                goal_family_policy(kind).free_interrupt,
                FreeInterruptRole::Normal,
                "Goal {kind:?} should have Normal free interrupt role"
            );
        }
    }

    // -- AcquireCommodity enterprise variant (non-SelfConsume) --

    #[test]
    fn acquire_commodity_enterprise_has_normal_role_and_no_penalty() {
        let enterprise_acquire = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::Restock,
        };
        let policy = goal_family_policy(&enterprise_acquire);
        assert_eq!(policy.suppression, SuppressionRule::Never);
        assert_eq!(policy.penalty_interrupt, PenaltyInterruptEligibility::Never);
        assert_eq!(policy.free_interrupt, FreeInterruptRole::Normal);
    }

    // -- evaluate_suppression tests --

    #[test]
    fn suppression_returns_suppressed_when_self_care_high() {
        let ctx = DecisionContext {
            max_self_care_class: GoalPriorityClass::High,
            danger_class: GoalPriorityClass::Low,
        };
        let outcome = evaluate_suppression(
            &GoalKind::LootCorpse {
                corpse: dummy_entity(),
            },
            &ctx,
        );
        assert_eq!(
            outcome,
            GoalPolicyOutcome::Suppressed {
                threshold: GoalPriorityClass::High,
                max_self_care: GoalPriorityClass::High,
                danger: GoalPriorityClass::Low,
            }
        );
    }

    #[test]
    fn suppression_returns_suppressed_when_danger_high() {
        let ctx = DecisionContext {
            max_self_care_class: GoalPriorityClass::Low,
            danger_class: GoalPriorityClass::High,
        };
        let outcome = evaluate_suppression(
            &GoalKind::LootCorpse {
                corpse: dummy_entity(),
            },
            &ctx,
        );
        assert_eq!(
            outcome,
            GoalPolicyOutcome::Suppressed {
                threshold: GoalPriorityClass::High,
                max_self_care: GoalPriorityClass::Low,
                danger: GoalPriorityClass::High,
            }
        );
    }

    #[test]
    fn suppression_returns_available_when_below_threshold() {
        let ctx = DecisionContext {
            max_self_care_class: GoalPriorityClass::Medium,
            danger_class: GoalPriorityClass::Low,
        };
        assert_eq!(
            evaluate_suppression(
                &GoalKind::LootCorpse {
                    corpse: dummy_entity()
                },
                &ctx
            ),
            GoalPolicyOutcome::Available,
        );
    }

    #[test]
    fn suppression_returns_available_for_self_care_regardless_of_stress() {
        let ctx = DecisionContext {
            max_self_care_class: GoalPriorityClass::Critical,
            danger_class: GoalPriorityClass::Critical,
        };
        assert_eq!(
            evaluate_suppression(&GoalKind::Sleep, &ctx),
            GoalPolicyOutcome::Available,
        );
    }

    // -- DecisionContext tests --

    #[test]
    fn is_stressed_at_or_above_returns_true_when_danger_meets_threshold() {
        let ctx = DecisionContext {
            max_self_care_class: GoalPriorityClass::Low,
            danger_class: GoalPriorityClass::Medium,
        };
        assert!(ctx.is_stressed_at_or_above(GoalPriorityClass::Medium));
    }

    #[test]
    fn is_stressed_at_or_above_returns_false_when_both_below() {
        let ctx = DecisionContext {
            max_self_care_class: GoalPriorityClass::Low,
            danger_class: GoalPriorityClass::Low,
        };
        assert!(!ctx.is_stressed_at_or_above(GoalPriorityClass::Medium));
    }

    #[test]
    fn is_stressed_at_or_above_returns_true_when_self_care_meets_threshold() {
        let ctx = DecisionContext {
            max_self_care_class: GoalPriorityClass::High,
            danger_class: GoalPriorityClass::Low,
        };
        assert!(ctx.is_stressed_at_or_above(GoalPriorityClass::Medium));
    }
}
