use worldwake_core::{
    CommodityKind, Discrepancy, EntityId, EventTag, ExpectationId, Quantity, WorldTxn, WoundCause,
};
use worldwake_sim::{EffectPrecondition, EffectSink, TargetSpec};

pub struct AuthoritativeEffectSink<'txn, 'world> {
    txn: &'txn mut WorldTxn<'world>,
    fulfilled_expectations: Vec<ExpectationId>,
}

impl<'txn, 'world> AuthoritativeEffectSink<'txn, 'world> {
    pub fn new(txn: &'txn mut WorldTxn<'world>) -> Self {
        Self {
            txn,
            fulfilled_expectations: Vec::new(),
        }
    }

    #[must_use]
    pub fn fulfilled_expectations(&self) -> &[ExpectationId] {
        &self.fulfilled_expectations
    }

    fn transfer_from_controlled_lots(
        &mut self,
        source: EntityId,
        dest: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Result<(), Discrepancy> {
        let mut remaining = quantity;
        let lots = self.controlled_lots(source, commodity);

        for (lot_id, lot_quantity) in lots {
            if remaining == Quantity(0) {
                break;
            }

            if lot_quantity <= remaining {
                self.txn
                    .set_possessor(lot_id, dest)
                    .map_err(|_| Discrepancy::PartialExecutionDrift)?;
                remaining = remaining
                    .checked_sub(lot_quantity)
                    .expect("remaining should be at least the lot quantity");
            } else {
                let (_, split_lot) = self
                    .txn
                    .split_lot(lot_id, remaining)
                    .map_err(|_| Discrepancy::PartialExecutionDrift)?;
                self.txn
                    .set_possessor(split_lot, dest)
                    .map_err(|_| Discrepancy::PartialExecutionDrift)?;
                remaining = Quantity(0);
            }
        }

        (remaining == Quantity(0))
            .then_some(())
            .ok_or(Discrepancy::PartialExecutionDrift)
    }

    fn consume_from_controlled_lots(
        &mut self,
        source: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Result<(), Discrepancy> {
        let mut remaining = quantity;
        let lots = self.controlled_lots(source, commodity);

        for (lot_id, lot_quantity) in lots {
            if remaining == Quantity(0) {
                break;
            }

            if lot_quantity <= remaining {
                self.txn
                    .archive_entity(lot_id)
                    .map_err(|_| Discrepancy::PartialExecutionDrift)?;
                remaining = remaining
                    .checked_sub(lot_quantity)
                    .expect("remaining should be at least the lot quantity");
            } else {
                let (_, consumed_lot) = self
                    .txn
                    .split_lot(lot_id, remaining)
                    .map_err(|_| Discrepancy::PartialExecutionDrift)?;
                self.txn
                    .archive_entity(consumed_lot)
                    .map_err(|_| Discrepancy::PartialExecutionDrift)?;
                remaining = Quantity(0);
            }
        }

        (remaining == Quantity(0))
            .then_some(())
            .ok_or(Discrepancy::PartialExecutionDrift)
    }

    fn controlled_lots(
        &self,
        holder: EntityId,
        commodity: CommodityKind,
    ) -> Vec<(EntityId, Quantity)> {
        self.txn
            .query_item_lot()
            .filter(|(lot_id, lot)| {
                lot.commodity == commodity && self.txn.has_control(holder, *lot_id)
            })
            .map(|(lot_id, lot)| (lot_id, lot.quantity))
            .collect()
    }
}

impl EffectSink for AuthoritativeEffectSink<'_, '_> {
    fn check_precondition(
        &self,
        precondition: &EffectPrecondition,
        _actor: EntityId,
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
                self.txn.effective_place(*actor) == self.txn.effective_place(*target)
            }
            EffectPrecondition::QuantityAvailable {
                source,
                commodity,
                min,
            } => self.txn.controlled_commodity_quantity(*source, *commodity) >= *min,
            EffectPrecondition::ContentionGrantHeld { actor, affordance } => self
                .txn
                .get_component_contention_queue(*affordance)
                .and_then(|queue| queue.granted.as_ref())
                .is_some_and(|grant| grant.actor == *actor),
        };

        ok.then_some(()).ok_or(Discrepancy::MissingObservation)
    }

