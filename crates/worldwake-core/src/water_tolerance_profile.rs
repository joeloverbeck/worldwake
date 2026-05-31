//! Per-agent tolerance to water quality.

use crate::{Component, Permille, WaterQuality};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Universal per-agent profile for water-quality consequences.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WaterToleranceProfile {
    /// Per-quality relief multiplier applied to water's intrinsic thirst relief.
    #[serde(default)]
    pub thirst_relief_factor: BTreeMap<WaterQuality, Permille>,
    /// Per-quality dirtiness penalty added when consuming water.
    #[serde(default)]
    pub dirtiness_penalty: BTreeMap<WaterQuality, Permille>,
}

impl Component for WaterToleranceProfile {}

impl Default for WaterToleranceProfile {
    fn default() -> Self {
        Self {
            thirst_relief_factor: BTreeMap::from([
                (WaterQuality::Clean, Permille::new(1000).unwrap()),
                (WaterQuality::Stale, Permille::new(700).unwrap()),
                (WaterQuality::Muddy, Permille::new(450).unwrap()),
            ]),
            dirtiness_penalty: BTreeMap::from([
                (WaterQuality::Clean, Permille::ZERO),
                (WaterQuality::Stale, Permille::new(80).unwrap()),
                (WaterQuality::Muddy, Permille::new(200).unwrap()),
            ]),
        }
    }
}

impl WaterToleranceProfile {
    #[must_use]
    pub fn thirst_relief_factor(&self, quality: WaterQuality) -> Permille {
        self.thirst_relief_factor
            .get(&quality)
            .copied()
            .unwrap_or_else(|| Permille::new(1000).unwrap())
    }

    #[must_use]
    pub fn dirtiness_penalty(&self, quality: WaterQuality) -> Permille {
        self.dirtiness_penalty
            .get(&quality)
            .copied()
            .unwrap_or(Permille::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::WaterToleranceProfile;
    use crate::{Permille, WaterQuality};
    use std::collections::BTreeMap;

    #[test]
    fn water_tolerance_profile_default_values() {
        let profile = WaterToleranceProfile::default();

        assert_eq!(
            profile.thirst_relief_factor(WaterQuality::Clean),
            Permille::new(1000).unwrap()
        );
        assert_eq!(
            profile.thirst_relief_factor(WaterQuality::Stale),
            Permille::new(700).unwrap()
        );
        assert_eq!(
            profile.thirst_relief_factor(WaterQuality::Muddy),
            Permille::new(450).unwrap()
        );
        assert_eq!(
            profile.dirtiness_penalty(WaterQuality::Clean),
            Permille::ZERO
        );
        assert_eq!(
            profile.dirtiness_penalty(WaterQuality::Stale),
            Permille::new(80).unwrap()
        );
        assert_eq!(
            profile.dirtiness_penalty(WaterQuality::Muddy),
            Permille::new(200).unwrap()
        );
    }

    #[test]
    fn water_tolerance_profile_accessor_methods_use_neutral_missing_values() {
        let profile = WaterToleranceProfile {
            thirst_relief_factor: BTreeMap::from([(
                WaterQuality::Muddy,
                Permille::new(300).unwrap(),
            )]),
            dirtiness_penalty: BTreeMap::from([(WaterQuality::Muddy, Permille::new(250).unwrap())]),
        };

        assert_eq!(
            profile.thirst_relief_factor(WaterQuality::Muddy),
            Permille::new(300).unwrap()
        );
        assert_eq!(
            profile.dirtiness_penalty(WaterQuality::Muddy),
            Permille::new(250).unwrap()
        );
        assert_eq!(
            profile.thirst_relief_factor(WaterQuality::Clean),
            Permille::new(1000).unwrap()
        );
        assert_eq!(
            profile.dirtiness_penalty(WaterQuality::Clean),
            Permille::ZERO
        );
    }

    #[test]
    fn water_tolerance_profile_serialization_roundtrip() {
        for profile in [
            WaterToleranceProfile::default(),
            WaterToleranceProfile {
                thirst_relief_factor: BTreeMap::from([
                    (WaterQuality::Clean, Permille::new(1000).unwrap()),
                    (WaterQuality::Muddy, Permille::new(250).unwrap()),
                ]),
                dirtiness_penalty: BTreeMap::from([
                    (WaterQuality::Clean, Permille::ZERO),
                    (WaterQuality::Muddy, Permille::new(300).unwrap()),
                ]),
            },
        ] {
            let bytes = bincode::serialize(&profile).unwrap();
            let roundtrip: WaterToleranceProfile = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, profile);
        }
    }
}
