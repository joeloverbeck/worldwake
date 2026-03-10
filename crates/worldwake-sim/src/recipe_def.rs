use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use worldwake_core::{BodyCostPerTick, CommodityKind, Quantity, WorkstationTag};

/// Data-driven production definition stored in simulation registry state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecipeDefinition {
    pub name: String,
    pub inputs: Vec<(CommodityKind, Quantity)>,
    pub outputs: Vec<(CommodityKind, Quantity)>,
    pub work_ticks: NonZeroU32,
    pub required_workstation_tag: Option<WorkstationTag>,
    pub required_tool_kinds: Vec<CommodityKind>,
    pub body_cost_per_tick: BodyCostPerTick,
}

#[cfg(test)]
mod tests {
    use super::RecipeDefinition;
    use serde::{de::DeserializeOwned, Serialize};
    use std::num::NonZeroU32;
    use worldwake_core::{BodyCostPerTick, CommodityKind, Permille, Quantity, WorkstationTag};

    fn assert_traits<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn sample_recipe() -> RecipeDefinition {
        RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(4).unwrap(),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: vec![CommodityKind::Water],
            body_cost_per_tick: BodyCostPerTick::new(pm(2), pm(3), pm(5), pm(1)),
        }
    }

    #[test]
    fn recipe_definition_satisfies_required_traits() {
        assert_traits::<RecipeDefinition>();
    }

    #[test]
    fn recipe_definition_roundtrips_through_bincode() {
        let recipe = sample_recipe();

        let bytes = bincode::serialize(&recipe).unwrap();
        let roundtrip: RecipeDefinition = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, recipe);
    }

    #[test]
    fn recipe_definition_allows_empty_inputs() {
        let mut recipe = sample_recipe();
        recipe.inputs.clear();

        assert!(recipe.inputs.is_empty());
    }

    #[test]
    fn recipe_definition_allows_empty_outputs() {
        let mut recipe = sample_recipe();
        recipe.outputs.clear();

        assert!(recipe.outputs.is_empty());
    }
}
