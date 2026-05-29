use crate::institutional_queries::consulted_office_holder_read_for_record_data;
use crate::planner_ops::ExpectedMaterialization;
use crate::planning_state::{PlanningEntityRef, PlanningState};
use worldwake_core::{
    ActionDefId, CommodityKind, Discrepancy, EntityId, EntityKind, EventTag, ExpectationId,
    InstitutionalBeliefRead, Permille, Quantity, WoundCause, load_per_unit,
};
use worldwake_sim::{
    ActionPayload, CombatBeliefView, ControlBeliefView, EconomicBeliefView, EffectEntityRef,
    EffectFact, EffectPrecondition, EffectSink, FacilityBeliefView, InventoryBeliefView,
    Materialization, MaterializationTag, ProfileBeliefView, SpatialBeliefView, TargetSpec,
    TemporalBeliefView,
};

pub struct HypotheticalEffectSink<'snapshot> {
    state: PlanningState<'snapshot>,
    emitted_events: Vec<EventTag>,
    fulfilled_expectations: Vec<ExpectationId>,
    expected_materializations: Vec<ExpectedMaterialization>,
    checkpoints: Vec<HypotheticalEffectSinkCheckpoint<'snapshot>>,
}

#[derive(Clone)]
struct HypotheticalEffectSinkCheckpoint<'snapshot> {
    state: PlanningState<'snapshot>,
    emitted_events: Vec<EventTag>,
    fulfilled_expectations: Vec<ExpectationId>,
    expected_materializations: Vec<ExpectedMaterialization>,
}

impl<'snapshot> HypotheticalEffectSink<'snapshot> {
    #[must_use]
    pub fn new(state: PlanningState<'snapshot>) -> Self {
        Self {
            state,
            emitted_events: Vec::new(),
            fulfilled_expectations: Vec::new(),
            expected_materializations: Vec::new(),
            checkpoints: Vec::new(),
        }
    }

