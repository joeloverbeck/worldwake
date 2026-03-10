//! Shared production-domain schema used by core components and sim registries.

use crate::{CommodityKind, Component, Quantity, Tick};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// Tag identifying what kind of workstation an entity is.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum WorkstationTag {
    Forge,
    Loom,
    Mill,
    ChoppingBlock,
    WashBasin,
    OrchardRow,
    FieldPlot,
}

impl WorkstationTag {
    pub const ALL: [Self; 7] = [
        Self::Forge,
        Self::Loom,
        Self::Mill,
        Self::ChoppingBlock,
        Self::WashBasin,
        Self::OrchardRow,
        Self::FieldPlot,
    ];
}

/// Identifies a recipe definition in the recipe registry.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct RecipeId(pub u32);

/// Concrete depletable stock of a commodity at a place or workstation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResourceSource {
    pub commodity: CommodityKind,
    pub available_quantity: Quantity,
    pub max_quantity: Quantity,
    pub regeneration_ticks_per_unit: Option<NonZeroU32>,
    pub last_regeneration_tick: Option<Tick>,
}

impl Component for ResourceSource {}

#[cfg(test)]
mod tests {
    use super::{RecipeId, ResourceSource, WorkstationTag};
    use crate::{CommodityKind, Component, Quantity, Tick};
    use serde::{de::DeserializeOwned, Serialize};
    use std::num::NonZeroU32;

    fn assert_bounds<
        T: Copy + Clone + Eq + Ord + std::hash::Hash + std::fmt::Debug + Serialize + DeserializeOwned,
    >() {
    }

    fn assert_component_bounds<
        T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned + Component,
    >() {
    }

    #[test]
    fn workstation_tag_trait_bounds() {
        assert_bounds::<WorkstationTag>();
    }

    #[test]
    fn recipe_id_trait_bounds() {
        assert_bounds::<RecipeId>();
    }

    #[test]
    fn resource_source_trait_bounds() {
        assert_component_bounds::<ResourceSource>();
    }

    #[test]
    fn workstation_tag_all_is_canonical_variant_list() {
        assert_eq!(
            WorkstationTag::ALL,
            [
                WorkstationTag::Forge,
                WorkstationTag::Loom,
                WorkstationTag::Mill,
                WorkstationTag::ChoppingBlock,
                WorkstationTag::WashBasin,
                WorkstationTag::OrchardRow,
                WorkstationTag::FieldPlot,
            ]
        );
    }

    #[test]
    fn workstation_tag_variants_roundtrip_through_bincode() {
        for tag in WorkstationTag::ALL {
            let bytes = bincode::serialize(&tag).unwrap();
            let roundtrip: WorkstationTag = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, tag);
        }
    }

    #[test]
    fn recipe_id_roundtrips_through_bincode() {
        let recipe_id = RecipeId(42);

        let bytes = bincode::serialize(&recipe_id).unwrap();
        let roundtrip: RecipeId = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, recipe_id);
    }

    #[test]
    fn recipe_id_ordering_is_deterministic() {
        assert!(RecipeId(0) < RecipeId(1));
    }

    #[test]
    fn workstation_tag_ordering_is_deterministic() {
        let mut reversed = WorkstationTag::ALL;
        reversed.reverse();
        reversed.sort();
        assert_eq!(reversed, WorkstationTag::ALL);
    }

    #[test]
    fn resource_source_roundtrips_without_regeneration_state() {
        let source = ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(8),
            max_quantity: Quantity(12),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        };

        let bytes = bincode::serialize(&source).unwrap();
        let roundtrip: ResourceSource = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, source);
    }

    #[test]
    fn resource_source_roundtrips_with_regeneration_state() {
        let source = ResourceSource {
            commodity: CommodityKind::Grain,
            available_quantity: Quantity(3),
            max_quantity: Quantity(20),
            regeneration_ticks_per_unit: Some(NonZeroU32::new(6).unwrap()),
            last_regeneration_tick: Some(Tick(44)),
        };

        let bytes = bincode::serialize(&source).unwrap();
        let roundtrip: ResourceSource = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, source);
    }
}
