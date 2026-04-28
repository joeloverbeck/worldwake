use crate::planning_snapshot::PlanningSnapshot;
use crate::shared_collections::{SharedMap, SharedSet};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use worldwake_core::{
    ActionDefId, ActionDomain, ArtifactPostingProfile, BelievedEntityState,
    BelievedInstitutionalClaim, CombatProfile, CommodityKind, ContentionGrant, DemandObservation,
    DisposalProfile, DriveThresholds, EntityId, EntityKind, HomeostaticNeeds, InTransitOnEdge,
    InstitutionalBeliefRead, JusticeDispositionProfile, LoadUnits, MetabolismProfile, OfficeData,
    PatrolProfile, PatrolRoute, Permille, PlaceTag, Quantity, RecipeId, RecipientKnowledgeStatus,
    RecordData, ResourceSource, SharedTellState, SocialObservation, SuccessionLaw, TellMemoryKey,
    TellProfile, TellTopic, TheftDispositionProfile, TickRange, ToldBeliefMemory,
    TradeDispositionProfile, UniqueItemKind, ViolationDispositionProfile, WorkstationTag, Wound,
    load_per_unit, to_shared_belief_snapshot,
};
use worldwake_sim::{
    ActionDuration, ActionPayload, CombatBeliefView, ControlBeliefView, DurationExpr,
    EconomicBeliefView, EntityBeliefView, FacilityBeliefView, InventoryBeliefView,
    PoliticalBeliefView, ProfileBeliefView, RuntimeBeliefView, SocialBeliefView, SpatialBeliefView,
    TemporalBeliefView, estimate_duration_from_beliefs,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct HypotheticalEntityId(pub u32);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum PlanningEntityRef {
    Authoritative(EntityId),
    Hypothetical(HypotheticalEntityId),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HypotheticalEntityMeta {
    pub kind: EntityKind,
    pub item_lot_commodity: Option<CommodityKind>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct HypotheticalQueueJoin {
    intended_action: ActionDefId,
}

#[derive(Clone)]
pub struct PlanningState<'snapshot> {
    snapshot: &'snapshot PlanningSnapshot,
    entity_place_overrides: SharedMap<PlanningEntityRef, Option<EntityId>>,
    bandit_camp_faction_overrides: SharedMap<EntityId, Option<EntityId>>,
    direct_container_overrides: SharedMap<PlanningEntityRef, Option<PlanningEntityRef>>,
    direct_possessor_overrides: SharedMap<PlanningEntityRef, Option<PlanningEntityRef>>,
    resource_quantity_overrides: SharedMap<EntityId, Quantity>,
    commodity_quantity_overrides: SharedMap<(PlanningEntityRef, CommodityKind), Quantity>,
    reservation_shadows: SharedMap<EntityId, Vec<TickRange>>,
    removed_entities: SharedSet<PlanningEntityRef>,
    sale_listing_overrides: SharedMap<PlanningEntityRef, bool>,
    sale_seller_overrides: SharedMap<PlanningEntityRef, Option<EntityId>>,
    needs_overrides: SharedMap<EntityId, HomeostaticNeeds>,
    pain_overrides: SharedMap<EntityId, Permille>,
    support_declaration_overrides: SharedMap<(EntityId, EntityId), Option<EntityId>>,
    office_holder_belief_overrides: SharedMap<EntityId, InstitutionalBeliefRead<Option<EntityId>>>,
    force_controller_belief_overrides:
        SharedMap<EntityId, InstitutionalBeliefRead<(Option<EntityId>, bool)>>,
    support_declaration_belief_overrides:
        SharedMap<(EntityId, EntityId), InstitutionalBeliefRead<Option<EntityId>>>,
    facility_queue_membership_overrides: SharedMap<EntityId, Option<HypotheticalQueueJoin>>,
    facility_grant_overrides: SharedMap<EntityId, Option<ContentionGrant>>,
    hypothetical_registry: SharedMap<HypotheticalEntityId, HypotheticalEntityMeta>,
    entities_at_cache: Rc<RefCell<BTreeMap<EntityId, Vec<EntityId>>>>,
    effective_place_cache: Rc<RefCell<BTreeMap<PlanningEntityRef, Option<EntityId>>>>,
    next_hypothetical_id: u32,
}

impl<'snapshot> PlanningState<'snapshot> {
    #[must_use]
    pub fn new(snapshot: &'snapshot PlanningSnapshot) -> Self {
        Self {
            snapshot,
            entity_place_overrides: SharedMap::new(),
            bandit_camp_faction_overrides: SharedMap::new(),
            direct_container_overrides: SharedMap::new(),
            direct_possessor_overrides: SharedMap::new(),
            resource_quantity_overrides: SharedMap::new(),
            commodity_quantity_overrides: SharedMap::new(),
            reservation_shadows: SharedMap::new(),
            removed_entities: SharedSet::new(),
            sale_listing_overrides: SharedMap::new(),
            sale_seller_overrides: SharedMap::new(),
            needs_overrides: SharedMap::new(),
            pain_overrides: SharedMap::new(),
            support_declaration_overrides: SharedMap::new(),
            office_holder_belief_overrides: SharedMap::new(),
            force_controller_belief_overrides: SharedMap::new(),
            support_declaration_belief_overrides: SharedMap::new(),
            facility_queue_membership_overrides: SharedMap::new(),
            facility_grant_overrides: SharedMap::new(),
            hypothetical_registry: SharedMap::new(),
            entities_at_cache: Rc::new(RefCell::new(BTreeMap::new())),
            effective_place_cache: Rc::new(RefCell::new(BTreeMap::new())),
            next_hypothetical_id: 0,
        }
    }

    fn invalidate_entities_at_cache(&mut self) {
        self.entities_at_cache = Rc::new(RefCell::new(BTreeMap::new()));
        self.effective_place_cache = Rc::new(RefCell::new(BTreeMap::new()));
    }

    #[must_use]
    pub fn snapshot(&self) -> &'snapshot PlanningSnapshot {
        self.snapshot
    }

    #[must_use]
    pub fn is_facility_use_blocked(
        &self,
        facility: EntityId,
        intended_action: ActionDefId,
    ) -> bool {
        self.snapshot
            .blocked_facility_uses
            .contains(&(facility, intended_action))
    }

    #[must_use]
    pub fn move_entity(self, entity: EntityId, destination: EntityId) -> Self {
        self.move_entity_ref(PlanningEntityRef::Authoritative(entity), destination)
    }

    #[must_use]
    pub fn move_actor_to(self, destination: EntityId) -> Self {
        let actor = self.snapshot.actor();
        self.move_entity(actor, destination)
    }

    #[must_use]
    pub fn bandit_camp_faction_at(&self, place: EntityId) -> Option<EntityId> {
        self.bandit_camp_faction_overrides
            .get(&place)
            .copied()
            .flatten()
            .or_else(|| {
                self.snapshot
                    .places
                    .get(&place)
                    .and_then(|place| place.bandit_camp_faction)
            })
    }

    #[must_use]
    pub fn with_bandit_camp_faction(mut self, place: EntityId, faction: Option<EntityId>) -> Self {
        self.bandit_camp_faction_overrides.insert(place, faction);
        self
    }

    #[must_use]
    pub fn with_support_declaration(
        mut self,
        supporter: EntityId,
        office: EntityId,
        candidate: EntityId,
    ) -> Self {
        self.support_declaration_overrides
            .insert((supporter, office), Some(candidate));
        self
    }

    #[must_use]
    pub fn believed_office_holder(
        &self,
        office: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        self.office_holder_belief_overrides
            .get(&office)
            .cloned()
            .unwrap_or_else(|| self.snapshot.believed_office_holder(office))
    }

    #[must_use]
    pub fn believed_force_controller(
        &self,
        office: EntityId,
    ) -> InstitutionalBeliefRead<(Option<EntityId>, bool)> {
        self.force_controller_belief_overrides
            .get(&office)
            .cloned()
            .unwrap_or_else(|| self.snapshot.believed_force_controller(office))
    }

    #[must_use]
    pub fn believed_faction_rally_point(
        &self,
        faction: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        let values = self
            .snapshot
            .actor_known_institutional_beliefs
            .iter()
            .filter_map(|belief| match belief.claim {
                worldwake_core::InstitutionalClaim::FactionRallyPoint {
                    faction: claim_faction,
                    rally_place,
                    ..
                } if claim_faction == faction => Some(rally_place),
                _ => None,
            })
            .collect::<std::collections::BTreeSet<_>>();

        match values.len() {
            0 => InstitutionalBeliefRead::Unknown,
            1 => InstitutionalBeliefRead::Certain(
                *values
                    .iter()
                    .next()
                    .expect("single rally-point read should contain a value"),
            ),
            _ => InstitutionalBeliefRead::Conflicted(values.into_iter().collect()),
        }
    }

    #[must_use]
    pub fn succession_law(&self, office: EntityId) -> Option<SuccessionLaw> {
        self.snapshot.succession_law(office)
    }

    #[must_use]
    pub fn believed_support_declaration(
        &self,
        office: EntityId,
        supporter: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        self.support_declaration_belief_overrides
            .get(&(office, supporter))
            .cloned()
            .unwrap_or_else(|| {
                self.snapshot
                    .believed_support_declaration(office, supporter)
            })
    }

    #[must_use]
    pub fn effective_support_declaration(
        &self,
        supporter: EntityId,
        office: EntityId,
    ) -> Option<EntityId> {
        self.support_declaration_overrides
            .get(&(supporter, office))
            .copied()
            .flatten()
            .or_else(|| {
                self.snapshot
                    .base_support_declarations_for_office(office)
                    .iter()
                    .find(|(base_supporter, _)| *base_supporter == supporter)
                    .map(|(_, candidate)| *candidate)
            })
    }

    #[must_use]
    pub fn believed_support_declarations_for_office(
        &self,
        office: EntityId,
    ) -> Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)> {
        let mut combined: BTreeMap<EntityId, InstitutionalBeliefRead<Option<EntityId>>> = self
            .snapshot
            .believed_support_declarations_for_office(office)
            .iter()
            .map(|(supporter, read)| (*supporter, read.clone()))
            .collect();

        for (&(override_office, supporter), read) in
            self.support_declaration_belief_overrides.iter()
        {
            if override_office == office {
                combined.insert(supporter, read.clone());
            }
        }

        combined.into_iter().collect()
    }

    pub fn override_office_holder_belief(
        &mut self,
        office: EntityId,
        value: InstitutionalBeliefRead<Option<EntityId>>,
    ) {
        self.office_holder_belief_overrides.insert(office, value);
    }

    pub fn override_force_controller_belief(
        &mut self,
        office: EntityId,
        value: InstitutionalBeliefRead<(Option<EntityId>, bool)>,
    ) {
        self.force_controller_belief_overrides.insert(office, value);
    }

    pub fn override_support_declaration_belief(
        &mut self,
        office: EntityId,
        supporter: EntityId,
        value: InstitutionalBeliefRead<Option<EntityId>>,
    ) {
        self.support_declaration_belief_overrides
            .insert((office, supporter), value);
    }

    #[must_use]
    pub fn record_data(&self, record: EntityId) -> Option<RecordData> {
        self.snapshot
            .entities
            .get(&record)
            .and_then(|snapshot| snapshot.political.record_data.clone())
    }

    /// Count hypothetical support declarations for `candidate` at `office`,
    /// combining base snapshot declarations with planning overrides.
    #[must_use]
    pub fn hypothetical_support_count(&self, office: EntityId, candidate: EntityId) -> usize {
        let base_declarations = self.snapshot.base_support_declarations_for_office(office);

        // Start with base declarations, applying any overrides
        let mut count = 0usize;
        for &(supporter, base_candidate) in base_declarations {
            let effective_candidate =
                match self.support_declaration_overrides.get(&(supporter, office)) {
                    Some(Some(c)) => Some(*c),    // overridden to support c
                    Some(None) => None,           // support withdrawn
                    None => Some(base_candidate), // no override, use base
                };
            if effective_candidate == Some(candidate) {
                count += 1;
            }
        }

        // Add purely hypothetical declarations (supporters NOT in base)
        for (&(supporter, decl_office), override_val) in self.support_declaration_overrides.iter() {
            if decl_office == office
                && let Some(decl_candidate) = override_val
                && *decl_candidate == candidate
                && !base_declarations.iter().any(|(s, _)| *s == supporter)
            {
                count += 1;
            }
        }

        count
    }

    /// Returns true if `candidate` has strictly more hypothetical support
    /// declarations than every other candidate for `office`.
    #[must_use]
    pub fn has_support_majority(&self, office: EntityId, candidate: EntityId) -> bool {
        let actor_count = self.hypothetical_support_count(office, candidate);
        if actor_count == 0 {
            return false;
        }

        // Collect all known candidates (from base + overrides)
        let base = self.snapshot.base_support_declarations_for_office(office);
        let mut all_candidates = BTreeSet::new();
        for &(_, c) in base {
            all_candidates.insert(c);
        }
        for (&(_, decl_office), override_val) in self.support_declaration_overrides.iter() {
            if decl_office == office
                && let Some(c) = override_val
            {
                all_candidates.insert(*c);
            }
        }

        // Actor must have strictly more than every other candidate
        all_candidates
            .into_iter()
            .filter(|&c| c != candidate)
            .all(|c| self.hypothetical_support_count(office, c) < actor_count)
    }

    #[must_use]
    pub fn move_lot_to_holder(
        self,
        lot: EntityId,
        holder: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Self {
        self.move_lot_ref_to_holder(
            PlanningEntityRef::Authoritative(lot),
            PlanningEntityRef::Authoritative(holder),
            commodity,
            quantity,
        )
    }

    #[must_use]
    pub fn move_lot_ref_to_holder(
        mut self,
        lot: PlanningEntityRef,
        holder: PlanningEntityRef,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Self {
        self.invalidate_entities_at_cache();
        let previous_holder = self.direct_possessor_ref(lot);
        self.direct_possessor_overrides.insert(lot, Some(holder));
        self.direct_container_overrides.insert(lot, None);
        self.entity_place_overrides.remove(&lot);

        if let Some(previous_holder) = previous_holder {
            let current = self.commodity_quantity_ref(previous_holder, commodity);
            let next = Quantity(current.0.saturating_sub(quantity.0));
            self.commodity_quantity_overrides
                .insert((previous_holder, commodity), next);
        }
        let current = self.commodity_quantity_ref(holder, commodity);
        let next = Quantity(current.0.saturating_add(quantity.0));
        self.commodity_quantity_overrides
            .insert((holder, commodity), next);
        self
    }

    #[must_use]
    pub fn move_lot_ref_to_ground(
        mut self,
        lot: PlanningEntityRef,
        place: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Self {
        self.invalidate_entities_at_cache();
        if let Some(previous_holder) = self.direct_possessor_ref(lot) {
            let current = self.commodity_quantity_ref(previous_holder, commodity);
            let next = Quantity(current.0.saturating_sub(quantity.0));
            self.commodity_quantity_overrides
                .insert((previous_holder, commodity), next);
        }
        self.direct_possessor_overrides.insert(lot, None);
        self.direct_container_overrides.insert(lot, None);
        self.entity_place_overrides.insert(lot, Some(place));
        self
    }

    pub fn spawn_hypothetical_lot(
        &mut self,
        kind: EntityKind,
        commodity: CommodityKind,
    ) -> HypotheticalEntityId {
        let id = HypotheticalEntityId(self.next_hypothetical_id);
        self.next_hypothetical_id = self
            .next_hypothetical_id
            .checked_add(1)
            .expect("hypothetical entity id overflow");
        self.hypothetical_registry.insert(
            id,
            HypotheticalEntityMeta {
                kind,
                item_lot_commodity: Some(commodity),
            },
        );
        id
    }

    #[must_use]
    pub fn entity_kind_ref(&self, entity: PlanningEntityRef) -> Option<EntityKind> {
        if self.removed_entities.contains(&entity) {
            return None;
        }
        match entity {
            PlanningEntityRef::Authoritative(entity) => self
                .snapshot
                .entities
                .get(&entity)
                .and_then(|snapshot| snapshot.entity.kind),
            PlanningEntityRef::Hypothetical(entity) => self
                .hypothetical_registry
                .get(&entity)
                .map(|meta| meta.kind),
        }
    }

    #[must_use]
    pub fn effective_place_ref(&self, entity: PlanningEntityRef) -> Option<EntityId> {
        if let Some(place) = self.effective_place_cache.borrow().get(&entity) {
            return *place;
        }
        let resolved = self.resolve_effective_place_ref(entity, &mut BTreeSet::new());
        self.effective_place_cache
            .borrow_mut()
            .insert(entity, resolved);
        resolved
    }

    #[must_use]
    pub fn commodity_quantity_ref(
        &self,
        holder: PlanningEntityRef,
        kind: CommodityKind,
    ) -> Quantity {
        if self.removed_entities.contains(&holder) {
            return Quantity(0);
        }
        self.commodity_quantity_overrides
            .get(&(holder, kind))
            .copied()
            .or_else(|| match holder {
                PlanningEntityRef::Authoritative(holder) => {
                    self.snapshot.entities.get(&holder).and_then(|snapshot| {
                        snapshot.inventory.commodity_quantities.get(&kind).copied()
                    })
                }
                PlanningEntityRef::Hypothetical(_) => None,
            })
            .unwrap_or(Quantity(0))
    }

    #[must_use]
    pub fn direct_container_ref(&self, entity: PlanningEntityRef) -> Option<PlanningEntityRef> {
        if self.removed_entities.contains(&entity) {
            return None;
        }
        match self.direct_container_overrides.get(&entity) {
            Some(override_value) => *override_value,
            None => match entity {
                PlanningEntityRef::Authoritative(entity) => self
                    .snapshot
                    .entities
                    .get(&entity)
                    .and_then(|snapshot| snapshot.inventory.direct_container)
                    .map(PlanningEntityRef::Authoritative),
                PlanningEntityRef::Hypothetical(_) => None,
            },
        }
    }

    #[must_use]
    pub fn direct_possessor_ref(&self, entity: PlanningEntityRef) -> Option<PlanningEntityRef> {
        if self.removed_entities.contains(&entity) {
            return None;
        }
        match self.direct_possessor_overrides.get(&entity) {
            Some(override_value) => *override_value,
            None => match entity {
                PlanningEntityRef::Authoritative(entity) => self
                    .snapshot
                    .entities
                    .get(&entity)
                    .and_then(|snapshot| snapshot.inventory.direct_possessor)
                    .map(PlanningEntityRef::Authoritative),
                PlanningEntityRef::Hypothetical(_) => None,
            },
        }
    }

    #[must_use]
    pub fn stock_storage_policy_snapshot(
        &self,
        entity: EntityId,
    ) -> Option<worldwake_core::StockStoragePolicy> {
        self.snapshot
            .entities
            .get(&entity)
            .and_then(|snapshot| snapshot.facility.stock_storage_policy.clone())
    }

    #[must_use]
    pub fn move_entity_ref(mut self, entity: PlanningEntityRef, destination: EntityId) -> Self {
        self.invalidate_entities_at_cache();
        self.entity_place_overrides
            .insert(entity, Some(destination));
        self
    }

    #[must_use]
    pub fn set_possessor_ref(
        mut self,
        entity: PlanningEntityRef,
        holder: PlanningEntityRef,
    ) -> Self {
        self.invalidate_entities_at_cache();
        self.direct_possessor_overrides.insert(entity, Some(holder));
        self.direct_container_overrides.insert(entity, None);
        self.entity_place_overrides.remove(&entity);
        self
    }

    #[must_use]
    pub fn set_container_ref(
        mut self,
        entity: PlanningEntityRef,
        container: PlanningEntityRef,
    ) -> Self {
        self.invalidate_entities_at_cache();
        self.direct_container_overrides
            .insert(entity, Some(container));
        self.direct_possessor_overrides.insert(entity, None);
        self.entity_place_overrides.remove(&entity);
        self
    }

    #[must_use]
    pub fn set_quantity_ref(
        mut self,
        entity: PlanningEntityRef,
        commodity: CommodityKind,
        qty: Quantity,
    ) -> Self {
        self.commodity_quantity_overrides
            .insert((entity, commodity), qty);
        self
    }

    #[must_use]
    pub fn mark_removed_ref(mut self, entity: PlanningEntityRef) -> Self {
        self.invalidate_entities_at_cache();
        self.removed_entities.insert(entity);
        self.entity_place_overrides.insert(entity, None);
        self.direct_container_overrides.insert(entity, None);
        self.direct_possessor_overrides.insert(entity, None);
        self.sale_listing_overrides.insert(entity, false);
        self.sale_seller_overrides.insert(entity, None);
        self
    }

    #[must_use]
    pub fn set_sale_listing_ref(
        mut self,
        entity: PlanningEntityRef,
        seller: Option<EntityId>,
    ) -> Self {
        self.sale_listing_overrides.insert(entity, seller.is_some());
        self.sale_seller_overrides.insert(entity, seller);
        self
    }

    #[must_use]
    pub fn clear_sale_listing_ref(mut self, entity: PlanningEntityRef) -> Self {
        self.sale_listing_overrides.insert(entity, false);
        self.sale_seller_overrides.insert(entity, None);
        self
    }

    #[must_use]
    pub fn item_lot_commodity_ref(&self, entity: PlanningEntityRef) -> Option<CommodityKind> {
        if self.removed_entities.contains(&entity) {
            return None;
        }
        match entity {
            PlanningEntityRef::Authoritative(entity) => self
                .snapshot
                .entities
                .get(&entity)
                .and_then(|snapshot| snapshot.inventory.item_lot_commodity),
            PlanningEntityRef::Hypothetical(entity) => self
                .hypothetical_registry
                .get(&entity)
                .and_then(|meta| meta.item_lot_commodity),
        }
    }

    #[must_use]
    pub fn carry_capacity_ref(&self, entity: PlanningEntityRef) -> Option<LoadUnits> {
        if self.removed_entities.contains(&entity) {
            return None;
        }
        match entity {
            PlanningEntityRef::Authoritative(entity) => self
                .snapshot
                .entities
                .get(&entity)
                .and_then(|snapshot| snapshot.inventory.carry_capacity),
            PlanningEntityRef::Hypothetical(_) => None,
        }
    }

    #[must_use]
    pub fn load_of_entity_ref(&self, entity: PlanningEntityRef) -> Option<LoadUnits> {
        if self.removed_entities.contains(&entity) {
            return None;
        }
        if self.entity_kind_ref(entity) == Some(EntityKind::ItemLot) {
            let commodity = self.item_lot_commodity_ref(entity)?;
            let quantity = self.commodity_quantity_ref(entity, commodity);
            return quantity
                .0
                .checked_mul(load_per_unit(commodity).0)
                .map(LoadUnits);
        }
        match entity {
            PlanningEntityRef::Authoritative(entity) => self
                .snapshot
                .entities
                .get(&entity)
                .map(|snapshot| snapshot.inventory.intrinsic_load),
            PlanningEntityRef::Hypothetical(_) => Some(LoadUnits(0)),
        }
    }

    #[must_use]
    pub fn remaining_carry_capacity_ref(&self, entity: PlanningEntityRef) -> Option<LoadUnits> {
        let capacity = self.carry_capacity_ref(entity)?.0;
        let carried = self.carried_load_ref(entity)?.0;
        capacity.checked_sub(carried).map(LoadUnits)
    }

    #[must_use]
    pub fn consume_commodity(mut self, commodity: CommodityKind) -> Self {
        let actor = self.snapshot.actor();
        let Some(mut needs) = self.homeostatic_needs(actor) else {
            return self;
        };
        let Some(thresholds) = self.drive_thresholds(actor) else {
            return self;
        };
        if let Some(profile) = commodity.spec().consumable_profile {
            if profile.hunger_relief_per_unit.value() > 0 {
                needs.hunger = thresholds
                    .hunger
                    .low()
                    .saturating_sub(Permille::new(1).unwrap());
            }
            if profile.thirst_relief_per_unit.value() > 0 {
                needs.thirst = thresholds
                    .thirst
                    .low()
                    .saturating_sub(Permille::new(1).unwrap());
            }
        }

        self.needs_overrides.insert(actor, needs);
        self
    }

    #[must_use]
    pub fn use_resource(mut self, source: EntityId, remaining_quantity: Quantity) -> Self {
        self.resource_quantity_overrides
            .insert(source, remaining_quantity);
        self
    }

    #[must_use]
    pub fn reserve(mut self, entity: EntityId, range: TickRange) -> Self {
        self.reservation_shadows
            .entry(entity)
            .or_default()
            .push(range);
        self
    }

    #[must_use]
    pub fn mark_removed(self, entity: EntityId) -> Self {
        self.mark_removed_ref(PlanningEntityRef::Authoritative(entity))
    }

    #[must_use]
    pub fn with_homeostatic_needs(mut self, entity: EntityId, needs: HomeostaticNeeds) -> Self {
        self.needs_overrides.insert(entity, needs);
        self
    }

    /// Read homeostatic needs for `agent` with overrides applied.
    #[must_use]
    pub fn homeostatic_needs_for(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
        self.needs_overrides.get(&agent).copied().or_else(|| {
            self.snapshot
                .entities
                .get(&agent)
                .and_then(|snapshot| snapshot.profiles.homeostatic_needs)
        })
    }

    #[must_use]
    pub fn with_commodity_quantity(
        mut self,
        entity: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Self {
        self.commodity_quantity_overrides.insert(
            (PlanningEntityRef::Authoritative(entity), commodity),
            quantity,
        );
        self
    }

    #[must_use]
    pub fn with_pain(mut self, entity: EntityId, pain: Permille) -> Self {
        self.pain_overrides.insert(entity, pain);
        self
    }

    #[must_use]
    pub fn pain_summary(&self, entity: EntityId) -> Option<Permille> {
        self.pain_overrides.get(&entity).copied().or_else(|| {
            self.snapshot.entities.get(&entity).map(|snapshot| {
                let total = snapshot.combat.wounds.iter().fold(0u16, |acc, wound| {
                    acc.saturating_add(wound.severity.value())
                });
                Permille::new(total.min(1000)).unwrap()
            })
        })
    }

    #[must_use]
    pub fn has_actor_facility_grant(&self, facility: EntityId, action_def: ActionDefId) -> bool {
        self.actor_facility_grant(facility).is_some_and(|grant| {
            grant.actor == self.snapshot.actor() && grant.intended_action == action_def
        })
    }

    #[must_use]
    pub fn is_actor_queued_at_facility(&self, facility: EntityId) -> bool {
        match self.facility_queue_membership_overrides.get(&facility) {
            Some(Some(_)) => true,
            Some(None) => false,
            None => self.actor_facility_queue_position(facility).is_some(),
        }
    }

    #[must_use]
    pub fn simulate_queue_join(mut self, facility: EntityId, action_def: ActionDefId) -> Self {
        self.facility_queue_membership_overrides.insert(
            facility,
            Some(HypotheticalQueueJoin {
                intended_action: action_def,
            }),
        );
        self.facility_grant_overrides.insert(facility, None);
        self
    }

    #[must_use]
    pub fn simulate_grant_received(mut self, facility: EntityId, action_def: ActionDefId) -> Self {
        self.facility_queue_membership_overrides
            .insert(facility, None);
        self.facility_grant_overrides.insert(
            facility,
            Some(ContentionGrant {
                actor: self.snapshot.actor(),
                intended_action: action_def,
                granted_at: worldwake_core::Tick(0),
                expires_at: worldwake_core::Tick(0),
            }),
        );
        self
    }

    #[must_use]
    pub fn simulate_grant_consumed(mut self, facility: EntityId) -> Self {
        self.facility_grant_overrides.insert(facility, None);
        self
    }

    fn actor_facility_queue_position(&self, facility: EntityId) -> Option<u32> {
        match self.facility_queue_membership_overrides.get(&facility) {
            Some(Some(_) | None) => None,
            None => self
                .snapshot
                .entities
                .get(&facility)
                .and_then(|snapshot| snapshot.temporal.facility_queue.as_ref())
                .and_then(|queue| queue.actor_queue_position),
        }
    }

    fn actor_facility_grant(&self, facility: EntityId) -> Option<&ContentionGrant> {
        match self.facility_grant_overrides.get(&facility) {
            Some(grant) => grant.as_ref(),
            None => self
                .snapshot
                .entities
                .get(&facility)
                .and_then(|snapshot| snapshot.temporal.facility_queue.as_ref())
                .and_then(|queue| queue.active_grant.as_ref()),
        }
    }

    fn resolve_effective_place(
        &self,
        entity: EntityId,
        visited: &mut BTreeSet<EntityId>,
    ) -> Option<EntityId> {
        let entity_ref = PlanningEntityRef::Authoritative(entity);
        let mut ref_visited = visited
            .iter()
            .copied()
            .map(PlanningEntityRef::Authoritative)
            .collect::<BTreeSet<_>>();
        let resolved = self.resolve_effective_place_ref(entity_ref, &mut ref_visited);
        *visited = ref_visited
            .into_iter()
            .filter_map(|entity| match entity {
                PlanningEntityRef::Authoritative(entity) => Some(entity),
                PlanningEntityRef::Hypothetical(_) => None,
            })
            .collect();
        resolved
    }

    fn resolve_effective_place_ref(
        &self,
        entity: PlanningEntityRef,
        visited: &mut BTreeSet<PlanningEntityRef>,
    ) -> Option<EntityId> {
        if !visited.insert(entity) || self.removed_entities.contains(&entity) {
            return None;
        }
        if let Some(place) = self.effective_place_cache.borrow().get(&entity) {
            return *place;
        }
        let resolved = if let Some(override_place) = self.entity_place_overrides.get(&entity) {
            *override_place
        } else if let Some(possessor) = self.direct_possessor_ref(entity) {
            self.resolve_effective_place_ref(possessor, visited)
        } else if let Some(container) = self.direct_container_ref(entity) {
            self.resolve_effective_place_ref(container, visited)
        } else {
            match entity {
                PlanningEntityRef::Authoritative(entity) => self
                    .snapshot
                    .entities
                    .get(&entity)
                    .and_then(|snapshot| snapshot.spatial.effective_place),
                PlanningEntityRef::Hypothetical(_) => None,
            }
        };
        self.effective_place_cache
            .borrow_mut()
            .insert(entity, resolved);
        resolved
    }

    fn carried_load_ref(&self, holder: PlanningEntityRef) -> Option<LoadUnits> {
        let mut seen = BTreeSet::new();
        let mut frontier = self.direct_child_refs(holder);
        let mut total = 0u32;

        while let Some(entity) = frontier.pop() {
            if !seen.insert(entity) {
                continue;
            }

            total = total.checked_add(self.load_of_entity_ref(entity)?.0)?;
            frontier.extend(self.direct_child_refs(entity));
        }

        Some(LoadUnits(total))
    }

    fn direct_child_refs(&self, holder: PlanningEntityRef) -> Vec<PlanningEntityRef> {
        let mut candidates = BTreeSet::new();
        if let PlanningEntityRef::Authoritative(holder_entity) = holder
            && let Some(snapshot) = self.snapshot.entities.get(&holder_entity)
        {
            candidates.extend(
                snapshot
                    .inventory
                    .direct_possessions
                    .iter()
                    .copied()
                    .map(PlanningEntityRef::Authoritative),
            );
            candidates.extend(
                snapshot
                    .inventory
                    .direct_contents
                    .iter()
                    .copied()
                    .map(PlanningEntityRef::Authoritative),
            );
        }
        candidates.extend(self.direct_possessor_overrides.keys().copied());
        candidates.extend(self.direct_container_overrides.keys().copied());
        candidates.extend(
            self.hypothetical_registry
                .keys()
                .copied()
                .map(PlanningEntityRef::Hypothetical),
        );
        candidates
            .into_iter()
            .filter(|entity| {
                self.direct_possessor_ref(*entity) == Some(holder)
                    || self.direct_container_ref(*entity) == Some(holder)
            })
            .collect()
    }

    #[must_use]
    pub fn direct_possessions_ref(&self, holder: PlanningEntityRef) -> Vec<PlanningEntityRef> {
        self.all_entity_refs()
            .into_iter()
            .filter(|entity| self.direct_possessor_ref(*entity) == Some(holder))
            .collect()
    }

    #[must_use]
    pub fn local_controlled_lot_refs_for(
        &self,
        agent: PlanningEntityRef,
        place: EntityId,
        commodity: CommodityKind,
    ) -> Vec<PlanningEntityRef> {
        let mut entities = self
            .all_entity_refs()
            .into_iter()
            .filter(|entity| self.effective_place_ref(*entity) == Some(place))
            .filter(|entity| self.item_lot_commodity_ref(*entity) == Some(commodity))
            .filter(|entity| self.can_control_ref(agent, *entity))
            .collect::<Vec<_>>();
        entities.sort();
        entities.dedup();
        entities
    }

    #[must_use]
    pub(crate) fn hypothetical_ground_lot_refs_at_place(
        &self,
        place: EntityId,
    ) -> Vec<PlanningEntityRef> {
        let mut entities = self
            .hypothetical_registry
            .keys()
            .copied()
            .map(PlanningEntityRef::Hypothetical)
            .filter(|entity| self.entity_kind_ref(*entity) == Some(EntityKind::ItemLot))
            .filter(|entity| self.direct_possessor_ref(*entity).is_none())
            .filter(|entity| self.direct_container_ref(*entity).is_none())
            .filter(|entity| self.effective_place_ref(*entity) == Some(place))
            .collect::<Vec<_>>();
        entities.sort();
        entities.dedup();
        entities
    }

    #[must_use]
    pub fn controlled_stock_containers_at_place(
        &self,
        agent: PlanningEntityRef,
        place: EntityId,
    ) -> Vec<PlanningEntityRef> {
        let mut containers = self
            .snapshot
            .entities
            .iter()
            .filter_map(|(facility, snapshot)| {
                let policy = snapshot.facility.stock_storage_policy.as_ref()?;
                (snapshot.spatial.effective_place == Some(place)
                    && self.can_control_ref(agent, PlanningEntityRef::Authoritative(*facility)))
                .then_some(PlanningEntityRef::Authoritative(policy.stock_container))
            })
            .collect::<Vec<_>>();
        containers.sort();
        containers.dedup();
        containers
    }

    fn all_entity_refs(&self) -> Vec<PlanningEntityRef> {
        let mut refs = self
            .snapshot
            .entities
            .keys()
            .copied()
            .map(PlanningEntityRef::Authoritative)
            .collect::<Vec<_>>();
        refs.extend(
            self.hypothetical_registry
                .keys()
                .copied()
                .map(PlanningEntityRef::Hypothetical),
        );
        refs
    }

    pub(crate) fn can_control_ref(
        &self,
        actor: PlanningEntityRef,
        entity: PlanningEntityRef,
    ) -> bool {
        if self.removed_entities.contains(&actor) || self.removed_entities.contains(&entity) {
            return false;
        }
        if entity == actor {
            return true;
        }
        if let Some(container) = self.direct_container_ref(entity) {
            return self.can_control_ref(actor, container);
        }
        if self.direct_possessor_ref(entity) == Some(actor) {
            return true;
        }
        match entity {
            PlanningEntityRef::Authoritative(entity) => self
                .snapshot
                .entities
                .get(&entity)
                .is_some_and(|snapshot| snapshot.control.controllable_by_actor),
            PlanningEntityRef::Hypothetical(_) => false,
        }
    }
}

#[cfg(test)]
impl PlanningState<'_> {
    pub(crate) fn test_support_override(
        &self,
        supporter: EntityId,
        office: EntityId,
    ) -> Option<EntityId> {
        self.support_declaration_overrides
            .get(&(supporter, office))
            .copied()
            .flatten()
    }

    pub(crate) fn test_support_belief_override(
        &self,
        office: EntityId,
        supporter: EntityId,
    ) -> Option<InstitutionalBeliefRead<Option<EntityId>>> {
        self.support_declaration_belief_overrides
            .get(&(office, supporter))
            .cloned()
    }
}

impl ControlBeliefView for PlanningState<'_> {
    fn believed_owner_of(&self, entity: EntityId) -> Option<EntityId> {
        self.snapshot
            .entities
            .get(&entity)
            .and_then(|snapshot| snapshot.control.owner)
    }

    fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
        actor == self.snapshot.actor()
            && self
                .snapshot
                .entities
                .get(&entity)
                .is_some_and(|snapshot| snapshot.control.controllable_by_actor)
    }

    fn has_control(&self, entity: EntityId) -> bool {
        self.snapshot
            .entities
            .get(&entity)
            .is_some_and(|snapshot| snapshot.control.has_control)
    }
}

impl EntityBeliefView for PlanningState<'_> {
    fn is_alive(&self, entity: EntityId) -> bool {
        !self
            .removed_entities
            .contains(&PlanningEntityRef::Authoritative(entity))
            && self
                .snapshot
                .entities
                .get(&entity)
                .is_some_and(|snapshot| snapshot.entity.alive)
    }

    fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
        self.entity_kind_ref(PlanningEntityRef::Authoritative(entity))
    }

    fn is_dead(&self, entity: EntityId) -> bool {
        self.removed_entities
            .contains(&PlanningEntityRef::Authoritative(entity))
            || self
                .snapshot
                .entities
                .get(&entity)
                .is_some_and(|snapshot| snapshot.entity.dead)
    }

    fn is_incapacitated(&self, entity: EntityId) -> bool {
        self.snapshot
            .entities
            .get(&entity)
            .is_some_and(|snapshot| snapshot.entity.incapacitated)
    }

    fn bandit_flee_wound_threshold(&self, faction: EntityId) -> Option<Permille> {
        self.snapshot.bandit_flee_wound_threshold(faction)
    }

    fn bandit_camp_establishment_ticks(&self, faction: EntityId) -> Option<std::num::NonZeroU32> {
        self.snapshot.bandit_camp_establishment_ticks(faction)
    }

    fn corpse_entities_at(&self, place: EntityId) -> Vec<EntityId> {
        self.entities_at(place)
            .into_iter()
            .filter(|entity| self.is_dead(*entity))
            .collect()
    }

    fn believed_target_location(
        &self,
        agent: EntityId,
        target: EntityId,
    ) -> worldwake_sim::belief_view::BeliefValue<Option<EntityId>> {
        if agent != self.snapshot.actor() {
            return worldwake_sim::belief_view::stale_default_value(None);
        }

        worldwake_sim::belief_view::project_claims_into_belief_set(
            self.snapshot
                .actor_belief_store
                .get_entity_claims(&target)
                .into_iter()
                .flatten()
                .filter_map(|claim| {
                    worldwake_sim::belief_view::location_claim_value(claim)
                        .map(|value| (claim.clone(), value))
                }),
            self.snapshot.current_tick,
            self.snapshot.actor_claim_confidence_threshold,
            &self.snapshot.actor_confidence_policy,
        )
        .best
        .unwrap_or_else(|| worldwake_sim::belief_view::stale_default_value(None))
    }
}

