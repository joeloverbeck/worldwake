use serde::{Deserialize, Serialize};
use worldwake_core::{
    CommodityKind, EntityId, Quantity, Tick, TradeRole, TravelEdgeId, ViolationId,
};

#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Serialize, Deserialize,
)]
pub enum ActionState {
    #[default]
    Empty,
    Heal {
        medicine_spent: bool,
    },
    Investigate {
        violation_id: ViolationId,
        subject: EntityId,
        place: EntityId,
        commodity: Option<CommodityKind>,
    },
    Travel {
        edge_id: TravelEdgeId,
        origin: EntityId,
        destination: EntityId,
        departure_tick: Tick,
        arrival_tick: Tick,
    },
    Escort {
        subject: EntityId,
        destination: EntityId,
        leg_index: u16,
        departure_tick: Tick,
        arrival_tick: Tick,
    },
    Trade {
        round: u32,
        initiator_role: TradeRole,
        initiator_last_offer: Option<Quantity>,
        responder_last_offer: Option<Quantity>,
        agreed_price: Option<Quantity>,
    },
}

#[cfg(test)]
mod tests {
    use super::ActionState;
    use serde::{Serialize, de::DeserializeOwned};
    use worldwake_core::{EntityId, Quantity, Tick, TradeRole, TravelEdgeId, ViolationId};

    fn assert_traits<
        T: Copy
            + Clone
            + Eq
            + Ord
            + std::hash::Hash
            + std::fmt::Debug
            + Default
            + Serialize
            + DeserializeOwned,
    >() {
    }

    #[test]
    fn action_state_satisfies_required_traits() {
        assert_traits::<ActionState>();
    }

    #[test]
    fn action_state_default_is_empty() {
        assert_eq!(ActionState::default(), ActionState::Empty);
    }

    #[test]
    fn action_state_bincode_roundtrip_covers_every_variant() {
        for state in [
            ActionState::Empty,
            ActionState::Heal {
                medicine_spent: true,
            },
            ActionState::Investigate {
                violation_id: ViolationId(6),
                subject: EntityId {
                    slot: 3,
                    generation: 0,
                },
                place: EntityId {
                    slot: 4,
                    generation: 0,
                },
                commodity: Some(worldwake_core::CommodityKind::Apple),
            },
            ActionState::Travel {
                edge_id: TravelEdgeId(5),
                origin: EntityId {
                    slot: 1,
                    generation: 0,
                },
                destination: EntityId {
                    slot: 2,
                    generation: 0,
                },
                departure_tick: Tick(7),
                arrival_tick: Tick(10),
            },
            ActionState::Escort {
                subject: EntityId {
                    slot: 9,
                    generation: 0,
                },
                destination: EntityId {
                    slot: 10,
                    generation: 0,
                },
                leg_index: 2,
                departure_tick: Tick(11),
                arrival_tick: Tick(15),
            },
            ActionState::Trade {
                round: 2,
                initiator_role: TradeRole::Buyer,
                initiator_last_offer: Some(Quantity(3)),
                responder_last_offer: Some(Quantity(5)),
                agreed_price: Some(Quantity(4)),
            },
        ] {
            let bytes = bincode::serialize(&state).unwrap();
            let roundtrip: ActionState = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, state);
        }
    }
}
