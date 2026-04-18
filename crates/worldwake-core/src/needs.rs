//! Concrete physiology state and per-agent metabolism parameters.

use crate::{Component, Permille};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// Authoritative current body state for agent physiology.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct HomeostaticNeeds {
    pub hunger: Permille,
    pub thirst: Permille,
    pub fatigue: Permille,
    pub bladder: Permille,
    pub dirtiness: Permille,
}

/// Identifies a specific homeostatic need field by name.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum HomeostaticNeedId {
    Hunger,
    Thirst,
    Fatigue,
    Bladder,
    Dirtiness,
}

impl HomeostaticNeedId {
    pub const ALL: [Self; 5] = [
        Self::Hunger,
        Self::Thirst,
        Self::Fatigue,
        Self::Bladder,
        Self::Dirtiness,
    ];

    pub const VARIANT_COUNT: usize = 5;
}

impl HomeostaticNeeds {
    #[must_use]
    pub const fn new(
        hunger: Permille,
        thirst: Permille,
        fatigue: Permille,
        bladder: Permille,
        dirtiness: Permille,
    ) -> Self {
        Self {
            hunger,
            thirst,
            fatigue,
            bladder,
            dirtiness,
        }
    }

    #[must_use]
    pub const fn new_sated() -> Self {
        Self::new(pm(0), pm(0), pm(0), pm(0), pm(0))
    }

    #[must_use]
    pub fn max_value(&self) -> u16 {
        let max = self.hunger.value().max(self.thirst.value());
        let max = max.max(self.fatigue.value());
        let max = max.max(self.bladder.value());
        max.max(self.dirtiness.value())
    }

    #[must_use]
    pub const fn value(&self, need: HomeostaticNeedId) -> Permille {
        match need {
            HomeostaticNeedId::Hunger => self.hunger,
            HomeostaticNeedId::Thirst => self.thirst,
            HomeostaticNeedId::Fatigue => self.fatigue,
            HomeostaticNeedId::Bladder => self.bladder,
            HomeostaticNeedId::Dirtiness => self.dirtiness,
        }
    }
}

impl Component for HomeostaticNeeds {}

impl Default for HomeostaticNeeds {
    fn default() -> Self {
        Self::new_sated()
    }
}

/// Sustained time spent at critical physiological pressure levels.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeprivationExposure {
    pub hunger_critical_ticks: u32,
    pub thirst_critical_ticks: u32,
    pub fatigue_critical_ticks: u32,
    pub bladder_critical_ticks: u32,
    pub dirtiness_critical_ticks: u32,
}

impl Component for DeprivationExposure {}

impl DeprivationExposure {
    #[must_use]
    pub const fn ticks_at_critical(&self, need: HomeostaticNeedId) -> u32 {
        match need {
            HomeostaticNeedId::Hunger => self.hunger_critical_ticks,
            HomeostaticNeedId::Thirst => self.thirst_critical_ticks,
            HomeostaticNeedId::Fatigue => self.fatigue_critical_ticks,
            HomeostaticNeedId::Bladder => self.bladder_critical_ticks,
            HomeostaticNeedId::Dirtiness => self.dirtiness_critical_ticks,
        }
    }
}

/// Per-agent physiology parameters that drive metabolism and recovery.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MetabolismProfile {
    /// Hunger increase per tick during normal activity.
    pub hunger_rate: Permille,
    /// Thirst increase per tick during normal activity.
    pub thirst_rate: Permille,
    /// Fatigue increase per tick during normal activity.
    pub fatigue_rate: Permille,
    /// Bladder pressure increase per tick during normal activity.
    pub bladder_rate: Permille,
    /// Dirtiness increase per tick during normal activity.
    pub dirtiness_rate: Permille,
    /// Fatigue decrease per tick while resting or sleeping.
    pub rest_efficiency: Permille,
    /// Ticks at critical hunger before starvation consequences begin.
    pub starvation_tolerance_ticks: NonZeroU32,
    /// Ticks at critical thirst before dehydration consequences begin.
    pub dehydration_tolerance_ticks: NonZeroU32,
    /// Ticks at critical fatigue before the agent collapses.
    pub exhaustion_collapse_ticks: NonZeroU32,
    /// Ticks at critical bladder pressure before an accident occurs.
    pub bladder_accident_tolerance_ticks: NonZeroU32,
    /// Duration in ticks to complete a toilet action.
    pub toilet_ticks: NonZeroU32,
    /// Duration in ticks to complete a washing action.
    pub wash_ticks: NonZeroU32,
    /// Multiplier applied to fatigue rate while traveling.
    pub travel_fatigue_multiplier: Permille,
    /// Multiplier applied to thirst rate while traveling.
    pub travel_thirst_multiplier: Permille,
    /// Multiplier applied to bladder rate while traveling.
    pub travel_bladder_multiplier: Permille,
    /// Additional dirtiness incurred when relieving oneself in the wilderness rather than at a proper facility.
    pub wilderness_relief_dirtiness_penalty: Permille,
}

