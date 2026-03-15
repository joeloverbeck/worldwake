//! Shared production and transport schema used by core components and sim registries.

use crate::{CommodityKind, Component, EntityId, LoadUnits, Quantity, Tick, TravelEdgeId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
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
    GravePlot,
}

impl WorkstationTag {
    pub const ALL: [Self; 8] = [
        Self::Forge,
        Self::Loom,
        Self::Mill,
        Self::ChoppingBlock,
        Self::WashBasin,
        Self::OrchardRow,
        Self::FieldPlot,
        Self::GravePlot,
    ];
}

/// Identifies a recipe definition in the recipe registry.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct RecipeId(pub u32);

/// Per-agent set of recipes this agent knows how to perform.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct KnownRecipes {
    pub recipes: BTreeSet<RecipeId>,
}

impl KnownRecipes {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(recipes: impl IntoIterator<Item = RecipeId>) -> Self {
        Self {
            recipes: recipes.into_iter().collect(),
        }
    }
}

impl Component for KnownRecipes {}

/// Marks a Facility entity as a workstation of a specific type.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct WorkstationMarker(pub WorkstationTag);

impl Component for WorkstationMarker {}

/// Maximum load an agent can carry.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct CarryCapacity(pub LoadUnits);

impl Component for CarryCapacity {}

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

/// Policy describing who owns output materialized by a producer.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ProductionOutputOwner {
    Actor,
    ProducerOwner,
    Unowned,
}

impl ProductionOutputOwner {
    pub const ALL: [Self; 3] = [Self::Actor, Self::ProducerOwner, Self::Unowned];
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ProductionOutputOwnershipPolicy {
    pub output_owner: ProductionOutputOwner,
}

impl Component for ProductionOutputOwnershipPolicy {}

/// Persistent work-in-progress state on a workstation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProductionJob {
    pub recipe_id: RecipeId,
    pub worker: EntityId,
    pub staged_inputs_container: EntityId,
    pub progress_ticks: u32,
}

impl Component for ProductionJob {}

/// Concrete route occupancy for an agent traveling along a topology edge.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InTransitOnEdge {
    pub edge_id: TravelEdgeId,
    pub origin: EntityId,
    pub destination: EntityId,
    pub departure_tick: Tick,
    pub arrival_tick: Tick,
}

impl Component for InTransitOnEdge {}

#[cfg(test)]
mod tests {
    use super::{
        CarryCapacity, InTransitOnEdge, KnownRecipes, ProductionJob, ProductionOutputOwner,
        ProductionOutputOwnershipPolicy, RecipeId, ResourceSource, WorkstationMarker,
        WorkstationTag,
    };
    use crate::{CommodityKind, Component, EntityId, LoadUnits, Quantity, Tick, TravelEdgeId};
    use serde::{de::DeserializeOwned, Serialize};
    use std::collections::BTreeSet;
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
    fn known_recipes_trait_bounds() {
        assert_component_bounds::<KnownRecipes>();
    }

    #[test]
    fn workstation_marker_trait_bounds() {
        assert_bounds::<WorkstationMarker>();
        assert_component_bounds::<WorkstationMarker>();
    }

    #[test]
    fn resource_source_trait_bounds() {
        assert_component_bounds::<ResourceSource>();
    }

    #[test]
    fn production_output_owner_trait_bounds() {
        assert_bounds::<ProductionOutputOwner>();
    }

    #[test]
    fn production_output_ownership_policy_trait_bounds() {
        assert_bounds::<ProductionOutputOwnershipPolicy>();
        assert_component_bounds::<ProductionOutputOwnershipPolicy>();
    }

    #[test]
    fn production_job_trait_bounds() {
        assert_component_bounds::<ProductionJob>();
    }

    #[test]
    fn carry_capacity_trait_bounds() {
        assert_bounds::<CarryCapacity>();
        assert_component_bounds::<CarryCapacity>();
    }

    #[test]
    fn in_transit_on_edge_trait_bounds() {
        assert_component_bounds::<InTransitOnEdge>();
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
                WorkstationTag::GravePlot,
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
    fn production_output_owner_variants_roundtrip_through_bincode() {
        for owner in ProductionOutputOwner::ALL {
            let bytes = bincode::serialize(&owner).unwrap();
            let roundtrip: ProductionOutputOwner = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, owner);
        }
    }

    #[test]
    fn production_output_owner_ordering_is_deterministic() {
        assert_eq!(
            ProductionOutputOwner::ALL,
            [
                ProductionOutputOwner::Actor,
                ProductionOutputOwner::ProducerOwner,
                ProductionOutputOwner::Unowned,
            ]
        );
    }

    #[test]
    fn production_output_ownership_policy_roundtrips_through_bincode() {
        let policy = ProductionOutputOwnershipPolicy {
            output_owner: ProductionOutputOwner::ProducerOwner,
        };

        let bytes = bincode::serialize(&policy).unwrap();
        let roundtrip: ProductionOutputOwnershipPolicy = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, policy);
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
    fn known_recipes_new_starts_empty() {
        assert_eq!(KnownRecipes::new(), KnownRecipes::default());
        assert!(KnownRecipes::new().recipes.is_empty());
    }

    #[test]
    fn known_recipes_with_deduplicates_and_sorts_recipe_ids() {
        let known = KnownRecipes::with([RecipeId(3), RecipeId(1), RecipeId(3), RecipeId(2)]);

        assert_eq!(
            known.recipes,
            BTreeSet::from([RecipeId(1), RecipeId(2), RecipeId(3)])
        );
    }

    #[test]
    fn known_recipes_roundtrips_through_bincode() {
        let known = KnownRecipes::with([RecipeId(4), RecipeId(1), RecipeId(9)]);

        let bytes = bincode::serialize(&known).unwrap();
        let roundtrip: KnownRecipes = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, known);
    }

    #[test]
    fn workstation_marker_roundtrips_through_bincode() {
        let marker = WorkstationMarker(WorkstationTag::Forge);

        let bytes = bincode::serialize(&marker).unwrap();
        let roundtrip: WorkstationMarker = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, marker);
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

    #[test]
    fn production_job_roundtrips_through_bincode() {
        let job = ProductionJob {
            recipe_id: RecipeId(8),
            worker: EntityId {
                slot: 3,
                generation: 0,
            },
            staged_inputs_container: EntityId {
                slot: 4,
                generation: 1,
            },
            progress_ticks: 11,
        };

        let bytes = bincode::serialize(&job).unwrap();
        let roundtrip: ProductionJob = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, job);
    }

    #[test]
    fn carry_capacity_roundtrips_through_bincode() {
        let capacity = CarryCapacity(LoadUnits(24));

        let bytes = bincode::serialize(&capacity).unwrap();
        let roundtrip: CarryCapacity = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, capacity);
    }

    #[test]
    fn in_transit_on_edge_roundtrips_through_bincode() {
        let transit = InTransitOnEdge {
            edge_id: TravelEdgeId(7),
            origin: EntityId {
                slot: 1,
                generation: 0,
            },
            destination: EntityId {
                slot: 2,
                generation: 0,
            },
            departure_tick: Tick(11),
            arrival_tick: Tick(17),
        };

        let bytes = bincode::serialize(&transit).unwrap();
        let roundtrip: InTransitOnEdge = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, transit);
    }
}
