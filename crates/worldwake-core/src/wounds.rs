//! Shared body-harm schema for deprivation and combat consequences.

use crate::{CommodityKind, Component, EntityId, Permille, Tick};
use serde::{Deserialize, Serialize};

/// Body part targeted by a wound.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum BodyPart {
    Head,
    Torso,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

/// Deprivation-specific wound source.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum DeprivationKind {
    Starvation,
    Dehydration,
}

/// Weapon provenance recorded on combat wounds.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CombatWeaponRef {
    Unarmed,
    Commodity(CommodityKind),
}

/// What caused a wound.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum WoundCause {
    Deprivation(DeprivationKind),
    Combat {
        attacker: EntityId,
        weapon: CombatWeaponRef,
    },
}

/// A single wound on an agent's body.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Wound {
    pub body_part: BodyPart,
    pub cause: WoundCause,
    pub severity: Permille,
    pub inflicted_at: Tick,
    pub bleed_rate_per_tick: Permille,
}

/// Authoritative list of wounds on an agent.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WoundList {
    pub wounds: Vec<Wound>,
}

impl Component for WoundList {}

#[cfg(test)]
mod tests {
    use super::{BodyPart, CombatWeaponRef, DeprivationKind, Wound, WoundCause, WoundList};
    use crate::{traits::Component, CommodityKind, EntityId, Permille, Tick};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;

    fn sample_wound() -> Wound {
        Wound {
            body_part: BodyPart::Torso,
            cause: WoundCause::Deprivation(DeprivationKind::Starvation),
            severity: Permille::new(650).unwrap(),
            inflicted_at: Tick(9),
            bleed_rate_per_tick: Permille::new(0).unwrap(),
        }
    }

    fn sample_combat_wound() -> Wound {
        Wound {
            body_part: BodyPart::Head,
            cause: WoundCause::Combat {
                attacker: EntityId {
                    slot: 4,
                    generation: 1,
                },
                weapon: CombatWeaponRef::Commodity(CommodityKind::Medicine),
            },
            severity: Permille::new(800).unwrap(),
            inflicted_at: Tick(11),
            bleed_rate_per_tick: Permille::new(35).unwrap(),
        }
    }

    fn assert_component_bounds<T: Component>() {}

    fn assert_enum_bounds<
        T: Copy + Clone + Eq + Ord + std::hash::Hash + Debug + Serialize + DeserializeOwned,
    >() {
    }

    #[test]
    fn wound_list_component_bounds() {
        assert_component_bounds::<WoundList>();
    }

    #[test]
    fn wound_enums_satisfy_required_traits() {
        assert_enum_bounds::<BodyPart>();
        assert_enum_bounds::<DeprivationKind>();
        assert_enum_bounds::<CombatWeaponRef>();
        assert_enum_bounds::<WoundCause>();
    }

    #[test]
    fn wound_with_zero_bleed_rate_roundtrips_through_bincode() {
        let wound = sample_wound();

        let bytes = bincode::serialize(&wound).unwrap();
        let roundtrip: Wound = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, wound);
    }

    #[test]
    fn combat_cause_roundtrips_through_bincode() {
        let cause = WoundCause::Combat {
            attacker: EntityId {
                slot: 9,
                generation: 2,
            },
            weapon: CombatWeaponRef::Unarmed,
        };

        let bytes = bincode::serialize(&cause).unwrap();
        let roundtrip: WoundCause = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, cause);
    }

    #[test]
    fn wound_with_bleed_rate_roundtrips_through_bincode() {
        let wound = sample_combat_wound();

        let bytes = bincode::serialize(&wound).unwrap();
        let roundtrip: Wound = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, wound);
    }

    #[test]
    fn mixed_cause_wound_list_roundtrips_through_bincode() {
        let wound_list = WoundList {
            wounds: vec![
                sample_wound(),
                sample_combat_wound(),
                Wound {
                    body_part: BodyPart::LeftLeg,
                    cause: WoundCause::Deprivation(DeprivationKind::Dehydration),
                    severity: Permille::new(400).unwrap(),
                    inflicted_at: Tick(12),
                    bleed_rate_per_tick: Permille::new(0).unwrap(),
                }
            ],
        };

        let bytes = bincode::serialize(&wound_list).unwrap();
        let roundtrip: WoundList = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, wound_list);
    }
}
