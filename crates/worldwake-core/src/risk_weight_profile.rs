use crate::{Component, Permille};
use serde::{Deserialize, Serialize};

/// Per-agent risk-aversion weights used when ranking opportunities.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiskWeightProfile {
    /// Penalty weight for opportunities whose lawful path involves theft.
    pub theft_aversion: Permille,
    /// Penalty weight for opportunities with public or socially exposed execution.
    pub exposure_aversion: Permille,
    /// Penalty weight for opportunities with nearby physical danger.
    pub threat_aversion: Permille,
}

impl Default for RiskWeightProfile {
    fn default() -> Self {
        Self {
            theft_aversion: Permille::ZERO,
            exposure_aversion: Permille::ZERO,
            threat_aversion: Permille::ZERO,
        }
    }
}

impl Component for RiskWeightProfile {}

#[cfg(test)]
mod tests {
    use super::RiskWeightProfile;
    use crate::Permille;

    #[test]
    fn default_is_zero_weighted() {
        let profile = RiskWeightProfile::default();

        assert_eq!(profile.theft_aversion, Permille::ZERO);
        assert_eq!(profile.exposure_aversion, Permille::ZERO);
        assert_eq!(profile.threat_aversion, Permille::ZERO);
    }

    #[test]
    fn roundtrips_through_bincode() {
        let profile = RiskWeightProfile {
            theft_aversion: Permille::new_unchecked(250),
            exposure_aversion: Permille::new_unchecked(400),
            threat_aversion: Permille::new_unchecked(900),
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: RiskWeightProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }
}
