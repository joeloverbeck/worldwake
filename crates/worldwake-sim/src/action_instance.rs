use crate::{ActionDuration, ActionInstanceId, ActionPayload, ActionState, ActionStatus};
use serde::{Deserialize, Serialize};
use worldwake_core::{ActionDefId, EntityId, ReservationId, Tick};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionInstance {
    pub instance_id: ActionInstanceId,
    pub def_id: ActionDefId,
    pub payload: ActionPayload,
    pub actor: EntityId,
    pub targets: Vec<EntityId>,
    pub start_tick: Tick,
    pub remaining_duration: ActionDuration,
    pub status: ActionStatus,
    pub reservation_ids: Vec<ReservationId>,
    pub local_state: Option<ActionState>,
}

#[cfg(test)]
mod tests {
    use super::ActionInstance;
    use crate::{ActionDuration, ActionInstanceId, ActionPayload, ActionState, ActionStatus};
    use serde::{de::DeserializeOwned, Serialize};
    use worldwake_core::{ActionDefId, CommodityKind, EntityId, Quantity, ReservationId, Tick};

    fn assert_traits<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    fn sample_instance(local_state: Option<ActionState>) -> ActionInstance {
        ActionInstance {
            instance_id: ActionInstanceId(3),
            def_id: ActionDefId(1),
            payload: ActionPayload::Trade(crate::TradeActionPayload {
                counterparty: EntityId {
                    slot: 6,
                    generation: 1,
                },
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(2),
                requested_commodity: CommodityKind::Bread,
                requested_quantity: Quantity(1),
            }),
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
            remaining_duration: ActionDuration::Finite(4),
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
