use crate::planning_snapshot::PlanningSnapshot;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{
    load_per_unit, ActionDefId, BelievedEntityState, CombatProfile, CommodityKind,
    DemandObservation, DriveThresholds, EntityId, EntityKind, GrantedFacilityUse, HomeostaticNeeds,
    InTransitOnEdge, LoadUnits, MetabolismProfile, Permille, PlaceTag, Quantity, RecipeId,
    RecipientKnowledgeStatus, ResourceSource, TellMemoryKey, TellProfile, TickRange,
    ToldBeliefMemory, TradeDispositionProfile, UniqueItemKind, WorkstationTag, Wound,
};
use worldwake_sim::{
    estimate_duration_from_beliefs, ActionDuration, ActionPayload, DurationExpr, RuntimeBeliefView,
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
    entity_place_overrides: BTreeMap<PlanningEntityRef, Option<EntityId>>,
    direct_container_overrides: BTreeMap<PlanningEntityRef, Option<PlanningEntityRef>>,
    direct_possessor_overrides: BTreeMap<PlanningEntityRef, Option<PlanningEntityRef>>,
    resource_quantity_overrides: BTreeMap<EntityId, Quantity>,
    commodity_quantity_overrides: BTreeMap<(PlanningEntityRef, CommodityKind), Quantity>,
    reservation_shadows: BTreeMap<EntityId, Vec<TickRange>>,
    removed_entities: BTreeSet<PlanningEntityRef>,
    needs_overrides: BTreeMap<EntityId, HomeostaticNeeds>,
    pain_overrides: BTreeMap<EntityId, Permille>,
    support_declaration_overrides: BTreeMap<(EntityId, EntityId), Option<EntityId>>,
    facility_queue_membership_overrides: BTreeMap<EntityId, Option<HypotheticalQueueJoin>>,
    facility_grant_overrides: BTreeMap<EntityId, Option<GrantedFacilityUse>>,
    hypothetical_registry: BTreeMap<HypotheticalEntityId, HypotheticalEntityMeta>,
    next_hypothetical_id: u32,
}

