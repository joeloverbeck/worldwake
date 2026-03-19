use crate::{ActionDuration, ActionPayload, DurationExpr};
use std::num::NonZeroU32;
use worldwake_core::{
    BeliefConfidencePolicy, BelievedEntityState, CombatProfile, CommodityConsumableProfile,
    CommodityKind, CommodityTreatmentProfile, DemandObservation, DriveThresholds, EntityId,
    EntityKind, GrantedFacilityUse, HomeostaticNeeds, InTransitOnEdge, LoadUnits,
    MerchandiseProfile, MetabolismProfile, OfficeData, Permille, PlaceTag, Quantity, RecipeId,
    RecipientKnowledgeStatus, ResourceSource, TellMemoryKey, TellProfile, Tick, TickRange,
    ToldBeliefMemory, TradeDispositionProfile, TravelDispositionProfile, UniqueItemKind,
    WorkstationTag, Wound,
};

/// Narrow AI-facing surface for goal formation, pressure derivation, ranking, and explanation.
///
/// Classification:
/// - subjective reads: observed non-self state such as `effective_place`, `commodity_quantity`,
///   `corpse_entities_at`, `agents_selling_at`
/// - self-authoritative reads: self needs, wounds, recipes, inventory, load, profiles
/// - public structure reads: topology, place tags, workstation and source discovery
///
/// Deliberately excluded from this trait:
/// - queue and reservation helpers
/// - duration estimation
/// - broader affordance/runtime helpers used by snapshot/search code
pub trait GoalBeliefView {
    fn current_tick(&self) -> Tick {
        Tick(0)
    }
    fn is_alive(&self, entity: EntityId) -> bool;
    fn is_dead(&self, entity: EntityId) -> bool;
    fn entity_kind(&self, entity: EntityId) -> Option<EntityKind>;
    fn effective_place(&self, entity: EntityId) -> Option<EntityId>;
    fn entities_at(&self, place: EntityId) -> Vec<EntityId>;
    fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
        let _ = agent;
        Vec::new()
    }
    fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId>;
    fn adjacent_places_with_travel_ticks(&self, place: EntityId) -> Vec<(EntityId, NonZeroU32)>;
    fn knows_recipe(&self, actor: EntityId, recipe: RecipeId) -> bool;
    fn known_recipes(&self, agent: EntityId) -> Vec<RecipeId>;
    fn unique_item_count(&self, holder: EntityId, kind: UniqueItemKind) -> u32;
    fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity;
    fn controlled_commodity_quantity_at_place(
        &self,
        agent: EntityId,
        place: EntityId,
        commodity: CommodityKind,
    ) -> Quantity;
    fn local_controlled_lots_for(
        &self,
        agent: EntityId,
        place: EntityId,
        commodity: CommodityKind,
    ) -> Vec<EntityId>;
    fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind>;
    fn item_lot_consumable_profile(&self, entity: EntityId) -> Option<CommodityConsumableProfile>;
    fn direct_container(&self, entity: EntityId) -> Option<EntityId>;
    fn direct_possessor(&self, entity: EntityId) -> Option<EntityId>;
    fn believed_owner_of(&self, entity: EntityId) -> Option<EntityId>;
    fn workstation_tag(&self, entity: EntityId) -> Option<WorkstationTag>;
    fn resource_source(&self, entity: EntityId) -> Option<ResourceSource>;
    fn resource_sources_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId>;
    fn matching_workstations_at(&self, place: EntityId, tag: WorkstationTag) -> Vec<EntityId>;
    fn has_production_job(&self, entity: EntityId) -> bool;
    fn can_control(&self, actor: EntityId, entity: EntityId) -> bool;
    fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits>;
    fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits>;
    fn is_incapacitated(&self, entity: EntityId) -> bool;
    fn has_wounds(&self, entity: EntityId) -> bool;
    fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds>;
    fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds>;
    fn belief_confidence_policy(&self, agent: EntityId) -> BeliefConfidencePolicy;
    fn tell_profile(&self, agent: EntityId) -> Option<TellProfile> {
        let _ = agent;
        None
    }
    fn told_belief_memories(&self, agent: EntityId) -> Vec<(TellMemoryKey, ToldBeliefMemory)> {
        let _ = agent;
        Vec::new()
    }
    fn told_belief_memory(
        &self,
        actor: EntityId,
        counterparty: EntityId,
        subject: EntityId,
    ) -> Option<ToldBeliefMemory> {
        let _ = (actor, counterparty, subject);
        None
    }
    fn recipient_knowledge_status(
        &self,
        actor: EntityId,
        counterparty: EntityId,
        subject: EntityId,
    ) -> Option<RecipientKnowledgeStatus> {
        let _ = (actor, counterparty, subject);
        None
    }
    fn courage(&self, agent: EntityId) -> Option<Permille> {
        let _ = agent;
        None
    }
    fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile>;
    fn wounds(&self, agent: EntityId) -> Vec<Wound>;
    fn hostile_targets_of(&self, agent: EntityId) -> Vec<EntityId>;
    fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId>;
    fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId>;
    fn agents_selling_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId>;
    fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation>;
    fn corpse_entities_at(&self, place: EntityId) -> Vec<EntityId>;
    fn office_data(&self, office: EntityId) -> Option<OfficeData> {
        let _ = office;
        None
    }
    fn office_holder(&self, office: EntityId) -> Option<EntityId> {
        let _ = office;
        None
    }
    fn factions_of(&self, member: EntityId) -> Vec<EntityId> {
        let _ = member;
        Vec::new()
    }
    fn loyalty_to(&self, subject: EntityId, target: EntityId) -> Option<Permille> {
        let _ = (subject, target);
        None
    }
    fn support_declaration(&self, supporter: EntityId, office: EntityId) -> Option<EntityId> {
        let _ = (supporter, office);
        None
    }
    fn support_declarations_for_office(&self, office: EntityId) -> Vec<(EntityId, EntityId)> {
        let _ = office;
        Vec::new()
    }
}

