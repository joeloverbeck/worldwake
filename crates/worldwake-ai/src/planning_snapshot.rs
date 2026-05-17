use std::cell::Cell;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::num::NonZeroU32;
use worldwake_core::{
    ActionDefId, AgentBeliefStore, ArtifactPostingProfile, BeliefConfidencePolicy,
    BelievedEntityState, BelievedInstitutionalClaim, BlockerMemory, BlockingFact, CombatProfile,
    CommodityConsumableProfile, CommodityKind, ContentionGrant, DemandObservation, DisposalProfile,
    DriveThresholds, EntityId, EntityKind, EpistemicDispositionProfile, ExpectationStore,
    HomeostaticNeeds, InTransitOnEdge, InstitutionalBeliefRead, JusticeDispositionProfile,
    LastSeenMemory, LoadUnits, MerchandiseProfile, MetabolismProfile, OfficeData, PatrolProfile,
    PatrolRoute, Permille, PlaceTag, Quantity, RecipeId, RecordData, RecordedViolation,
    ResourceSource, SocialObservation, StockStoragePolicy, SuccessionLaw, TellMemoryKey,
    TellProfile, TheftDispositionProfile, Tick, TickRange, ToldBeliefMemory,
    TradeDispositionProfile, UniqueItemKind, ViolationDispositionProfile, WashBasinState,
    WorkstationTag, Wound,
};
use worldwake_sim::{BeliefRead, RuntimeBeliefView};

use crate::route_threat::perceived_direct_travel_cost_from_memory;
use crate::{PlannerOpKind, SnapshotCacheCounters};

type SupportBeliefRead = InstitutionalBeliefRead<Option<EntityId>>;
type ForceControllerBeliefRead = InstitutionalBeliefRead<(Option<EntityId>, bool)>;
type OfficeSupportBeliefReads = Vec<(EntityId, SupportBeliefRead)>;

const ITEM_INTERACTING_OPS: [PlannerOpKind; 8] = [
    PlannerOpKind::Consume,
    PlannerOpKind::Trade,
    PlannerOpKind::Craft,
    PlannerOpKind::Loot,
    PlannerOpKind::Harvest,
    PlannerOpKind::MoveCargo,
    PlannerOpKind::StockManagement,
    PlannerOpKind::Heal,
];

const INSTITUTIONAL_OPS: [PlannerOpKind; 14] = [
    PlannerOpKind::ConsultRecord,
    PlannerOpKind::ReportFound,
    PlannerOpKind::PostBounty,
    PlannerOpKind::ClaimBounty,
    PlannerOpKind::PostNotice,
    PlannerOpKind::Accuse,
    PlannerOpKind::Fine,
    PlannerOpKind::Investigate,
    PlannerOpKind::Bribe,
    PlannerOpKind::Threaten,
    PlannerOpKind::Exile,
    PlannerOpKind::DeclareSupport,
    PlannerOpKind::PressForceClaim,
    PlannerOpKind::YieldForceClaim,
];

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotEntityFilter {
    needs_items: bool,
    needs_institutional: bool,
    needs_dead_agents: bool,
}

impl SnapshotEntityFilter {
    #[must_use]
    pub(crate) fn from_relevant_ops(ops: &[PlannerOpKind]) -> Self {
        if ops.is_empty() {
            return Self::unfiltered();
        }
        Self {
            needs_items: ops.iter().any(|op| ITEM_INTERACTING_OPS.contains(op)),
            needs_institutional: ops.iter().any(|op| INSTITUTIONAL_OPS.contains(op)),
            needs_dead_agents: ops.contains(&PlannerOpKind::Loot),
        }
    }

    #[must_use]
    pub(crate) const fn unfiltered() -> Self {
        Self {
            needs_items: true,
            needs_institutional: true,
            needs_dead_agents: true,
        }
    }

    #[must_use]
    pub(crate) const fn needs_items(self) -> bool {
        self.needs_items
    }

    #[must_use]
    pub(crate) const fn needs_institutional(self) -> bool {
        self.needs_institutional
    }

