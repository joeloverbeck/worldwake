use std::collections::BTreeMap;

use crate::opportunity_compiler::EffectFactKey;
use worldwake_sim::{ActionDefRegistry, EffectStep};

pub struct EffectSchemaIndex {
    pub by_effect: BTreeMap<EffectFactKey, Vec<worldwake_core::ActionDefId>>,
}

impl EffectSchemaIndex {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            by_effect: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn build(registry: &ActionDefRegistry) -> Self {
        let mut by_effect: BTreeMap<EffectFactKey, Vec<worldwake_core::ActionDefId>> =
            BTreeMap::new();
        for action_def in registry.iter() {
            for key in effect_keys_for_steps(&action_def.effect_schema.steps) {
                by_effect.entry(key).or_default().push(action_def.id);
            }
        }
        for ids in by_effect.values_mut() {
            ids.sort();
            ids.dedup();
        }
        Self { by_effect }
    }

    #[must_use]
    pub fn actions_producing(&self, fact: EffectFactKey) -> &[worldwake_core::ActionDefId] {
        self.by_effect
            .get(&fact)
            .map_or(&[], std::vec::Vec::as_slice)
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
        VisibilitySpec,
    };
    use worldwake_sim::{
        ActionDef, ActionHandlerId, ActionPayload, BindingStrictness, Constraint, DurationExpr,
        EffectEntityRef, EffectSchema, Interruptibility, Precondition, ReservationReq, TargetSpec,
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
}
