use crate::planning_state::{PlanningEntityRef, PlanningState};
use worldwake_core::{
    CommodityKind, Discrepancy, EntityId, EventTag, ExpectationId, Quantity, WoundCause,
};
use worldwake_sim::{
    EffectEntityRef, EffectPrecondition, EffectSink, TargetSpec, TemporalBeliefView,
};

pub struct HypotheticalEffectSink<'snapshot> {
    state: PlanningState<'snapshot>,
    emitted_events: Vec<EventTag>,
    fulfilled_expectations: Vec<ExpectationId>,
    checkpoints: Vec<HypotheticalEffectSinkCheckpoint<'snapshot>>,
}

#[derive(Clone)]
struct HypotheticalEffectSinkCheckpoint<'snapshot> {
    state: PlanningState<'snapshot>,
    emitted_events: Vec<EventTag>,
    fulfilled_expectations: Vec<ExpectationId>,
}

impl<'snapshot> HypotheticalEffectSink<'snapshot> {
    #[must_use]
    pub fn new(state: PlanningState<'snapshot>) -> Self {
        Self {
            state,
            emitted_events: Vec::new(),
            fulfilled_expectations: Vec::new(),
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
}

impl EffectSink for HypotheticalEffectSink<'_> {
    fn check_precondition(
        &self,
        precondition: &EffectPrecondition,
        actor_entity: EntityId,
        targets: &[EntityId],
    ) -> Result<(), Discrepancy> {
        let ok = match precondition {
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
                self.state
                    .effective_place_ref(PlanningEntityRef::Authoritative(actor))
                    == self
                        .state
                        .effective_place_ref(PlanningEntityRef::Authoritative(target))
            }
            EffectPrecondition::QuantityAvailable {
                source,
                commodity,
                min,
            } => {
                let source = resolve_entity_ref(*source, actor_entity, targets)?;
                self.quantity_available(source, *commodity, *min)
            }
            EffectPrecondition::ContentionGrantHeld { actor, affordance } => {
                let actor = resolve_entity_ref(*actor, actor_entity, targets)?;
                let affordance = resolve_entity_ref(*affordance, actor_entity, targets)?;
                self.state
                    .facility_grant(affordance)
                    .is_some_and(|grant| grant.actor == actor)
            }
        };

        ok.then_some(()).ok_or(Discrepancy::MissingObservation)
    }

    fn checkpoint(&mut self) -> usize {
        let id = self.checkpoints.len();
        self.checkpoints.push(HypotheticalEffectSinkCheckpoint {
            state: self.state.clone(),
            emitted_events: self.emitted_events.clone(),
            fulfilled_expectations: self.fulfilled_expectations.clone(),
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
        Err(Discrepancy::ImproperPlanningState)
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