    #[must_use]
    pub(crate) const fn includes(self, kind: EntityKind, alive: bool) -> bool {
        match kind {
            EntityKind::Agent => alive || self.needs_dead_agents,
            EntityKind::Facility
            | EntityKind::Container
            | EntityKind::Office
            | EntityKind::Place => true,
            EntityKind::ItemLot | EntityKind::UniqueItem => self.needs_items(),
            EntityKind::Faction | EntityKind::Record | EntityKind::SocialArtifact => {
                self.needs_institutional()
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotFacilityQueue {
    pub(crate) actor_queue_position: Option<u32>,
    pub(crate) active_grant: Option<ContentionGrant>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotEntityCore {
    pub(crate) kind: Option<EntityKind>,
    pub(crate) alive: bool,
    pub(crate) dead: bool,
    pub(crate) incapacitated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotSpatial {
    pub(crate) effective_place: Option<EntityId>,
    pub(crate) in_transit_state: Option<InTransitOnEdge>,
    pub(crate) patrol_route: Option<PatrolRoute>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotInventory {
    pub(crate) direct_container: Option<EntityId>,
    pub(crate) direct_possessor: Option<EntityId>,
    pub(crate) direct_possessions: BTreeSet<EntityId>,
    pub(crate) direct_contents: BTreeSet<EntityId>,
    pub(crate) known_recipes: Vec<RecipeId>,
    pub(crate) unique_item_counts: BTreeMap<UniqueItemKind, u32>,
    pub(crate) commodity_quantities: BTreeMap<CommodityKind, Quantity>,
    pub(crate) item_lot_commodity: Option<CommodityKind>,
    pub(crate) carry_capacity: Option<LoadUnits>,
    pub(crate) intrinsic_load: LoadUnits,
    pub(crate) item_lot_consumable_profile: Option<CommodityConsumableProfile>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotCombat {
    pub(crate) wounds: Vec<Wound>,
    pub(crate) patrol_profile: Option<PatrolProfile>,
    pub(crate) combat_profile: Option<CombatProfile>,
    pub(crate) courage: Option<Permille>,
    pub(crate) hostile_targets: Vec<EntityId>,
    pub(crate) visible_hostiles: Vec<EntityId>,
    pub(crate) current_attackers: Vec<EntityId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotSocial {
    pub(crate) theft_disposition_profile: Option<TheftDispositionProfile>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotEconomic {
    pub(crate) trade_disposition_profile: Option<TradeDispositionProfile>,
    pub(crate) demand_memory: Vec<DemandObservation>,
    pub(crate) merchandise_profile: Option<MerchandiseProfile>,
    pub(crate) has_sale_listing: bool,
    pub(crate) seller_for_sale_lot: Option<EntityId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotPolitical {
    pub(crate) justice_disposition_profile: Option<JusticeDispositionProfile>,
    pub(crate) violation_disposition_profile: Option<ViolationDispositionProfile>,
    pub(crate) record_data: Option<RecordData>,
    /// For Office entities: authoritative office metadata preserved for planning.
    pub(crate) office_data: Option<OfficeData>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotTemporal {
    pub(crate) reservation_ranges: Vec<TickRange>,
    pub(crate) facility_queue: Option<SnapshotFacilityQueue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotProfiles {
    pub(crate) homeostatic_needs: Option<HomeostaticNeeds>,
    pub(crate) drive_thresholds: Option<DriveThresholds>,
    pub(crate) metabolism_profile: Option<MetabolismProfile>,
    pub(crate) disposal_profile: Option<DisposalProfile>,
    pub(crate) artifact_posting_profile: Option<ArtifactPostingProfile>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotFacility {
    pub(crate) workstation_tag: Option<WorkstationTag>,
    pub(crate) stock_storage_policy: Option<StockStoragePolicy>,
    pub(crate) resource_source: Option<ResourceSource>,
    pub(crate) wash_basin_state: Option<WashBasinState>,
    pub(crate) has_production_job: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotControl {
    pub(crate) owner: Option<EntityId>,
    pub(crate) controllable_by_actor: bool,
    pub(crate) has_control: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotEntity {
    pub(crate) entity: SnapshotEntityCore,
    pub(crate) spatial: SnapshotSpatial,
    pub(crate) inventory: SnapshotInventory,
    pub(crate) combat: SnapshotCombat,
    pub(crate) social: SnapshotSocial,
    pub(crate) economic: SnapshotEconomic,
    pub(crate) political: SnapshotPolitical,
    pub(crate) temporal: SnapshotTemporal,
    pub(crate) profiles: SnapshotProfiles,
    pub(crate) facility: SnapshotFacility,
    pub(crate) control: SnapshotControl,
}

impl Default for SnapshotEntity {
    fn default() -> Self {
        Self {
            entity: SnapshotEntityCore {
                kind: None,
                alive: false,
                dead: false,
                incapacitated: false,
            },
            spatial: SnapshotSpatial {
                effective_place: None,
                in_transit_state: None,
                patrol_route: None,
            },
            inventory: SnapshotInventory {
                direct_container: None,
                direct_possessor: None,
                direct_possessions: BTreeSet::new(),
                direct_contents: BTreeSet::new(),
                known_recipes: Vec::new(),
                unique_item_counts: BTreeMap::new(),
                commodity_quantities: BTreeMap::new(),
                item_lot_commodity: None,
                carry_capacity: None,
                intrinsic_load: LoadUnits(0),
                item_lot_consumable_profile: None,
            },
            combat: SnapshotCombat {
                wounds: Vec::new(),
                patrol_profile: None,
                combat_profile: None,
                courage: None,
                hostile_targets: Vec::new(),
                visible_hostiles: Vec::new(),
                current_attackers: Vec::new(),
            },
            social: SnapshotSocial {
                theft_disposition_profile: None,
            },
            economic: SnapshotEconomic {
                trade_disposition_profile: None,
                demand_memory: Vec::new(),
                merchandise_profile: None,
                has_sale_listing: false,
                seller_for_sale_lot: None,
            },
            political: SnapshotPolitical {
                justice_disposition_profile: None,
                violation_disposition_profile: None,
                record_data: None,
                office_data: None,
            },
            temporal: SnapshotTemporal {
                reservation_ranges: Vec::new(),
                facility_queue: None,
            },
            profiles: SnapshotProfiles {
                homeostatic_needs: None,
                drive_thresholds: None,
                metabolism_profile: None,
                disposal_profile: None,
                artifact_posting_profile: None,
            },
            facility: SnapshotFacility {
                workstation_tag: None,
                stock_storage_policy: None,
                resource_source: None,
                wash_basin_state: None,
                has_production_job: false,
            },
            control: SnapshotControl {
                owner: None,
                controllable_by_actor: false,
                has_control: false,
            },
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct DirectPerceivedTravelCostBreakdown {
    pub base_ticks: u32,
    pub threat: Permille,
    pub penalty_ticks: u32,
    pub perceived_cost: u32,
}

/// Compact all-pairs shortest distance matrix using a flat `Vec<u32>`.
/// Place IDs are mapped to dense indices for O(1) lookups instead of
/// `BTreeMap<(EntityId, EntityId), u32>` which has O(log n) per lookup.
#[derive(Clone, Debug, Eq, PartialEq)]
struct DistanceMatrix {
    /// Ordered place IDs — index in this vec is the dense index.
    place_ids: Vec<EntityId>,
    /// Flat row-major matrix of size `n * n`. `u32::MAX` means unreachable.
    data: Vec<u32>,
}

impl DistanceMatrix {
    fn new(places: &BTreeMap<EntityId, SnapshotPlace>) -> Self {
        Self::with_edge_cost(places, |_from, _to, ticks| ticks.get())
    }

    fn with_edge_cost<F>(places: &BTreeMap<EntityId, SnapshotPlace>, edge_cost: F) -> Self
    where
        F: Fn(EntityId, EntityId, NonZeroU32) -> u32,
    {
        let place_ids: Vec<EntityId> = places.keys().copied().collect();
        let n = place_ids.len();
        let idx: BTreeMap<EntityId, usize> = place_ids
            .iter()
            .copied()
            .enumerate()
            .map(|(i, id)| (id, i))
            .collect();
        let mut data = vec![u32::MAX; n * n];

        // Initialize from direct adjacency edges.
        for (&place, snapshot_place) in places {
            let Some(&i) = idx.get(&place) else {
                continue;
            };
            for &(adjacent, ticks) in &snapshot_place.adjacent_places_with_travel_ticks {
                let Some(&j) = idx.get(&adjacent) else {
                    continue;
                };
                let weight = edge_cost(place, adjacent, ticks);
                let cell = &mut data[i * n + j];
                *cell = (*cell).min(weight);
            }
        }

        // Floyd-Warshall relaxation.
        for k in 0..n {
            for i in 0..n {
                let dist_ik = data[i * n + k];
                if dist_ik == u32::MAX {
                    continue;
                }
                for j in 0..n {
                    let dist_kj = data[k * n + j];
                    if dist_kj == u32::MAX {
                        continue;
                    }
                    let candidate = dist_ik.saturating_add(dist_kj);
                    let cell = &mut data[i * n + j];
                    if candidate < *cell {
                        *cell = candidate;
                    }
                }
            }
        }

        Self { place_ids, data }
    }

    fn get(&self, from: EntityId, to: EntityId) -> Option<u32> {
        let n = self.place_ids.len();
        let i = self.place_ids.binary_search(&from).ok()?;
        let j = self.place_ids.binary_search(&to).ok()?;
        let val = self.data[i * n + j];
        if val == u32::MAX { None } else { Some(val) }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct SnapshotPlace {
    pub(crate) entities: BTreeSet<EntityId>,
    pub(crate) tags: BTreeSet<PlaceTag>,
    pub(crate) adjacent_places_with_travel_ticks: Vec<(EntityId, NonZeroU32)>,
    pub(crate) bandit_camp_faction: Option<EntityId>,
}

/// Authoritative travel data is intentionally unreachable from outside
/// `worldwake-ai`. External planner-adjacent code must use accessor methods
/// instead of reading the underlying matrix.
///
/// ```compile_fail
/// use worldwake_ai::PlanningSnapshot;
///
/// fn read_authoritative(snapshot: &PlanningSnapshot) {
///     let _ = &snapshot.shortest_travel_ticks as *const _ as *const ();
/// }
/// ```
///
/// ```compile_fail
/// fn mention_authoritative_type() {
///     let _: Option<worldwake_ai::planning_snapshot::DistanceMatrix> = None;
/// }
/// ```
///
/// ```
/// use worldwake_ai::PlanningSnapshot;
/// use worldwake_core::EntityId;
///
/// fn read_via_accessors(
///     snapshot: &PlanningSnapshot,
///     from: EntityId,
///     to: EntityId,
///     destinations: &[EntityId],
/// ) -> (Option<u32>, Option<u32>) {
///     (
///         snapshot.min_travel_ticks(from, to),
///         snapshot.min_travel_ticks_to_any(from, destinations),
///     )
/// }
/// ```
pub struct PlanningSnapshot {
    pub(crate) actor: EntityId,
    pub(crate) current_tick: Tick,
    pub(crate) actor_belief_store: AgentBeliefStore,
    pub(crate) entities: BTreeMap<EntityId, SnapshotEntity>,
    pub(crate) places: BTreeMap<EntityId, SnapshotPlace>,
    pub(crate) blocked_facility_uses: BTreeSet<(EntityId, ActionDefId)>,
    pub(crate) actor_known_entity_beliefs: BTreeMap<EntityId, BelievedEntityState>,
    pub(crate) actor_known_social_observations: Vec<SocialObservation>,
    pub(crate) actor_known_institutional_beliefs: Vec<BelievedInstitutionalClaim>,
    pub(crate) actor_told_beliefs: BTreeMap<TellMemoryKey, ToldBeliefMemory>,
    pub(crate) actor_bandit_factions: Vec<EntityId>,
    pub(crate) actor_active_violation_records: Vec<RecordedViolation>,
    pub(crate) actor_contested_offices: Vec<EntityId>,
    pub(crate) actor_loyalties: BTreeMap<EntityId, Permille>,
    pub(crate) actor_office_holder_beliefs: BTreeMap<EntityId, SupportBeliefRead>,
    pub(crate) actor_force_controller_beliefs: BTreeMap<EntityId, ForceControllerBeliefRead>,
    /// Baseline believed-certain support declarations per office: (supporter, candidate) pairs.
    pub(crate) office_certain_support_declarations: BTreeMap<EntityId, Vec<(EntityId, EntityId)>>,
    /// Belief-derived support declaration reads per office.
    pub(crate) office_support_declaration_beliefs: BTreeMap<EntityId, OfficeSupportBeliefReads>,
    pub(crate) actor_confidence_policy: BeliefConfidencePolicy,
    pub(crate) actor_claim_confidence_threshold: Permille,
    pub(crate) actor_tell_profile: Option<TellProfile>,
    pub(crate) actor_epistemic_profile: Option<EpistemicDispositionProfile>,
    pub(crate) actor_consultation_speed_factor: Option<Permille>,
    pub(crate) actor_expectation_store: Option<ExpectationStore>,
    pub(crate) actor_last_seen_memory: Option<LastSeenMemory>,
    pub(crate) actor_bandit_flee_thresholds: BTreeMap<EntityId, Permille>,
    pub(crate) actor_bandit_establishment_ticks: BTreeMap<EntityId, NonZeroU32>,
    /// All-pairs shortest travel times between snapshot places.
    /// Computed via Floyd-Warshall during construction. O(n^3) where n is
    /// the number of places in the snapshot (typically 10-20, so < 8000 ops).
    shortest_travel_ticks: DistanceMatrix,
    perceived_travel_costs: DistanceMatrix,
    cache_hit_count: Cell<u64>,
    cache_miss_count: Cell<u64>,
}

impl PlanningSnapshot {
    #[must_use]
    pub fn build(
        view: &dyn RuntimeBeliefView,
        actor: EntityId,
        evidence_entities: &BTreeSet<EntityId>,
        evidence_places: &BTreeSet<EntityId>,
        travel_horizon: u8,
    ) -> Self {
        Self::build_with_blocked_facility_uses(
            view,
            actor,
            evidence_entities,
            evidence_places,
            travel_horizon,
            &BTreeSet::new(),
            SnapshotEntityFilter::unfiltered(),
        )
    }

    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn build_with_blocked_facility_uses(
        view: &dyn RuntimeBeliefView,
        actor: EntityId,
        evidence_entities: &BTreeSet<EntityId>,
        evidence_places: &BTreeSet<EntityId>,
        travel_horizon: u8,
        blocked_facility_uses: &BTreeSet<(EntityId, ActionDefId)>,
        filter: SnapshotEntityFilter,
    ) -> Self {
        let included_places = collect_places(
            view,
            actor,
            evidence_entities,
            evidence_places,
            travel_horizon,
        );
        let actor_known_entity_beliefs: BTreeMap<EntityId, BelievedEntityState> =
            view.known_entity_beliefs(actor).into_iter().collect();
        let included_entities = collect_entities(
            view,
            actor,
            evidence_entities,
            &included_places,
            filter,
            &actor_known_entity_beliefs,
        );
        let mut entities: BTreeMap<EntityId, SnapshotEntity> = included_entities
            .iter()
            .copied()
            .map(|entity| {
                (
                    entity,
                    build_snapshot_entity(
                        view,
                        actor,
                        entity,
                        evidence_entities,
                        &included_entities,
                        &actor_known_entity_beliefs,
                    ),
                )
            })
            .collect();
        let places =
            build_snapshot_places(view, actor, &included_places, &entities, evidence_entities);
        let actor_known_social_observations = view.known_social_observations(actor);
        let actor_confidence_policy = view.belief_confidence_policy(actor);
        let actor_claim_confidence_threshold = view.claim_confidence_threshold(actor);
        let content_edges = entities
            .iter()
            .filter_map(|(&entity, snapshot)| {
                snapshot
                    .inventory
                    .direct_container
                    .map(|container| (entity, container))
            })
            .collect::<Vec<_>>();
        for (entity, container) in content_edges {
            if let Some(container_snapshot) = entities.get_mut(&container) {
                container_snapshot.inventory.direct_contents.insert(entity);
            }
        }

        let shortest_travel_ticks = DistanceMatrix::new(&places);
        let perceived_travel_costs = DistanceMatrix::with_edge_cost(&places, |from, to, ticks| {
            perceived_direct_travel_cost_from_memory(
                view.current_tick(),
                actor_confidence_policy,
                &actor_known_entity_beliefs,
                &actor_known_social_observations,
                from,
                to,
                ticks.get(),
            )
        });

        Self {
            actor,
            current_tick: view.current_tick(),
            actor_belief_store: view.agent_belief_store(actor).cloned().unwrap_or_default(),
            entities,
            places,
            blocked_facility_uses: blocked_facility_uses.clone(),
            actor_known_entity_beliefs,
            actor_known_social_observations,
            actor_known_institutional_beliefs: view.known_institutional_beliefs(actor),
            actor_told_beliefs: view.told_belief_memories(actor).into_iter().collect(),
            actor_bandit_factions: view.bandit_factions_of(actor),
            actor_active_violation_records: view.active_violation_records(actor),
            actor_contested_offices: view.offices_contested_by(actor),
            actor_loyalties: included_entities
                .iter()
                .copied()
                .filter_map(|target| {
                    view.loyalty_to(actor, target)
                        .map(|strength| (target, strength))
                })
                .collect(),
            actor_office_holder_beliefs: included_entities
                .iter()
                .copied()
                .filter(|entity| view.entity_kind(*entity) == Some(EntityKind::Office))
                .map(|office| {
                    (
                        office,
                        view.agent_belief_store(actor).map_or_else(
                            || match view.believed_office_holder(office) {
                                BeliefRead::Known(holder) | BeliefRead::Stale(holder) => {
                                    InstitutionalBeliefRead::Certain(holder.value)
                                }
                                BeliefRead::Unknown => InstitutionalBeliefRead::Unknown,
                            },
                            |store| store.believed_office_holder(office),
                        ),
                    )
                })
                .collect(),
            actor_force_controller_beliefs: included_entities
                .iter()
                .copied()
                .filter(|entity| view.entity_kind(*entity) == Some(EntityKind::Office))
                .map(|office| (office, view.believed_force_controller(office)))
                .collect(),
            office_certain_support_declarations: included_entities
                .iter()
                .copied()
                .filter(|entity| view.entity_kind(*entity) == Some(EntityKind::Office))
                .map(|office| {
                    let declarations = view
                        .believed_support_declarations_for_office(office)
                        .into_iter()
                        .filter_map(|(supporter, read)| match read {
                            InstitutionalBeliefRead::Certain(Some(candidate)) => {
                                Some((supporter, candidate))
                            }
                            _ => None,
                        })
                        .collect();
                    (office, declarations)
                })
                .collect(),
            office_support_declaration_beliefs: included_entities
                .iter()
                .copied()
                .filter(|entity| view.entity_kind(*entity) == Some(EntityKind::Office))
                .map(|office| {
                    (
                        office,
                        view.believed_support_declarations_for_office(office),
                    )
                })
                .collect(),
            actor_confidence_policy,
            actor_claim_confidence_threshold,
            actor_tell_profile: view.tell_profile(actor),
            actor_expectation_store: view.expectation_store(actor),
            actor_last_seen_memory: view.last_seen_memory(actor),
            actor_epistemic_profile: view.epistemic_disposition_profile(actor),
            actor_consultation_speed_factor: view.consultation_speed_factor(actor),
            actor_bandit_flee_thresholds: view
                .bandit_factions_of(actor)
                .into_iter()
                .filter_map(|faction| {
                    view.bandit_flee_wound_threshold(faction)
                        .map(|threshold| (faction, threshold))
                })
                .collect(),
            actor_bandit_establishment_ticks: view
                .bandit_factions_of(actor)
                .into_iter()
                .filter_map(|faction| {
                    view.bandit_camp_establishment_ticks(faction)
                        .map(|ticks| (faction, ticks))
                })
                .collect(),
            shortest_travel_ticks,
            perceived_travel_costs,
            cache_hit_count: Cell::new(0),
            cache_miss_count: Cell::new(0),
        }
    }

    #[must_use]
    pub fn actor(&self) -> EntityId {
        self.actor
    }

    #[cfg(test)]
    pub(crate) fn for_effect_sink_test(
        actor: EntityId,
        actor_belief_store: AgentBeliefStore,
        present_entities: &[EntityId],
    ) -> Self {
        let entities = present_entities
            .iter()
            .copied()
            .map(|entity| (entity, SnapshotEntity::default()))
            .collect();
        let places = BTreeMap::new();
        let shortest_travel_ticks = DistanceMatrix::new(&places);
        let perceived_travel_costs = DistanceMatrix::new(&places);

        Self {
            actor,
            current_tick: Tick(0),
            actor_belief_store,
            entities,
            places,
            blocked_facility_uses: BTreeSet::new(),
            actor_known_entity_beliefs: BTreeMap::new(),
            actor_known_social_observations: Vec::new(),
            actor_known_institutional_beliefs: Vec::new(),
            actor_told_beliefs: BTreeMap::new(),
            actor_bandit_factions: Vec::new(),
            actor_active_violation_records: Vec::new(),
            actor_contested_offices: Vec::new(),
            actor_loyalties: BTreeMap::new(),
            actor_office_holder_beliefs: BTreeMap::new(),
            actor_force_controller_beliefs: BTreeMap::new(),
            office_certain_support_declarations: BTreeMap::new(),
            office_support_declaration_beliefs: BTreeMap::new(),
            actor_confidence_policy: BeliefConfidencePolicy::default(),
            actor_claim_confidence_threshold: Permille::ZERO,
            actor_tell_profile: None,
            actor_epistemic_profile: None,
            actor_consultation_speed_factor: None,
            actor_expectation_store: None,
            actor_last_seen_memory: None,
            actor_bandit_flee_thresholds: BTreeMap::new(),
            actor_bandit_establishment_ticks: BTreeMap::new(),
            shortest_travel_ticks,
            perceived_travel_costs,
            cache_hit_count: Cell::new(0),
            cache_miss_count: Cell::new(0),
        }
    }

    /// Canonical seat place for an Office entity, captured from `OfficeData.seat`.
    #[must_use]
    pub(crate) fn seat(&self, office: EntityId) -> Option<EntityId> {
        self.entities
            .get(&office)
            .and_then(|snapshot| snapshot.political.office_data.as_ref())
            .map(|office_data| office_data.seat)
    }

    /// Base support declarations for an office, captured at snapshot build time.
    /// Returns `(supporter, candidate)` pairs.
    #[must_use]
    pub(crate) fn base_support_declarations_for_office(
        &self,
        office: EntityId,
    ) -> &[(EntityId, EntityId)] {
        self.office_certain_support_declarations
            .get(&office)
            .map_or(&[], std::vec::Vec::as_slice)
    }

    #[must_use]
    pub(crate) fn believed_office_holder(&self, office: EntityId) -> SupportBeliefRead {
        self.actor_office_holder_beliefs
            .get(&office)
            .cloned()
            .unwrap_or(InstitutionalBeliefRead::Unknown)
    }

    #[must_use]
    pub(crate) fn believed_force_controller(&self, office: EntityId) -> ForceControllerBeliefRead {
        self.actor_force_controller_beliefs
            .get(&office)
            .cloned()
            .unwrap_or(InstitutionalBeliefRead::Unknown)
    }

    #[must_use]
    pub(crate) fn succession_law(&self, office: EntityId) -> Option<SuccessionLaw> {
        self.entities
            .get(&office)
            .and_then(|snapshot| snapshot.political.office_data.as_ref())
            .map(|office_data| office_data.succession_law.clone())
    }

    #[must_use]
    pub(crate) fn office_data(&self, office: EntityId) -> Option<OfficeData> {
        self.entities
            .get(&office)
            .and_then(|snapshot| snapshot.political.office_data.clone())
    }

    #[must_use]
    pub(crate) fn bandit_camp_establishment_ticks(&self, faction: EntityId) -> Option<NonZeroU32> {
        self.actor_bandit_establishment_ticks.get(&faction).copied()
    }

    #[must_use]
    pub(crate) fn bandit_flee_wound_threshold(&self, faction: EntityId) -> Option<Permille> {
        self.actor_bandit_flee_thresholds.get(&faction).copied()
    }

    #[must_use]
    pub(crate) fn believed_support_declarations_for_office(
        &self,
        office: EntityId,
    ) -> &[(EntityId, SupportBeliefRead)] {
        self.office_support_declaration_beliefs
            .get(&office)
            .map_or(&[], std::vec::Vec::as_slice)
    }

    #[must_use]
    pub(crate) fn believed_support_declaration(
        &self,
        office: EntityId,
        supporter: EntityId,
    ) -> SupportBeliefRead {
        self.believed_support_declarations_for_office(office)
            .iter()
            .find(|(belief_supporter, _)| *belief_supporter == supporter)
            .map_or(InstitutionalBeliefRead::Unknown, |(_, read)| read.clone())
    }

    /// Minimum travel ticks from `from` to `to`, or `None` if unreachable.
    /// Returns `Some(0)` if `from == to`.
    #[must_use]
    pub fn min_travel_ticks(&self, from: EntityId, to: EntityId) -> Option<u32> {
        if from == to {
            return Some(0);
        }
        self.record_cache_access(self.shortest_travel_ticks.get(from, to))
    }

    /// Minimum travel ticks from `from` to the nearest place in `destinations`.
    /// Returns `Some(0)` if `from` is in `destinations`.
    /// Returns `None` if `destinations` is empty or all destinations are unreachable.
    #[must_use]
    pub fn min_travel_ticks_to_any(
        &self,
        from: EntityId,
        destinations: &[EntityId],
    ) -> Option<u32> {
        if destinations.contains(&from) {
            return Some(0);
        }
        destinations
            .iter()
            .filter_map(|dest| {
                self.record_cache_access(self.shortest_travel_ticks.get(from, *dest))
            })
            .min()
    }

    #[must_use]
    pub(crate) fn min_perceived_travel_cost_to_any(
        &self,
        from: EntityId,
        destinations: &[EntityId],
    ) -> Option<u32> {
        if destinations.contains(&from) {
            return Some(0);
        }
        destinations
            .iter()
            .filter_map(|dest| {
                self.record_cache_access(self.perceived_travel_costs.get(from, *dest))
            })
            .min()
    }

    #[must_use]
    pub(crate) fn direct_perceived_travel_cost(&self, from: EntityId, to: EntityId) -> Option<u32> {
        self.direct_perceived_travel_breakdown(from, to)
            .map(|breakdown| breakdown.perceived_cost)
    }

    #[must_use]
    pub(crate) fn direct_perceived_travel_breakdown(
        &self,
        from: EntityId,
        to: EntityId,
    ) -> Option<DirectPerceivedTravelCostBreakdown> {
        let base_ticks = self
            .places
            .get(&from)?
            .adjacent_places_with_travel_ticks
            .iter()
            .find(|(adjacent, _)| *adjacent == to)
            .map(|(_, ticks)| ticks.get())?;
        let threat = crate::route_threat::route_threat_estimate(self, from, to);
        let penalty_ticks = if threat.value() == 0 {
            0
        } else {
            base_ticks
                .saturating_mul(u32::from(threat.value()))
                .div_ceil(1000)
        };
        Some(DirectPerceivedTravelCostBreakdown {
            base_ticks,
            threat,
            penalty_ticks,
            perceived_cost: base_ticks.saturating_add(penalty_ticks),
        })
    }

    #[must_use]
    pub fn snapshot_cache_counters(&self) -> SnapshotCacheCounters {
        SnapshotCacheCounters {
            cache_hit_count: self.cache_hit_count.get(),
            cache_miss_count: self.cache_miss_count.get(),
            cache_invalidation_count: 0,
        }
    }

    fn record_cache_access(&self, result: Option<u32>) -> Option<u32> {
        if result.is_some() {
            self.cache_hit_count
                .set(self.cache_hit_count.get().saturating_add(1));
        } else {
            self.cache_miss_count
                .set(self.cache_miss_count.get().saturating_add(1));
        }
        result
    }
}

fn build_snapshot_places(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    included_places: &BTreeSet<EntityId>,
    entities: &BTreeMap<EntityId, SnapshotEntity>,
    evidence_entities: &BTreeSet<EntityId>,
) -> BTreeMap<EntityId, SnapshotPlace> {
    let mut places: BTreeMap<EntityId, SnapshotPlace> = included_places
        .iter()
        .copied()
        .map(|place| {
            let entities = entities
                .iter()
                .filter_map(|(entity, snapshot)| {
                    (snapshot.spatial.effective_place == Some(place)).then_some(*entity)
                })
                .collect();
            let adjacent_places_with_travel_ticks = view
                .adjacent_places_with_travel_ticks(place)
                .into_iter()
                .filter(|(adjacent, _)| included_places.contains(adjacent))
                .collect();
            let tags = PlaceTag::ALL
                .into_iter()
                .filter(|tag| view.place_has_tag(place, *tag))
                .collect();
            (
                place,
                SnapshotPlace {
                    entities,
                    tags,
                    adjacent_places_with_travel_ticks,
                    bandit_camp_faction: view.locally_observed_bandit_camp_faction_at(actor, place),
                },
            )
        })
        .collect();

    for entity in evidence_entities {
        if !entities.contains_key(entity) || view.effective_place(*entity).is_some() {
            continue;
        }

        for place in included_places {
            if view.entities_at(*place).contains(entity) {
                places
                    .get_mut(place)
                    .expect("included place must exist in snapshot place map")
                    .entities
                    .insert(*entity);
                break;
            }
        }
    }

    places
}

fn build_snapshot_entity(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    entity: EntityId,
    evidence_entities: &BTreeSet<EntityId>,
    included_entities: &BTreeSet<EntityId>,
    known_entity_beliefs: &BTreeMap<EntityId, BelievedEntityState>,
) -> SnapshotEntity {
    let belief_backed =
        snapshot_entity_belief(view, actor, entity, evidence_entities, known_entity_beliefs);
    let kind = belief_backed
        .and_then(|belief| belief.believed_kind)
        .or_else(|| view.entity_kind(entity));
    let alive = belief_backed.map_or_else(|| view.is_alive(entity), |belief| belief.alive);
    let dead = belief_backed.map_or_else(|| view.is_dead(entity), |belief| !belief.alive);
    let incapacitated = belief_backed.map_or_else(
        || view.is_incapacitated(entity),
        |belief| !belief.alive && !belief.wounds.is_empty(),
    );
    let effective_place = belief_backed
        .and_then(|belief| belief.last_known_place)
        .or_else(|| view.effective_place(entity));
    let in_transit_state = view.in_transit_state(entity);
    let patrol_route = view.patrol_route(entity);
    let co_located_with_actor = view
        .effective_place(actor)
        .zip(view.effective_place(entity))
        .is_some_and(|(actor_place, entity_place)| actor_place == entity_place);
    let direct_container = if belief_backed.is_none() || co_located_with_actor {
        view.direct_container(entity)
    } else {
        None
    };
    let believed_holder = match view.believed_holder_of(entity) {
        BeliefRead::Known(holder) | BeliefRead::Stale(holder) => Some(holder.value),
        BeliefRead::Unknown => None,
    };
    let direct_possessor = if belief_backed.is_none() || co_located_with_actor {
        view.direct_possessor(entity)
    } else if believed_holder == Some(actor) {
        None
    } else {
        believed_holder
    };
    let owner = match view.believed_owner_of(entity) {
        BeliefRead::Known(owner) | BeliefRead::Stale(owner) => Some(owner.value),
        BeliefRead::Unknown => None,
    };
    let direct_possessions = view
        .direct_possessions(entity)
        .into_iter()
        .filter(|possessed| included_entities.contains(possessed))
        .collect();
    let known_recipes = view.known_recipes(entity);
    let unique_item_counts = collect_unique_item_counts(view, entity);
    let commodity_quantities = belief_backed.map_or_else(
        || collect_commodity_quantities(view, entity),
        |belief| belief.last_known_inventory.clone(),
    );
    let item_lot_commodity = belief_backed
        .and_then(believed_item_lot_commodity)
        .or_else(|| view.item_lot_commodity(entity));
    let carry_capacity = view.carry_capacity(entity);
    let intrinsic_load = view.load_of_entity(entity).unwrap_or(LoadUnits(0));
    let item_lot_consumable_profile = view.item_lot_consumable_profile(entity);
    let workstation_tag = belief_backed
        .and_then(|belief| belief.workstation_tag)
        .or_else(|| view.workstation_tag(entity));
    let stock_storage_policy = view.stock_storage_policy(entity);
    let resource_source = belief_backed
        .and_then(|belief| belief.resource_source.clone())
        .or_else(|| view.resource_source(entity));
    let wash_basin_state = belief_backed
        .and_then(|belief| belief.wash_basin_state)
        .or_else(|| worldwake_sim::FacilityBeliefView::wash_basin_state(view, entity));
    let has_production_job = view.has_production_job(entity);
    let controllable_by_actor = view.can_control(actor, entity);
    let has_control = view.has_control(entity);
    let wounds = belief_backed.map_or_else(|| view.wounds(entity), |belief| belief.wounds.clone());
    let patrol_profile = view.patrol_profile(entity);
    let combat_profile = view.combat_profile(entity);
    let courage = belief_backed
        .and_then(|belief| belief.last_known_courage)
        .or_else(|| view.courage(entity));
    let hostile_targets = view.hostile_targets_of(entity);
    let visible_hostiles = view.visible_hostiles_for(entity);
    let current_attackers = view.current_attackers_of(entity);
    let homeostatic_needs = view.homeostatic_needs(entity);
    let drive_thresholds = view.drive_thresholds(entity);
    let metabolism_profile = view.metabolism_profile(entity);
    let disposal_profile = view.disposal_profile(entity);
    let artifact_posting_profile = view.artifact_posting_profile(entity);
    let theft_disposition_profile = view.theft_disposition_profile(entity);
    let trade_disposition_profile = view.trade_disposition_profile(entity);
    let justice_disposition_profile = view.justice_disposition_profile(entity);
    let violation_disposition_profile = view.violation_disposition_profile(entity);
    let record_data = view.record_data(entity);
    let demand_memory = view.demand_memory(entity);
    let merchandise_profile = view.merchandise_profile(entity);
    let has_sale_listing = view.has_sale_listing(entity);
    let seller_for_sale_lot = view.seller_for_sale_lot(entity);
    let reservation_ranges = view.reservation_ranges(entity);
    let facility_queue = snapshot_facility_queue(view, actor, entity);
    let office_data = view.office_data(entity);

    SnapshotEntity {
        entity: SnapshotEntityCore {
            kind,
            alive,
            dead,
            incapacitated,
        },
        spatial: SnapshotSpatial {
            effective_place,
            in_transit_state,
            patrol_route,
        },
        inventory: SnapshotInventory {
            direct_container,
            direct_possessor,
            direct_possessions,
            direct_contents: BTreeSet::new(),
            known_recipes,
            unique_item_counts,
            commodity_quantities,
            item_lot_commodity,
            carry_capacity,
            intrinsic_load,
            item_lot_consumable_profile,
        },
        combat: SnapshotCombat {
            wounds,
            patrol_profile,
            combat_profile,
            courage,
            hostile_targets,
            visible_hostiles,
            current_attackers,
        },
        social: SnapshotSocial {
            theft_disposition_profile,
        },
        economic: SnapshotEconomic {
            trade_disposition_profile,
            demand_memory,
            merchandise_profile,
            has_sale_listing,
            seller_for_sale_lot,
        },
        political: SnapshotPolitical {
            justice_disposition_profile,
            violation_disposition_profile,
            record_data,
            office_data,
        },
        temporal: SnapshotTemporal {
            reservation_ranges,
            facility_queue,
        },
        profiles: SnapshotProfiles {
            homeostatic_needs,
            drive_thresholds,
            metabolism_profile,
            disposal_profile,
            artifact_posting_profile,
        },
        facility: SnapshotFacility {
            workstation_tag,
            stock_storage_policy,
            resource_source,
            wash_basin_state,
            has_production_job,
        },
        control: SnapshotControl {
            owner,
            controllable_by_actor,
            has_control,
        },
    }
}

fn snapshot_entity_belief<'a>(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    entity: EntityId,
    evidence_entities: &BTreeSet<EntityId>,
    known_entity_beliefs: &'a BTreeMap<EntityId, BelievedEntityState>,
) -> Option<&'a BelievedEntityState> {
    if entity == actor
        || evidence_entities.contains(&entity)
        || view.entity_kind(entity) == Some(EntityKind::Place)
    {
        None
    } else {
        known_entity_beliefs.get(&entity)
    }
}

fn believed_item_lot_commodity(belief: &BelievedEntityState) -> Option<CommodityKind> {
    (belief.believed_kind == Some(EntityKind::ItemLot) && belief.last_known_inventory.len() == 1)
        .then(|| belief.last_known_inventory.keys().next().copied())
        .flatten()
}

fn snapshot_facility_queue(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    entity: EntityId,
) -> Option<SnapshotFacilityQueue> {
    let has_policy = view.has_contention_policy(entity);
    let actor_queue_position = view.facility_queue_position(entity, actor);
    let active_grant = view.facility_grant(entity).cloned();
    (has_policy || actor_queue_position.is_some() || active_grant.is_some()).then_some(
        SnapshotFacilityQueue {
            actor_queue_position,
            active_grant,
        },
    )
}

fn collect_unique_item_counts(
    view: &dyn RuntimeBeliefView,
    entity: EntityId,
) -> BTreeMap<UniqueItemKind, u32> {
    UniqueItemKind::ALL
        .into_iter()
        .filter_map(|kind| {
            let count = view.unique_item_count(entity, kind);
            (count > 0).then_some((kind, count))
        })
        .collect()
}

fn collect_commodity_quantities(
    view: &dyn RuntimeBeliefView,
    entity: EntityId,
) -> BTreeMap<CommodityKind, Quantity> {
    if view.entity_kind(entity) == Some(EntityKind::ItemLot) {
        let Some(kind) = view.item_lot_commodity(entity) else {
            return BTreeMap::new();
        };
        let quantity = view.commodity_quantity(entity, kind);
        return if quantity > Quantity(0) {
            BTreeMap::from([(kind, quantity)])
        } else {
            BTreeMap::new()
        };
    }

    CommodityKind::ALL
        .into_iter()
        .filter_map(|kind| {
            let quantity = view.commodity_quantity(entity, kind);
            (quantity > Quantity(0)).then_some((kind, quantity))
        })
        .collect()
}

#[must_use]
pub fn build_planning_snapshot(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    evidence_entities: &BTreeSet<EntityId>,
    evidence_places: &BTreeSet<EntityId>,
    travel_horizon: u8,
) -> PlanningSnapshot {
    PlanningSnapshot::build(
        view,
        actor,
        evidence_entities,
        evidence_places,
        travel_horizon,
    )
}

#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn build_planning_snapshot_with_blocked_facility_uses(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    evidence_entities: &BTreeSet<EntityId>,
    evidence_places: &BTreeSet<EntityId>,
    travel_horizon: u8,
    blocked_memory: &BlockerMemory,
    current_tick: Tick,
    relevant_ops: &[PlannerOpKind],
) -> PlanningSnapshot {
    PlanningSnapshot::build_with_blocked_facility_uses(
        view,
        actor,
        evidence_entities,
        evidence_places,
        travel_horizon,
        &blocked_facility_uses(blocked_memory, current_tick),
        SnapshotEntityFilter::from_relevant_ops(relevant_ops),
    )
}

fn blocked_facility_uses(
    blocked_memory: &BlockerMemory,
    current_tick: Tick,
) -> BTreeSet<(EntityId, ActionDefId)> {
    blocked_memory
        .intents
        .values()
        .filter(|intent| intent.expires_tick > current_tick)
        .filter(|intent| intent.blocking_fact == BlockingFact::ExclusiveFacilityUnavailable)
        .filter_map(|intent| {
            intent
                .scope
                .exact_target()
                .zip(intent.scope.exact_action_def())
        })
        .collect()
}

fn collect_places(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    evidence_entities: &BTreeSet<EntityId>,
    evidence_places: &BTreeSet<EntityId>,
    travel_horizon: u8,
) -> BTreeSet<EntityId> {
    let mut included = evidence_places.clone();

    if let Some(actor_place) = view.effective_place(actor) {
        included.insert(actor_place);
        let mut frontier = VecDeque::from([(actor_place, 0u8)]);
        let mut visited = BTreeSet::from([actor_place]);
        while let Some((place, depth)) = frontier.pop_front() {
            if depth >= travel_horizon {
                continue;
            }
            for (adjacent, _) in view.adjacent_places_with_travel_ticks(place) {
                if visited.insert(adjacent) {
                    included.insert(adjacent);
                    frontier.push_back((adjacent, depth.saturating_add(1)));
                }
            }
        }
    }

    for entity in evidence_entities {
        if let Some(place) = view.effective_place(*entity) {
            included.insert(place);
        }
        if let Some(transit) = view.in_transit_state(*entity) {
            included.insert(transit.origin);
            included.insert(transit.destination);
        }
    }
    included
}

fn collect_entities(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    evidence_entities: &BTreeSet<EntityId>,
    included_places: &BTreeSet<EntityId>,
    filter: SnapshotEntityFilter,
    known_entity_beliefs: &BTreeMap<EntityId, BelievedEntityState>,
) -> BTreeSet<EntityId> {
    let mut included = BTreeSet::from([actor]);
    included.extend(evidence_entities.iter().copied());
    included.extend(included_places.iter().copied());
    let actor_place = view.effective_place(actor);

    for place in included_places {
        let mut filtered = if Some(*place) == actor_place {
            view.entities_at(*place)
                .into_iter()
                .filter(|entity| {
                    view.entity_kind(*entity)
                        .is_some_and(|kind| filter.includes(kind, view.is_alive(*entity)))
                })
                .collect::<Vec<_>>()
        } else {
            known_entity_beliefs
                .iter()
                .filter_map(|(entity, belief)| {
                    (belief.last_known_place == Some(*place)).then_some((*entity, belief))
                })
                .filter_map(|(entity, belief)| {
                    let kind = belief.believed_kind.or_else(|| view.entity_kind(entity))?;
                    filter.includes(kind, belief.alive).then_some(entity)
                })
                .collect::<Vec<_>>()
        };
        filtered.sort_by_key(|entity| {
            (
                Reverse(observed_tick_for(*entity, known_entity_beliefs)),
                Reverse(*entity),
            )
        });
        included.extend(filtered);
    }

    let mut frontier: VecDeque<_> = included.iter().copied().collect();
    while let Some(entity) = frontier.pop_front() {
        for related in view.direct_possessions(entity) {
            if included.insert(related) {
                frontier.push_back(related);
            }
        }
        if let Some(container) = view.direct_container(entity)
            && included.insert(container)
        {
            frontier.push_back(container);
        }
        if let Some(possessor) = view.direct_possessor(entity)
            && included.insert(possessor)
        {
            frontier.push_back(possessor);
        }
    }
    included
}

fn observed_tick_for(
    entity: EntityId,
    known_entity_beliefs: &BTreeMap<EntityId, BelievedEntityState>,
) -> Tick {
    known_entity_beliefs.get(&entity).map_or(Tick(0), |belief| {
        belief.last_observed_tick().unwrap_or(Tick(0))
    })
}

#[cfg(test)]
mod tests {
    use super::{
        SnapshotEntityFilter, SnapshotFacilityQueue, build_planning_snapshot,
        build_planning_snapshot_with_blocked_facility_uses,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        ActionDefId, ArtifactPostingProfile, BeliefConfidencePolicy, BelievedEntityState,
        BlockerMemory, CombatProfile, CommodityConsumableProfile, CommodityKind, ContentionGrant,
        DemandObservation, DisposalProfile, DriveThresholds, EligibilityRule, EntityId, EntityKind,
        HomeostaticNeeds, InTransitOnEdge, InstitutionalBeliefRead, LoadUnits, MerchandiseProfile,
        MetabolismProfile, OfficeData, PatrolProfile, PatrolRoute, Quantity, RecipeId,
        ResourceSource, SuccessionLaw, TellMemoryKey, TellProfile, Tick, TickRange,
        ToldBeliefMemory, TradeDispositionProfile, UniqueItemKind, WorkstationTag, Wound,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionDuration, ActionHandlerRegistry, ActionPayload, ControlBeliefView,
        DurationExpr, EntityBeliefView, PoliticalBeliefView, ProfileBeliefView, RuntimeBeliefView,
        SpatialBeliefView, TemporalBeliefView, get_affordances_for_defs,
    };

    use crate::{PlannerOpKind, PlanningState};

    type SupportDeclarationBeliefs =
        BTreeMap<EntityId, Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)>>;

    struct StubBeliefView {
        current_tick: Tick,
        alive: BTreeMap<EntityId, bool>,
        dead: BTreeSet<EntityId>,
        kinds: BTreeMap<EntityId, EntityKind>,
        effective_places: BTreeMap<EntityId, EntityId>,
        entities_at: BTreeMap<EntityId, Vec<EntityId>>,
        adjacent: BTreeMap<EntityId, Vec<(EntityId, NonZeroU32)>>,
        known_entity_beliefs: BTreeMap<EntityId, Vec<(EntityId, BelievedEntityState)>>,
        direct_possessions: BTreeMap<EntityId, Vec<EntityId>>,
        direct_containers: BTreeMap<EntityId, EntityId>,
        direct_possessors: BTreeMap<EntityId, EntityId>,
        commodity_quantities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        item_lot_commodities: BTreeMap<EntityId, CommodityKind>,
        carry_capacities: BTreeMap<EntityId, LoadUnits>,
        entity_loads: BTreeMap<EntityId, LoadUnits>,
        exclusive_facilities: BTreeSet<EntityId>,
        facility_queue_positions: BTreeMap<(EntityId, EntityId), u32>,
        facility_grants: BTreeMap<EntityId, ContentionGrant>,
        tell_profiles: BTreeMap<EntityId, TellProfile>,
        disposal_profiles: BTreeMap<EntityId, DisposalProfile>,
        artifact_posting_profiles: BTreeMap<EntityId, ArtifactPostingProfile>,
        told_beliefs: BTreeMap<EntityId, Vec<(TellMemoryKey, ToldBeliefMemory)>>,
        confidence_policies: BTreeMap<EntityId, BeliefConfidencePolicy>,
        patrol_routes: BTreeMap<EntityId, PatrolRoute>,
        office_holder_beliefs: BTreeMap<EntityId, InstitutionalBeliefRead<Option<EntityId>>>,
        support_declaration_beliefs: SupportDeclarationBeliefs,
        office_data: BTreeMap<EntityId, OfficeData>,
    }

    impl Default for StubBeliefView {
        fn default() -> Self {
            Self {
                current_tick: Tick(0),
                alive: BTreeMap::new(),
                dead: BTreeSet::new(),
                kinds: BTreeMap::new(),
                effective_places: BTreeMap::new(),
                entities_at: BTreeMap::new(),
                adjacent: BTreeMap::new(),
                known_entity_beliefs: BTreeMap::new(),
                direct_possessions: BTreeMap::new(),
                direct_containers: BTreeMap::new(),
                direct_possessors: BTreeMap::new(),
                commodity_quantities: BTreeMap::new(),
                item_lot_commodities: BTreeMap::new(),
                carry_capacities: BTreeMap::new(),
                entity_loads: BTreeMap::new(),
                exclusive_facilities: BTreeSet::new(),
                facility_queue_positions: BTreeMap::new(),
                facility_grants: BTreeMap::new(),
                tell_profiles: BTreeMap::new(),
                disposal_profiles: BTreeMap::new(),
                artifact_posting_profiles: BTreeMap::new(),
                told_beliefs: BTreeMap::new(),
                confidence_policies: BTreeMap::new(),
                patrol_routes: BTreeMap::new(),
                office_holder_beliefs: BTreeMap::new(),
                support_declaration_beliefs: BTreeMap::new(),
                office_data: BTreeMap::new(),
            }
        }
    }

    impl ControlBeliefView for StubBeliefView {
        fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
            actor == entity
        }

        fn has_control(&self, entity: EntityId) -> bool {
            self.kinds.get(&entity) == Some(&EntityKind::Agent)
        }
    }

    impl worldwake_sim::BelievedAuthorityView for StubBeliefView {
        fn believed_office_holder(
            &self,
            office: EntityId,
        ) -> worldwake_sim::BeliefRead<Option<EntityId>> {
            match self
                .office_holder_beliefs
                .get(&office)
                .cloned()
                .unwrap_or(InstitutionalBeliefRead::Unknown)
            {
                InstitutionalBeliefRead::Certain(holder) => {
                    worldwake_sim::BeliefRead::known_certain(holder, Tick(0))
                }
                InstitutionalBeliefRead::Conflicted(_) | InstitutionalBeliefRead::Unknown => {
                    worldwake_sim::BeliefRead::Unknown
                }
            }
        }
    }

    impl EntityBeliefView for StubBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.get(&entity).copied().unwrap_or(false)
        }

        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.kinds.get(&entity).copied()
        }

        fn is_dead(&self, entity: EntityId) -> bool {
            self.dead.contains(&entity) || self.alive.get(&entity) == Some(&false)
        }

        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }

        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
    }

    impl ProfileBeliefView for StubBeliefView {
        fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
            None
        }

        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
        }

        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            None
        }

        fn disposal_profile(&self, agent: EntityId) -> Option<DisposalProfile> {
            self.disposal_profiles.get(&agent).copied()
        }

        fn artifact_posting_profile(&self, agent: EntityId) -> Option<ArtifactPostingProfile> {
            self.artifact_posting_profiles.get(&agent).cloned()
        }
    }

    impl SpatialBeliefView for StubBeliefView {
        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.effective_places.get(&entity).copied()
        }

        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }

        fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
            self.entities_at.get(&place).cloned().unwrap_or_default()
        }

        fn adjacent_places(&self, place: EntityId) -> Vec<EntityId> {
            self.adjacent_places_with_travel_ticks(place)
                .into_iter()
                .map(|(adjacent, _)| adjacent)
                .collect()
        }

        fn patrol_route(&self, agent: EntityId) -> Option<PatrolRoute> {
            self.patrol_routes.get(&agent).cloned()
        }

        fn route_exists(&self, _from: EntityId, _to: EntityId) -> bool {
            false
        }

        fn in_transit_state(&self, _entity: EntityId) -> Option<InTransitOnEdge> {
            None
        }

        fn adjacent_places_with_travel_ticks(
            &self,
            place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            self.adjacent.get(&place).cloned().unwrap_or_default()
        }
    }

    impl TemporalBeliefView for StubBeliefView {
        fn current_tick(&self) -> Tick {
            self.current_tick
        }

        fn has_contention_policy(&self, entity: EntityId) -> bool {
            self.exclusive_facilities.contains(&entity)
        }

        fn facility_queue_position(&self, facility: EntityId, actor: EntityId) -> Option<u32> {
            self.facility_queue_positions
                .get(&(facility, actor))
                .copied()
        }

        fn facility_grant(&self, facility: EntityId) -> Option<&ContentionGrant> {
            self.facility_grants.get(&facility)
        }

        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
            false
        }

        fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> {
            Vec::new()
        }

        fn estimate_duration(
            &self,
            _actor: EntityId,
            _duration: &DurationExpr,
            _targets: &[EntityId],
            _payload: &ActionPayload,
        ) -> Option<ActionDuration> {
            None
        }
    }

    impl RuntimeBeliefView for StubBeliefView {}
    impl worldwake_sim::LocalPhysicalObservationView for StubBeliefView {}

    impl worldwake_sim::SocialBeliefView for StubBeliefView {
        fn belief_confidence_policy(&self, agent: EntityId) -> BeliefConfidencePolicy {
            self.confidence_policies
                .get(&agent)
                .copied()
                .unwrap_or_default()
        }

        fn intention_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::IntentionDispositionProfile> {
            None
        }

        fn tell_profile(&self, agent: EntityId) -> Option<TellProfile> {
            self.tell_profiles.get(&agent).copied()
        }

        fn told_belief_memories(&self, agent: EntityId) -> Vec<(TellMemoryKey, ToldBeliefMemory)> {
            self.told_beliefs.get(&agent).cloned().unwrap_or_default()
        }

        fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
            self.known_entity_beliefs
                .get(&agent)
                .cloned()
                .unwrap_or_default()
        }
    }

    impl worldwake_sim::PoliticalBeliefView for StubBeliefView {
        fn office_data(&self, office: EntityId) -> Option<OfficeData> {
            self.office_data.get(&office).cloned()
        }

        fn believed_support_declarations_for_office(
            &self,
            office: EntityId,
        ) -> Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)> {
            self.support_declaration_beliefs
                .get(&office)
                .cloned()
                .unwrap_or_default()
        }
    }

    impl worldwake_sim::CombatBeliefView for StubBeliefView {
        fn combat_profile(&self, _agent: EntityId) -> Option<CombatProfile> {
            None
        }
        fn wounds(&self, _agent: EntityId) -> Vec<Wound> {
            Vec::new()
        }
        fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn patrol_profile(&self, _agent: EntityId) -> Option<PatrolProfile> {
            None
        }
        fn has_wounds(&self, _entity: EntityId) -> bool {
            false
        }
    }

    impl worldwake_sim::EconomicBeliefView for StubBeliefView {
        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
            None
        }
        fn controlled_commodity_quantity_at_place(
            &self,
            _actor: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Quantity {
            Quantity(0)
        }
        fn local_controlled_lots_for(
            &self,
            _actor: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn listed_sale_lots_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn seller_for_sale_lot(&self, _lot: EntityId) -> Option<EntityId> {
            None
        }
        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            Vec::new()
        }
        fn merchandise_profile(&self, _agent: EntityId) -> Option<MerchandiseProfile> {
            None
        }
    }

    impl worldwake_sim::InventoryBeliefView for StubBeliefView {
        fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId> {
            self.direct_possessions
                .get(&holder)
                .cloned()
                .unwrap_or_default()
        }

        fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool {
            false
        }

        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 {
            0
        }

        fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity {
            self.commodity_quantities
                .get(&(holder, kind))
                .copied()
                .unwrap_or(Quantity(0))
        }

        fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind> {
            self.item_lot_commodities.get(&entity).copied()
        }

        fn item_lot_consumable_profile(
            &self,
            _entity: EntityId,
        ) -> Option<CommodityConsumableProfile> {
            None
        }

        fn direct_container(&self, entity: EntityId) -> Option<EntityId> {
            self.direct_containers.get(&entity).copied()
        }

        fn direct_possessor(&self, entity: EntityId) -> Option<EntityId> {
            self.direct_possessors.get(&entity).copied()
        }

        fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.carry_capacities.get(&entity).copied()
        }

        fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.entity_loads.get(&entity).copied()
        }

        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
            Vec::new()
        }
    }

