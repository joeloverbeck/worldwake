#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

use crate::{GoalPriorityClass, goal_model::AgendaEntry, ranking::OrderedRanked};
pub use worldwake_core::SlotKind;
use worldwake_core::{
    Discrepancy, EntityId, MotiveSourceDiscriminant, OperatingMode, OpportunityKey, Permille,
    PortfolioWeightsProfile,
};
use worldwake_sim::GoalBeliefView;

#[derive(Clone, Debug)]
pub(crate) struct Portfolio {
    pub(crate) slots: BTreeMap<SlotKind, PortfolioSlot>,
}

#[derive(Clone, Debug)]
pub(crate) struct PortfolioSlot {
    pub(crate) ranked: AgendaEntry,
    pub(crate) feasibility: FeasibilityVerdict,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FeasibilityVerdict {
    Plausible,
    RejectedBeforeSearch { reason: Discrepancy },
}

pub(crate) fn assemble_portfolio(
    ranked: &OrderedRanked<'_>,
    committed: Option<OpportunityKey>,
    weights: &PortfolioWeightsProfile,
    mode: OperatingMode,
    probe: impl Fn(&AgendaEntry) -> FeasibilityVerdict,
) -> Portfolio {
    let effective_weights = apply_mode(weights, mode);
    let mut slots = BTreeMap::new();
    let mut selected = BTreeSet::new();

    for slot in [
        SlotKind::NeedSurvival,
        SlotKind::PainCare,
        SlotKind::ObligationDuty,
        SlotKind::EconomicOpportunity,
        SlotKind::SocialMotive,
    ] {
        if effective_weights.weight_for(slot) == Permille::ZERO {
            continue;
        }
        if let Some(winner) =
            select_best_candidate_for_slot(ranked, &selected, committed, |entry| {
                primary_motive_slot(entry) == slot
            })
        {
            selected.insert(opportunity_key(winner));
            slots.insert(
                slot,
                PortfolioSlot {
                    ranked: winner.clone(),
                    feasibility: probe(winner),
                },
            );
        }
    }

    Portfolio { slots }
}

fn primary_motive_slot(entry: &AgendaEntry) -> SlotKind {
    entry
        .motive_source_contributions
        .iter()
        .max_by(|(left_ref, left_weight), (right_ref, right_weight)| {
            left_weight
                .cmp(right_weight)
                .then_with(|| right_ref.introduced_tick.cmp(&left_ref.introduced_tick))
        })
        .map_or(SlotKind::EconomicOpportunity, |(motive_ref, _)| {
            worldwake_core::motive_source_slot_for(MotiveSourceDiscriminant::from(
                &motive_ref.source,
            ))
        })
}

fn apply_mode(weights: &PortfolioWeightsProfile, mode: OperatingMode) -> PortfolioWeightsProfile {
    match mode {
        OperatingMode::Emergency => PortfolioWeightsProfile {
            economic_opportunity: Permille::ZERO,
            social_motive: Permille::ZERO,
            ..*weights
        },
        OperatingMode::Normal | OperatingMode::Idle => *weights,
    }
}

pub(crate) fn derive_operating_mode<V: GoalBeliefView + ?Sized>(
    _belief: &V,
    _agent: EntityId,
    ranked: &OrderedRanked<'_>,
) -> OperatingMode {
    derive_operating_mode_from_ranked(ranked)
}

fn derive_operating_mode_from_ranked(ranked: &OrderedRanked<'_>) -> OperatingMode {
    let mut highest_priority = GoalPriorityClass::Background;
    let mut has_critical_pain_or_need = false;

    for entry in ranked {
        highest_priority = highest_priority.max(entry.priority_class);
        if entry.priority_class == GoalPriorityClass::Critical
            && entry.motive_source_contributions.iter().any(|(source, _)| {
                matches!(
                    MotiveSourceDiscriminant::from(&source.source),
                    MotiveSourceDiscriminant::Pain | MotiveSourceDiscriminant::NeedPressure
                )
            })
        {
            has_critical_pain_or_need = true;
        }
    }

    if has_critical_pain_or_need {
        OperatingMode::Emergency
    } else if highest_priority <= GoalPriorityClass::Background {
        OperatingMode::Idle
    } else {
        OperatingMode::Normal
    }
}

impl Portfolio {
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub(crate) fn plausible_slots_by_score<'a>(
        &'a self,
        weights: &PortfolioWeightsProfile,
    ) -> Vec<(SlotKind, &'a PortfolioSlot)> {
        let mut slots: Vec<_> = self
            .slots
            .iter()
            .filter_map(|(kind, slot)| {
                matches!(slot.feasibility, FeasibilityVerdict::Plausible).then_some((*kind, slot))
            })
            .collect();
        slots.sort_by(|(left_kind, left_slot), (right_kind, right_slot)| {
            compare_plausible_slots(*left_kind, left_slot, *right_kind, right_slot, *weights)
        });
        slots
    }
}