/// Richer AI/runtime-facing surface for planning snapshots, affordance search, revalidation,
/// failure handling, and duration estimation.
///
/// This trait is intentionally broader than `GoalBeliefView`. Callers should only depend on it
/// when they truly need runtime-only helpers such as reservations, queue state, or duration
/// estimation.
pub trait RuntimeBeliefView {
    fn current_tick(&self) -> Tick {
        Tick(0)
    }
    fn is_alive(&self, entity: EntityId) -> bool;
    fn entity_kind(&self, entity: EntityId) -> Option<EntityKind>;
    fn effective_place(&self, entity: EntityId) -> Option<EntityId>;
    fn is_in_transit(&self, entity: EntityId) -> bool;
    fn entities_at(&self, place: EntityId) -> Vec<EntityId>;
    fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
        let _ = agent;
        Vec::new()
    }
    fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId>;
    fn adjacent_places(&self, place: EntityId) -> Vec<EntityId>;
    fn knows_recipe(&self, actor: EntityId, recipe: RecipeId) -> bool;
    fn unique_item_count(&self, holder: EntityId, kind: UniqueItemKind) -> u32;
    fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity;
    fn controlled_commodity_quantity_at_place(
        &self,
        agent: EntityId,
        place: EntityId,
        commodity: CommodityKind,
    ) -> Quantity;
    fn local_controlled_lots_for(
        &self,
        agent: EntityId,
        place: EntityId,
        commodity: CommodityKind,
    ) -> Vec<EntityId>;
    fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind>;
    fn item_lot_consumable_profile(&self, entity: EntityId) -> Option<CommodityConsumableProfile>;
    fn direct_container(&self, entity: EntityId) -> Option<EntityId>;
    fn direct_possessor(&self, entity: EntityId) -> Option<EntityId>;
    fn believed_owner_of(&self, entity: EntityId) -> Option<EntityId>;
    fn workstation_tag(&self, entity: EntityId) -> Option<WorkstationTag>;
    fn has_exclusive_facility_policy(&self, entity: EntityId) -> bool {
        let _ = entity;
        false
    }
    fn facility_queue_position(&self, facility: EntityId, actor: EntityId) -> Option<u32> {
        let _ = (facility, actor);
        None
    }
    fn facility_grant(&self, facility: EntityId) -> Option<&GrantedFacilityUse> {
        let _ = facility;
        None
    }
    fn facility_queue_join_tick(&self, facility: EntityId, actor: EntityId) -> Option<Tick> {
        let _ = (facility, actor);
        None
    }
    fn facility_queue_patience_ticks(&self, agent: EntityId) -> Option<NonZeroU32> {
        let _ = agent;
        None
    }
    fn place_has_tag(&self, place: EntityId, tag: PlaceTag) -> bool {
        let _ = (place, tag);
        false
    }
    fn resource_source(&self, entity: EntityId) -> Option<ResourceSource>;
    fn has_production_job(&self, entity: EntityId) -> bool;
    fn can_control(&self, actor: EntityId, entity: EntityId) -> bool;
    fn has_control(&self, entity: EntityId) -> bool;
    fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits>;
    fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits>;
    fn reservation_conflicts(&self, entity: EntityId, range: TickRange) -> bool;
    fn reservation_ranges(&self, entity: EntityId) -> Vec<TickRange>;
    fn is_dead(&self, entity: EntityId) -> bool;
    fn is_incapacitated(&self, entity: EntityId) -> bool;
    fn has_wounds(&self, entity: EntityId) -> bool;
    fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds>;
    fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds>;
    fn belief_confidence_policy(&self, agent: EntityId) -> BeliefConfidencePolicy;
    fn metabolism_profile(&self, agent: EntityId) -> Option<MetabolismProfile>;
    fn trade_disposition_profile(&self, agent: EntityId) -> Option<TradeDispositionProfile>;
    fn travel_disposition_profile(&self, agent: EntityId) -> Option<TravelDispositionProfile>;
    fn tell_profile(&self, agent: EntityId) -> Option<TellProfile> {
        let _ = agent;
        None
    }
    fn told_belief_memories(&self, agent: EntityId) -> Vec<(TellMemoryKey, ToldBeliefMemory)> {
        let _ = agent;
        Vec::new()
    }
    fn told_belief_memory(
        &self,
        actor: EntityId,
        counterparty: EntityId,
        subject: EntityId,
    ) -> Option<ToldBeliefMemory> {
        let _ = (actor, counterparty, subject);
        None
    }
    fn recipient_knowledge_status(
        &self,
        actor: EntityId,
        counterparty: EntityId,
        subject: EntityId,
    ) -> Option<RecipientKnowledgeStatus> {
        let _ = (actor, counterparty, subject);
        None
    }
    fn combat_profile(&self, agent: EntityId) -> Option<CombatProfile>;
    fn courage(&self, agent: EntityId) -> Option<Permille> {
        let _ = agent;
        None
    }
    fn wounds(&self, agent: EntityId) -> Vec<Wound>;
    fn hostile_targets_of(&self, agent: EntityId) -> Vec<EntityId> {
        self.visible_hostiles_for(agent)
    }
    fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId>;
    fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId>;
    fn agents_selling_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId>;
    fn known_recipes(&self, agent: EntityId) -> Vec<RecipeId>;
    fn matching_workstations_at(&self, place: EntityId, tag: WorkstationTag) -> Vec<EntityId>;
    fn resource_sources_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId>;
    fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation>;
    fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile>;
    fn corpse_entities_at(&self, place: EntityId) -> Vec<EntityId>;
    fn office_data(&self, office: EntityId) -> Option<OfficeData> {
        let _ = office;
        None
    }
    fn office_holder(&self, office: EntityId) -> Option<EntityId> {
        let _ = office;
        None
    }
    fn factions_of(&self, member: EntityId) -> Vec<EntityId> {
        let _ = member;
        Vec::new()
    }
    fn loyalty_to(&self, subject: EntityId, target: EntityId) -> Option<Permille> {
        let _ = (subject, target);
        None
    }
    fn support_declaration(&self, supporter: EntityId, office: EntityId) -> Option<EntityId> {
        let _ = (supporter, office);
        None
    }
    fn support_declarations_for_office(&self, office: EntityId) -> Vec<(EntityId, EntityId)> {
        let _ = office;
        Vec::new()
    }
    fn in_transit_state(&self, entity: EntityId) -> Option<InTransitOnEdge>;
    fn adjacent_places_with_travel_ticks(&self, place: EntityId) -> Vec<(EntityId, NonZeroU32)>;
    fn estimate_duration(
        &self,
        actor: EntityId,
        duration: &DurationExpr,
        targets: &[EntityId],
        payload: &ActionPayload,
    ) -> Option<ActionDuration>;
}

