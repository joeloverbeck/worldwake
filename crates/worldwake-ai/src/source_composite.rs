use crate::{GoalOffer, ranking::RankingContext};
use serde::{Deserialize, Serialize};
use worldwake_core::{
    CommodityKind, EntityId, PreferenceProfile, ReliabilityRecord, SourceKey, SourceReliability,
    Tick, failure_ratio_permille,
};

const NEUTRAL_FACTOR_PERMILLE: u32 = 1000;
const MAX_FACTOR_PERMILLE: u32 = 2000;
const WAIT_NORMALIZER_TICKS: u32 = 60;
const WAIT_PENALTY_CAP_PERMILLE: u32 = 800;
const EMPTY_FRESH_CAPACITY_FLOOR_PERMILLE: u32 = 500;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SourceCompositeRank {
    pub source_entity: EntityId,
    pub commodity: CommodityKind,
    pub trust_factor_permille: u32,
    pub wait_factor_permille: u32,
    pub capacity_factor_permille: u32,
    pub composite_permille: u32,
}

pub(crate) fn source_composite_rank(
    candidate: &GoalOffer,
    context: &RankingContext<'_>,
) -> Option<SourceCompositeRank> {
    crate::ranking::source_reliability_discount_scope(candidate)?;
    let source_reliability = context.view.source_reliability(context.agent)?;
    let profile = context.view.preference_profile(context.agent)?;
    source_composite_rank_from_reliability(
        candidate,
        &source_reliability,
        profile,
        context.current_tick,
    )
}

fn source_composite_rank_from_reliability(
    candidate: &GoalOffer,
    source_reliability: &SourceReliability,
    profile: PreferenceProfile,
    current_tick: Tick,
) -> Option<SourceCompositeRank> {
    let (source_entity, commodity) = crate::ranking::source_reliability_discount_scope(candidate)?;
    let key = SourceKey {
        entity: source_entity,
        commodity,
    };
    let record = source_reliability.sources.get(&key)?;
    let trust_factor_permille = trust_factor_permille(record, profile);
    let wait_factor_permille = wait_factor_permille(record, profile);
    let capacity_factor_permille = capacity_factor_permille(record, profile, current_tick);
    let composite_permille = compose_factors(
        trust_factor_permille,
        wait_factor_permille,
        capacity_factor_permille,
    );

    Some(SourceCompositeRank {
        source_entity,
        commodity,
        trust_factor_permille,
        wait_factor_permille,
        capacity_factor_permille,
        composite_permille,
    })
}

fn trust_factor_permille(record: &ReliabilityRecord, profile: PreferenceProfile) -> u32 {
    let failure_ratio = failure_ratio_permille(record);
    let trust_weight = u32::from(profile.source_trust_weight.value());
    NEUTRAL_FACTOR_PERMILLE
        .saturating_sub(failure_ratio.saturating_mul(trust_weight) / NEUTRAL_FACTOR_PERMILLE)
}

fn wait_factor_permille(record: &ReliabilityRecord, profile: PreferenceProfile) -> u32 {
    let wait_weight = u32::from(profile.wait_sensitivity_weight.value());
    let penalty = record.average_wait_ticks.saturating_mul(wait_weight) / WAIT_NORMALIZER_TICKS;
    NEUTRAL_FACTOR_PERMILLE.saturating_sub(penalty.min(WAIT_PENALTY_CAP_PERMILLE))
}

fn capacity_factor_permille(
    record: &ReliabilityRecord,
    profile: PreferenceProfile,
    current_tick: Tick,
) -> u32 {
    let capacity_age_ticks = current_tick
        .0
        .saturating_sub(record.last_observed_capacity_tick.0);
    if capacity_age_ticks > profile.memory_retention_ticks {
        return NEUTRAL_FACTOR_PERMILLE;
    }
    if record.wait_observation_count == 0 && record.last_observed_capacity == 0 {
        return NEUTRAL_FACTOR_PERMILLE;
    }

    let freshness_factor_permille = freshness_factor_permille(capacity_age_ticks, profile);
    if record.last_observed_capacity == 0 {
        return NEUTRAL_FACTOR_PERMILLE
            .saturating_sub(freshness_factor_permille / 2)
            .max(EMPTY_FRESH_CAPACITY_FLOOR_PERMILLE);
    }

    let capacity_signal_permille = capacity_signal_permille(record, profile);
    let bonus = capacity_signal_permille.saturating_mul(freshness_factor_permille)
        / NEUTRAL_FACTOR_PERMILLE;
    NEUTRAL_FACTOR_PERMILLE
        .saturating_add(bonus)
        .min(MAX_FACTOR_PERMILLE)
}

fn freshness_factor_permille(capacity_age_ticks: u64, profile: PreferenceProfile) -> u32 {
    if profile.memory_retention_ticks == 0 {
        return 0;
    }
    let freshness_penalty = (capacity_age_ticks.saturating_mul(u64::from(NEUTRAL_FACTOR_PERMILLE))
        / profile.memory_retention_ticks)
        .min(u64::from(NEUTRAL_FACTOR_PERMILLE)) as u32;
    NEUTRAL_FACTOR_PERMILLE.saturating_sub(freshness_penalty)
}

