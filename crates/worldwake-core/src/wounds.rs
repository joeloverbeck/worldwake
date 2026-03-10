//! Shared body-harm schema for deprivation and combat consequences.

use crate::{Component, Permille, Tick};
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

/// What caused a wound.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum WoundCause {
    Deprivation(DeprivationKind),
}

/// A single wound on an agent's body.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Wound {
    pub body_part: BodyPart,
    pub cause: WoundCause,
    pub severity: Permille,
    pub inflicted_at: Tick,
}

/// Authoritative list of wounds on an agent.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WoundList {
    pub wounds: Vec<Wound>,
}

impl Component for WoundList {}

#[cfg(test)]
mod tests {
    use super::{BodyPart, DeprivationKind, Wound, WoundCause, WoundList};
    use crate::{traits::Component, Permille, Tick};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;

    fn sample_wound() -> Wound {
        Wound {
            body_part: BodyPart::Torso,
            cause: WoundCause::Deprivation(DeprivationKind::Starvation),
            severity: Permille::new(650).unwrap(),
            inflicted_at: Tick(9),
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
        assert_enum_bounds::<WoundCause>();
    }

    #[test]
    fn wound_roundtrips_through_bincode() {
        let wound = sample_wound();

        let bytes = bincode::serialize(&wound).unwrap();
        let roundtrip: Wound = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, wound);
    }

    #[test]
    fn wound_list_roundtrips_through_bincode() {
        let wound_list = WoundList {
            wounds: vec![
                sample_wound(),
                Wound {
                    body_part: BodyPart::LeftLeg,
                    cause: WoundCause::Deprivation(DeprivationKind::Dehydration),
                    severity: Permille::new(400).unwrap(),
                    inflicted_at: Tick(12),
                },
            ],
        };

        let bytes = bincode::serialize(&wound_list).unwrap();
        let roundtrip: WoundList = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, wound_list);
    }
}
