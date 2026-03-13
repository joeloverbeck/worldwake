worldwake_core::worldwake_prefixed_id_type!(
    pub struct ActionHandlerId(u32, "ah");
);

worldwake_core::worldwake_prefixed_id_type!(
    pub struct ActionInstanceId(u64, "ai");
);

#[cfg(test)]
mod tests {
    use super::{ActionHandlerId, ActionInstanceId};
    use serde::{de::DeserializeOwned, Serialize};

    fn assert_traits<
        T: Copy
            + Clone
            + Eq
            + Ord
            + std::hash::Hash
            + std::fmt::Debug
            + std::fmt::Display
            + Serialize
            + DeserializeOwned,
    >() {
    }

    #[test]
    fn action_id_types_satisfy_required_traits() {
        assert_traits::<ActionHandlerId>();
        assert_traits::<ActionInstanceId>();
    }

    #[test]
    fn action_handler_id_display_is_stable() {
        assert_eq!(ActionHandlerId(7).to_string(), "ah7");
    }

    #[test]
    fn action_instance_id_display_is_stable() {
        assert_eq!(ActionInstanceId(11).to_string(), "ai11");
    }

    #[test]
    fn action_handler_id_bincode_roundtrip() {
        let bytes = bincode::serialize(&ActionHandlerId(9)).unwrap();
        let roundtrip: ActionHandlerId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, ActionHandlerId(9));
    }

    #[test]
    fn action_instance_id_bincode_roundtrip() {
        let bytes = bincode::serialize(&ActionInstanceId(13)).unwrap();
        let roundtrip: ActionInstanceId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, ActionInstanceId(13));
    }
}