impl MetabolismProfile {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        hunger_rate: Permille,
        thirst_rate: Permille,
        fatigue_rate: Permille,
        bladder_rate: Permille,
        dirtiness_rate: Permille,
        rest_efficiency: Permille,
        starvation_tolerance_ticks: NonZeroU32,
        dehydration_tolerance_ticks: NonZeroU32,
        exhaustion_collapse_ticks: NonZeroU32,
        bladder_accident_tolerance_ticks: NonZeroU32,
        toilet_ticks: NonZeroU32,
        wash_ticks: NonZeroU32,
        travel_fatigue_multiplier: Permille,
        travel_thirst_multiplier: Permille,
        travel_bladder_multiplier: Permille,
        wilderness_relief_dirtiness_penalty: Permille,
    ) -> Self {
        Self {
            hunger_rate,
            thirst_rate,
            fatigue_rate,
            bladder_rate,
            dirtiness_rate,
            rest_efficiency,
            starvation_tolerance_ticks,
            dehydration_tolerance_ticks,
            exhaustion_collapse_ticks,
            bladder_accident_tolerance_ticks,
            toilet_ticks,
            wash_ticks,
            travel_fatigue_multiplier,
            travel_thirst_multiplier,
            travel_bladder_multiplier,
            wilderness_relief_dirtiness_penalty,
        }
    }
}

impl Component for MetabolismProfile {}

impl Default for MetabolismProfile {
    fn default() -> Self {
        Self {
            hunger_rate: pm(2),
            thirst_rate: pm(3),
            fatigue_rate: pm(2),
            bladder_rate: pm(4),
            dirtiness_rate: pm(1),
            rest_efficiency: pm(20),
            starvation_tolerance_ticks: nz(480),
            dehydration_tolerance_ticks: nz(240),
            exhaustion_collapse_ticks: nz(120),
            bladder_accident_tolerance_ticks: nz(40),
            toilet_ticks: nz(8),
            wash_ticks: nz(12),
            travel_fatigue_multiplier: pm(0),
            travel_thirst_multiplier: pm(0),
            travel_bladder_multiplier: pm(0),
            wilderness_relief_dirtiness_penalty: pm(0),
        }
    }
}

/// Deterministic per-tick physiology cost applied by long-running actions.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BodyCostPerTick {
    pub hunger_delta: Permille,
    pub thirst_delta: Permille,
    pub fatigue_delta: Permille,
    pub bladder_delta: Permille,
    pub dirtiness_delta: Permille,
}

impl BodyCostPerTick {
    #[must_use]
    pub const fn new(
        hunger_delta: Permille,
        thirst_delta: Permille,
        fatigue_delta: Permille,
        bladder_delta: Permille,
        dirtiness_delta: Permille,
    ) -> Self {
        Self {
            hunger_delta,
            thirst_delta,
            fatigue_delta,
            bladder_delta,
            dirtiness_delta,
        }
    }

    #[must_use]
    pub const fn zero() -> Self {
        Self::new(pm(0), pm(0), pm(0), pm(0), pm(0))
    }
}

impl Default for BodyCostPerTick {
    fn default() -> Self {
        Self::zero()
    }
}

const fn pm(value: u16) -> Permille {
    Permille::new_unchecked(value)
}

