//! Shared body-harm schema for deprivation and combat consequences.

use crate::{CombatProfile, CommodityKind, Component, EntityId, Permille, Tick};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Stable identifier for an individual wound within an agent's wound history.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct WoundId(pub u64);

impl fmt::Display for WoundId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "w{}", self.0)
    }
}

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
    pub id: WoundId,
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

impl WoundList {
    #[must_use]
    pub fn next_wound_id(&self) -> WoundId {
        WoundId(
            self.wounds
                .iter()
                .map(|wound| wound.id.0)
                .max()
                .unwrap_or(0),
        )
        .next()
    }

    #[must_use]
    pub fn wound_load(&self) -> u32 {
        self.wounds
            .iter()
            .map(|wound| u32::from(wound.severity.value()))
            .sum()
    }

    #[must_use]
    pub fn wound_ids(&self) -> Vec<WoundId> {
        self.wounds.iter().map(|wound| wound.id).collect()
    }

    #[must_use]
    pub fn has_bleeding_wounds(&self) -> bool {
        self.wounds
            .iter()
            .any(|wound| wound.bleed_rate_per_tick.value() > 0)
    }

    #[must_use]
    pub fn find_deprivation_wound(&self, kind: DeprivationKind) -> Option<&Wound> {
        self.wounds.iter().find(|wound| {
            matches!(
                wound.cause,
                WoundCause::Deprivation(existing_kind) if existing_kind == kind
            )
        })
    }

    pub fn find_deprivation_wound_mut(&mut self, kind: DeprivationKind) -> Option<&mut Wound> {
        self.wounds.iter_mut().find(|wound| {
            matches!(
                wound.cause,
                WoundCause::Deprivation(existing_kind) if existing_kind == kind
            )
        })
    }
}

#[must_use]
pub fn is_incapacitated(wounds: &WoundList, profile: &CombatProfile) -> bool {
    wounds.wound_load() >= u32::from(profile.incapacitation_threshold.value())
}

#[must_use]
pub fn is_wound_load_fatal(wounds: &WoundList, profile: &CombatProfile) -> bool {
    wounds.wound_load() >= u32::from(profile.wound_capacity.value())
}

impl Component for WoundList {}

