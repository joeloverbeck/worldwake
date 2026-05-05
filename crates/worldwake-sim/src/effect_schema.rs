use crate::{ActionPayload, ConsumableEffect, Materialization, TargetSpec};
use serde::{Deserialize, Serialize};
use worldwake_core::{
    ActionDefId, BeliefClaimKey, CombatStance, CommodityKind, Discrepancy, EntityId, EventTag,
    ExpectationId, Quantity, WoundCause,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EffectSchema {
    pub preconditions: Vec<EffectPrecondition>,
    pub steps: Vec<EffectStep>,
}

impl EffectSchema {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            preconditions: Vec::new(),
            steps: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EffectMode {
    Authoritative,
    Hypothetical,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EffectPrecondition {
    TargetMatchesSlot {
        slot_index: usize,
        shape: TargetSpec,
    },
    CoLocated {
        actor: EffectEntityRef,
        target: EffectEntityRef,
    },
    QuantityAvailable {
        source: EffectEntityRef,
        commodity: CommodityKind,
        min: Quantity,
    },
    CapacityFloor {
        container: EntityId,
        min_free: Quantity,
    },
    ContentionGrantHeld {
        actor: EffectEntityRef,
        affordance: EffectEntityRef,
    },
    BeliefHeld {
        agent: EffectEntityRef,
        claim: BeliefClaimKey,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EffectEntityRef {
    Actor,
    Target { index: usize },
    Entity(EntityId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EffectActionRef {
    CurrentAction,
    PayloadQueueIntendedAction,
    Action(ActionDefId),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EffectStep {
    Transfer {
        source: EffectEntityRef,
        dest: EffectEntityRef,
        commodity: CommodityKind,
        quantity: Quantity,
    },
    Consume {
        source: EffectEntityRef,
        commodity: CommodityKind,
        quantity: Quantity,
    },
    Produce {
        sink: EffectEntityRef,
        commodity: CommodityKind,
        quantity: Quantity,
    },
    ApplyWound {
        target: EffectEntityRef,
        cause: WoundCause,
    },
    EmitEvent {
        tag: EventTag,
    },
    AssertExpectationFulfilled {
        expectation: ExpectationId,
    },
    ConsumeContentionGrant {
        grant: EffectEntityRef,
    },
    SetCombatStance {
        entity: EffectEntityRef,
        stance: CombatStance,
    },
    ClearCombatStance {
        entity: EffectEntityRef,
    },
    EnqueueContention {
        actor: EffectEntityRef,
        entity: EffectEntityRef,
        intended_action: EffectActionRef,
    },
    ClearContentionMembership {
        actor: EffectEntityRef,
        entity: EffectEntityRef,
        action: EffectActionRef,
    },
    LootPossessionsWithinCapacity {
        looter: EffectEntityRef,
        corpse: EffectEntityRef,
    },
    BuryCorpse {
        corpse: EffectEntityRef,
        burial_site: EffectEntityRef,
    },
    ResolveCombatAttack {
        attacker: EffectEntityRef,
        target: EffectEntityRef,
    },
    ClearEntityContentionIfNoWounds {
        entity: EffectEntityRef,
    },
    ConsumeTargetConsumable {
        target: EffectEntityRef,
        effect: ConsumableEffect,
    },
    EndSleepEpisode,
    UseToilet,
    RelieveWilderness,
    UseWashBasin {
        basin: EffectEntityRef,
    },
    HarvestResource {
        workstation: EffectEntityRef,
    },
    FinishCraft {
        workstation: EffectEntityRef,
    },
    StoreStock {
        lot: EffectEntityRef,
    },
    CollectDisplayStock {
        lot: EffectEntityRef,
    },
    StageStockForSale {
        lot: EffectEntityRef,
    },
    UnstageStock {
        lot: EffectEntityRef,
    },
    PickUp {
        target: EffectEntityRef,
    },
    PutDown {
        target: EffectEntityRef,
    },
    DropItem {
        target: EffectEntityRef,
    },
    Steal {
        target: EffectEntityRef,
    },
    CompleteTrade,
    RecordStaffMarketDemand,
    CompleteEscortToSafety,
    PartialOnFailure {
        primary: Vec<EffectStep>,
        fallback: Vec<EffectStep>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EffectFact {
    CommodityTransfer {
        source: EntityId,
        dest: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    },
    PartialQuantity {
        requested: Quantity,
        delivered: Quantity,
    },
    WoundApplied {
        target: EntityId,
        cause: WoundCause,
    },
    ExpectationFulfilled {
        expectation: ExpectationId,
    },
    ContentionGrantConsumed {
        grant: EntityId,
    },
    EventEmitted {
        tag: EventTag,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EffectOutcome {
    pub facts: Vec<EffectFact>,
}

pub trait EffectSink {
    fn check_precondition(
        &self,
        precondition: &EffectPrecondition,
        actor: EntityId,
        targets: &[EntityId],
    ) -> Result<(), Discrepancy>;

    fn checkpoint(&mut self) -> usize;

    fn restore(&mut self, checkpoint: usize) -> Result<(), Discrepancy>;

    fn write_transfer(
        &mut self,
        source: EntityId,
        dest: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Result<(), Discrepancy>;

    fn write_consume(
        &mut self,
        source: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Result<(), Discrepancy>;

    fn write_produce(
        &mut self,
        sink: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Result<(), Discrepancy>;

    fn write_wound(&mut self, target: EntityId, cause: WoundCause) -> Result<(), Discrepancy>;

    fn write_event(&mut self, tag: EventTag) -> Result<(), Discrepancy>;

    fn assert_expectation_fulfilled(
        &mut self,
        expectation: ExpectationId,
    ) -> Result<(), Discrepancy>;

    fn consume_grant(&mut self, grant: EntityId) -> Result<(), Discrepancy>;

    fn set_combat_stance(
        &mut self,
        _entity: EntityId,
        _stance: CombatStance,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn clear_combat_stance(&mut self, _entity: EntityId) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn enqueue_contention(
        &mut self,
        _actor: EntityId,
        _entity: EntityId,
        _intended_action: ActionDefId,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn clear_contention_membership(
        &mut self,
        _actor: EntityId,
        _entity: EntityId,
        _action: ActionDefId,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn loot_possessions_within_capacity(
        &mut self,
        _looter: EntityId,
        _corpse: EntityId,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn bury_corpse(
        &mut self,
        _corpse: EntityId,
        _burial_site: EntityId,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn resolve_combat_attack(
        &mut self,
        _attacker: EntityId,
        _target: EntityId,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn clear_entity_contention_if_no_wounds(
        &mut self,
        _entity: EntityId,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn consume_target_consumable(
        &mut self,
        _actor: EntityId,
        _target: EntityId,
        _effect: ConsumableEffect,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn end_sleep_episode(&mut self, _actor: EntityId) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn use_toilet(&mut self, _actor: EntityId) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn relieve_wilderness(&mut self, _actor: EntityId) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn use_wash_basin(&mut self, _actor: EntityId, _basin: EntityId) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn harvest_resource(
        &mut self,
        _actor: EntityId,
        _workstation: EntityId,
        _payload: &ActionPayload,
    ) -> Result<Vec<EffectFact>, Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn finish_craft(
        &mut self,
        _actor: EntityId,
        _workstation: EntityId,
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn store_stock(&mut self, _actor: EntityId, _lot: EntityId) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn collect_display_stock(
        &mut self,
        _actor: EntityId,
        _lot: EntityId,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn stage_stock_for_sale(
        &mut self,
        _actor: EntityId,
        _lot: EntityId,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn unstage_stock(&mut self, _actor: EntityId, _lot: EntityId) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn pick_up(
        &mut self,
        _actor: EntityId,
        _target: EntityId,
        _payload: &ActionPayload,
        _action_def_id: ActionDefId,
    ) -> Result<Option<Materialization>, Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn put_down(&mut self, _actor: EntityId, _target: EntityId) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn drop_item(&mut self, _actor: EntityId, _target: EntityId) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn steal(&mut self, _actor: EntityId, _target: EntityId) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn complete_trade(
        &mut self,
        _actor: EntityId,
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn record_staff_market_demand(
        &mut self,
        _actor: EntityId,
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn complete_escort_to_safety(
        &mut self,
        _actor: EntityId,
        _target: EntityId,
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }
}

#[derive(Clone, Copy)]
pub struct EffectEvaluationContext<'a> {
    pub actor: EntityId,
    pub targets: &'a [EntityId],
    pub payload: &'a ActionPayload,
    pub action_def_id: ActionDefId,
}

pub fn apply_effects(
    schema: &EffectSchema,
    actor: EntityId,
    targets: &[EntityId],
    sink: &mut dyn EffectSink,
    mode: EffectMode,
) -> Result<EffectOutcome, Discrepancy> {
    apply_effects_with_context(
        schema,
        EffectEvaluationContext {
            actor,
            targets,
            payload: &ActionPayload::None,
            action_def_id: ActionDefId(0),
        },
        sink,
        mode,
    )
}

pub fn apply_effects_with_context(
    schema: &EffectSchema,
    context: EffectEvaluationContext<'_>,
    sink: &mut dyn EffectSink,
    mode: EffectMode,
) -> Result<EffectOutcome, Discrepancy> {
    match mode {
        EffectMode::Authoritative | EffectMode::Hypothetical => {}
    }

    for precondition in &schema.preconditions {
        sink.check_precondition(precondition, context.actor, context.targets)?;
    }

    let mut facts = Vec::new();
    apply_steps(&schema.steps, &context, sink, &mut facts)?;
    Ok(EffectOutcome { facts })
}

fn apply_steps(
    steps: &[EffectStep],
    context: &EffectEvaluationContext<'_>,
    sink: &mut dyn EffectSink,
    facts: &mut Vec<EffectFact>,
) -> Result<(), Discrepancy> {
    for step in steps {
        apply_step(step, context, sink, facts)?;
    }
    Ok(())
}

fn apply_step(
    step: &EffectStep,
    context: &EffectEvaluationContext<'_>,
    sink: &mut dyn EffectSink,
    facts: &mut Vec<EffectFact>,
) -> Result<(), Discrepancy> {
    match step {
        EffectStep::Transfer {
            source,
            dest,
            commodity,
            quantity,
        } => {
            let source = resolve_entity_ref(*source, context)?;
            let dest = resolve_entity_ref(*dest, context)?;
            sink.write_transfer(source, dest, *commodity, *quantity)?;
            facts.push(EffectFact::CommodityTransfer {
                source,
                dest,
                commodity: *commodity,
                quantity: *quantity,
            });
        }
        EffectStep::Consume {
            source,
            commodity,
            quantity,
        } => {
            let source = resolve_entity_ref(*source, context)?;
            sink.write_consume(source, *commodity, *quantity)?;
        }
        EffectStep::Produce {
            sink: sink_entity,
            commodity,
            quantity,
        } => {
            let sink_entity = resolve_entity_ref(*sink_entity, context)?;
            sink.write_produce(sink_entity, *commodity, *quantity)?;
        }
        EffectStep::ApplyWound { target, cause } => {
            let target = resolve_entity_ref(*target, context)?;
            sink.write_wound(target, *cause)?;
            facts.push(EffectFact::WoundApplied {
                target,
                cause: *cause,
            });
        }
        EffectStep::EmitEvent { tag } => {
            sink.write_event(*tag)?;
            facts.push(EffectFact::EventEmitted { tag: *tag });
        }
        EffectStep::AssertExpectationFulfilled { expectation } => {
            sink.assert_expectation_fulfilled(*expectation)?;
            facts.push(EffectFact::ExpectationFulfilled {
                expectation: *expectation,
            });
        }
        EffectStep::ConsumeContentionGrant { grant } => {
            let grant = resolve_entity_ref(*grant, context)?;
            sink.consume_grant(grant)?;
            facts.push(EffectFact::ContentionGrantConsumed { grant });
        }
        EffectStep::SetCombatStance { entity, stance } => {
            let entity = resolve_entity_ref(*entity, context)?;
            sink.set_combat_stance(entity, *stance)?;
        }
        EffectStep::ClearCombatStance { entity } => {
            let entity = resolve_entity_ref(*entity, context)?;
            sink.clear_combat_stance(entity)?;
        }
        EffectStep::EnqueueContention {
            actor,
            entity,
            intended_action,
        } => {
            let actor = resolve_entity_ref(*actor, context)?;
            let entity = resolve_entity_ref(*entity, context)?;
            let intended_action = resolve_action_ref(*intended_action, context)?;
            sink.enqueue_contention(actor, entity, intended_action)?;
        }
        EffectStep::ClearContentionMembership {
            actor,
            entity,
            action,
        } => {
            let actor = resolve_entity_ref(*actor, context)?;
            let entity = resolve_entity_ref(*entity, context)?;
            let action = resolve_action_ref(*action, context)?;
            sink.clear_contention_membership(actor, entity, action)?;
        }
        EffectStep::LootPossessionsWithinCapacity { looter, corpse } => {
            let looter = resolve_entity_ref(*looter, context)?;
            let corpse = resolve_entity_ref(*corpse, context)?;
            sink.loot_possessions_within_capacity(looter, corpse)?;
        }
        EffectStep::BuryCorpse {
            corpse,
            burial_site,
        } => {
            let corpse = resolve_entity_ref(*corpse, context)?;
            let burial_site = resolve_entity_ref(*burial_site, context)?;
            sink.bury_corpse(corpse, burial_site)?;
        }
        EffectStep::ResolveCombatAttack { attacker, target } => {
            let attacker = resolve_entity_ref(*attacker, context)?;
            let target = resolve_entity_ref(*target, context)?;
            sink.resolve_combat_attack(attacker, target)?;
        }
        EffectStep::ClearEntityContentionIfNoWounds { entity } => {
            let entity = resolve_entity_ref(*entity, context)?;
            sink.clear_entity_contention_if_no_wounds(entity)?;
        }
        EffectStep::ConsumeTargetConsumable { target, effect } => {
            let target = resolve_entity_ref(*target, context)?;
            sink.consume_target_consumable(context.actor, target, *effect)?;
        }
        EffectStep::EndSleepEpisode => {
            sink.end_sleep_episode(context.actor)?;
        }
        EffectStep::UseToilet => {
            sink.use_toilet(context.actor)?;
        }
        EffectStep::RelieveWilderness => {
            sink.relieve_wilderness(context.actor)?;
        }
        EffectStep::UseWashBasin { basin } => {
            let basin = resolve_entity_ref(*basin, context)?;
            sink.use_wash_basin(context.actor, basin)?;
        }
        EffectStep::HarvestResource { workstation } => {
            let workstation = resolve_entity_ref(*workstation, context)?;
            facts.extend(sink.harvest_resource(context.actor, workstation, context.payload)?);
        }
        EffectStep::FinishCraft { workstation } => {
            let workstation = resolve_entity_ref(*workstation, context)?;
            sink.finish_craft(context.actor, workstation, context.payload)?;
        }
        EffectStep::StoreStock { lot } => {
            let lot = resolve_entity_ref(*lot, context)?;
            sink.store_stock(context.actor, lot)?;
        }
        EffectStep::CollectDisplayStock { lot } => {
            let lot = resolve_entity_ref(*lot, context)?;
            sink.collect_display_stock(context.actor, lot)?;
        }
        EffectStep::StageStockForSale { lot } => {
            let lot = resolve_entity_ref(*lot, context)?;
            sink.stage_stock_for_sale(context.actor, lot)?;
        }
        EffectStep::UnstageStock { lot } => {
            let lot = resolve_entity_ref(*lot, context)?;
            sink.unstage_stock(context.actor, lot)?;
        }
        EffectStep::PickUp { target } => {
            let target = resolve_entity_ref(*target, context)?;
            let _materialization = sink.pick_up(
                context.actor,
                target,
                context.payload,
                context.action_def_id,
            )?;
        }
        EffectStep::PutDown { target } => {
            let target = resolve_entity_ref(*target, context)?;
            sink.put_down(context.actor, target)?;
        }
        EffectStep::DropItem { target } => {
            let target = resolve_entity_ref(*target, context)?;
            sink.drop_item(context.actor, target)?;
        }
        EffectStep::Steal { target } => {
            let target = resolve_entity_ref(*target, context)?;
            sink.steal(context.actor, target)?;
        }
        EffectStep::CompleteTrade => {
            sink.complete_trade(context.actor, context.payload)?;
        }
        EffectStep::RecordStaffMarketDemand => {
            sink.record_staff_market_demand(context.actor, context.payload)?;
        }
        EffectStep::CompleteEscortToSafety => {
            let target = resolve_entity_ref(EffectEntityRef::Target { index: 0 }, context)?;
            sink.complete_escort_to_safety(context.actor, target, context.payload)?;
        }
        EffectStep::PartialOnFailure { primary, fallback } => {
            let checkpoint = sink.checkpoint();
            let fact_checkpoint = facts.len();
            if apply_steps(primary, context, sink, facts).is_err() {
                facts.truncate(fact_checkpoint);
                sink.restore(checkpoint)?;
                apply_steps(fallback, context, sink, facts)?;
            }
        }
    }
    Ok(())
}

fn resolve_entity_ref(
    entity_ref: EffectEntityRef,
    context: &EffectEvaluationContext<'_>,
) -> Result<EntityId, Discrepancy> {
    match entity_ref {
        EffectEntityRef::Actor => Ok(context.actor),
        EffectEntityRef::Target { index } => context
            .targets
            .get(index)
            .copied()
            .ok_or(Discrepancy::NoLegalBinding),
        EffectEntityRef::Entity(entity) => Ok(entity),
    }
}

fn resolve_action_ref(
    action_ref: EffectActionRef,
    context: &EffectEvaluationContext<'_>,
) -> Result<ActionDefId, Discrepancy> {
    match action_ref {
        EffectActionRef::CurrentAction => Ok(context.action_def_id),
        EffectActionRef::PayloadQueueIntendedAction => context
            .payload
            .as_queue_for_facility_use()
            .map(|payload| payload.intended_action)
            .ok_or(Discrepancy::NoLegalBinding),
        EffectActionRef::Action(action) => Ok(action),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EffectEntityRef, EffectFact, EffectMode, EffectOutcome, EffectPrecondition, EffectSchema,
        EffectSink, EffectStep, apply_effects,
    };
    use worldwake_core::{CommodityKind, EntityId, EventTag, ExpectationId, Quantity, WoundCause};

    #[derive(Clone, Default)]
    struct NoopSink {
        calls: Vec<&'static str>,
        fail_next_consume: bool,
        checkpoints: Vec<(Vec<&'static str>, bool)>,
    }

    impl EffectSink for NoopSink {
        fn check_precondition(
            &self,
            _precondition: &EffectPrecondition,
            _actor: EntityId,
            _targets: &[EntityId],
        ) -> Result<(), worldwake_core::Discrepancy> {
            Ok(())
        }

        fn checkpoint(&mut self) -> usize {
            let id = self.checkpoints.len();
            self.checkpoints
                .push((self.calls.clone(), self.fail_next_consume));
            id
        }

        fn restore(&mut self, checkpoint: usize) -> Result<(), worldwake_core::Discrepancy> {
            let (calls, fail_next_consume) = self
                .checkpoints
                .get(checkpoint)
                .cloned()
                .ok_or(worldwake_core::Discrepancy::ImproperPlanningState)?;
            self.calls = calls;
            self.fail_next_consume = fail_next_consume;
            Ok(())
        }

        fn write_transfer(
            &mut self,
            _source: EntityId,
            _dest: EntityId,
            _commodity: CommodityKind,
            _quantity: Quantity,
        ) -> Result<(), worldwake_core::Discrepancy> {
            self.calls.push("transfer");
            Ok(())
        }

        fn write_consume(
            &mut self,
            _source: EntityId,
            _commodity: CommodityKind,
            _quantity: Quantity,
        ) -> Result<(), worldwake_core::Discrepancy> {
            self.calls.push("consume");
            if self.fail_next_consume {
                self.fail_next_consume = false;
                return Err(worldwake_core::Discrepancy::PartialExecutionDrift);
            }
            Ok(())
        }

        fn write_produce(
            &mut self,
            _sink: EntityId,
            _commodity: CommodityKind,
            _quantity: Quantity,
        ) -> Result<(), worldwake_core::Discrepancy> {
            self.calls.push("produce");
            Ok(())
        }

        fn write_wound(
            &mut self,
            _target: EntityId,
            _cause: WoundCause,
        ) -> Result<(), worldwake_core::Discrepancy> {
            self.calls.push("wound");
            Ok(())
        }

        fn write_event(&mut self, _tag: EventTag) -> Result<(), worldwake_core::Discrepancy> {
            self.calls.push("event");
            Ok(())
        }

        fn assert_expectation_fulfilled(
            &mut self,
            _expectation: ExpectationId,
        ) -> Result<(), worldwake_core::Discrepancy> {
            self.calls.push("expectation");
            Ok(())
        }

        fn consume_grant(&mut self, _grant: EntityId) -> Result<(), worldwake_core::Discrepancy> {
            self.calls.push("grant");
            Ok(())
        }
    }

    #[test]
    fn empty_schema_has_no_preconditions_or_steps() {
        let schema = EffectSchema::empty();

        assert!(schema.preconditions.is_empty());
        assert!(schema.steps.is_empty());
    }

    #[test]
    fn empty_schema_roundtrips_through_bincode() {
        let schema = EffectSchema::empty();

        let bytes = bincode::serialize(&schema).unwrap();
        let roundtrip: EffectSchema = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, schema);
    }

    #[test]
    fn apply_effects_returns_noop_outcome_for_empty_schema() {
        let schema = EffectSchema::empty();
        let mut sink = NoopSink::default();
        let actor = EntityId {
            slot: 1,
            generation: 0,
        };

        let outcome =
            apply_effects(&schema, actor, &[], &mut sink, EffectMode::Hypothetical).unwrap();

        assert_eq!(outcome, EffectOutcome { facts: Vec::new() });
    }

    #[test]
    fn apply_effects_dispatches_steps_and_returns_facts() {
        let actor = EntityId {
            slot: 1,
            generation: 0,
        };
        let target = EntityId {
            slot: 2,
            generation: 0,
        };
        let schema = EffectSchema {
            preconditions: Vec::new(),
            steps: vec![
                EffectStep::Transfer {
                    source: EffectEntityRef::Actor,
                    dest: EffectEntityRef::Target { index: 0 },
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(3),
                },
                EffectStep::EmitEvent {
                    tag: EventTag::Transfer,
                },
                EffectStep::AssertExpectationFulfilled {
                    expectation: ExpectationId(7),
                },
                EffectStep::ConsumeContentionGrant {
                    grant: EffectEntityRef::Target { index: 0 },
                },
            ],
        };
        let mut sink = NoopSink::default();

        let outcome = apply_effects(
            &schema,
            actor,
            &[target],
            &mut sink,
            EffectMode::Authoritative,
        )
        .unwrap();

        assert_eq!(
            sink.calls,
            vec!["transfer", "event", "expectation", "grant"]
        );
        assert_eq!(
            outcome.facts,
            vec![
                EffectFact::CommodityTransfer {
                    source: actor,
                    dest: target,
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(3),
                },
                EffectFact::EventEmitted {
                    tag: EventTag::Transfer,
                },
                EffectFact::ExpectationFulfilled {
                    expectation: ExpectationId(7),
                },
                EffectFact::ContentionGrantConsumed { grant: target },
            ]
        );
    }

    #[test]
    fn partial_on_failure_restores_sink_and_runs_fallback() {
        let actor = EntityId {
            slot: 1,
            generation: 0,
        };
        let schema = EffectSchema {
            preconditions: Vec::new(),
            steps: vec![EffectStep::PartialOnFailure {
                primary: vec![
                    EffectStep::Produce {
                        sink: EffectEntityRef::Actor,
                        commodity: CommodityKind::Bread,
                        quantity: Quantity(1),
                    },
                    EffectStep::Consume {
                        source: EffectEntityRef::Actor,
                        commodity: CommodityKind::Bread,
                        quantity: Quantity(2),
                    },
                ],
                fallback: vec![EffectStep::EmitEvent {
                    tag: EventTag::ExpectationMismatch,
                }],
            }],
        };
        let mut sink = NoopSink {
            calls: Vec::new(),
            fail_next_consume: true,
            checkpoints: Vec::new(),
        };

        let outcome =
            apply_effects(&schema, actor, &[], &mut sink, EffectMode::Authoritative).unwrap();

        assert_eq!(sink.calls, vec!["event"]);
        assert_eq!(
            outcome.facts,
            vec![EffectFact::EventEmitted {
                tag: EventTag::ExpectationMismatch,
            }]
        );
    }

    #[test]
    fn authoritative_and_hypothetical_modes_share_interpretation() {
        let actor = EntityId {
            slot: 1,
            generation: 0,
        };
        let target = EntityId {
            slot: 2,
            generation: 0,
        };
        let schema = EffectSchema {
            preconditions: Vec::new(),
            steps: vec![
                EffectStep::Transfer {
                    source: EffectEntityRef::Actor,
                    dest: EffectEntityRef::Target { index: 0 },
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(1),
                },
                EffectStep::EmitEvent {
                    tag: EventTag::Transfer,
                },
            ],
        };
        let mut authoritative_sink = NoopSink::default();
        let mut hypothetical_sink = NoopSink::default();

        let authoritative = apply_effects(
            &schema,
            actor,
            &[target],
            &mut authoritative_sink,
            EffectMode::Authoritative,
        )
        .unwrap();
        let hypothetical = apply_effects(
            &schema,
            actor,
            &[target],
            &mut hypothetical_sink,
            EffectMode::Hypothetical,
        )
        .unwrap();

        assert_eq!(authoritative, hypothetical);
        assert_eq!(authoritative_sink.calls, hypothetical_sink.calls);
    }
}
