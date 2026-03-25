//! Crime-specific authoritative types shared across theft and justice systems.

use crate::{CommodityKind, Component, EntityId, Permille, Quantity};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// Per-agent parameters governing theft behavior.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TheftDispositionProfile {
    pub steal_duration_ticks: NonZeroU32,
    pub theft_motive_weight: Permille,
    pub witness_risk_penalty: Permille,
}

impl Component for TheftDispositionProfile {}

/// Per-agent parameters governing accusation and punishment behavior.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JusticeDispositionProfile {
    pub accusation_motive_weight: Permille,
    pub fine_severity: Permille,
}

impl Component for JusticeDispositionProfile {}

/// The kind of punishment imposed by institutional authority.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum PunishmentKind {
    Fine {
        commodity: CommodityKind,
        amount: Quantity,
    },
    Exile {
        from_faction: EntityId,
    },
}

#[cfg(test)]
mod tests {
    use super::{JusticeDispositionProfile, PunishmentKind, TheftDispositionProfile};
    use crate::{traits::Component, CommodityKind, EntityId, Permille, Quantity};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;
    use std::num::NonZeroU32;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    fn sample_theft_profile() -> TheftDispositionProfile {
        TheftDispositionProfile {
            steal_duration_ticks: nz(6),
            theft_motive_weight: pm(620),
            witness_risk_penalty: pm(180),
        }
    }

    fn sample_justice_profile() -> JusticeDispositionProfile {
        JusticeDispositionProfile {
            accusation_motive_weight: pm(700),
            fine_severity: pm(450),
        }
    }

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn assert_ordinal_value_bounds<T: Copy + Clone + Eq + Ord + Debug + Serialize + DeserializeOwned>()
    {
    }

    #[test]
    fn crime_types_satisfy_required_traits() {
        assert_component_bounds::<TheftDispositionProfile>();
        assert_component_bounds::<JusticeDispositionProfile>();
        assert_value_bounds::<TheftDispositionProfile>();
        assert_value_bounds::<JusticeDispositionProfile>();
        assert_ordinal_value_bounds::<PunishmentKind>();
    }

    #[test]
    fn theft_disposition_profile_stores_every_field() {
        let profile = sample_theft_profile();

        assert_eq!(profile.steal_duration_ticks, nz(6));
        assert_eq!(profile.theft_motive_weight, pm(620));
        assert_eq!(profile.witness_risk_penalty, pm(180));
    }

    #[test]
    fn justice_disposition_profile_stores_every_field() {
        let profile = sample_justice_profile();

        assert_eq!(profile.accusation_motive_weight, pm(700));
        assert_eq!(profile.fine_severity, pm(450));
    }

    #[test]
    fn theft_disposition_profile_roundtrips_through_bincode() {
        let profile = sample_theft_profile();

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: TheftDispositionProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn justice_disposition_profile_roundtrips_through_bincode() {
        let profile = sample_justice_profile();

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: JusticeDispositionProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn punishment_kind_roundtrips_through_bincode_for_both_variants() {
        let fine = PunishmentKind::Fine {
            commodity: CommodityKind::Coin,
            amount: Quantity(12),
        };
        let exile = PunishmentKind::Exile {
            from_faction: entity(9),
        };

        let fine_bytes = bincode::serialize(&fine).unwrap();
        let fine_roundtrip: PunishmentKind = bincode::deserialize(&fine_bytes).unwrap();
        assert_eq!(fine_roundtrip, fine);

        let exile_bytes = bincode::serialize(&exile).unwrap();
        let exile_roundtrip: PunishmentKind = bincode::deserialize(&exile_bytes).unwrap();
        assert_eq!(exile_roundtrip, exile);
    }

    #[test]
    fn punishment_kind_order_is_deterministic() {
        let mut punishments = vec![
            PunishmentKind::Exile {
                from_faction: entity(3),
            },
            PunishmentKind::Fine {
                commodity: CommodityKind::Bread,
                amount: Quantity(2),
            },
            PunishmentKind::Fine {
                commodity: CommodityKind::Coin,
                amount: Quantity(1),
            },
        ];

        punishments.sort();
        let mut sorted_again = punishments.clone();
        sorted_again.sort();

        assert_eq!(punishments, sorted_again);
    }
}
