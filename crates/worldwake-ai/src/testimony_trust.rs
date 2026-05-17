use worldwake_core::{
    EntityId, Permille, TellTopic, TestimonyReliability, TestimonyReliabilityKey,
    TestimonyTrustProfile, TestimonyTrustSummary, belief_topic_to_topic_scope,
};

const TESTIMONY_SUPPRESSION_FLOOR_FACTOR: Permille = Permille::new_unchecked(500);
const TESTIMONY_DAMPING_STRENGTH: Permille = Permille::new_unchecked(500);

#[must_use]
pub(crate) fn testimony_trust_summary(
    reliability: &TestimonyReliability,
    profile: &TestimonyTrustProfile,
    source: EntityId,
    topic: TellTopic,
) -> Option<TestimonyTrustSummary> {
    let topic = belief_topic_to_topic_scope(&topic);
    let key = TestimonyReliabilityKey { source, topic };
    reliability.get(&key).map(|entry| TestimonyTrustSummary {
        source,
        topic,
        trust: entry.trust(profile, topic),
        observations: entry.observations(),
    })
}

#[must_use]
pub(crate) fn testimony_suppression_floor(profile: &TestimonyTrustProfile) -> Permille {
    scale_permille(profile.trust_threshold, TESTIMONY_SUPPRESSION_FLOOR_FACTOR)
}

#[must_use]
pub(crate) fn testimony_damping_factor(
    trust: Permille,
    threshold: Permille,
    floor: Permille,
) -> Permille {
    if trust >= threshold {
        return Permille::new_unchecked(1000);
    }
    if trust <= floor {
        return TESTIMONY_DAMPING_STRENGTH;
    }
    let span = u32::from(threshold.value().saturating_sub(floor.value())).max(1);
    let deficit = u32::from(threshold.value().saturating_sub(trust.value()));
    let attenuation = deficit.saturating_mul(u32::from(TESTIMONY_DAMPING_STRENGTH.value())) / span;
    Permille::new_unchecked(1000u32.saturating_sub(attenuation).min(1000) as u16)
}

fn scale_permille(value: Permille, factor: Permille) -> Permille {
    let scaled = u32::from(value.value()).saturating_mul(u32::from(factor.value())) / 1000;
    Permille::new_unchecked(scaled.min(1000) as u16)
}
