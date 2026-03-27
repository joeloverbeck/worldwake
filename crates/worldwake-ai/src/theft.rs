use worldwake_core::{EntityId, EntityKind, TheftDispositionProfile};
use worldwake_sim::GoalBeliefView;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct TheftDeterrenceAssessment {
    pub witness_count: u32,
    pub witness_penalty: u32,
    pub effective_motive: u32,
}

impl TheftDeterrenceAssessment {
    #[must_use]
    pub(crate) fn from_witness_count(
        profile: &TheftDispositionProfile,
        witness_count: u32,
    ) -> Self {
        let witness_penalty =
            u32::from(profile.witness_risk_penalty.value()).saturating_mul(witness_count);
        let effective_motive =
            u32::from(profile.theft_motive_weight.value()).saturating_sub(witness_penalty);

        Self {
            witness_count,
            witness_penalty,
            effective_motive,
        }
    }
}

#[must_use]
pub(crate) fn assess_theft_deterrence(
    view: &dyn GoalBeliefView,
    agent: EntityId,
) -> Option<TheftDeterrenceAssessment> {
    let profile = view.theft_disposition_profile(agent)?;
    let place = view.effective_place(agent)?;

    let witness_count = view
        .locally_observed_entities_at(agent, place)
        .into_iter()
        .filter(|entity| *entity != agent)
        .filter(|entity| view.entity_kind(*entity) == Some(EntityKind::Agent))
        .filter(|entity| view.is_alive(*entity))
        .count() as u32;

    Some(TheftDeterrenceAssessment::from_witness_count(
        &profile,
        witness_count,
    ))
}

#[cfg(test)]
mod tests {
    use super::TheftDeterrenceAssessment;
    use std::num::NonZeroU32;
    use worldwake_core::{Permille, TheftDispositionProfile};

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    #[test]
    fn theft_deterrence_assessment_supports_zero_partial_and_full_deterrence() {
        let profile = TheftDispositionProfile {
            steal_duration_ticks: NonZeroU32::new(3).unwrap(),
            theft_motive_weight: pm(500),
            witness_risk_penalty: pm(200),
        };

        let zero = TheftDeterrenceAssessment::from_witness_count(&profile, 0);
        assert_eq!(zero.witness_count, 0);
        assert_eq!(zero.witness_penalty, 0);
        assert_eq!(zero.effective_motive, 500);

        let partial = TheftDeterrenceAssessment::from_witness_count(&profile, 2);
        assert_eq!(partial.witness_count, 2);
        assert_eq!(partial.witness_penalty, 400);
        assert_eq!(partial.effective_motive, 100);

        let full = TheftDeterrenceAssessment::from_witness_count(&profile, 3);
        assert_eq!(full.witness_count, 3);
        assert_eq!(full.witness_penalty, 600);
        assert_eq!(full.effective_motive, 0);
    }
}
