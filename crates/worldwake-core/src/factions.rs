use crate::Component;
use serde::{Deserialize, Serialize};

/// Authoritative metadata attached to faction entities.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FactionData {
    pub name: String,
    pub purpose: FactionPurpose,
}

impl Component for FactionData {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FactionPurpose {
    Political,
    Military,
    Trade,
    Religious,
}

#[cfg(test)]
mod tests {
    use super::{FactionData, FactionPurpose};
    use crate::traits::Component;
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn faction_data_component_bounds() {
        assert_component_bounds::<FactionData>();
        assert_value_bounds::<FactionData>();
    }

    #[test]
    fn faction_data_roundtrips_through_bincode() {
        let faction = FactionData {
            name: "Town Council".to_string(),
            purpose: FactionPurpose::Political,
        };

        let bytes = bincode::serialize(&faction).unwrap();
        let roundtrip: FactionData = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, faction);
    }

    #[test]
    fn faction_purpose_variants_roundtrip_through_bincode() {
        for purpose in [
            FactionPurpose::Political,
            FactionPurpose::Military,
            FactionPurpose::Trade,
            FactionPurpose::Religious,
        ] {
            let bytes = bincode::serialize(&purpose).unwrap();
            let roundtrip: FactionPurpose = bincode::deserialize(&bytes).unwrap();

            assert_eq!(roundtrip, purpose);
        }
    }
}
