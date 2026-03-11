use crate::{AbortReason, ActionDefId, ActionInstanceId};
use serde::{Deserialize, Serialize};
use worldwake_core::{EntityId, Tick};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReplanNeeded {
    pub agent: EntityId,
    pub failed_action_def: ActionDefId,
    pub failed_instance: ActionInstanceId,
    pub reason: AbortReason,
    pub tick: Tick,
}

#[cfg(test)]
mod tests {
    use super::ReplanNeeded;
    use crate::{AbortReason, ActionDefId, ActionInstanceId, InterruptReason};
    use serde::{de::DeserializeOwned, Serialize};
    use worldwake_core::{EntityId, Tick};

    fn assert_traits<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    fn sample_replan_needed() -> ReplanNeeded {
        ReplanNeeded {
            agent: EntityId {
                slot: 3,
                generation: 1,
            },
            failed_action_def: ActionDefId(4),
            failed_instance: ActionInstanceId(9),
            reason: AbortReason::interrupted(InterruptReason::DangerNearby),
            tick: Tick(17),
        }
    }

    #[test]
    fn replan_needed_satisfies_required_traits() {
        assert_traits::<ReplanNeeded>();
    }

    #[test]
    fn replan_needed_roundtrips_through_bincode() {
        let record = sample_replan_needed();

        let bytes = bincode::serialize(&record).unwrap();
        let roundtrip: ReplanNeeded = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, record);
    }
}
