//! Goal-family decision policy — single authoritative surface for suppression,
//! penalty-interrupt eligibility, and free-interrupt role per `GoalKind`.

use crate::goal_model::GoalPriorityClass;
use crate::goal_schema::GoalDispatchKeySchemaExt;
use crate::interrupts::InterruptTrigger;
use worldwake_core::GoalKind;

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
// Suppression evaluation
// ---------------------------------------------------------------------------

/// Evaluates whether a goal is suppressed given the current decision context.
pub fn evaluate_suppression(kind: &GoalKind, context: &DecisionContext) -> GoalPolicyOutcome {
    let policy = crate::GoalDispatchKey::from_goal_kind(kind)
        .declaration()
        .family_policy;
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
    use worldwake_core::{
        AcquisitionQuantity, ArtifactPostingContext, CommodityKind, CommodityPurpose, EntityId,
        GoalKind, NoticeTopic, PunishmentKind, Quantity, RecipeId, ViolationId,
    };

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
                quantity: AcquisitionQuantity::single(),
            },
            GoalKind::Sleep,
            GoalKind::Relieve,
            GoalKind::Wash,
        ];
        for kind in &self_care {
            assert_eq!(
                crate::GoalDispatchKey::from_goal_kind(kind)
                    .declaration()
                    .family_policy
                    .suppression,
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
                crate::GoalDispatchKey::from_goal_kind(kind)
                    .declaration()
                    .family_policy
                    .suppression,
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
                topic: worldwake_core::TellTopic::EntityBelief {
                    subject: dummy_entity(),
                },
                communication_class: worldwake_core::CommunicationClass::Gossip,
            },
            GoalKind::ClaimOffice {
                office: dummy_entity(),
            },
            GoalKind::SupportCandidateForOffice {
                office: dummy_entity(),
                candidate: dummy_entity(),
            },
            GoalKind::InvestigateViolation {
                violation_id: ViolationId(1),
                place: dummy_entity(),
            },
            GoalKind::Patrol {
                place: dummy_entity(),
            },
            GoalKind::Accuse {
                crime_register: dummy_entity(),
                accused: dummy_entity(),
                violation_id: ViolationId(2),
            },
            GoalKind::PunishAccused {
                office: dummy_entity(),
                accused: dummy_entity(),
                accusation_entry: worldwake_core::RecordEntryId(1),
                punishment: PunishmentKind::Fine {
                    commodity: CommodityKind::Coin,
                    amount: Quantity(1),
                },
            },
        ];
        for kind in &goals {
            assert_eq!(
                crate::GoalDispatchKey::from_goal_kind(kind)
                    .declaration()
                    .family_policy
                    .suppression,
                SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::High),
                "Goal {kind:?} should be suppressed when stressed at or above High"
            );
        }
    }

    #[test]
    fn suppression_never_for_steal_item() {
        assert_eq!(
            crate::GoalDispatchKey::from_goal_kind(&GoalKind::StealItem {
                target_item: dummy_entity(),
            })
            .declaration()
            .family_policy
            .suppression,
            SuppressionRule::Never,
        );
    }

    #[test]
    fn share_belief_suppression_depends_on_communication_class() {
        let listener = dummy_entity();
        let topic = worldwake_core::TellTopic::EntityBelief {
            subject: dummy_entity(),
        };

        assert_eq!(
            crate::GoalDispatchKey::from_goal_kind(&GoalKind::ShareBelief {
                listener,
                topic,
                communication_class: worldwake_core::CommunicationClass::Alarm,
            })
            .declaration()
            .family_policy
            .suppression,
            SuppressionRule::Never,
        );
        assert_eq!(
            crate::GoalDispatchKey::from_goal_kind(&GoalKind::ShareBelief {
                listener,
                topic,
                communication_class: worldwake_core::CommunicationClass::Testimony,
            })
            .declaration()
            .family_policy
            .suppression,
            SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::Critical),
        );
        assert_eq!(
            crate::GoalDispatchKey::from_goal_kind(&GoalKind::ShareBelief {
                listener,
                topic,
                communication_class: worldwake_core::CommunicationClass::Gossip,
            })
            .declaration()
            .family_policy
            .suppression,
            SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::High),
        );
    }

    #[test]
    fn ask_witness_epistemic_sensing_suppresses_at_critical_stress() {
        let goal = GoalKind::AskWitness {
            witness: dummy_entity(),
            topic: worldwake_core::TellTopic::EntityBelief {
                subject: dummy_entity(),
            },
        };
        let policy = crate::GoalDispatchKey::from_goal_kind(&goal)
            .declaration()
            .family_policy;

        assert_eq!(
            policy.suppression,
            SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::Critical)
        );
        assert_eq!(policy.penalty_interrupt, PenaltyInterruptEligibility::Never);
        assert_eq!(policy.free_interrupt, FreeInterruptRole::Normal);

        let below_critical = DecisionContext {
            max_self_care_class: GoalPriorityClass::High,
            danger_class: GoalPriorityClass::Low,
        };
        assert_eq!(
            evaluate_suppression(&goal, &below_critical),
            GoalPolicyOutcome::Available
        );

        let critical = DecisionContext {
            max_self_care_class: GoalPriorityClass::Critical,
            danger_class: GoalPriorityClass::Low,
        };
        assert_eq!(
            evaluate_suppression(&goal, &critical),
            GoalPolicyOutcome::Suppressed {
                threshold: GoalPriorityClass::Critical,
                max_self_care: GoalPriorityClass::Critical,
                danger: GoalPriorityClass::Low,
            }
        );
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
                quantity: AcquisitionQuantity::single(),
            },
            GoalKind::Sleep,
            GoalKind::Relieve,
            GoalKind::Wash,
        ];
        for kind in &self_care {
            assert_eq!(
                crate::GoalDispatchKey::from_goal_kind(kind)
                    .declaration()
                    .family_policy
                    .penalty_interrupt,
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
            crate::GoalDispatchKey::from_goal_kind(&GoalKind::ReduceDanger)
                .declaration()
                .family_policy
                .penalty_interrupt,
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
                topic: worldwake_core::TellTopic::EntityBelief {
                    subject: dummy_entity(),
                },
                communication_class: worldwake_core::CommunicationClass::Gossip,
            },
            GoalKind::ClaimOffice {
                office: dummy_entity(),
            },
            GoalKind::SupportCandidateForOffice {
                office: dummy_entity(),
                candidate: dummy_entity(),
            },
            GoalKind::InvestigateViolation {
                violation_id: ViolationId(3),
                place: dummy_entity(),
            },
            GoalKind::StealItem {
                target_item: dummy_entity(),
            },
            GoalKind::Accuse {
                crime_register: dummy_entity(),
                accused: dummy_entity(),
                violation_id: ViolationId(4),
            },
            GoalKind::PunishAccused {
                office: dummy_entity(),
                accused: dummy_entity(),
                accusation_entry: worldwake_core::RecordEntryId(2),
                punishment: PunishmentKind::Exile {
                    from_faction: dummy_entity(),
                },
            },
        ];
        for kind in &goals {
            assert_eq!(
                crate::GoalDispatchKey::from_goal_kind(kind)
                    .declaration()
                    .family_policy
                    .penalty_interrupt,
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
                quantity: AcquisitionQuantity::single(),
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
                crate::GoalDispatchKey::from_goal_kind(kind)
                    .declaration()
                    .family_policy
                    .free_interrupt,
                FreeInterruptRole::Reactive,
                "Goal {kind:?} should have Reactive free interrupt role"
            );
        }
    }

    #[test]
    fn free_interrupt_opportunistic_for_loot_corpse() {
        assert_eq!(
            crate::GoalDispatchKey::from_goal_kind(&GoalKind::LootCorpse {
                corpse: dummy_entity()
            })
            .declaration()
            .family_policy
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
                topic: worldwake_core::TellTopic::EntityBelief {
                    subject: dummy_entity(),
                },
                communication_class: worldwake_core::CommunicationClass::Gossip,
            },
            GoalKind::ClaimOffice {
                office: dummy_entity(),
            },
            GoalKind::SupportCandidateForOffice {
                office: dummy_entity(),
                candidate: dummy_entity(),
            },
            GoalKind::InvestigateViolation {
                violation_id: ViolationId(5),
                place: dummy_entity(),
            },
            GoalKind::StealItem {
                target_item: dummy_entity(),
            },
            GoalKind::Accuse {
                crime_register: dummy_entity(),
                accused: dummy_entity(),
                violation_id: ViolationId(6),
            },
            GoalKind::PunishAccused {
                office: dummy_entity(),
                accused: dummy_entity(),
                accusation_entry: worldwake_core::RecordEntryId(3),
                punishment: PunishmentKind::Exile {
                    from_faction: dummy_entity(),
                },
            },
        ];
        for kind in &normal {
            assert_eq!(
                crate::GoalDispatchKey::from_goal_kind(kind)
                    .declaration()
                    .family_policy
                    .free_interrupt,
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
            quantity: AcquisitionQuantity::single(),
        };
        let policy = crate::GoalDispatchKey::from_goal_kind(&enterprise_acquire)
            .declaration()
            .family_policy;
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

    #[test]
    fn raid_target_remains_available_even_under_critical_stress() {
        let ctx = DecisionContext {
            max_self_care_class: GoalPriorityClass::Critical,
            danger_class: GoalPriorityClass::Critical,
        };

        assert_eq!(
            evaluate_suppression(
                &GoalKind::RaidTarget {
                    target: dummy_entity(),
                },
                &ctx
            ),
            GoalPolicyOutcome::Available,
        );
        assert_eq!(
            crate::GoalDispatchKey::from_goal_kind(&GoalKind::RaidTarget {
                target: dummy_entity(),
            })
            .declaration()
            .family_policy
            .free_interrupt,
            FreeInterruptRole::Normal,
        );
    }

    #[test]
    fn regroup_with_faction_is_suppressed_at_high_stress_and_uses_normal_role() {
        let ctx = DecisionContext {
            max_self_care_class: GoalPriorityClass::High,
            danger_class: GoalPriorityClass::Low,
        };

        assert_eq!(
            evaluate_suppression(
                &GoalKind::RegroupWithFaction {
                    faction: dummy_entity(),
                },
                &ctx
            ),
            GoalPolicyOutcome::Suppressed {
                threshold: GoalPriorityClass::High,
                max_self_care: GoalPriorityClass::High,
                danger: GoalPriorityClass::Low,
            },
        );
        assert_eq!(
            crate::GoalDispatchKey::from_goal_kind(&GoalKind::RegroupWithFaction {
                faction: dummy_entity(),
            })
            .declaration()
            .family_policy
            .free_interrupt,
            FreeInterruptRole::Normal,
        );
    }

    #[test]
    fn threat_warning_notice_remains_available_under_high_danger() {
        let ctx = DecisionContext {
            max_self_care_class: GoalPriorityClass::Low,
            danger_class: GoalPriorityClass::High,
        };

        assert_eq!(
            evaluate_suppression(
                &GoalKind::PostNotice {
                    posting: ArtifactPostingContext {
                        posting_place: dummy_entity(),
                        issuing_authority: None,
                        expires_at: None,
                        jurisdiction: Some(dummy_entity()),
                    },
                    topic: NoticeTopic::ThreatWarning {
                        place: dummy_entity(),
                    },
                },
                &ctx
            ),
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
