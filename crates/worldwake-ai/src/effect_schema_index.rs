use std::collections::{BTreeMap, BTreeSet};

use crate::PlannerOpKind;
use crate::opportunity_compiler::EffectFactKey;
use crate::planner_ops::classify_action_def;
use worldwake_sim::{ActionDefRegistry, EffectStep};

pub struct EffectSchemaIndex {
    pub by_effect: BTreeMap<EffectFactKey, Vec<worldwake_core::ActionDefId>>,
    pub by_effect_op: BTreeMap<EffectFactKey, BTreeSet<PlannerOpKind>>,
}

impl EffectSchemaIndex {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            by_effect: BTreeMap::new(),
            by_effect_op: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn build(registry: &ActionDefRegistry) -> Self {
        let mut by_effect: BTreeMap<EffectFactKey, Vec<worldwake_core::ActionDefId>> =
            BTreeMap::new();
        let mut by_effect_op: BTreeMap<EffectFactKey, BTreeSet<PlannerOpKind>> = BTreeMap::new();
        for action_def in registry.iter() {
            let op_kind = classify_action_def(action_def);
            for key in effect_keys_for_steps(&action_def.effect_schema.steps) {
                by_effect.entry(key).or_default().push(action_def.id);
                if let Some(op_kind) = op_kind {
                    by_effect_op.entry(key).or_default().insert(op_kind);
                }
            }
        }
        for ids in by_effect.values_mut() {
            ids.sort();
            ids.dedup();
        }
        Self {
            by_effect,
            by_effect_op,
        }
    }

    #[must_use]
    pub fn actions_producing(&self, fact: EffectFactKey) -> &[worldwake_core::ActionDefId] {
        self.by_effect
            .get(&fact)
            .map_or(&[], std::vec::Vec::as_slice)
    }

    #[must_use]
    pub fn planner_ops_producing(&self, fact: EffectFactKey) -> &BTreeSet<PlannerOpKind> {
        static EMPTY: BTreeSet<PlannerOpKind> = BTreeSet::new();
        self.by_effect_op.get(&fact).unwrap_or(&EMPTY)
    }
}

impl Default for EffectSchemaIndex {
    fn default() -> Self {
        Self::empty()
    }
}

fn effect_keys_for_steps(steps: &[EffectStep]) -> Vec<EffectFactKey> {
    let mut keys = Vec::new();
    for step in steps {
        collect_effect_keys(step, &mut keys);
    }
    keys.sort();
    keys.dedup();
    keys
}

