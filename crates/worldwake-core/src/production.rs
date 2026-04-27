//! Shared production and transport schema used by core components and sim registries.

use crate::{CommodityKind, Component, EntityId, LoadUnits, Quantity, Tick, TravelEdgeId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::num::{NonZeroU8, NonZeroU32};

/// Tag identifying what kind of workstation an entity is.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum WorkstationTag {
    Forge,
    Loom,
    Mill,
    ChoppingBlock,
    WashBasin,
    Well,
    OrchardRow,
    FieldPlot,
    GravePlot,
}

impl WorkstationTag {
    pub const ALL: [Self; 9] = [
        Self::Forge,
        Self::Loom,
        Self::Mill,
        Self::ChoppingBlock,
        Self::WashBasin,
        Self::Well,
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
    pub extraction_slots: NonZeroU8,
    pub extraction_duration_ticks: NonZeroU32,
}

impl Component for ResourceSource {}

/// Default retention window (in ticks) for entries in [`LastHarvestTrace`].
/// Anchored to the agent's near-term observation horizon (S127 §D5).
/// Per-scenario overrides flow through `World::set_harvest_trace_retention_ticks`.
pub const HARVEST_TRACE_RETENTION_TICKS: u32 = 200;

/// Maximum number of entries retained in [`LastHarvestTrace`] before the
/// oldest entry by `tick` is evicted (S127 §D5).
pub const HARVEST_TRACE_MAX_ENTRIES: usize = 8;

/// One observed harvest event at a resource source.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HarvestTraceEntry {
    pub harvester: EntityId,
    pub tick: Tick,
    pub quantity: u16,
    pub partial: bool,
}

/// Bounded ring of recent harvest events at a resource source. Decays
/// alongside other site evidence in the existing item-decay maintenance
/// pass; retention is configured via `World::harvest_trace_retention_ticks`
/// (default `HARVEST_TRACE_RETENTION_TICKS`).
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct LastHarvestTrace {
    pub entries: Vec<HarvestTraceEntry>,
}

impl LastHarvestTrace {
    /// Append an entry, dropping the oldest by `tick` if the ring is at
    /// capacity. Ties on `tick` drop the earliest-positioned entry.
    pub fn push(&mut self, entry: HarvestTraceEntry) {
        if self.entries.len() >= HARVEST_TRACE_MAX_ENTRIES {
            let evict_index = self
                .entries
                .iter()
                .enumerate()
                .min_by_key(|(_, e)| e.tick.0)
                .map_or(0, |(idx, _)| idx);
            self.entries.remove(evict_index);
        }
        self.entries.push(entry);
    }
}

impl Component for LastHarvestTrace {}

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
        CarryCapacity, HARVEST_TRACE_MAX_ENTRIES, HarvestTraceEntry, InTransitOnEdge, KnownRecipes,
        LastHarvestTrace, ProductionJob, ProductionOutputOwner, ProductionOutputOwnershipPolicy,
        RecipeId, ResourceSource, WorkstationMarker, WorkstationTag,
    };
    use crate::{CommodityKind, Component, EntityId, LoadUnits, Quantity, Tick, TravelEdgeId};
    use serde::{Serialize, de::DeserializeOwned};
    use std::collections::BTreeSet;
    use std::num::{NonZeroU8, NonZeroU32};

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
                WorkstationTag::Well,
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
            extraction_slots: NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: NonZeroU32::new(1).unwrap(),
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
            extraction_slots: NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: NonZeroU32::new(1).unwrap(),
        };

        let bytes = bincode::serialize(&source).unwrap();
        let roundtrip: ResourceSource = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, source);
    }

    #[test]
    fn resource_source_bincode_roundtrip_includes_extraction_fields() {
        let source = ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(7),
            max_quantity: Quantity(15),
            regeneration_ticks_per_unit: Some(NonZeroU32::new(3).unwrap()),
            last_regeneration_tick: Some(Tick(12)),
            extraction_slots: NonZeroU8::new(5).unwrap(),
            extraction_duration_ticks: NonZeroU32::new(8).unwrap(),
        };

        let bytes = bincode::serialize(&source).unwrap();
        let roundtrip: ResourceSource = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, source);
        assert_eq!(roundtrip.extraction_slots.get(), 5);
        assert_eq!(roundtrip.extraction_duration_ticks.get(), 8);
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
    fn last_harvest_trace_trait_bounds() {
        assert_component_bounds::<LastHarvestTrace>();
    }

    fn entry(slot: u32, tick: u64, quantity: u16, partial: bool) -> HarvestTraceEntry {
        HarvestTraceEntry {
            harvester: EntityId {
                slot,
                generation: 0,
            },
            tick: Tick(tick),
            quantity,
            partial,
        }
    }

    #[test]
    fn last_harvest_trace_bincode_roundtrip() {
        let trace = LastHarvestTrace {
            entries: vec![
                entry(1, 10, 3, false),
                entry(2, 14, 1, true),
                entry(3, 22, 5, false),
            ],
        };

        let bytes = bincode::serialize(&trace).unwrap();
        let roundtrip: LastHarvestTrace = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, trace);
        assert_eq!(roundtrip.entries.len(), 3);
        assert_eq!(roundtrip.entries[1].quantity, 1);
        assert!(roundtrip.entries[1].partial);
    }

    #[test]
    fn last_harvest_trace_default_is_empty() {
        let trace = LastHarvestTrace::default();
        assert!(trace.entries.is_empty());
    }

    #[test]
    fn last_harvest_trace_push_evicts_oldest_when_full() {
        let mut trace = LastHarvestTrace::default();
        for tick in 1..=HARVEST_TRACE_MAX_ENTRIES as u64 {
            trace.push(entry(1, tick, 1, false));
        }
        assert_eq!(trace.entries.len(), HARVEST_TRACE_MAX_ENTRIES);
        assert_eq!(trace.entries.first().unwrap().tick, Tick(1));

        // 9th push should evict the entry with the smallest tick (Tick(1)).
        trace.push(entry(2, 99, 7, true));
        assert_eq!(trace.entries.len(), HARVEST_TRACE_MAX_ENTRIES);
        let smallest_tick = trace.entries.iter().map(|e| e.tick.0).min().unwrap();
        assert_eq!(smallest_tick, 2, "Tick(1) should have been evicted");
        assert!(
            trace
                .entries
                .iter()
                .any(|e| e.tick == Tick(99) && e.quantity == 7),
            "newest entry must be retained"
        );
    }

    #[test]
    fn last_harvest_trace_push_under_capacity_appends() {
        let mut trace = LastHarvestTrace::default();
        trace.push(entry(1, 5, 2, false));
        trace.push(entry(1, 10, 1, true));
        assert_eq!(trace.entries.len(), 2);
        assert_eq!(trace.entries[0].tick, Tick(5));
        assert_eq!(trace.entries[1].tick, Tick(10));
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
