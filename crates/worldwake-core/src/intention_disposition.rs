//! Per-agent intention disposition profile for domain-specific patience and
//! commitment switch margins.
//!
//! `IntentionDispositionProfile` replaces the travel-specific
//! `TravelDispositionProfile` with a general per-domain patience model.

use crate::intention_frame::IntentionDomainTag;
use crate::numerics::Permille;
use crate::traits::Component;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::num::NonZeroU32;

/// Per-agent disposition profile governing intention frame patience and
/// commitment switch behavior across all domains.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntentionDispositionProfile {
    /// Per-domain patience limits. If the agent's current frame's domain tag
    /// is not present, falls back to `default_patience_ticks`.
    pub domain_patience: BTreeMap<IntentionDomainTag, NonZeroU32>,
    /// Fallback patience limit when no domain-specific entry exists.
    pub default_patience_ticks: NonZeroU32,
    /// Switch margin applied when the agent has an active frame and a
    /// challenger plan would abandon it.
    pub commitment_switch_margin: Permille,
}

impl Default for IntentionDispositionProfile {
    fn default() -> Self {
        Self {
            domain_patience: BTreeMap::new(),
            default_patience_ticks: NonZeroU32::new(30).unwrap(),
            commitment_switch_margin: Permille::new_unchecked(200),
        }
    }
}

impl IntentionDispositionProfile {
    /// Returns the patience limit for a given domain tag, falling back to
    /// the default if no domain-specific entry exists.
    #[must_use]
    pub fn patience_for(&self, tag: IntentionDomainTag) -> u32 {
        self.domain_patience
            .get(&tag)
            .map_or(self.default_patience_ticks.get(), |v| v.get())
    }
}

impl Component for IntentionDispositionProfile {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{de::DeserializeOwned, Serialize as SerializeTrait};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + SerializeTrait + DeserializeOwned>() {}

    #[test]
    fn intention_disposition_profile_satisfies_component_bounds() {
        assert_component_bounds::<IntentionDispositionProfile>();
        assert_value_bounds::<IntentionDispositionProfile>();
    }

    #[test]
    fn patience_for_returns_domain_specific_value() {
        let mut domain_patience = BTreeMap::new();
        domain_patience.insert(IntentionDomainTag::Travel, NonZeroU32::new(50).unwrap());
        let profile = IntentionDispositionProfile {
            domain_patience,
            default_patience_ticks: NonZeroU32::new(30).unwrap(),
            commitment_switch_margin: Permille::new(200).unwrap(),
        };
        assert_eq!(profile.patience_for(IntentionDomainTag::Travel), 50);
    }

    #[test]
    fn patience_for_falls_back_to_default() {
        let profile = IntentionDispositionProfile {
            domain_patience: BTreeMap::new(),
            default_patience_ticks: NonZeroU32::new(30).unwrap(),
            commitment_switch_margin: Permille::new(200).unwrap(),
        };
        assert_eq!(profile.patience_for(IntentionDomainTag::Care), 30);
        assert_eq!(profile.patience_for(IntentionDomainTag::Generic), 30);
    }

    #[test]
    fn patience_for_mixed_domains() {
        let mut domain_patience = BTreeMap::new();
        domain_patience.insert(IntentionDomainTag::Travel, NonZeroU32::new(100).unwrap());
        domain_patience.insert(IntentionDomainTag::Escort, NonZeroU32::new(75).unwrap());
        let profile = IntentionDispositionProfile {
            domain_patience,
            default_patience_ticks: NonZeroU32::new(20).unwrap(),
            commitment_switch_margin: Permille::new(150).unwrap(),
        };
        assert_eq!(profile.patience_for(IntentionDomainTag::Travel), 100);
        assert_eq!(profile.patience_for(IntentionDomainTag::Escort), 75);
        assert_eq!(profile.patience_for(IntentionDomainTag::Care), 20);
        assert_eq!(profile.patience_for(IntentionDomainTag::Errand), 20);
        assert_eq!(profile.patience_for(IntentionDomainTag::Generic), 20);
    }

    #[test]
    fn intention_disposition_profile_default_matches_fixture_baseline() {
        let profile = IntentionDispositionProfile::default();

        assert!(profile.domain_patience.is_empty());
        assert_eq!(profile.default_patience_ticks, NonZeroU32::new(30).unwrap());
        assert_eq!(
            profile.commitment_switch_margin,
            Permille::new(200).unwrap()
        );
    }
}
