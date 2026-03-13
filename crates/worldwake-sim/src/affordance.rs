use crate::{ActionDef, ActionPayload};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use worldwake_core::{ActionDefId, EntityId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Affordance {
    pub def_id: ActionDefId,
    pub actor: EntityId,
    pub bound_targets: Vec<EntityId>,
    pub payload_override: Option<ActionPayload>,
    pub explanation: Option<String>,
}

impl Affordance {
    #[must_use]
    pub fn effective_payload<'a>(&'a self, def: &'a ActionDef) -> &'a ActionPayload {
        self.payload_override.as_ref().unwrap_or(&def.payload)
    }

    #[must_use]
    pub fn matches_request_identity(
        &self,
        def: &ActionDef,
        actor: EntityId,
        targets: &[EntityId],
        payload_override: Option<&ActionPayload>,
    ) -> bool {
        self.actor == actor
            && self.def_id == def.id
            && self.bound_targets == targets
            && self.effective_payload(def) == payload_override.unwrap_or(&def.payload)
    }
}

impl Ord for Affordance {
    fn cmp(&self, other: &Self) -> Ordering {
        self.def_id
            .cmp(&other.def_id)
            .then_with(|| self.bound_targets.cmp(&other.bound_targets))
            .then_with(|| self.actor.cmp(&other.actor))
            .then_with(|| self.payload_override.cmp(&other.payload_override))
            .then_with(|| self.explanation.cmp(&other.explanation))
    }
}

impl PartialOrd for Affordance {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::Affordance;
    use crate::{
        ActionDef, ActionDomain, ActionHandlerId, ActionPayload, DurationExpr, Interruptibility,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use std::collections::BTreeSet;
    use std::num::NonZeroU32;
    use worldwake_core::EntityId;
    use worldwake_core::{ActionDefId, BodyCostPerTick, CommodityKind, Quantity, VisibilitySpec};

    fn assert_traits<T: Clone + Eq + Ord + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    #[test]
    fn affordance_satisfies_required_traits() {
        assert_traits::<Affordance>();
    }

    fn sample_action_def(id: ActionDefId, payload: ActionPayload) -> ActionDef {
        ActionDef {
            id,
            name: format!("action-{}", id.0),
            domain: ActionDomain::Generic,
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload,
            handler: ActionHandlerId(0),
        }
    }

    #[test]
    fn affordance_ordering_uses_def_id_then_bound_targets() {
        let actor = entity(99);
        let mut affordances = [
            Affordance {
                def_id: ActionDefId(2),
                actor,
                bound_targets: vec![entity(4)],
                payload_override: None,
                explanation: Some("later".to_string()),
            },
            Affordance {
                def_id: ActionDefId(1),
                actor,
                bound_targets: vec![entity(7)],
                payload_override: None,
                explanation: Some("human".to_string()),
            },
            Affordance {
                def_id: ActionDefId(1),
                actor,
                bound_targets: vec![entity(3)],
                payload_override: None,
                explanation: None,
            },
        ];

        affordances.sort();

        assert_eq!(affordances[0].def_id, ActionDefId(1));
        assert_eq!(affordances[0].bound_targets, vec![entity(3)]);
        assert_eq!(affordances[1].def_id, ActionDefId(1));
        assert_eq!(affordances[1].bound_targets, vec![entity(7)]);
        assert_eq!(affordances[2].def_id, ActionDefId(2));
    }

    #[test]
    fn matches_request_identity_requires_exact_actor_def_payload_and_ordered_targets() {
        let def = sample_action_def(
            ActionDefId(4),
            ActionPayload::Harvest(crate::HarvestActionPayload {
                recipe_id: worldwake_core::RecipeId(2),
                required_workstation_tag: worldwake_core::WorkstationTag::OrchardRow,
                output_commodity: CommodityKind::Apple,
                output_quantity: Quantity(1),
                required_tool_kinds: Vec::new(),
            }),
        );
        let affordance = Affordance {
            def_id: def.id,
            actor: entity(9),
            bound_targets: vec![entity(3), entity(5)],
            payload_override: None,
            explanation: None,
        };

        assert!(affordance.matches_request_identity(
            &def,
            entity(9),
            &[entity(3), entity(5)],
            None,
        ));
        assert!(affordance.matches_request_identity(
            &def,
            entity(9),
            &[entity(3), entity(5)],
            Some(&def.payload),
        ));
        assert!(!affordance.matches_request_identity(
            &def,
            entity(8),
            &[entity(3), entity(5)],
            None,
        ));
        assert!(!affordance.matches_request_identity(
            &sample_action_def(ActionDefId(7), def.payload.clone()),
            entity(9),
            &[entity(3), entity(5)],
            None,
        ));
        assert!(!affordance.matches_request_identity(
            &def,
            entity(9),
            &[entity(5), entity(3)],
            None,
        ));
        assert!(!affordance.matches_request_identity(
            &def,
            entity(9),
            &[entity(3), entity(5)],
            Some(&ActionPayload::None),
        ));
    }

    #[test]
    fn effective_payload_prefers_affordance_override_over_definition_default() {
        let def = sample_action_def(ActionDefId(1), ActionPayload::None);
        let affordance = Affordance {
            def_id: def.id,
            actor: entity(2),
            bound_targets: vec![entity(8)],
            payload_override: Some(ActionPayload::Loot(crate::LootActionPayload {
                target: entity(8),
            })),
            explanation: None,
        };

        assert_eq!(
            affordance.effective_payload(&def),
            &ActionPayload::Loot(crate::LootActionPayload { target: entity(8) })
        );
    }
}