fn collect_effect_keys(step: &EffectStep, keys: &mut Vec<EffectFactKey>) {
    match step {
        EffectStep::Transfer { .. }
        | EffectStep::LootPossessionsWithinCapacity { .. }
        | EffectStep::FinishCraft { .. }
        | EffectStep::CollectDisplayStock { .. }
        | EffectStep::PickUp { .. }
        | EffectStep::PutDown { .. }
        | EffectStep::DropItem { .. }
        | EffectStep::Steal { .. }
        | EffectStep::CompleteTrade => keys.push(EffectFactKey::CommodityTransfer),
        EffectStep::HarvestResource { .. } => {
            keys.push(EffectFactKey::CommodityTransfer);
            keys.push(EffectFactKey::PartialQuantity);
        }
        EffectStep::ApplyWound { .. } => keys.push(EffectFactKey::WoundApplied),
        EffectStep::EmitEvent { .. } => keys.push(EffectFactKey::EventEmitted),
        EffectStep::AssertExpectationFulfilled { .. } => {
            keys.push(EffectFactKey::ExpectationFulfilled);
        }
        EffectStep::ConsumeContentionGrant { .. } => {
            keys.push(EffectFactKey::ContentionGrantConsumed);
        }
        EffectStep::PartialOnFailure { primary, fallback } => {
            for nested in primary.iter().chain(fallback) {
                collect_effect_keys(nested, keys);
            }
        }
        EffectStep::Consume { .. }
        | EffectStep::Produce { .. }
        | EffectStep::SetCombatStance { .. }
        | EffectStep::ClearCombatStance { .. }
        | EffectStep::EnqueueContention { .. }
        | EffectStep::ClearContentionMembership { .. }
        | EffectStep::BuryCorpse { .. }
        | EffectStep::ResolveCombatAttack { .. }
        | EffectStep::ClearEntityContentionIfNoWounds { .. }
        | EffectStep::ConsumeTargetConsumable { .. }
        | EffectStep::EndSleepEpisode
        | EffectStep::UseToilet
        | EffectStep::RelieveWilderness
        | EffectStep::UseWashBasin { .. }
        | EffectStep::StoreStock { .. }
        | EffectStep::StageStockForSale { .. }
        | EffectStep::UnstageStock { .. }
        | EffectStep::RecordStaffMarketDemand
        | EffectStep::CompleteEscortToSafety
        | EffectStep::CompleteTravel
        | EffectStep::AdvancePatrolRoute
        | EffectStep::EstablishBanditCamp
        | EffectStep::CommitTell
        | EffectStep::ConsultRecord
        | EffectStep::AskAboutPerson
        | EffectStep::AskWitness
        | EffectStep::SearchPlace
        | EffectStep::Investigate
        | EffectStep::ReportMissing
        | EffectStep::ReportFound
        | EffectStep::Accuse
        | EffectStep::Fine
        | EffectStep::Exile
        | EffectStep::Bribe
        | EffectStep::Threaten
        | EffectStep::DeclareSupport
        | EffectStep::PressForceClaim
        | EffectStep::YieldForceClaim
        | EffectStep::PostBounty
        | EffectStep::PostNotice
        | EffectStep::ClaimBounty
        | EffectStep::WithdrawBounty => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::num::NonZeroU32;
    use worldwake_core::{
        ActionDefId, ActionDomain, BodyCostPerTick, CommodityKind, EntityId, EventTag, Quantity,
        RecipeId, VisibilitySpec, WorkstationTag,
    };
    use worldwake_sim::{
        ActionDef, ActionHandlerId, ActionPayload, BindingStrictness, Constraint, DurationExpr,
        EffectEntityRef, EffectSchema, HarvestActionPayload, Interruptibility, Precondition,
        ReservationReq, TargetSpec,
    };

    fn action_def(id: u32, name: &str, steps: Vec<EffectStep>) -> ActionDef {
        ActionDef {
            id: ActionDefId(id),
            name: name.to_string(),
            domain: ActionDomain::Generic,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: vec![TargetSpec::SpecificEntity(EntityId {
                slot: id + 10,
                generation: 1,
            })],
            preconditions: vec![Precondition::TargetExists(0)],
            reservation_requirements: vec![ReservationReq { target_index: 0 }],
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![],
            visibility: VisibilitySpec::ParticipantsOnly,
            causal_event_tags: BTreeSet::from([EventTag::ActionCommitted]),
            payload: ActionPayload::None,
            handler: ActionHandlerId(id),
            binding_strictness: BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: EffectSchema {
                preconditions: Vec::new(),
                steps,
            },
        }
    }

    fn registry_with_defs(defs: Vec<ActionDef>) -> ActionDefRegistry {
        let mut registry = ActionDefRegistry::new();
        for def in defs {
            registry.register(def);
        }
        registry
    }

    fn classified_action_def(
        id: u32,
        name: &str,
        domain: ActionDomain,
        payload: ActionPayload,
        steps: Vec<EffectStep>,
    ) -> ActionDef {
        let mut def = action_def(id, name, steps);
        def.domain = domain;
        def.payload = payload;
        def
    }

    #[test]
    fn build_maps_effect_keys_to_action_defs_and_empty_lookup_is_stable() {
        let registry = registry_with_defs(vec![
            action_def(
                0,
                "transfer",
                vec![EffectStep::Transfer {
                    source: EffectEntityRef::Actor,
                    dest: EffectEntityRef::Target { index: 0 },
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(1),
                }],
            ),
            action_def(
                1,
                "event",
                vec![EffectStep::EmitEvent {
                    tag: EventTag::ActionCommitted,
                }],
            ),
            action_def(2, "empty", Vec::new()),
        ]);

        let index = EffectSchemaIndex::build(&registry);

        assert_eq!(
            index.actions_producing(EffectFactKey::CommodityTransfer),
            &[ActionDefId(0)]
        );
        assert_eq!(
            index.actions_producing(EffectFactKey::EventEmitted),
            &[ActionDefId(1)]
        );
        assert!(
            index
                .actions_producing(EffectFactKey::WoundApplied)
                .is_empty()
        );
    }

    #[test]
    fn build_sorts_and_deduplicates_action_ids_per_effect_key() {
        let registry = registry_with_defs(vec![
            action_def(
                0,
                "duplicate",
                vec![
                    EffectStep::EmitEvent {
                        tag: EventTag::ActionStarted,
                    },
                    EffectStep::EmitEvent {
                        tag: EventTag::ActionCommitted,
                    },
                ],
            ),
            action_def(
                1,
                "nested",
                vec![EffectStep::PartialOnFailure {
                    primary: vec![EffectStep::EmitEvent {
                        tag: EventTag::Travel,
                    }],
                    fallback: vec![EffectStep::EmitEvent {
                        tag: EventTag::ActionAborted,
                    }],
                }],
            ),
        ]);

        let index = EffectSchemaIndex::build(&registry);

        assert_eq!(
            index.actions_producing(EffectFactKey::EventEmitted),
            &[ActionDefId(0), ActionDefId(1)]
        );
    }

    #[test]
    fn build_is_deterministic_for_same_registry() {
        let registry = registry_with_defs(vec![
            action_def(
                0,
                "wound",
                vec![EffectStep::ApplyWound {
                    target: EffectEntityRef::Target { index: 0 },
                    cause: worldwake_core::WoundCause::Deprivation(
                        worldwake_core::DeprivationKind::Starvation,
                    ),
                }],
            ),
            action_def(
                1,
                "grant",
                vec![EffectStep::ConsumeContentionGrant {
                    grant: EffectEntityRef::Target { index: 0 },
                }],
            ),
        ]);

        let first = EffectSchemaIndex::build(&registry);
        let second = EffectSchemaIndex::build(&registry);

        assert_eq!(first.by_effect, second.by_effect);
    }

    #[test]
    fn planner_ops_producing_returns_classified_set() {
        let registry = registry_with_defs(vec![
            classified_action_def(
                0,
                "pick_up",
                ActionDomain::Transport,
                ActionPayload::None,
                vec![EffectStep::PickUp {
                    target: EffectEntityRef::Target { index: 0 },
                }],
            ),
            classified_action_def(
                1,
                "harvest:bread",
                ActionDomain::Production,
                ActionPayload::Harvest(HarvestActionPayload {
                    recipe_id: RecipeId(1),
                    required_workstation_tag: WorkstationTag::FieldPlot,
                    output_commodity: CommodityKind::Bread,
                    requested_quantity: Quantity(1),
                    required_tool_kinds: Vec::new(),
                }),
                vec![EffectStep::HarvestResource {
                    workstation: EffectEntityRef::Target { index: 0 },
                }],
            ),
            classified_action_def(
                2,
                "trade",
                ActionDomain::Trade,
                ActionPayload::None,
                vec![EffectStep::CompleteTrade],
            ),
            classified_action_def(
                3,
                "transfer",
                ActionDomain::Generic,
                ActionPayload::None,
                vec![EffectStep::Transfer {
                    source: EffectEntityRef::Actor,
                    dest: EffectEntityRef::Target { index: 0 },
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(1),
                }],
            ),
        ]);

        let index = EffectSchemaIndex::build(&registry);
        let expected = BTreeSet::from([
            PlannerOpKind::Harvest,
            PlannerOpKind::Trade,
            PlannerOpKind::MoveCargo,
        ]);

        assert_eq!(
            index.planner_ops_producing(EffectFactKey::CommodityTransfer),
            &expected
        );
        assert!(
            index
                .planner_ops_producing(EffectFactKey::WoundApplied)
                .is_empty()
        );
    }
}
