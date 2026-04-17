//! Authoritative obligation satiation state for repeated obligation goals.

use crate::{Component, Permille, Tick};
use serde::{Deserialize, Serialize};

/// Per-agent parameters controlling how obligation-class goals decay after
/// repeated execution within a recent window.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObligationSatiationProfile {
    /// Number of obligation executions within the lookback window before satiation dampening begins.
    pub satiation_threshold: u32,
    /// Lookback window in ticks over which recent obligation executions are counted.
    pub window_ticks: u32,
    /// Motive weight decay applied per execution above the satiation threshold.
    pub decay_per_execution: Permille,
    /// Minimum motive weight that satiation decay cannot reduce below.
    pub satiation_floor: Permille,
}

impl Default for ObligationSatiationProfile {
    fn default() -> Self {
        Self {
            satiation_threshold: 2,
            window_ticks: 48,
            decay_per_execution: Permille::new_unchecked(200),
            satiation_floor: Permille::new_unchecked(50),
        }
    }
}

impl Component for ObligationSatiationProfile {}

/// Runtime-generated record of recent obligation completions.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObligationExecutionTracker {
    pub completion_ticks: Vec<Tick>,
}

impl Component for ObligationExecutionTracker {}

#[cfg(test)]
mod tests {
    use super::{ObligationExecutionTracker, ObligationSatiationProfile};
    use crate::{ControlSource, EntityKind, Tick, Topology, World, traits::Component};
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn obligation_components_satisfy_required_traits() {
        assert_component_bounds::<ObligationSatiationProfile>();
        assert_component_bounds::<ObligationExecutionTracker>();
        assert_value_bounds::<ObligationSatiationProfile>();
        assert_value_bounds::<ObligationExecutionTracker>();
    }

    #[test]
    fn obligation_satiation_profile_default_matches_spec_defaults() {
        let profile = ObligationSatiationProfile::default();

        assert_eq!(profile.satiation_threshold, 2);
        assert_eq!(profile.window_ticks, 48);
        assert_eq!(
            profile.decay_per_execution,
            crate::Permille::new(200).unwrap()
        );
        assert_eq!(profile.satiation_floor, crate::Permille::new(50).unwrap());
    }

    #[test]
    fn obligation_execution_tracker_default_starts_empty() {
        let tracker = ObligationExecutionTracker::default();

        assert!(tracker.completion_ticks.is_empty());
    }

    #[test]
    fn obligation_satiation_profile_registers_for_agents() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world
            .create_agent("Herald", ControlSource::Ai, Tick(1))
            .unwrap();
        let profile = ObligationSatiationProfile {
            satiation_threshold: 4,
            ..ObligationSatiationProfile::default()
        };

        assert_eq!(
            world.get_component_obligation_satiation_profile(agent),
            Some(&ObligationSatiationProfile::default())
        );
        assert_eq!(
            world
                .remove_component_obligation_satiation_profile(agent)
                .unwrap(),
            Some(ObligationSatiationProfile::default())
        );

        world
            .insert_component_obligation_satiation_profile(agent, profile.clone())
            .unwrap();

        assert_eq!(
            world.get_component_obligation_satiation_profile(agent),
            Some(&profile)
        );
        assert_eq!(
            world
                .entities_with_obligation_satiation_profile()
                .collect::<Vec<_>>(),
            vec![agent]
        );
        assert_eq!(
            world
                .query_obligation_satiation_profile()
                .collect::<Vec<_>>(),
            vec![(agent, &profile)]
        );
        assert_eq!(world.count_with_obligation_satiation_profile(), 1);
        assert_eq!(world.entity_kind(agent), Some(EntityKind::Agent));
    }
}
