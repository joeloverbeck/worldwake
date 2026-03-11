//! Combat-specific authoritative agent state.

use crate::{Component, Permille, Tick};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// Per-agent combat and bodily resilience parameters.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CombatProfile {
    pub wound_capacity: Permille,
    pub incapacitation_threshold: Permille,
    pub attack_skill: Permille,
    pub guard_skill: Permille,
    pub defend_bonus: Permille,
    pub natural_clot_resistance: Permille,
    pub natural_recovery_rate: Permille,
    pub unarmed_wound_severity: Permille,
    pub unarmed_bleed_rate: Permille,
    pub unarmed_attack_ticks: NonZeroU32,
}

impl CombatProfile {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        wound_capacity: Permille,
        incapacitation_threshold: Permille,
        attack_skill: Permille,
        guard_skill: Permille,
        defend_bonus: Permille,
        natural_clot_resistance: Permille,
        natural_recovery_rate: Permille,
        unarmed_wound_severity: Permille,
        unarmed_bleed_rate: Permille,
        unarmed_attack_ticks: NonZeroU32,
    ) -> Self {
        Self {
            wound_capacity,
            incapacitation_threshold,
            attack_skill,
            guard_skill,
            defend_bonus,
            natural_clot_resistance,
            natural_recovery_rate,
            unarmed_wound_severity,
            unarmed_bleed_rate,
            unarmed_attack_ticks,
        }
    }
}

impl Component for CombatProfile {}

/// Tick at which an agent died.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeadAt(pub Tick);

impl Component for DeadAt {}

/// Active combat posture projected into authoritative state.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CombatStance {
    Defending,
}

impl Component for CombatStance {}

#[cfg(test)]
mod tests {
    use super::{CombatProfile, CombatStance, DeadAt};
    use crate::{traits::Component, Permille, Tick};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;
    use std::num::NonZeroU32;

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    fn sample_combat_profile() -> CombatProfile {
        CombatProfile::new(
            pm(1000),
            pm(700),
            pm(620),
            pm(580),
            pm(80),
            pm(25),
            pm(18),
            pm(120),
            pm(35),
            nz(6),
        )
    }

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Copy + Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn assert_ordinal_value_bounds<
        T: Copy + Clone + Eq + Ord + std::hash::Hash + Debug + Serialize + DeserializeOwned,
    >() {
    }

    #[test]
    fn combat_components_satisfy_required_traits() {
        assert_component_bounds::<CombatProfile>();
        assert_component_bounds::<DeadAt>();
        assert_component_bounds::<CombatStance>();
        assert_value_bounds::<CombatProfile>();
        assert_value_bounds::<DeadAt>();
        assert_ordinal_value_bounds::<CombatStance>();
    }

    #[test]
    fn combat_profile_new_stores_every_field() {
        let profile = sample_combat_profile();

        assert_eq!(profile.wound_capacity, pm(1000));
        assert_eq!(profile.incapacitation_threshold, pm(700));
        assert_eq!(profile.attack_skill, pm(620));
        assert_eq!(profile.guard_skill, pm(580));
        assert_eq!(profile.defend_bonus, pm(80));
        assert_eq!(profile.natural_clot_resistance, pm(25));
        assert_eq!(profile.natural_recovery_rate, pm(18));
        assert_eq!(profile.unarmed_wound_severity, pm(120));
        assert_eq!(profile.unarmed_bleed_rate, pm(35));
        assert_eq!(profile.unarmed_attack_ticks, nz(6));
    }

    #[test]
    fn combat_profile_roundtrips_through_bincode() {
        let profile = sample_combat_profile();

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: CombatProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn dead_at_roundtrips_through_bincode() {
        let dead_at = DeadAt(Tick(42));

        let bytes = bincode::serialize(&dead_at).unwrap();
        let roundtrip: DeadAt = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, dead_at);
    }

    #[test]
    fn combat_stance_roundtrips_through_bincode() {
        let stance = CombatStance::Defending;

        let bytes = bincode::serialize(&stance).unwrap();
        let roundtrip: CombatStance = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, stance);
    }
}
