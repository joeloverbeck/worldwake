use crate::Component;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemoryCapacityProfile {
    #[serde(default = "default_memory_capacity")]
    pub memory_capacity: u32,
}

impl Default for MemoryCapacityProfile {
    fn default() -> Self {
        Self {
            memory_capacity: default_memory_capacity(),
        }
    }
}

impl Component for MemoryCapacityProfile {}

const fn default_memory_capacity() -> u32 {
    32
}

#[cfg(test)]
mod tests {
    use super::MemoryCapacityProfile;
    use crate::traits::Component;
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Copy + Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn memory_capacity_profile_satisfies_required_bounds() {
        assert_component_bounds::<MemoryCapacityProfile>();
        assert_value_bounds::<MemoryCapacityProfile>();
    }

    #[test]
    fn memory_capacity_profile_defaults_to_32() {
        assert_eq!(MemoryCapacityProfile::default().memory_capacity, 32);
    }

    #[test]
    fn memory_capacity_profile_roundtrips_through_bincode() {
        let profile = MemoryCapacityProfile {
            memory_capacity: 17,
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: MemoryCapacityProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn memory_capacity_profile_default_matches_declared_constant() {
        let profile = MemoryCapacityProfile::default();

        assert_eq!(profile.memory_capacity, 32);
    }
}
