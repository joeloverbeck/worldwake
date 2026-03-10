//! Shared drive-threshold schema used by physiology and AI.

use crate::{Component, Permille};
use serde::{Deserialize, Serialize};

/// Ordered urgency thresholds for a single drive or derived pressure.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ThresholdBand {
    low: Permille,
    medium: Permille,
    high: Permille,
    critical: Permille,
}

impl ThresholdBand {
    /// Create a threshold band with strictly increasing thresholds.
    pub fn new(
        low: Permille,
        medium: Permille,
        high: Permille,
        critical: Permille,
    ) -> Result<Self, &'static str> {
        if !(low < medium && medium < high && high < critical) {
            return Err("threshold band values must satisfy low < medium < high < critical");
        }

        Ok(Self {
            low,
            medium,
            high,
            critical,
        })
    }

    #[must_use]
    pub const fn low(self) -> Permille {
        self.low
    }

    #[must_use]
    pub const fn medium(self) -> Permille {
        self.medium
    }

    #[must_use]
    pub const fn high(self) -> Permille {
        self.high
    }

    #[must_use]
    pub const fn critical(self) -> Permille {
        self.critical
    }
}

/// Per-agent threshold bands for embodied and derived drive pressures.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DriveThresholds {
    pub hunger: ThresholdBand,
    pub thirst: ThresholdBand,
    pub fatigue: ThresholdBand,
    pub bladder: ThresholdBand,
    pub dirtiness: ThresholdBand,
    pub pain: ThresholdBand,
    pub danger: ThresholdBand,
}

impl DriveThresholds {
    /// Construct a complete per-agent threshold set.
    #[must_use]
    pub const fn new(
        hunger: ThresholdBand,
        thirst: ThresholdBand,
        fatigue: ThresholdBand,
        bladder: ThresholdBand,
        dirtiness: ThresholdBand,
        pain: ThresholdBand,
        danger: ThresholdBand,
    ) -> Self {
        Self {
            hunger,
            thirst,
            fatigue,
            bladder,
            dirtiness,
            pain,
            danger,
        }
    }
}

impl Component for DriveThresholds {}

impl Default for DriveThresholds {
    fn default() -> Self {
        Self::new(
            ThresholdBand::new(pm(250), pm(500), pm(750), pm(900)).unwrap(),
            ThresholdBand::new(pm(200), pm(450), pm(700), pm(850)).unwrap(),
            ThresholdBand::new(pm(300), pm(550), pm(800), pm(920)).unwrap(),
            ThresholdBand::new(pm(350), pm(600), pm(800), pm(930)).unwrap(),
            ThresholdBand::new(pm(400), pm(650), pm(850), pm(950)).unwrap(),
            ThresholdBand::new(pm(150), pm(350), pm(600), pm(850)).unwrap(),
            ThresholdBand::new(pm(100), pm(300), pm(550), pm(800)).unwrap(),
        )
    }
}

const fn pm(value: u16) -> Permille {
    Permille::new_unchecked(value)
}

#[cfg(test)]
mod tests {
    use super::{pm, DriveThresholds, ThresholdBand};
    use crate::{traits::Component, Permille};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_threshold_band_bounds<
        T: Copy + Clone + Eq + Debug + Serialize + DeserializeOwned,
    >() {
    }

    #[test]
    fn threshold_band_new_accepts_strictly_increasing_values() {
        let band = ThresholdBand::new(
            Permille::new(100).unwrap(),
            Permille::new(300).unwrap(),
            Permille::new(600).unwrap(),
            Permille::new(900).unwrap(),
        )
        .unwrap();

        assert_eq!(band.low(), Permille::new(100).unwrap());
        assert_eq!(band.medium(), Permille::new(300).unwrap());
        assert_eq!(band.high(), Permille::new(600).unwrap());
        assert_eq!(band.critical(), Permille::new(900).unwrap());
    }

    #[test]
    fn threshold_band_new_rejects_non_increasing_values() {
        let err = ThresholdBand::new(
            Permille::new(100).unwrap(),
            Permille::new(100).unwrap(),
            Permille::new(600).unwrap(),
            Permille::new(900).unwrap(),
        )
        .unwrap_err();

        assert_eq!(
            err,
            "threshold band values must satisfy low < medium < high < critical"
        );
    }

    #[test]
    fn drive_thresholds_new_stores_all_bands() {
        let hunger = ThresholdBand::new(pm(250), pm(500), pm(750), pm(900)).unwrap();
        let thirst = ThresholdBand::new(pm(200), pm(450), pm(700), pm(850)).unwrap();
        let fatigue = ThresholdBand::new(pm(300), pm(550), pm(800), pm(920)).unwrap();
        let bladder = ThresholdBand::new(pm(350), pm(600), pm(800), pm(930)).unwrap();
        let dirtiness = ThresholdBand::new(pm(400), pm(650), pm(850), pm(950)).unwrap();
        let pain = ThresholdBand::new(pm(150), pm(350), pm(600), pm(850)).unwrap();
        let danger = ThresholdBand::new(pm(100), pm(300), pm(550), pm(800)).unwrap();
        let thresholds =
            DriveThresholds::new(hunger, thirst, fatigue, bladder, dirtiness, pain, danger);

        assert_eq!(thresholds.hunger, hunger);
        assert_eq!(thresholds.thirst, thirst);
        assert_eq!(thresholds.fatigue, fatigue);
        assert_eq!(thresholds.bladder, bladder);
        assert_eq!(thresholds.dirtiness, dirtiness);
        assert_eq!(thresholds.pain, pain);
        assert_eq!(thresholds.danger, danger);
    }

    #[test]
    fn drive_thresholds_default_produces_valid_bands() {
        let thresholds = DriveThresholds::default();

        for band in [
            thresholds.hunger,
            thresholds.thirst,
            thresholds.fatigue,
            thresholds.bladder,
            thresholds.dirtiness,
            thresholds.pain,
            thresholds.danger,
        ] {
            assert!(band.low() < band.medium());
            assert!(band.medium() < band.high());
            assert!(band.high() < band.critical());
        }
    }

    #[test]
    fn drive_thresholds_component_bounds() {
        assert_component_bounds::<DriveThresholds>();
    }

    #[test]
    fn threshold_band_satisfies_required_traits() {
        assert_threshold_band_bounds::<ThresholdBand>();
    }

    #[test]
    fn drive_thresholds_roundtrip_through_bincode() {
        let thresholds = DriveThresholds::default();

        let bytes = bincode::serialize(&thresholds).unwrap();
        let roundtrip: DriveThresholds = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, thresholds);
    }
}
