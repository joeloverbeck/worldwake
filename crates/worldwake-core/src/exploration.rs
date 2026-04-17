use crate::{Component, HomeostaticNeedId, Permille};
use serde::{Deserialize, Serialize};

/// Stable per-agent parameters governing need-driven exploration pressure.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ExplorationProfile {
    /// Base motive weight for exploration-driven goals.
    pub curiosity_weight: Permille,
    /// Need pressure level that must be exceeded before exploration is considered.
    pub need_activation_threshold: Permille,
    /// Maximum graph distance (in hops) the agent considers as frontier for exploration.
    pub frontier_depth: u16,
    /// Number of consecutive acquisition failures before exploration is triggered.
    pub acquisition_failure_threshold: u8,
    /// Utility bonus applied upon arriving at a newly explored location.
    pub exploration_arrival_boost: Permille,
    /// Maximum consecutive exploration actions before the agent must address other goals.
    pub max_consecutive_explorations: u8,
    /// How far back in ticks the agent looks when evaluating recently visited places.
    pub visit_lookback_ticks: u32,
    /// Runtime counter tracking consecutive explorations (not a tuning parameter).
    pub consecutive_exploration_count: u8,
}

impl Default for ExplorationProfile {
    fn default() -> Self {
        Self {
            curiosity_weight: Permille::new_unchecked(500),
            need_activation_threshold: Permille::new_unchecked(400),
            frontier_depth: 2,
            acquisition_failure_threshold: 3,
            exploration_arrival_boost: Permille::new_unchecked(500),
            max_consecutive_explorations: 3,
            visit_lookback_ticks: 200,
            consecutive_exploration_count: 0,
        }
    }
}

impl Component for ExplorationProfile {}

/// Tracks per-need budget exhaustion counts for exploration fallback gating.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct AcquisitionExhaustionTracker {
    counts: [u8; HomeostaticNeedId::VARIANT_COUNT],
}

impl AcquisitionExhaustionTracker {
    #[must_use]
    pub fn count(&self, need: HomeostaticNeedId) -> u8 {
        self.counts[need as usize]
    }

    pub fn increment(&mut self, need: HomeostaticNeedId) {
        let idx = need as usize;
        self.counts[idx] = self.counts[idx].saturating_add(1);
    }

    pub fn reset(&mut self, need: HomeostaticNeedId) {
        self.counts[need as usize] = 0;
    }
}

impl Component for AcquisitionExhaustionTracker {}

