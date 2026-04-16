//! Observation fidelity inputs and place-level concealment profiles.

use crate::{Component, Permille};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservationContext {
    pub base_fidelity: Permille,
    pub fatigue_penalty: Permille,
    pub occupancy_penalty: Permille,
    pub place_concealment: Permille,
    pub entity_concealment: Permille,
}

impl ObservationContext {
    #[must_use]
    pub fn effective_fidelity(&self) -> Permille {
        let mut fidelity = u32::from(self.base_fidelity.value());
        fidelity = fidelity * (1000 - u32::from(self.fatigue_penalty.value())) / 1000;
        fidelity = fidelity * (1000 - u32::from(self.occupancy_penalty.value())) / 1000;

        let concealment = u32::from(self.place_concealment.value())
            .max(u32::from(self.entity_concealment.value()));
        fidelity = fidelity * (1000 - concealment) / 1000;

        Permille::new_unchecked(fidelity.min(1000) as u16)
    }
}

/// Environmental visibility parameters attached to place entities.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlaceVisibilityProfile {
    /// Baseline concealment modifier for entities at this place; higher values make observation harder.
    pub base_concealment: Permille,
}

impl Component for PlaceVisibilityProfile {}

#[cfg(test)]
mod tests {
    use super::{ObservationContext, PlaceVisibilityProfile};
    use crate::{Permille, traits::Component};
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn permille(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn observation_context_all_zero_penalties_preserve_base_fidelity() {
        let context = ObservationContext {
            base_fidelity: permille(725),
            fatigue_penalty: Permille::ZERO,
            occupancy_penalty: Permille::ZERO,
            place_concealment: Permille::ZERO,
            entity_concealment: Permille::ZERO,
        };

        assert_eq!(context.effective_fidelity(), permille(725));
    }

    #[test]
    fn observation_context_applies_single_fatigue_penalty() {
        let context = ObservationContext {
            base_fidelity: permille(1000),
            fatigue_penalty: permille(300),
            occupancy_penalty: Permille::ZERO,
            place_concealment: Permille::ZERO,
            entity_concealment: Permille::ZERO,
        };

        assert_eq!(context.effective_fidelity(), permille(700));
    }

    #[test]
    fn observation_context_all_max_penalties_reduce_to_zero() {
        let context = ObservationContext {
            base_fidelity: permille(1000),
            fatigue_penalty: permille(1000),
            occupancy_penalty: permille(1000),
            place_concealment: permille(1000),
            entity_concealment: permille(1000),
        };

        assert_eq!(context.effective_fidelity(), Permille::ZERO);
    }

    #[test]
    fn observation_context_zero_base_stays_zero() {
        let context = ObservationContext {
            base_fidelity: Permille::ZERO,
            fatigue_penalty: permille(120),
            occupancy_penalty: permille(400),
            place_concealment: permille(250),
            entity_concealment: permille(900),
        };

        assert_eq!(context.effective_fidelity(), Permille::ZERO);
    }

    #[test]
    fn observation_context_stacks_penalties_multiplicatively() {
        let context = ObservationContext {
            base_fidelity: permille(800),
            fatigue_penalty: permille(120),
            occupancy_penalty: permille(400),
            place_concealment: permille(400),
            entity_concealment: permille(250),
        };

        assert_eq!(context.effective_fidelity(), permille(253));
    }

    #[test]
    fn observation_context_uses_higher_concealment_source() {
        let context = ObservationContext {
            base_fidelity: permille(900),
            fatigue_penalty: Permille::ZERO,
            occupancy_penalty: Permille::ZERO,
            place_concealment: permille(200),
            entity_concealment: permille(450),
        };

        assert_eq!(context.effective_fidelity(), permille(495));
    }

    #[test]
    fn place_visibility_profile_component_bounds_and_roundtrip() {
        let profile = PlaceVisibilityProfile {
            base_concealment: permille(275),
        };

        assert_component_bounds::<PlaceVisibilityProfile>();
        assert_value_bounds::<ObservationContext>();
        assert_value_bounds::<PlaceVisibilityProfile>();

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: PlaceVisibilityProfile = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, profile);
    }
}