    #[must_use]
    pub fn state(&self) -> &PlanningState<'snapshot> {
        &self.state
    }

    #[must_use]
    pub fn into_state(self) -> PlanningState<'snapshot> {
        self.state
    }

    #[must_use]
    pub fn emitted_events(&self) -> &[EventTag] {
        &self.emitted_events
    }

    #[must_use]
    pub fn fulfilled_expectations(&self) -> &[ExpectationId] {
        &self.fulfilled_expectations
    }

    #[must_use]
    pub fn expected_materializations(&self) -> &[ExpectedMaterialization] {
        &self.expected_materializations
    }

    fn update_state(
        &mut self,
        update: impl FnOnce(PlanningState<'snapshot>) -> PlanningState<'snapshot>,
    ) {
        let placeholder = PlanningState::new(self.state.snapshot());
        let state = std::mem::replace(&mut self.state, placeholder);
        self.state = update(state);
    }

    fn quantity_available(
        &self,
        source: EntityId,
        commodity: CommodityKind,
        min: Quantity,
    ) -> bool {
        self.state
            .commodity_quantity_ref(PlanningEntityRef::Authoritative(source), commodity)
            >= min
    }

    fn clear_actor_need(
        &mut self,
        actor: EntityId,
        update: impl FnOnce(&mut worldwake_core::HomeostaticNeeds),
    ) {
        let Some(mut needs) = self.state.homeostatic_needs(actor) else {
            return;
        };
        update(&mut needs);
        self.update_state(|state| state.with_homeostatic_needs(actor, needs));
    }

    fn missing_observation_for(&self, actor: EntityId, target: EntityId) -> Discrepancy {
        if self.state.snapshot().entities.contains_key(&target) {
            return Discrepancy::MissingObservation;
        }

        worldwake_sim::GoalBeliefView::observation_omission_log(&self.state, actor)
            .and_then(|log| {
                log.entries
                    .iter()
                    .rev()
                    .find(|entry| entry.omitted_entity == target)
            })
            .map_or(Discrepancy::MissingObservation, |entry| {
                Discrepancy::Omission(entry.reason)
            })
    }

    fn apply_pick_up(
        &mut self,
        actor: EntityId,
        target: EntityId,
        payload: &ActionPayload,
    ) -> Result<Option<Materialization>, Discrepancy> {
        let actor_ref = PlanningEntityRef::Authoritative(actor);
        let lot_ref = PlanningEntityRef::Authoritative(target);
        if self.state.entity_kind_ref(lot_ref).is_none()
            || self.state.effective_place_ref(lot_ref).is_none()
        {
            return Err(self.missing_observation_for(actor, target));
        }
        if self.state.entity_kind_ref(lot_ref) != Some(EntityKind::ItemLot)
            || self.state.direct_possessor_ref(lot_ref).is_some()
            || self.state.effective_place_ref(lot_ref) != self.state.effective_place_ref(actor_ref)
        {
            return Err(Discrepancy::MissingObservation);
        }
        let commodity = self
            .state
            .item_lot_commodity_ref(lot_ref)
            .ok_or_else(|| self.missing_observation_for(actor, target))?;
        let quantity = self.state.commodity_quantity_ref(lot_ref, commodity);
        if quantity == Quantity(0) {
            return Err(Discrepancy::MissingObservation);
        }
        let remaining_capacity = self
            .state
            .remaining_carry_capacity_ref(actor_ref)
            .ok_or(Discrepancy::MissingObservation)?
            .0;
        let per_unit = load_per_unit(commodity).0;
        if remaining_capacity < per_unit {
            return Err(Discrepancy::PartialExecutionDrift);
        }
        let requested_quantity = payload.as_transport().map(|payload| payload.quantity);
        let moved_quantity = requested_quantity.unwrap_or_else(|| {
            if self
                .state
                .load_of_entity_ref(lot_ref)
                .is_some_and(|load| load.0 <= remaining_capacity)
            {
                quantity
            } else {
                Quantity(remaining_capacity / per_unit)
            }
        });
        if moved_quantity == Quantity(0) || moved_quantity > quantity {
            return Err(Discrepancy::PartialExecutionDrift);
        }
        if requested_quantity.is_some_and(|requested| requested != moved_quantity) {
            return Err(Discrepancy::PartialExecutionDrift);
        }
        if moved_quantity == quantity {
            self.update_state(|state| {
                state.move_lot_ref_to_holder(lot_ref, actor_ref, commodity, moved_quantity)
            });
            return Ok(None);
        }

        let actor_place = self.state.effective_place_ref(actor_ref);
        let snapshot = self.state.snapshot();
        let mut state = std::mem::replace(&mut self.state, PlanningState::new(snapshot));
        state = state.set_quantity_ref(lot_ref, commodity, Quantity(quantity.0 - moved_quantity.0));
        let hypothetical_id = state.spawn_hypothetical_lot(EntityKind::ItemLot, commodity);
        let hypothetical_ref = PlanningEntityRef::Hypothetical(hypothetical_id);
        state = state
            .move_entity_ref(
                hypothetical_ref,
                actor_place.ok_or(Discrepancy::MissingObservation)?,
            )
            .set_quantity_ref(hypothetical_ref, commodity, moved_quantity)
            .move_lot_ref_to_holder(hypothetical_ref, actor_ref, commodity, moved_quantity);
        self.state = state;
        self.expected_materializations
            .push(ExpectedMaterialization {
                tag: MaterializationTag::SplitOffLot,
                hypothetical_id,
            });
        Ok(None)
    }

    fn apply_put_down(&mut self, actor: EntityId, target: EntityId) -> Result<(), Discrepancy> {
        let actor_ref = PlanningEntityRef::Authoritative(actor);
        let lot_ref = PlanningEntityRef::Authoritative(target);
        if self.state.direct_possessor_ref(lot_ref) != Some(actor_ref) {
            return Err(self.missing_observation_for(actor, target));
        }
        let place = self
            .state
            .effective_place_ref(actor_ref)
            .ok_or(Discrepancy::MissingObservation)?;
        let commodity = self
            .state
            .item_lot_commodity_ref(lot_ref)
            .ok_or_else(|| self.missing_observation_for(actor, target))?;
        let quantity = self.state.commodity_quantity_ref(lot_ref, commodity);
        self.update_state(|state| {
            state.move_lot_ref_to_ground(lot_ref, place, commodity, quantity)
        });
        Ok(())
    }

    fn apply_steal(&mut self, actor: EntityId, target: EntityId) -> Result<(), Discrepancy> {
        let actor_ref = PlanningEntityRef::Authoritative(actor);
        let lot_ref = PlanningEntityRef::Authoritative(target);
        let commodity = self
            .state
            .item_lot_commodity_ref(lot_ref)
            .ok_or_else(|| self.missing_observation_for(actor, target))?;
        let quantity = self.state.commodity_quantity_ref(lot_ref, commodity);
        if quantity == Quantity(0) {
            return Err(Discrepancy::MissingObservation);
        }
        self.update_state(|state| {
            state.move_lot_ref_to_holder(lot_ref, actor_ref, commodity, quantity)
        });
        Ok(())
    }

    fn display_container_for_lot(
        &self,
        actor: EntityId,
        lot: PlanningEntityRef,
    ) -> Option<PlanningEntityRef> {
        let actor_ref = PlanningEntityRef::Authoritative(actor);
        let place = self.state.effective_place_ref(actor_ref)?;
        let container = self.state.direct_container_ref(lot)?;
        let container = match container {
            PlanningEntityRef::Authoritative(entity) => entity,
            PlanningEntityRef::Hypothetical(_) => return None,
        };
        self.state
            .snapshot()
            .entities
            .iter()
            .find_map(|(facility, snapshot)| {
                let policy = snapshot.facility.stock_storage_policy.as_ref()?;
                (snapshot.spatial.effective_place == Some(place)
                    && policy.stock_container == container
                    && self
                        .state
                        .can_control_ref(actor_ref, PlanningEntityRef::Authoritative(*facility)))
                .then(|| {
                    policy
                        .display_container
                        .map(PlanningEntityRef::Authoritative)
                })
                .flatten()
            })
    }

    fn stock_container_for_lot(
        &self,
        actor: EntityId,
        lot: PlanningEntityRef,
    ) -> Option<PlanningEntityRef> {
        let actor_ref = PlanningEntityRef::Authoritative(actor);
        let place = self.state.effective_place_ref(actor_ref)?;
        let container = self.state.direct_container_ref(lot)?;
        let container = match container {
            PlanningEntityRef::Authoritative(entity) => entity,
            PlanningEntityRef::Hypothetical(_) => return None,
        };
        self.state
            .snapshot()
            .entities
            .iter()
            .find_map(|(facility, snapshot)| {
                let policy = snapshot.facility.stock_storage_policy.as_ref()?;
                (snapshot.spatial.effective_place == Some(place)
                    && policy.display_container == Some(container)
                    && self
                        .state
                        .can_control_ref(actor_ref, PlanningEntityRef::Authoritative(*facility)))
                .then_some(PlanningEntityRef::Authoritative(policy.stock_container))
            })
    }

    fn controls_facility_for_lot_container(&self, actor: EntityId, lot: PlanningEntityRef) -> bool {
        let actor_ref = PlanningEntityRef::Authoritative(actor);
        let Some(place) = self.state.effective_place_ref(actor_ref) else {
            return false;
        };
        let Some(container) = self.state.direct_container_ref(lot) else {
            return false;
        };
        let container = match container {
            PlanningEntityRef::Authoritative(entity) => entity,
            PlanningEntityRef::Hypothetical(_) => return false,
        };
        self.state
            .snapshot()
            .entities
            .iter()
            .any(|(facility, snapshot)| {
                let Some(policy) = snapshot.facility.stock_storage_policy.as_ref() else {
                    return false;
                };
                snapshot.spatial.effective_place == Some(place)
                    && (policy.stock_container == container
                        || policy.display_container == Some(container))
                    && self
                        .state
                        .can_control_ref(actor_ref, PlanningEntityRef::Authoritative(*facility))
            })
    }
}