fn select_best_candidate_for_slot<'a>(
    ranked: &'a OrderedRanked<'_>,
    selected: &BTreeSet<OpportunityKey>,
    committed: Option<OpportunityKey>,
    predicate: impl Fn(&AgendaEntry) -> bool,
) -> Option<&'a AgendaEntry> {
    if let Some(committed) = committed {
        let committed_candidate = ranked
            .iter()
            .find(|candidate| opportunity_key(candidate) == committed);
        if let Some(candidate) = committed_candidate
            && !selected.contains(&committed)
            && predicate(candidate)
        {
            return Some(candidate);
        }
    }

    // `ranked` is pre-sorted by `ranking::compare_ranked_goals`, so the first
    // matching entry is the ranking contract's preferred candidate for this
    // slot.
    ranked
        .iter()
        .find(|candidate| predicate(candidate) && !selected.contains(&opportunity_key(candidate)))
}

fn compare_plausible_slots(
    left_kind: SlotKind,
    left_slot: &PortfolioSlot,
    right_kind: SlotKind,
    right_slot: &PortfolioSlot,
    weights: PortfolioWeightsProfile,
) -> std::cmp::Ordering {
    right_slot
        .ranked
        .priority_class
        .cmp(&left_slot.ranked.priority_class)
        .then_with(|| {
            weighted_score(right_kind, right_slot, &weights)
                .cmp(&weighted_score(left_kind, left_slot, &weights))
        })
        .then_with(|| left_kind.cmp(&right_kind))
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn weighted_score(kind: SlotKind, slot: &PortfolioSlot, weights: &PortfolioWeightsProfile) -> u32 {
    slot.ranked
        .motive_score
        .saturating_mul(u32::from(weights.weight_for(kind).value()))
        / 1000
}

fn opportunity_key(ranked: &AgendaEntry) -> OpportunityKey {
    OpportunityKey {
        goal_key: ranked.offer.key,
        anchor: ranked.offer.anchor,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FeasibilityVerdict, Portfolio, PortfolioSlot, SlotKind, apply_mode, assemble_portfolio,
        derive_operating_mode_from_ranked, opportunity_key, primary_motive_slot,
    };
    use crate::{
        GoalPriorityClass,
        feasibility::FeasibilityHint,
        goal_model::{AgendaEntry, GoalOffer},
        ranking,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use worldwake_core::{
        AcquisitionQuantity, ArtifactPostingContext, CommodityKind, CommodityPurpose, Discrepancy,
        EntityId, GoalKey, GoalKind, HomeostaticNeedId, MotiveSource, MotiveSourceRef, NoticeTopic,
        OperatingMode, OpportunityAnchor, OpportunityKey, Permille, PortfolioWeightsProfile, Tick,
        ViolationId, WoundId,
    };

    #[test]
    fn slot_kind_round_trips_through_serde() {
        for slot in [
            SlotKind::NeedSurvival,
            SlotKind::PainCare,
            SlotKind::ObligationDuty,
            SlotKind::EconomicOpportunity,
            SlotKind::SocialMotive,
        ] {
            let bytes = bincode::serialize(&slot).expect("SlotKind should serialize");
            let decoded: SlotKind =
                bincode::deserialize(&bytes).expect("SlotKind should deserialize");
            assert_eq!(decoded, slot);
        }
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn ranked_goal(kind: GoalKind, motive_score: u32, anchor: OpportunityAnchor) -> AgendaEntry {
        AgendaEntry {
            key: worldwake_core::OpportunityKey {
                goal_key: GoalKey::from(kind),
                anchor,
            },
            offer: GoalOffer {
                key: GoalKey::from(kind),
                anchor,
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                motive_sources: Vec::new(),
                acquisition_quantity: None,
            },
            priority_class: GoalPriorityClass::High,
            motive_score,
            motive_source_contributions: Vec::new(),
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            source_composite: None,
            feasibility: FeasibilityHint::Uncertain,
            phase: crate::AgendaPhase::Pending,
            origin: crate::AgendaOrigin::NeedDrive,
            introduced_tick: Tick(0),
            last_reconsidered_tick: Tick(0),
            revival_trigger: None,
            kill_condition: crate::KillCondition::External,
        }
    }

    fn ranked_goal_with_priority(
        kind: GoalKind,
        priority_class: GoalPriorityClass,
        motive_score: u32,
        anchor: OpportunityAnchor,
    ) -> AgendaEntry {
        let mut ranked = ranked_goal(kind, motive_score, anchor);
        ranked.priority_class = priority_class;
        ranked
    }

    fn ranked_goal_with_priority_and_motive(
        kind: GoalKind,
        priority_class: GoalPriorityClass,
        source: MotiveSource,
    ) -> AgendaEntry {
        let mut ranked =
            ranked_goal_with_priority(kind, priority_class, 500, OpportunityAnchor::None);
        ranked.motive_source_contributions = vec![(
            MotiveSourceRef {
                source,
                introduced_tick: Tick(3),
            },
            500,
        )];
        ranked
    }

    fn with_motive(mut ranked: AgendaEntry, source: MotiveSource, weight: u32) -> AgendaEntry {
        ranked.motive_source_contributions = vec![(
            MotiveSourceRef {
                source,
                introduced_tick: Tick(3),
            },
            weight,
        )];
        ranked
    }

    fn with_motive_at_tick(
        mut ranked: AgendaEntry,
        source: MotiveSource,
        weight: u32,
        introduced_tick: Tick,
    ) -> AgendaEntry {
        ranked.motive_source_contributions.push((
            MotiveSourceRef {
                source,
                introduced_tick,
            },
            weight,
        ));
        ranked
    }

    fn post_notice_goal(posting_place: EntityId) -> GoalKind {
        GoalKind::PostNotice {
            posting: ArtifactPostingContext {
                posting_place,
                issuing_authority: None,
                expires_at: Some(Tick(10)),
                jurisdiction: Some(posting_place),
            },
            topic: NoticeTopic::ThreatWarning {
                place: posting_place,
            },
        }
    }

    #[test]
    fn survival_slot_picks_highest_motive_survival() {
        // Input mirrors the real call path: `agent_tick::mod` sorts candidates
        // by `ranking::compare_ranked_goals` (highest-preference first) before
        // handing the list to `assemble_portfolio`. Pass candidates in that
        // pre-sorted order here.
        let mut ranked = vec![
            with_motive(
                ranked_goal(GoalKind::Sleep, 600, OpportunityAnchor::None),
                MotiveSource::NeedPressure {
                    need: HomeostaticNeedId::Hunger,
                },
                600,
            ),
            with_motive(
                ranked_goal(GoalKind::Relieve, 500, OpportunityAnchor::None),
                MotiveSource::NeedPressure {
                    need: HomeostaticNeedId::Fatigue,
                },
                500,
            ),
            with_motive(
                ranked_goal(GoalKind::Wash, 400, OpportunityAnchor::None),
                MotiveSource::NeedPressure {
                    need: HomeostaticNeedId::Dirtiness,
                },
                400,
            ),
        ];
        let ranked = ranking::sort_in_place(&mut ranked);

        let portfolio = assemble_portfolio(
            &ranked,
            None,
            &PortfolioWeightsProfile::default(),
            OperatingMode::Normal,
            |_| FeasibilityVerdict::Plausible,
        );

        let survival = portfolio
            .slots
            .get(&SlotKind::NeedSurvival)
            .expect("survival slot should be populated");
        assert_eq!(survival.ranked.offer.key, GoalKey::from(GoalKind::Sleep));
    }

    #[test]
    fn commitment_slot_picks_committed_opportunity_when_ranked() {
        let committed_place = entity(10);
        let committed_goal = with_motive(
            ranked_goal(
                post_notice_goal(committed_place),
                400,
                OpportunityAnchor::Place(committed_place),
            ),
            MotiveSource::OfficeDuty {
                office: committed_place,
            },
            400,
        );
        let higher_motive = with_motive(
            ranked_goal(
                GoalKind::ReportMissing {
                    subject: entity(2),
                    to_office: Some(entity(3)),
                    expectation_id: None,
                },
                700,
                OpportunityAnchor::Place(entity(3)),
            ),
            MotiveSource::OfficeDuty { office: entity(3) },
            700,
        );
        let committed = Some(opportunity_key(&committed_goal));
        let mut ranked = vec![higher_motive, committed_goal.clone()];
        let ranked = ranking::sort_in_place(&mut ranked);

        let portfolio = assemble_portfolio(
            &ranked,
            committed,
            &PortfolioWeightsProfile::default(),
            OperatingMode::Normal,
            |_| FeasibilityVerdict::Plausible,
        );

        let slot = portfolio
            .slots
            .get(&SlotKind::ObligationDuty)
            .expect("commitment slot should be populated");
        assert_eq!(
            opportunity_key(&slot.ranked),
            opportunity_key(&committed_goal)
        );
    }

    #[test]
    fn commitment_slot_falls_back_to_highest_obligation_when_commitment_unranked() {
        let mut ranked = vec![
            with_motive(
                ranked_goal(
                    GoalKind::ReportFound {
                        subject: entity(7),
                        expectation_id: worldwake_core::ExpectationId(2),
                    },
                    600,
                    OpportunityAnchor::Entity(entity(7)),
                ),
                MotiveSource::Loyalty { other: entity(7) },
                600,
            ),
            with_motive(
                ranked_goal(
                    post_notice_goal(entity(11)),
                    450,
                    OpportunityAnchor::Place(entity(11)),
                ),
                MotiveSource::OfficeDuty { office: entity(11) },
                450,
            ),
        ];
        let ranked = ranking::sort_in_place(&mut ranked);
        let missing_commitment = Some(OpportunityKey {
            goal_key: GoalKey::from(GoalKind::Sleep),
            anchor: OpportunityAnchor::None,
        });

        let portfolio = assemble_portfolio(
            &ranked,
            missing_commitment,
            &PortfolioWeightsProfile::default(),
            OperatingMode::Normal,
            |_| FeasibilityVerdict::Plausible,
        );

        let slot = portfolio
            .slots
            .get(&SlotKind::ObligationDuty)
            .expect("commitment slot should fall back to an obligation");
        assert_eq!(
            slot.ranked.offer.key,
            GoalKey::from(GoalKind::ReportFound {
                subject: entity(7),
                expectation_id: worldwake_core::ExpectationId(2),
            })
        );
    }

    #[test]
    fn self_consume_acquire_populates_survival_slot() {
        let mut ranked = vec![
            with_motive(
                ranked_goal(
                    GoalKind::AcquireCommodity {
                        commodity: CommodityKind::Bread,
                        purpose: CommodityPurpose::SelfConsume,
                        quantity: AcquisitionQuantity::single(),
                    },
                    550,
                    OpportunityAnchor::Place(entity(21)),
                ),
                MotiveSource::NeedPressure {
                    need: HomeostaticNeedId::Hunger,
                },
                550,
            ),
            with_motive(
                ranked_goal(
                    GoalKind::AcquireCommodity {
                        commodity: CommodityKind::Firewood,
                        purpose: CommodityPurpose::Restock,
                        quantity: AcquisitionQuantity::single(),
                    },
                    600,
                    OpportunityAnchor::Place(entity(22)),
                ),
                MotiveSource::Greed {
                    opportunity: OpportunityKey {
                        goal_key: GoalKey::from(GoalKind::AcquireCommodity {
                            commodity: CommodityKind::Firewood,
                            purpose: CommodityPurpose::Restock,
                            quantity: AcquisitionQuantity::single(),
                        }),
                        anchor: OpportunityAnchor::Place(entity(22)),
                    },
                },
                600,
            ),
        ];
        let ranked = ranking::sort_in_place(&mut ranked);

        let portfolio = assemble_portfolio(
            &ranked,
            None,
            &PortfolioWeightsProfile::default(),
            OperatingMode::Normal,
            |_| FeasibilityVerdict::Plausible,
        );

        assert_eq!(
            portfolio
                .slots
                .get(&SlotKind::NeedSurvival)
                .expect("self-consume acquisition should count as survival")
                .ranked
                .offer
                .key,
            GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            })
        );
        assert_eq!(
            portfolio
                .slots
                .get(&SlotKind::EconomicOpportunity)
                .expect("restock acquisition should still populate economic")
                .ranked
                .offer
                .key,
            GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Firewood,
                purpose: CommodityPurpose::Restock,
                quantity: AcquisitionQuantity::single(),
            })
        );
    }

    #[test]
    fn plausible_slots_by_score_applies_weights() {
        let survival = ranked_goal(GoalKind::Sleep, 500, OpportunityAnchor::None);
        let economic = ranked_goal(
            GoalKind::SellCommodity {
                commodity: CommodityKind::Bread,
            },
            600,
            OpportunityAnchor::Place(entity(30)),
        );
        let rejected = ranked_goal(
            post_notice_goal(entity(31)),
            900,
            OpportunityAnchor::Place(entity(31)),
        );
        let portfolio = Portfolio {
            slots: BTreeMap::from([
                (
                    SlotKind::NeedSurvival,
                    PortfolioSlot {
                        ranked: survival,
                        feasibility: FeasibilityVerdict::Plausible,
                    },
                ),
                (
                    SlotKind::EconomicOpportunity,
                    PortfolioSlot {
                        ranked: economic,
                        feasibility: FeasibilityVerdict::Plausible,
                    },
                ),
                (
                    SlotKind::ObligationDuty,
                    PortfolioSlot {
                        ranked: rejected,
                        feasibility: FeasibilityVerdict::RejectedBeforeSearch {
                            reason: Discrepancy::MissingObservation,
                        },
                    },
                ),
            ]),
        };

        let ordered = portfolio.plausible_slots_by_score(&PortfolioWeightsProfile::default());

        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].0, SlotKind::NeedSurvival);
        assert_eq!(ordered[1].0, SlotKind::EconomicOpportunity);
    }

    #[test]
    fn survival_slot_prefers_higher_priority_class_over_higher_motive() {
        // `ranking::compare_ranked_goals` sorts higher priority class first,
        // so Critical Relieve leads High-priority apple even though apple has
        // the higher raw motive. Tests that feed `assemble_portfolio` must
        // preserve that sort order because the portfolio trusts the list.
        let mut ranked = vec![
            with_motive(
                ranked_goal_with_priority(
                    GoalKind::Relieve,
                    GoalPriorityClass::Critical,
                    500,
                    OpportunityAnchor::None,
                ),
                MotiveSource::NeedPressure {
                    need: HomeostaticNeedId::Thirst,
                },
                500,
            ),
            with_motive(
                ranked_goal_with_priority(
                    GoalKind::AcquireCommodity {
                        commodity: CommodityKind::Apple,
                        purpose: CommodityPurpose::SelfConsume,
                        quantity: AcquisitionQuantity::single(),
                    },
                    GoalPriorityClass::High,
                    900,
                    OpportunityAnchor::Place(entity(40)),
                ),
                MotiveSource::NeedPressure {
                    need: HomeostaticNeedId::Hunger,
                },
                900,
            ),
        ];
        let ranked = ranking::sort_in_place(&mut ranked);

        let portfolio = assemble_portfolio(
            &ranked,
            None,
            &PortfolioWeightsProfile::default(),
            OperatingMode::Normal,
            |_| FeasibilityVerdict::Plausible,
        );
        let survival = portfolio
            .slots
            .get(&SlotKind::NeedSurvival)
            .expect("survival slot should be populated");
        assert_eq!(survival.ranked.offer.key.kind, GoalKind::Relieve);
    }

    #[test]
    fn plausible_slots_by_score_prefers_higher_priority_class_over_weighted_score() {
        let portfolio = Portfolio {
            slots: BTreeMap::from([
                (
                    SlotKind::NeedSurvival,
                    PortfolioSlot {
                        ranked: ranked_goal_with_priority(
                            GoalKind::Relieve,
                            GoalPriorityClass::Critical,
                            500,
                            OpportunityAnchor::None,
                        ),
                        feasibility: FeasibilityVerdict::Plausible,
                    },
                ),
                (
                    SlotKind::ObligationDuty,
                    PortfolioSlot {
                        ranked: ranked_goal_with_priority(
                            post_notice_goal(entity(41)),
                            GoalPriorityClass::High,
                            900,
                            OpportunityAnchor::Place(entity(41)),
                        ),
                        feasibility: FeasibilityVerdict::Plausible,
                    },
                ),
            ]),
        };

        let ordered = portfolio.plausible_slots_by_score(&PortfolioWeightsProfile::default());

        assert_eq!(ordered[0].0, SlotKind::NeedSurvival);
        assert_eq!(ordered[1].0, SlotKind::ObligationDuty);
    }

    #[test]
    fn derive_operating_mode_returns_emergency_for_critical_need_or_pain() {
        let ranked = vec![ranked_goal_with_priority_and_motive(
            GoalKind::Sleep,
            GoalPriorityClass::Critical,
            MotiveSource::NeedPressure {
                need: HomeostaticNeedId::Hunger,
            },
        )];
        let ranked = ranking::OrderedRanked::from_sorted_for_test(&ranked);

        assert_eq!(
            derive_operating_mode_from_ranked(&ranked),
            OperatingMode::Emergency
        );

        let ranked = vec![ranked_goal_with_priority_and_motive(
            GoalKind::TreatWounds { patient: entity(8) },
            GoalPriorityClass::Critical,
            MotiveSource::Pain { wound: WoundId(8) },
        )];
        let ranked = ranking::OrderedRanked::from_sorted_for_test(&ranked);

        assert_eq!(
            derive_operating_mode_from_ranked(&ranked),
            OperatingMode::Emergency
        );
    }

    #[test]
    fn derive_operating_mode_returns_idle_when_all_candidates_are_background() {
        let ranked = vec![ranked_goal_with_priority(
            GoalKind::SellCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalPriorityClass::Background,
            100,
            OpportunityAnchor::Place(entity(61)),
        )];
        let ranked = ranking::OrderedRanked::from_sorted_for_test(&ranked);

        assert_eq!(
            derive_operating_mode_from_ranked(&ranked),
            OperatingMode::Idle
        );
    }

    #[test]
    fn derive_operating_mode_returns_normal_for_non_emergency_pressure() {
        let ranked = vec![ranked_goal_with_priority_and_motive(
            GoalKind::SellCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalPriorityClass::High,
            MotiveSource::Greed {
                opportunity: OpportunityKey {
                    goal_key: GoalKey::from(GoalKind::SellCommodity {
                        commodity: CommodityKind::Bread,
                    }),
                    anchor: OpportunityAnchor::Place(entity(62)),
                },
            },
        )];
        let ranked = ranking::OrderedRanked::from_sorted_for_test(&ranked);

        assert_eq!(
            derive_operating_mode_from_ranked(&ranked),
            OperatingMode::Normal
        );
    }

    #[test]
    fn assemble_portfolio_populates_all_five_slots_under_normal() {
        let greed_opportunity = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::SellCommodity {
                commodity: CommodityKind::Bread,
            }),
            anchor: OpportunityAnchor::Place(entity(71)),
        };
        let mut ranked = vec![
            with_motive(
                ranked_goal(GoalKind::Sleep, 900, OpportunityAnchor::None),
                MotiveSource::NeedPressure {
                    need: HomeostaticNeedId::Hunger,
                },
                900,
            ),
            with_motive(
                ranked_goal(
                    GoalKind::TreatWounds {
                        patient: entity(70),
                    },
                    800,
                    OpportunityAnchor::Entity(entity(70)),
                ),
                MotiveSource::Pain { wound: WoundId(70) },
                800,
            ),
            with_motive(
                ranked_goal(
                    post_notice_goal(entity(72)),
                    700,
                    OpportunityAnchor::Place(entity(72)),
                ),
                MotiveSource::OfficeDuty { office: entity(72) },
                700,
            ),
            with_motive(
                ranked_goal(
                    GoalKind::SellCommodity {
                        commodity: CommodityKind::Bread,
                    },
                    600,
                    OpportunityAnchor::Place(entity(71)),
                ),
                MotiveSource::Greed {
                    opportunity: greed_opportunity,
                },
                600,
            ),
            with_motive(
                ranked_goal(
                    GoalKind::Accuse {
                        crime_register: entity(74),
                        accused: entity(75),
                        violation_id: ViolationId(76),
                    },
                    500,
                    OpportunityAnchor::Entity(entity(75)),
                ),
                MotiveSource::Revenge {
                    violation: ViolationId(76),
                },
                500,
            ),
        ];
        let ranked = ranking::sort_in_place(&mut ranked);

        let portfolio = assemble_portfolio(
            &ranked,
            None,
            &PortfolioWeightsProfile::default(),
            OperatingMode::Normal,
            |_| FeasibilityVerdict::Plausible,
        );

        assert_eq!(portfolio.slots.len(), 5);
        assert!(portfolio.slots.contains_key(&SlotKind::NeedSurvival));
        assert!(portfolio.slots.contains_key(&SlotKind::PainCare));
        assert!(portfolio.slots.contains_key(&SlotKind::ObligationDuty));
        assert!(portfolio.slots.contains_key(&SlotKind::EconomicOpportunity));
        assert!(portfolio.slots.contains_key(&SlotKind::SocialMotive));
    }

    #[test]
    fn assemble_portfolio_emergency_skips_economic_and_social() {
        let mut ranked = vec![
            with_motive(
                ranked_goal(GoalKind::Sleep, 900, OpportunityAnchor::None),
                MotiveSource::NeedPressure {
                    need: HomeostaticNeedId::Hunger,
                },
                900,
            ),
            with_motive(
                ranked_goal(
                    GoalKind::SellCommodity {
                        commodity: CommodityKind::Bread,
                    },
                    700,
                    OpportunityAnchor::Place(entity(80)),
                ),
                MotiveSource::Greed {
                    opportunity: OpportunityKey {
                        goal_key: GoalKey::from(GoalKind::SellCommodity {
                            commodity: CommodityKind::Bread,
                        }),
                        anchor: OpportunityAnchor::Place(entity(80)),
                    },
                },
                700,
            ),
            with_motive(
                ranked_goal(
                    GoalKind::Accuse {
                        crime_register: entity(81),
                        accused: entity(82),
                        violation_id: ViolationId(83),
                    },
                    600,
                    OpportunityAnchor::Entity(entity(82)),
                ),
                MotiveSource::Revenge {
                    violation: ViolationId(83),
                },
                600,
            ),
        ];
        let ranked = ranking::sort_in_place(&mut ranked);

        let portfolio = assemble_portfolio(
            &ranked,
            None,
            &PortfolioWeightsProfile::default(),
            OperatingMode::Emergency,
            |_| FeasibilityVerdict::Plausible,
        );

        assert!(portfolio.slots.contains_key(&SlotKind::NeedSurvival));
        assert!(!portfolio.slots.contains_key(&SlotKind::EconomicOpportunity));
        assert!(!portfolio.slots.contains_key(&SlotKind::SocialMotive));
    }

    #[test]
    fn primary_motive_slot_picks_highest_weight_contribution() {
        let ranked = with_motive_at_tick(
            with_motive_at_tick(
                ranked_goal(GoalKind::Sleep, 900, OpportunityAnchor::None),
                MotiveSource::NeedPressure {
                    need: HomeostaticNeedId::Hunger,
                },
                200,
                Tick(1),
            ),
            MotiveSource::Pain { wound: WoundId(90) },
            700,
            Tick(2),
        );

        assert_eq!(primary_motive_slot(&ranked), SlotKind::PainCare);
    }

    #[test]
    fn primary_motive_slot_breaks_ties_with_older_introduced_tick() {
        let ranked = with_motive_at_tick(
            with_motive_at_tick(
                ranked_goal(GoalKind::Sleep, 900, OpportunityAnchor::None),
                MotiveSource::Pain { wound: WoundId(91) },
                500,
                Tick(8),
            ),
            MotiveSource::NeedPressure {
                need: HomeostaticNeedId::Hunger,
            },
            500,
            Tick(3),
        );

        assert_eq!(primary_motive_slot(&ranked), SlotKind::NeedSurvival);
    }

    #[test]
    fn apply_mode_zeroes_economic_and_social_only() {
        let weights = PortfolioWeightsProfile {
            need_survival: Permille::new(1000).unwrap(),
            pain_care: Permille::new(900).unwrap(),
            obligation_duty: Permille::new(800).unwrap(),
            economic_opportunity: Permille::new(600).unwrap(),
            social_motive: Permille::new(400).unwrap(),
            max_plans_normal: 5,
            max_plans_emergency: 3,
            max_plans_idle: 5,
        };

        let emergency = apply_mode(&weights, OperatingMode::Emergency);

        assert_eq!(emergency.need_survival, weights.need_survival);
        assert_eq!(emergency.pain_care, weights.pain_care);
        assert_eq!(emergency.obligation_duty, weights.obligation_duty);
        assert_eq!(emergency.economic_opportunity, Permille::ZERO);
        assert_eq!(emergency.social_motive, Permille::ZERO);
    }
}