impl ProfileBeliefView for PlanningState<'_> {
    fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
        self.needs_overrides.get(&agent).copied().or_else(|| {
            self.snapshot
                .entities
                .get(&agent)
                .and_then(|snapshot| snapshot.profiles.homeostatic_needs)
        })
    }

    fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.profiles.drive_thresholds)
    }

    fn metabolism_profile(&self, agent: EntityId) -> Option<MetabolismProfile> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.profiles.metabolism_profile)
    }

    fn disposal_profile(&self, agent: EntityId) -> Option<DisposalProfile> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.profiles.disposal_profile)
    }

    fn artifact_posting_profile(&self, agent: EntityId) -> Option<ArtifactPostingProfile> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.profiles.artifact_posting_profile.clone())
    }
}

impl SpatialBeliefView for PlanningState<'_> {
    fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
        self.resolve_effective_place(entity, &mut BTreeSet::new())
    }

    fn is_in_transit(&self, entity: EntityId) -> bool {
        self.in_transit_state(entity).is_some()
    }

    fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
        // Fast path: when no overrides exist, use the snapshot's pre-indexed
        // place→entities set directly.  This avoids O(all_entities) scans with
        // expensive effective_place resolution on the root node and unmodified
        // states during early search.
        if self.entity_place_overrides.is_empty()
            && self.direct_container_overrides.is_empty()
            && self.direct_possessor_overrides.is_empty()
            && self.removed_entities.is_empty()
        {
            let entities = self
                .snapshot
                .places
                .get(&place)
                .map(|p| p.entities.iter().copied().collect())
                .unwrap_or_default();
            return entities;
        }

        if let Some(entities) = self.entities_at_cache.borrow().get(&place) {
            return entities.clone();
        }

        // Slow path: full scan with override resolution.
        let mut entities = self
            .snapshot
            .entities
            .keys()
            .copied()
            .filter(|entity| self.effective_place(*entity) == Some(place))
            .filter(|entity| {
                !self
                    .removed_entities
                    .contains(&PlanningEntityRef::Authoritative(*entity))
            })
            .collect::<Vec<_>>();
        entities.sort();
        entities.dedup();
        // Slow-path queries repeat heavily across affordance enumeration for a
        // single immutable planning state. Cache per-place results until a
        // later state mutation invalidates them.
        self.entities_at_cache
            .borrow_mut()
            .insert(place, entities.clone());
        entities
    }

    fn adjacent_places(&self, place: EntityId) -> Vec<EntityId> {
        self.adjacent_places_with_travel_ticks(place)
            .into_iter()
            .map(|(adjacent, _)| adjacent)
            .collect()
    }

    fn place_has_tag(&self, place: EntityId, tag: PlaceTag) -> bool {
        self.snapshot
            .places
            .get(&place)
            .is_some_and(|snapshot| snapshot.tags.contains(&tag))
    }

    fn patrol_route(&self, agent: EntityId) -> Option<PatrolRoute> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.spatial.patrol_route.clone())
    }

    fn route_exists(&self, _from: EntityId, _to: EntityId) -> bool {
        false
    }

    fn in_transit_state(&self, entity: EntityId) -> Option<InTransitOnEdge> {
        self.snapshot
            .entities
            .get(&entity)
            .and_then(|snapshot| snapshot.spatial.in_transit_state.clone())
    }

    fn adjacent_places_with_travel_ticks(
        &self,
        place: EntityId,
    ) -> Vec<(EntityId, std::num::NonZeroU32)> {
        self.snapshot
            .places
            .get(&place)
            .map(|snapshot| snapshot.adjacent_places_with_travel_ticks.clone())
            .unwrap_or_default()
    }

    fn believed_entities_at(
        &self,
        agent: EntityId,
        place: EntityId,
        kind: EntityKind,
    ) -> Vec<worldwake_sim::belief_view::BeliefValue<EntityId>> {
        if agent != self.snapshot.actor() {
            return Vec::new();
        }

        self.snapshot
            .actor_known_entity_beliefs
            .iter()
            .filter_map(|(subject, state)| (state.believed_kind == Some(kind)).then_some(*subject))
            .filter_map(|subject| {
                let best = worldwake_sim::belief_view::project_claims_into_belief_set(
                    self.snapshot
                        .actor_belief_store
                        .get_entity_claims(&subject)
                        .into_iter()
                        .flatten()
                        .filter_map(|claim| {
                            worldwake_sim::belief_view::location_claim_value(claim)
                                .map(|value| (claim.clone(), value))
                        }),
                    self.snapshot.current_tick,
                    self.snapshot.actor_claim_confidence_threshold,
                    &self.snapshot.actor_confidence_policy,
                )
                .best?;
                (best.value == Some(place)).then_some(worldwake_sim::belief_view::BeliefValue {
                    value: subject,
                    confidence: best.confidence,
                    acquired_tick: best.acquired_tick,
                    claimed_event_tick: best.claimed_event_tick,
                    status: best.status,
                })
            })
            .collect()
    }
}

