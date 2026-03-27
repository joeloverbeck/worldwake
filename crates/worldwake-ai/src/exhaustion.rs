use serde::{Deserialize, Serialize};
use worldwake_core::{
    CommodityKind, EntityId, HomeostaticNeedId, HomeostaticNeeds, Permille, Quantity,
    UniqueItemKind,
};

/// Condition that would make a previously exhausted goal worth re-searching.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ExhaustionInvalidationCondition {
    PositionChanged,
    CommodityChanged(CommodityKind),
    UniqueItemChanged(UniqueItemKind),
    WoundsChanged,
    FacilitiesChanged,
    BlockerExpired,
    HostilesChanged,
    NeedCrossedThreshold {
        need: HomeostaticNeedId,
        threshold_delta: Permille,
    },
    TargetDead(EntityId),
}

/// Snapshot of the goal-relevant state when a goal exhausted planning.
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ExhaustionBaseline {
    pub position: Option<EntityId>,
    pub needs: Option<HomeostaticNeeds>,
    pub commodity_quantities: Vec<(CommodityKind, Quantity)>,
    pub unique_item_counts: Vec<(UniqueItemKind, u32)>,
    pub wound_count: usize,
    pub hostile_count: usize,
}

#[cfg(test)]
mod tests {
    use super::ExhaustionBaseline;

    #[test]
    fn exhaustion_baseline_default_is_zero_value() {
        let baseline = ExhaustionBaseline::default();

        assert_eq!(baseline.position, None);
        assert_eq!(baseline.needs, None);
        assert!(baseline.commodity_quantities.is_empty());
        assert!(baseline.unique_item_counts.is_empty());
        assert_eq!(baseline.wound_count, 0);
        assert_eq!(baseline.hostile_count, 0);
    }
}