impl WoundId {
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        is_incapacitated, is_wound_load_fatal, BodyPart, CombatWeaponRef, DeprivationKind, Wound,
        WoundCause, WoundId, WoundList,
    };
    use crate::{traits::Component, CombatProfile, CommodityKind, EntityId, Permille, Tick};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;
    use std::num::NonZeroU32;

    fn sample_wound() -> Wound {
        Wound {
            id: WoundId(1),
            body_part: BodyPart::Torso,
            cause: WoundCause::Deprivation(DeprivationKind::Starvation),
            severity: Permille::new(650).unwrap(),
            inflicted_at: Tick(9),
            bleed_rate_per_tick: Permille::new(0).unwrap(),
        }
    }

    fn sample_combat_wound() -> Wound {
        Wound {
            id: WoundId(2),
            body_part: BodyPart::Head,
            cause: WoundCause::Combat {
                attacker: EntityId {
                    slot: 4,
                    generation: 1,
                },
                weapon: CombatWeaponRef::Commodity(CommodityKind::Sword),
            },
            severity: Permille::new(800).unwrap(),
            inflicted_at: Tick(11),
            bleed_rate_per_tick: Permille::new(35).unwrap(),
        }
    }

    fn sample_combat_profile() -> CombatProfile {
        CombatProfile::new(
            Permille::new(1000).unwrap(),
            Permille::new(700).unwrap(),
            Permille::new(600).unwrap(),
            Permille::new(550).unwrap(),
            Permille::new(75).unwrap(),
            Permille::new(20).unwrap(),
            Permille::new(15).unwrap(),
            Permille::new(120).unwrap(),
            Permille::new(30).unwrap(),
            NonZeroU32::new(6).unwrap(),
            NonZeroU32::new(10).unwrap(),
        )
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
                    id: WoundId(3),
                    body_part: BodyPart::LeftLeg,
                    cause: WoundCause::Deprivation(DeprivationKind::Dehydration),
                    severity: Permille::new(400).unwrap(),
                    inflicted_at: Tick(12),
                    bleed_rate_per_tick: Permille::new(0).unwrap(),
                },
            ],
        };

        let bytes = bincode::serialize(&wound_list).unwrap();
        let roundtrip: WoundList = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, wound_list);
    }

    #[test]
    fn wound_load_is_zero_for_empty_list_and_sums_all_wounds() {
        let empty = WoundList::default();
        assert_eq!(empty.wound_load(), 0);

        let wound_list = WoundList {
            wounds: vec![
                sample_wound(),
                sample_combat_wound(),
                Wound {
                    id: WoundId(3),
                    body_part: BodyPart::LeftArm,
                    cause: WoundCause::Deprivation(DeprivationKind::Dehydration),
                    severity: Permille::new(150).unwrap(),
                    inflicted_at: Tick(13),
                    bleed_rate_per_tick: Permille::new(0).unwrap(),
                },
            ],
        };

        assert_eq!(wound_list.wound_load(), 1600);
    }

    #[test]
    fn wound_load_helpers_derive_incapacitation_and_fatality_from_profile_thresholds() {
        let profile = sample_combat_profile();
        let light = WoundList {
            wounds: vec![Wound {
                id: WoundId(1),
                body_part: BodyPart::Torso,
                cause: WoundCause::Deprivation(DeprivationKind::Starvation),
                severity: Permille::new(300).unwrap(),
                inflicted_at: Tick(1),
                bleed_rate_per_tick: Permille::new(0).unwrap(),
            }],
        };
        let incapacitated = WoundList {
            wounds: vec![Wound {
                id: WoundId(2),
                body_part: BodyPart::Torso,
                cause: WoundCause::Deprivation(DeprivationKind::Starvation),
                severity: Permille::new(700).unwrap(),
                inflicted_at: Tick(1),
                bleed_rate_per_tick: Permille::new(0).unwrap(),
            }],
        };
        let fatal = WoundList {
            wounds: vec![Wound {
                id: WoundId(3),
                body_part: BodyPart::Head,
                cause: WoundCause::Combat {
                    attacker: EntityId {
                        slot: 1,
                        generation: 0,
                    },
                    weapon: CombatWeaponRef::Unarmed,
                },
                severity: Permille::new(1000).unwrap(),
                inflicted_at: Tick(2),
                bleed_rate_per_tick: Permille::new(20).unwrap(),
            }],
        };

        assert!(!is_incapacitated(&light, &profile));
        assert!(is_incapacitated(&incapacitated, &profile));
        assert!(!is_wound_load_fatal(&incapacitated, &profile));
        assert!(is_wound_load_fatal(&fatal, &profile));
    }

    #[test]
    fn has_bleeding_wounds_detects_any_positive_bleed_rate() {
        assert!(!WoundList::default().has_bleeding_wounds());
        assert!(!WoundList {
            wounds: vec![sample_wound()]
        }
        .has_bleeding_wounds());
        assert!(WoundList {
            wounds: vec![sample_wound(), sample_combat_wound()]
        }
        .has_bleeding_wounds());
    }

    #[test]
    fn find_deprivation_wound_returns_match() {
        let wound_list = WoundList {
            wounds: vec![
                sample_combat_wound(),
                sample_wound(),
                Wound {
                    id: WoundId(3),
                    body_part: BodyPart::LeftLeg,
                    cause: WoundCause::Deprivation(DeprivationKind::Dehydration),
                    severity: Permille::new(400).unwrap(),
                    inflicted_at: Tick(12),
                    bleed_rate_per_tick: Permille::new(0).unwrap(),
                },
            ],
        };

        let starvation = wound_list.find_deprivation_wound(DeprivationKind::Starvation);
        let dehydration = wound_list.find_deprivation_wound(DeprivationKind::Dehydration);

        assert_eq!(starvation.map(|wound| wound.id), Some(WoundId(1)));
        assert_eq!(dehydration.map(|wound| wound.id), Some(WoundId(3)));
        assert_eq!(
            WoundList {
                wounds: vec![sample_combat_wound()],
            }
            .find_deprivation_wound(DeprivationKind::Starvation),
            None
        );
    }

    #[test]
    fn find_deprivation_wound_mut_updates_severity() {
        let mut wound_list = WoundList {
            wounds: vec![sample_combat_wound(), sample_wound()],
        };

        let wound = wound_list
            .find_deprivation_wound_mut(DeprivationKind::Starvation)
            .expect("starvation wound should exist");
        let wound_id = wound.id;
        wound.severity = wound.severity.saturating_add(Permille::new(200).unwrap());

        let updated = wound_list
            .find_deprivation_wound(DeprivationKind::Starvation)
            .expect("updated starvation wound should remain present");
        assert_eq!(updated.id, wound_id);
        assert_eq!(updated.severity, Permille::new(850).unwrap());
    }

    #[test]
    fn find_deprivation_wound_returns_none_for_empty_list() {
        let mut wounds = WoundList::default();

        assert_eq!(wounds.find_deprivation_wound(DeprivationKind::Starvation), None);
        assert_eq!(
            wounds.find_deprivation_wound_mut(DeprivationKind::Starvation),
            None
        );
    }

    #[test]
    fn next_wound_id_advances_past_highest_existing_identifier() {
        assert_eq!(WoundList::default().next_wound_id(), WoundId(1));
        assert_eq!(
            WoundList {
                wounds: vec![
                    Wound {
                        id: WoundId(4),
                        ..sample_wound()
                    },
                    Wound {
                        id: WoundId(9),
                        ..sample_combat_wound()
                    },
                ],
            }
            .next_wound_id(),
            WoundId(10)
        );
    }
}
