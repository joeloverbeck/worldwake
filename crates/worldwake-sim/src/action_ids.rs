use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! action_id_type {
    ($name:ident, $inner:ty, $prefix:literal) => {
        #[derive(
            Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize,
        )]
        pub struct $name(pub $inner);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!($prefix, "{}"), self.0)
            }
        }
    };
}

action_id_type!(ActionDefId, u32, "adef");
action_id_type!(ActionHandlerId, u32, "ah");
action_id_type!(ActionInstanceId, u64, "ai");

#[cfg(test)]
mod tests {
    use super::{ActionDefId, ActionHandlerId, ActionInstanceId};
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
        assert_traits::<ActionDefId>();
        assert_traits::<ActionHandlerId>();
        assert_traits::<ActionInstanceId>();
    }

    #[test]
    fn action_def_id_display_is_stable() {
        assert_eq!(ActionDefId(3).to_string(), "adef3");
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
    fn action_def_id_bincode_roundtrip() {
        let bytes = bincode::serialize(&ActionDefId(5)).unwrap();
        let roundtrip: ActionDefId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, ActionDefId(5));
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
