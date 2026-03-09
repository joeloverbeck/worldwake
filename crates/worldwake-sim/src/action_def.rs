use crate::{
    ActionDefId, ActionHandlerId, Constraint, DurationExpr, Interruptibility, Precondition,
    ReservationReq, TargetSpec,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use worldwake_core::{EventTag, VisibilitySpec};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionDef {
    pub id: ActionDefId,
    pub name: String,
    pub actor_constraints: Vec<Constraint>,
    pub targets: Vec<TargetSpec>,
    pub preconditions: Vec<Precondition>,
    pub reservation_requirements: Vec<ReservationReq>,
    pub duration: DurationExpr,
    pub interruptibility: Interruptibility,
    pub commit_conditions: Vec<Precondition>,
    pub visibility: VisibilitySpec,
    pub causal_event_tags: BTreeSet<EventTag>,
    pub handler: ActionHandlerId,
}

#[cfg(test)]
mod tests {
    use super::ActionDef;
    use crate::{
        ActionDefId, ActionHandlerId, Constraint, DurationExpr, Interruptibility, Precondition,
        ReservationReq, TargetSpec,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use std::collections::BTreeSet;
    use worldwake_core::{CommodityKind, EntityId, EntityKind, EventTag, Quantity, VisibilitySpec};

    fn assert_traits<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    fn sample_action_def(id: ActionDefId) -> ActionDef {
        ActionDef {
            id,
            name: format!("action-{}", id.0),
            actor_constraints: vec![
                Constraint::ActorAlive,
                Constraint::ActorHasCommodity {
                    kind: CommodityKind::Bread,
                    min_qty: Quantity(2),
                },
            ],
            targets: vec![
                TargetSpec::SpecificEntity(EntityId {
                    slot: 4,
                    generation: 1,
                }),
                TargetSpec::EntityAtActorPlace {
                    kind: EntityKind::Facility,
                },
            ],
            preconditions: vec![
                Precondition::ActorAlive,
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(1),
            ],
            reservation_requirements: vec![ReservationReq { target_index: 0 }],
            duration: DurationExpr::Fixed(3),
            interruptibility: Interruptibility::InterruptibleWithPenalty,
            commit_conditions: vec![Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Facility,
            }],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::from([EventTag::ActionCommitted, EventTag::Travel]),
            handler: ActionHandlerId(7),
        }
    }

    #[test]
    fn action_def_satisfies_required_traits() {
        assert_traits::<ActionDef>();
    }

    #[test]
    fn action_def_requires_all_expected_fields_with_concrete_non_optional_semantics() {
        let action_def = sample_action_def(ActionDefId(2));

        let ActionDef {
            id,
            name,
            actor_constraints,
            targets,
            preconditions,
            reservation_requirements,
            duration,
            interruptibility,
            commit_conditions,
            visibility,
            causal_event_tags,
            handler,
        } = action_def;

        let _: ActionDefId = id;
        let _: String = name;
        let _: Vec<Constraint> = actor_constraints;
        let _: Vec<TargetSpec> = targets;
        let _: Vec<Precondition> = preconditions;
        let _: Vec<ReservationReq> = reservation_requirements;
        let _: DurationExpr = duration;
        let _: Interruptibility = interruptibility;
        let _: Vec<Precondition> = commit_conditions;
        let _: VisibilitySpec = visibility;
        let _: BTreeSet<EventTag> = causal_event_tags;
        let _: ActionHandlerId = handler;
    }

    #[test]
    fn action_def_roundtrips_through_bincode() {
        let action_def = sample_action_def(ActionDefId(5));

        let bytes = bincode::serialize(&action_def).unwrap();
        let roundtrip: ActionDef = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, action_def);
    }
}
