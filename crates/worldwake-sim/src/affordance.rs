use crate::ActionDefId;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use worldwake_core::EntityId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Affordance {
    pub def_id: ActionDefId,
    pub actor: EntityId,
    pub bound_targets: Vec<EntityId>,
    pub explanation: Option<String>,
}

impl Ord for Affordance {
    fn cmp(&self, other: &Self) -> Ordering {
        self.def_id
            .cmp(&other.def_id)
            .then_with(|| self.bound_targets.cmp(&other.bound_targets))
            .then_with(|| self.actor.cmp(&other.actor))
            .then_with(|| self.explanation.cmp(&other.explanation))
    }
}

impl PartialOrd for Affordance {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::Affordance;
    use crate::ActionDefId;
    use serde::{de::DeserializeOwned, Serialize};
    use worldwake_core::EntityId;

    fn assert_traits<T: Clone + Eq + Ord + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    #[test]
    fn affordance_satisfies_required_traits() {
        assert_traits::<Affordance>();
    }

    #[test]
    fn affordance_ordering_uses_def_id_then_bound_targets() {
        let actor = entity(99);
        let mut affordances = [
            Affordance {
                def_id: ActionDefId(2),
                actor,
                bound_targets: vec![entity(4)],
                explanation: Some("later".to_string()),
            },
            Affordance {
                def_id: ActionDefId(1),
                actor,
                bound_targets: vec![entity(7)],
                explanation: Some("human".to_string()),
            },
            Affordance {
                def_id: ActionDefId(1),
                actor,
                bound_targets: vec![entity(3)],
                explanation: None,
            },
        ];

        affordances.sort();

        assert_eq!(affordances[0].def_id, ActionDefId(1));
        assert_eq!(affordances[0].bound_targets, vec![entity(3)]);
        assert_eq!(affordances[1].def_id, ActionDefId(1));
        assert_eq!(affordances[1].bound_targets, vec![entity(7)]);
        assert_eq!(affordances[2].def_id, ActionDefId(2));
    }
}