impl TemporalBeliefView for PlanningState<'_> {
    fn current_tick(&self) -> worldwake_core::Tick {
        self.snapshot.current_tick
    }

    fn has_contention_policy(&self, entity: EntityId) -> bool {
        self.snapshot
            .entities
            .get(&entity)
            .and_then(|snapshot| snapshot.temporal.facility_queue.as_ref())
            .is_some()
    }

    fn facility_queue_position(&self, facility: EntityId, actor: EntityId) -> Option<u32> {
        (actor == self.snapshot.actor()).then(|| self.actor_facility_queue_position(facility))?
    }

    fn facility_grant(&self, facility: EntityId) -> Option<&worldwake_core::ContentionGrant> {
        self.actor_facility_grant(facility)
    }

    fn reservation_conflicts(&self, entity: EntityId, range: TickRange) -> bool {
        self.reservation_shadows
            .get(&entity)
            .into_iter()
            .flatten()
            .any(|shadow| shadow.overlaps(&range))
            || self
                .snapshot
                .entities
                .get(&entity)
                .into_iter()
                .flat_map(|snapshot| snapshot.temporal.reservation_ranges.iter())
                .any(|existing| existing.overlaps(&range))
    }

    fn reservation_ranges(&self, entity: EntityId) -> Vec<TickRange> {
        let mut ranges = self
            .snapshot
            .entities
            .get(&entity)
            .map(|snapshot| snapshot.temporal.reservation_ranges.clone())
            .unwrap_or_default();
        if let Some(shadows) = self.reservation_shadows.get(&entity) {
            ranges.extend(shadows.iter().copied());
        }
        ranges
    }

    fn estimate_duration(
        &self,
        actor: EntityId,
        duration: &DurationExpr,
        targets: &[EntityId],
        payload: &ActionPayload,
    ) -> Option<ActionDuration> {
        estimate_duration_from_beliefs(self, actor, duration, targets, payload)
    }
}

impl SocialBeliefView for PlanningState<'_> {
    fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
        if agent != self.snapshot.actor() {
            return Vec::new();
        }

        self.snapshot
            .actor_known_entity_beliefs
            .iter()
            .map(|(entity, belief)| (*entity, belief.clone()))
            .collect()
    }

    fn agent_belief_store(&self, agent: EntityId) -> Option<&worldwake_core::AgentBeliefStore> {
        (agent == self.snapshot.actor()).then_some(&self.snapshot.actor_belief_store)
    }

    fn known_social_observations(&self, agent: EntityId) -> Vec<SocialObservation> {
        if agent != self.snapshot.actor() {
            return Vec::new();
        }

        self.snapshot.actor_known_social_observations.clone()
    }

    fn claim_confidence_threshold(&self, agent: EntityId) -> Permille {
        assert_eq!(
            agent,
            self.snapshot.actor(),
            "claim_confidence_threshold is a self-authoritative read and must only be requested for the planning actor"
        );
        self.snapshot.actor_claim_confidence_threshold
    }

    fn believed_activity_of(&self, entity: EntityId) -> Option<&worldwake_core::BelievedActivity> {
        self.snapshot
            .actor_known_entity_beliefs
            .get(&entity)
            .and_then(|belief| belief.believed_activity.as_ref())
    }

    fn agents_active_at(
        &self,
        place: EntityId,
        domain: ActionDomain,
        target: Option<EntityId>,
    ) -> Vec<EntityId> {
        let mut entities = self
            .snapshot
            .actor_known_entity_beliefs
            .iter()
            .filter_map(|(entity, belief)| {
                (belief.last_known_place == Some(place)
                    && belief.believed_activity.as_ref().is_some_and(|activity| {
                        activity.action_domain == domain
                            && (target.is_none() || activity.target == target)
                    }))
                .then_some(*entity)
            })
            .collect::<Vec<_>>();
        entities.sort();
        entities.dedup();
        entities
    }

    fn belief_confidence_policy(&self, agent: EntityId) -> worldwake_core::BeliefConfidencePolicy {
        assert_eq!(
            agent,
            self.snapshot.actor(),
            "belief_confidence_policy is a self-authoritative read and must only be requested for the planning actor"
        );
        self.snapshot.actor_confidence_policy
    }

    fn expectation_store(&self, agent: EntityId) -> Option<worldwake_core::ExpectationStore> {
        (agent == self.snapshot.actor())
            .then_some(self.snapshot.actor_expectation_store.clone())
            .flatten()
    }

    fn last_seen_memory(&self, agent: EntityId) -> Option<worldwake_core::LastSeenMemory> {
        (agent == self.snapshot.actor())
            .then_some(self.snapshot.actor_last_seen_memory.clone())
            .flatten()
    }

    fn epistemic_disposition_profile(
        &self,
        agent: EntityId,
    ) -> Option<worldwake_core::EpistemicDispositionProfile> {
        (agent == self.snapshot.actor())
            .then_some(self.snapshot.actor_epistemic_profile.clone())
            .flatten()
    }

    fn theft_disposition_profile(&self, agent: EntityId) -> Option<TheftDispositionProfile> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.social.theft_disposition_profile.clone())
    }

    fn intention_disposition_profile(
        &self,
        _agent: EntityId,
    ) -> Option<worldwake_core::IntentionDispositionProfile> {
        None
    }

    fn tell_profile(&self, agent: EntityId) -> Option<TellProfile> {
        (agent == self.snapshot.actor())
            .then_some(self.snapshot.actor_tell_profile)
            .flatten()
    }

    fn told_belief_memories(&self, agent: EntityId) -> Vec<(TellMemoryKey, ToldBeliefMemory)> {
        if agent != self.snapshot.actor() {
            return Vec::new();
        }

        self.snapshot
            .actor_told_beliefs
            .iter()
            .map(|(key, memory)| (*key, memory.clone()))
            .collect()
    }

    fn told_belief_memory(
        &self,
        actor: EntityId,
        counterparty: EntityId,
        topic: &TellTopic,
    ) -> Option<ToldBeliefMemory> {
        if actor != self.snapshot.actor() {
            return None;
        }

        let profile = self.tell_profile(actor)?;
        self.snapshot
            .actor_told_beliefs
            .get(&TellMemoryKey {
                counterparty,
                topic: *topic,
            })
            .filter(|memory| {
                self.snapshot
                    .current_tick
                    .0
                    .saturating_sub(memory.told_tick.0)
                    <= profile.conversation_memory_retention_ticks
            })
            .cloned()
    }

    fn recipient_knowledge_status(
        &self,
        actor: EntityId,
        counterparty: EntityId,
        topic: &TellTopic,
    ) -> Option<RecipientKnowledgeStatus> {
        if actor != self.snapshot.actor() {
            return None;
        }

        let current_state = match topic {
            TellTopic::EntityBelief { subject } => SharedTellState::EntityBelief(
                to_shared_belief_snapshot(self.snapshot.actor_known_entity_beliefs.get(subject)?),
            ),
            TellTopic::SocialObservation { observation } => self
                .snapshot
                .actor_known_social_observations
                .contains(observation)
                .then_some(SharedTellState::SocialObservation(*observation))?,
            TellTopic::InstitutionalClaim { claim } => self
                .snapshot
                .actor_known_institutional_beliefs
                .iter()
                .filter(|belief| belief.claim == *claim)
                .max_by_key(|belief| {
                    (
                        std::cmp::Reverse(worldwake_core::institutional_knowledge_chain_len(
                            belief.source,
                        )),
                        belief.learned_tick,
                        belief.learned_at,
                    )
                })
                .map(|belief| {
                    SharedTellState::InstitutionalClaim(worldwake_core::SharedInstitutionalBelief {
                        claim: belief.claim,
                        source: belief.source,
                    })
                })?,
        };
        self.tell_profile(actor)?;
        let remembered = self
            .snapshot
            .actor_told_beliefs
            .iter()
            .filter(|(key, _)| {
                key.counterparty == counterparty
                    && match (&key.topic, topic) {
                        (
                            TellTopic::InstitutionalClaim { claim: left_claim },
                            TellTopic::InstitutionalClaim { claim: right_claim },
                        ) => worldwake_core::institutional_claim_same_memory_lane(
                            *left_claim,
                            *right_claim,
                        ),
                        _ => key.topic == *topic,
                    }
            })
            .filter(|(_, memory)| {
                self.snapshot
                    .current_tick
                    .0
                    .saturating_sub(memory.told_tick.0)
                    <= self
                        .snapshot
                        .actor_tell_profile
                        .map_or(0, |profile| profile.conversation_memory_retention_ticks)
            })
            .map(|(_, memory)| memory)
            .max_by_key(|memory| memory.told_tick);

        Some(match remembered.as_ref() {
            Some(memory) => {
                worldwake_core::recipient_knowledge_status(&current_state, Some(memory))
            }
            None if self.snapshot.actor_told_beliefs.keys().any(|memory_key| {
                memory_key.counterparty == counterparty
                    && match (&memory_key.topic, topic) {
                        (
                            TellTopic::InstitutionalClaim { claim: left_claim },
                            TellTopic::InstitutionalClaim { claim: right_claim },
                        ) => worldwake_core::institutional_claim_same_memory_lane(
                            *left_claim,
                            *right_claim,
                        ),
                        _ => memory_key.topic == *topic,
                    }
            }) =>
            {
                RecipientKnowledgeStatus::SpeakerPreviouslyToldButMemoryExpired
            }
            None => RecipientKnowledgeStatus::UnknownToSpeaker,
        })
    }
}

impl PoliticalBeliefView for PlanningState<'_> {
    fn known_institutional_beliefs(&self, agent: EntityId) -> Vec<BelievedInstitutionalClaim> {
        if agent != self.snapshot.actor() {
            return Vec::new();
        }

        self.snapshot.actor_known_institutional_beliefs.clone()
    }

    fn factions_of(&self, entity: EntityId) -> Vec<EntityId> {
        self.snapshot
            .actor_known_institutional_beliefs
            .iter()
            .filter_map(|belief| match belief.claim {
                worldwake_core::InstitutionalClaim::FactionMembership {
                    faction,
                    member,
                    active: true,
                    ..
                } if member == entity => Some(faction),
                _ => None,
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn bandit_factions_of(&self, entity: EntityId) -> Vec<EntityId> {
        if entity == self.snapshot.actor() {
            return self.snapshot.actor_bandit_factions.clone();
        }

        self.factions_of(entity)
            .into_iter()
            .filter(|faction| self.snapshot.actor_bandit_factions.contains(faction))
            .collect()
    }

    fn locally_observed_bandit_camp_faction_at(
        &self,
        agent: EntityId,
        place: EntityId,
    ) -> Option<EntityId> {
        (agent == self.snapshot.actor() && self.effective_place(agent) == Some(place))
            .then(|| self.bandit_camp_faction_at(place))
            .flatten()
    }

    fn justice_disposition_profile(&self, agent: EntityId) -> Option<JusticeDispositionProfile> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.political.justice_disposition_profile.clone())
    }

    fn violation_disposition_profile(
        &self,
        agent: EntityId,
    ) -> Option<ViolationDispositionProfile> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.political.violation_disposition_profile.clone())
    }

    fn active_violation_records(&self, agent: EntityId) -> Vec<worldwake_core::RecordedViolation> {
        if agent != self.snapshot.actor() {
            return Vec::new();
        }

        self.snapshot.actor_active_violation_records.clone()
    }

    fn record_data(&self, record: EntityId) -> Option<RecordData> {
        PlanningState::record_data(self, record)
    }

    fn office_data(&self, office: EntityId) -> Option<OfficeData> {
        self.snapshot.office_data(office)
    }

    fn believed_office_holder(
        &self,
        office: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        PlanningState::believed_office_holder(self, office)
    }

    fn believed_force_controller(
        &self,
        office: EntityId,
    ) -> InstitutionalBeliefRead<(Option<EntityId>, bool)> {
        PlanningState::believed_force_controller(self, office)
    }

    fn believed_membership(
        &self,
        faction: EntityId,
        member: EntityId,
    ) -> InstitutionalBeliefRead<bool> {
        self.snapshot
            .actor_belief_store
            .believed_membership(faction, member)
    }

    fn believed_faction_rally_point(
        &self,
        faction: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        PlanningState::believed_faction_rally_point(self, faction)
    }

    fn offices_contested_by(&self, claimant: EntityId) -> Vec<EntityId> {
        if claimant != self.snapshot.actor() {
            return Vec::new();
        }

        self.snapshot.actor_contested_offices.clone()
    }

    fn loyalty_to(&self, subject: EntityId, target: EntityId) -> Option<Permille> {
        (subject == self.snapshot.actor())
            .then(|| self.snapshot.actor_loyalties.get(&target).copied())
            .flatten()
    }

    fn believed_support_declaration(
        &self,
        office: EntityId,
        supporter: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        PlanningState::believed_support_declaration(self, office, supporter)
    }

    fn believed_support_declarations_for_office(
        &self,
        office: EntityId,
    ) -> Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)> {
        PlanningState::believed_support_declarations_for_office(self, office)
    }

    fn institutional_belief_claims(
        &self,
        agent: EntityId,
        key: worldwake_core::InstitutionalBeliefKey,
    ) -> Vec<BelievedInstitutionalClaim> {
        if agent != self.snapshot.actor() {
            return Vec::new();
        }

        self.snapshot
            .actor_belief_store
            .get_institutional_beliefs(&key)
            .map_or_else(Vec::new, ToOwned::to_owned)
    }
}

