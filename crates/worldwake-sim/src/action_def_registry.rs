use crate::{ActionDef, ActionDefId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionDefRegistry {
    defs: Vec<ActionDef>,
}

impl ActionDefRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, def: ActionDef) -> ActionDefId {
        let expected = ActionDefId(self.defs.len() as u32);
        assert_eq!(
            def.id, expected,
            "action def id {:?} does not match expected sequential id {:?}",
            def.id, expected
        );
        self.defs.push(def);
        expected
    }

    #[must_use]
    pub fn get(&self, id: ActionDefId) -> Option<&ActionDef> {
        self.defs.get(id.0 as usize)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ActionDef> {
        self.defs.iter()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.defs.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::ActionDefRegistry;
    use crate::{
        ActionDef, ActionDefId, ActionHandlerId, Constraint, DurationExpr, Interruptibility,
        Precondition, ReservationReq, TargetSpec,
    };
    use std::collections::BTreeSet;
    use std::num::NonZeroU32;
    use worldwake_core::{
        BodyCostPerTick, CommodityKind, EntityId, EntityKind, EventTag, Quantity, VisibilitySpec,
    };

    fn sample_action_def(id: ActionDefId, name: &str) -> ActionDef {
        ActionDef {
            id,
            name: name.to_string(),
            actor_constraints: vec![
                Constraint::ActorAlive,
                Constraint::ActorHasCommodity {
                    kind: CommodityKind::Water,
                    min_qty: Quantity(1),
                },
            ],
            targets: vec![TargetSpec::SpecificEntity(EntityId {
                slot: id.0 + 10,
                generation: 1,
            })],
            preconditions: vec![Precondition::TargetExists(0)],
            reservation_requirements: vec![ReservationReq { target_index: 0 }],
            duration: DurationExpr::Fixed(NonZeroU32::new(id.0 + 1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Agent,
            }],
            visibility: VisibilitySpec::ParticipantsOnly,
            causal_event_tags: BTreeSet::from([EventTag::ActionStarted, EventTag::ActionCommitted]),
            handler: ActionHandlerId(id.0),
        }
    }

    #[test]
    fn registry_starts_empty() {
        let registry = ActionDefRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn register_validates_embedded_ids_and_get_returns_defs() {
        let mut registry = ActionDefRegistry::new();
        let first = sample_action_def(ActionDefId(0), "first");
        let second = sample_action_def(ActionDefId(1), "second");

        let first_id = registry.register(first.clone());
        let second_id = registry.register(second.clone());

        assert_eq!(first_id, ActionDefId(0));
        assert_eq!(second_id, ActionDefId(1));
        assert_eq!(registry.get(first_id), Some(&first));
        assert_eq!(registry.get(second_id), Some(&second));
        assert!(registry.get(ActionDefId(2)).is_none());
    }

    #[test]
    fn iter_returns_defs_in_registration_order() {
        let mut registry = ActionDefRegistry::new();
        registry.register(sample_action_def(ActionDefId(0), "first"));
        registry.register(sample_action_def(ActionDefId(1), "second"));
        registry.register(sample_action_def(ActionDefId(2), "third"));

        let names = registry
            .iter()
            .map(|def| def.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["first", "second", "third"]);
    }

    #[test]
    fn registry_roundtrips_through_bincode() {
        let mut registry = ActionDefRegistry::new();
        registry.register(sample_action_def(ActionDefId(0), "first"));
        registry.register(sample_action_def(ActionDefId(1), "second"));

        let bytes = bincode::serialize(&registry).unwrap();
        let roundtrip: ActionDefRegistry = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, registry);
    }

    #[test]
    fn registry_preserves_body_cost_metadata() {
        let mut registry = ActionDefRegistry::new();
        let mut def = sample_action_def(ActionDefId(0), "work");
        def.body_cost_per_tick = BodyCostPerTick::new(
            worldwake_core::Permille::new(1).unwrap(),
            worldwake_core::Permille::new(4).unwrap(),
            worldwake_core::Permille::new(7).unwrap(),
            worldwake_core::Permille::new(2).unwrap(),
        );
        let id = registry.register(def.clone());

        assert_eq!(
            registry.get(id).unwrap().body_cost_per_tick,
            def.body_cost_per_tick
        );
    }

    #[test]
    #[should_panic(expected = "does not match expected sequential id")]
    fn register_panics_when_embedded_id_skips_sequence() {
        let mut registry = ActionDefRegistry::new();
        registry.register(sample_action_def(ActionDefId(1), "invalid"));
    }
}