impl EffectSink for HypotheticalEffectSink<'_> {
    fn check_precondition(
        &self,
        precondition: &EffectPrecondition,
        actor_entity: EntityId,
        targets: &[EntityId],
    ) -> Result<(), Discrepancy> {
        match precondition {
            EffectPrecondition::TargetMatchesSlot {
                slot_index,
                shape: TargetSpec::SpecificEntity(expected),
            } => targets
                .get(*slot_index)
                .is_some_and(|target| target == expected),
            EffectPrecondition::TargetMatchesSlot { .. }
            | EffectPrecondition::CapacityFloor { .. }
            | EffectPrecondition::BeliefHeld { .. } => true,
            EffectPrecondition::CoLocated { actor, target } => {
                let actor = resolve_entity_ref(*actor, actor_entity, targets)?;
                let target = resolve_entity_ref(*target, actor_entity, targets)?;
                let colocated = self
                    .state
                    .effective_place_ref(PlanningEntityRef::Authoritative(actor))
                    == self
                        .state
                        .effective_place_ref(PlanningEntityRef::Authoritative(target));
                if !colocated {
                    return Err(self.missing_observation_for(actor_entity, target));
                }
                true
            }
            EffectPrecondition::QuantityAvailable {
                source,
                commodity,
                min,
            } => {
                let source = resolve_entity_ref(*source, actor_entity, targets)?;
                if !self.quantity_available(source, *commodity, *min) {
                    return Err(self.missing_observation_for(actor_entity, source));
                }
                true
            }
            EffectPrecondition::ContentionGrantHeld { actor, affordance } => {
                let actor = resolve_entity_ref(*actor, actor_entity, targets)?;
                let affordance = resolve_entity_ref(*affordance, actor_entity, targets)?;
                self.state
                    .facility_grant(affordance)
                    .is_some_and(|grant| grant.actor == actor)
            }
        }
        .then_some(())
        .ok_or(Discrepancy::MissingObservation)
    }

    fn checkpoint(&mut self) -> usize {
        let id = self.checkpoints.len();
        self.checkpoints.push(HypotheticalEffectSinkCheckpoint {
            state: self.state.clone(),
            emitted_events: self.emitted_events.clone(),
            fulfilled_expectations: self.fulfilled_expectations.clone(),
            expected_materializations: self.expected_materializations.clone(),
        });
        id
    }

    fn restore(&mut self, checkpoint: usize) -> Result<(), Discrepancy> {
        let checkpoint = self
            .checkpoints
            .get(checkpoint)
            .cloned()
            .ok_or(Discrepancy::ImproperPlanningState)?;
        self.state = checkpoint.state;
        self.emitted_events = checkpoint.emitted_events;
        self.fulfilled_expectations = checkpoint.fulfilled_expectations;
        self.expected_materializations = checkpoint.expected_materializations;
        Ok(())
    }

    fn write_transfer(
        &mut self,
        source: EntityId,
        dest: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Result<(), Discrepancy> {
        let source_ref = PlanningEntityRef::Authoritative(source);
        let dest_ref = PlanningEntityRef::Authoritative(dest);
        if self.state.commodity_quantity_ref(source_ref, commodity) < quantity {
            return Err(Discrepancy::PartialExecutionDrift);
        }

        self.update_state(|state| {
            let source_quantity = state.commodity_quantity_ref(source_ref, commodity);
            let dest_quantity = state.commodity_quantity_ref(dest_ref, commodity);
            state
                .set_quantity_ref(
                    source_ref,
                    commodity,
                    source_quantity
                        .checked_sub(quantity)
                        .expect("quantity availability checked before transfer"),
                )
                .set_quantity_ref(dest_ref, commodity, dest_quantity + quantity)
        });
        Ok(())
    }

    fn write_consume(
        &mut self,
        source: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Result<(), Discrepancy> {
        let source_ref = PlanningEntityRef::Authoritative(source);
        let current = self.state.commodity_quantity_ref(source_ref, commodity);
        let Some(next) = current.checked_sub(quantity) else {
            return Err(Discrepancy::PartialExecutionDrift);
        };

        self.update_state(|state| state.set_quantity_ref(source_ref, commodity, next));
        Ok(())
    }

    fn write_produce(
        &mut self,
        sink: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Result<(), Discrepancy> {
        let sink_ref = PlanningEntityRef::Authoritative(sink);
        let current = self.state.commodity_quantity_ref(sink_ref, commodity);
        self.update_state(|state| state.set_quantity_ref(sink_ref, commodity, current + quantity));
        Ok(())
    }

    fn write_wound(&mut self, _target: EntityId, _cause: WoundCause) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn write_event(&mut self, tag: EventTag) -> Result<(), Discrepancy> {
        self.emitted_events.push(tag);
        Ok(())
    }

    fn assert_expectation_fulfilled(
        &mut self,
        expectation: ExpectationId,
    ) -> Result<(), Discrepancy> {
        self.fulfilled_expectations.push(expectation);
        Ok(())
    }

    fn consume_grant(&mut self, grant: EntityId) -> Result<(), Discrepancy> {
        self.update_state(|state| state.simulate_grant_consumed(grant));
        Ok(())
    }

    fn set_combat_stance(
        &mut self,
        _entity: EntityId,
        _stance: worldwake_core::CombatStance,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn clear_combat_stance(&mut self, _entity: EntityId) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn clear_contention_membership(
        &mut self,
        _actor: EntityId,
        _entity: EntityId,
        _action: ActionDefId,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn resolve_combat_attack(
        &mut self,
        _attacker: EntityId,
        _target: EntityId,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn clear_entity_contention_if_no_wounds(
        &mut self,
        entity: EntityId,
    ) -> Result<(), Discrepancy> {
        self.update_state(|state| state.with_pain(entity, Permille::ZERO));
        Ok(())
    }

    fn consume_target_consumable(
        &mut self,
        actor: EntityId,
        target: EntityId,
        _effect: worldwake_sim::ConsumableEffect,
    ) -> Result<(), Discrepancy> {
        let commodity = self
            .state
            .item_lot_commodity_ref(PlanningEntityRef::Authoritative(target))
            .ok_or_else(|| self.missing_observation_for(actor, target))?;
        let _ = actor;
        self.update_state(|state| state.consume_commodity(commodity));
        Ok(())
    }

    fn end_sleep_episode(&mut self, actor: EntityId) -> Result<(), Discrepancy> {
        let Some(mut needs) = self.state.homeostatic_needs(actor) else {
            return Ok(());
        };
        let Some(profile) = self.state.metabolism_profile(actor) else {
            return Ok(());
        };
        needs.fatigue = needs.fatigue.saturating_sub(profile.rest_efficiency);
        self.update_state(|state| state.with_homeostatic_needs(actor, needs));
        Ok(())
    }

    fn use_toilet(&mut self, actor: EntityId) -> Result<(), Discrepancy> {
        self.clear_actor_need(actor, |needs| needs.bladder = Permille::ZERO);
        Ok(())
    }

    fn relieve_wilderness(&mut self, actor: EntityId) -> Result<(), Discrepancy> {
        self.clear_actor_need(actor, |needs| needs.bladder = Permille::ZERO);
        Ok(())
    }

    fn use_wash_basin(&mut self, actor: EntityId, _basin: EntityId) -> Result<(), Discrepancy> {
        self.clear_actor_need(actor, |needs| needs.dirtiness = Permille::ZERO);
        Ok(())
    }

    fn clean_wash_basin(&mut self, _actor: EntityId, basin: EntityId) -> Result<(), Discrepancy> {
        // S176 D6: simulate the basin's dirtiness reset so the search sees the
        // `TargetWashBasinNotTooDirty` gate unblock after the cleaning step.
        let Some(mut state) = FacilityBeliefView::wash_basin_state(&self.state, basin) else {
            return Ok(());
        };
        state.dirtiness_level = Permille::ZERO;
        let consumed = state
            .clean_water_units
            .min(state.units_per_full_wash.max(1));
        state.clean_water_units -= consumed;
        self.update_state(|s| s.with_wash_basin_state(basin, state));
        Ok(())
    }

    fn empty_latrine(&mut self, actor: EntityId) -> Result<(), Discrepancy> {
        // S176 D6: simulate the latrine fill reset so the search sees the
        // `PlaceLatrineNotFull` gate unblock after the emptying step.
        let Some(place) = self.state.effective_place(actor) else {
            return Ok(());
        };
        let Some(mut fullness) = FacilityBeliefView::latrine_fullness(&self.state, place) else {
            return Ok(());
        };
        fullness.fill = Permille::ZERO;
        self.update_state(|s| s.with_latrine_fullness(place, fullness));
        Ok(())
    }

    fn harvest_resource(
        &mut self,
        actor: EntityId,
        _workstation: EntityId,
        payload: &ActionPayload,
    ) -> Result<Vec<EffectFact>, Discrepancy> {
        let harvest = payload.as_harvest().ok_or(Discrepancy::NoLegalBinding)?;
        let actor_place = self.state.effective_place(actor);
        let commodity = harvest.output_commodity;
        let requested = harvest.requested_quantity;
        let snapshot = self.state.snapshot();
        let mut state = std::mem::replace(&mut self.state, PlanningState::new(snapshot));
        let hypothetical = state.spawn_hypothetical_lot(EntityKind::ItemLot, commodity);
        let hypothetical_ref = PlanningEntityRef::Hypothetical(hypothetical);
        state = state.set_quantity_ref(hypothetical_ref, commodity, requested);
        if let Some(place) = actor_place {
            state = state.move_entity_ref(hypothetical_ref, place);
        }
        self.state = state;
        Ok(Vec::new())
    }

    fn finish_craft(
        &mut self,
        _actor: EntityId,
        _workstation: EntityId,
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn store_stock(&mut self, actor: EntityId, lot: EntityId) -> Result<(), Discrepancy> {
        let actor_ref = PlanningEntityRef::Authoritative(actor);
        let actor_place = self
            .state
            .effective_place_ref(actor_ref)
            .ok_or(Discrepancy::MissingObservation)?;
        let stock_container = self
            .state
            .merchandise_profile(actor)
            .and_then(|profile| profile.home_facility)
            .filter(|facility| self.state.effective_place(*facility) == Some(actor_place))
            .and_then(|facility| {
                self.state
                    .can_control(actor, facility)
                    .then(|| self.state.stock_storage_policy(facility))
                    .flatten()
            })
            .map(|policy| PlanningEntityRef::Authoritative(policy.stock_container))
            .or_else(|| {
                self.state
                    .controlled_stock_containers_at_place(actor_ref, actor_place)
                    .into_iter()
                    .next()
            })
            .ok_or(Discrepancy::MissingObservation)?;
        let lot_ref = PlanningEntityRef::Authoritative(lot);
        self.update_state(|state| state.set_container_ref(lot_ref, stock_container));
        Ok(())
    }

    fn collect_display_stock(&mut self, actor: EntityId, lot: EntityId) -> Result<(), Discrepancy> {
        let actor_ref = PlanningEntityRef::Authoritative(actor);
        let lot_ref = PlanningEntityRef::Authoritative(lot);
        if !self.controls_facility_for_lot_container(actor, lot_ref) {
            return Err(self.missing_observation_for(actor, lot));
        }
        self.update_state(|state| {
            state
                .set_possessor_ref(lot_ref, actor_ref)
                .clear_sale_listing_ref(lot_ref)
        });
        Ok(())
    }

    fn stage_stock_for_sale(&mut self, actor: EntityId, lot: EntityId) -> Result<(), Discrepancy> {
        let lot_ref = PlanningEntityRef::Authoritative(lot);
        let display_container = self
            .display_container_for_lot(actor, lot_ref)
            .ok_or_else(|| self.missing_observation_for(actor, lot))?;
        self.update_state(|state| {
            state
                .set_container_ref(lot_ref, display_container)
                .set_sale_listing_ref(lot_ref, Some(actor))
        });
        Ok(())
    }

    fn unstage_stock(&mut self, actor: EntityId, lot: EntityId) -> Result<(), Discrepancy> {
        let lot_ref = PlanningEntityRef::Authoritative(lot);
        let stock_container = self
            .stock_container_for_lot(actor, lot_ref)
            .ok_or_else(|| self.missing_observation_for(actor, lot))?;
        self.update_state(|state| {
            state
                .set_container_ref(lot_ref, stock_container)
                .clear_sale_listing_ref(lot_ref)
        });
        Ok(())
    }

    fn pick_up(
        &mut self,
        actor: EntityId,
        target: EntityId,
        payload: &ActionPayload,
        _action_def_id: ActionDefId,
    ) -> Result<Option<Materialization>, Discrepancy> {
        self.apply_pick_up(actor, target, payload)
    }

    fn put_down(&mut self, actor: EntityId, target: EntityId) -> Result<(), Discrepancy> {
        self.apply_put_down(actor, target)
    }

    fn drop_item(&mut self, actor: EntityId, target: EntityId) -> Result<(), Discrepancy> {
        self.apply_put_down(actor, target)
    }

    fn steal(&mut self, actor: EntityId, target: EntityId) -> Result<(), Discrepancy> {
        self.apply_steal(actor, target)
    }

    fn complete_trade(
        &mut self,
        _actor: EntityId,
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn record_staff_market_demand(
        &mut self,
        _actor: EntityId,
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn complete_travel_to(
        &mut self,
        _actor: EntityId,
        destination: Option<EntityId>,
    ) -> Result<(), Discrepancy> {
        let destination = destination.ok_or(Discrepancy::NoLegalBinding)?;
        self.update_state(|state| state.move_actor_to(destination));
        Ok(())
    }

    fn enqueue_contention(
        &mut self,
        _actor: EntityId,
        entity: EntityId,
        intended_action: ActionDefId,
    ) -> Result<(), Discrepancy> {
        self.update_state(|state| state.simulate_queue_join(entity, intended_action));
        Ok(())
    }

    fn bury_corpse(&mut self, corpse: EntityId, burial_site: EntityId) -> Result<(), Discrepancy> {
        let corpse_ref = PlanningEntityRef::Authoritative(corpse);
        let burial_ref = PlanningEntityRef::Authoritative(burial_site);
        self.update_state(|state| state.set_container_ref(corpse_ref, burial_ref));
        Ok(())
    }

    fn loot_possessions_within_capacity(
        &mut self,
        looter: EntityId,
        corpse: EntityId,
    ) -> Result<(), Discrepancy> {
        self.update_state(|state| {
            CommodityKind::ALL
                .iter()
                .copied()
                .fold(state, |next, commodity| {
                    let quantity = next.commodity_quantity(corpse, commodity);
                    if quantity == Quantity(0) {
                        return next;
                    }
                    let looter_quantity = next.commodity_quantity(looter, commodity);
                    next.with_commodity_quantity(corpse, commodity, Quantity(0))
                        .with_commodity_quantity(looter, commodity, looter_quantity + quantity)
                })
        });
        Ok(())
    }

    fn complete_escort_to_safety(
        &mut self,
        _actor: EntityId,
        _target: EntityId,
        payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        let payload = payload
            .as_escort_to_safety()
            .ok_or(Discrepancy::NoLegalBinding)?;
        self.update_state(|state| state.move_actor_to(payload.destination));
        Ok(())
    }

    fn advance_patrol_route(
        &mut self,
        _actor: EntityId,
        _target: EntityId,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn establish_bandit_camp(
        &mut self,
        actor: EntityId,
        _target: EntityId,
        payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        let payload = payload
            .as_establish_camp()
            .ok_or(Discrepancy::NoLegalBinding)?;
        let Some(place) = self.state.effective_place(actor) else {
            return Ok(());
        };
        self.update_state(|state| state.with_bandit_camp_faction(place, Some(payload.faction)));
        Ok(())
    }

    fn declare_support(
        &mut self,
        actor: EntityId,
        payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        let payload = payload
            .as_declare_support()
            .ok_or(Discrepancy::NoLegalBinding)?;
        if !actor_at_office_seat(&self.state, actor, payload.office) {
            return Err(self.missing_observation_for(actor, payload.office));
        }
        self.update_state(|state| {
            state.with_support_declaration(actor, payload.office, payload.candidate)
        });
        Ok(())
    }

    fn commit_tell(
        &mut self,
        _actor: EntityId,
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn consult_record(
        &mut self,
        _actor: EntityId,
        payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        let Some(consult) = payload.as_consult_record() else {
            return Ok(());
        };
        let Some(record_data) = self.state.record_data(consult.record) else {
            return Ok(());
        };
        let offices = self
            .state
            .snapshot()
            .entities
            .keys()
            .copied()
            .filter(|entity| {
                self.state
                    .entity_kind_ref(PlanningEntityRef::Authoritative(*entity))
                    == Some(EntityKind::Office)
            })
            .collect::<Vec<_>>();
        self.update_state(|mut state| {
            for office in offices {
                let read = consulted_office_holder_read_for_record_data(&record_data, office);
                if read != InstitutionalBeliefRead::Unknown {
                    state.override_office_holder_belief(office, read);
                }
            }
            state
        });
        Ok(())
    }

    fn ask_about_person(
        &mut self,
        _actor: EntityId,
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn ask_witness(
        &mut self,
        _actor: EntityId,
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn search_place(
        &mut self,
        _actor: EntityId,
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn investigate(
        &mut self,
        _actor: EntityId,
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn report_missing(
        &mut self,
        _actor: EntityId,
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn report_found(
        &mut self,
        _actor: EntityId,
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn accuse(
        &mut self,
        _actor: EntityId,
        _targets: &[EntityId],
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn fine(
        &mut self,
        _actor: EntityId,
        _targets: &[EntityId],
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn exile(
        &mut self,
        _actor: EntityId,
        _targets: &[EntityId],
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn bribe(
        &mut self,
        actor: EntityId,
        _targets: &[EntityId],
        payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        let Some(bribe) = payload.as_bribe() else {
            return Ok(());
        };
        let current_qty = self
            .state
            .commodity_quantity(actor, bribe.offered_commodity);
        if current_qty < bribe.offered_quantity {
            return Ok(());
        }
        let Some(office) = local_office_for_actor(&self.state, actor) else {
            return Ok(());
        };
        if !office_vacancy_known(&self.state, office) {
            return Ok(());
        }
        let remaining = Quantity(current_qty.0.saturating_sub(bribe.offered_quantity.0));
        self.update_state(|state| {
            state
                .with_commodity_quantity(actor, bribe.offered_commodity, remaining)
                .with_support_declaration(bribe.target, office, actor)
        });
        Ok(())
    }

    fn threaten(
        &mut self,
        actor: EntityId,
        _targets: &[EntityId],
        payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        let Some(threaten) = payload.as_threaten() else {
            return Ok(());
        };
        let attack_skill = self
            .state
            .combat_profile(actor)
            .map_or(Permille::ZERO, |profile| profile.attack_skill);
        let target_courage = self
            .state
            .courage(threaten.target)
            .unwrap_or(Permille::new_unchecked(1000));
        if attack_skill <= target_courage {
            return Ok(());
        }
        let Some(office) = local_office_for_actor(&self.state, actor) else {
            return Ok(());
        };
        if !office_vacancy_known(&self.state, office) {
            return Ok(());
        }
        self.update_state(|state| state.with_support_declaration(threaten.target, office, actor));
        Ok(())
    }

    fn press_force_claim(
        &mut self,
        actor: EntityId,
        payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        let payload = payload
            .as_press_force_claim()
            .ok_or(Discrepancy::NoLegalBinding)?;
        if !actor_at_office_seat(&self.state, actor, payload.office) {
            return Err(self.missing_observation_for(actor, payload.office));
        }
        self.update_state(|mut state| {
            state.override_force_controller_belief(
                payload.office,
                InstitutionalBeliefRead::Certain((Some(actor), false)),
            );
            state
        });
        Ok(())
    }

    fn yield_force_claim(
        &mut self,
        _actor: EntityId,
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn post_bounty(
        &mut self,
        _actor: EntityId,
        _targets: &[EntityId],
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn post_notice(
        &mut self,
        _actor: EntityId,
        _targets: &[EntityId],
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn claim_bounty(
        &mut self,
        _actor: EntityId,
        _targets: &[EntityId],
        _action_def_id: ActionDefId,
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn withdraw_bounty(
        &mut self,
        _actor: EntityId,
        _targets: &[EntityId],
    ) -> Result<(), Discrepancy> {
        Ok(())
    }
}

fn resolve_entity_ref(
    entity_ref: EffectEntityRef,
    actor: EntityId,
    targets: &[EntityId],
) -> Result<EntityId, Discrepancy> {
    match entity_ref {
        EffectEntityRef::Actor => Ok(actor),
        EffectEntityRef::Target { index } => targets
            .get(index)
            .copied()
            .ok_or(Discrepancy::NoLegalBinding),
        EffectEntityRef::Entity(entity) => Ok(entity),
    }
}

fn local_office_for_actor(state: &PlanningState<'_>, actor: EntityId) -> Option<EntityId> {
    let actor_place = state.effective_place(actor)?;
    state.snapshot().entities.keys().copied().find(|office| {
        state.entity_kind_ref(PlanningEntityRef::Authoritative(*office)) == Some(EntityKind::Office)
            && state.snapshot().seat(*office) == Some(actor_place)
    })
}

fn office_vacancy_known(state: &PlanningState<'_>, office: EntityId) -> bool {
    state.believed_office_holder(office) == InstitutionalBeliefRead::Certain(None)
}

fn actor_at_office_seat(state: &PlanningState<'_>, actor: EntityId, office: EntityId) -> bool {
    state.effective_place(actor).is_some_and(|actor_place| {
        state
            .snapshot()
            .seat(office)
            .is_some_and(|seat| seat == actor_place)
    })
}

#[cfg(test)]
mod tests {
    use super::HypotheticalEffectSink;
    use crate::{PlanningSnapshot, PlanningState};
    use std::collections::VecDeque;
    use worldwake_core::{
        ActionDefId, AgentBeliefStore, Discrepancy, EntityId, ObservationOmission,
        ObservationOmissionLog, OmissionReason, Tick,
    };
    use worldwake_sim::{ActionPayload, EffectSink};

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn sink_with_store(
        actor: EntityId,
        store: AgentBeliefStore,
        present_entities: &[EntityId],
    ) -> HypotheticalEffectSink<'static> {
        let snapshot = Box::leak(Box::new(PlanningSnapshot::for_effect_sink_test(
            actor,
            store,
            present_entities,
        )));
        HypotheticalEffectSink::new(PlanningState::new(snapshot))
    }

    #[test]
    fn pick_up_omitted_target_returns_omission_discrepancy() {
        let actor = entity(1);
        let omitted = entity(2);
        let reason = OmissionReason::OverBudget {
            budget: 5,
            candidates_seen: 10,
        };
        let store = AgentBeliefStore {
            observation_omission_log: ObservationOmissionLog {
                entries: VecDeque::from([ObservationOmission {
                    omitted_entity: omitted,
                    reason,
                    observed_tick: Tick(7),
                }]),
            },
            ..AgentBeliefStore::default()
        };
        let mut sink = sink_with_store(actor, store, &[]);

        assert_eq!(
            sink.pick_up(actor, omitted, &ActionPayload::None, ActionDefId(1)),
            Err(Discrepancy::Omission(reason))
        );
    }

    #[test]
    fn pick_up_never_observed_target_returns_missing_observation() {
        let actor = entity(1);
        let target = entity(2);
        let mut sink = sink_with_store(actor, AgentBeliefStore::default(), &[]);

        assert_eq!(
            sink.pick_up(actor, target, &ActionPayload::None, ActionDefId(1)),
            Err(Discrepancy::MissingObservation)
        );
    }

    #[test]
    fn pick_up_non_snapshot_actor_returns_missing_observation() {
        let actor = entity(1);
        let snapshot_actor = entity(2);
        let omitted = entity(3);
        let store = AgentBeliefStore {
            observation_omission_log: ObservationOmissionLog {
                entries: VecDeque::from([ObservationOmission {
                    omitted_entity: omitted,
                    reason: OmissionReason::OverBudget {
                        budget: 5,
                        candidates_seen: 10,
                    },
                    observed_tick: Tick(7),
                }]),
            },
            ..AgentBeliefStore::default()
        };
        let mut sink = sink_with_store(snapshot_actor, store, &[]);

        assert_eq!(
            sink.pick_up(actor, omitted, &ActionPayload::None, ActionDefId(1)),
            Err(Discrepancy::MissingObservation)
        );
    }

    #[test]
    fn pick_up_target_present_in_snapshot_returns_missing_observation() {
        let actor = entity(1);
        let target = entity(2);
        let store = AgentBeliefStore {
            observation_omission_log: ObservationOmissionLog {
                entries: VecDeque::from([ObservationOmission {
                    omitted_entity: target,
                    reason: OmissionReason::OverBudget {
                        budget: 5,
                        candidates_seen: 10,
                    },
                    observed_tick: Tick(7),
                }]),
            },
            ..AgentBeliefStore::default()
        };
        let mut sink = sink_with_store(actor, store, &[target]);

        assert_eq!(
            sink.pick_up(actor, target, &ActionPayload::None, ActionDefId(1)),
            Err(Discrepancy::MissingObservation)
        );
    }
}
