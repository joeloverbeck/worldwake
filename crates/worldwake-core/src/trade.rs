//! Trade-domain authoritative components and shared schema.

use crate::{CommodityKind, Component, EntityId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Concrete merchant sale intent for an agent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MerchandiseProfile {
    pub sale_kinds: BTreeSet<CommodityKind>,
    pub home_market: Option<EntityId>,
}

impl Component for MerchandiseProfile {}

#[cfg(test)]
mod tests {
    use super::MerchandiseProfile;
    use crate::{traits::Component, CommodityKind, EntityId};
    use serde::{de::DeserializeOwned, Serialize};
    use std::collections::BTreeSet;
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn merchandise_profile_component_bounds() {
        assert_component_bounds::<MerchandiseProfile>();
        assert_value_bounds::<MerchandiseProfile>();
    }

    #[test]
    fn merchandise_profile_roundtrips_through_bincode() {
        let profile = MerchandiseProfile {
            sale_kinds: BTreeSet::from([CommodityKind::Bread, CommodityKind::Water]),
            home_market: Some(EntityId {
                slot: 7,
                generation: 2,
            }),
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: MerchandiseProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }
}
