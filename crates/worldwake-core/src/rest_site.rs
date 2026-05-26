//! Rest-site capacity and occupancy state for sleep affordances.

use crate::{Component, EntityId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

/// Maximum simultaneous sleepers a Place can host as a known rest site.
///
/// A Place without `RestCapacity` is not a known rest site. Rough sleep can
/// still occur there without consuming a rest slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestCapacity(pub NonZeroU32);

impl Component for RestCapacity {}

/// Authoritative multi-occupant rest-site state.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestOccupancy {
    pub occupants: BTreeSet<EntityId>,
}

impl Component for RestOccupancy {}

#[cfg(test)]
mod tests {
    use super::{RestCapacity, RestOccupancy};
    use crate::{Component, EntityId};
    use std::collections::BTreeSet;
    use std::num::NonZeroU32;

    fn assert_component<T: Component>() {}

    #[test]
    fn rest_capacity_constructs_as_component() {
        assert_component::<RestCapacity>();
        let capacity = RestCapacity(NonZeroU32::new(2).unwrap());
        assert_eq!(capacity.0.get(), 2);
    }

    #[test]
    fn rest_occupancy_tracks_deterministic_occupants() {
        assert_component::<RestOccupancy>();
        let occupant = EntityId {
            slot: 42,
            generation: 0,
        };
        let occupancy = RestOccupancy {
            occupants: BTreeSet::from([occupant]),
        };

        assert!(occupancy.occupants.contains(&occupant));
        assert_eq!(RestOccupancy::default().occupants.len(), 0);
    }
}
