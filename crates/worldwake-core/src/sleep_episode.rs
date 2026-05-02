//! Duration-bearing sleep episode state and per-place recovery quality.

use crate::{Component, EntityId, HomeostaticNeedId, Permille, Tick};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// Sleep recovery multiplier in permille units where `1000 == 1x`.
///
/// Unlike [`Permille`], this can represent above-default recovery
/// (`> 1000`) while still allowing below-default sites (`< 1000`).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct SleepRecoveryModifier(u16);

impl SleepRecoveryModifier {
    pub const IDENTITY: Self = Self(1000);

    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(self) -> u16 {
        self.0
    }
}

/// A condition that can end an active sleep episode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum WakeCondition {
    IntendedDurationReached,
    TargetRecoveryReached,
    ProjectedNeedBreach { need: HomeostaticNeedId },
    ScheduledCommitmentDue { tick: Tick },
    LocalDisturbance,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SleepEpisode {
    pub place: EntityId,
    pub start_tick: Tick,
    pub intended_min_ticks: NonZeroU32,
    pub intended_max_ticks: NonZeroU32,
    pub target_recovery: Permille,
    pub accumulated_recovery: Permille,
    pub recovery_modifier: SleepRecoveryModifier,
    pub wake_conditions: Vec<WakeCondition>,
}

impl Component for SleepEpisode {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ShelterTag {
    Open,
    PartialCover,
    Roofed,
    Shelter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum GroundComfortTag {
    Hard,
    Earth,
    Soft,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct SleepQualityProfile {
    pub shelter: ShelterTag,
    pub ground_comfort: GroundComfortTag,
    pub recovery_modifier: SleepRecoveryModifier,
}

impl Component for SleepQualityProfile {}

impl Default for SleepQualityProfile {
    fn default() -> Self {
        Self {
            shelter: ShelterTag::Open,
            ground_comfort: GroundComfortTag::Earth,
            recovery_modifier: SleepRecoveryModifier::IDENTITY,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GroundComfortTag, ShelterTag, SleepEpisode, SleepQualityProfile, SleepRecoveryModifier,
        WakeCondition,
    };
    use crate::{EntityId, HomeostaticNeedId, Permille, Tick};
    use std::num::NonZeroU32;

    #[test]
    fn sleep_quality_profile_default_is_unmodulated_open_earth() {
        assert_eq!(
            SleepQualityProfile::default(),
            SleepQualityProfile {
                shelter: ShelterTag::Open,
                ground_comfort: GroundComfortTag::Earth,
                recovery_modifier: SleepRecoveryModifier::IDENTITY,
            }
        );
    }

    #[test]
    fn sleep_recovery_modifier_represents_below_and_above_default_quality() {
        assert_eq!(SleepRecoveryModifier::new(875).value(), 875);
        assert_eq!(SleepRecoveryModifier::IDENTITY.value(), 1000);
        assert_eq!(SleepRecoveryModifier::new(1250).value(), 1250);
    }

    #[test]
    fn sleep_episode_roundtrips_through_bincode() {
        let episode = SleepEpisode {
            place: EntityId {
                slot: 7,
                generation: 0,
            },
            start_tick: Tick(12),
            intended_min_ticks: NonZeroU32::new(4).unwrap(),
            intended_max_ticks: NonZeroU32::new(40).unwrap(),
            target_recovery: Permille::new(700).unwrap(),
            accumulated_recovery: Permille::new(125).unwrap(),
            recovery_modifier: SleepRecoveryModifier::new(1250),
            wake_conditions: vec![
                WakeCondition::IntendedDurationReached,
                WakeCondition::TargetRecoveryReached,
                WakeCondition::ProjectedNeedBreach {
                    need: HomeostaticNeedId::Thirst,
                },
                WakeCondition::ScheduledCommitmentDue { tick: Tick(30) },
                WakeCondition::LocalDisturbance,
            ],
        };

        let bytes = bincode::serialize(&episode).unwrap();
        let roundtrip: SleepEpisode = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, episode);
    }

    #[test]
    fn sleep_quality_profile_roundtrips_through_bincode() {
        let profile = SleepQualityProfile {
            shelter: ShelterTag::Shelter,
            ground_comfort: GroundComfortTag::Soft,
            recovery_modifier: SleepRecoveryModifier::new(1250),
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: SleepQualityProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }
}
