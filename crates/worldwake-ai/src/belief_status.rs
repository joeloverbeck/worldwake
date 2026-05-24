use worldwake_core::{
    BeliefConfidencePolicy, BeliefStatusTag, EntityBeliefClaim, EntityId, Permille, Tick,
    effective_claim_confidence,
};
use worldwake_sim::RuntimeBeliefView;

pub(crate) fn belief_status_tag_for_claim(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    claim: &EntityBeliefClaim,
    tick: Tick,
) -> BeliefStatusTag {
    belief_status_tag_for_claim_parts(
        claim,
        tick,
        &view.belief_confidence_policy(agent),
        view.claim_confidence_threshold(agent),
    )
}

fn belief_status_tag_for_claim_parts(
    claim: &EntityBeliefClaim,
    tick: Tick,
    policy: &BeliefConfidencePolicy,
    threshold: Permille,
) -> BeliefStatusTag {
    if claim.refuted_at_tick.is_some() {
        return BeliefStatusTag::Contradicted;
    }

    let effective = effective_claim_confidence(claim, tick, policy);
    let threshold = threshold.value();
    let certain_floor = threshold.saturating_mul(2).min(1000);
    if effective >= certain_floor {
        BeliefStatusTag::Certain
    } else if effective >= threshold {
        BeliefStatusTag::Probable
    } else {
        BeliefStatusTag::Stale
    }
}

#[cfg(test)]
mod tests {
    use super::belief_status_tag_for_claim_parts;
    use worldwake_core::{
        BeliefConfidencePolicy, BeliefStatusTag, ClaimId, ClaimValue, CommodityKind,
        EntityBeliefAspect, EntityBeliefClaim, EntityId, PerceptionSource, Permille, Quantity,
        Tick,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn claim(confidence: u16, acquired_tick: Tick) -> EntityBeliefClaim {
        EntityBeliefClaim {
            claim_id: ClaimId(1),
            subject: entity(10),
            aspect: EntityBeliefAspect::Inventory(CommodityKind::Bread),
            value: ClaimValue::Quantity(Quantity(1)),
            source: PerceptionSource::DirectObservation,
            acquired_tick,
            claimed_event_tick: None,
            confidence: Permille::new(confidence).unwrap(),
            refuted_at_tick: None,
        }
    }

    #[test]
    fn derives_tag_for_each_status_class() {
        let policy = BeliefConfidencePolicy {
            staleness_penalty_per_tick: Permille::new(10).unwrap(),
            ..BeliefConfidencePolicy::default()
        };
        let threshold = Permille::new(400).unwrap();
        let tick = Tick(10);

        let mut refuted = claim(950, tick);
        refuted.refuted_at_tick = Some(tick);
        assert_eq!(
            belief_status_tag_for_claim_parts(&refuted, tick, &policy, threshold),
            BeliefStatusTag::Contradicted
        );

        assert_eq!(
            belief_status_tag_for_claim_parts(&claim(900, tick), tick, &policy, threshold),
            BeliefStatusTag::Certain
        );
        assert_eq!(
            belief_status_tag_for_claim_parts(&claim(600, tick), tick, &policy, threshold),
            BeliefStatusTag::Probable
        );
        assert_eq!(
            belief_status_tag_for_claim_parts(&claim(450, Tick(0)), tick, &policy, threshold),
            BeliefStatusTag::Stale
        );
    }
}
