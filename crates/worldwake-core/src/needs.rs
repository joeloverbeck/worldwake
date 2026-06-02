//! Concrete physiology state and per-agent metabolism parameters.

use crate::{Component, Permille, Tick};
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

    /// Projected tick at which `need` reaches `target_level` given the
    /// agent's `base_rate` for that need, starting from `current_tick`.
    /// Returns `None` if the need would never reach the target (rate is
    /// zero with the current level still below target).
    #[must_use]
    pub fn projected_tick_of(
        &self,
        need: HomeostaticNeedId,
        target_level: Permille,
        base_rate: Permille,
        current_tick: Tick,
    ) -> Option<Tick> {
        let current = self.value(need).value();
        let target = target_level.value();
        if current >= target {
            return Some(current_tick);
        }
        let rate = base_rate.value();
        if rate == 0 {
            return None;
        }
        let delta_ticks = u64::from(target - current).div_ceil(u64::from(rate));
        Some(Tick(current_tick.0.saturating_add(delta_ticks)))
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
    /// Duration in ticks to complete a wash-basin cleaning action.
    #[serde(default = "default_clean_basin_duration_ticks")]
    pub clean_basin_duration_ticks: NonZeroU32,
    /// Duration in ticks to complete a latrine-emptying action.
    #[serde(default = "default_empty_latrine_duration_ticks")]
    pub empty_latrine_duration_ticks: NonZeroU32,
    /// Minimum duration in ticks for a sleep episode.
    #[serde(default = "default_min_sleep_ticks")]
    pub min_sleep_ticks: NonZeroU32,
    /// Hard ceiling on per-tick fatigue recovery when sleeping rough without a known rest site.
    #[serde(default = "default_rough_sleep_recovery_floor")]
    pub rough_sleep_recovery_floor: Permille,
    /// Multiplier applied to fatigue rate while traveling.
    pub travel_fatigue_multiplier: Permille,
    /// Multiplier applied to thirst rate while traveling.
    pub travel_thirst_multiplier: Permille,
    /// Multiplier applied to bladder rate while traveling.
    pub travel_bladder_multiplier: Permille,
    /// Additional dirtiness incurred when relieving oneself in the wilderness rather than at a proper facility.
    pub wilderness_relief_dirtiness_penalty: Permille,
    /// Minimum acceptable wash effectiveness (the effective relief fraction in
    /// permille) before the planner inserts proactive `clean_wash_basin`
    /// maintenance ahead of a wash. A basin's live effectiveness is
    /// `(max_effective_dirtiness - dirtiness_level) / max_effective_dirtiness`.
    /// When that falls below this floor — while the basin is still below its
    /// hard `max_effective_dirtiness` block — the agent cleans the basin first
    /// so the recovered wash delivers worthwhile relief. This is the FND-11
    /// maintenance-labor dampener engaging before the basin becomes unusable.
    /// `0` disables proactive cleaning, leaving only the hard authoritative
    /// block (`TargetWashBasinNotTooDirty`).
    #[serde(default = "default_wash_worthwhile_effectiveness_floor")]
    pub wash_worthwhile_effectiveness_floor: Permille,
    /// Hunger threshold at or above which spoiled food becomes a candidate
    /// desperation affordance.
    #[serde(default = "default_spoiled_food_hunger_threshold")]
    pub spoiled_food_hunger_threshold: Permille,
}

impl MetabolismProfile {
    /// Per-need base depletion rate per tick.
    #[must_use]
    pub const fn rate(&self, need: HomeostaticNeedId) -> Permille {
        match need {
            HomeostaticNeedId::Hunger => self.hunger_rate,
            HomeostaticNeedId::Thirst => self.thirst_rate,
            HomeostaticNeedId::Fatigue => self.fatigue_rate,
            HomeostaticNeedId::Bladder => self.bladder_rate,
            HomeostaticNeedId::Dirtiness => self.dirtiness_rate,
        }
    }

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
        min_sleep_ticks: NonZeroU32,
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
            clean_basin_duration_ticks: default_clean_basin_duration_ticks(),
            empty_latrine_duration_ticks: default_empty_latrine_duration_ticks(),
            min_sleep_ticks,
            rough_sleep_recovery_floor: default_rough_sleep_recovery_floor(),
            travel_fatigue_multiplier,
            travel_thirst_multiplier,
            travel_bladder_multiplier,
            wilderness_relief_dirtiness_penalty,
            wash_worthwhile_effectiveness_floor: default_wash_worthwhile_effectiveness_floor(),
            spoiled_food_hunger_threshold: default_spoiled_food_hunger_threshold(),
        }
    }
}

