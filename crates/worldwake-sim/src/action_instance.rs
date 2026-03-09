use crate::{ActionDefId, ActionHandlerId, ActionInstanceId, ActionState, ActionStatus};
use serde::{Deserialize, Serialize};
use worldwake_core::{EntityId, ReservationId, Tick};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionInstance {
    pub instance_id: ActionInstanceId,
    pub def_id: ActionDefId,
    pub handler_id: ActionHandlerId,
    pub actor: EntityId,
    pub targets: Vec<EntityId>,
    pub start_tick: Tick,
    pub remaining_ticks: u32,
    pub status: ActionStatus,
    pub reservation_ids: Vec<ReservationId>,
    pub local_state: Option<ActionState>,
}

#[cfg(test)]
mod tests {
    use super::ActionInstance;
    use crate::{ActionDefId, ActionHandlerId, ActionInstanceId, ActionState, ActionStatus};
    use serde::{de::DeserializeOwned, Serialize};
    use worldwake_core::{EntityId, ReservationId, Tick};

    fn assert_traits<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    fn sample_instance(local_state: Option<ActionState>) -> ActionInstance {
        ActionInstance {
            instance_id: ActionInstanceId(3),
            def_id: ActionDefId(1),
            handler_id: ActionHandlerId(2),
            actor: EntityId {
                slot: 5,
                generation: 1,
            },
            targets: vec![
                EntityId {
                    slot: 7,
                    generation: 1,
                },
                EntityId {
                    slot: 8,
                    generation: 2,
                },
            ],
            start_tick: Tick(11),
            remaining_ticks: 4,
            status: ActionStatus::Active,
            reservation_ids: vec![ReservationId(13), ReservationId(21)],
            local_state,
        }
    }

    #[test]
    fn action_instance_satisfies_required_traits() {
        assert_traits::<ActionInstance>();
    }

    #[test]
    fn action_instance_roundtrips_with_some_local_state() {
        let instance = sample_instance(Some(ActionState::Empty));

        let bytes = bincode::serialize(&instance).unwrap();
        let roundtrip: ActionInstance = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, instance);
    }

    #[test]
    fn action_instance_roundtrips_with_no_local_state() {
        let instance = sample_instance(None);

        let bytes = bincode::serialize(&instance).unwrap();
        let roundtrip: ActionInstance = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, instance);
    }
}
