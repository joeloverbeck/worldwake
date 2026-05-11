//! Typed event payloads for explicit contention-resolution records.

use crate::{ActionDefId, ContentionQueue, EntityId, Tick};
use serde::{Deserialize, Serialize};

const MAX_INLINE_CLAIMANTS: usize = 8;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct AffordanceKey {
    pub facility: EntityId,
    pub action: ActionDefId,
}

impl Default for AffordanceKey {
    fn default() -> Self {
        Self {
            facility: EntityId {
                slot: 0,
                generation: 0,
            },
            action: ActionDefId(0),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentionEventPayload {
    pub contested_affordance: AffordanceKey,
    pub place: EntityId,
    pub resolution_rule: ContentionResolutionRule,
    pub claimants: Vec<ContentionClaimant>,
    pub total_claimants: u16,
    pub winner: Option<EntityId>,
    pub at_tick: Tick,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ContentionClaimant {
    pub agent: EntityId,
    pub arrived_tick: Tick,
    pub queue_position: u16,
    pub outcome: ClaimantOutcome,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ClaimantOutcome {
    Granted,
    QueuedAhead,
    QueuedBehind,
    Denied { reason: DenialReason },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum DenialReason {
    QueueFull,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ContentionResolutionRule {
    ArrivalTime,
}

#[must_use]
pub fn build_contention_event_payload(
    queue_snapshot: &ContentionQueue,
    facility: EntityId,
    place: EntityId,
    action: ActionDefId,
    rule: ContentionResolutionRule,
    granted_actor: Option<EntityId>,
    tick: Tick,
) -> ContentionEventPayload {
    let granted_position = granted_actor.and_then(|actor| {
        queue_snapshot
            .waiting
            .values()
            .position(|waiter| waiter.actor == actor)
    });
    let total_claimants = u16::try_from(queue_snapshot.waiting.len()).unwrap_or(u16::MAX);
    let claimants = queue_snapshot
        .waiting
        .values()
        .enumerate()
        .take(MAX_INLINE_CLAIMANTS)
        .map(|(index, waiter)| {
            let outcome = match (granted_actor, granted_position) {
                (Some(actor), _) if waiter.actor == actor => ClaimantOutcome::Granted,
                (Some(_), Some(position)) if index < position => ClaimantOutcome::QueuedAhead,
                (Some(_), Some(_)) => ClaimantOutcome::QueuedBehind,
                (Some(_), None) | (None, _) => ClaimantOutcome::QueuedAhead,
            };

            ContentionClaimant {
                agent: waiter.actor,
                arrived_tick: waiter.queued_at,
                queue_position: u16::try_from(index + 1).unwrap_or(u16::MAX),
                outcome,
            }
        })
        .collect();

    ContentionEventPayload {
        contested_affordance: AffordanceKey { facility, action },
        place,
        resolution_rule: rule,
        claimants,
        total_claimants,
        winner: granted_actor,
        at_tick: tick,
    }
}

#[cfg(test)]
mod tests {
    use super::{ClaimantOutcome, ContentionResolutionRule, build_contention_event_payload};
    use crate::{ActionDefId, ContentionQueue, EntityId, Tick};

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn queue_with_waiters(count: u32) -> ContentionQueue {
        let mut queue = ContentionQueue::default();
        for ordinal in 0..count {
            queue
                .enqueue(
                    entity(ordinal + 1),
                    ActionDefId(77),
                    Tick(u64::from(10 + ordinal)),
                    None,
                )
                .unwrap();
        }
        queue
    }

    #[test]
    fn build_contention_event_payload_classifies_granted_and_waiting_claimants() {
        let queue = queue_with_waiters(3);
        let payload = build_contention_event_payload(
            &queue,
            entity(100),
            entity(200),
            ActionDefId(77),
            ContentionResolutionRule::ArrivalTime,
            Some(entity(2)),
            Tick(50),
        );

        assert_eq!(payload.contested_affordance.facility, entity(100));
        assert_eq!(payload.contested_affordance.action, ActionDefId(77));
        assert_eq!(payload.place, entity(200));
        assert_eq!(
            payload.resolution_rule,
            ContentionResolutionRule::ArrivalTime
        );
        assert_eq!(payload.total_claimants, 3);
        assert_eq!(payload.winner, Some(entity(2)));
        assert_eq!(payload.at_tick, Tick(50));
        assert_eq!(payload.claimants.len(), 3);
        assert_eq!(payload.claimants[0].agent, entity(1));
        assert_eq!(payload.claimants[0].arrived_tick, Tick(10));
        assert_eq!(payload.claimants[0].queue_position, 1);
        assert_eq!(payload.claimants[0].outcome, ClaimantOutcome::QueuedAhead);
        assert_eq!(payload.claimants[1].agent, entity(2));
        assert_eq!(payload.claimants[1].arrived_tick, Tick(11));
        assert_eq!(payload.claimants[1].queue_position, 2);
        assert_eq!(payload.claimants[1].outcome, ClaimantOutcome::Granted);
        assert_eq!(payload.claimants[2].agent, entity(3));
        assert_eq!(payload.claimants[2].arrived_tick, Tick(12));
        assert_eq!(payload.claimants[2].queue_position, 3);
        assert_eq!(payload.claimants[2].outcome, ClaimantOutcome::QueuedBehind);
    }

    #[test]
    fn build_contention_event_payload_truncates_to_head_eight() {
        let queue = queue_with_waiters(12);
        let payload = build_contention_event_payload(
            &queue,
            entity(100),
            entity(200),
            ActionDefId(77),
            ContentionResolutionRule::ArrivalTime,
            Some(entity(1)),
            Tick(50),
        );

        assert_eq!(payload.claimants.len(), 8);
        assert_eq!(payload.total_claimants, 12);
        assert_eq!(payload.claimants[0].agent, entity(1));
        assert_eq!(payload.claimants[7].agent, entity(8));
    }

    #[test]
    fn build_contention_event_payload_uses_btreemap_ordinal_order() {
        let mut queue = ContentionQueue::default();
        let _ = queue.enqueue(entity(10), ActionDefId(77), Tick(30), None);
        let _ = queue.enqueue(entity(20), ActionDefId(77), Tick(10), None);
        let _ = queue.enqueue(entity(30), ActionDefId(77), Tick(20), None);

        let payload = build_contention_event_payload(
            &queue,
            entity(100),
            entity(200),
            ActionDefId(77),
            ContentionResolutionRule::ArrivalTime,
            Some(entity(10)),
            Tick(50),
        );

        let agents: Vec<_> = payload
            .claimants
            .iter()
            .map(|claimant| claimant.agent)
            .collect();
        let positions: Vec<_> = payload
            .claimants
            .iter()
            .map(|claimant| claimant.queue_position)
            .collect();
        assert_eq!(agents, vec![entity(10), entity(20), entity(30)]);
        assert_eq!(positions, vec![1, 2, 3]);
    }

    #[test]
    fn build_contention_event_payload_without_grant_keeps_all_claimants_ahead() {
        let queue = queue_with_waiters(3);
        let payload = build_contention_event_payload(
            &queue,
            entity(100),
            entity(200),
            ActionDefId(77),
            ContentionResolutionRule::ArrivalTime,
            None,
            Tick(50),
        );

        assert_eq!(payload.winner, None);
        assert!(
            payload
                .claimants
                .iter()
                .all(|claimant| claimant.outcome == ClaimantOutcome::QueuedAhead)
        );
    }
}
