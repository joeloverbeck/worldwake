//! Golden coverage for S151 route preference state contracts.

use worldwake_core::{
    Blocker, BlockerClearingCondition, BlockerMemory, BlockerScope, BlockingFact, EntityId,
    EventId, Permille, RoutePreference, RoutePreferenceProfile, RouteSegment, Tick,
};

const START: EntityId = entity(1520);
const DIRECT: EntityId = entity(1521);
const ALTERNATE: EntityId = entity(1522);

const fn entity(slot: u32) -> EntityId {
    EntityId {
        slot,
        generation: 0,
    }
}

fn profile() -> RoutePreferenceProfile {
    RoutePreferenceProfile {
        safe_traversal_weight: Permille::new_unchecked(200),
        dangerous_traversal_penalty: Permille::new_unchecked(600),
        days_to_decay_observations: 1,
        minimum_traversals: 2,
    }
}

fn direct_segment() -> RouteSegment {
    RouteSegment::new(START, DIRECT)
}

fn positive_preference() -> RoutePreference {
    let mut preference = RoutePreference::default();
    for tick in 0..5 {
        preference.record_safe(direct_segment(), EventId(tick), Tick(tick));
    }
    preference
}

// Scenario 427: S151 Safe Route Traversal Raises Preference
// Systems: AI, Travel
// GoalKinds: Travel
// ActionDomains: Travel
// Principles: P3, P15, P31
// Setup: the same route segment is traversed safely five times; no dangerous
//        traversal is staged, so the signal is a pure safe-route preference.
// Proves: the route preference entry records all safe traversals and derives a
//         preference above neutral.
// Cross-system chain: safe traversal observations -> RoutePreferenceEntry ->
//                     derived route preference.
#[test]
fn golden_route_preference_safe_traversals_raise_preference() {
    let preference = positive_preference();
    let entry = preference
        .get(&direct_segment())
        .expect("safe traversals should create route preference entry");

    assert_eq!(entry.safe_traversals, 5);
    assert_eq!(entry.dangerous_traversals, 0);
    assert_eq!(entry.last_traversal_event, Some(EventId(4)));
    assert!(entry.preference(&profile(), Tick(5)) > Permille::new_unchecked(500));
}

// Scenario 428: S151 Dangerous Traversal Lowers Preference
// Systems: AI, Travel, Combat
// GoalKinds: Travel
// ActionDomains: Travel
// Principles: P3, P15, P31
// Setup: a dangerous traversal event is recorded on the direct segment with no
//        safe offsetting observations.
// Proves: the route preference entry stores the dangerous event provenance and
//         derives a preference below neutral.
// Cross-system chain: hostile route experience -> RoutePreferenceEntry ->
//                     derived route avoidance.
#[test]
fn golden_route_preference_dangerous_traversal_lowers_preference() {
    let mut preference = RoutePreference::default();
    preference.record_dangerous(direct_segment(), EventId(77), Tick(9));
    preference.record_dangerous(direct_segment(), EventId(78), Tick(10));
    let entry = preference
        .get(&direct_segment())
        .expect("dangerous traversals should create route preference entry");

    assert_eq!(entry.dangerous_traversals, 2);
    assert_eq!(entry.last_dangerous_tick, Some(Tick(10)));
    assert_eq!(entry.last_traversal_event, Some(EventId(78)));
    assert!(entry.preference(&profile(), Tick(10)) < Permille::new_unchecked(500));
}

// Scenario 429: S151 Route Preference Decays To Neutral
// Systems: AI, Travel
// GoalKinds: Travel
// ActionDomains: Travel
// Principles: P3, P15, P31
// Setup: safe traversals occur at tick 0 and time advances beyond the profile's
//        one-day decay window with no further observations.
// Proves: the derived route preference returns to neutral after the concrete
//         decay horizon.
// Cross-system chain: route observation age -> RoutePreferenceProfile decay ->
//                     neutral preference.
#[test]
fn golden_route_preference_decays_to_neutral_after_profile_window() {
    let mut preference = RoutePreference::default();
    for _ in 0..5 {
        preference.record_safe(direct_segment(), EventId(0), Tick(0));
    }
    let entry = preference
        .get(&direct_segment())
        .expect("safe traversals should create route preference entry");

    assert_eq!(
        entry.preference(&profile(), Tick(1440)),
        Permille::new_unchecked(500)
    );
}

// Scenario 430: S151 Route Preference And S150 Blocker Compose
// Systems: AI, Travel, BlockerMemory
// GoalKinds: Travel
// ActionDomains: Travel
// Principles: P12, P26, P28
// Setup: the same segment has a positive route preference and an active
//        RouteSegment blocker; the alternate segment has neither.
// Proves: the preference remains inspectable as a soft signal while the blocker
//         is independently active as a hard suppression surface.
// Cross-system chain: route preference state + blocker memory -> independent
//                     route decision inputs.
#[test]
fn golden_route_preference_and_route_segment_blocker_compose_independently() {
    let preference = positive_preference();
    let entry = preference
        .get(&direct_segment())
        .expect("positive preference should exist for direct segment");
    let mut blockers = BlockerMemory::default();
    blockers.record(Blocker {
        scope: BlockerScope::RouteSegment(direct_segment()),
        blocking_fact: BlockingFact::DangerTooHigh,
        diagnostic_context: None,
        observed_tick: Tick(5),
        expires_tick: Tick(50),
        clearing_condition: BlockerClearingCondition::RouteRetraversedSafely(direct_segment()),
        baseline_snapshot: None,
        source: worldwake_core::BlockerSource::Event(EventId(90)),
    });

    assert!(entry.preference(&profile(), Tick(6)) > Permille::new_unchecked(500));
    assert!(
        blockers
            .route_segment_blocked(START, DIRECT, Tick(6))
            .is_some()
    );
    assert!(
        blockers
            .route_segment_blocked(START, ALTERNATE, Tick(6))
            .is_none()
    );
}
