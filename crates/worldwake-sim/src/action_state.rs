use serde::{Deserialize, Serialize};

#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Serialize, Deserialize,
)]
pub enum ActionState {
    #[default]
    Empty,
}

#[cfg(test)]
mod tests {
    use super::ActionState;
    use serde::{de::DeserializeOwned, Serialize};

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
        let bytes = bincode::serialize(&ActionState::Empty).unwrap();
        let roundtrip: ActionState = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, ActionState::Empty);
    }
}
