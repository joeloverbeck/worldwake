use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum ActionStatus {
    Pending,
    Active,
    Committed,
    Aborted,
    Interrupted,
}

#[cfg(test)]
mod tests {
    use super::ActionStatus;
    use serde::{de::DeserializeOwned, Serialize};

    fn assert_traits<
        T: Copy + Clone + Eq + Ord + std::hash::Hash + std::fmt::Debug + Serialize + DeserializeOwned,
    >() {
    }

    const ALL_ACTION_STATUSES: [ActionStatus; 5] = [
        ActionStatus::Pending,
        ActionStatus::Active,
        ActionStatus::Committed,
        ActionStatus::Aborted,
        ActionStatus::Interrupted,
    ];

    #[test]
    fn action_status_satisfies_required_traits() {
        assert_traits::<ActionStatus>();
    }

    #[test]
    fn action_status_has_canonical_variant_list() {
        assert_eq!(ALL_ACTION_STATUSES.len(), 5);
    }

    #[test]
    fn action_status_order_is_declaration_stable() {
        let mut statuses = ALL_ACTION_STATUSES;
        statuses.reverse();
        statuses.sort_unstable();

        assert_eq!(statuses, ALL_ACTION_STATUSES);
    }

    #[test]
    fn action_status_bincode_roundtrip_covers_every_variant() {
        for status in ALL_ACTION_STATUSES {
            let bytes = bincode::serialize(&status).unwrap();
            let roundtrip: ActionStatus = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, status);
        }
    }
}
