use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SlotKind {
    NeedSurvival,
    PainCare,
    ObligationDuty,
    EconomicOpportunity,
    SocialMotive,
}

#[cfg(test)]
mod tests {
    use super::SlotKind;

    #[test]
    fn slot_kind_serde_round_trips_all_five_variants() {
        for slot in [
            SlotKind::NeedSurvival,
            SlotKind::PainCare,
            SlotKind::ObligationDuty,
            SlotKind::EconomicOpportunity,
            SlotKind::SocialMotive,
        ] {
            let bytes = bincode::serialize(&slot).expect("SlotKind should serialize");
            let decoded: SlotKind =
                bincode::deserialize(&bytes).expect("SlotKind should deserialize");
            assert_eq!(decoded, slot);
        }
    }

    #[test]
    fn slot_kind_ord_matches_declaration_order() {
        assert!(SlotKind::NeedSurvival < SlotKind::PainCare);
        assert!(SlotKind::PainCare < SlotKind::ObligationDuty);
        assert!(SlotKind::ObligationDuty < SlotKind::EconomicOpportunity);
        assert!(SlotKind::EconomicOpportunity < SlotKind::SocialMotive);
    }
}
