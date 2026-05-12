use crate::{Component, Permille};
use serde::{Deserialize, Serialize};

/// Per-agent legal and social-norm weights used when ranking opportunities.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LawAbidingProfile {
    /// Minimum legal-risk salience before an opportunity is treated as criminally unacceptable.
    pub criminal_threshold: Permille,
    /// Weight assigned to non-criminal social norms and shame costs.
    pub social_norm_weight: Permille,
}

impl Default for LawAbidingProfile {
    fn default() -> Self {
        Self {
            criminal_threshold: Permille::ZERO,
            social_norm_weight: Permille::ZERO,
        }
    }
}

impl Component for LawAbidingProfile {}

#[cfg(test)]
mod tests {
    use super::LawAbidingProfile;
    use crate::Permille;

    #[test]
    fn default_is_zero_weighted() {
        let profile = LawAbidingProfile::default();

        assert_eq!(profile.criminal_threshold, Permille::ZERO);
        assert_eq!(profile.social_norm_weight, Permille::ZERO);
    }

    #[test]
    fn roundtrips_through_bincode() {
        let profile = LawAbidingProfile {
            criminal_threshold: Permille::new_unchecked(600),
            social_norm_weight: Permille::new_unchecked(275),
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: LawAbidingProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }
}
