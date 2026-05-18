use crate::{MotiveSourceDiscriminant, SlotKind};

#[must_use]
pub fn slot_for(discriminant: MotiveSourceDiscriminant) -> SlotKind {
    match discriminant {
        MotiveSourceDiscriminant::NeedPressure => SlotKind::NeedSurvival,
        MotiveSourceDiscriminant::Pain => SlotKind::PainCare,
        MotiveSourceDiscriminant::OfficeDuty | MotiveSourceDiscriminant::Loyalty => {
            SlotKind::ObligationDuty
        }
        MotiveSourceDiscriminant::Greed => SlotKind::EconomicOpportunity,
        MotiveSourceDiscriminant::Shame | MotiveSourceDiscriminant::Revenge => {
            SlotKind::SocialMotive
        }
    }
}

#[cfg(test)]
mod tests {
    use super::slot_for;
    use crate::{MotiveSourceDiscriminant, SlotKind};

    #[test]
    fn slot_for_is_defined_for_every_motive_source_discriminant() {
        assert_eq!(
            slot_for(MotiveSourceDiscriminant::NeedPressure),
            SlotKind::NeedSurvival
        );
        assert_eq!(slot_for(MotiveSourceDiscriminant::Pain), SlotKind::PainCare);
        assert_eq!(
            slot_for(MotiveSourceDiscriminant::OfficeDuty),
            SlotKind::ObligationDuty
        );
        assert_eq!(
            slot_for(MotiveSourceDiscriminant::Loyalty),
            SlotKind::ObligationDuty
        );
        assert_eq!(
            slot_for(MotiveSourceDiscriminant::Greed),
            SlotKind::EconomicOpportunity
        );
        assert_eq!(
            slot_for(MotiveSourceDiscriminant::Shame),
            SlotKind::SocialMotive
        );
        assert_eq!(
            slot_for(MotiveSourceDiscriminant::Revenge),
            SlotKind::SocialMotive
        );
    }
}
