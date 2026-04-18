use crate::{Component, HomeostaticNeedId, Permille};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Multiplier scale expressed in permille units where `1000 == 1x`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct MultiplierPermille(u16);

impl MultiplierPermille {
    pub const IDENTITY: Self = Self(1000);

    /// Create a new multiplier scale value. `1000` means `1x`.
    pub const fn new(value: u16) -> Result<Self, &'static str> {
        if value < 1000 {
            Err("MultiplierPermille value must be >= 1000")
        } else {
            Ok(Self(value))
        }
    }

    /// Create a multiplier scale value without validation.
    ///
    /// # Safety (logical)
    /// Caller must ensure `value >= 1000`.
    pub const fn new_unchecked(value: u16) -> Self {
        assert!(value >= 1000, "MultiplierPermille value must be >= 1000");
        Self(value)
    }

    #[must_use]
    pub const fn value(self) -> u16 {
        self.0
    }
}

/// Per-agent escalation profile for sustained critical homeostatic needs.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct DriveEscalationProfile {
    /// Per-need overrides. Missing entries fall back to `default_per_need`.
    pub per_need: BTreeMap<HomeostaticNeedId, DriveEscalationParams>,
    pub default_per_need: DriveEscalationParams,
}

/// Escalation parameters for one homeostatic need.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DriveEscalationParams {
    /// How many ticks above the critical threshold before escalation begins.
    pub start_after_ticks: u32,
    /// Multiplier growth per tick after start, expressed in permille units.
    pub growth_per_tick: Permille,
    /// Hard cap on the multiplier scale where `1000 == 1x`.
    pub max_multiplier: MultiplierPermille,
}

impl Component for DriveEscalationProfile {}

impl Default for DriveEscalationParams {
    fn default() -> Self {
        Self {
            start_after_ticks: 100,
            growth_per_tick: Permille::new_unchecked(10),
            max_multiplier: MultiplierPermille::new_unchecked(3000),
        }
    }
}

impl DriveEscalationProfile {
    #[must_use]
    pub fn params_for(&self, need: HomeostaticNeedId) -> DriveEscalationParams {
        self.per_need
            .get(&need)
            .copied()
            .unwrap_or(self.default_per_need)
    }
}

/// Compute the motive-score multiplier for a sustained critical run.
#[must_use]
pub fn escalation_multiplier(
    ticks_over_critical: u32,
    params: DriveEscalationParams,
) -> MultiplierPermille {
    if ticks_over_critical <= params.start_after_ticks {
        return MultiplierPermille::IDENTITY;
    }
    let over_start = ticks_over_critical - params.start_after_ticks;
    let raw = 1000u32
        .saturating_add(over_start.saturating_mul(u32::from(params.growth_per_tick.value())));
    let capped = raw.min(u32::from(params.max_multiplier.value()));
    MultiplierPermille::new_unchecked(capped as u16)
}

#[cfg(test)]
mod tests {
    use super::{
        DriveEscalationParams, DriveEscalationProfile, MultiplierPermille, escalation_multiplier,
    };
    use crate::{HomeostaticNeedId, Permille, traits::Component};
    use serde::{Serialize, de::DeserializeOwned};
    use std::collections::BTreeMap;
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Copy + Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn multiplier_permille_rejects_values_below_identity() {
        assert_eq!(
            MultiplierPermille::new(999),
            Err("MultiplierPermille value must be >= 1000")
        );
    }

    #[test]
    fn multiplier_permille_accepts_identity_and_above() {
        assert_eq!(
            MultiplierPermille::new(1000).unwrap(),
            MultiplierPermille::IDENTITY
        );
        assert_eq!(MultiplierPermille::new(3000).unwrap().value(), 3000);
    }

    #[test]
    fn drive_escalation_profile_default_matches_engine_constants() {
        let profile = DriveEscalationProfile::default();

        assert!(profile.per_need.is_empty());
        assert_eq!(profile.default_per_need.start_after_ticks, 100);
        assert_eq!(
            profile.default_per_need.growth_per_tick,
            Permille::new(10).unwrap()
        );
        assert_eq!(
            profile.default_per_need.max_multiplier,
            MultiplierPermille::new(3000).unwrap()
        );
    }