impl<'snapshot> PlanningState<'snapshot> {
    #[must_use]
    pub fn new(snapshot: &'snapshot PlanningSnapshot) -> Self {
        Self {
            snapshot,
            entity_place_overrides: BTreeMap::new(),
            direct_container_overrides: BTreeMap::new(),
            direct_possessor_overrides: BTreeMap::new(),
            resource_quantity_overrides: BTreeMap::new(),
            commodity_quantity_overrides: BTreeMap::new(),
            reservation_shadows: BTreeMap::new(),
            removed_entities: BTreeSet::new(),
            needs_overrides: BTreeMap::new(),
            pain_overrides: BTreeMap::new(),
            support_declaration_overrides: BTreeMap::new(),
            facility_queue_membership_overrides: BTreeMap::new(),
            facility_grant_overrides: BTreeMap::new(),
            hypothetical_registry: BTreeMap::new(),
            next_hypothetical_id: 0,
        }
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
        for (&(supporter, decl_office), override_val) in &self.support_declaration_overrides {
            if decl_office == office {
                if let Some(decl_candidate) = override_val {
                    if *decl_candidate == candidate
                        && !base_declarations.iter().any(|(s, _)| *s == supporter)
                    {
                        count += 1;
                    }
                }
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
        for (&(_, decl_office), override_val) in &self.support_declaration_overrides {
            if decl_office == office {
                if let Some(c) = override_val {
                    all_candidates.insert(*c);
                }
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
                .and_then(|snapshot| snapshot.kind),
            PlanningEntityRef::Hypothetical(entity) => self
                .hypothetical_registry
                .get(&entity)
                .map(|meta| meta.kind),
        }
    }

    #[must_use]
    pub fn effective_place_ref(&self, entity: PlanningEntityRef) -> Option<EntityId> {
        self.resolve_effective_place_ref(entity, &mut BTreeSet::new())
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
                PlanningEntityRef::Authoritative(holder) => self
                    .snapshot
                    .entities
                    .get(&holder)
                    .and_then(|snapshot| snapshot.commodity_quantities.get(&kind).copied()),
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
                    .and_then(|snapshot| snapshot.direct_container)
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
                    .and_then(|snapshot| snapshot.direct_possessor)
                    .map(PlanningEntityRef::Authoritative),
                PlanningEntityRef::Hypothetical(_) => None,
            },
        }
    }

    #[must_use]
    pub fn move_entity_ref(mut self, entity: PlanningEntityRef, destination: EntityId) -> Self {
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
        self.removed_entities.insert(entity);
        self.entity_place_overrides.insert(entity, None);
        self.direct_container_overrides.insert(entity, None);
        self.direct_possessor_overrides.insert(entity, None);
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
                .and_then(|snapshot| snapshot.item_lot_commodity),
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
                .and_then(|snapshot| snapshot.carry_capacity),
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
                .map(|snapshot| snapshot.intrinsic_load),
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

        match commodity {
            CommodityKind::Bread | CommodityKind::Apple | CommodityKind::Grain => {
                needs.hunger = thresholds
                    .hunger
                    .low()
                    .saturating_sub(Permille::new(1).unwrap());
            }
            CommodityKind::Water => {
                needs.thirst = thresholds
                    .thirst
                    .low()
                    .saturating_sub(Permille::new(1).unwrap());
            }
            _ => {}
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
                let total = snapshot.wounds.iter().fold(0u16, |acc, wound| {
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
            Some(GrantedFacilityUse {
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
                .and_then(|snapshot| snapshot.facility_queue.as_ref())
                .and_then(|queue| queue.actor_queue_position),
        }
    }

    fn actor_facility_grant(&self, facility: EntityId) -> Option<&GrantedFacilityUse> {
        match self.facility_grant_overrides.get(&facility) {
            Some(grant) => grant.as_ref(),
            None => self
                .snapshot
                .entities
                .get(&facility)
                .and_then(|snapshot| snapshot.facility_queue.as_ref())
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
        if let Some(override_place) = self.entity_place_overrides.get(&entity) {
            return *override_place;
        }
        if let Some(possessor) = self.direct_possessor_ref(entity) {
            return self.resolve_effective_place_ref(possessor, visited);
        }
        if let Some(container) = self.direct_container_ref(entity) {
            return self.resolve_effective_place_ref(container, visited);
        }
        match entity {
            PlanningEntityRef::Authoritative(entity) => self
                .snapshot
                .entities
                .get(&entity)
                .and_then(|snapshot| snapshot.effective_place),
            PlanningEntityRef::Hypothetical(_) => None,
        }
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
        self.all_entity_refs()
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

    fn can_control_ref(&self, actor: PlanningEntityRef, entity: PlanningEntityRef) -> bool {
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
                .is_some_and(|snapshot| snapshot.action_flags.controllable_by_actor),
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
}

impl RuntimeBeliefView for PlanningState<'_> {
    fn current_tick(&self) -> worldwake_core::Tick {
        self.snapshot.current_tick
    }

    fn is_alive(&self, entity: EntityId) -> bool {
        !self
            .removed_entities
            .contains(&PlanningEntityRef::Authoritative(entity))
            && self
                .snapshot
                .entities
                .get(&entity)
                .is_some_and(|snapshot| snapshot.lifecycle.alive)
    }

    fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
        self.entity_kind_ref(PlanningEntityRef::Authoritative(entity))
    }

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
            return self
                .snapshot
                .places
                .get(&place)
                .map(|p| p.entities.iter().copied().collect())
                .unwrap_or_default();
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
        entities
    }

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

    fn adjacent_places(&self, place: EntityId) -> Vec<EntityId> {
        self.adjacent_places_with_travel_ticks(place)
            .into_iter()
            .map(|(adjacent, _)| adjacent)
            .collect()
    }

    fn knows_recipe(&self, actor: EntityId, recipe: RecipeId) -> bool {
        self.known_recipes(actor).contains(&recipe)
    }

    fn unique_item_count(&self, holder: EntityId, kind: UniqueItemKind) -> u32 {
        self.snapshot
            .entities
            .get(&holder)
            .and_then(|snapshot| snapshot.unique_item_counts.get(&kind).copied())
            .unwrap_or(0)
    }

    fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity {
        self.commodity_quantity_ref(PlanningEntityRef::Authoritative(holder), kind)
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
            .and_then(|snapshot| snapshot.item_lot_consumable_profile)
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

    fn believed_owner_of(&self, entity: EntityId) -> Option<EntityId> {
        self.snapshot
            .entities
            .get(&entity)
            .and_then(|snapshot| snapshot.owner)
    }

    fn workstation_tag(&self, entity: EntityId) -> Option<WorkstationTag> {
        self.snapshot
            .entities
            .get(&entity)
            .and_then(|snapshot| snapshot.workstation_tag)
    }

    fn facility_queue_position(&self, facility: EntityId, actor: EntityId) -> Option<u32> {
        (actor == self.snapshot.actor()).then(|| self.actor_facility_queue_position(facility))?
    }

    fn facility_grant(&self, facility: EntityId) -> Option<&worldwake_core::GrantedFacilityUse> {
        self.actor_facility_grant(facility)
    }

    fn place_has_tag(&self, place: EntityId, tag: PlaceTag) -> bool {
        self.snapshot
            .places
            .get(&place)
            .is_some_and(|snapshot| snapshot.tags.contains(&tag))
    }

    fn resource_source(&self, entity: EntityId) -> Option<ResourceSource> {
        let mut source = self
            .snapshot
            .entities
            .get(&entity)
            .and_then(|snapshot| snapshot.resource_source.clone())?;
        if let Some(quantity) = self.resource_quantity_overrides.get(&entity).copied() {
            source.available_quantity = quantity;
        }
        Some(source)
    }

    fn has_production_job(&self, entity: EntityId) -> bool {
        self.snapshot
            .entities
            .get(&entity)
            .is_some_and(|snapshot| snapshot.action_flags.has_production_job)
    }

    fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
        actor == self.snapshot.actor()
            && self
                .snapshot
                .entities
                .get(&entity)
                .is_some_and(|snapshot| snapshot.action_flags.controllable_by_actor)
    }

    fn has_control(&self, entity: EntityId) -> bool {
        self.snapshot
            .entities
            .get(&entity)
            .is_some_and(|snapshot| snapshot.action_flags.has_control)
    }

    fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits> {
        self.carry_capacity_ref(PlanningEntityRef::Authoritative(entity))
    }

    fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits> {
        self.load_of_entity_ref(PlanningEntityRef::Authoritative(entity))
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
                .flat_map(|snapshot| snapshot.reservation_ranges.iter())
                .any(|existing| existing.overlaps(&range))
    }

    fn reservation_ranges(&self, entity: EntityId) -> Vec<TickRange> {
        let mut ranges = self
            .snapshot
            .entities
            .get(&entity)
            .map(|snapshot| snapshot.reservation_ranges.clone())
            .unwrap_or_default();
        if let Some(shadows) = self.reservation_shadows.get(&entity) {
            ranges.extend(shadows.iter().copied());
        }
        ranges
    }

    fn is_dead(&self, entity: EntityId) -> bool {
        self.removed_entities
            .contains(&PlanningEntityRef::Authoritative(entity))
            || self
                .snapshot
                .entities
                .get(&entity)
                .is_some_and(|snapshot| snapshot.lifecycle.dead)
    }

    fn is_incapacitated(&self, entity: EntityId) -> bool {
        self.snapshot
            .entities
            .get(&entity)
            .is_some_and(|snapshot| snapshot.lifecycle.incapacitated)
    }

    fn has_wounds(&self, entity: EntityId) -> bool {
        self.snapshot
            .entities
            .get(&entity)
            .is_some_and(|snapshot| !snapshot.wounds.is_empty())
    }

    fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
        self.needs_overrides.get(&agent).copied().or_else(|| {
            self.snapshot
                .entities
                .get(&agent)
                .and_then(|snapshot| snapshot.homeostatic_needs)
        })
    }

    fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.drive_thresholds)
    }

    fn belief_confidence_policy(&self, agent: EntityId) -> worldwake_core::BeliefConfidencePolicy {
        assert_eq!(
            agent,
            self.snapshot.actor(),
            "belief_confidence_policy is a self-authoritative read and must only be requested for the planning actor"
        );
        self.snapshot.actor_confidence_policy
    }

    fn metabolism_profile(&self, agent: EntityId) -> Option<MetabolismProfile> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.metabolism_profile)
    }