impl Component for MetabolismProfile {}

fn default_min_sleep_ticks() -> NonZeroU32 {
    nz(8)
}

const fn default_clean_basin_duration_ticks() -> NonZeroU32 {
    nz(10)
}

const fn default_empty_latrine_duration_ticks() -> NonZeroU32 {
    nz(16)
}

const fn default_wash_worthwhile_effectiveness_floor() -> Permille {
    pm(500)
}

const fn default_spoiled_food_hunger_threshold() -> Permille {
    pm(800)
}

const fn default_rough_sleep_recovery_floor() -> Permille {
    pm(300)
}

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
            clean_basin_duration_ticks: default_clean_basin_duration_ticks(),
            empty_latrine_duration_ticks: default_empty_latrine_duration_ticks(),
            min_sleep_ticks: default_min_sleep_ticks(),
            rough_sleep_recovery_floor: default_rough_sleep_recovery_floor(),
            travel_fatigue_multiplier: pm(0),
            travel_thirst_multiplier: pm(0),
            travel_bladder_multiplier: pm(0),
            wilderness_relief_dirtiness_penalty: pm(0),
            wash_worthwhile_effectiveness_floor: default_wash_worthwhile_effectiveness_floor(),
            spoiled_food_hunger_threshold: default_spoiled_food_hunger_threshold(),
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
    use crate::{Permille, Tick, traits::Component};
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
            nz(18),
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
        // Cleaning durations follow the `rough_sleep_recovery_floor` precedent:
        // not in the `new()` signature, so `new()` seeds them from defaults.
        assert_eq!(profile.clean_basin_duration_ticks, nz(10));
        assert_eq!(profile.empty_latrine_duration_ticks, nz(16));
        assert_eq!(profile.min_sleep_ticks, nz(18));
        assert_eq!(profile.rough_sleep_recovery_floor, pm(300));
        assert_eq!(profile.travel_fatigue_multiplier, pm(200));
        assert_eq!(profile.travel_thirst_multiplier, pm(300));
        assert_eq!(profile.travel_bladder_multiplier, pm(400));
        assert_eq!(profile.wilderness_relief_dirtiness_penalty, pm(100));
        // Worthwhile-wash floor follows the `rough_sleep_recovery_floor`
        // precedent: not in the `new()` signature, so `new()` seeds it from the
        // default.
        assert_eq!(profile.wash_worthwhile_effectiveness_floor, pm(500));
        assert_eq!(profile.spoiled_food_hunger_threshold, pm(800));
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
        assert_eq!(profile.clean_basin_duration_ticks, nz(10));
        assert_eq!(profile.empty_latrine_duration_ticks, nz(16));
        assert_eq!(profile.min_sleep_ticks, nz(8));
        assert_eq!(profile.rough_sleep_recovery_floor, pm(300));
        assert_eq!(profile.wash_worthwhile_effectiveness_floor, pm(500));
        assert_eq!(profile.spoiled_food_hunger_threshold, pm(800));
    }

    #[test]
    fn metabolism_profile_default_includes_spoiled_food_hunger_threshold() {
        let profile = MetabolismProfile::default();

        assert_eq!(profile.spoiled_food_hunger_threshold, pm(800));
    }

    #[test]
    fn metabolism_profile_ron_omits_defaulted_sleep_fields() {
        let ron = r"(
            hunger_rate: 2,
            thirst_rate: 3,
            fatigue_rate: 4,
            bladder_rate: 5,
            dirtiness_rate: 6,
            rest_efficiency: 20,
            starvation_tolerance_ticks: 480,
            dehydration_tolerance_ticks: 240,
            exhaustion_collapse_ticks: 120,
            bladder_accident_tolerance_ticks: 40,
            toilet_ticks: 8,
            wash_ticks: 12,
            travel_fatigue_multiplier: 0,
            travel_thirst_multiplier: 0,
            travel_bladder_multiplier: 0,
            wilderness_relief_dirtiness_penalty: 0,
        )";

        let profile: MetabolismProfile = ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::UNWRAP_NEWTYPES)
            .from_str(ron)
            .unwrap();

        assert_eq!(profile.min_sleep_ticks, nz(8));
        assert_eq!(profile.rough_sleep_recovery_floor, pm(300));
        // The 24 inline `metabolism_profile` scenarios omit the cleaning
        // durations; serde defaults must hold so they deserialize unchanged.
        assert_eq!(profile.clean_basin_duration_ticks, nz(10));
        assert_eq!(profile.empty_latrine_duration_ticks, nz(16));
        // Scenarios that omit the worthwhile-wash floor inherit the proactive
        // cleaning default so the FND-11 dampener engages without opt-in.
        assert_eq!(profile.wash_worthwhile_effectiveness_floor, pm(500));
        assert_eq!(profile.spoiled_food_hunger_threshold, pm(800));
    }

    #[test]
    fn metabolism_profile_serde_default_absorbs_missing_field() {
        let ron = r"(
            hunger_rate: 2,
            thirst_rate: 3,
            fatigue_rate: 4,
            bladder_rate: 5,
            dirtiness_rate: 6,
            rest_efficiency: 20,
            starvation_tolerance_ticks: 480,
            dehydration_tolerance_ticks: 240,
            exhaustion_collapse_ticks: 120,
            bladder_accident_tolerance_ticks: 40,
            toilet_ticks: 8,
            wash_ticks: 12,
            travel_fatigue_multiplier: 0,
            travel_thirst_multiplier: 0,
            travel_bladder_multiplier: 0,
            wilderness_relief_dirtiness_penalty: 0,
        )";

        let profile: MetabolismProfile = ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::UNWRAP_NEWTYPES)
            .from_str(ron)
            .unwrap();

        assert_eq!(profile.spoiled_food_hunger_threshold, pm(800));
    }

    #[test]
    fn metabolism_profile_ron_accepts_explicit_wash_worthwhile_floor() {
        let ron = r"(
            hunger_rate: 2,
            thirst_rate: 3,
            fatigue_rate: 4,
            bladder_rate: 5,
            dirtiness_rate: 6,
            rest_efficiency: 20,
            starvation_tolerance_ticks: 480,
            dehydration_tolerance_ticks: 240,
            exhaustion_collapse_ticks: 120,
            bladder_accident_tolerance_ticks: 40,
            toilet_ticks: 8,
            wash_ticks: 12,
            wash_worthwhile_effectiveness_floor: 0,
            travel_fatigue_multiplier: 0,
            travel_thirst_multiplier: 0,
            travel_bladder_multiplier: 0,
            wilderness_relief_dirtiness_penalty: 0,
        )";

        let profile: MetabolismProfile = ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::UNWRAP_NEWTYPES)
            .from_str(ron)
            .unwrap();

        assert_eq!(profile.wash_worthwhile_effectiveness_floor, pm(0));
    }

    #[test]
    fn metabolism_profile_ron_accepts_explicit_cleaning_durations() {
        let ron = r"(
            hunger_rate: 2,
            thirst_rate: 3,
            fatigue_rate: 4,
            bladder_rate: 5,
            dirtiness_rate: 6,
            rest_efficiency: 20,
            starvation_tolerance_ticks: 480,
            dehydration_tolerance_ticks: 240,
            exhaustion_collapse_ticks: 120,
            bladder_accident_tolerance_ticks: 40,
            toilet_ticks: 8,
            wash_ticks: 12,
            clean_basin_duration_ticks: 25,
            empty_latrine_duration_ticks: 30,
            travel_fatigue_multiplier: 0,
            travel_thirst_multiplier: 0,
            travel_bladder_multiplier: 0,
            wilderness_relief_dirtiness_penalty: 0,
        )";

        let profile: MetabolismProfile = ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::UNWRAP_NEWTYPES)
            .from_str(ron)
            .unwrap();

        assert_eq!(profile.clean_basin_duration_ticks, nz(25));
        assert_eq!(profile.empty_latrine_duration_ticks, nz(30));
    }

    #[test]
    fn metabolism_profile_ron_accepts_explicit_rough_sleep_recovery_floor() {
        let ron = r"(
            hunger_rate: 2,
            thirst_rate: 3,
            fatigue_rate: 4,
            bladder_rate: 5,
            dirtiness_rate: 6,
            rest_efficiency: 20,
            starvation_tolerance_ticks: 480,
            dehydration_tolerance_ticks: 240,
            exhaustion_collapse_ticks: 120,
            bladder_accident_tolerance_ticks: 40,
            toilet_ticks: 8,
            wash_ticks: 12,
            rough_sleep_recovery_floor: 500,
            travel_fatigue_multiplier: 0,
            travel_thirst_multiplier: 0,
            travel_bladder_multiplier: 0,
            wilderness_relief_dirtiness_penalty: 0,
        )";

        let profile: MetabolismProfile = ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::UNWRAP_NEWTYPES)
            .from_str(ron)
            .unwrap();

        assert_eq!(profile.rough_sleep_recovery_floor, pm(500));
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
    fn projected_tick_of_returns_current_tick_when_already_at_or_above_target() {
        let needs = HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0));

        let projected =
            needs.projected_tick_of(HomeostaticNeedId::Hunger, pm(700), pm(50), Tick(10));

        assert_eq!(projected, Some(Tick(10)));
    }

    #[test]
    fn projected_tick_of_returns_none_when_rate_is_zero_below_target() {
        let needs = HomeostaticNeeds::new(pm(400), pm(0), pm(0), pm(0), pm(0));

        let projected =
            needs.projected_tick_of(HomeostaticNeedId::Hunger, pm(700), pm(0), Tick(10));

        assert_eq!(projected, None);
    }

    #[test]
    fn projected_tick_of_returns_current_plus_div_ceil_of_gap_over_rate() {
        let needs = HomeostaticNeeds::new(pm(400), pm(0), pm(0), pm(0), pm(0));

        let projected =
            needs.projected_tick_of(HomeostaticNeedId::Hunger, pm(700), pm(50), Tick(10));

        // gap = 300, rate = 50, ceil(300/50) = 6, current_tick + 6 = 16
        assert_eq!(projected, Some(Tick(16)));
    }

    #[test]
    fn projected_tick_of_uses_div_ceil_for_partial_remainder() {
        let needs = HomeostaticNeeds::new(pm(400), pm(0), pm(0), pm(0), pm(0));

        let projected =
            needs.projected_tick_of(HomeostaticNeedId::Hunger, pm(700), pm(40), Tick(0));

        // gap = 300, rate = 40, ceil(300/40) = 8 (300/40 = 7.5)
        assert_eq!(projected, Some(Tick(8)));
    }

    #[test]
    fn metabolism_profile_rate_reads_all_variants() {
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
            nz(18),
            pm(200),
            pm(300),
            pm(400),
            pm(100),
        );

        for need in HomeostaticNeedId::ALL {
            let expected = match need {
                HomeostaticNeedId::Hunger => profile.hunger_rate,
                HomeostaticNeedId::Thirst => profile.thirst_rate,
                HomeostaticNeedId::Fatigue => profile.fatigue_rate,
                HomeostaticNeedId::Bladder => profile.bladder_rate,
                HomeostaticNeedId::Dirtiness => profile.dirtiness_rate,
            };
            assert_eq!(profile.rate(need), expected);
        }
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
