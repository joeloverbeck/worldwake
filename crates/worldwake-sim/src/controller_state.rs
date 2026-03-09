use serde::{Deserialize, Serialize};
use worldwake_core::EntityId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ControllerState {
    controlled_entity: Option<EntityId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlError {
    MismatchedFrom {
        expected: Option<EntityId>,
        actual: Option<EntityId>,
    },
}

impl ControllerState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            controlled_entity: None,
        }
    }

    #[must_use]
    pub const fn with_entity(entity: EntityId) -> Self {
        Self {
            controlled_entity: Some(entity),
        }
    }

    #[must_use]
    pub const fn controlled_entity(&self) -> Option<EntityId> {
        self.controlled_entity
    }

    pub fn switch_control(
        &mut self,
        from: Option<EntityId>,
        to: Option<EntityId>,
    ) -> Result<(), ControlError> {
        if self.controlled_entity != from {
            return Err(ControlError::MismatchedFrom {
                expected: from,
                actual: self.controlled_entity,
            });
        }

        self.controlled_entity = to;
        Ok(())
    }

    pub fn clear(&mut self) {
        self.controlled_entity = None;
    }
}

impl Default for ControllerState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{ControlError, ControllerState};
    use serde::{de::DeserializeOwned, Serialize};
    use worldwake_core::EntityId;

    fn assert_traits<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    const fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    #[test]
    fn controller_state_satisfies_required_traits() {
        assert_traits::<ControllerState>();
    }

    #[test]
    fn new_starts_without_controlled_entity() {
        let state = ControllerState::new();

        assert_eq!(state.controlled_entity(), None);
    }

    #[test]
    fn with_entity_starts_with_given_controlled_entity() {
        let controlled = entity(4);
        let state = ControllerState::with_entity(controlled);

        assert_eq!(state.controlled_entity(), Some(controlled));
    }

    #[test]
    fn switch_control_accepts_none_to_some_transition() {
        let controlled = entity(7);
        let mut state = ControllerState::new();

        state.switch_control(None, Some(controlled)).unwrap();

        assert_eq!(state.controlled_entity(), Some(controlled));
    }

    #[test]
    fn switch_control_accepts_some_to_none_transition() {
        let controlled = entity(8);
        let mut state = ControllerState::with_entity(controlled);

        state.switch_control(Some(controlled), None).unwrap();

        assert_eq!(state.controlled_entity(), None);
    }

    #[test]
    fn switch_control_rejects_mismatched_from_entity() {
        let current = entity(2);
        let wrong = entity(3);
        let replacement = entity(4);
        let mut state = ControllerState::with_entity(current);

        let error = state
            .switch_control(Some(wrong), Some(replacement))
            .unwrap_err();

        assert_eq!(
            error,
            ControlError::MismatchedFrom {
                expected: Some(wrong),
                actual: Some(current),
            }
        );
        assert_eq!(state.controlled_entity(), Some(current));
    }

    #[test]
    fn clear_always_removes_controlled_entity() {
        let mut state = ControllerState::with_entity(entity(9));

        state.clear();

        assert_eq!(state.controlled_entity(), None);
    }

    #[test]
    fn bincode_roundtrip_preserves_state() {
        let state = ControllerState::with_entity(entity(11));

        let bytes = bincode::serialize(&state).unwrap();
        let roundtrip: ControllerState = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, state);
    }
}