    fn trade_disposition_profile(&self, agent: EntityId) -> Option<TradeDispositionProfile> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.trade_disposition_profile.clone())
    }

    fn travel_disposition_profile(
        &self,
        _agent: EntityId,
    ) -> Option<worldwake_core::TravelDispositionProfile> {
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
        subject: EntityId,
    ) -> Option<ToldBeliefMemory> {
        if actor != self.snapshot.actor() {
            return None;
        }

        let profile = self.tell_profile(actor)?;
        self.snapshot
            .actor_told_beliefs
            .get(&TellMemoryKey {
                counterparty,
                subject,
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
        subject: EntityId,
    ) -> Option<RecipientKnowledgeStatus> {
        if actor != self.snapshot.actor() {
            return None;
        }

        let current_belief = self.snapshot.actor_known_entity_beliefs.get(&subject)?;
        let key = TellMemoryKey {
            counterparty,
            subject,
        };
        self.tell_profile(actor)?;
        let remembered = self.told_belief_memory(actor, counterparty, subject);

        Some(match remembered.as_ref() {
            Some(memory) => {
                worldwake_core::recipient_knowledge_status(current_belief, Some(memory))
            }
            None if self.snapshot.actor_told_beliefs.contains_key(&key) => {
                RecipientKnowledgeStatus::SpeakerPreviouslyToldButMemoryExpired
            }
            None => RecipientKnowledgeStatus::UnknownToSpeaker,
        })
    }

    fn combat_profile(&self, agent: EntityId) -> Option<CombatProfile> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.combat_profile)
    }

    fn courage(&self, agent: EntityId) -> Option<Permille> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.courage)
    }

    fn wounds(&self, agent: EntityId) -> Vec<Wound> {
        self.snapshot
            .entities
            .get(&agent)
            .map(|snapshot| snapshot.wounds.clone())
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

    fn hostile_targets_of(&self, agent: EntityId) -> Vec<EntityId> {
        let agent_place = self.effective_place(agent);
        let agent_transit = self.in_transit_state(agent);
        self.snapshot
            .entities
            .get(&agent)
            .map(|snapshot| {
                snapshot
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

    fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId> {
        let agent_place = self.effective_place(agent);
        let agent_transit = self.in_transit_state(agent);
        self.snapshot
            .entities
            .get(&agent)
            .map(|snapshot| {
                snapshot
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

    fn agents_selling_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
        let mut sellers = self
            .entities_at(place)
            .into_iter()
            .filter(|entity| self.entity_kind(*entity) == Some(EntityKind::Agent))
            .filter(|entity| {
                self.snapshot
                    .entities
                    .get(entity)
                    .and_then(|snapshot| snapshot.merchandise_profile.as_ref())
                    .is_some_and(|profile| profile.sale_kinds.contains(&commodity))
            })
            .collect::<Vec<_>>();
        sellers.sort();
        sellers.dedup();
        sellers
    }

    fn known_recipes(&self, agent: EntityId) -> Vec<RecipeId> {
        self.snapshot
            .entities
            .get(&agent)
            .map(|snapshot| snapshot.known_recipes.clone())
            .unwrap_or_default()
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

    fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation> {
        self.snapshot
            .entities
            .get(&agent)
            .map(|snapshot| snapshot.demand_memory.clone())
            .unwrap_or_default()
    }

    fn support_declaration(&self, supporter: EntityId, office: EntityId) -> Option<EntityId> {
        if supporter != self.snapshot.actor() {
            return None;
        }

        self.support_declaration_overrides
            .get(&(supporter, office))
            .copied()
            .flatten()
            .or_else(|| {
                self.snapshot
                    .actor_support_declarations
                    .get(&office)
                    .copied()
            })
    }

    fn merchandise_profile(&self, agent: EntityId) -> Option<worldwake_core::MerchandiseProfile> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.merchandise_profile.clone())
    }

    fn corpse_entities_at(&self, place: EntityId) -> Vec<EntityId> {
        self.entities_at(place)
            .into_iter()
            .filter(|entity| self.is_dead(*entity))
            .collect()
    }

    fn in_transit_state(&self, entity: EntityId) -> Option<InTransitOnEdge> {
        self.snapshot
            .entities
            .get(&entity)
            .and_then(|snapshot| snapshot.in_transit_state.clone())
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

worldwake_sim::impl_goal_belief_view!(PlanningState<'_>);

#[cfg(test)]
mod tests {
    use super::{HypotheticalEntityId, PlanningEntityRef, PlanningState};
    use crate::planning_snapshot::build_planning_snapshot;
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        ActionDefId, BelievedEntityState, BodyCostPerTick, CombatProfile,
        CommodityConsumableProfile, CommodityKind, DemandObservation, DemandObservationReason,
        DriveThresholds, EntityId, EntityKind, GrantedFacilityUse, HomeostaticNeeds,
        InTransitOnEdge, LoadUnits, MerchandiseProfile, MetabolismProfile, Permille, Quantity,
        RecipeId, RecipientKnowledgeStatus, ResourceSource, TellMemoryKey, TellProfile, Tick,
        TickRange, ToldBeliefMemory, TradeDispositionProfile, UniqueItemKind, WorkstationTag,
        Wound, WoundCause, WoundId,
    };
    use worldwake_sim::{
        get_affordances, ActionDef, ActionDefRegistry, ActionDomain, ActionDuration, ActionError,
        ActionHandler, ActionHandlerId, ActionHandlerRegistry, ActionPayload, ActionProgress,
        ActionState, Constraint, DeterministicRng, DurationExpr, GoalBeliefView, Interruptibility,
        Precondition, ReservationReq, RuntimeBeliefView, TargetSpec,
    };

    struct StubBeliefView {
        current_tick: Tick,
        alive: BTreeMap<EntityId, bool>,
        kinds: BTreeMap<EntityId, EntityKind>,
        effective_places: BTreeMap<EntityId, EntityId>,
        entities_at: BTreeMap<EntityId, Vec<EntityId>>,
        beliefs: BTreeMap<EntityId, Vec<(EntityId, BelievedEntityState)>>,
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
        demand_memory: BTreeMap<EntityId, Vec<DemandObservation>>,
        merchandise_profiles: BTreeMap<EntityId, MerchandiseProfile>,
        tell_profiles: BTreeMap<EntityId, TellProfile>,
        told_beliefs: BTreeMap<EntityId, Vec<(TellMemoryKey, ToldBeliefMemory)>>,
        reservations: BTreeMap<EntityId, Vec<TickRange>>,
        durations: BTreeMap<(EntityId, ActionDefId), ActionDuration>,
        wounds: BTreeMap<EntityId, Vec<Wound>>,
        hostiles: BTreeMap<EntityId, Vec<EntityId>>,
        attackers: BTreeMap<EntityId, Vec<EntityId>>,
        facility_queue_positions: BTreeMap<(EntityId, EntityId), u32>,
        facility_grants: BTreeMap<EntityId, GrantedFacilityUse>,
        courages: BTreeMap<EntityId, Permille>,
        support_declarations: BTreeMap<EntityId, Vec<(EntityId, EntityId)>>,
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
                demand_memory: BTreeMap::new(),
                merchandise_profiles: BTreeMap::new(),
                tell_profiles: BTreeMap::new(),
                told_beliefs: BTreeMap::new(),
                reservations: BTreeMap::new(),
                durations: BTreeMap::new(),
                wounds: BTreeMap::new(),
                hostiles: BTreeMap::new(),
                attackers: BTreeMap::new(),
                facility_queue_positions: BTreeMap::new(),
                facility_grants: BTreeMap::new(),
                courages: BTreeMap::new(),
                support_declarations: BTreeMap::new(),
            }
        }
    }

    impl RuntimeBeliefView for StubBeliefView {
        fn current_tick(&self) -> Tick {
            self.current_tick
        }

        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.get(&entity).copied().unwrap_or(false)
        }

        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.kinds.get(&entity).copied()
        }

        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.effective_places.get(&entity).copied()
        }

        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }

        fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
            self.entities_at.get(&place).cloned().unwrap_or_default()
        }

        fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
            self.beliefs.get(&agent).cloned().unwrap_or_default()
        }

        fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId> {
            self.direct_possessions
                .get(&holder)
                .cloned()
                .unwrap_or_default()
        }

        fn adjacent_places(&self, place: EntityId) -> Vec<EntityId> {
            self.adjacent_places_with_travel_ticks(place)
                .into_iter()
                .map(|(adjacent, _)| adjacent)
                .collect()
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

        fn controlled_commodity_quantity_at_place(
            &self,
            actor: EntityId,
            place: EntityId,
            commodity: CommodityKind,
        ) -> Quantity {
            self.local_controlled_lots_for(actor, place, commodity)
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
            let mut entities = self
                .entities_at(place)
                .into_iter()
                .filter(|entity| self.item_lot_commodity(*entity) == Some(commodity))
                .filter(|entity| self.can_control(actor, *entity))
                .collect::<Vec<_>>();
            entities.sort();
            entities.dedup();
            entities
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

        fn believed_owner_of(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }

        fn facility_queue_position(&self, facility: EntityId, actor: EntityId) -> Option<u32> {
            self.facility_queue_positions
                .get(&(facility, actor))
                .copied()
        }

        fn facility_grant(&self, facility: EntityId) -> Option<&GrantedFacilityUse> {
            self.facility_grants.get(&facility)
        }

        fn resource_source(&self, entity: EntityId) -> Option<ResourceSource> {
            self.resource_sources.get(&entity).cloned()
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }

        fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
            actor == entity || self.direct_possessor(entity) == Some(actor)
        }

        fn has_control(&self, entity: EntityId) -> bool {
            self.kinds.get(&entity) == Some(&EntityKind::Agent)
        }

        fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.carry_capacities.get(&entity).copied()
        }

        fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.entity_loads.get(&entity).copied()
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

        fn is_dead(&self, entity: EntityId) -> bool {
            !self.is_alive(entity)
        }

        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }

        fn has_wounds(&self, entity: EntityId) -> bool {
            self.wounds
                .get(&entity)
                .is_some_and(|wounds| !wounds.is_empty())
        }

        fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
            self.needs.get(&agent).copied()
        }

        fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
            self.thresholds.get(&agent).copied()
        }

        fn belief_confidence_policy(
            &self,
            _agent: EntityId,
        ) -> worldwake_core::BeliefConfidencePolicy {
            worldwake_core::BeliefConfidencePolicy::default()
        }

        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            None
        }

        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
            None
        }

        fn travel_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::TravelDispositionProfile> {
            None
        }

        fn tell_profile(&self, agent: EntityId) -> Option<TellProfile> {
            self.tell_profiles.get(&agent).copied()
        }

        fn told_belief_memories(&self, agent: EntityId) -> Vec<(TellMemoryKey, ToldBeliefMemory)> {
            self.told_beliefs.get(&agent).cloned().unwrap_or_default()
        }

        fn combat_profile(&self, _agent: EntityId) -> Option<CombatProfile> {
            None
        }

        fn courage(&self, agent: EntityId) -> Option<Permille> {
            self.courages.get(&agent).copied()
        }

        fn support_declarations_for_office(&self, office: EntityId) -> Vec<(EntityId, EntityId)> {
            self.support_declarations
                .get(&office)
                .cloned()
                .unwrap_or_default()
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

        fn agents_selling_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            self.entities_at(place)
                .into_iter()
                .filter(|entity| {
                    self.merchandise_profiles
                        .get(entity)
                        .is_some_and(|profile| profile.sale_kinds.contains(&commodity))
                })
                .collect()
        }

        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
            Vec::new()
        }

        fn matching_workstations_at(
            &self,
            _place: EntityId,
            _tag: WorkstationTag,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn resource_sources_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            self.entities_at(place)
                .into_iter()
                .filter(|entity| {
                    self.resource_sources
                        .get(entity)
                        .is_some_and(|source| source.commodity == commodity)
                })
                .collect()
        }

        fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation> {
            self.demand_memory.get(&agent).cloned().unwrap_or_default()
        }

        fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile> {
            self.merchandise_profiles.get(&agent).cloned()
        }

        fn corpse_entities_at(&self, place: EntityId) -> Vec<EntityId> {
            self.entities_at(place)
                .into_iter()
                .filter(|entity| self.is_dead(*entity))
                .collect()
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

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_start(
        _def: &ActionDef,
        _instance: &worldwake_sim::ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut worldwake_core::WorldTxn<'_>,
    ) -> Result<Option<ActionState>, ActionError> {
        Ok(None)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_tick(
        _def: &ActionDef,
        _instance: &mut worldwake_sim::ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut worldwake_core::WorldTxn<'_>,
    ) -> Result<ActionProgress, ActionError> {
        Ok(ActionProgress::Continue)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_commit(
        _def: &ActionDef,
        _instance: &worldwake_sim::ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut worldwake_core::WorldTxn<'_>,
    ) -> Result<worldwake_sim::CommitOutcome, ActionError> {
        Ok(worldwake_sim::CommitOutcome::empty())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_abort(
        _def: &ActionDef,
        _instance: &worldwake_sim::ActionInstance,
        _reason: &worldwake_sim::AbortReason,
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
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![Precondition::ActorAlive],
            visibility: worldwake_core::VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
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
            RuntimeBeliefView::effective_place(&state, actor),
            Some(town)
        );
        assert_eq!(
            RuntimeBeliefView::direct_possessions(&state, actor),
            vec![bread]
        );
        assert_eq!(
            RuntimeBeliefView::commodity_quantity(&state, actor, CommodityKind::Bread),
            Quantity(1)
        );
        assert_eq!(
            RuntimeBeliefView::demand_memory(&state, actor),
            RuntimeBeliefView::demand_memory(&view, actor)
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
            RuntimeBeliefView::entity_kind(&state, corpse),
            Some(EntityKind::Agent)
        );
        assert!(RuntimeBeliefView::is_dead(&state, corpse));
        assert_eq!(
            RuntimeBeliefView::effective_place(&state, corpse),
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
            GrantedFacilityUse {
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
            Some(&GrantedFacilityUse {
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
            Some(&GrantedFacilityUse {
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
            GrantedFacilityUse {
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
                .and_then(|entity| entity.facility_queue.as_ref())
                .and_then(|queue| queue.active_grant.as_ref()),
            Some(&GrantedFacilityUse {
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
            RuntimeBeliefView::effective_place(&state, actor),
            Some(field)
        );
        assert_eq!(
            RuntimeBeliefView::effective_place(&state, bread),
            Some(field)
        );
        assert_eq!(
            RuntimeBeliefView::entities_at(&state, field),
            vec![actor, bread]
        );
        assert_eq!(
            RuntimeBeliefView::direct_possessions(&state, actor),
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
            RuntimeBeliefView::resource_source(&state, bread)
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
        assert!(RuntimeBeliefView::is_dead(&removed, bread));
        assert!(!RuntimeBeliefView::is_alive(&removed, bread));
        assert!(RuntimeBeliefView::entities_at(&removed, entity(10))
            .iter()
            .all(|entity| *entity != bread));
        assert!(get_affordances(&removed, actor, &registry, &handlers).is_empty());
    }

    #[test]
    fn consume_override_reduces_hunger_conservatively() {
        let (view, actor, _town, _field, _bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot).consume_commodity(CommodityKind::Bread);
        let thresholds = RuntimeBeliefView::drive_thresholds(&state, actor).unwrap();

        assert!(
            RuntimeBeliefView::homeostatic_needs(&state, actor)
                .unwrap()
                .hunger
                < thresholds.hunger.low()
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
                    last_known_place: Some(town),
                    last_known_inventory: BTreeMap::from([(CommodityKind::Bread, Quantity(1))]),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    observed_tick: Tick(4),
                    source: worldwake_core::PerceptionSource::DirectObservation,
                },
            )],
        );
        view.tell_profiles.insert(
            actor,
            TellProfile {
                max_tell_candidates: 4,
                max_relay_chain_len: 2,
                acceptance_fidelity: pm(650),
                ..TellProfile::default()
            },
        );

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            RuntimeBeliefView::known_entity_beliefs(&state, actor),
            view.beliefs.get(&actor).cloned().unwrap()
        );
        assert_eq!(
            RuntimeBeliefView::tell_profile(&state, actor),
            view.tell_profiles.get(&actor).copied()
        );
    }

    #[test]
    fn planning_state_preserves_missing_actor_tell_profile_from_snapshot() {
        let (view, actor, _town, _field, bread) = test_view();
        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::from([bread]), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(RuntimeBeliefView::tell_profile(&state, actor), None);
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
                    last_known_place: Some(town),
                    last_known_inventory: BTreeMap::from([(CommodityKind::Bread, Quantity(2))]),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    observed_tick: Tick(7),
                    source: worldwake_core::PerceptionSource::DirectObservation,
                },
            )],
        );
        view.tell_profiles.insert(actor, TellProfile::default());
        view.told_beliefs.insert(
            actor,
            vec![(
                TellMemoryKey {
                    counterparty: listener,
                    subject: bread,
                },
                ToldBeliefMemory {
                    shared_state: worldwake_core::to_shared_belief_snapshot(&BelievedEntityState {
                        last_known_place: Some(town),
                        last_known_inventory: BTreeMap::from([(CommodityKind::Bread, Quantity(1))]),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        last_known_courage: None,
                        observed_tick: Tick(4),
                        source: worldwake_core::PerceptionSource::DirectObservation,
                    }),
                    told_tick: Tick(6),
                },
            )],
        );

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(RuntimeBeliefView::current_tick(&state), Tick(8));
        assert_eq!(
            RuntimeBeliefView::told_belief_memory(&state, actor, listener, bread)
                .map(|m| m.told_tick),
            Some(Tick(6))
        );
        assert_eq!(
            RuntimeBeliefView::recipient_knowledge_status(&state, actor, listener, bread),
            Some(RecipientKnowledgeStatus::SpeakerHasOnlyToldStaleBelief)
        );
    }

    #[test]
    fn overlay_clones_share_snapshot_owned_heavy_vectors() {
        let (view, actor, _town, field, _bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let base = PlanningState::new(&snapshot);
        let moved = base.clone().move_actor_to(field);

        let base_wounds = &base.snapshot().entities.get(&actor).unwrap().wounds;
        let moved_wounds = &moved.snapshot().entities.get(&actor).unwrap().wounds;
        let base_demand = &base.snapshot().entities.get(&actor).unwrap().demand_memory;
        let moved_demand = &moved.snapshot().entities.get(&actor).unwrap().demand_memory;

        assert!(std::ptr::eq(base_wounds.as_ptr(), moved_wounds.as_ptr()));
        assert!(std::ptr::eq(base_demand.as_ptr(), moved_demand.as_ptr()));
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

        assert!(RuntimeBeliefView::visible_hostiles_for(&moved, actor).is_empty());
        assert!(RuntimeBeliefView::current_attackers_of(&moved, actor).is_empty());
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

        assert!(RuntimeBeliefView::visible_hostiles_for(&state, actor).is_empty());
        assert!(RuntimeBeliefView::hostile_targets_of(&state, actor).is_empty());
        assert!(RuntimeBeliefView::current_attackers_of(&state, actor).is_empty());
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
            RuntimeBeliefView::controlled_commodity_quantity_at_place(
                &local,
                actor,
                town,
                CommodityKind::Bread
            ),
            Quantity(3)
        );
        assert_eq!(
            RuntimeBeliefView::controlled_commodity_quantity_at_place(
                &local,
                actor,
                field,
                CommodityKind::Bread
            ),
            Quantity(0)
        );
        assert_eq!(
            RuntimeBeliefView::controlled_commodity_quantity_at_place(
                &moved,
                actor,
                town,
                CommodityKind::Bread
            ),
            Quantity(0)
        );
        assert_eq!(
            RuntimeBeliefView::controlled_commodity_quantity_at_place(
                &moved,
                actor,
                field,
                CommodityKind::Bread
            ),
            Quantity(3)
        );
        assert_eq!(
            RuntimeBeliefView::local_controlled_lots_for(&local, actor, town, CommodityKind::Bread),
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
            RuntimeBeliefView::controlled_commodity_quantity_at_place(
                &moved,
                actor,
                field,
                CommodityKind::Bread
            ),
            Quantity(3)
        );
        assert_eq!(
            RuntimeBeliefView::local_controlled_lots_for(
                &moved,
                actor,
                field,
                CommodityKind::Bread
            ),
            vec![bread]
        );
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
    fn removed_hypothetical_entities_stop_answering_ref_queries_and_do_not_leak_through_belief_view(
    ) {
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
            RuntimeBeliefView::direct_possessions(&removed, actor),
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
            RuntimeBeliefView::carry_capacity(&state, actor),
            Some(LoadUnits(10))
        );
        assert_eq!(
            RuntimeBeliefView::load_of_entity(&state, bread),
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
            Some(LoadUnits(worldwake_core::load_per_unit(CommodityKind::Bread).0))
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
            RuntimeBeliefView::courage(&state, actor),
            Some(courage_value)
        );

        // Entity in snapshot without UtilityProfile (bread is an ItemLot) returns None
        assert_eq!(RuntimeBeliefView::courage(&state, bread), None);

        // Entity not in snapshot returns None
        let unknown = entity(999);
        assert_eq!(RuntimeBeliefView::courage(&state, unknown), None);
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
    fn hypothetical_support_count_base_only() {
        let (mut view, actor, _rival, supporter_a, supporter_b, office) = support_test_setup();
        // supporter_a → actor, supporter_b → actor
        view.support_declarations
            .insert(office, vec![(supporter_a, actor), (supporter_b, actor)]);

        let snapshot = build_support_snapshot(&view, actor, office);
        let state = PlanningState::new(&snapshot);

        assert_eq!(state.hypothetical_support_count(office, actor), 2);
    }

    #[test]
    fn hypothetical_support_count_with_override_changing_existing() {
        let (mut view, actor, rival, supporter_a, supporter_b, office) = support_test_setup();
        // Base: both support actor
        view.support_declarations
            .insert(office, vec![(supporter_a, actor), (supporter_b, actor)]);

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
        // Base: only supporter_a supports actor
        view.support_declarations
            .insert(office, vec![(supporter_a, actor)]);

        let snapshot = build_support_snapshot(&view, actor, office);
        // Hypothetical: supporter_b (not in base) now also supports actor
        let state =
            PlanningState::new(&snapshot).with_support_declaration(supporter_b, office, actor);

        assert_eq!(state.hypothetical_support_count(office, actor), 2);
    }

    #[test]
    fn has_support_majority_true_when_strictly_more() {
        let (mut view, actor, rival, supporter_a, supporter_b, office) = support_test_setup();
        // actor has 2, rival has 1
        view.support_declarations.insert(
            office,
            vec![(supporter_a, actor), (supporter_b, actor), (rival, rival)],
        );

        let snapshot = build_support_snapshot(&view, actor, office);
        let state = PlanningState::new(&snapshot);

        assert!(state.has_support_majority(office, actor));
        assert!(!state.has_support_majority(office, rival));
    }

    #[test]
    fn has_support_majority_false_on_tie() {
        let (mut view, actor, rival, supporter_a, supporter_b, office) = support_test_setup();
        // actor has 1, rival has 1
        view.support_declarations
            .insert(office, vec![(supporter_a, actor), (supporter_b, rival)]);

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
        // Only one declaration, no competitors
        view.support_declarations
            .insert(office, vec![(supporter_a, actor)]);

        let snapshot = build_support_snapshot(&view, actor, office);
        let state = PlanningState::new(&snapshot);

        assert!(state.has_support_majority(office, actor));
    }
}
