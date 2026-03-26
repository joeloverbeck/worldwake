use serde::{Deserialize, Serialize};
use worldwake_core::{CommodityKind, EntityId, Tick, TravelEdgeId, ViolationId};

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
}

#[cfg(test)]
mod tests {
    use super::ActionState;
    use serde::{de::DeserializeOwned, Serialize};
    use worldwake_core::{EntityId, Tick, TravelEdgeId, ViolationId};

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
        ] {
            let bytes = bincode::serialize(&state).unwrap();
            let roundtrip: ActionState = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, state);
        }
    }
}
