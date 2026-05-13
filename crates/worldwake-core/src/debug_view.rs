use crate::{EntityId, EntityKind};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EntityState {
    pub kind: Option<EntityKind>,
    pub place: Option<EntityId>,
    pub alive: bool,
    pub container: Option<EntityId>,
    pub possessor: Option<EntityId>,
}

#[cfg(test)]
mod tests {
    use super::EntityState;
    use crate::{EntityId, EntityKind};

    #[test]
    fn entity_state_default_is_empty_and_not_alive() {
        assert_eq!(
            EntityState::default(),
            EntityState {
                kind: None,
                place: None,
                alive: false,
                container: None,
                possessor: None,
            }
        );
    }

    #[test]
    fn entity_state_preserves_authoritative_snapshot_fields() {
        let state = EntityState {
            kind: Some(EntityKind::Container),
            place: Some(EntityId {
                slot: 1,
                generation: 0,
            }),
            alive: true,
            container: Some(EntityId {
                slot: 2,
                generation: 0,
            }),
            possessor: Some(EntityId {
                slot: 3,
                generation: 0,
            }),
        };

        assert_eq!(state.kind, Some(EntityKind::Container));
        assert_eq!(state.place.map(|place| place.slot), Some(1));
        assert!(state.alive);
        assert_eq!(state.container.map(|container| container.slot), Some(2));
        assert_eq!(state.possessor.map(|possessor| possessor.slot), Some(3));
    }
}
