//! Place and facility hygiene state.

use crate::{Component, Permille};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlaceDirtiness {
    pub value: Permille,
    pub decay_per_tick: Permille,
    pub dirtiness_per_use: Permille,
}

impl Default for PlaceDirtiness {
    fn default() -> Self {
        Self {
            value: Permille::ZERO,
            decay_per_tick: Permille::new_unchecked(2),
            dirtiness_per_use: Permille::new_unchecked(80),
        }
    }
}

impl Component for PlaceDirtiness {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LatrineFullness {
    pub fill: Permille,
    pub fill_per_use: Permille,
    pub critical_threshold: Permille,
}

impl Default for LatrineFullness {
    fn default() -> Self {
        Self {
            fill: Permille::ZERO,
            fill_per_use: Permille::new_unchecked(80),
            critical_threshold: Permille::new_unchecked(800),
        }
    }
}

impl Component for LatrineFullness {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WashBasinState {
    pub clean_water_units: u16,
    pub max_clean_water: u16,
    pub refill_per_tick: u16,
    pub units_per_full_wash: u16,
    pub dirtiness_level: Permille,
    pub dirtiness_per_use: Permille,
}

impl Default for WashBasinState {
    fn default() -> Self {
        Self {
            clean_water_units: 10,
            max_clean_water: 10,
            refill_per_tick: 1,
            units_per_full_wash: 2,
            dirtiness_level: Permille::ZERO,
            dirtiness_per_use: Permille::new_unchecked(50),
        }
    }
}

impl Component for WashBasinState {}

#[cfg(test)]
mod tests {
    use super::{LatrineFullness, PlaceDirtiness, WashBasinState};
    use crate::{Component, EntityKind, Permille, Tick, Topology, World, WorldError};
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component + Copy + Eq + PartialEq>() {}

    fn assert_bincode_roundtrip<T>(value: &T)
    where
        T: Debug + Eq + Serialize + DeserializeOwned,
    {
        let bytes = bincode::serialize(value).unwrap();
        let roundtrip: T = bincode::deserialize(&bytes).unwrap();

        assert_eq!(&roundtrip, value);
    }

    #[test]
    fn hygiene_components_have_spec_defaults() {
        assert_eq!(
            PlaceDirtiness::default(),
            PlaceDirtiness {
                value: Permille::ZERO,
                decay_per_tick: Permille::new_unchecked(2),
                dirtiness_per_use: Permille::new_unchecked(80),
            }
        );
        assert_eq!(
            LatrineFullness::default(),
            LatrineFullness {
                fill: Permille::ZERO,
                fill_per_use: Permille::new_unchecked(80),
                critical_threshold: Permille::new_unchecked(800),
            }
        );
        assert_eq!(
            WashBasinState::default(),
            WashBasinState {
                clean_water_units: 10,
                max_clean_water: 10,
                refill_per_tick: 1,
                units_per_full_wash: 2,
                dirtiness_level: Permille::ZERO,
                dirtiness_per_use: Permille::new_unchecked(50),
            }
        );
    }

    #[test]
    fn hygiene_components_satisfy_component_bounds() {
        assert_component_bounds::<PlaceDirtiness>();
        assert_component_bounds::<LatrineFullness>();
        assert_component_bounds::<WashBasinState>();
    }

    #[test]
    fn hygiene_components_roundtrip_through_bincode() {
        assert_bincode_roundtrip(&PlaceDirtiness {
            value: Permille::new_unchecked(500),
            decay_per_tick: Permille::new_unchecked(2),
            dirtiness_per_use: Permille::new_unchecked(80),
        });
        assert_bincode_roundtrip(&LatrineFullness {
            fill: Permille::new_unchecked(650),
            fill_per_use: Permille::new_unchecked(90),
            critical_threshold: Permille::new_unchecked(850),
        });
        assert_bincode_roundtrip(&WashBasinState {
            clean_water_units: 3,
            max_clean_water: 12,
            refill_per_tick: 2,
            units_per_full_wash: 4,
            dirtiness_level: Permille::new_unchecked(250),
            dirtiness_per_use: Permille::new_unchecked(75),
        });
    }

    #[test]
    fn place_dirtiness_accessors_are_place_only() {
        let mut world = World::new(Topology::new()).unwrap();
        let place = world.create_entity(EntityKind::Place, Tick(1));
        let facility = world.create_entity(EntityKind::Facility, Tick(1));
        let component = PlaceDirtiness {
            value: Permille::new_unchecked(500),
            ..PlaceDirtiness::default()
        };

        world
            .insert_component_place_dirtiness(place, component)
            .unwrap();

        assert_eq!(world.get_component_place_dirtiness(place), Some(&component));
        assert!(world.has_component_place_dirtiness(place));
        assert_eq!(world.count_with_place_dirtiness(), 1);
        assert_eq!(
            world.entities_with_place_dirtiness().collect::<Vec<_>>(),
            vec![place]
        );

        let error = world
            .insert_component_place_dirtiness(facility, PlaceDirtiness::default())
            .unwrap_err();
        assert!(matches!(error, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn latrine_fullness_accessors_are_place_only() {
        let mut world = World::new(Topology::new()).unwrap();
        let place = world.create_entity(EntityKind::Place, Tick(1));
        let facility = world.create_entity(EntityKind::Facility, Tick(1));
        let component = LatrineFullness {
            fill: Permille::new_unchecked(700),
            ..LatrineFullness::default()
        };

        world
            .insert_component_latrine_fullness(place, component)
            .unwrap();

        assert_eq!(
            world.get_component_latrine_fullness(place),
            Some(&component)
        );
        assert!(world.has_component_latrine_fullness(place));
        assert_eq!(world.count_with_latrine_fullness(), 1);

        let error = world
            .insert_component_latrine_fullness(facility, LatrineFullness::default())
            .unwrap_err();
        assert!(matches!(error, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn wash_basin_state_accessors_are_facility_only() {
        let mut world = World::new(Topology::new()).unwrap();
        let place = world.create_entity(EntityKind::Place, Tick(1));
        let facility = world.create_entity(EntityKind::Facility, Tick(1));
        let component = WashBasinState {
            clean_water_units: 4,
            max_clean_water: 12,
            ..WashBasinState::default()
        };

        world
            .insert_component_wash_basin_state(facility, component)
            .unwrap();

        assert_eq!(
            world.get_component_wash_basin_state(facility),
            Some(&component)
        );
        assert!(world.has_component_wash_basin_state(facility));
        assert_eq!(world.count_with_wash_basin_state(), 1);

        let error = world
            .insert_component_wash_basin_state(place, WashBasinState::default())
            .unwrap_err();
        assert!(matches!(error, WorldError::InvalidOperation(_)));
    }
}
