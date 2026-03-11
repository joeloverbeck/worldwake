use crate::{ActionDefId, ActionInstanceId, ActionPayload};
use serde::{Deserialize, Serialize};
use worldwake_core::{EntityId, Tick};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum InputKind {
    RequestAction {
        actor: EntityId,
        def_id: ActionDefId,
        targets: Vec<EntityId>,
        payload_override: Option<ActionPayload>,
    },
    CancelAction {
        actor: EntityId,
        action_instance_id: ActionInstanceId,
    },
    SwitchControl {
        from: Option<EntityId>,
        to: Option<EntityId>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct InputEvent {
    pub scheduled_tick: Tick,
    pub sequence_no: u64,
    pub kind: InputKind,
}

#[cfg(test)]
mod tests {
    use super::{InputEvent, InputKind};
    use crate::{ActionDefId, ActionInstanceId, ActionPayload, TradeActionPayload};
    use worldwake_core::{CommodityKind, Quantity};
    use serde::{de::DeserializeOwned, Serialize};
    use worldwake_core::{EntityId, Tick};

    fn assert_traits<T: Clone + Eq + Ord + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    #[test]
    fn input_kind_satisfies_required_traits() {
        assert_traits::<InputKind>();
    }

    #[test]
    fn input_event_satisfies_required_traits() {
        assert_traits::<InputEvent>();
    }

    #[test]
    fn input_event_ordering_is_tick_then_sequence() {
        let early = InputEvent {
            scheduled_tick: Tick(3),
            sequence_no: 0,
            kind: InputKind::SwitchControl {
                from: None,
                to: Some(entity(1)),
            },
        };
        let same_tick_later = InputEvent {
            scheduled_tick: Tick(3),
            sequence_no: 1,
            kind: InputKind::RequestAction {
                actor: entity(2),
                def_id: ActionDefId(7),
                targets: vec![entity(3)],
                payload_override: None,
            },
        };
        let later_tick = InputEvent {
            scheduled_tick: Tick(5),
            sequence_no: 0,
            kind: InputKind::CancelAction {
                actor: entity(4),
                action_instance_id: ActionInstanceId(9),
            },
        };

        assert!(early < same_tick_later);
        assert!(same_tick_later < later_tick);
    }

    #[test]
    fn request_action_roundtrips_through_bincode() {
        let kind = InputKind::RequestAction {
            actor: entity(7),
            def_id: ActionDefId(3),
            targets: vec![entity(8), entity(9)],
            payload_override: Some(ActionPayload::Trade(TradeActionPayload {
                counterparty: entity(8),
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(2),
                requested_commodity: CommodityKind::Bread,
                requested_quantity: Quantity(1),
            })),
        };

        let bytes = bincode::serialize(&kind).unwrap();
        let roundtrip: InputKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, kind);
    }

    #[test]
    fn cancel_action_roundtrips_through_bincode() {
        let kind = InputKind::CancelAction {
            actor: entity(7),
            action_instance_id: ActionInstanceId(12),
        };

        let bytes = bincode::serialize(&kind).unwrap();
        let roundtrip: InputKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, kind);
    }

    #[test]
    fn switch_control_roundtrips_through_bincode() {
        let kind = InputKind::SwitchControl {
            from: Some(entity(1)),
            to: Some(entity(2)),
        };

        let bytes = bincode::serialize(&kind).unwrap();
        let roundtrip: InputKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, kind);
    }

    #[test]
    fn input_event_roundtrips_through_bincode() {
        let event = InputEvent {
            scheduled_tick: Tick(9),
            sequence_no: 4,
            kind: InputKind::RequestAction {
                actor: entity(5),
                def_id: ActionDefId(2),
                targets: vec![entity(6)],
                payload_override: None,
            },
        };

        let bytes = bincode::serialize(&event).unwrap();
        let roundtrip: InputEvent = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, event);
    }
}