impl RuntimeBeliefView for PlanningState<'_> {}

impl CombatBeliefView for PlanningState<'_> {
    fn combat_profile(&self, agent: EntityId) -> Option<CombatProfile> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.combat.combat_profile)
    }

    fn courage(&self, agent: EntityId) -> Option<Permille> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.combat.courage)
    }

    fn consultation_speed_factor(&self, agent: EntityId) -> Option<Permille> {
        (agent == self.snapshot.actor())
            .then_some(self.snapshot.actor_consultation_speed_factor)
            .flatten()
    }

    fn wounds(&self, agent: EntityId) -> Vec<Wound> {
        self.snapshot
            .entities
            .get(&agent)
            .map(|snapshot| snapshot.combat.wounds.clone())
            .unwrap_or_default()
    }

    fn hostile_targets_of(&self, agent: EntityId) -> Vec<EntityId> {
        let agent_place = self.effective_place(agent);
        let agent_transit = self.in_transit_state(agent);
        self.snapshot
            .entities
            .get(&agent)
            .map(|snapshot| {
                snapshot
                    .combat
                    .hostile_targets
                    .iter()
                    .copied()
                    .filter(|entity| self.is_alive(*entity) && !self.is_dead(*entity))
                    .filter(|entity| {
                        !self
                            .removed_entities
                            .contains(&PlanningEntityRef::Authoritative(*entity))
                    })
                    .filter(|entity| {
                        self.effective_place(*entity) == agent_place
                            || agent_transit.is_some()
                                && self.in_transit_state(*entity) == agent_transit
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId> {
        let agent_place = self.effective_place(agent);
        let agent_transit = self.in_transit_state(agent);
        self.snapshot
            .entities
            .get(&agent)
            .map(|snapshot| {
                snapshot
                    .combat
                    .visible_hostiles
                    .iter()
                    .copied()
                    .filter(|entity| self.is_alive(*entity) && !self.is_dead(*entity))
                    .filter(|entity| {
                        !self
                            .removed_entities
                            .contains(&PlanningEntityRef::Authoritative(*entity))
                    })
                    .filter(|entity| {
                        self.effective_place(*entity) == agent_place
                            || agent_transit.is_some()
                                && self.in_transit_state(*entity) == agent_transit
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId> {
        let agent_place = self.effective_place(agent);
        let agent_transit = self.in_transit_state(agent);
        self.snapshot
            .entities
            .get(&agent)
            .map(|snapshot| {
                snapshot
                    .combat
                    .current_attackers
                    .iter()
                    .copied()
                    .filter(|entity| self.is_alive(*entity) && !self.is_dead(*entity))
                    .filter(|entity| {
                        !self
                            .removed_entities
                            .contains(&PlanningEntityRef::Authoritative(*entity))
                    })
                    .filter(|entity| {
                        self.effective_place(*entity) == agent_place
                            || agent_transit.is_some()
                                && self.in_transit_state(*entity) == agent_transit
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn patrol_profile(&self, agent: EntityId) -> Option<PatrolProfile> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.combat.patrol_profile.clone())
    }

    fn has_wounds(&self, entity: EntityId) -> bool {
        self.snapshot
            .entities
            .get(&entity)
            .is_some_and(|snapshot| !snapshot.combat.wounds.is_empty())
    }
}

impl EconomicBeliefView for PlanningState<'_> {
    fn trade_disposition_profile(&self, agent: EntityId) -> Option<TradeDispositionProfile> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.economic.trade_disposition_profile.clone())
    }

    fn controlled_commodity_quantity_at_place(
        &self,
        agent: EntityId,
        place: EntityId,
        commodity: CommodityKind,
    ) -> Quantity {
        self.local_controlled_lot_refs_for(
            PlanningEntityRef::Authoritative(agent),
            place,
            commodity,
        )
        .into_iter()
        .fold(Quantity(0), |total, entity| {
            let quantity = self.commodity_quantity_ref(entity, commodity);
            Quantity(
                total
                    .0
                    .checked_add(quantity.0)
                    .expect("local controlled commodity quantity overflowed"),
            )
        })
    }

    fn local_controlled_lots_for(
        &self,
        agent: EntityId,
        place: EntityId,
        commodity: CommodityKind,
    ) -> Vec<EntityId> {
        self.local_controlled_lot_refs_for(
            PlanningEntityRef::Authoritative(agent),
            place,
            commodity,
        )
        .into_iter()
        .filter_map(|entity| match entity {
            PlanningEntityRef::Authoritative(entity) => Some(entity),
            PlanningEntityRef::Hypothetical(_) => None,
        })
        .collect()
    }

    fn listed_sale_lots_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
        let mut lots = self
            .entities_at(place)
            .into_iter()
            .filter(|entity| self.entity_kind(*entity) == Some(EntityKind::ItemLot))
            .filter(|entity| self.item_lot_commodity(*entity) == Some(commodity))
            .filter(|entity| self.has_sale_listing(*entity))
            .filter(|entity| {
                self.seller_for_sale_lot(*entity).is_some_and(|seller| {
                    self.is_alive(seller) && self.effective_place(seller) == Some(place)
                })
            })
            .collect::<Vec<_>>();
        lots.sort();
        lots.dedup();
        lots
    }

    fn seller_for_sale_lot(&self, lot: EntityId) -> Option<EntityId> {
        if !self.has_sale_listing(lot) {
            return None;
        }
        self.sale_seller_overrides
            .get(&PlanningEntityRef::Authoritative(lot))
            .copied()
            .flatten()
            .or_else(|| {
                self.snapshot
                    .entities
                    .get(&lot)
                    .and_then(|snapshot| snapshot.economic.seller_for_sale_lot)
            })
    }

    fn has_sale_listing(&self, lot: EntityId) -> bool {
        self.sale_listing_overrides
            .get(&PlanningEntityRef::Authoritative(lot))
            .copied()
            .unwrap_or_else(|| {
                self.snapshot
                    .entities
                    .get(&lot)
                    .is_some_and(|snapshot| snapshot.economic.has_sale_listing)
            })
    }

    fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation> {
        self.snapshot
            .entities
            .get(&agent)
            .map(|snapshot| snapshot.economic.demand_memory.clone())
            .unwrap_or_default()
    }

    fn merchandise_profile(&self, agent: EntityId) -> Option<worldwake_core::MerchandiseProfile> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.economic.merchandise_profile.clone())
    }
}

impl InventoryBeliefView for PlanningState<'_> {
    fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId> {
        let mut entities = self
            .snapshot
            .entities
            .keys()
            .copied()
            .filter(|entity| self.direct_possessor(*entity) == Some(holder))
            .filter(|entity| {
                !self
                    .removed_entities
                    .contains(&PlanningEntityRef::Authoritative(*entity))
            })
            .collect::<Vec<_>>();
        entities.sort();
        entities.dedup();
        entities
    }

    fn knows_recipe(&self, actor: EntityId, recipe: RecipeId) -> bool {
        self.known_recipes(actor).contains(&recipe)
    }

    fn unique_item_count(&self, holder: EntityId, kind: UniqueItemKind) -> u32 {
        self.snapshot
            .entities
            .get(&holder)
            .and_then(|snapshot| snapshot.inventory.unique_item_counts.get(&kind).copied())
            .unwrap_or(0)
    }

    fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity {
        self.commodity_quantity_ref(PlanningEntityRef::Authoritative(holder), kind)
    }

    fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind> {
        self.item_lot_commodity_ref(PlanningEntityRef::Authoritative(entity))
    }

    fn item_lot_consumable_profile(
        &self,
        entity: EntityId,
    ) -> Option<worldwake_core::CommodityConsumableProfile> {
        self.snapshot
            .entities
            .get(&entity)
            .and_then(|snapshot| snapshot.inventory.item_lot_consumable_profile)
    }

    fn direct_container(&self, entity: EntityId) -> Option<EntityId> {
        self.direct_container_ref(PlanningEntityRef::Authoritative(entity))
            .and_then(|entity| match entity {
                PlanningEntityRef::Authoritative(entity) => Some(entity),
                PlanningEntityRef::Hypothetical(_) => None,
            })
    }

    fn direct_possessor(&self, entity: EntityId) -> Option<EntityId> {
        self.direct_possessor_ref(PlanningEntityRef::Authoritative(entity))
            .and_then(|entity| match entity {
                PlanningEntityRef::Authoritative(entity) => Some(entity),
                PlanningEntityRef::Hypothetical(_) => None,
            })
    }

    fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits> {
        self.carry_capacity_ref(PlanningEntityRef::Authoritative(entity))
    }

    fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits> {
        self.load_of_entity_ref(PlanningEntityRef::Authoritative(entity))
    }

    fn known_recipes(&self, agent: EntityId) -> Vec<RecipeId> {
        self.snapshot
            .entities
            .get(&agent)
            .map(|snapshot| snapshot.inventory.known_recipes.clone())
            .unwrap_or_default()
    }

    fn believed_commodity_stock(
        &self,
        agent: EntityId,
        place: EntityId,
        kind: CommodityKind,
    ) -> worldwake_sim::belief_view::BeliefValue<Quantity> {
        if agent != self.snapshot.actor() {
            return worldwake_sim::belief_view::stale_default_value(Quantity(0));
        }

        worldwake_sim::belief_view::project_claims_into_belief_set(
            self.snapshot
                .actor_belief_store
                .get_entity_claims(&place)
                .into_iter()
                .flatten()
                .filter_map(|claim| {
                    worldwake_sim::belief_view::inventory_claim_value(claim, kind)
                        .map(|value| (claim.clone(), value))
                }),
            self.snapshot.current_tick,
            self.snapshot.actor_claim_confidence_threshold,
            &self.snapshot.actor_confidence_policy,
        )
        .best
        .unwrap_or_else(|| worldwake_sim::belief_view::stale_default_value(Quantity(0)))
    }
}

impl FacilityBeliefView for PlanningState<'_> {
    fn workstation_tag(&self, entity: EntityId) -> Option<WorkstationTag> {
        self.snapshot
            .entities
            .get(&entity)
            .and_then(|snapshot| snapshot.facility.workstation_tag)
    }

    fn stock_storage_policy(
        &self,
        facility: EntityId,
    ) -> Option<worldwake_core::StockStoragePolicy> {
        self.stock_storage_policy_snapshot(facility)
    }

    fn resource_source(&self, entity: EntityId) -> Option<ResourceSource> {
        let mut source = self
            .snapshot
            .entities
            .get(&entity)
            .and_then(|snapshot| snapshot.facility.resource_source.clone())?;
        if let Some(quantity) = self.resource_quantity_overrides.get(&entity).copied() {
            source.available_quantity = quantity;
        }
        Some(source)
    }

    fn has_production_job(&self, entity: EntityId) -> bool {
        self.snapshot
            .entities
            .get(&entity)
            .is_some_and(|snapshot| snapshot.facility.has_production_job)
    }

    fn matching_workstations_at(&self, place: EntityId, tag: WorkstationTag) -> Vec<EntityId> {
        self.entities_at(place)
            .into_iter()
            .filter(|entity| self.workstation_tag(*entity) == Some(tag))
            .collect()
    }

    fn resource_sources_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
        self.entities_at(place)
            .into_iter()
            .filter(|entity| {
                self.resource_source(*entity)
                    .is_some_and(|source| source.commodity == commodity)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{HypotheticalEntityId, PlanningEntityRef, PlanningState};
    use crate::planner_duration_contract::PlannerDurationDependency;
    use crate::planning_snapshot::build_planning_snapshot;
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::ActionDomain;
    use worldwake_core::{
        ActionDefId, AgentBeliefStore, ArtifactPostingProfile, BeliefConfidencePolicy,
        BelievedActivity, BelievedEntityState, BodyCostPerTick, ClaimId, ClaimValue, CombatProfile,
        CommodityConsumableProfile, CommodityKind, ContentionGrant, DemandObservation,
        DemandObservationReason, DisposalProfile, DriveThresholds, EntityBeliefAspect,
        EntityBeliefClaim, EntityId, EntityKind, EpistemicDispositionProfile, HomeostaticNeeds,
        InTransitOnEdge, InstitutionalBeliefRead, JusticeDispositionProfile, LoadUnits,
        MerchandiseProfile, MetabolismProfile, OfficeData, PatrolProfile, PatrolRoute,
        PerceptionSource, Permille, Quantity, RecipeId, RecipientKnowledgeStatus, RecordData,
        RecordKind, ResourceSource, SharedTellState, SuccessionLaw, TellMemoryKey, TellProfile,
        TellTopic, TheftDispositionProfile, Tick, TickRange, ToldBeliefMemory,
        TradeDispositionProfile, UniqueItemKind, ViolationDispositionProfile, WorkstationTag,
        Wound, WoundCause, WoundId,
    };
    use worldwake_sim::{
        ActionDef, ActionDefRegistry, ActionDuration, ActionError, ActionHandler, ActionHandlerId,
        ActionHandlerRegistry, ActionPayload, ActionProgress, ActionState, CombatActionPayload,
        CombatBeliefView, Constraint, ControlBeliefView, DeterministicRng, DurationExpr,
        EconomicBeliefView, EntityBeliefView, GoalBeliefView, Interruptibility,
        PoliticalBeliefView, Precondition, ProfileBeliefView, ReservationReq, RuntimeBeliefView,
        SocialBeliefView, SpatialBeliefView, TargetSpec, TemporalBeliefView,
        estimate_duration_from_beliefs, get_affordances,
    };
    use worldwake_systems::register_office_actions;

    struct StubBeliefView {
        current_tick: Tick,
        alive: BTreeMap<EntityId, bool>,
        kinds: BTreeMap<EntityId, EntityKind>,
        effective_places: BTreeMap<EntityId, EntityId>,
        entities_at: BTreeMap<EntityId, Vec<EntityId>>,
        beliefs: BTreeMap<EntityId, Vec<(EntityId, BelievedEntityState)>>,
        belief_stores: BTreeMap<EntityId, AgentBeliefStore>,
        direct_possessions: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessors: BTreeMap<EntityId, EntityId>,
        direct_containers: BTreeMap<EntityId, EntityId>,
        adjacent: BTreeMap<EntityId, Vec<(EntityId, NonZeroU32)>>,
        item_lot_commodities: BTreeMap<EntityId, CommodityKind>,
        consumable_profiles: BTreeMap<EntityId, CommodityConsumableProfile>,
        commodity_quantities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        carry_capacities: BTreeMap<EntityId, LoadUnits>,
        entity_loads: BTreeMap<EntityId, LoadUnits>,
        resource_sources: BTreeMap<EntityId, ResourceSource>,
        needs: BTreeMap<EntityId, HomeostaticNeeds>,
        thresholds: BTreeMap<EntityId, DriveThresholds>,
        metabolism_profiles: BTreeMap<EntityId, MetabolismProfile>,
        disposal_profiles: BTreeMap<EntityId, DisposalProfile>,
        artifact_posting_profiles: BTreeMap<EntityId, ArtifactPostingProfile>,
        trade_profiles: BTreeMap<EntityId, TradeDispositionProfile>,
        patrol_profiles: BTreeMap<EntityId, PatrolProfile>,
        patrol_routes: BTreeMap<EntityId, PatrolRoute>,
        epistemic_profiles: BTreeMap<EntityId, EpistemicDispositionProfile>,
        theft_profiles: BTreeMap<EntityId, TheftDispositionProfile>,
        justice_profiles: BTreeMap<EntityId, JusticeDispositionProfile>,
        violation_profiles: BTreeMap<EntityId, ViolationDispositionProfile>,
        demand_memory: BTreeMap<EntityId, Vec<DemandObservation>>,
        merchandise_profiles: BTreeMap<EntityId, MerchandiseProfile>,
        tell_profiles: BTreeMap<EntityId, TellProfile>,
        told_beliefs: BTreeMap<EntityId, Vec<(TellMemoryKey, ToldBeliefMemory)>>,
        reservations: BTreeMap<EntityId, Vec<TickRange>>,
        durations: BTreeMap<(EntityId, ActionDefId), ActionDuration>,
        wounds: BTreeMap<EntityId, Vec<Wound>>,
        hostiles: BTreeMap<EntityId, Vec<EntityId>>,
        attackers: BTreeMap<EntityId, Vec<EntityId>>,
        bandit_factions_by_member: BTreeMap<EntityId, Vec<EntityId>>,
        record_data: BTreeMap<EntityId, RecordData>,
        consultation_speed_factors: BTreeMap<EntityId, Permille>,
        combat_profiles: BTreeMap<EntityId, CombatProfile>,
        bandit_flee_thresholds: BTreeMap<EntityId, Permille>,
        bandit_establishment_ticks: BTreeMap<EntityId, NonZeroU32>,
        facility_queue_positions: BTreeMap<(EntityId, EntityId), u32>,
        facility_grants: BTreeMap<EntityId, ContentionGrant>,
        courages: BTreeMap<EntityId, Permille>,
        office_holder_beliefs: BTreeMap<EntityId, InstitutionalBeliefRead<Option<EntityId>>>,
        faction_rally_point_beliefs: BTreeMap<EntityId, InstitutionalBeliefRead<Option<EntityId>>>,
        support_declaration_beliefs:
            BTreeMap<(EntityId, EntityId), InstitutionalBeliefRead<Option<EntityId>>>,
        claim_confidence_thresholds: BTreeMap<EntityId, Permille>,
        office_data: BTreeMap<EntityId, OfficeData>,
    }

    impl Default for StubBeliefView {
        fn default() -> Self {
            Self {
                current_tick: Tick(0),
                alive: BTreeMap::new(),
                kinds: BTreeMap::new(),
                effective_places: BTreeMap::new(),
                entities_at: BTreeMap::new(),
                beliefs: BTreeMap::new(),
                belief_stores: BTreeMap::new(),
                direct_possessions: BTreeMap::new(),
                direct_possessors: BTreeMap::new(),
                direct_containers: BTreeMap::new(),
                adjacent: BTreeMap::new(),
                item_lot_commodities: BTreeMap::new(),
                consumable_profiles: BTreeMap::new(),
                commodity_quantities: BTreeMap::new(),
                carry_capacities: BTreeMap::new(),
                entity_loads: BTreeMap::new(),
                resource_sources: BTreeMap::new(),
                needs: BTreeMap::new(),
                thresholds: BTreeMap::new(),
                metabolism_profiles: BTreeMap::new(),
                disposal_profiles: BTreeMap::new(),
                artifact_posting_profiles: BTreeMap::new(),
                trade_profiles: BTreeMap::new(),
                patrol_profiles: BTreeMap::new(),
                patrol_routes: BTreeMap::new(),
                epistemic_profiles: BTreeMap::new(),
                theft_profiles: BTreeMap::new(),
                justice_profiles: BTreeMap::new(),
                violation_profiles: BTreeMap::new(),
                demand_memory: BTreeMap::new(),
                merchandise_profiles: BTreeMap::new(),
                tell_profiles: BTreeMap::new(),
                told_beliefs: BTreeMap::new(),
                reservations: BTreeMap::new(),
                durations: BTreeMap::new(),
                wounds: BTreeMap::new(),
                hostiles: BTreeMap::new(),
                attackers: BTreeMap::new(),
                bandit_factions_by_member: BTreeMap::new(),
                record_data: BTreeMap::new(),
                consultation_speed_factors: BTreeMap::new(),
                combat_profiles: BTreeMap::new(),
                bandit_flee_thresholds: BTreeMap::new(),
                bandit_establishment_ticks: BTreeMap::new(),
                facility_queue_positions: BTreeMap::new(),
                facility_grants: BTreeMap::new(),
                courages: BTreeMap::new(),
                office_holder_beliefs: BTreeMap::new(),
                faction_rally_point_beliefs: BTreeMap::new(),
                support_declaration_beliefs: BTreeMap::new(),
                claim_confidence_thresholds: BTreeMap::new(),
                office_data: BTreeMap::new(),
            }
        }
    }

    impl ControlBeliefView for StubBeliefView {
        fn believed_owner_of(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
            actor == entity
                || <Self as worldwake_sim::InventoryBeliefView>::direct_possessor(self, entity)
                    == Some(actor)
        }

        fn has_control(&self, entity: EntityId) -> bool {
            self.kinds.get(&entity) == Some(&EntityKind::Agent)
        }
    }

    impl EntityBeliefView for StubBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.get(&entity).copied().unwrap_or(false)
        }

        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.kinds.get(&entity).copied()
        }

        fn bandit_flee_wound_threshold(&self, faction: EntityId) -> Option<Permille> {
            self.bandit_flee_thresholds.get(&faction).copied()
        }

        fn bandit_camp_establishment_ticks(&self, faction: EntityId) -> Option<NonZeroU32> {
            self.bandit_establishment_ticks.get(&faction).copied()
        }

        fn is_dead(&self, entity: EntityId) -> bool {
            !EntityBeliefView::is_alive(self, entity)
        }

        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }

        fn corpse_entities_at(&self, place: EntityId) -> Vec<EntityId> {
            SpatialBeliefView::entities_at(self, place)
                .into_iter()
                .filter(|entity| EntityBeliefView::is_dead(self, *entity))
                .collect()
        }
    }

    impl ProfileBeliefView for StubBeliefView {
        fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
            self.needs.get(&agent).copied()
        }

        fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
            self.thresholds.get(&agent).copied()
        }

        fn metabolism_profile(&self, agent: EntityId) -> Option<MetabolismProfile> {
            self.metabolism_profiles.get(&agent).copied()
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
            SpatialBeliefView::adjacent_places_with_travel_ticks(self, place)
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

        fn facility_queue_position(&self, facility: EntityId, actor: EntityId) -> Option<u32> {
            self.facility_queue_positions
                .get(&(facility, actor))
                .copied()
        }

        fn facility_grant(&self, facility: EntityId) -> Option<&ContentionGrant> {
            self.facility_grants.get(&facility)
        }

        fn reservation_conflicts(&self, entity: EntityId, range: TickRange) -> bool {
            self.reservations
                .get(&entity)
                .into_iter()
                .flatten()
                .any(|existing| existing.overlaps(&range))
        }

        fn reservation_ranges(&self, entity: EntityId) -> Vec<TickRange> {
            self.reservations.get(&entity).cloned().unwrap_or_default()
        }

        fn estimate_duration(
            &self,
            actor: EntityId,
            _duration: &DurationExpr,
            targets: &[EntityId],
            _payload: &ActionPayload,
        ) -> Option<ActionDuration> {
            let def_id = ActionDefId(targets.first().map_or(0, |target| target.slot));
            self.durations.get(&(actor, def_id)).copied()
        }
    }

    impl RuntimeBeliefView for StubBeliefView {}

    impl SocialBeliefView for StubBeliefView {
        fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
            self.beliefs.get(&agent).cloned().unwrap_or_default()
        }

        fn agent_belief_store(&self, agent: EntityId) -> Option<&AgentBeliefStore> {
            self.belief_stores.get(&agent)
        }

        fn claim_confidence_threshold(&self, agent: EntityId) -> Permille {
            self.claim_confidence_thresholds
                .get(&agent)
                .copied()
                .unwrap_or(Permille::ZERO)
        }

        fn belief_confidence_policy(
            &self,
            _agent: EntityId,
        ) -> worldwake_core::BeliefConfidencePolicy {
            worldwake_core::BeliefConfidencePolicy::default()
        }

        fn epistemic_disposition_profile(
            &self,
            agent: EntityId,
        ) -> Option<EpistemicDispositionProfile> {
            self.epistemic_profiles.get(&agent).cloned()
        }

        fn theft_disposition_profile(&self, agent: EntityId) -> Option<TheftDispositionProfile> {
            self.theft_profiles.get(&agent).cloned()
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
    }

    impl PoliticalBeliefView for StubBeliefView {
        fn justice_disposition_profile(
            &self,
            agent: EntityId,
        ) -> Option<JusticeDispositionProfile> {
            self.justice_profiles.get(&agent).cloned()
        }

        fn violation_disposition_profile(
            &self,
            agent: EntityId,
        ) -> Option<ViolationDispositionProfile> {
            self.violation_profiles.get(&agent).cloned()
        }

        fn believed_office_holder(
            &self,
            office: EntityId,
        ) -> InstitutionalBeliefRead<Option<EntityId>> {
            self.office_holder_beliefs
                .get(&office)
                .cloned()
                .unwrap_or(InstitutionalBeliefRead::Unknown)
        }

        fn believed_faction_rally_point(
            &self,
            faction: EntityId,
        ) -> InstitutionalBeliefRead<Option<EntityId>> {
            self.faction_rally_point_beliefs
                .get(&faction)
                .cloned()
                .unwrap_or(InstitutionalBeliefRead::Unknown)
        }

        fn believed_support_declaration(
            &self,
            office: EntityId,
            supporter: EntityId,
        ) -> InstitutionalBeliefRead<Option<EntityId>> {
            self.support_declaration_beliefs
                .get(&(office, supporter))
                .cloned()
                .unwrap_or(InstitutionalBeliefRead::Unknown)
        }

        fn believed_support_declarations_for_office(
            &self,
            office: EntityId,
        ) -> Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)> {
            self.support_declaration_beliefs
                .iter()
                .filter_map(|(&(belief_office, supporter), read)| {
                    (belief_office == office).then_some((supporter, read.clone()))
                })
                .collect()
        }

        fn bandit_factions_of(&self, entity: EntityId) -> Vec<EntityId> {
            self.bandit_factions_by_member
                .get(&entity)
                .cloned()
                .unwrap_or_default()
        }

        fn record_data(&self, record: EntityId) -> Option<RecordData> {
            self.record_data.get(&record).cloned()
        }

        fn office_data(&self, office: EntityId) -> Option<OfficeData> {
            self.office_data.get(&office).cloned()
        }
    }

    impl CombatBeliefView for StubBeliefView {
        fn has_wounds(&self, entity: EntityId) -> bool {
            self.wounds
                .get(&entity)
                .is_some_and(|wounds| !wounds.is_empty())
        }

        fn patrol_profile(&self, agent: EntityId) -> Option<PatrolProfile> {
            self.patrol_profiles.get(&agent).cloned()
        }

        fn combat_profile(&self, agent: EntityId) -> Option<CombatProfile> {
            self.combat_profiles.get(&agent).copied()
        }

        fn courage(&self, agent: EntityId) -> Option<Permille> {
            self.courages.get(&agent).copied()
        }

        fn consultation_speed_factor(&self, agent: EntityId) -> Option<Permille> {
            self.consultation_speed_factors.get(&agent).copied()
        }

        fn wounds(&self, agent: EntityId) -> Vec<Wound> {
            self.wounds.get(&agent).cloned().unwrap_or_default()
        }

        fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId> {
            self.hostiles.get(&agent).cloned().unwrap_or_default()
        }

        fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId> {
            self.attackers.get(&agent).cloned().unwrap_or_default()
        }
    }

    impl EconomicBeliefView for StubBeliefView {
        fn controlled_commodity_quantity_at_place(
            &self,
            actor: EntityId,
            place: EntityId,
            commodity: CommodityKind,
        ) -> Quantity {
            EconomicBeliefView::local_controlled_lots_for(self, actor, place, commodity)
                .into_iter()
                .fold(Quantity(0), |total, entity| {
                    let quantity = self
                        .commodity_quantities
                        .get(&(entity, commodity))
                        .copied()
                        .unwrap_or(Quantity(0));
                    Quantity(total.0 + quantity.0)
                })
        }

        fn local_controlled_lots_for(
            &self,
            actor: EntityId,
            place: EntityId,
            commodity: CommodityKind,
        ) -> Vec<EntityId> {
            let mut entities = SpatialBeliefView::entities_at(self, place)
                .into_iter()
                .filter(|entity| {
                    <Self as worldwake_sim::InventoryBeliefView>::item_lot_commodity(self, *entity)
                        == Some(commodity)
                })
                .filter(|entity| ControlBeliefView::can_control(self, actor, *entity))
                .collect::<Vec<_>>();
            entities.sort();
            entities.dedup();
            entities
        }

        fn trade_disposition_profile(&self, agent: EntityId) -> Option<TradeDispositionProfile> {
            self.trade_profiles.get(&agent).cloned()
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

        fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation> {
            self.demand_memory.get(&agent).cloned().unwrap_or_default()
        }

        fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile> {
            self.merchandise_profiles.get(&agent).cloned()
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
            entity: EntityId,
        ) -> Option<CommodityConsumableProfile> {
            self.consumable_profiles.get(&entity).copied()
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

        fn resource_source(&self, entity: EntityId) -> Option<ResourceSource> {
            self.resource_sources.get(&entity).cloned()
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

        fn resource_sources_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            SpatialBeliefView::entities_at(self, place)
                .into_iter()
                .filter(|entity| {
                    self.resource_sources
                        .get(entity)
                        .is_some_and(|source| source.commodity == commodity)
                })
                .collect()
        }
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn belief_with_activity(
        place: EntityId,
        domain: ActionDomain,
        target: Option<EntityId>,
        observed_tick: u64,
    ) -> BelievedEntityState {
        BelievedEntityState {
            believed_kind: None,
            last_known_place: Some(place),
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: Some(BelievedActivity {
                action_domain: domain,
                target,
                observed_tick: Tick(observed_tick),
            }),
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            ..BelievedEntityState::single_observation_defaults(
                Tick(observed_tick),
                worldwake_core::PerceptionSource::DirectObservation,
            )
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_start(
        _def: &ActionDef,
        _instance: &mut worldwake_sim::ActionInstance,
        _context: &worldwake_sim::ActionExecutionContext<'_>,
        _rng: &mut DeterministicRng,
        _txn: &mut worldwake_core::WorldTxn<'_>,
    ) -> Result<Option<ActionState>, ActionError> {
        Ok(None)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_tick(
        _def: &ActionDef,
        _instance: &mut worldwake_sim::ActionInstance,
        _context: &worldwake_sim::ActionExecutionContext<'_>,
        _rng: &mut DeterministicRng,
        _txn: &mut worldwake_core::WorldTxn<'_>,
    ) -> Result<ActionProgress, ActionError> {
        Ok(ActionProgress::Continue)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_commit(
        _def: &ActionDef,
        _instance: &worldwake_sim::ActionInstance,
        _context: &worldwake_sim::ActionExecutionContext<'_>,
        _event_log: &worldwake_core::EventLog,
        _rng: &mut DeterministicRng,
        _txn: &mut worldwake_core::WorldTxn<'_>,
    ) -> Result<worldwake_sim::CommitOutcome, ActionError> {
        Ok(worldwake_sim::CommitOutcome::empty())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_abort(
        _def: &ActionDef,
        _instance: &worldwake_sim::ActionInstance,
        _context: &worldwake_sim::ActionExecutionContext<'_>,
        _reason: &worldwake_sim::AbortReason,
        _event_log: &worldwake_core::EventLog,
        _rng: &mut DeterministicRng,
        _txn: &mut worldwake_core::WorldTxn<'_>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    fn sample_registry() -> (ActionDefRegistry, ActionHandlerRegistry) {
        let mut registry = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(ActionHandler::new(
            noop_start,
            noop_tick,
            noop_commit,
            noop_abort,
        ));
        registry.register(ActionDef {
            id: ActionDefId(0),
            name: "eat".to_string(),
            domain: ActionDomain::Needs,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: vec![TargetSpec::EntityDirectlyPossessedByActor {
                kind: EntityKind::ItemLot,
            }],
            preconditions: vec![
                Precondition::TargetCommodity {
                    target_index: 0,
                    kind: CommodityKind::Bread,
                },
                Precondition::TargetHasConsumableEffect {
                    target_index: 0,
                    effect: worldwake_sim::ConsumableEffect::Hunger,
                },
            ],
            reservation_requirements: vec![ReservationReq { target_index: 0 }],
            duration: DurationExpr::Fixed(NonZeroU32::new(3).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![Precondition::ActorAlive],
            visibility: worldwake_core::VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
        });
        (registry, handlers)
    }

    fn test_view() -> (StubBeliefView, EntityId, EntityId, EntityId, EntityId) {
        let actor = entity(1);
        let town = entity(10);
        let field = entity(11);
        let bread = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(town, true);
        view.alive.insert(field, true);
        view.alive.insert(bread, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(field, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(bread, town);
        view.entities_at.insert(town, vec![actor, bread]);
        view.entities_at.insert(field, vec![]);
        view.direct_possessions.insert(actor, vec![bread]);
        view.direct_possessors.insert(bread, actor);
        view.item_lot_commodities
            .insert(bread, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread,
            CommodityConsumableProfile::new(NonZeroU32::new(2).unwrap(), pm(250), pm(0), pm(0)),
        );
        view.carry_capacities.insert(actor, LoadUnits(10));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.entity_loads.insert(bread, LoadUnits(1));
        view.commodity_quantities
            .insert((actor, CommodityKind::Bread), Quantity(1));
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(1));
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(700), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        view.demand_memory.insert(
            actor,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(2),
                place: town,
                tick: Tick(3),
                counterparty: None,
                reason: DemandObservationReason::WantedToBuyButNoSeller,
            }],
        );
        view.resource_sources.insert(
            bread,
            ResourceSource {
                commodity: CommodityKind::Bread,
                available_quantity: Quantity(4),
                max_quantity: Quantity(4),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        );
        view.adjacent
            .insert(town, vec![(field, NonZeroU32::new(5).unwrap())]);
        view.adjacent
            .insert(field, vec![(town, NonZeroU32::new(5).unwrap())]);
        view.wounds.insert(
            actor,
            vec![Wound {
                id: WoundId(1),
                body_part: worldwake_core::BodyPart::Torso,
                cause: WoundCause::Deprivation(worldwake_core::DeprivationKind::Starvation),
                severity: pm(200),
                inflicted_at: Tick(1),
                bleed_rate_per_tick: pm(0),
            }],
        );
        (view, actor, town, field, bread)
    }

    #[test]
    fn planning_state_implements_goal_and_runtime_surfaces() {
        fn assert_goal<T: GoalBeliefView>() {}
        fn assert_runtime<T: RuntimeBeliefView>() {}
        assert_goal::<PlanningState<'_>>();
        assert_runtime::<PlanningState<'_>>();
    }

    #[test]
    fn planning_state_without_overrides_matches_snapshot_answers() {
        let (view, actor, town, _field, bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            SpatialBeliefView::effective_place(&state, actor),
            Some(town)
        );
        assert_eq!(
            worldwake_sim::InventoryBeliefView::direct_possessions(&state, actor),
            vec![bread]
        );
        assert_eq!(
            worldwake_sim::InventoryBeliefView::commodity_quantity(
                &state,
                actor,
                CommodityKind::Bread,
            ),
            Quantity(1)
        );
        assert_eq!(
            EconomicBeliefView::demand_memory(&state, actor),
            EconomicBeliefView::demand_memory(&view, actor)
        );
    }

    #[test]
    fn dead_entities_retain_kind_for_planning_queries() {
        let (mut view, actor, town, _field, _bread) = test_view();
        let corpse = entity(30);
        view.alive.insert(corpse, false);
        view.kinds.insert(corpse, EntityKind::Agent);
        view.effective_places.insert(corpse, town);
        view.entities_at.entry(town).or_default().push(corpse);

        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::from([corpse]), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            EntityBeliefView::entity_kind(&state, corpse),
            Some(EntityKind::Agent)
        );
        assert!(EntityBeliefView::is_dead(&state, corpse));
        assert_eq!(
            SpatialBeliefView::effective_place(&state, corpse),
            Some(town)
        );
    }

    #[test]
    fn planning_state_queue_and_grant_queries_read_snapshot_data() {
        let (view, actor, _town, field, _bread) = test_view();
        let other = entity(99);
        let mut view = view;
        view.facility_queue_positions.insert((field, actor), 2);
        view.facility_grants.insert(
            field,
            ContentionGrant {
                actor: other,
                intended_action: ActionDefId(7),
                granted_at: Tick(3),
                expires_at: Tick(6),
            },
        );
        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::from([field]), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(state.facility_queue_position(field, actor), Some(2));
        assert_eq!(
            state.facility_grant(field),
            Some(&ContentionGrant {
                actor: other,
                intended_action: ActionDefId(7),
                granted_at: Tick(3),
                expires_at: Tick(6),
            })
        );
    }

    #[test]
    fn planning_state_queue_queries_remain_conservative_for_other_actors() {
        let (view, actor, _town, field, _bread) = test_view();
        let other = entity(99);
        let mut view = view;
        view.facility_queue_positions.insert((field, actor), 1);
        view.facility_queue_positions.insert((field, other), 0);
        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::from([field]), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(state.facility_queue_position(field, actor), Some(1));
        assert_eq!(state.facility_queue_position(field, other), None);
    }

    #[test]
    fn simulated_queue_join_marks_actor_as_queued_without_fabricating_position() {
        let (view, actor, _town, field, _bread) = test_view();
        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::from([field]), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot).simulate_queue_join(field, ActionDefId(44));

        assert!(state.is_actor_queued_at_facility(field));
        assert_eq!(state.facility_queue_position(field, actor), None);
        assert!(!state.has_actor_facility_grant(field, ActionDefId(44)));
    }

    #[test]
    fn simulated_grant_received_sets_matching_grant_and_clears_queue_membership() {
        let (view, actor, _town, field, _bread) = test_view();
        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::from([field]), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot)
            .simulate_queue_join(field, ActionDefId(44))
            .simulate_grant_received(field, ActionDefId(44));

        assert!(!state.is_actor_queued_at_facility(field));
        assert!(state.has_actor_facility_grant(field, ActionDefId(44)));
        assert_eq!(
            state.facility_grant(field),
            Some(&ContentionGrant {
                actor,
                intended_action: ActionDefId(44),
                granted_at: Tick(0),
                expires_at: Tick(0),
            })
        );
    }

    #[test]
    fn simulated_grant_consumed_clears_grant_without_mutating_snapshot() {
        let (view, actor, _town, field, _bread) = test_view();
        let mut view = view;
        view.facility_grants.insert(
            field,
            ContentionGrant {
                actor,
                intended_action: ActionDefId(44),
                granted_at: Tick(3),
                expires_at: Tick(6),
            },
        );
        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::from([field]), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot).simulate_grant_consumed(field);

        assert_eq!(state.facility_grant(field), None);
        assert_eq!(
            snapshot
                .entities
                .get(&field)
                .and_then(|entity| entity.temporal.facility_queue.as_ref())
                .and_then(|queue| queue.active_grant.as_ref()),
            Some(&ContentionGrant {
                actor,
                intended_action: ActionDefId(44),
                granted_at: Tick(3),
                expires_at: Tick(6),
            })
        );
    }

    #[test]
    fn movement_and_possession_overrides_update_effective_queries() {
        let (view, actor, _town, field, bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot)
            .move_actor_to(field)
            .move_lot_to_holder(bread, actor, CommodityKind::Bread, Quantity(1));

        assert_eq!(
            SpatialBeliefView::effective_place(&state, actor),
            Some(field)
        );
        assert_eq!(
            SpatialBeliefView::effective_place(&state, bread),
            Some(field)
        );
        assert_eq!(
            SpatialBeliefView::entities_at(&state, field),
            vec![actor, bread]
        );
        assert_eq!(
            worldwake_sim::InventoryBeliefView::direct_possessions(&state, actor),
            vec![bread]
        );
    }

    #[test]
    fn resource_and_reservation_overrides_are_visible() {
        let (view, actor, _town, _field, bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let range = TickRange::new(Tick(4), Tick(6)).unwrap();
        let state = PlanningState::new(&snapshot)
            .use_resource(bread, Quantity(1))
            .reserve(bread, range);

        assert_eq!(
            worldwake_sim::FacilityBeliefView::resource_source(&state, bread)
                .map(|source| source.available_quantity),
            Some(Quantity(1))
        );
        assert!(state.reservation_conflicts(bread, range));
    }

    #[test]
    fn removing_target_updates_lifecycle_and_affordances() {
        let (view, actor, _town, _field, bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let (registry, handlers) = sample_registry();

        let base = PlanningState::new(&snapshot);
        let removed = base.clone().mark_removed(bread);

        assert_eq!(get_affordances(&base, actor, &registry, &handlers).len(), 1);
        assert!(EntityBeliefView::is_dead(&removed, bread));
        assert!(!EntityBeliefView::is_alive(&removed, bread));
        assert!(
            SpatialBeliefView::entities_at(&removed, entity(10))
                .iter()
                .all(|entity| *entity != bread)
        );
        assert!(get_affordances(&removed, actor, &registry, &handlers).is_empty());
    }

    #[test]
    fn consume_override_reduces_hunger_conservatively() {
        let (view, actor, _town, _field, _bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot).consume_commodity(CommodityKind::Bread);
        let thresholds = ProfileBeliefView::drive_thresholds(&state, actor).unwrap();

        assert!(
            ProfileBeliefView::homeostatic_needs(&state, actor)
                .unwrap()
                .hunger
                < thresholds.hunger.low()
        );
    }

    #[test]
    fn consume_override_applies_all_relieved_drive_bands_for_multi_effect_food() {
        let (view, actor, _town, _field, _bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot).consume_commodity(CommodityKind::Apple);
        let thresholds = ProfileBeliefView::drive_thresholds(&state, actor).unwrap();
        let needs = ProfileBeliefView::homeostatic_needs(&state, actor).unwrap();

        assert!(needs.hunger < thresholds.hunger.low());
        assert!(needs.thirst < thresholds.thirst.low());
    }

    #[test]
    fn planning_state_preserves_bandit_policy_queries_from_snapshot() {
        let (mut view, actor, _town, _field, _bread) = test_view();
        let faction = entity(77);
        let flee_threshold = Permille::new(650).unwrap();
        let establish_ticks = NonZeroU32::new(8).unwrap();
        view.bandit_factions_by_member.insert(actor, vec![faction]);
        view.bandit_flee_thresholds.insert(faction, flee_threshold);
        view.bandit_establishment_ticks
            .insert(faction, establish_ticks);
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            EntityBeliefView::bandit_flee_wound_threshold(&state, faction),
            Some(flee_threshold)
        );
        assert_eq!(
            EntityBeliefView::bandit_camp_establishment_ticks(&state, faction),
            Some(establish_ticks)
        );
    }

    #[test]
    fn planning_state_preserves_actor_belief_memory_and_tell_profile_from_snapshot() {
        let (mut view, actor, town, _field, bread) = test_view();
        view.beliefs.insert(
            actor,
            vec![(
                bread,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(town),
                    last_known_inventory: BTreeMap::from([(CommodityKind::Bread, Quantity(1))]),
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
                        Tick(4),
                        worldwake_core::PerceptionSource::DirectObservation,
                    )
                },
            )],
        );
        view.tell_profiles.insert(
            actor,
            TellProfile {
                max_tell_candidates: 4,
                max_relay_chain_len: 2,
                ..TellProfile::default()
            },
        );

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            SocialBeliefView::known_entity_beliefs(&state, actor),
            view.beliefs.get(&actor).cloned().unwrap()
        );
        assert_eq!(
            SocialBeliefView::tell_profile(&state, actor),
            view.tell_profiles.get(&actor).copied()
        );
    }

    #[test]
    fn planning_state_exposes_believed_activity_from_snapshot() {
        let (mut view, actor, town, _field, _bread) = test_view();
        let observed = entity(30);
        view.beliefs.insert(
            actor,
            vec![(
                observed,
                belief_with_activity(town, ActionDomain::Production, Some(entity(40)), 9),
            )],
        );

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            SocialBeliefView::believed_activity_of(&state, observed),
            Some(&BelievedActivity {
                action_domain: ActionDomain::Production,
                target: Some(entity(40)),
                observed_tick: Tick(9),
            })
        );
        assert_eq!(SocialBeliefView::believed_activity_of(&state, actor), None);
    }

    #[test]
    fn planning_state_agents_active_at_filters_snapshot_beliefs() {
        let (mut view, actor, town, field, _bread) = test_view();
        let source = entity(40);
        let other_source = entity(41);
        let producer = entity(30);
        let trader = entity(31);
        let other_target = entity(32);
        let remote = entity(33);
        view.beliefs.insert(
            actor,
            vec![
                (
                    producer,
                    belief_with_activity(town, ActionDomain::Production, Some(source), 9),
                ),
                (
                    trader,
                    belief_with_activity(town, ActionDomain::Trade, Some(source), 9),
                ),
                (
                    other_target,
                    belief_with_activity(town, ActionDomain::Production, Some(other_source), 9),
                ),
                (
                    remote,
                    belief_with_activity(field, ActionDomain::Production, Some(source), 9),
                ),
            ],
        );

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            SocialBeliefView::agents_active_at(&state, town, ActionDomain::Production, None),
            vec![producer, other_target]
        );
        assert_eq!(
            SocialBeliefView::agents_active_at(
                &state,
                town,
                ActionDomain::Production,
                Some(source)
            ),
            vec![producer]
        );
        assert_eq!(
            SocialBeliefView::agents_active_at(&state, town, ActionDomain::Trade, Some(source)),
            vec![trader]
        );
        assert!(
            SocialBeliefView::agents_active_at(&state, field, ActionDomain::Trade, Some(source))
                .is_empty()
        );
    }

    #[test]
    fn planning_state_preserves_missing_actor_tell_profile_from_snapshot() {
        let (view, actor, _town, _field, bread) = test_view();
        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::from([bread]), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(SocialBeliefView::tell_profile(&state, actor), None);
    }

    #[test]
    fn planning_state_preserves_actor_conversation_memory_from_snapshot() {
        let (base_view, actor, town, _field, bread) = test_view();
        let listener = entity(99);
        let mut view = StubBeliefView {
            current_tick: Tick(8),
            ..base_view
        };
        view.beliefs.insert(
            actor,
            vec![(
                bread,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(town),
                    last_known_inventory: BTreeMap::from([(CommodityKind::Bread, Quantity(2))]),
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
                        Tick(7),
                        worldwake_core::PerceptionSource::DirectObservation,
                    )
                },
            )],
        );
        view.tell_profiles.insert(actor, TellProfile::default());
        view.told_beliefs.insert(
            actor,
            vec![(
                TellMemoryKey {
                    counterparty: listener,
                    topic: TellTopic::EntityBelief { subject: bread },
                },
                ToldBeliefMemory {
                    shared_state: SharedTellState::EntityBelief(
                        worldwake_core::to_shared_belief_snapshot(&BelievedEntityState {
                            believed_kind: None,
                            last_known_place: Some(town),
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
                                Tick(4),
                                worldwake_core::PerceptionSource::DirectObservation,
                            )
                        }),
                    ),
                    told_tick: Tick(6),
                },
            )],
        );

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(TemporalBeliefView::current_tick(&state), Tick(8));
        assert_eq!(
            SocialBeliefView::told_belief_memory(
                &state,
                actor,
                listener,
                &TellTopic::EntityBelief { subject: bread },
            )
            .map(|m| m.told_tick),
            Some(Tick(6))
        );
        assert_eq!(
            SocialBeliefView::recipient_knowledge_status(
                &state,
                actor,
                listener,
                &TellTopic::EntityBelief { subject: bread },
            ),
            Some(RecipientKnowledgeStatus::SpeakerHasOnlyToldStaleBelief)
        );
    }

    #[test]
    fn planning_state_matches_live_office_data_and_force_claim_affordances() {
        let actor = entity(1);
        let office = entity(100);
        let town = entity(10);

        let mut view = StubBeliefView::default();
        for &entity in &[actor, office, town] {
            view.alive.insert(entity, true);
        }
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(office, EntityKind::Office);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(office, town);
        view.entities_at.insert(town, vec![actor, office]);
        view.carry_capacities.insert(actor, LoadUnits(10));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.beliefs.insert(
            actor,
            vec![(
                office,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(town),
                    last_known_inventory: BTreeMap::new(),
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
                        Tick(0),
                        worldwake_core::PerceptionSource::DirectObservation,
                    )
                },
            )],
        );
        view.office_data.insert(
            office,
            OfficeData {
                title: "Marshal".to_string(),
                seat: town,
                jurisdiction: BTreeSet::from([town]),
                succession_law: SuccessionLaw::Force,
                eligibility_rules: Vec::new(),
                succession_period_ticks: 19,
                vacancy_since: Some(Tick(7)),
            },
        );
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(None));

        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::from([office]), &BTreeSet::new(), 0);
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            PoliticalBeliefView::office_data(&state, office),
            PoliticalBeliefView::office_data(&view, office)
        );

        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let office_ids = register_office_actions(&mut defs, &mut handlers);
        let press_force_claim_def = office_ids[3];

        let live_affordances = get_affordances(&view, actor, &defs, &handlers)
            .into_iter()
            .filter(|affordance| affordance.def_id == press_force_claim_def)
            .map(|affordance| affordance.payload_override)
            .collect::<Vec<_>>();
        let planning_affordances = get_affordances(&state, actor, &defs, &handlers)
            .into_iter()
            .filter(|affordance| affordance.def_id == press_force_claim_def)
            .map(|affordance| affordance.payload_override)
            .collect::<Vec<_>>();

        assert_eq!(live_affordances, planning_affordances);
        assert_eq!(
            live_affordances,
            vec![Some(ActionPayload::PressForceClaim(
                worldwake_sim::PressForceClaimActionPayload { office }
            ))]
        );
    }

    #[test]
    fn overlay_clones_share_snapshot_owned_heavy_vectors() {
        let (view, actor, _town, field, _bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let base = PlanningState::new(&snapshot);
        let moved = base.clone().move_actor_to(field);

        let base_wounds = &base.snapshot().entities.get(&actor).unwrap().combat.wounds;
        let moved_wounds = &moved.snapshot().entities.get(&actor).unwrap().combat.wounds;
        let base_demand = &base
            .snapshot()
            .entities
            .get(&actor)
            .unwrap()
            .economic
            .demand_memory;
        let moved_demand = &moved
            .snapshot()
            .entities
            .get(&actor)
            .unwrap()
            .economic
            .demand_memory;

        assert!(std::ptr::eq(base_wounds.as_ptr(), moved_wounds.as_ptr()));
        assert!(std::ptr::eq(base_demand.as_ptr(), moved_demand.as_ptr()));
    }

    #[test]
    fn cloned_overlay_mutations_do_not_leak_between_branches() {
        let (view, actor, town, field, bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let base = PlanningState::new(&snapshot);
        let range = TickRange::new(Tick(4), Tick(6)).unwrap();

        let branched = base
            .clone()
            .move_actor_to(field)
            .reserve(bread, range)
            .mark_removed(bread);

        assert_eq!(SpatialBeliefView::effective_place(&base, actor), Some(town));
        assert_eq!(
            SpatialBeliefView::effective_place(&branched, actor),
            Some(field)
        );
        assert!(!base.reservation_conflicts(bread, range));
        assert!(branched.reservation_conflicts(bread, range));
        assert!(EntityBeliefView::is_alive(&base, bread));
        assert!(EntityBeliefView::is_dead(&branched, bread));
    }

    #[test]
    fn hostile_queries_respect_hypothetical_location_changes() {
        let actor = entity(1);
        let attacker = entity(2);
        let town = entity(10);
        let field = entity(11);
        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(attacker, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(attacker, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(field, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(attacker, town);
        view.entities_at.insert(town, vec![actor, attacker]);
        view.entities_at.insert(field, vec![]);
        view.adjacent
            .insert(town, vec![(field, NonZeroU32::new(1).unwrap())]);
        view.adjacent
            .insert(field, vec![(town, NonZeroU32::new(1).unwrap())]);
        view.thresholds.insert(actor, DriveThresholds::default());
        view.hostiles.insert(actor, vec![attacker]);
        view.attackers.insert(actor, vec![attacker]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([attacker]),
            &BTreeSet::from([town, field]),
            1,
        );

        let moved = PlanningState::new(&snapshot).move_actor_to(field);

        assert!(CombatBeliefView::visible_hostiles_for(&moved, actor).is_empty());
        assert!(CombatBeliefView::current_attackers_of(&moved, actor).is_empty());
    }

    #[test]
    fn dead_hostiles_are_not_visible_or_actionable_in_snapshot_state() {
        let actor = entity(0);
        let attacker = entity(1);
        let town = entity(2);
        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(attacker, false);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(attacker, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(attacker, town);
        view.entities_at.insert(town, vec![actor, attacker]);
        view.thresholds.insert(actor, DriveThresholds::default());
        view.hostiles.insert(actor, vec![attacker]);
        view.attackers.insert(actor, vec![attacker]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([attacker]),
            &BTreeSet::from([town]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert!(CombatBeliefView::visible_hostiles_for(&state, actor).is_empty());
        assert!(CombatBeliefView::hostile_targets_of(&state, actor).is_empty());
        assert!(CombatBeliefView::current_attackers_of(&state, actor).is_empty());
    }

    #[test]
    fn spawn_hypothetical_lot_allocates_monotonic_ids_and_clones_preserve_branch_counters() {
        let (view, actor, _town, _field, _bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let mut base = PlanningState::new(&snapshot);

        let first = base.spawn_hypothetical_lot(EntityKind::ItemLot, CommodityKind::Water);
        let mut branch = base.clone();
        let second = base.spawn_hypothetical_lot(EntityKind::ItemLot, CommodityKind::Bread);
        let branch_second =
            branch.spawn_hypothetical_lot(EntityKind::ItemLot, CommodityKind::Apple);

        assert_eq!(first, HypotheticalEntityId(0));
        assert_eq!(second, HypotheticalEntityId(1));
        assert_eq!(branch_second, HypotheticalEntityId(1));
    }

    #[test]
    fn authoritative_ref_queries_fall_back_to_snapshot_data() {
        let (view, actor, town, _field, bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            state.effective_place_ref(PlanningEntityRef::Authoritative(actor)),
            Some(town)
        );
        assert_eq!(
            state.item_lot_commodity_ref(PlanningEntityRef::Authoritative(bread)),
            Some(CommodityKind::Bread)
        );
        assert_eq!(
            state.commodity_quantity_ref(
                PlanningEntityRef::Authoritative(actor),
                CommodityKind::Bread
            ),
            Quantity(1)
        );
    }

    #[test]
    fn hypothetical_ref_queries_read_registry_and_overrides_without_snapshot_fallback() {
        let (view, actor, town, _field, bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let mut state = PlanningState::new(&snapshot);
        let hid = state.spawn_hypothetical_lot(EntityKind::ItemLot, CommodityKind::Water);
        let hypothetical = PlanningEntityRef::Hypothetical(hid);
        let actor_ref = PlanningEntityRef::Authoritative(actor);

        let state = state
            .set_possessor_ref(hypothetical, actor_ref)
            .set_quantity_ref(hypothetical, CommodityKind::Water, Quantity(2));

        assert_eq!(
            state.item_lot_commodity_ref(hypothetical),
            Some(CommodityKind::Water)
        );
        assert_eq!(
            state.entity_kind_ref(hypothetical),
            Some(EntityKind::ItemLot)
        );
        assert_eq!(state.direct_possessor_ref(hypothetical), Some(actor_ref));
        assert_eq!(state.effective_place_ref(hypothetical), Some(town));
        assert_eq!(
            state.commodity_quantity_ref(hypothetical, CommodityKind::Water),
            Quantity(2)
        );
        assert_eq!(
            state.item_lot_commodity_ref(PlanningEntityRef::Authoritative(bread)),
            Some(CommodityKind::Bread)
        );
    }

    #[test]
    fn controlled_commodity_quantity_at_place_counts_local_authoritative_and_hypothetical_stock() {
        let (view, actor, town, field, bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let mut state = PlanningState::new(&snapshot);
        let hid = state.spawn_hypothetical_lot(EntityKind::ItemLot, CommodityKind::Bread);
        let hypothetical = PlanningEntityRef::Hypothetical(hid);
        let actor_ref = PlanningEntityRef::Authoritative(actor);

        let local = state
            .set_possessor_ref(hypothetical, actor_ref)
            .set_quantity_ref(hypothetical, CommodityKind::Bread, Quantity(2));
        let moved = local.clone().move_actor_to(field);

        assert_eq!(
            EconomicBeliefView::controlled_commodity_quantity_at_place(
                &local,
                actor,
                town,
                CommodityKind::Bread
            ),
            Quantity(3)
        );
        assert_eq!(
            EconomicBeliefView::controlled_commodity_quantity_at_place(
                &local,
                actor,
                field,
                CommodityKind::Bread
            ),
            Quantity(0)
        );
        assert_eq!(
            EconomicBeliefView::controlled_commodity_quantity_at_place(
                &moved,
                actor,
                town,
                CommodityKind::Bread
            ),
            Quantity(0)
        );
        assert_eq!(
            EconomicBeliefView::controlled_commodity_quantity_at_place(
                &moved,
                actor,
                field,
                CommodityKind::Bread
            ),
            Quantity(3)
        );
        assert_eq!(
            EconomicBeliefView::local_controlled_lots_for(
                &local,
                actor,
                town,
                CommodityKind::Bread
            ),
            vec![bread]
        );
    }

    #[test]
    fn possessed_entities_follow_holder_movement_without_stale_place_overrides() {
        let (view, actor, town, field, bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let mut state = PlanningState::new(&snapshot);
        let cargo_id = state.spawn_hypothetical_lot(EntityKind::ItemLot, CommodityKind::Bread);
        let actor_ref = PlanningEntityRef::Authoritative(actor);
        let cargo_ref = PlanningEntityRef::Hypothetical(cargo_id);

        let state = state
            .set_possessor_ref(cargo_ref, actor_ref)
            .set_quantity_ref(cargo_ref, CommodityKind::Bread, Quantity(2));

        assert_eq!(state.effective_place_ref(cargo_ref), Some(town));

        let moved = state.move_actor_to(field);

        assert_eq!(moved.effective_place_ref(cargo_ref), Some(field));
        assert_eq!(
            EconomicBeliefView::controlled_commodity_quantity_at_place(
                &moved,
                actor,
                field,
                CommodityKind::Bread
            ),
            Quantity(3)
        );
        assert_eq!(
            EconomicBeliefView::local_controlled_lots_for(
                &moved,
                actor,
                field,
                CommodityKind::Bread
            ),
            vec![bread]
        );
    }

    #[test]
    fn entities_at_cache_is_invalidated_when_holder_moves_across_branches() {
        let (view, actor, town, field, bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let mut state = PlanningState::new(&snapshot);
        let cargo_id = state.spawn_hypothetical_lot(EntityKind::ItemLot, CommodityKind::Bread);
        let actor_ref = PlanningEntityRef::Authoritative(actor);
        let cargo_ref = PlanningEntityRef::Hypothetical(cargo_id);

        let base = state
            .set_possessor_ref(cargo_ref, actor_ref)
            .set_quantity_ref(cargo_ref, CommodityKind::Bread, Quantity(2));

        assert_eq!(
            SpatialBeliefView::entities_at(&base, town),
            vec![actor, bread]
        );

        let moved = base.clone().move_actor_to(field);

        assert_eq!(
            SpatialBeliefView::entities_at(&base, town),
            vec![actor, bread]
        );
        assert_eq!(
            SpatialBeliefView::entities_at(&moved, field),
            vec![actor, bread]
        );
        assert_eq!(moved.effective_place_ref(cargo_ref), Some(field));
    }

    #[test]
    fn effective_place_cache_is_invalidated_when_holder_moves_across_branches() {
        let (view, actor, town, field, _bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let mut state = PlanningState::new(&snapshot);
        let cargo_id = state.spawn_hypothetical_lot(EntityKind::ItemLot, CommodityKind::Bread);
        let actor_ref = PlanningEntityRef::Authoritative(actor);
        let cargo_ref = PlanningEntityRef::Hypothetical(cargo_id);

        let base = state
            .set_possessor_ref(cargo_ref, actor_ref)
            .set_quantity_ref(cargo_ref, CommodityKind::Bread, Quantity(2));

        assert_eq!(base.effective_place_ref(cargo_ref), Some(town));

        let moved = base.clone().move_actor_to(field);

        assert_eq!(base.effective_place_ref(cargo_ref), Some(town));
        assert_eq!(moved.effective_place_ref(cargo_ref), Some(field));
    }

    #[test]
    fn local_controlled_lot_refs_for_tracks_hypotheticals_and_removals() {
        let (view, actor, town, _field, bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let mut state = PlanningState::new(&snapshot);
        let first = state.spawn_hypothetical_lot(EntityKind::ItemLot, CommodityKind::Bread);
        let second = state.spawn_hypothetical_lot(EntityKind::ItemLot, CommodityKind::Bread);
        let actor_ref = PlanningEntityRef::Authoritative(actor);
        let first_ref = PlanningEntityRef::Hypothetical(first);
        let second_ref = PlanningEntityRef::Hypothetical(second);

        let state = state
            .set_possessor_ref(first_ref, actor_ref)
            .set_quantity_ref(first_ref, CommodityKind::Bread, Quantity(2))
            .set_possessor_ref(second_ref, actor_ref)
            .set_quantity_ref(second_ref, CommodityKind::Bread, Quantity(4));
        let removed = state.clone().mark_removed_ref(first_ref);

        assert_eq!(
            state.local_controlled_lot_refs_for(actor_ref, town, CommodityKind::Bread),
            vec![
                PlanningEntityRef::Authoritative(bread),
                first_ref,
                second_ref
            ]
        );
        assert_eq!(
            removed.local_controlled_lot_refs_for(actor_ref, town, CommodityKind::Bread),
            vec![PlanningEntityRef::Authoritative(bread), second_ref]
        );
    }

    #[test]
    fn removed_hypothetical_entities_stop_answering_ref_queries_and_do_not_leak_through_belief_view()
     {
        let (view, actor, _town, _field, bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let mut state = PlanningState::new(&snapshot);
        let hid = state.spawn_hypothetical_lot(EntityKind::ItemLot, CommodityKind::Water);
        let hypothetical = PlanningEntityRef::Hypothetical(hid);
        let actor_ref = PlanningEntityRef::Authoritative(actor);

        let state = state
            .set_possessor_ref(hypothetical, actor_ref)
            .set_quantity_ref(hypothetical, CommodityKind::Water, Quantity(2));
        let removed = state.mark_removed_ref(hypothetical);

        assert_eq!(removed.entity_kind_ref(hypothetical), None);
        assert_eq!(removed.item_lot_commodity_ref(hypothetical), None);
        assert_eq!(removed.direct_possessor_ref(hypothetical), None);
        assert_eq!(removed.effective_place_ref(hypothetical), None);
        assert_eq!(
            worldwake_sim::InventoryBeliefView::direct_possessions(&removed, actor),
            vec![bread]
        );
    }

    #[test]
    fn carry_capacity_and_authoritative_load_queries_read_snapshot_data() {
        let (view, actor, _town, _field, bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            state.carry_capacity_ref(PlanningEntityRef::Authoritative(actor)),
            Some(LoadUnits(10))
        );
        assert_eq!(
            state.load_of_entity_ref(PlanningEntityRef::Authoritative(bread)),
            Some(LoadUnits(1))
        );
        assert_eq!(
            worldwake_sim::InventoryBeliefView::carry_capacity(&state, actor),
            Some(LoadUnits(10))
        );
        assert_eq!(
            worldwake_sim::InventoryBeliefView::load_of_entity(&state, bread),
            Some(LoadUnits(1))
        );
    }

    #[test]
    fn authoritative_item_lot_load_is_derived_when_snapshot_intrinsic_load_is_missing() {
        let (mut view, actor, _town, _field, bread) = test_view();
        view.entity_loads.remove(&bread);

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            state.load_of_entity_ref(PlanningEntityRef::Authoritative(bread)),
            Some(LoadUnits(
                worldwake_core::load_per_unit(CommodityKind::Bread).0
            ))
        );
    }

    #[test]
    fn remaining_carry_capacity_counts_nested_and_hypothetical_load() {
        let actor = entity(1);
        let town = entity(10);
        let satchel = entity(20);
        let water = entity(21);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(town, true);
        view.alive.insert(satchel, true);
        view.alive.insert(water, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(satchel, EntityKind::Container);
        view.kinds.insert(water, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(satchel, town);
        view.effective_places.insert(water, town);
        view.entities_at.insert(town, vec![actor, satchel, water]);
        view.direct_possessions.insert(actor, vec![satchel]);
        view.direct_possessors.insert(satchel, actor);
        view.direct_containers.insert(water, satchel);
        view.item_lot_commodities
            .insert(water, CommodityKind::Water);
        view.commodity_quantities
            .insert((water, CommodityKind::Water), Quantity(2));
        view.carry_capacities.insert(actor, LoadUnits(10));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.entity_loads.insert(satchel, LoadUnits(2));
        view.entity_loads.insert(water, LoadUnits(4));

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 0);
        let mut state = PlanningState::new(&snapshot);
        let hypothetical = PlanningEntityRef::Hypothetical(
            state.spawn_hypothetical_lot(EntityKind::ItemLot, CommodityKind::Apple),
        );
        let state = state
            .set_possessor_ref(hypothetical, PlanningEntityRef::Authoritative(actor))
            .set_quantity_ref(hypothetical, CommodityKind::Apple, Quantity(1));

        assert_eq!(state.load_of_entity_ref(hypothetical), Some(LoadUnits(1)));
        assert_eq!(
            state.remaining_carry_capacity_ref(PlanningEntityRef::Authoritative(actor)),
            Some(LoadUnits(3))
        );
    }

    #[test]
    fn remaining_carry_capacity_supports_full_partial_and_zero_fit_checks() {
        let (view, actor, _town, _field, _bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let base = PlanningState::new(&snapshot);
        let actor_ref = PlanningEntityRef::Authoritative(actor);

        assert!(base.load_of_entity_ref(actor_ref).is_some());
        assert!(base.remaining_carry_capacity_ref(actor_ref).is_some());

        let full_fit = base.clone();
        assert!(
            full_fit
                .load_of_entity_ref(PlanningEntityRef::Authoritative(entity(20)))
                .unwrap()
                <= full_fit.remaining_carry_capacity_ref(actor_ref).unwrap()
        );

        let mut partial_base = base.clone();
        let ballast =
            partial_base.spawn_hypothetical_lot(EntityKind::ItemLot, CommodityKind::Apple);
        let hid = partial_base.spawn_hypothetical_lot(EntityKind::ItemLot, CommodityKind::Water);
        let partial = partial_base
            .set_possessor_ref(PlanningEntityRef::Hypothetical(ballast), actor_ref)
            .set_quantity_ref(
                PlanningEntityRef::Hypothetical(ballast),
                CommodityKind::Apple,
                Quantity(7),
            )
            .set_quantity_ref(
                PlanningEntityRef::Hypothetical(hid),
                CommodityKind::Water,
                Quantity(2),
            );
        let remaining = partial.remaining_carry_capacity_ref(actor_ref).unwrap();
        let water_load = partial
            .load_of_entity_ref(PlanningEntityRef::Hypothetical(hid))
            .unwrap();
        let per_unit = LoadUnits(worldwake_core::load_per_unit(CommodityKind::Water).0);
        assert!(water_load > remaining);
        assert!(per_unit <= remaining);

        let mut zero_base = base.clone();
        let zero_ballast =
            zero_base.spawn_hypothetical_lot(EntityKind::ItemLot, CommodityKind::Apple);
        let zero_hid =
            zero_base.spawn_hypothetical_lot(EntityKind::ItemLot, CommodityKind::Firewood);
        let zero = zero_base
            .set_possessor_ref(PlanningEntityRef::Hypothetical(zero_ballast), actor_ref)
            .set_quantity_ref(
                PlanningEntityRef::Hypothetical(zero_ballast),
                CommodityKind::Apple,
                Quantity(7),
            )
            .set_quantity_ref(
                PlanningEntityRef::Hypothetical(zero_hid),
                CommodityKind::Firewood,
                Quantity(1),
            );
        let zero_remaining = zero.remaining_carry_capacity_ref(actor_ref).unwrap();
        let firewood_unit = LoadUnits(worldwake_core::load_per_unit(CommodityKind::Firewood).0);
        assert!(firewood_unit > zero_remaining);
    }

    #[test]
    fn courage_round_trips_through_snapshot_and_planning_state() {
        let (mut view, actor, _town, _field, bread) = test_view();
        let courage_value = Permille::new(500).unwrap();
        view.courages.insert(actor, courage_value);

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        // Agent with courage returns Some
        assert_eq!(
            CombatBeliefView::courage(&state, actor),
            Some(courage_value)
        );

        // Entity in snapshot without UtilityProfile (bread is an ItemLot) returns None
        assert_eq!(CombatBeliefView::courage(&state, bread), None);

        // Entity not in snapshot returns None
        let unknown = entity(999);
        assert_eq!(CombatBeliefView::courage(&state, unknown), None);
    }

    // ── hypothetical_support_count / has_support_majority ──────────────

    fn support_test_setup() -> (
        StubBeliefView,
        EntityId,
        EntityId,
        EntityId,
        EntityId,
        EntityId,
    ) {
        let actor = entity(1);
        let rival = entity(2);
        let supporter_a = entity(3);
        let supporter_b = entity(4);
        let office = entity(100);
        let town = entity(10);

        let mut view = StubBeliefView::default();
        for &e in &[actor, rival, supporter_a, supporter_b, office, town] {
            view.alive.insert(e, true);
        }
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(rival, EntityKind::Agent);
        view.kinds.insert(supporter_a, EntityKind::Agent);
        view.kinds.insert(supporter_b, EntityKind::Agent);
        view.kinds.insert(office, EntityKind::Office);
        view.kinds.insert(town, EntityKind::Place);

        view.effective_places.insert(actor, town);
        view.effective_places.insert(rival, town);
        view.effective_places.insert(supporter_a, town);
        view.effective_places.insert(supporter_b, town);
        view.effective_places.insert(office, town);

        view.entities_at
            .insert(town, vec![actor, rival, supporter_a, supporter_b, office]);
        view.carry_capacities.insert(actor, LoadUnits(10));
        view.entity_loads.insert(actor, LoadUnits(0));

        (view, actor, rival, supporter_a, supporter_b, office)
    }

    fn build_support_snapshot(
        view: &StubBeliefView,
        actor: EntityId,
        office: EntityId,
    ) -> crate::planning_snapshot::PlanningSnapshot {
        let mut evidence = BTreeSet::new();
        evidence.insert(office);
        build_planning_snapshot(view, actor, &evidence, &BTreeSet::new(), 1)
    }

    #[test]
    fn believed_office_holder_uses_snapshot_then_override_then_unknown() {
        let (mut view, actor, rival, _supporter_a, _supporter_b, office) = support_test_setup();
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(Some(actor)));

        let snapshot = build_support_snapshot(&view, actor, office);
        let mut state = PlanningState::new(&snapshot);

        assert_eq!(
            state.believed_office_holder(office),
            InstitutionalBeliefRead::Certain(Some(actor))
        );

        state.override_office_holder_belief(office, InstitutionalBeliefRead::Certain(Some(rival)));

        assert_eq!(
            state.believed_office_holder(office),
            InstitutionalBeliefRead::Certain(Some(rival))
        );
        assert_eq!(
            state.believed_office_holder(entity(999)),
            InstitutionalBeliefRead::Unknown
        );
    }

    #[test]
    fn believed_support_declaration_uses_snapshot_then_override_then_unknown() {
        let (mut view, actor, rival, supporter_a, _supporter_b, office) = support_test_setup();
        view.support_declaration_beliefs.insert(
            (office, supporter_a),
            InstitutionalBeliefRead::Certain(Some(actor)),
        );

        let snapshot = build_support_snapshot(&view, actor, office);
        let mut state = PlanningState::new(&snapshot);

        assert_eq!(
            state.believed_support_declaration(office, supporter_a),
            InstitutionalBeliefRead::Certain(Some(actor))
        );

        state.override_support_declaration_belief(
            office,
            supporter_a,
            InstitutionalBeliefRead::Certain(Some(rival)),
        );

        assert_eq!(
            state.believed_support_declaration(office, supporter_a),
            InstitutionalBeliefRead::Certain(Some(rival))
        );
        assert_eq!(
            state.believed_support_declaration(office, entity(999)),
            InstitutionalBeliefRead::Unknown
        );
    }

    #[test]
    fn institutional_belief_overrides_do_not_touch_world_state_support_overrides() {
        let (mut view, actor, rival, supporter_a, _supporter_b, office) = support_test_setup();
        view.support_declaration_beliefs.insert(
            (office, supporter_a),
            InstitutionalBeliefRead::Certain(Some(actor)),
        );

        let snapshot = build_support_snapshot(&view, actor, office);
        let mut state =
            PlanningState::new(&snapshot).with_support_declaration(supporter_a, office, rival);
        state.override_support_declaration_belief(
            office,
            supporter_a,
            InstitutionalBeliefRead::Conflicted(vec![Some(actor), Some(rival)]),
        );

        assert_eq!(
            state.effective_support_declaration(supporter_a, office),
            Some(rival)
        );
        assert_eq!(
            state.test_support_override(supporter_a, office),
            Some(rival)
        );
        assert_eq!(
            state.test_support_belief_override(office, supporter_a),
            Some(InstitutionalBeliefRead::Conflicted(vec![
                Some(actor),
                Some(rival),
            ]))
        );
    }

    #[test]
    fn planning_state_uses_snapshot_record_data_and_consultation_speed_factor() {
        let actor = entity(1);
        let record = entity(40);
        let town = entity(10);
        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(record, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(record, EntityKind::Record);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(record, town);
        view.entities_at.insert(town, vec![actor, record]);
        view.carry_capacities.insert(actor, LoadUnits(10));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.consultation_speed_factors
            .insert(actor, Permille::new(500).unwrap());
        view.disposal_profiles.insert(
            actor,
            DisposalProfile {
                capacity_strain_threshold: Permille::new(750).unwrap(),
            },
        );
        view.record_data.insert(
            record,
            RecordData {
                record_kind: RecordKind::OfficeRegister,
                home_place: town,
                issuer: actor,
                consultation_ticks: 4,
                max_entries_per_consult: 2,
                entries: Vec::new(),
                next_entry_id: 0,
            },
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([record]),
            &BTreeSet::from([town]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            state.record_data(record),
            view.record_data.get(&record).cloned()
        );
        assert_eq!(
            CombatBeliefView::consultation_speed_factor(&state, actor),
            Some(Permille::new(500).unwrap())
        );
    }

    #[test]
    fn planning_state_matches_runtime_duration_estimation_for_dynamic_duration_contract() {
        let actor = entity(1);
        let town = entity(10);
        let market = entity(11);
        let record = entity(40);
        let bread = entity(41);
        let patient = entity(42);
        let hostile = entity(43);
        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(record, true);
        view.alive.insert(bread, true);
        view.alive.insert(patient, true);
        view.alive.insert(hostile, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(market, EntityKind::Place);
        view.kinds.insert(record, EntityKind::Record);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.kinds.insert(patient, EntityKind::Agent);
        view.kinds.insert(hostile, EntityKind::Agent);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(record, town);
        view.effective_places.insert(bread, town);
        view.effective_places.insert(patient, town);
        view.effective_places.insert(hostile, town);
        view.entities_at
            .insert(town, vec![actor, record, bread, patient, hostile]);
        view.entities_at.insert(market, Vec::new());
        view.adjacent
            .insert(town, vec![(market, NonZeroU32::new(3).unwrap())]);
        view.adjacent
            .insert(market, vec![(town, NonZeroU32::new(3).unwrap())]);
        view.item_lot_commodities
            .insert(bread, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread,
            CommodityKind::Bread.spec().consumable_profile.unwrap(),
        );
        view.commodity_quantities
            .insert((actor, CommodityKind::Medicine), Quantity(2));
        view.carry_capacities.insert(actor, LoadUnits(10));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.metabolism_profiles.insert(
            actor,
            MetabolismProfile::new(
                pm(10),
                pm(10),
                pm(10),
                pm(10),
                pm(10),
                pm(1000),
                NonZeroU32::new(20).unwrap(),
                NonZeroU32::new(20).unwrap(),
                NonZeroU32::new(20).unwrap(),
                NonZeroU32::new(20).unwrap(),
                NonZeroU32::new(8).unwrap(),
                NonZeroU32::new(9).unwrap(),
                NonZeroU32::new(8).unwrap(),
                pm(0),
                pm(0),
                pm(0),
                pm(0),
            ),
        );
        view.artifact_posting_profiles.insert(
            actor,
            ArtifactPostingProfile {
                threat_warning_ttl: 36,
                office_vacancy_ttl: 72,
                bounty_ttl: 108,
            },
        );
        view.trade_profiles.insert(
            actor,
            TradeDispositionProfile {
                negotiation_round_ticks: NonZeroU32::new(4).unwrap(),
                initial_offer_bias: pm(120),
                concession_rate: pm(80),
                rejection_escalation_rate: pm(200),
                demand_memory_retention_ticks: 12,
                market_presence_ticks: NonZeroU32::new(30).unwrap(),
            },
        );
        view.patrol_profiles.insert(
            actor,
            PatrolProfile {
                base_dwell_ticks: 8,
                dwell_vigilance_scale_ticks: 8,
                vigilance: pm(625),
                route_adaptation_sensitivity: pm(400),
                patrol_motive_weight: pm(550),
            },
        );
        view.patrol_routes.insert(
            actor,
            PatrolRoute {
                assigned_places: vec![town, market],
                current_index: 1,
            },
        );
        view.epistemic_profiles.insert(
            actor,
            EpistemicDispositionProfile {
                stale_evidence_barrier_threshold: pm(400),
                witness_query_duration_ticks: NonZeroU32::new(3).unwrap(),
                ask_memory_retention_ticks: 10,
            },
        );
        view.theft_profiles.insert(
            actor,
            TheftDispositionProfile {
                steal_duration_ticks: NonZeroU32::new(5).unwrap(),
                theft_motive_weight: pm(400),
                witness_risk_penalty: pm(100),
            },
        );
        view.justice_profiles.insert(
            actor,
            JusticeDispositionProfile {
                accusation_motive_weight: pm(650),
                fine_severity: pm(500),
            },
        );
        view.violation_profiles.insert(
            actor,
            ViolationDispositionProfile {
                investigation_duration_ticks: NonZeroU32::new(6).unwrap(),
                violation_memory_retention_ticks: 12,
                investigation_motive_weight: pm(500),
                ownership_motive_bonus: pm(150),
            },
        );
        view.combat_profiles.insert(
            actor,
            CombatProfile::new(
                pm(1000),
                pm(700),
                pm(620),
                pm(580),
                pm(80),
                pm(25),
                pm(18),
                pm(120),
                pm(35),
                NonZeroU32::new(7).unwrap(),
                NonZeroU32::new(11).unwrap(),
            ),
        );
        view.wounds.insert(
            patient,
            vec![Wound {
                id: WoundId(1),
                body_part: worldwake_core::BodyPart::Torso,
                cause: WoundCause::Deprivation(worldwake_core::DeprivationKind::Starvation),
                severity: pm(200),
                inflicted_at: Tick(1),
                bleed_rate_per_tick: pm(0),
            }],
        );
        view.consultation_speed_factors
            .insert(actor, Permille::new(500).unwrap());
        view.record_data.insert(
            record,
            RecordData {
                record_kind: RecordKind::OfficeRegister,
                home_place: town,
                issuer: actor,
                consultation_ticks: 4,
                max_entries_per_consult: 2,
                entries: Vec::new(),
                next_entry_id: 0,
            },
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([record, market]),
            &BTreeSet::from([town, market]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            EconomicBeliefView::trade_disposition_profile(&state, actor),
            view.trade_profiles.get(&actor).cloned()
        );
        assert_eq!(
            CombatBeliefView::patrol_profile(&state, actor),
            view.patrol_profiles.get(&actor).cloned()
        );
        assert_eq!(
            SpatialBeliefView::patrol_route(&state, actor),
            view.patrol_routes.get(&actor).cloned()
        );
        assert_eq!(
            SocialBeliefView::epistemic_disposition_profile(&state, actor),
            view.epistemic_profiles.get(&actor).cloned()
        );
        assert_eq!(
            SocialBeliefView::theft_disposition_profile(&state, actor),
            view.theft_profiles.get(&actor).cloned()
        );
        assert_eq!(
            PoliticalBeliefView::justice_disposition_profile(&state, actor),
            view.justice_profiles.get(&actor).cloned()
        );
        assert_eq!(
            PoliticalBeliefView::violation_disposition_profile(&state, actor),
            view.violation_profiles.get(&actor).cloned()
        );
        assert_eq!(
            CombatBeliefView::combat_profile(&state, actor),
            view.combat_profiles.get(&actor).copied()
        );
        assert_eq!(
            ProfileBeliefView::metabolism_profile(&state, actor),
            view.metabolism_profiles.get(&actor).copied()
        );
        assert_eq!(
            ProfileBeliefView::disposal_profile(&state, actor),
            view.disposal_profiles.get(&actor).copied()
        );
        assert_eq!(
            ProfileBeliefView::artifact_posting_profile(&state, actor),
            view.artifact_posting_profiles.get(&actor).cloned()
        );

        for dependency in PlannerDurationDependency::all() {
            let (duration, targets, payload) = match dependency {
                PlannerDurationDependency::TargetConsumable => (
                    DurationExpr::TargetConsumable { target_index: 0 },
                    vec![bread],
                    ActionPayload::None,
                ),
                PlannerDurationDependency::ActorMetabolism => (
                    DurationExpr::ActorMetabolism {
                        kind: worldwake_sim::MetabolismDurationKind::Wash,
                    },
                    Vec::new(),
                    ActionPayload::None,
                ),
                PlannerDurationDependency::BanditCampEstablishmentProfile => (
                    DurationExpr::BanditCampEstablishmentProfile,
                    Vec::new(),
                    ActionPayload::EstablishCamp(worldwake_sim::EstablishCampActionPayload {
                        faction: actor,
                    }),
                ),
                PlannerDurationDependency::ActorTradeDisposition => (
                    DurationExpr::ActorTradeDisposition,
                    Vec::new(),
                    ActionPayload::None,
                ),
                PlannerDurationDependency::ActorMarketPresence => (
                    DurationExpr::ActorMarketPresence,
                    Vec::new(),
                    ActionPayload::None,
                ),
                PlannerDurationDependency::ActorPatrolProfile => (
                    DurationExpr::ActorPatrolProfile,
                    Vec::new(),
                    ActionPayload::None,
                ),
                PlannerDurationDependency::ActorTheftDisposition => (
                    DurationExpr::ActorTheftDisposition,
                    Vec::new(),
                    ActionPayload::None,
                ),
                PlannerDurationDependency::ActorInvestigationDisposition => (
                    DurationExpr::ActorInvestigationDisposition,
                    Vec::new(),
                    ActionPayload::None,
                ),
                PlannerDurationDependency::ActorWitnessQueryDisposition => (
                    DurationExpr::ActorWitnessQueryDisposition,
                    Vec::new(),
                    ActionPayload::None,
                ),
                PlannerDurationDependency::ActorDefendStance => (
                    DurationExpr::ActorDefendStance,
                    Vec::new(),
                    ActionPayload::None,
                ),
                PlannerDurationDependency::CombatWeapon => (
                    DurationExpr::CombatWeapon,
                    vec![hostile],
                    ActionPayload::Combat(CombatActionPayload {
                        target: hostile,
                        weapon: worldwake_core::CombatWeaponRef::Unarmed,
                    }),
                ),
                PlannerDurationDependency::TargetTreatment => (
                    DurationExpr::TargetTreatment {
                        target_index: 0,
                        commodity: CommodityKind::Medicine,
                    },
                    vec![patient],
                    ActionPayload::None,
                ),
                PlannerDurationDependency::ConsultRecord => (
                    DurationExpr::ConsultRecord { target_index: 0 },
                    vec![record],
                    ActionPayload::None,
                ),
                PlannerDurationDependency::TravelToTarget => (
                    DurationExpr::TravelToTarget { target_index: 0 },
                    vec![market],
                    ActionPayload::None,
                ),
                PlannerDurationDependency::Variable => (
                    DurationExpr::Variable {
                        min: std::num::NonZeroU32::new(1).unwrap(),
                        max: std::num::NonZeroU32::new(64).unwrap(),
                    },
                    Vec::new(),
                    ActionPayload::None,
                ),
            };
            let runtime_duration =
                estimate_duration_from_beliefs(&view, actor, &duration, &targets, &payload);
            let snapshot_duration =
                estimate_duration_from_beliefs(&state, actor, &duration, &targets, &payload);
            assert_eq!(
                snapshot_duration,
                runtime_duration,
                "snapshot parity diverged for {}",
                dependency.label()
            );
        }
    }

    #[test]
    fn planning_state_artifact_posting_profile_is_none_when_absent() {
        let actor = entity(1);
        let town = entity(10);
        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.entities_at.insert(town, vec![actor]);

        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::from([town]), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            ProfileBeliefView::artifact_posting_profile(&state, actor),
            None
        );
        assert_eq!(
            GoalBeliefView::artifact_posting_profile(&state, actor),
            None
        );
    }

    #[test]
    fn planning_state_artifact_posting_profile_round_trips_through_snapshot() {
        let actor = entity(1);
        let town = entity(10);
        let profile = ArtifactPostingProfile {
            threat_warning_ttl: 36,
            office_vacancy_ttl: 72,
            bounty_ttl: 108,
        };
        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.entities_at.insert(town, vec![actor]);
        view.artifact_posting_profiles
            .insert(actor, profile.clone());

        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::from([town]), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            ProfileBeliefView::artifact_posting_profile(&state, actor),
            Some(profile.clone())
        );
        assert_eq!(
            GoalBeliefView::artifact_posting_profile(&state, actor),
            Some(profile)
        );
    }

    #[test]
    fn hypothetical_support_count_base_only() {
        let (mut view, actor, _rival, supporter_a, supporter_b, office) = support_test_setup();
        view.support_declaration_beliefs.insert(
            (office, supporter_a),
            InstitutionalBeliefRead::Certain(Some(actor)),
        );
        view.support_declaration_beliefs.insert(
            (office, supporter_b),
            InstitutionalBeliefRead::Certain(Some(actor)),
        );

        let snapshot = build_support_snapshot(&view, actor, office);
        let state = PlanningState::new(&snapshot);

        assert_eq!(state.hypothetical_support_count(office, actor), 2);
    }

    #[test]
    fn hypothetical_support_count_with_override_changing_existing() {
        let (mut view, actor, rival, supporter_a, supporter_b, office) = support_test_setup();
        view.support_declaration_beliefs.insert(
            (office, supporter_a),
            InstitutionalBeliefRead::Certain(Some(actor)),
        );
        view.support_declaration_beliefs.insert(
            (office, supporter_b),
            InstitutionalBeliefRead::Certain(Some(actor)),
        );

        let snapshot = build_support_snapshot(&view, actor, office);
        // Override: supporter_b now supports rival
        let state =
            PlanningState::new(&snapshot).with_support_declaration(supporter_b, office, rival);

        assert_eq!(state.hypothetical_support_count(office, actor), 1);
        assert_eq!(state.hypothetical_support_count(office, rival), 1);
    }

    #[test]
    fn hypothetical_support_count_with_purely_hypothetical_new_declaration() {
        let (mut view, actor, _rival, supporter_a, supporter_b, office) = support_test_setup();
        view.support_declaration_beliefs.insert(
            (office, supporter_a),
            InstitutionalBeliefRead::Certain(Some(actor)),
        );

        let snapshot = build_support_snapshot(&view, actor, office);
        // Hypothetical: supporter_b (not in base) now also supports actor
        let state =
            PlanningState::new(&snapshot).with_support_declaration(supporter_b, office, actor);

        assert_eq!(state.hypothetical_support_count(office, actor), 2);
    }

    #[test]
    fn has_support_majority_true_when_strictly_more() {
        let (mut view, actor, rival, supporter_a, supporter_b, office) = support_test_setup();
        view.support_declaration_beliefs.insert(
            (office, supporter_a),
            InstitutionalBeliefRead::Certain(Some(actor)),
        );
        view.support_declaration_beliefs.insert(
            (office, supporter_b),
            InstitutionalBeliefRead::Certain(Some(actor)),
        );
        view.support_declaration_beliefs.insert(
            (office, rival),
            InstitutionalBeliefRead::Certain(Some(rival)),
        );

        let snapshot = build_support_snapshot(&view, actor, office);
        let state = PlanningState::new(&snapshot);

        assert!(state.has_support_majority(office, actor));
        assert!(!state.has_support_majority(office, rival));
    }

    #[test]
    fn has_support_majority_false_on_tie() {
        let (mut view, actor, rival, supporter_a, supporter_b, office) = support_test_setup();
        view.support_declaration_beliefs.insert(
            (office, supporter_a),
            InstitutionalBeliefRead::Certain(Some(actor)),
        );
        view.support_declaration_beliefs.insert(
            (office, supporter_b),
            InstitutionalBeliefRead::Certain(Some(rival)),
        );

        let snapshot = build_support_snapshot(&view, actor, office);
        let state = PlanningState::new(&snapshot);

        assert!(!state.has_support_majority(office, actor));
        assert!(!state.has_support_majority(office, rival));
    }

    #[test]
    fn has_support_majority_false_when_zero_support() {
        let (view, actor, _rival, _supporter_a, _supporter_b, office) = support_test_setup();
        // No declarations at all
        let snapshot = build_support_snapshot(&view, actor, office);
        let state = PlanningState::new(&snapshot);

        assert!(!state.has_support_majority(office, actor));
    }

    #[test]
    fn has_support_majority_true_sole_candidate_with_one_support() {
        let (mut view, actor, _rival, supporter_a, _supporter_b, office) = support_test_setup();
        view.support_declaration_beliefs.insert(
            (office, supporter_a),
            InstitutionalBeliefRead::Certain(Some(actor)),
        );

        let snapshot = build_support_snapshot(&view, actor, office);
        let state = PlanningState::new(&snapshot);

        assert!(state.has_support_majority(office, actor));
    }

    fn sample_claim(
        claim_id: u64,
        subject: EntityId,
        aspect: EntityBeliefAspect,
        value: ClaimValue,
        acquired_tick: u64,
        confidence: u16,
    ) -> EntityBeliefClaim {
        EntityBeliefClaim {
            claim_id: ClaimId(claim_id),
            subject,
            aspect,
            value,
            source: PerceptionSource::DirectObservation,
            acquired_tick: Tick(acquired_tick),
            claimed_event_tick: Some(Tick(acquired_tick)),
            confidence: Permille::new(confidence).unwrap(),
            refuted_at_tick: None,
        }
    }

    #[test]
    fn planning_state_projects_actor_belief_store_location_claims() {
        let actor = entity(1);
        let target = entity(2);
        let place_a = entity(10);
        let place_b = entity(11);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(target, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(target, EntityKind::Agent);
        view.kinds.insert(place_a, EntityKind::Place);
        view.kinds.insert(place_b, EntityKind::Place);
        view.effective_places.insert(actor, place_a);
        view.entities_at.insert(place_a, vec![actor]);
        view.carry_capacities.insert(actor, LoadUnits(10));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.claim_confidence_thresholds
            .insert(actor, Permille::new(300).unwrap());

        let mut belief_store = AgentBeliefStore::new();
        let mut state = belief_with_activity(place_a, ActionDomain::Needs, None, 9);
        state.believed_kind = Some(EntityKind::Agent);
        belief_store.update_entity(target, state);
        belief_store.record_entity_claim(sample_claim(
            1,
            target,
            EntityBeliefAspect::Location,
            ClaimValue::Place(Some(place_a)),
            7,
            950,
        ));
        belief_store.record_entity_claim(EntityBeliefClaim {
            claim_id: ClaimId(2),
            subject: target,
            aspect: EntityBeliefAspect::Location,
            value: ClaimValue::Place(Some(place_b)),
            source: PerceptionSource::Report {
                from: entity(77),
                chain_len: 1,
            },
            acquired_tick: Tick(9),
            claimed_event_tick: Some(Tick(9)),
            confidence: Permille::new(980).unwrap(),
            refuted_at_tick: None,
        });
        view.belief_stores.insert(actor, belief_store);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([target]),
            &BTreeSet::from([place_a, place_b]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let location = EntityBeliefView::believed_target_location(&state, actor, target);

        assert_eq!(location.value, Some(place_b));
        assert_eq!(
            location.status,
            worldwake_sim::belief_view::BeliefStatus::Disputed
        );
        assert_eq!(location.claimed_event_tick, Some(Tick(9)));
        assert_eq!(
            SocialBeliefView::claim_confidence_threshold(&state, actor),
            Permille::new(300).unwrap()
        );
        assert_eq!(
            SocialBeliefView::belief_confidence_policy(&state, actor),
            BeliefConfidencePolicy::default()
        );
    }
}
