//! Shared goal identity types used across authoritative memory and AI planning.

use crate::{CommodityKind, EntityId, RecipeId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum CommodityPurpose {
    SelfConsume,
    Restock,
    RecipeInput(RecipeId),
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
    EngageHostile {
        target: EntityId,
    },
    ReduceDanger,
    TreatWounds {
        patient: EntityId,
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
        commodity: CommodityKind,
        destination: EntityId,
    },
    LootCorpse {
        corpse: EntityId,
    },
    BuryCorpse {
        corpse: EntityId,
        burial_site: EntityId,
    },
    ShareBelief {
        listener: EntityId,
        subject: EntityId,
    },
    ClaimOffice {
        office: EntityId,
    },
    SupportCandidateForOffice {
        office: EntityId,
        candidate: EntityId,
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
            GoalKind::EngageHostile { target }
            | GoalKind::TreatWounds { patient: target }
            | GoalKind::LootCorpse { corpse: target }
            | GoalKind::ClaimOffice { office: target } => (None, Some(target), None),
            GoalKind::MoveCargo {
                commodity,
                destination,
            } => (Some(commodity), None, Some(destination)),
            GoalKind::BuryCorpse {
                corpse,
                burial_site,
            } => (None, Some(corpse), Some(burial_site)),
            GoalKind::ShareBelief { listener, subject } => (None, Some(listener), Some(subject)),
            GoalKind::SupportCandidateForOffice { office, candidate } => {
                (None, Some(office), Some(candidate))
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
    fn goal_key_extracts_patient_for_treat_wounds() {
        let patient = entity_id(20, 0);
        let key = GoalKey::from(GoalKind::TreatWounds { patient });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(patient));
        assert_eq!(key.place, None);
    }

    #[test]
    fn treat_wounds_goal_roundtrips_through_bincode() {
        let patient = entity_id(21, 0);
        let goal = GoalKind::TreatWounds { patient };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GoalKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn goal_key_extracts_canonical_entity_for_engage_hostile() {
        let target = entity_id(8, 2);
        let key = GoalKey::from(&GoalKind::EngageHostile { target });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(target));
        assert_eq!(key.place, None);
    }

    #[test]
    fn goal_key_extracts_entity_and_place_for_move_cargo() {
        let destination = entity_id(9, 2);
        let key = GoalKey::from(GoalKind::MoveCargo {
            commodity: CommodityKind::Water,
            destination,
        });

        assert_eq!(key.commodity, Some(CommodityKind::Water));
        assert_eq!(key.entity, None);
        assert_eq!(key.place, Some(destination));
    }

    #[test]
    fn move_cargo_goal_identity_depends_on_commodity_and_destination_not_lot_identity() {
        let destination = entity_id(9, 2);
        let first = GoalKey::from(GoalKind::MoveCargo {
            commodity: CommodityKind::Water,
            destination,
        });
        let second = GoalKey::from(GoalKind::MoveCargo {
            commodity: CommodityKind::Water,
            destination,
        });

        assert_eq!(first.commodity, second.commodity);
        assert_eq!(first.entity, None);
        assert_eq!(second.entity, None);
        assert_eq!(first.place, second.place);
        assert_eq!(first, second);
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

    #[test]
    fn goal_key_extracts_listener_and_subject_for_share_belief() {
        let listener = entity_id(11, 0);
        let subject = entity_id(12, 0);
        let key = GoalKey::from(GoalKind::ShareBelief { listener, subject });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(listener));
        assert_eq!(key.place, Some(subject));
    }

    #[test]
    fn share_belief_goal_identity_distinguishes_listener_and_subject() {
        let listener_a = entity_id(13, 0);
        let listener_b = entity_id(13, 1);
        let subject_a = entity_id(14, 0);
        let subject_b = entity_id(14, 1);

        let first = GoalKey::from(GoalKind::ShareBelief {
            listener: listener_a,
            subject: subject_a,
        });
        let second = GoalKey::from(GoalKind::ShareBelief {
            listener: listener_b,
            subject: subject_a,
        });
        let third = GoalKey::from(GoalKind::ShareBelief {
            listener: listener_a,
            subject: subject_b,
        });

        assert_ne!(first, second);
        assert_ne!(first, third);
        assert_eq!(first.entity, Some(listener_a));
        assert_eq!(first.place, Some(subject_a));
    }

    #[test]
    fn claim_office_goal_roundtrips_through_bincode() {
        let office = entity_id(15, 0);
        let goal = GoalKind::ClaimOffice { office };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GoalKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn support_candidate_goal_roundtrips_through_bincode() {
        let office = entity_id(16, 0);
        let candidate = entity_id(16, 1);
        let goal = GoalKind::SupportCandidateForOffice { office, candidate };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GoalKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn goal_key_extracts_office_for_claim_office() {
        let office = entity_id(17, 0);
        let key = GoalKey::from(GoalKind::ClaimOffice { office });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(office));
        assert_eq!(key.place, None);
    }

    #[test]
    fn goal_key_extracts_office_and_candidate_for_support_candidate() {
        let office = entity_id(18, 0);
        let candidate = entity_id(18, 1);
        let key = GoalKey::from(GoalKind::SupportCandidateForOffice { office, candidate });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(office));
        assert_eq!(key.place, Some(candidate));
    }

    #[test]
    fn support_candidate_goal_identity_distinguishes_candidate() {
        let office = entity_id(19, 0);
        let candidate_a = entity_id(19, 1);
        let candidate_b = entity_id(19, 2);

        let first = GoalKey::from(GoalKind::SupportCandidateForOffice {
            office,
            candidate: candidate_a,
        });
        let second = GoalKey::from(GoalKind::SupportCandidateForOffice {
            office,
            candidate: candidate_b,
        });

        assert_ne!(first, second);
        assert_eq!(first.entity, Some(office));
        assert_eq!(first.place, Some(candidate_a));
    }
}
