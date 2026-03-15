use crate::{Component, EntityId, Tick};
use serde::{Deserialize, Serialize};

/// Authoritative metadata attached to office entities.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OfficeData {
    pub title: String,
    pub jurisdiction: EntityId,
    pub succession_law: SuccessionLaw,
    pub eligibility_rules: Vec<EligibilityRule>,
    pub succession_period_ticks: u64,
    pub vacancy_since: Option<Tick>,
}

impl Component for OfficeData {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SuccessionLaw {
    Support,
    Force,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EligibilityRule {
    FactionMember(EntityId),
}

#[cfg(test)]
mod tests {
    use super::{EligibilityRule, OfficeData, SuccessionLaw};
    use crate::{traits::Component, EntityId, Tick};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn office_data_component_bounds() {
        assert_component_bounds::<OfficeData>();
        assert_value_bounds::<OfficeData>();
    }

    #[test]
    fn office_data_roundtrips_through_bincode() {
        let office = OfficeData {
            title: "Village Ruler".to_string(),
            jurisdiction: entity(10),
            succession_law: SuccessionLaw::Support,
            eligibility_rules: vec![EligibilityRule::FactionMember(entity(12))],
            succession_period_ticks: 48,
            vacancy_since: Some(Tick(22)),
        };

        let bytes = bincode::serialize(&office).unwrap();
        let roundtrip: OfficeData = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, office);
    }

    #[test]
    fn succession_law_variants_roundtrip_through_bincode() {
        for law in [SuccessionLaw::Support, SuccessionLaw::Force] {
            let bytes = bincode::serialize(&law).unwrap();
            let roundtrip: SuccessionLaw = bincode::deserialize(&bytes).unwrap();

            assert_eq!(roundtrip, law);
        }
    }

    #[test]
    fn eligibility_rule_roundtrips_through_bincode() {
        let rule = EligibilityRule::FactionMember(entity(7));

        let bytes = bincode::serialize(&rule).unwrap();
        let roundtrip: EligibilityRule = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, rule);
    }
}