const fn nz(value: u32) -> NonZeroU32 {
    match NonZeroU32::new(value) {
        Some(value) => value,
        None => panic!("NonZeroU32 value must be greater than zero"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BodyCostPerTick, DeprivationExposure, HomeostaticNeedId, HomeostaticNeeds,
        MetabolismProfile, nz, pm,
    };
    use crate::{Permille, traits::Component};
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;
    use std::num::NonZeroU32;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Copy + Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn homeostatic_needs_new_sated_is_all_zero() {
        let needs = HomeostaticNeeds::new_sated();

        assert_eq!(needs.hunger, Permille::new(0).unwrap());
        assert_eq!(needs.thirst, Permille::new(0).unwrap());
        assert_eq!(needs.fatigue, Permille::new(0).unwrap());
        assert_eq!(needs.bladder, Permille::new(0).unwrap());
        assert_eq!(needs.dirtiness, Permille::new(0).unwrap());
        assert_eq!(needs, HomeostaticNeeds::default());
    }

    #[test]
    fn homeostatic_need_id_variant_count_matches_enum() {
        assert_eq!(HomeostaticNeedId::VARIANT_COUNT, 5);
    }

    #[test]
    fn homeostatic_need_id_all_matches_enum_declaration_order() {
        assert_eq!(
            HomeostaticNeedId::ALL,
            [
                HomeostaticNeedId::Hunger,
                HomeostaticNeedId::Thirst,
                HomeostaticNeedId::Fatigue,
                HomeostaticNeedId::Bladder,
                HomeostaticNeedId::Dirtiness,
            ]
        );
    }

    #[test]
    fn homeostatic_needs_max_value_returns_highest_need() {
        let needs = HomeostaticNeeds::new(pm(10), pm(875), pm(120), pm(640), pm(500));

        assert_eq!(needs.max_value(), 875);
    }

    #[test]
    fn homeostatic_needs_max_value_all_zero() {
        assert_eq!(HomeostaticNeeds::new_sated().max_value(), 0);
    }

    #[test]
    fn deprivation_exposure_default_is_zeroed() {
        assert_eq!(
            DeprivationExposure::default(),
            DeprivationExposure {
                hunger_critical_ticks: 0,
                thirst_critical_ticks: 0,
                fatigue_critical_ticks: 0,
                bladder_critical_ticks: 0,
                dirtiness_critical_ticks: 0,
            }
        );
    }

    #[test]
    fn homeostatic_needs_value_reads_all_variants() {
        let needs = HomeostaticNeeds::new(pm(10), pm(20), pm(30), pm(40), pm(50));

        assert_eq!(needs.value(HomeostaticNeedId::Hunger), pm(10));
        assert_eq!(needs.value(HomeostaticNeedId::Thirst), pm(20));
        assert_eq!(needs.value(HomeostaticNeedId::Fatigue), pm(30));
        assert_eq!(needs.value(HomeostaticNeedId::Bladder), pm(40));
        assert_eq!(needs.value(HomeostaticNeedId::Dirtiness), pm(50));
    }

    #[test]
    fn deprivation_exposure_ticks_at_critical_reads_all_variants() {
        let exposure = DeprivationExposure {
            hunger_critical_ticks: 1,
            thirst_critical_ticks: 2,
            fatigue_critical_ticks: 3,
            bladder_critical_ticks: 4,
            dirtiness_critical_ticks: 5,
        };

        assert_eq!(exposure.ticks_at_critical(HomeostaticNeedId::Hunger), 1);
        assert_eq!(exposure.ticks_at_critical(HomeostaticNeedId::Thirst), 2);
        assert_eq!(exposure.ticks_at_critical(HomeostaticNeedId::Fatigue), 3);
        assert_eq!(exposure.ticks_at_critical(HomeostaticNeedId::Bladder), 4);
        assert_eq!(exposure.ticks_at_critical(HomeostaticNeedId::Dirtiness), 5);
    }

    #[test]
    fn metabolism_profile_new_stores_every_field() {
        let profile = MetabolismProfile::new(
            pm(5),
            pm(6),
            pm(7),
            pm(8),
            pm(9),
            pm(25),
            nz(100),
            nz(110),
            nz(120),
            nz(130),
            nz(14),
            nz(16),
            pm(200),
            pm(300),
            pm(400),
            pm(100),
        );

        assert_eq!(profile.hunger_rate, pm(5));
        assert_eq!(profile.thirst_rate, pm(6));
        assert_eq!(profile.fatigue_rate, pm(7));
        assert_eq!(profile.bladder_rate, pm(8));
        assert_eq!(profile.dirtiness_rate, pm(9));
        assert_eq!(profile.rest_efficiency, pm(25));
        assert_eq!(profile.starvation_tolerance_ticks, nz(100));
        assert_eq!(profile.dehydration_tolerance_ticks, nz(110));
        assert_eq!(profile.exhaustion_collapse_ticks, nz(120));
        assert_eq!(profile.bladder_accident_tolerance_ticks, nz(130));
        assert_eq!(profile.toilet_ticks, nz(14));
        assert_eq!(profile.wash_ticks, nz(16));
        assert_eq!(profile.travel_fatigue_multiplier, pm(200));
        assert_eq!(profile.travel_thirst_multiplier, pm(300));
        assert_eq!(profile.travel_bladder_multiplier, pm(400));
        assert_eq!(profile.wilderness_relief_dirtiness_penalty, pm(100));
    }

    #[test]
    fn metabolism_profile_default_travel_multipliers_zero() {
        let profile = MetabolismProfile::default();

        assert_eq!(profile.travel_fatigue_multiplier, pm(0));
        assert_eq!(profile.travel_thirst_multiplier, pm(0));
        assert_eq!(profile.travel_bladder_multiplier, pm(0));
        assert_eq!(profile.wilderness_relief_dirtiness_penalty, pm(0));
    }

    #[test]
    fn metabolism_profile_default_uses_non_zero_durations() {
        let profile = MetabolismProfile::default();

        assert!(profile.starvation_tolerance_ticks.get() > 0);
        assert!(profile.dehydration_tolerance_ticks.get() > 0);
        assert!(profile.exhaustion_collapse_ticks.get() > 0);
        assert!(profile.bladder_accident_tolerance_ticks.get() > 0);
        assert!(profile.toilet_ticks.get() > 0);
        assert!(profile.wash_ticks.get() > 0);
    }

    #[test]
    fn non_zero_u32_rejects_zero_for_profile_fields() {
        assert_eq!(NonZeroU32::new(0), None);
    }

    #[test]
    fn physiology_components_satisfy_required_bounds() {
        assert_component_bounds::<HomeostaticNeeds>();
        assert_component_bounds::<DeprivationExposure>();
        assert_component_bounds::<MetabolismProfile>();
        assert_value_bounds::<BodyCostPerTick>();
        assert_value_bounds::<HomeostaticNeeds>();
        assert_value_bounds::<DeprivationExposure>();
        assert_value_bounds::<MetabolismProfile>();
    }

    #[test]
    fn body_cost_per_tick_zero_is_all_zero() {
        let cost = BodyCostPerTick::zero();

        assert_eq!(cost.hunger_delta, Permille::new(0).unwrap());
        assert_eq!(cost.thirst_delta, Permille::new(0).unwrap());
        assert_eq!(cost.fatigue_delta, Permille::new(0).unwrap());
        assert_eq!(cost.bladder_delta, Permille::new(0).unwrap());
        assert_eq!(cost.dirtiness_delta, Permille::new(0).unwrap());
        assert_eq!(cost, BodyCostPerTick::default());
    }

    #[test]
    fn body_cost_per_tick_new_stores_every_field() {
        let cost = BodyCostPerTick::new(pm(3), pm(5), pm(8), pm(7), pm(2));

        assert_eq!(cost.hunger_delta, pm(3));
        assert_eq!(cost.thirst_delta, pm(5));
        assert_eq!(cost.fatigue_delta, pm(8));
        assert_eq!(cost.bladder_delta, pm(7));
        assert_eq!(cost.dirtiness_delta, pm(2));
    }

    #[test]
    fn physiology_types_roundtrip_through_bincode() {
        let needs = HomeostaticNeeds::new(pm(10), pm(20), pm(30), pm(40), pm(50));
        let exposure = DeprivationExposure {
            hunger_critical_ticks: 1,
            thirst_critical_ticks: 2,
            fatigue_critical_ticks: 3,
            bladder_critical_ticks: 4,
            dirtiness_critical_ticks: 5,
        };
        let profile = MetabolismProfile::default();
        let cost = BodyCostPerTick::new(pm(4), pm(6), pm(9), pm(7), pm(3));

        let needs_bytes = bincode::serialize(&needs).unwrap();
        let exposure_bytes = bincode::serialize(&exposure).unwrap();
        let profile_bytes = bincode::serialize(&profile).unwrap();
        let cost_bytes = bincode::serialize(&cost).unwrap();

        let roundtrip_needs: HomeostaticNeeds = bincode::deserialize(&needs_bytes).unwrap();
        let roundtrip_exposure: DeprivationExposure =
            bincode::deserialize(&exposure_bytes).unwrap();
        let roundtrip_profile: MetabolismProfile = bincode::deserialize(&profile_bytes).unwrap();
        let roundtrip_cost: BodyCostPerTick = bincode::deserialize(&cost_bytes).unwrap();

        assert_eq!(roundtrip_needs, needs);
        assert_eq!(roundtrip_exposure, exposure);
        assert_eq!(roundtrip_profile, profile);
        assert_eq!(roundtrip_cost, cost);
    }
}