    #[test]
    fn params_for_falls_back_to_default_when_need_has_no_override() {
        let profile = DriveEscalationProfile::default();

        assert_eq!(
            profile.params_for(HomeostaticNeedId::Dirtiness),
            profile.default_per_need
        );
    }

    #[test]
    fn params_for_prefers_per_need_override() {
        let override_params = DriveEscalationParams {
            start_after_ticks: 12,
            growth_per_tick: Permille::new(25).unwrap(),
            max_multiplier: MultiplierPermille::new(1800).unwrap(),
        };
        let profile = DriveEscalationProfile {
            per_need: BTreeMap::from([(HomeostaticNeedId::Hunger, override_params)]),
            default_per_need: DriveEscalationParams::default(),
        };

        assert_eq!(
            profile.params_for(HomeostaticNeedId::Hunger),
            override_params
        );
    }

    #[test]
    fn escalation_multiplier_is_identity_at_and_below_start_after() {
        let params = DriveEscalationParams::default();

        assert_eq!(
            escalation_multiplier(params.start_after_ticks, params),
            MultiplierPermille::IDENTITY
        );
        assert_eq!(
            escalation_multiplier(params.start_after_ticks.saturating_sub(1), params),
            MultiplierPermille::IDENTITY
        );
    }

    #[test]
    fn escalation_multiplier_grows_linearly_above_start_after() {
        let params = DriveEscalationParams {
            start_after_ticks: 100,
            growth_per_tick: Permille::new(10).unwrap(),
            max_multiplier: MultiplierPermille::new(3000).unwrap(),
        };

        assert_eq!(
            escalation_multiplier(150, params),
            MultiplierPermille::new(1500).unwrap()
        );
    }

    #[test]
    fn escalation_multiplier_saturates_at_max_multiplier() {
        let params = DriveEscalationParams {
            start_after_ticks: 10,
            growth_per_tick: Permille::new(50).unwrap(),
            max_multiplier: MultiplierPermille::new(1600).unwrap(),
        };

        assert_eq!(
            escalation_multiplier(1000, params),
            MultiplierPermille::new(1600).unwrap()
        );
    }

    #[test]
    fn escalation_multiplier_handles_u32_max_ticks_without_panic() {
        let params = DriveEscalationParams {
            start_after_ticks: 0,
            growth_per_tick: Permille::new(1000).unwrap(),
            max_multiplier: MultiplierPermille::new(u16::MAX).unwrap(),
        };

        assert_eq!(
            escalation_multiplier(u32::MAX, params),
            MultiplierPermille::new(u16::MAX).unwrap()
        );
    }

    #[test]
    fn drive_escalation_types_roundtrip_through_bincode() {
        let params = DriveEscalationParams {
            start_after_ticks: 55,
            growth_per_tick: Permille::new(15).unwrap(),
            max_multiplier: MultiplierPermille::new(2500).unwrap(),
        };
        let profile = DriveEscalationProfile {
            per_need: BTreeMap::from([(HomeostaticNeedId::Bladder, params)]),
            default_per_need: DriveEscalationParams::default(),
        };

        let params_bytes = bincode::serialize(&params).unwrap();
        let profile_bytes = bincode::serialize(&profile).unwrap();
        let multiplier_bytes = bincode::serialize(&MultiplierPermille::new(1750).unwrap()).unwrap();

        let roundtrip_params: DriveEscalationParams = bincode::deserialize(&params_bytes).unwrap();
        let roundtrip_profile: DriveEscalationProfile =
            bincode::deserialize(&profile_bytes).unwrap();
        let roundtrip_multiplier: MultiplierPermille =
            bincode::deserialize(&multiplier_bytes).unwrap();

        assert_eq!(roundtrip_params, params);
        assert_eq!(roundtrip_profile, profile);
        assert_eq!(roundtrip_multiplier, MultiplierPermille::new(1750).unwrap());
    }

    #[test]
    fn drive_escalation_types_satisfy_required_traits() {
        assert_component_bounds::<DriveEscalationProfile>();
        assert_value_bounds::<DriveEscalationParams>();
        assert_value_bounds::<MultiplierPermille>();
    }
}