    impl worldwake_sim::FacilityBeliefView for StubBeliefView {
        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }

        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }

        fn matching_workstations_at(
            &self,
            _place: EntityId,
            _tag: WorkstationTag,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn resource_sources_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn sample_belief(alive: bool, observed_tick: u64) -> BelievedEntityState {
        BelievedEntityState {
            believed_kind: None,
            last_known_place: None,
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            ..BelievedEntityState::single_observation_defaults(
                Tick(observed_tick),
                worldwake_core::PerceptionSource::DirectObservation,
            )
        }
    }

    #[test]
    fn snapshot_entity_filter_derives_item_and_institutional_flags() {
        let unfiltered = SnapshotEntityFilter::from_relevant_ops(&[]);
        assert!(unfiltered.needs_items());
        assert!(unfiltered.needs_institutional());

        let travel_only = SnapshotEntityFilter::from_relevant_ops(&[PlannerOpKind::Travel]);
        assert!(!travel_only.needs_items());
        assert!(!travel_only.needs_institutional());

        let trade = SnapshotEntityFilter::from_relevant_ops(&[PlannerOpKind::Trade]);
        assert!(trade.needs_items());
        assert!(!trade.needs_institutional());

        let institutional =
            SnapshotEntityFilter::from_relevant_ops(&[PlannerOpKind::ConsultRecord]);
        assert!(!institutional.needs_items());
        assert!(institutional.needs_institutional());
    }

    #[test]
    fn snapshot_filter_excludes_items_for_travel_only_goal() {
        let actor = entity(1);
        let place = entity(10);
        let item = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(item, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(item, EntityKind::ItemLot);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(item, place);
        view.entities_at.insert(place, vec![actor, item]);
        view.known_entity_beliefs
            .insert(actor, vec![(item, sample_belief(true, 3))]);

        let snapshot = build_planning_snapshot_with_blocked_facility_uses(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::new(),
            0,
            &BlockerMemory::default(),
            Tick(0),
            &[PlannerOpKind::Travel],
        );

        assert!(!snapshot.entities.contains_key(&item));
    }

    #[test]
    fn snapshot_filter_includes_items_for_trade_goal() {
        let actor = entity(1);
        let place = entity(10);
        let item = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(item, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(item, EntityKind::ItemLot);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(item, place);
        view.entities_at.insert(place, vec![actor, item]);
        view.known_entity_beliefs
            .insert(actor, vec![(item, sample_belief(true, 3))]);

        let snapshot = build_planning_snapshot_with_blocked_facility_uses(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::new(),
            0,
            &BlockerMemory::default(),
            Tick(0),
            &[PlannerOpKind::Trade],
        );

        assert!(snapshot.entities.contains_key(&item));
    }

    #[test]
    fn snapshot_filter_excludes_dead_agents_without_loot() {
        let actor = entity(1);
        let place = entity(10);
        let corpse = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(corpse, false);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(corpse, EntityKind::Agent);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(corpse, place);
        view.entities_at.insert(place, vec![actor, corpse]);
        view.known_entity_beliefs
            .insert(actor, vec![(corpse, sample_belief(false, 3))]);

        let snapshot = build_planning_snapshot_with_blocked_facility_uses(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::new(),
            0,
            &BlockerMemory::default(),
            Tick(0),
            &[PlannerOpKind::Travel],
        );

        assert!(!snapshot.entities.contains_key(&corpse));
    }

    #[test]
    fn snapshot_filter_includes_dead_agents_with_loot() {
        let actor = entity(1);
        let place = entity(10);
        let corpse = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(corpse, false);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(corpse, EntityKind::Agent);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(corpse, place);
        view.entities_at.insert(place, vec![actor, corpse]);
        view.known_entity_beliefs
            .insert(actor, vec![(corpse, sample_belief(false, 3))]);

        let snapshot = build_planning_snapshot_with_blocked_facility_uses(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::new(),
            0,
            &BlockerMemory::default(),
            Tick(0),
            &[PlannerOpKind::Loot],
        );

        assert!(snapshot.entities.contains_key(&corpse));
    }

    #[test]
    fn snapshot_filter_excludes_records_without_institutional_ops() {
        let actor = entity(1);
        let place = entity(10);
        let record = entity(20);
        let artifact = entity(21);
        let faction = entity(22);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        for entity in [record, artifact, faction] {
            view.alive.insert(entity, true);
            view.effective_places.insert(entity, place);
        }
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(record, EntityKind::Record);
        view.kinds.insert(artifact, EntityKind::SocialArtifact);
        view.kinds.insert(faction, EntityKind::Faction);
        view.effective_places.insert(actor, place);
        view.entities_at
            .insert(place, vec![actor, record, artifact, faction]);
        view.known_entity_beliefs.insert(
            actor,
            vec![
                (record, sample_belief(true, 1)),
                (artifact, sample_belief(true, 2)),
                (faction, sample_belief(true, 3)),
            ],
        );

        let snapshot = build_planning_snapshot_with_blocked_facility_uses(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::new(),
            0,
            &BlockerMemory::default(),
            Tick(0),
            &[PlannerOpKind::Travel],
        );

        assert!(!snapshot.entities.contains_key(&record));
        assert!(!snapshot.entities.contains_key(&artifact));
        assert!(!snapshot.entities.contains_key(&faction));
    }

    #[test]
    fn snapshot_filter_includes_records_with_institutional_ops() {
        let actor = entity(1);
        let place = entity(10);
        let record = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(record, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(record, EntityKind::Record);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(record, place);
        view.entities_at.insert(place, vec![actor, record]);
        view.known_entity_beliefs
            .insert(actor, vec![(record, sample_belief(true, 4))]);

        let snapshot = build_planning_snapshot_with_blocked_facility_uses(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::new(),
            0,
            &BlockerMemory::default(),
            Tick(0),
            &[PlannerOpKind::ConsultRecord],
        );

        assert!(snapshot.entities.contains_key(&record));
    }

    #[test]
    fn snapshot_filter_includes_records_for_report_found() {
        let actor = entity(1);
        let place = entity(10);
        let record = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(record, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(record, EntityKind::Record);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(record, place);
        view.entities_at.insert(place, vec![actor, record]);
        view.known_entity_beliefs
            .insert(actor, vec![(record, sample_belief(true, 4))]);

        let snapshot = build_planning_snapshot_with_blocked_facility_uses(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::new(),
            0,
            &BlockerMemory::default(),
            Tick(0),
            &[PlannerOpKind::ReportFound],
        );

        assert!(snapshot.entities.contains_key(&record));
    }

    #[test]
    fn snapshot_includes_all_believed_entities_at_place() {
        let actor = entity(1);
        let place = entity(10);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.effective_places.insert(actor, place);

        let mut entities = vec![actor];
        let mut beliefs = Vec::new();
        for slot in 20..80 {
            let item = entity(slot);
            view.alive.insert(item, true);
            view.kinds.insert(item, EntityKind::ItemLot);
            view.effective_places.insert(item, place);
            entities.push(item);
            beliefs.push((item, sample_belief(true, u64::from(slot))));
        }
        view.entities_at.insert(place, entities);
        view.known_entity_beliefs.insert(actor, beliefs);

        let snapshot = build_planning_snapshot_with_blocked_facility_uses(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::new(),
            0,
            &BlockerMemory::default(),
            Tick(0),
            &[PlannerOpKind::Trade],
        );

        let item_count = snapshot
            .entities
            .keys()
            .filter(|entity| view.kinds.get(entity) == Some(&EntityKind::ItemLot))
            .count();
        assert_eq!(item_count, 60);
    }

    #[test]
    fn snapshot_without_per_place_cap_keeps_older_believed_entities() {
        let actor = entity(1);
        let place = entity(10);
        let older = entity(20);
        let newer = entity(21);
        let tied_lower = entity(22);
        let tied_higher = entity(23);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.effective_places.insert(actor, place);
        for entity in [older, newer, tied_lower, tied_higher] {
            view.alive.insert(entity, true);
            view.kinds.insert(entity, EntityKind::ItemLot);
            view.effective_places.insert(entity, place);
        }
        view.entities_at
            .insert(place, vec![actor, older, newer, tied_lower, tied_higher]);
        view.known_entity_beliefs.insert(
            actor,
            vec![
                (older, sample_belief(true, 1)),
                (newer, sample_belief(true, 9)),
                (tied_lower, sample_belief(true, 5)),
                (tied_higher, sample_belief(true, 5)),
            ],
        );

        let snapshot = build_planning_snapshot_with_blocked_facility_uses(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::new(),
            0,
            &BlockerMemory::default(),
            Tick(0),
            &[PlannerOpKind::Trade],
        );

        assert!(snapshot.entities.contains_key(&newer));
        assert!(snapshot.entities.contains_key(&tied_higher));
        assert!(snapshot.entities.contains_key(&tied_lower));
        assert!(snapshot.entities.contains_key(&older));
    }

    #[test]
    fn snapshot_filter_always_includes_evidence_entities() {
        let actor = entity(1);
        let place = entity(10);
        let evidence_item = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(evidence_item, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(evidence_item, EntityKind::ItemLot);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(evidence_item, place);
        view.entities_at.insert(place, vec![actor, evidence_item]);
        view.known_entity_beliefs
            .insert(actor, vec![(evidence_item, sample_belief(true, 4))]);

        let snapshot = build_planning_snapshot_with_blocked_facility_uses(
            &view,
            actor,
            &BTreeSet::from([evidence_item]),
            &BTreeSet::new(),
            0,
            &BlockerMemory::default(),
            Tick(0),
            &[PlannerOpKind::Travel],
        );

        assert!(snapshot.entities.contains_key(&evidence_item));
    }

    #[test]
    fn evidence_entity_with_unknown_liveness_is_not_treated_as_dead() {
        let actor = entity(1);
        let place = entity(10);
        let remote = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(remote, EntityKind::Agent);
        view.effective_places.insert(actor, place);
        view.entities_at.insert(place, vec![actor]);

        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::from([remote]), &BTreeSet::new(), 0);
        let snapshot_entity = snapshot
            .entities
            .get(&remote)
            .expect("evidence entity should be admitted");

        assert!(!snapshot_entity.entity.alive);
        assert!(
            !snapshot_entity.entity.dead,
            "unknown liveness must not satisfy combat goals as known-dead"
        );
    }

    #[test]
    fn snapshot_indexes_evidence_entity_at_place_when_effective_place_is_missing() {
        let actor = entity(1);
        let place = entity(10);
        let listener = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(listener, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(listener, EntityKind::Agent);
        view.effective_places.insert(actor, place);
        view.entities_at.insert(place, vec![actor, listener]);
        view.known_entity_beliefs
            .insert(actor, vec![(listener, sample_belief(true, 4))]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([listener]),
            &BTreeSet::new(),
            0,
        );

        let indexed_entities = snapshot
            .places
            .get(&place)
            .map(|snapshot_place| snapshot_place.entities.clone())
            .unwrap_or_default();
        assert!(indexed_entities.contains(&actor));
        assert!(
            indexed_entities.contains(&listener),
            "evidence listener should be indexed at the actor's place even when effective_place is absent"
        );
    }

    #[test]
    fn tell_affordance_surfaces_for_evidence_listener_without_effective_place() {
        let actor = entity(1);
        let place = entity(10);
        let listener = entity(20);
        let subject = entity(30);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(listener, true);
        view.alive.insert(subject, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(listener, EntityKind::Agent);
        view.kinds.insert(subject, EntityKind::ItemLot);
        view.effective_places.insert(actor, place);
        view.entities_at.insert(place, vec![actor, listener]);
        view.tell_profiles.insert(actor, TellProfile::default());
        view.tell_profiles.insert(listener, TellProfile::default());
        view.known_entity_beliefs.insert(
            actor,
            vec![
                (listener, sample_belief(true, 4)),
                (subject, sample_belief(true, 5)),
            ],
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([listener]),
            &BTreeSet::new(),
            0,
        );
        let planning_state = PlanningState::new(&snapshot);

        let mut defs = ActionDefRegistry::default();
        let mut handlers = ActionHandlerRegistry::default();
        let tell_def =
            worldwake_systems::tell_actions::register_tell_action(&mut defs, &mut handlers);
        let affordances = get_affordances_for_defs(
            &planning_state,
            actor,
            &defs,
            &handlers,
            &BTreeSet::from([tell_def]),
        );

        assert!(
            affordances
                .iter()
                .any(|affordance| affordance.bound_targets == vec![listener]),
            "tell affordance should bind the co-located listener carried only by evidence_entities"
        );
    }

    #[test]
    fn snapshot_filter_containment_walk_includes_inventory() {
        let actor = entity(1);
        let place = entity(10);
        let other_agent = entity(20);
        let carried_item = entity(21);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(other_agent, true);
        view.alive.insert(carried_item, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(other_agent, EntityKind::Agent);
        view.kinds.insert(carried_item, EntityKind::ItemLot);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(other_agent, place);
        view.entities_at.insert(place, vec![actor, other_agent]);
        view.direct_possessions
            .insert(other_agent, vec![carried_item]);
        view.direct_possessors.insert(carried_item, other_agent);
        view.known_entity_beliefs.insert(
            actor,
            vec![
                (other_agent, sample_belief(true, 4)),
                (carried_item, sample_belief(true, 5)),
            ],
        );

        let snapshot = build_planning_snapshot_with_blocked_facility_uses(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::new(),
            0,
            &BlockerMemory::default(),
            Tick(0),
            &[PlannerOpKind::Travel],
        );

        assert!(snapshot.entities.contains_key(&other_agent));
        assert!(snapshot.entities.contains_key(&carried_item));
    }

    #[test]
    fn build_snapshot_includes_actor_evidence_and_places_within_horizon() {
        let actor = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let place_c = entity(12);
        let remote_place = entity(19);
        let evidence_entity = entity(2);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(evidence_entity, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(evidence_entity, EntityKind::Agent);
        view.effective_places.insert(actor, place_a);
        view.effective_places.insert(evidence_entity, remote_place);
        view.entities_at.insert(place_a, vec![actor]);
        view.entities_at.insert(place_b, vec![]);
        view.entities_at.insert(place_c, vec![]);
        view.entities_at.insert(remote_place, vec![evidence_entity]);
        view.adjacent
            .insert(place_a, vec![(place_b, NonZeroU32::new(3).unwrap())]);
        view.adjacent.insert(
            place_b,
            vec![
                (place_a, NonZeroU32::new(3).unwrap()),
                (place_c, NonZeroU32::new(5).unwrap()),
            ],
        );
        view.adjacent
            .insert(place_c, vec![(place_b, NonZeroU32::new(5).unwrap())]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([evidence_entity]),
            &BTreeSet::new(),
            1,
        );

        assert!(snapshot.entities.contains_key(&actor));
        assert!(snapshot.entities.contains_key(&evidence_entity));
        assert!(snapshot.places.contains_key(&place_a));
        assert!(snapshot.places.contains_key(&place_b));
        assert!(snapshot.places.contains_key(&remote_place));
        assert!(!snapshot.places.contains_key(&place_c));
    }

    #[test]
    fn build_snapshot_preserves_missing_actor_tell_profile() {
        let actor = entity(1);
        let place = entity(10);
        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.effective_places.insert(actor, place);
        view.entities_at.insert(place, vec![actor]);

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 0);

        assert_eq!(snapshot.actor_tell_profile, None);
    }

    #[test]
    fn build_snapshot_preserves_present_actor_tell_profile() {
        let actor = entity(1);
        let place = entity(10);
        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.effective_places.insert(actor, place);
        view.entities_at.insert(place, vec![actor]);
        view.tell_profiles.insert(
            actor,
            TellProfile {
                max_tell_candidates: 4,
                max_relay_chain_len: 2,
                ..TellProfile::default()
            },
        );

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 0);

        assert_eq!(
            snapshot.actor_tell_profile,
            view.tell_profiles.get(&actor).copied()
        );
    }

    #[test]
    fn build_snapshot_preserves_actor_told_belief_memory() {
        let actor = entity(1);
        let listener = entity(2);
        let subject = entity(3);
        let place = entity(10);
        let mut view = StubBeliefView {
            current_tick: Tick(8),
            ..StubBeliefView::default()
        };
        view.alive.insert(actor, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.effective_places.insert(actor, place);
        view.entities_at.insert(place, vec![actor]);
        view.tell_profiles.insert(actor, TellProfile::default());
        view.told_beliefs.insert(
            actor,
            vec![(
                TellMemoryKey {
                    counterparty: listener,
                    topic: worldwake_core::TellTopic::EntityBelief { subject },
                },
                ToldBeliefMemory {
                    shared_state: worldwake_core::SharedTellState::EntityBelief(
                        worldwake_core::to_shared_belief_snapshot(&BelievedEntityState {
                            believed_kind: None,
                            last_known_place: Some(place),
                            last_known_inventory: BTreeMap::from([(
                                CommodityKind::Bread,
                                Quantity(1),
                            )]),
                            workstation_tag: None,
                            resource_source: None,
                            alive: true,
                            wounds: Vec::new(),
                            last_known_courage: None,
                            believed_activity: None,
                            believed_artifact: None,
                            believed_contention: None,
                            believed_evidence: None,
                            ..BelievedEntityState::single_observation_defaults(
                                Tick(6),
                                worldwake_core::PerceptionSource::DirectObservation,
                            )
                        }),
                    ),
                    told_tick: Tick(7),
                },
            )],
        );

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 0);

        assert_eq!(snapshot.current_tick, Tick(8));
        assert_eq!(
            snapshot
                .actor_told_beliefs
                .get(&TellMemoryKey {
                    counterparty: listener,
                    topic: worldwake_core::TellTopic::EntityBelief { subject },
                })
                .map(|memory| memory.told_tick),
            Some(Tick(7))
        );
    }

    #[test]
    fn build_snapshot_does_not_pull_in_unreachable_places_without_evidence() {
        let actor = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let place_c = entity(12);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.effective_places.insert(actor, place_a);
        view.entities_at.insert(place_a, vec![actor]);
        view.entities_at.insert(place_b, vec![]);
        view.entities_at.insert(place_c, vec![]);
        view.adjacent
            .insert(place_a, vec![(place_b, NonZeroU32::new(1).unwrap())]);
        view.adjacent.insert(
            place_b,
            vec![
                (place_a, NonZeroU32::new(1).unwrap()),
                (place_c, NonZeroU32::new(1).unwrap()),
            ],
        );
        view.adjacent
            .insert(place_c, vec![(place_b, NonZeroU32::new(1).unwrap())]);

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);

        assert!(snapshot.places.contains_key(&place_a));
        assert!(snapshot.places.contains_key(&place_b));
        assert!(!snapshot.places.contains_key(&place_c));
    }

    #[test]
    fn build_snapshot_excludes_remote_unbelieved_facility_within_horizon() {
        let actor = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let basin = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(basin, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place_a, EntityKind::Place);
        view.kinds.insert(place_b, EntityKind::Place);
        view.kinds.insert(basin, EntityKind::Facility);
        view.effective_places.insert(actor, place_a);
        view.effective_places.insert(basin, place_b);
        view.entities_at.insert(place_a, vec![actor]);
        view.entities_at.insert(place_b, vec![basin]);
        view.adjacent
            .insert(place_a, vec![(place_b, NonZeroU32::new(1).unwrap())]);
        view.adjacent
            .insert(place_b, vec![(place_a, NonZeroU32::new(1).unwrap())]);

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);

        assert!(!snapshot.entities.contains_key(&basin));
        assert!(
            !snapshot
                .places
                .get(&place_b)
                .expect("adjacent place should still be in snapshot")
                .entities
                .contains(&basin)
        );
    }

    #[test]
    fn build_snapshot_uses_belief_summary_for_remote_facility_visibility() {
        let actor = entity(1);
        let place_a = entity(10);
        let believed_place = entity(11);
        let authoritative_place = entity(12);
        let basin = entity(20);
        let source = ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(7),
            max_quantity: Quantity(7),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
        };

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(basin, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place_a, EntityKind::Place);
        view.kinds.insert(believed_place, EntityKind::Place);
        view.kinds.insert(authoritative_place, EntityKind::Place);
        view.kinds.insert(basin, EntityKind::Facility);
        view.effective_places.insert(actor, place_a);
        view.effective_places.insert(basin, authoritative_place);
        view.entities_at.insert(place_a, vec![actor]);
        view.entities_at.insert(believed_place, vec![]);
        view.entities_at.insert(authoritative_place, vec![basin]);
        view.adjacent.insert(
            place_a,
            vec![
                (believed_place, NonZeroU32::new(1).unwrap()),
                (authoritative_place, NonZeroU32::new(1).unwrap()),
            ],
        );
        view.adjacent
            .insert(believed_place, vec![(place_a, NonZeroU32::new(1).unwrap())]);
        view.adjacent.insert(
            authoritative_place,
            vec![(place_a, NonZeroU32::new(1).unwrap())],
        );
        view.known_entity_beliefs.insert(
            actor,
            vec![(
                basin,
                BelievedEntityState {
                    believed_kind: Some(EntityKind::Facility),
                    last_known_place: Some(believed_place),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: Some(WorkstationTag::WashBasin),
                    resource_source: Some(source.clone()),
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: None,
                    believed_contention: None,
                    believed_evidence: None,
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(4),
                        worldwake_core::PerceptionSource::DirectObservation,
                    )
                },
            )],
        );

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);

        assert_eq!(
            snapshot
                .entities
                .get(&basin)
                .map(|entity| entity.spatial.effective_place),
            Some(Some(believed_place))
        );
        assert_eq!(
            snapshot
                .entities
                .get(&basin)
                .and_then(|entity| entity.facility.workstation_tag),
            Some(WorkstationTag::WashBasin)
        );
        assert_eq!(
            snapshot
                .entities
                .get(&basin)
                .and_then(|entity| entity.facility.resource_source.clone()),
            Some(source)
        );
        assert!(
            snapshot
                .places
                .get(&believed_place)
                .expect("believed place should be in snapshot")
                .entities
                .contains(&basin)
        );
        assert!(
            !snapshot
                .places
                .get(&authoritative_place)
                .expect("authoritative place should remain in topology snapshot")
                .entities
                .contains(&basin)
        );
    }

    #[test]
    fn build_snapshot_captures_carry_capacity_and_intrinsic_load() {
        let actor = entity(1);
        let place = entity(10);
        let lot = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(place, true);
        view.alive.insert(lot, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(lot, EntityKind::ItemLot);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(lot, place);
        view.entities_at.insert(place, vec![actor, lot]);
        view.carry_capacities.insert(actor, LoadUnits(9));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.entity_loads.insert(lot, LoadUnits(6));

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 0);

        assert_eq!(
            snapshot
                .entities
                .get(&actor)
                .and_then(|entity| entity.inventory.carry_capacity),
            Some(LoadUnits(9))
        );
        assert_eq!(
            snapshot
                .entities
                .get(&lot)
                .map(|entity| entity.inventory.intrinsic_load),
            Some(LoadUnits(6))
        );
        assert_eq!(
            snapshot
                .entities
                .get(&place)
                .map(|entity| entity.inventory.intrinsic_load),
            Some(LoadUnits(0))
        );
    }

    #[test]
    fn build_snapshot_item_lot_queries_only_its_commodity_quantity() {
        let actor = entity(1);
        let place = entity(10);
        let lot = entity(20);
        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(lot, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(lot, EntityKind::ItemLot);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(lot, place);
        view.entities_at.insert(place, vec![actor, lot]);
        view.item_lot_commodities.insert(lot, CommodityKind::Bread);
        view.commodity_quantities
            .insert((lot, CommodityKind::Bread), Quantity(3));
        view.commodity_quantities
            .insert((lot, CommodityKind::Water), Quantity(99));

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 0);

        assert_eq!(
            snapshot
                .entities
                .get(&lot)
                .and_then(|entity| {
                    entity
                        .inventory
                        .commodity_quantities
                        .get(&CommodityKind::Bread)
                })
                .copied(),
            Some(Quantity(3))
        );
        assert_eq!(
            snapshot
                .entities
                .get(&lot)
                .and_then(|entity| {
                    entity
                        .inventory
                        .commodity_quantities
                        .get(&CommodityKind::Water)
                })
                .copied(),
            None
        );
    }

    #[test]
    fn build_snapshot_captures_facility_queue_position_for_planning_actor() {
        let actor = entity(1);
        let place = entity(10);
        let facility = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(place, true);
        view.alive.insert(facility, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(facility, EntityKind::Facility);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(facility, place);
        view.entities_at.insert(place, vec![actor, facility]);
        view.facility_queue_positions.insert((facility, actor), 2);

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 0);

        assert_eq!(
            snapshot
                .entities
                .get(&facility)
                .and_then(|entity| entity.temporal.facility_queue.as_ref())
                .and_then(|queue| queue.actor_queue_position),
            Some(2)
        );
    }

    #[test]
    fn build_snapshot_captures_active_facility_grant() {
        let actor = entity(1);
        let other = entity(2);
        let place = entity(10);
        let facility = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(other, true);
        view.alive.insert(place, true);
        view.alive.insert(facility, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(other, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(facility, EntityKind::Facility);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(other, place);
        view.effective_places.insert(facility, place);
        view.entities_at.insert(place, vec![actor, other, facility]);
        view.facility_grants.insert(
            facility,
            ContentionGrant {
                actor: other,
                intended_action: ActionDefId(77),
                granted_at: Tick(5),
                expires_at: Tick(8),
            },
        );

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 0);

        assert_eq!(
            snapshot
                .entities
                .get(&facility)
                .and_then(|entity| entity.temporal.facility_queue.as_ref())
                .and_then(|queue| queue.active_grant.clone()),
            Some(ContentionGrant {
                actor: other,
                intended_action: ActionDefId(77),
                granted_at: Tick(5),
                expires_at: Tick(8),
            })
        );
    }

    #[test]
    fn build_snapshot_omits_facility_queue_data_when_none_exists() {
        let actor = entity(1);
        let place = entity(10);
        let facility = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(place, true);
        view.alive.insert(facility, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(facility, EntityKind::Facility);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(facility, place);
        view.entities_at.insert(place, vec![actor, facility]);

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 0);

        assert_eq!(
            snapshot
                .entities
                .get(&facility)
                .and_then(|entity| entity.temporal.facility_queue.as_ref()),
            None
        );
    }

    // ---- Floyd-Warshall distance matrix tests ----

    /// Helper: build a snapshot with a given topology for distance matrix tests.
    fn build_chain_snapshot() -> super::PlanningSnapshot {
        // Topology: A --3--> B --5--> C (directed chain)
        let actor = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let place_c = entity(12);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.effective_places.insert(actor, place_a);
        view.entities_at.insert(place_a, vec![actor]);
        view.entities_at.insert(place_b, vec![]);
        view.entities_at.insert(place_c, vec![]);
        // Bidirectional edges: A<->B(3), B<->C(5)
        view.adjacent
            .insert(place_a, vec![(place_b, NonZeroU32::new(3).unwrap())]);
        view.adjacent.insert(
            place_b,
            vec![
                (place_a, NonZeroU32::new(3).unwrap()),
                (place_c, NonZeroU32::new(5).unwrap()),
            ],
        );
        view.adjacent
            .insert(place_c, vec![(place_b, NonZeroU32::new(5).unwrap())]);

        // travel_horizon=3 to include all places
        build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 3)
    }

    #[test]
    fn min_travel_ticks_self_is_zero() {
        let snapshot = build_chain_snapshot();
        let place_a = entity(10);
        assert_eq!(snapshot.min_travel_ticks(place_a, place_a), Some(0));
    }

    #[test]
    fn min_travel_ticks_direct_adjacent() {
        let snapshot = build_chain_snapshot();
        let place_a = entity(10);
        let place_b = entity(11);
        assert_eq!(snapshot.min_travel_ticks(place_a, place_b), Some(3));
        assert_eq!(snapshot.min_travel_ticks(place_b, place_a), Some(3));
    }

    #[test]
    fn min_travel_ticks_multi_hop() {
        let snapshot = build_chain_snapshot();
        let place_a = entity(10);
        let place_c = entity(12);
        // A -> B -> C = 3 + 5 = 8
        assert_eq!(snapshot.min_travel_ticks(place_a, place_c), Some(8));
        assert_eq!(snapshot.min_travel_ticks(place_c, place_a), Some(8));
    }

    #[test]
    fn min_travel_ticks_unreachable_place() {
        let snapshot = build_chain_snapshot();
        let place_a = entity(10);
        let unknown = entity(99);
        assert_eq!(snapshot.min_travel_ticks(place_a, unknown), None);
    }

    #[test]
    fn min_travel_ticks_to_any_returns_minimum() {
        let snapshot = build_chain_snapshot();
        let place_a = entity(10);
        let place_b = entity(11);
        let place_c = entity(12);
        // A->B=3, A->C=8 → min=3
        assert_eq!(
            snapshot.min_travel_ticks_to_any(place_a, &[place_b, place_c]),
            Some(3)
        );
    }

    #[test]
    fn min_travel_ticks_to_any_self_in_destinations() {
        let snapshot = build_chain_snapshot();
        let place_a = entity(10);
        let place_c = entity(12);
        assert_eq!(
            snapshot.min_travel_ticks_to_any(place_a, &[place_a, place_c]),
            Some(0)
        );
    }

    #[test]
    fn min_travel_ticks_to_any_empty_destinations() {
        let snapshot = build_chain_snapshot();
        let place_a = entity(10);
        assert_eq!(snapshot.min_travel_ticks_to_any(place_a, &[]), None);
    }

    #[test]
    fn snapshot_cache_counters_record_matrix_hits_and_misses() {
        let snapshot = build_chain_snapshot();
        let place_a = entity(10);
        let place_b = entity(11);
        let place_c = entity(12);
        let unknown = entity(99);

        assert_eq!(
            snapshot.snapshot_cache_counters(),
            crate::SnapshotCacheCounters::default()
        );
        assert_eq!(snapshot.min_travel_ticks(place_a, place_b), Some(3));
        assert_eq!(snapshot.min_travel_ticks(place_a, unknown), None);
        assert_eq!(
            snapshot.min_travel_ticks_to_any(place_a, &[unknown, place_c]),
            Some(8)
        );
        assert_eq!(
            snapshot.min_perceived_travel_cost_to_any(place_a, &[place_b]),
            Some(3)
        );
        assert_eq!(snapshot.min_travel_ticks(place_a, place_a), Some(0));

        assert_eq!(
            snapshot.snapshot_cache_counters(),
            crate::SnapshotCacheCounters {
                cache_hit_count: 3,
                cache_miss_count: 2,
                cache_invalidation_count: 0,
            }
        );
    }

    #[test]
    fn distance_matrix_is_deterministic() {
        let s1 = build_chain_snapshot();
        let s2 = build_chain_snapshot();
        assert_eq!(s1.shortest_travel_ticks, s2.shortest_travel_ticks);
    }

    #[test]
    fn build_snapshot_keeps_empty_facility_queue_data_for_exclusive_facility() {
        let actor = entity(1);
        let place = entity(10);
        let facility = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(place, true);
        view.alive.insert(facility, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(facility, EntityKind::Facility);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(facility, place);
        view.entities_at.insert(place, vec![actor, facility]);
        view.exclusive_facilities.insert(facility);

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 0);

        assert_eq!(
            snapshot
                .entities
                .get(&facility)
                .and_then(|entity| entity.temporal.facility_queue.as_ref()),
            Some(&SnapshotFacilityQueue {
                actor_queue_position: None,
                active_grant: None,
            })
        );
    }

    #[test]
    fn snapshot_captures_belief_backed_certain_support_declarations() {
        let actor = entity(1);
        let supporter_a = entity(2);
        let supporter_b = entity(3);
        let office = entity(100);
        let town = entity(10);

        let mut view = StubBeliefView::default();
        for &e in &[actor, supporter_a, supporter_b, office, town] {
            view.alive.insert(e, true);
        }
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(supporter_a, EntityKind::Agent);
        view.kinds.insert(supporter_b, EntityKind::Agent);
        view.kinds.insert(office, EntityKind::Office);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(supporter_a, town);
        view.effective_places.insert(supporter_b, town);
        view.effective_places.insert(office, town);
        view.entities_at
            .insert(town, vec![actor, supporter_a, supporter_b, office]);
        view.carry_capacities.insert(actor, LoadUnits(10));
        view.entity_loads.insert(actor, LoadUnits(0));

        view.support_declaration_beliefs.insert(
            office,
            vec![
                (supporter_a, InstitutionalBeliefRead::Certain(Some(actor))),
                (supporter_b, InstitutionalBeliefRead::Certain(Some(actor))),
                (
                    entity(999),
                    InstitutionalBeliefRead::Conflicted(vec![Some(actor), None]),
                ),
            ],
        );

        let mut evidence = BTreeSet::new();
        evidence.insert(office);
        let snapshot = build_planning_snapshot(&view, actor, &evidence, &BTreeSet::new(), 1);

        let declarations = snapshot.base_support_declarations_for_office(office);
        assert_eq!(declarations.len(), 2);
        assert!(declarations.contains(&(supporter_a, actor)));
        assert!(declarations.contains(&(supporter_b, actor)));

        // Non-office returns empty
        assert!(
            snapshot
                .base_support_declarations_for_office(entity(999))
                .is_empty()
        );
    }

    #[test]
    fn snapshot_captures_institutional_belief_reads() {
        let actor = entity(1);
        let supporter = entity(2);
        let office = entity(100);
        let town = entity(10);

        let mut view = StubBeliefView::default();
        for &e in &[actor, supporter, office, town] {
            view.alive.insert(e, true);
        }
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(supporter, EntityKind::Agent);
        view.kinds.insert(office, EntityKind::Office);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(supporter, town);
        view.effective_places.insert(office, town);
        view.entities_at
            .insert(town, vec![actor, supporter, office]);
        view.carry_capacities.insert(actor, LoadUnits(10));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(Some(actor)));
        view.support_declaration_beliefs.insert(
            office,
            vec![(supporter, InstitutionalBeliefRead::Certain(Some(actor)))],
        );

        let mut evidence = BTreeSet::new();
        evidence.insert(office);
        let snapshot = build_planning_snapshot(&view, actor, &evidence, &BTreeSet::new(), 1);

        assert_eq!(
            snapshot.believed_office_holder(office),
            InstitutionalBeliefRead::Certain(Some(actor))
        );
        assert_eq!(
            snapshot.believed_support_declaration(office, supporter),
            InstitutionalBeliefRead::Certain(Some(actor))
        );
        assert_eq!(
            snapshot.believed_support_declaration(office, actor),
            InstitutionalBeliefRead::Unknown
        );
    }

    #[test]
    fn snapshot_preserves_full_office_data_for_planner_semantics() {
        let actor = entity(1);
        let office = entity(100);
        let town = entity(10);
        let faction = entity(200);

        let mut view = StubBeliefView::default();
        for &entity in &[actor, office, town, faction] {
            view.alive.insert(entity, true);
        }
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(office, EntityKind::Office);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(faction, EntityKind::Faction);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(office, town);
        view.entities_at.insert(town, vec![actor, office]);
        view.carry_capacities.insert(actor, LoadUnits(10));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.office_data.insert(
            office,
            OfficeData {
                title: "Marshal".to_string(),
                seat: town,
                jurisdiction: BTreeSet::from([town]),
                succession_law: SuccessionLaw::Force,
                eligibility_rules: vec![EligibilityRule::FactionMember(faction)],
                succession_period_ticks: 19,
                vacancy_since: Some(Tick(7)),
            },
        );

        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::from([office]), &BTreeSet::new(), 0);

        assert_eq!(snapshot.office_data(office), view.office_data(office));
    }
}
