use crate::RecipeDefinition;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldwake_core::{RecipeId, WorkstationTag};

/// Deterministic registry of all available production recipes.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecipeRegistry {
    recipes: Vec<RecipeDefinition>,
    by_workstation: BTreeMap<WorkstationTag, Vec<RecipeId>>,
}

impl RecipeRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, def: RecipeDefinition) -> RecipeId {
        let id = RecipeId(self.recipes.len() as u32);
        if let Some(tag) = def.required_workstation_tag {
            self.by_workstation.entry(tag).or_default().push(id);
        }
        self.recipes.push(def);
        id
    }

    #[must_use]
    pub fn get(&self, id: RecipeId) -> Option<&RecipeDefinition> {
        self.recipes.get(id.0 as usize)
    }

    #[must_use]
    pub fn recipes_for_workstation(&self, tag: WorkstationTag) -> &[RecipeId] {
        self.by_workstation.get(&tag).map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.recipes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.recipes.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (RecipeId, &RecipeDefinition)> {
        self.recipes
            .iter()
            .enumerate()
            .map(|(idx, def)| (RecipeId(idx as u32), def))
    }
}

#[cfg(test)]
mod tests {
    use super::RecipeRegistry;
    use crate::RecipeDefinition;
    use serde::{de::DeserializeOwned, Serialize};
    use std::num::NonZeroU32;
    use worldwake_core::{
        BodyCostPerTick, CommodityKind, Permille, Quantity, RecipeId, WorkstationTag,
    };

    fn assert_traits<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn sample_recipe(name: &str, workstation: Option<WorkstationTag>) -> RecipeDefinition {
        RecipeDefinition {
            name: name.to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: workstation,
            required_tool_kinds: vec![CommodityKind::Water],
            body_cost_per_tick: BodyCostPerTick::new(pm(1), pm(2), pm(3), pm(4)),
        }
    }

    #[test]
    fn recipe_registry_satisfies_required_traits() {
        assert_traits::<RecipeRegistry>();
    }

    #[test]
    fn registry_starts_empty() {
        let registry = RecipeRegistry::new();

        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.iter().next().is_none());
    }

    #[test]
    fn register_assigns_sequential_ids_and_get_returns_definitions() {
        let mut registry = RecipeRegistry::new();
        let first = sample_recipe("Harvest Apples", Some(WorkstationTag::OrchardRow));
        let second = sample_recipe("Bake Bread", Some(WorkstationTag::Mill));

        let first_id = registry.register(first.clone());
        let second_id = registry.register(second.clone());

        assert_eq!(first_id, RecipeId(0));
        assert_eq!(second_id, RecipeId(1));
        assert_eq!(registry.get(first_id), Some(&first));
        assert_eq!(registry.get(second_id), Some(&second));
        assert!(registry.get(RecipeId(2)).is_none());
    }

    #[test]
    fn recipes_for_workstation_returns_registered_ids_in_order() {
        let mut registry = RecipeRegistry::new();
        let orchard_first = registry.register(sample_recipe(
            "Harvest Apples",
            Some(WorkstationTag::OrchardRow),
        ));
        let mill = registry.register(sample_recipe("Bake Bread", Some(WorkstationTag::Mill)));
        let orchard_second = registry.register(sample_recipe(
            "Harvest Pears",
            Some(WorkstationTag::OrchardRow),
        ));

        assert_eq!(
            registry.recipes_for_workstation(WorkstationTag::OrchardRow),
            &[orchard_first, orchard_second]
        );
        assert_eq!(
            registry.recipes_for_workstation(WorkstationTag::Mill),
            &[mill]
        );
        assert!(registry
            .recipes_for_workstation(WorkstationTag::Forge)
            .is_empty());
    }

    #[test]
    fn workstationless_recipes_do_not_enter_secondary_index() {
        let mut registry = RecipeRegistry::new();
        let id = registry.register(sample_recipe("Rest", None));

        assert_eq!(registry.get(id).unwrap().required_workstation_tag, None);
        assert!(registry
            .recipes_for_workstation(WorkstationTag::Mill)
            .is_empty());
    }

    #[test]
    fn iter_returns_ids_and_definitions_in_registration_order() {
        let mut registry = RecipeRegistry::new();
        registry.register(sample_recipe("first", Some(WorkstationTag::Forge)));
        registry.register(sample_recipe("second", None));
        registry.register(sample_recipe("third", Some(WorkstationTag::Mill)));

        let names = registry
            .iter()
            .map(|(id, def)| (id, def.name.as_str()))
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                (RecipeId(0), "first"),
                (RecipeId(1), "second"),
                (RecipeId(2), "third"),
            ]
        );
    }

    #[test]
    fn registry_roundtrips_through_bincode() {
        let mut registry = RecipeRegistry::new();
        registry.register(sample_recipe(
            "Harvest Apples",
            Some(WorkstationTag::OrchardRow),
        ));
        registry.register(sample_recipe("Rest", None));

        let bytes = bincode::serialize(&registry).unwrap();
        let roundtrip: RecipeRegistry = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, registry);
    }
}
