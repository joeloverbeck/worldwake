use crate::{Component, EntityId, Tick};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

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

/// Explicit force-succession timing policy attached to force-law offices.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OfficeForceProfile {
    pub uncontested_hold_ticks: NonZeroU32,
    pub vacancy_claim_grace_ticks: NonZeroU32,
    pub challenger_presence_grace_ticks: NonZeroU32,
}

impl Component for OfficeForceProfile {}

/// Mutable continuity state for force-based office control.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OfficeForceState {
    pub control_since: Option<Tick>,
    pub contested_since: Option<Tick>,
    pub last_uncontested_tick: Option<Tick>,
}

impl Component for OfficeForceState {}

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
    use super::{
        EligibilityRule, OfficeData, OfficeForceProfile, OfficeForceState, SuccessionLaw,
    };
    use crate::{traits::Component, EntityId, Tick};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;
    use std::num::NonZeroU32;

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
        assert_component_bounds::<OfficeForceProfile>();
        assert_value_bounds::<OfficeForceProfile>();
        assert_component_bounds::<OfficeForceState>();
        assert_value_bounds::<OfficeForceState>();
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

    #[test]
    fn office_force_profile_roundtrips_through_bincode() {
        let profile = OfficeForceProfile {
            uncontested_hold_ticks: NonZeroU32::new(12).unwrap(),
            vacancy_claim_grace_ticks: NonZeroU32::new(7).unwrap(),
            challenger_presence_grace_ticks: NonZeroU32::new(3).unwrap(),
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: OfficeForceProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn office_force_state_roundtrips_through_bincode() {
        let state = OfficeForceState {
            control_since: Some(Tick(9)),
            contested_since: Some(Tick(13)),
            last_uncontested_tick: Some(Tick(15)),
        };

        let bytes = bincode::serialize(&state).unwrap();
        let roundtrip: OfficeForceState = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, state);
    }
}
