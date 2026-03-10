//! Shared production-domain schema used by core components and sim registries.

use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::{RecipeId, WorkstationTag};
    use serde::{de::DeserializeOwned, Serialize};

    fn assert_bounds<
        T: Copy + Clone + Eq + Ord + std::hash::Hash + std::fmt::Debug + Serialize + DeserializeOwned,
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
}