#[macro_export]
macro_rules! impl_goal_belief_view {
    ($ty:ty) => {
        impl $crate::GoalBeliefView for $ty {
            fn current_tick(&self) -> worldwake_core::Tick {
                $crate::RuntimeBeliefView::current_tick(self)
            }

            fn is_alive(&self, entity: worldwake_core::EntityId) -> bool {
                $crate::RuntimeBeliefView::is_alive(self, entity)
            }

            fn is_dead(&self, entity: worldwake_core::EntityId) -> bool {
                $crate::RuntimeBeliefView::is_dead(self, entity)
            }

            fn entity_kind(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::EntityKind> {
                $crate::RuntimeBeliefView::entity_kind(self, entity)
            }

            fn effective_place(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::effective_place(self, entity)
            }

            fn entities_at(
                &self,
                place: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::entities_at(self, place)
            }

            fn direct_possessions(
                &self,
                holder: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::direct_possessions(self, holder)
            }

            fn known_entity_beliefs(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Vec<(
                worldwake_core::EntityId,
                worldwake_core::BelievedEntityState,
            )> {
                $crate::RuntimeBeliefView::known_entity_beliefs(self, agent)
            }

            fn adjacent_places_with_travel_ticks(
                &self,
                place: worldwake_core::EntityId,
            ) -> Vec<(worldwake_core::EntityId, std::num::NonZeroU32)> {
                $crate::RuntimeBeliefView::adjacent_places_with_travel_ticks(self, place)
            }

            fn knows_recipe(
                &self,
                actor: worldwake_core::EntityId,
                recipe: worldwake_core::RecipeId,
            ) -> bool {
                $crate::RuntimeBeliefView::knows_recipe(self, actor, recipe)
            }

            fn known_recipes(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::RecipeId> {
                $crate::RuntimeBeliefView::known_recipes(self, agent)
            }

            fn unique_item_count(
                &self,
                holder: worldwake_core::EntityId,
                kind: worldwake_core::UniqueItemKind,
            ) -> u32 {
                $crate::RuntimeBeliefView::unique_item_count(self, holder, kind)
            }

            fn commodity_quantity(
                &self,
                holder: worldwake_core::EntityId,
                kind: worldwake_core::CommodityKind,
            ) -> worldwake_core::Quantity {
                $crate::RuntimeBeliefView::commodity_quantity(self, holder, kind)
            }

            fn controlled_commodity_quantity_at_place(
                &self,
                agent: worldwake_core::EntityId,
                place: worldwake_core::EntityId,
                commodity: worldwake_core::CommodityKind,
            ) -> worldwake_core::Quantity {
                $crate::RuntimeBeliefView::controlled_commodity_quantity_at_place(
                    self, agent, place, commodity,
                )
            }

            fn local_controlled_lots_for(
                &self,
                agent: worldwake_core::EntityId,
                place: worldwake_core::EntityId,
                commodity: worldwake_core::CommodityKind,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::local_controlled_lots_for(self, agent, place, commodity)
            }

            fn item_lot_commodity(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::CommodityKind> {
                $crate::RuntimeBeliefView::item_lot_commodity(self, entity)
            }

            fn item_lot_consumable_profile(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::CommodityConsumableProfile> {
                $crate::RuntimeBeliefView::item_lot_consumable_profile(self, entity)
            }

            fn direct_container(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::direct_container(self, entity)
            }

            fn direct_possessor(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::direct_possessor(self, entity)
            }

            fn believed_owner_of(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::believed_owner_of(self, entity)
            }

            fn workstation_tag(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::WorkstationTag> {
                $crate::RuntimeBeliefView::workstation_tag(self, entity)
            }

            fn resource_source(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::ResourceSource> {
                $crate::RuntimeBeliefView::resource_source(self, entity)
            }

            fn resource_sources_at(
                &self,
                place: worldwake_core::EntityId,
                commodity: worldwake_core::CommodityKind,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::resource_sources_at(self, place, commodity)
            }

            fn matching_workstations_at(
                &self,
                place: worldwake_core::EntityId,
                tag: worldwake_core::WorkstationTag,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::matching_workstations_at(self, place, tag)
            }

            fn has_production_job(&self, entity: worldwake_core::EntityId) -> bool {
                $crate::RuntimeBeliefView::has_production_job(self, entity)
            }

            fn can_control(
                &self,
                actor: worldwake_core::EntityId,
                entity: worldwake_core::EntityId,
            ) -> bool {
                $crate::RuntimeBeliefView::can_control(self, actor, entity)
            }

            fn carry_capacity(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::LoadUnits> {
                $crate::RuntimeBeliefView::carry_capacity(self, entity)
            }

            fn load_of_entity(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::LoadUnits> {
                $crate::RuntimeBeliefView::load_of_entity(self, entity)
            }

            fn is_incapacitated(&self, entity: worldwake_core::EntityId) -> bool {
                $crate::RuntimeBeliefView::is_incapacitated(self, entity)
            }

            fn has_wounds(&self, entity: worldwake_core::EntityId) -> bool {
                $crate::RuntimeBeliefView::has_wounds(self, entity)
            }

            fn homeostatic_needs(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::HomeostaticNeeds> {
                $crate::RuntimeBeliefView::homeostatic_needs(self, agent)
            }

            fn drive_thresholds(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::DriveThresholds> {
                $crate::RuntimeBeliefView::drive_thresholds(self, agent)
            }

            fn belief_confidence_policy(
                &self,
                agent: worldwake_core::EntityId,
            ) -> worldwake_core::BeliefConfidencePolicy {
                $crate::RuntimeBeliefView::belief_confidence_policy(self, agent)
            }

            fn tell_profile(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::TellProfile> {
                $crate::RuntimeBeliefView::tell_profile(self, agent)
            }

            fn told_belief_memories(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Vec<(
                worldwake_core::TellMemoryKey,
                worldwake_core::ToldBeliefMemory,
            )> {
                $crate::RuntimeBeliefView::told_belief_memories(self, agent)
            }

            fn told_belief_memory(
                &self,
                actor: worldwake_core::EntityId,
                counterparty: worldwake_core::EntityId,
                subject: worldwake_core::EntityId,
            ) -> Option<worldwake_core::ToldBeliefMemory> {
                $crate::RuntimeBeliefView::told_belief_memory(self, actor, counterparty, subject)
            }

            fn recipient_knowledge_status(
                &self,
                actor: worldwake_core::EntityId,
                counterparty: worldwake_core::EntityId,
                subject: worldwake_core::EntityId,
            ) -> Option<worldwake_core::RecipientKnowledgeStatus> {
                $crate::RuntimeBeliefView::recipient_knowledge_status(
                    self,
                    actor,
                    counterparty,
                    subject,
                )
            }

            fn courage(&self, agent: worldwake_core::EntityId) -> Option<worldwake_core::Permille> {
                $crate::RuntimeBeliefView::courage(self, agent)
            }

            fn merchandise_profile(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::MerchandiseProfile> {
                $crate::RuntimeBeliefView::merchandise_profile(self, agent)
            }

            fn wounds(&self, agent: worldwake_core::EntityId) -> Vec<worldwake_core::Wound> {
                $crate::RuntimeBeliefView::wounds(self, agent)
            }

            fn hostile_targets_of(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::hostile_targets_of(self, agent)
            }

            fn visible_hostiles_for(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::visible_hostiles_for(self, agent)
            }

            fn current_attackers_of(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::current_attackers_of(self, agent)
            }

            fn agents_selling_at(
                &self,
                place: worldwake_core::EntityId,
                commodity: worldwake_core::CommodityKind,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::agents_selling_at(self, place, commodity)
            }

            fn demand_memory(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::DemandObservation> {
                $crate::RuntimeBeliefView::demand_memory(self, agent)
            }

            fn corpse_entities_at(
                &self,
                place: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::corpse_entities_at(self, place)
            }

            fn office_data(
                &self,
                office: worldwake_core::EntityId,
            ) -> Option<worldwake_core::OfficeData> {
                $crate::RuntimeBeliefView::office_data(self, office)
            }

            fn office_holder(
                &self,
                office: worldwake_core::EntityId,
            ) -> Option<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::office_holder(self, office)
            }

            fn factions_of(
                &self,
                member: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::factions_of(self, member)
            }

            fn loyalty_to(
                &self,
                subject: worldwake_core::EntityId,
                target: worldwake_core::EntityId,
            ) -> Option<worldwake_core::Permille> {
                $crate::RuntimeBeliefView::loyalty_to(self, subject, target)
            }

            fn support_declaration(
                &self,
                supporter: worldwake_core::EntityId,
                office: worldwake_core::EntityId,
            ) -> Option<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::support_declaration(self, supporter, office)
            }

            fn support_declarations_for_office(
                &self,
                office: worldwake_core::EntityId,
            ) -> Vec<(worldwake_core::EntityId, worldwake_core::EntityId)> {
                $crate::RuntimeBeliefView::support_declarations_for_office(self, office)
            }
        }
    };
}

#[must_use]
pub fn estimate_duration_from_beliefs(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    duration: &DurationExpr,
    targets: &[EntityId],
    payload: &ActionPayload,
) -> Option<ActionDuration> {
    match *duration {
        DurationExpr::Fixed(ticks) => Some(ActionDuration::Finite(ticks.get())),
        DurationExpr::TargetConsumable { target_index } => {
            let target = targets.get(usize::from(target_index)).copied()?;
            let profile = view.item_lot_consumable_profile(target)?;
            Some(ActionDuration::Finite(
                profile.consumption_ticks_per_unit.get(),
            ))
        }
        DurationExpr::TravelToTarget { target_index } => {
            let target = targets.get(usize::from(target_index)).copied()?;
            let origin = view.effective_place(actor)?;
            view.adjacent_places_with_travel_ticks(origin)
                .into_iter()
                .find_map(|(adjacent, ticks)| {
                    (adjacent == target).then_some(ActionDuration::Finite(ticks.get()))
                })
        }
        DurationExpr::ActorMetabolism { kind } => {
            let profile = view.metabolism_profile(actor)?;
            let ticks = match kind {
                crate::MetabolismDurationKind::Toilet => profile.toilet_ticks.get(),
                crate::MetabolismDurationKind::Wash => profile.wash_ticks.get(),
            };
            Some(ActionDuration::Finite(ticks))
        }
        DurationExpr::ActorTradeDisposition => view
            .trade_disposition_profile(actor)
            .map(|profile| ActionDuration::Finite(profile.negotiation_round_ticks.get())),
        DurationExpr::Indefinite => Some(ActionDuration::Indefinite),
        DurationExpr::CombatWeapon => {
            let combat = payload.as_combat()?;
            match combat.weapon {
                worldwake_core::CombatWeaponRef::Unarmed => view
                    .combat_profile(actor)
                    .map(|profile| ActionDuration::Finite(profile.unarmed_attack_ticks.get())),
                worldwake_core::CombatWeaponRef::Commodity(kind) => kind
                    .spec()
                    .combat_weapon_profile
                    .map(|profile| ActionDuration::Finite(profile.attack_duration_ticks.get())),
            }
        }
        DurationExpr::TargetTreatment {
            target_index,
            commodity,
        } => {
            if view.commodity_quantity(actor, commodity) == Quantity(0) {
                return None;
            }
            let target = targets.get(usize::from(target_index)).copied()?;
            let wounds = view.wounds(target);
            if wounds.is_empty() {
                return None;
            }
            let CommodityTreatmentProfile {
                treatment_ticks_per_unit,
                severity_reduction_per_tick,
                ..
            } = commodity.spec().treatment_profile?;
            let wound_load = wounds.iter().fold(0u32, |acc, wound| {
                acc.saturating_add(u32::from(wound.severity.value()))
            });
            let severity_per_tick = u32::from(severity_reduction_per_tick.value()).max(1);
            let wound_ticks = wound_load.div_ceil(severity_per_tick).max(1);
            Some(ActionDuration::Finite(
                treatment_ticks_per_unit.get().max(wound_ticks),
            ))
        }
    }
}
