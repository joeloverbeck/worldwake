use std::collections::BTreeMap;

use crate::PlanningSnapshot;
use worldwake_core::{
    belief_confidence, ActionDomain, ArtifactKind, ArtifactState, BeliefConfidencePolicy,
    BelievedArtifactState, BelievedEntityState, EntityId, NoticeTopic, Permille,
    SocialObservation, SocialObservationKind, Tick,
};

fn place_threat_estimate_from_memory(
    current_tick: Tick,
    confidence_policy: BeliefConfidencePolicy,
    entity_beliefs: &BTreeMap<EntityId, BelievedEntityState>,
    social_observations: &[SocialObservation],
    place: EntityId,
) -> Permille {
    let belief_threat = entity_beliefs
        .values()
        .filter(|belief| belief.last_known_place == Some(place))
        .filter(|belief| {
            belief
                .believed_activity
                .as_ref()
                .is_some_and(|activity| activity.action_domain == ActionDomain::Combat)
                || (belief.alive && !belief.wounds.is_empty())
        })
        .map(|belief| {
            belief_confidence(
                &belief.source,
                current_tick.0.saturating_sub(belief.observed_tick.0),
                &confidence_policy,
            )
            .value()
        })
        .max()
        .unwrap_or(0);

    let social_threat = social_observations
        .iter()
        .filter(|observation| observation.place == place)
        .filter(|observation| observation.kind() == SocialObservationKind::WitnessedConflict)
        .map(|observation| {
            belief_confidence(
                &observation.source,
                current_tick.0.saturating_sub(observation.observed_tick.0),
                &confidence_policy,
            )
            .value()
        })
        .max()
        .unwrap_or(0);

    let notice_threat = entity_beliefs
        .values()
        .filter_map(|belief| {
            let artifact = belief.believed_artifact.as_ref()?;
            matches!(
                artifact,
                BelievedArtifactState {
                    kind: ArtifactKind::Notice,
                    state: ArtifactState::Active,
                    notice_topic: Some(NoticeTopic::ThreatWarning {
                        place: warned_place
                    }),
                    ..
                } if *warned_place == place
            )
            .then(|| {
                belief_confidence(
                    &belief.source,
                    current_tick.0.saturating_sub(artifact.observed_tick.0),
                    &confidence_policy,
                )
                .value()
            })
        })
        .max()
        .unwrap_or(0);

    Permille::new(belief_threat.max(social_threat).max(notice_threat))
        .expect("place threat estimate must remain within permille bounds")
}

pub(crate) fn route_threat_estimate_from_memory(
    current_tick: Tick,
    confidence_policy: BeliefConfidencePolicy,
    entity_beliefs: &BTreeMap<EntityId, BelievedEntityState>,
    social_observations: &[SocialObservation],
    edge_from: EntityId,
    edge_to: EntityId,
) -> Permille {
    let from_threat = place_threat_estimate_from_memory(
        current_tick,
        confidence_policy,
        entity_beliefs,
        social_observations,
        edge_from,
    )
    .value();
    let to_threat = if edge_from == edge_to {
        0
    } else {
        place_threat_estimate_from_memory(
            current_tick,
            confidence_policy,
            entity_beliefs,
            social_observations,
            edge_to,
        )
        .value()
    };

    Permille::new(from_threat.saturating_add(to_threat).min(1000))
        .expect("route threat estimate must remain within permille bounds")
}

pub(crate) fn route_threat_estimate(
    snapshot: &PlanningSnapshot,
    edge_from: EntityId,
    edge_to: EntityId,
) -> Permille {
    route_threat_estimate_from_memory(
        snapshot.current_tick,
        snapshot.actor_confidence_policy,
        &snapshot.actor_known_entity_beliefs,
        &snapshot.actor_known_social_observations,
        edge_from,
        edge_to,
    )
}