fn capacity_signal_permille(record: &ReliabilityRecord, profile: PreferenceProfile) -> u32 {
    let capacity_weight = u32::from(profile.capacity_observation_weight.value());
    if capacity_weight == 0 {
        return NEUTRAL_FACTOR_PERMILLE;
    }
    (u32::from(record.last_observed_capacity).saturating_mul(NEUTRAL_FACTOR_PERMILLE)
        / capacity_weight)
        .min(NEUTRAL_FACTOR_PERMILLE)
}

fn compose_factors(trust: u32, wait: u32, capacity: u32) -> u32 {
    (trust.saturating_mul(wait) / NEUTRAL_FACTOR_PERMILLE)
        .saturating_mul(capacity)
        .saturating_div(NEUTRAL_FACTOR_PERMILLE)
        .min(MAX_FACTOR_PERMILLE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use worldwake_core::{
        AcquisitionQuantity, CommodityPurpose, EntityId, GoalKey, GoalKind, OpportunityAnchor,
        Permille,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn profile() -> PreferenceProfile {
        PreferenceProfile {
            route_caution_weight: pm(300),
            source_trust_weight: pm(1000),
            route_memory_capacity: 8,
            source_memory_capacity: 8,
            memory_retention_ticks: 100,
            wait_sensitivity_weight: pm(1000),
            capacity_observation_weight: pm(20),
        }
    }

    fn record() -> ReliabilityRecord {
        ReliabilityRecord {
            last_attempt_tick: Tick(1),
            ..ReliabilityRecord::default()
        }
    }

    fn goal(kind: GoalKind) -> GoalOffer {
        GoalOffer {
            key: GoalKey::from(kind),
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            acquisition_quantity: None,
        }
    }

    fn acquisition_goal(source: EntityId, commodity: CommodityKind) -> GoalOffer {
        GoalOffer {
            key: GoalKey::from(GoalKind::AcquireCommodity {
                commodity,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::from([source]),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            acquisition_quantity: Some(AcquisitionQuantity::single()),
        }
    }

    #[test]
    fn trust_factor_neutral_without_failures() {
        let mut record = record();
        record.successful_acquisitions = 3;

        assert_eq!(trust_factor_permille(&record, profile()), 1000);
    }

    #[test]
    fn trust_factor_floors_at_zero_with_total_failures() {
        let mut record = record();
        record.failed_attempts = 3;

        assert_eq!(trust_factor_permille(&record, profile()), 0);
    }

    #[test]
    fn wait_factor_caps_at_floor_200_under_extreme_contention() {
        let mut record = record();
        record.average_wait_ticks = 10_000;

        assert_eq!(wait_factor_permille(&record, profile()), 200);
    }

    #[test]
    fn capacity_factor_neutral_for_stale_observation() {
        let mut record = record();
        record.wait_observation_count = 1;
        record.last_observed_capacity = 20;
        record.last_observed_capacity_tick = Tick(10);

        assert_eq!(
            capacity_factor_permille(&record, profile(), Tick(111)),
            1000
        );
    }

    #[test]
    fn capacity_factor_neutral_for_never_observed() {
        assert_eq!(
            capacity_factor_permille(&record(), profile(), Tick(10)),
            1000
        );
    }

    #[test]
    fn capacity_factor_floors_at_500_for_empty_fresh_observation() {
        let mut record = record();
        record.wait_observation_count = 1;
        record.last_observed_capacity = 0;
        record.last_observed_capacity_tick = Tick(10);

        assert_eq!(capacity_factor_permille(&record, profile(), Tick(10)), 500);
    }

    #[test]
    fn capacity_factor_returns_bonus_for_fresh_full_observation() {
        let mut record = record();
        record.wait_observation_count = 1;
        record.last_observed_capacity = 20;
        record.last_observed_capacity_tick = Tick(10);

        assert_eq!(capacity_factor_permille(&record, profile(), Tick(10)), 2000);
    }

    #[test]
    fn compose_factors_clamps_at_2000_permille() {
        assert_eq!(compose_factors(2000, 2000, 2000), 2000);
    }

    #[test]
    fn source_composite_rank_returns_none_for_non_acquisition_goal() {
        let reliability = SourceReliability::default();

        assert_eq!(
            source_composite_rank_from_reliability(
                &goal(GoalKind::Sleep),
                &reliability,
                profile(),
                Tick(10)
            ),
            None
        );
    }

    #[test]
    fn source_composite_rank_returns_none_without_reliability_record() {
        let source = entity(2);
        let reliability = SourceReliability {
            sources: BTreeMap::new(),
        };

        assert_eq!(
            source_composite_rank_from_reliability(
                &acquisition_goal(source, CommodityKind::Bread),
                &reliability,
                profile(),
                Tick(10)
            ),
            None
        );
    }
}
