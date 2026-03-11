//! Shared goal identity types used across authoritative memory and AI planning.

use crate::{CommodityKind, EntityId, RecipeId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum CommodityPurpose {
    SelfConsume,
    Restock,
    RecipeInput(RecipeId),
    Treatment,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum GoalKind {
    ConsumeOwnedCommodity {
        commodity: CommodityKind,
    },
    AcquireCommodity {
        commodity: CommodityKind,
        purpose: CommodityPurpose,
    },
    Sleep,
    Relieve,
    Wash,
    ReduceDanger,
    Heal {
        target: EntityId,
    },
    ProduceCommodity {
        recipe_id: RecipeId,
    },
    SellCommodity {
        commodity: CommodityKind,
    },
    RestockCommodity {
        commodity: CommodityKind,
    },
    MoveCargo {
        lot: EntityId,
        destination: EntityId,
    },
    LootCorpse {
        corpse: EntityId,
    },
    BuryCorpse {
        corpse: EntityId,
        burial_site: EntityId,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct GoalKey {
    pub kind: GoalKind,
    pub commodity: Option<CommodityKind>,
    pub entity: Option<EntityId>,
    pub place: Option<EntityId>,
}

impl GoalKey {
    pub fn new(kind: GoalKind) -> Self {
        Self::from(kind)
    }
}

impl From<GoalKind> for GoalKey {
    fn from(kind: GoalKind) -> Self {
        let (commodity, entity, place) = match kind {
            GoalKind::ConsumeOwnedCommodity { commodity }
            | GoalKind::AcquireCommodity { commodity, .. }
            | GoalKind::SellCommodity { commodity }
            | GoalKind::RestockCommodity { commodity } => (Some(commodity), None, None),
            GoalKind::Heal { target } | GoalKind::LootCorpse { corpse: target } => {
                (None, Some(target), None)
            }
            GoalKind::MoveCargo { lot, destination } => (None, Some(lot), Some(destination)),
            GoalKind::BuryCorpse { corpse, burial_site } => {
                (None, Some(corpse), Some(burial_site))
            }
            GoalKind::Sleep
            | GoalKind::Relieve
            | GoalKind::Wash
            | GoalKind::ReduceDanger
            | GoalKind::ProduceCommodity { .. } => (None, None, None),
        };

        Self {
            kind,
            commodity,
            entity,
            place,
        }
    }
}

impl From<&GoalKind> for GoalKey {
    fn from(kind: &GoalKind) -> Self {
        Self::from(*kind)
    }
}

#[cfg(test)]
mod tests {
    use super::{CommodityPurpose, GoalKey, GoalKind};
    use crate::{test_utils::entity_id, CommodityKind, RecipeId};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;

    fn assert_value_bounds<T: Clone + Eq + Ord + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn goal_model_types_satisfy_value_bounds() {
        assert_value_bounds::<CommodityPurpose>();
        assert_value_bounds::<GoalKind>();
        assert_value_bounds::<GoalKey>();
    }

    #[test]
    fn goal_key_extracts_canonical_commodity_for_acquisition() {
        let key = GoalKey::from(&GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
        });

        assert_eq!(key.commodity, Some(CommodityKind::Apple));
        assert_eq!(key.entity, None);
        assert_eq!(key.place, None);
    }

    #[test]
    fn goal_key_extracts_canonical_entity_for_loot_corpse() {
        let corpse = entity_id(8, 1);
        let key = GoalKey::from(&GoalKind::LootCorpse { corpse });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(corpse));
        assert_eq!(key.place, None);
    }

    #[test]
    fn goal_key_extracts_entity_and_place_for_move_cargo() {
        let lot = entity_id(4, 0);
        let destination = entity_id(9, 2);
        let key = GoalKey::from(GoalKind::MoveCargo { lot, destination });

        assert_eq!(key.entity, Some(lot));
        assert_eq!(key.place, Some(destination));
    }

    #[test]
    fn goal_key_sleep_has_no_canonical_target_fields() {
        let key = GoalKey::from(&GoalKind::Sleep);

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, None);
        assert_eq!(key.place, None);
    }

    #[test]
    fn goal_key_roundtrips_through_bincode() {
        let key = GoalKey::from(GoalKind::ProduceCommodity {
            recipe_id: RecipeId(12),
        });

        let bytes = bincode::serialize(&key).unwrap();
        let roundtrip: GoalKey = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, key);
    }
}