pub(crate) fn perceived_direct_travel_cost_from_memory(
    current_tick: Tick,
    confidence_policy: BeliefConfidencePolicy,
    entity_beliefs: &BTreeMap<EntityId, BelievedEntityState>,
    social_observations: &[SocialObservation],
    edge_from: EntityId,
    edge_to: EntityId,
    base_ticks: u32,
) -> u32 {
    let threat = route_threat_estimate_from_memory(
        current_tick,
        confidence_policy,
        entity_beliefs,
        social_observations,
        edge_from,
        edge_to,
    );
    if threat.value() == 0 {
        return base_ticks;
    }

    let penalty = base_ticks
        .saturating_mul(u32::from(threat.value()))
        .div_ceil(1000);
    base_ticks.saturating_add(penalty)
}

#[cfg(test)]
mod tests {
    use super::{perceived_direct_travel_cost_from_memory, route_threat_estimate_from_memory};
    use std::collections::BTreeMap;
    use worldwake_core::{
        ActionDomain, ArtifactKind, ArtifactState, BeliefConfidencePolicy, BelievedActivity,
        BelievedArtifactState, BelievedEntityState, BodyPart, EntityId, NoticeTopic,
        PerceptionSource, SocialObservation, SocialObservationDetail, Tick, Wound, WoundCause,
        WoundId,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn pm(value: u16) -> worldwake_core::Permille {
        worldwake_core::Permille::new(value).unwrap()
    }

    fn sample_belief(place: EntityId, observed_tick: Tick) -> BelievedEntityState {
        BelievedEntityState {
            last_known_place: Some(place),
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: Some(BelievedActivity {
                action_domain: ActionDomain::Combat,
                target: None,
                observed_tick,
            }),
            believed_artifact: None,
            believed_contention: None,
            observed_tick,
            source: PerceptionSource::DirectObservation,
        }
    }

    fn sample_conflict(place: EntityId, observed_tick: Tick) -> SocialObservation {
        SocialObservation {
            detail: SocialObservationDetail::WitnessedConflict {
                actor: entity(90),
                target: entity(91),
            },
            place,
            observed_tick,
            source: PerceptionSource::DirectObservation,
        }
    }

    fn sample_wound() -> Wound {
        Wound {
            id: WoundId(1),
            body_part: BodyPart::Torso,
            cause: WoundCause::Deprivation(worldwake_core::DeprivationKind::Starvation),
            severity: pm(300),
            inflicted_at: Tick(1),
            bleed_rate_per_tick: pm(0),
        }
    }

    fn sample_threat_warning(place: EntityId, observed_tick: Tick) -> BelievedEntityState {
        BelievedEntityState {
            last_known_place: Some(entity(40)),
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: Some(BelievedArtifactState {
                kind: ArtifactKind::Notice,
                state: ArtifactState::Active,
                issuer: entity(41),
                expires_at: None,
                bounty_terms: None,
                notice_topic: Some(NoticeTopic::ThreatWarning { place }),
                observed_tick,
            }),
            believed_contention: None,
            observed_tick,
            source: PerceptionSource::DirectObservation,
        }
    }

    #[test]
    fn route_threat_is_nonzero_for_endpoint_conflict_belief() {
        let place_a = entity(1);
        let place_b = entity(2);
        let hostile = entity(10);
        let beliefs = BTreeMap::from([(hostile, sample_belief(place_a, Tick(9)))]);

        let threat = route_threat_estimate_from_memory(
            Tick(9),
            BeliefConfidencePolicy::default(),
            &beliefs,
            &[],
            place_a,
            place_b,
        );

        assert!(threat.value() > 0);
    }

    #[test]
    fn route_threat_is_zero_without_relevant_beliefs() {
        let threat = route_threat_estimate_from_memory(
            Tick(9),
            BeliefConfidencePolicy::default(),
            &BTreeMap::new(),
            &[],
            entity(1),
            entity(2),
        );

        assert_eq!(threat.value(), 0);
    }

    #[test]
    fn route_threat_decays_with_staleness() {
        let place_a = entity(1);
        let hostile = entity(10);
        let fresh = BTreeMap::from([(hostile, sample_belief(place_a, Tick(9)))]);
        let stale = BTreeMap::from([(hostile, sample_belief(place_a, Tick(1)))]);

        let fresh_threat = route_threat_estimate_from_memory(
            Tick(9),
            BeliefConfidencePolicy::default(),
            &fresh,
            &[],
            place_a,
            entity(2),
        );
        let stale_threat = route_threat_estimate_from_memory(
            Tick(9),
            BeliefConfidencePolicy::default(),
            &stale,
            &[],
            place_a,
            entity(2),
        );

        assert!(stale_threat < fresh_threat);
    }

    #[test]
    fn route_threat_aggregates_both_endpoints() {
        let place_a = entity(1);
        let place_b = entity(2);
        let beliefs = BTreeMap::from([
            (entity(10), sample_belief(place_a, Tick(9))),
            (entity(11), sample_belief(place_b, Tick(9))),
        ]);

        let one_endpoint = route_threat_estimate_from_memory(
            Tick(9),
            BeliefConfidencePolicy::default(),
            &BTreeMap::from([(entity(10), sample_belief(place_a, Tick(9)))]),
            &[],
            place_a,
            place_b,
        );
        let both_endpoints = route_threat_estimate_from_memory(
            Tick(9),
            BeliefConfidencePolicy::default(),
            &beliefs,
            &[],
            place_a,
            place_b,
        );

        assert!(both_endpoints > one_endpoint);
    }

    #[test]
    fn social_conflict_and_wounded_entities_contribute_route_threat() {
        let place = entity(1);
        let wounded = BelievedEntityState {
            wounds: vec![sample_wound()],
            believed_activity: None,
            ..sample_belief(place, Tick(8))
        };
        let beliefs = BTreeMap::from([(entity(10), wounded)]);
        let threat = route_threat_estimate_from_memory(
            Tick(9),
            BeliefConfidencePolicy::default(),
            &beliefs,
            &[sample_conflict(place, Tick(9))],
            place,
            entity(2),
        );

        assert!(threat.value() > 0);
    }

    #[test]
    fn threat_warning_notice_contributes_route_threat() {
        let place = entity(5);
        let notice = entity(50);
        let beliefs = BTreeMap::from([(notice, sample_threat_warning(place, Tick(9)))]);

        let threat = route_threat_estimate_from_memory(
            Tick(9),
            BeliefConfidencePolicy::default(),
            &beliefs,
            &[],
            place,
            entity(6),
        );

        assert!(threat.value() > 0);
    }

    #[test]
    fn perceived_direct_travel_cost_scales_with_route_threat() {
        let place_a = entity(1);
        let place_b = entity(2);
        let beliefs = BTreeMap::from([(entity(10), sample_belief(place_a, Tick(9)))]);

        let safe_cost = perceived_direct_travel_cost_from_memory(
            Tick(9),
            BeliefConfidencePolicy::default(),
            &BTreeMap::new(),
            &[],
            place_a,
            place_b,
            3,
        );
        let dangerous_cost = perceived_direct_travel_cost_from_memory(
            Tick(9),
            BeliefConfidencePolicy::default(),
            &beliefs,
            &[],
            place_a,
            place_b,
            3,
        );

        assert!(dangerous_cost > safe_cost);
    }

    #[test]
    fn perceived_direct_travel_cost_scales_with_threat_warning_notice() {
        let place_a = entity(1);
        let place_b = entity(2);
        let notice = entity(70);

        let safe_cost = perceived_direct_travel_cost_from_memory(
            Tick(9),
            BeliefConfidencePolicy::default(),
            &BTreeMap::new(),
            &[],
            place_a,
            place_b,
            3,
        );
        let warned_cost = perceived_direct_travel_cost_from_memory(
            Tick(9),
            BeliefConfidencePolicy::default(),
            &BTreeMap::from([(notice, sample_threat_warning(place_b, Tick(9)))]),
            &[],
            place_a,
            place_b,
            3,
        );

        assert!(warned_cost > safe_cost);
    }
}