    fn checkpoint(&mut self) -> usize {
        0
    }

    fn restore(&mut self, _checkpoint: usize) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn write_transfer(
        &mut self,
        source: EntityId,
        dest: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Result<(), Discrepancy> {
        self.transfer_from_controlled_lots(source, dest, commodity, quantity)
    }

    fn write_consume(
        &mut self,
        source: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Result<(), Discrepancy> {
        self.consume_from_controlled_lots(source, commodity, quantity)
    }

    fn write_produce(
        &mut self,
        sink: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Result<(), Discrepancy> {
        let lot = self
            .txn
            .create_item_lot(commodity, quantity)
            .map_err(|_| Discrepancy::PartialExecutionDrift)?;
        self.txn
            .set_possessor(lot, sink)
            .map_err(|_| Discrepancy::PartialExecutionDrift)
    }

    fn write_wound(&mut self, _target: EntityId, _cause: WoundCause) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn write_event(&mut self, tag: EventTag) -> Result<(), Discrepancy> {
        self.txn.add_tag(tag);
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
        let mut queue = self
            .txn
            .get_component_contention_queue(grant)
            .cloned()
            .ok_or(Discrepancy::PartialExecutionDrift)?;
        queue.granted = None;
        self.txn
            .set_component_contention_queue(grant, queue)
            .map_err(|_| Discrepancy::PartialExecutionDrift)
    }
}

#[cfg(test)]
mod tests {
    use super::AuthoritativeEffectSink;
    use worldwake_core::{
        CauseRef, CommodityKind, EntityId, EventLog, EventTag, PrototypePlace, Quantity, Tick,
        VisibilitySpec, WitnessData, World, WorldTxn, build_prototype_world,
        prototype_place_entity,
    };
    use worldwake_sim::EffectSink;

    fn new_txn(world: &mut World) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        )
    }

    fn setup_world() -> (World, EntityId, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::VillageSquare);
        let mut txn = new_txn(&mut world);
        let source = txn
            .create_agent("source", worldwake_core::ControlSource::None)
            .unwrap();
        let dest = txn
            .create_agent("dest", worldwake_core::ControlSource::None)
            .unwrap();
        txn.set_ground_location(source, place).unwrap();
        txn.set_ground_location(dest, place).unwrap();
        let lot = txn
            .create_item_lot(CommodityKind::Bread, Quantity(3))
            .unwrap();
        txn.set_possessor(lot, source).unwrap();
        let mut log = EventLog::new();
        txn.commit(&mut log);
        (world, source, dest)
    }

    #[test]
    fn authoritative_sink_transfers_controlled_lot_quantities() {
        let (mut world, source, dest) = setup_world();
        let mut txn = new_txn(&mut world);
        {
            let mut sink = AuthoritativeEffectSink::new(&mut txn);
            sink.write_transfer(source, dest, CommodityKind::Bread, Quantity(2))
                .unwrap();
        }

        assert_eq!(
            txn.controlled_commodity_quantity(source, CommodityKind::Bread),
            Quantity(1)
        );
        assert_eq!(
            txn.controlled_commodity_quantity(dest, CommodityKind::Bread),
            Quantity(2)
        );
    }

    #[test]
    fn authoritative_sink_records_event_tags_on_txn() {
        let (mut world, _source, _dest) = setup_world();
        let mut txn = new_txn(&mut world);
        {
            let mut sink = AuthoritativeEffectSink::new(&mut txn);
            sink.write_event(EventTag::Transfer).unwrap();
        }

        assert!(txn.tags().contains(&EventTag::Transfer));
    }

    #[test]
    fn authoritative_sink_rejects_rollback_checkpoint_restore() {
        let (mut world, _source, _dest) = setup_world();
        let mut txn = new_txn(&mut world);
        let mut sink = AuthoritativeEffectSink::new(&mut txn);

        assert!(sink.restore(0).is_err());
    }
}