#[cfg(test)]
mod tests {
    use super::{AcquisitionExhaustionTracker, ExplorationProfile};
    use crate::{
        ControlSource, EntityKind, HomeostaticNeedId, Tick, Topology, World, traits::Component,
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn exploration_profile_component_bounds() {
        assert_component_bounds::<ExplorationProfile>();
        assert_value_bounds::<ExplorationProfile>();
    }

    #[test]
    fn acquisition_exhaustion_tracker_component_bounds() {
        assert_component_bounds::<AcquisitionExhaustionTracker>();
        assert_value_bounds::<AcquisitionExhaustionTracker>();
    }

    #[test]
    fn exploration_profile_default_matches_spec_defaults() {
        let profile = ExplorationProfile::default();

        assert_eq!(profile.curiosity_weight, crate::Permille::new(500).unwrap());
        assert_eq!(
            profile.need_activation_threshold,
            crate::Permille::new(400).unwrap()
        );
        assert_eq!(profile.frontier_depth, 2);
        assert_eq!(profile.acquisition_failure_threshold, 3);
        assert_eq!(
            profile.exploration_arrival_boost,
            crate::Permille::new(500).unwrap()
        );
        assert_eq!(profile.max_consecutive_explorations, 3);
        assert_eq!(profile.visit_lookback_ticks, 200);
        assert_eq!(profile.consecutive_exploration_count, 0);
    }

    #[test]
    fn exploration_profile_roundtrips_through_bincode() {
        let profile = ExplorationProfile {
            curiosity_weight: crate::Permille::new(650).unwrap(),
            need_activation_threshold: crate::Permille::new(550).unwrap(),
            frontier_depth: 4,
            acquisition_failure_threshold: 5,
            exploration_arrival_boost: crate::Permille::new(625).unwrap(),
            max_consecutive_explorations: 4,
            visit_lookback_ticks: 320,
            consecutive_exploration_count: 2,
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: ExplorationProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn exploration_profile_registers_for_agents() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world
            .create_agent("Scout", ControlSource::Ai, Tick(1))
            .unwrap();
        let profile = ExplorationProfile {
            max_consecutive_explorations: 5,
            ..ExplorationProfile::default()
        };

        assert_eq!(
            world.get_component_exploration_profile(agent),
            Some(&ExplorationProfile::default())
        );
        assert_eq!(
            world.remove_component_exploration_profile(agent).unwrap(),
            Some(ExplorationProfile::default())
        );

        world
            .insert_component_exploration_profile(agent, profile)
            .unwrap();

        assert_eq!(
            world.get_component_exploration_profile(agent),
            Some(&profile)
        );
        assert_eq!(
            world
                .entities_with_exploration_profile()
                .collect::<Vec<_>>(),
            vec![agent]
        );
        assert_eq!(
            world.query_exploration_profile().collect::<Vec<_>>(),
            vec![(agent, &profile)]
        );
        assert_eq!(world.count_with_exploration_profile(), 1);
        assert_eq!(world.entity_kind(agent), Some(EntityKind::Agent));
    }

    #[test]
    fn acquisition_exhaustion_tracker_default_is_zeroed() {
        let tracker = AcquisitionExhaustionTracker::default();

        assert_eq!(tracker.count(HomeostaticNeedId::Hunger), 0);
        assert_eq!(tracker.count(HomeostaticNeedId::Thirst), 0);
        assert_eq!(tracker.count(HomeostaticNeedId::Fatigue), 0);
        assert_eq!(tracker.count(HomeostaticNeedId::Bladder), 0);
        assert_eq!(tracker.count(HomeostaticNeedId::Dirtiness), 0);
    }

    #[test]
    fn acquisition_exhaustion_tracker_increment_and_reset_are_need_specific() {
        let mut tracker = AcquisitionExhaustionTracker::default();

        tracker.increment(HomeostaticNeedId::Hunger);
        tracker.increment(HomeostaticNeedId::Hunger);
        tracker.increment(HomeostaticNeedId::Thirst);

        assert_eq!(tracker.count(HomeostaticNeedId::Hunger), 2);
        assert_eq!(tracker.count(HomeostaticNeedId::Thirst), 1);
        assert_eq!(tracker.count(HomeostaticNeedId::Dirtiness), 0);

        tracker.reset(HomeostaticNeedId::Hunger);

        assert_eq!(tracker.count(HomeostaticNeedId::Hunger), 0);
        assert_eq!(tracker.count(HomeostaticNeedId::Thirst), 1);
    }

    #[test]
    fn acquisition_exhaustion_tracker_saturates_at_u8_max() {
        let mut tracker = AcquisitionExhaustionTracker::default();

        for _ in 0..=u8::MAX {
            tracker.increment(HomeostaticNeedId::Hunger);
        }

        assert_eq!(tracker.count(HomeostaticNeedId::Hunger), u8::MAX);
    }

    #[test]
    fn acquisition_exhaustion_tracker_registers_for_agents() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world
            .create_agent("Scout", ControlSource::Ai, Tick(1))
            .unwrap();
        let mut tracker = AcquisitionExhaustionTracker::default();
        tracker.increment(HomeostaticNeedId::Fatigue);

        assert_eq!(
            world.get_component_acquisition_exhaustion_tracker(agent),
            Some(&AcquisitionExhaustionTracker::default())
        );
        assert_eq!(
            world
                .remove_component_acquisition_exhaustion_tracker(agent)
                .unwrap(),
            Some(AcquisitionExhaustionTracker::default())
        );

        world
            .insert_component_acquisition_exhaustion_tracker(agent, tracker)
            .unwrap();

        assert_eq!(
            world.get_component_acquisition_exhaustion_tracker(agent),
            Some(&tracker)
        );
        assert_eq!(
            world
                .entities_with_acquisition_exhaustion_tracker()
                .collect::<Vec<_>>(),
            vec![agent]
        );
        assert_eq!(
            world
                .query_acquisition_exhaustion_tracker()
                .collect::<Vec<_>>(),
            vec![(agent, &tracker)]
        );
        assert_eq!(world.count_with_acquisition_exhaustion_tracker(), 1);
        assert_eq!(world.entity_kind(agent), Some(EntityKind::Agent));
    }
}
