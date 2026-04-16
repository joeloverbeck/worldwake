//! Authoritative generalized contention state for exclusive affordances.

use crate::{ActionDefId, Component, EntityId, GoalKey, Tick};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::num::NonZeroU32;

/// Stored queue/grant state for a single contended entity.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentionQueue {
    pub next_ordinal: u32,
    pub waiting: BTreeMap<u32, ContentionWaiter>,
    pub granted: Option<ContentionGrant>,
}

impl Component for ContentionQueue {}

/// One waiting actor's intended exclusive action on a contended entity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentionWaiter {
    pub actor: EntityId,
    pub intended_action: ActionDefId,
    pub queued_at: Tick,
}

/// The currently active exclusive-access grant on a contended entity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentionGrant {
    pub actor: EntityId,
    pub intended_action: ActionDefId,
    pub granted_at: Tick,
    pub expires_at: Tick,
}

/// Per-entity policy governing exclusive-access contention.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentionPolicy {
    pub grant_hold_ticks: NonZeroU32,
    pub auto_promote: bool,
    pub max_waiters: Option<u8>,
}

impl Component for ContentionPolicy {}

/// Per-agent tracking of entities the agent is contending for.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentionIntents {
    pub intents: BTreeMap<EntityId, QueuedContentionIntent>,
}

impl Component for ContentionIntents {}

/// The queued contention-relevant intention for one contended entity.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QueuedContentionIntent {
    pub goal_key: GoalKey,
    pub intended_action: ActionDefId,
}

/// Per-agent tolerance for waiting in generalized contention queues.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentionDispositionProfile {
    /// Maximum ticks the agent will wait in a contention queue before abandoning. None means infinite patience.
    pub queue_patience_ticks: Option<NonZeroU32>,
}

impl Component for ContentionDispositionProfile {}

/// Typed queue-state errors for contention operations.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ContentionError {
    DuplicateActor(EntityId),
    OrdinalOverflow,
    QueueFull,
}

/// Derived affordance-time summary of an actor's relation to a contended entity.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ContentionStatus {
    #[default]
    Unmanaged,
    Granted,
    Queued {
        position: u32,
    },
    Available,
    Full,
}

impl ContentionQueue {
    pub fn enqueue(
        &mut self,
        actor: EntityId,
        intended_action: ActionDefId,
        tick: Tick,
        max_waiters: Option<u8>,
    ) -> Result<u32, ContentionError> {
        if self.has_actor(actor) {
            return Err(ContentionError::DuplicateActor(actor));
        }

        if max_waiters.is_some_and(|limit| self.waiting.len() >= usize::from(limit)) {
            return Err(ContentionError::QueueFull);
        }

        let ordinal = self.next_ordinal;
        self.next_ordinal = self
            .next_ordinal
            .checked_add(1)
            .ok_or(ContentionError::OrdinalOverflow)?;
        self.waiting.insert(
            ordinal,
            ContentionWaiter {
                actor,
                intended_action,
                queued_at: tick,
            },
        );
        Ok(ordinal)
    }

    pub fn position_of(&self, actor: EntityId) -> Option<u32> {
        self.waiting
            .values()
            .enumerate()
            .find_map(|(position, queued)| (queued.actor == actor).then_some(position as u32))
    }

    pub fn has_actor(&self, actor: EntityId) -> bool {
        self.waiting.values().any(|queued| queued.actor == actor)
            || self
                .granted
                .as_ref()
                .is_some_and(|granted| granted.actor == actor)
    }

    pub fn remove_actor(&mut self, actor: EntityId) -> bool {
        if let Some(ordinal) = self
            .waiting
            .iter()
            .find_map(|(ordinal, queued)| (queued.actor == actor).then_some(*ordinal))
        {
            self.waiting.remove(&ordinal);
            return true;
        }

        if self
            .granted
            .as_ref()
            .is_some_and(|granted| granted.actor == actor)
        {
            self.granted = None;
            return true;
        }

        false
    }

    pub fn promote_head(
        &mut self,
        tick: Tick,
        grant_hold_ticks: NonZeroU32,
    ) -> Option<&ContentionGrant> {
        if self.granted.is_some() {
            return self.granted.as_ref();
        }

        let (&ordinal, queued) = self.waiting.iter().next()?;
        let granted = ContentionGrant {
            actor: queued.actor,
            intended_action: queued.intended_action,
            granted_at: tick,
            expires_at: tick + u64::from(grant_hold_ticks.get()),
        };
        self.waiting.remove(&ordinal);
        self.granted = Some(granted);
        self.granted.as_ref()
    }

    pub fn clear_grant(&mut self) {
        self.granted = None;
    }

    pub fn grant_expired(&self, current_tick: Tick) -> bool {
        self.granted
            .as_ref()
            .is_some_and(|granted| current_tick >= granted.expires_at)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ContentionDispositionProfile, ContentionError, ContentionGrant, ContentionIntents,
        ContentionPolicy, ContentionQueue, ContentionStatus, ContentionWaiter,
        QueuedContentionIntent,
    };
    use crate::{ActionDefId, GoalKey, GoalKind, Tick, traits::Component};
    use serde::{Serialize, de::DeserializeOwned};
    use std::collections::BTreeMap;
    use std::fmt::Debug;
    use std::num::NonZeroU32;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn actor(slot: u32) -> crate::EntityId {
        crate::test_utils::entity_id(slot, 0)
    }

    #[test]
    fn queue_types_satisfy_component_and_value_bounds() {
        assert_component_bounds::<ContentionPolicy>();
        assert_component_bounds::<ContentionQueue>();
        assert_component_bounds::<ContentionIntents>();
        assert_component_bounds::<ContentionDispositionProfile>();
        assert_value_bounds::<ContentionPolicy>();
        assert_value_bounds::<ContentionQueue>();
        assert_value_bounds::<ContentionWaiter>();
        assert_value_bounds::<ContentionGrant>();
        assert_value_bounds::<ContentionIntents>();
        assert_value_bounds::<QueuedContentionIntent>();
        assert_value_bounds::<ContentionDispositionProfile>();
        assert_value_bounds::<ContentionStatus>();
    }

    #[test]
    fn enqueue_appends_and_returns_incrementing_ordinals() {
        let mut queue = ContentionQueue::default();

        assert_eq!(
            queue.enqueue(actor(1), ActionDefId(4), Tick(10), None),
            Ok(0)
        );
        assert_eq!(
            queue.enqueue(actor(2), ActionDefId(5), Tick(11), None),
            Ok(1)
        );
        assert_eq!(queue.next_ordinal, 2);
        assert_eq!(queue.waiting.len(), 2);
    }

    #[test]
    fn enqueue_rejects_duplicate_actor_membership() {
        let mut queue = ContentionQueue::default();
        queue
            .enqueue(actor(1), ActionDefId(4), Tick(10), None)
            .unwrap();

        assert_eq!(
            queue.enqueue(actor(1), ActionDefId(5), Tick(11), None),
            Err(ContentionError::DuplicateActor(actor(1)))
        );
    }

    #[test]
    fn position_of_is_zero_indexed_from_queue_head() {
        let mut queue = ContentionQueue::default();
        queue
            .enqueue(actor(1), ActionDefId(4), Tick(10), None)
            .unwrap();
        queue
            .enqueue(actor(2), ActionDefId(5), Tick(11), None)
            .unwrap();
        queue
            .enqueue(actor(3), ActionDefId(6), Tick(12), None)
            .unwrap();

        assert_eq!(queue.position_of(actor(1)), Some(0));
        assert_eq!(queue.position_of(actor(2)), Some(1));
        assert_eq!(queue.position_of(actor(3)), Some(2));
        assert_eq!(queue.position_of(actor(4)), None);
    }

    #[test]
    fn has_actor_and_remove_actor_cover_waiting_and_granted_entries() {
        let mut queue = ContentionQueue::default();
        queue
            .enqueue(actor(1), ActionDefId(4), Tick(10), None)
            .unwrap();
        queue
            .enqueue(actor(2), ActionDefId(5), Tick(11), None)
            .unwrap();
        queue.promote_head(Tick(20), NonZeroU32::new(3).unwrap());

        assert!(queue.has_actor(actor(1)));
        assert!(queue.has_actor(actor(2)));
        assert!(!queue.has_actor(actor(3)));

        assert!(queue.remove_actor(actor(1)));
        assert!(!queue.has_actor(actor(1)));
        assert!(queue.remove_actor(actor(2)));
        assert!(!queue.has_actor(actor(2)));
        assert!(!queue.remove_actor(actor(3)));
    }

    #[test]
    fn promote_head_grants_oldest_waiter_with_expiry() {
        let mut queue = ContentionQueue::default();
        queue
            .enqueue(actor(1), ActionDefId(4), Tick(10), None)
            .unwrap();
        queue
            .enqueue(actor(2), ActionDefId(5), Tick(11), None)
            .unwrap();

        let granted = queue.promote_head(Tick(20), NonZeroU32::new(3).unwrap());

        assert_eq!(
            granted,
            Some(&ContentionGrant {
                actor: actor(1),
                intended_action: ActionDefId(4),
                granted_at: Tick(20),
                expires_at: Tick(23),
            })
        );
        assert_eq!(queue.position_of(actor(2)), Some(0));
    }

    #[test]
    fn grant_expired_tracks_expiry_boundary() {
        let mut queue = ContentionQueue::default();
        queue
            .enqueue(actor(1), ActionDefId(4), Tick(10), None)
            .unwrap();
        queue.promote_head(Tick(20), NonZeroU32::new(3).unwrap());

        assert!(!queue.grant_expired(Tick(22)));
        assert!(queue.grant_expired(Tick(23)));
    }

    #[test]
    fn enqueue_rejects_when_waiter_limit_reached() {
        let mut queue = ContentionQueue::default();
        queue
            .enqueue(actor(1), ActionDefId(4), Tick(10), Some(1))
            .unwrap();

        assert_eq!(
            queue.enqueue(actor(2), ActionDefId(5), Tick(11), Some(1)),
            Err(ContentionError::QueueFull)
        );
    }

    #[test]
    fn enqueue_rejects_in_race_mode_when_no_waiters_allowed() {
        let mut queue = ContentionQueue {
            granted: Some(ContentionGrant {
                actor: actor(1),
                intended_action: ActionDefId(4),
                granted_at: Tick(10),
                expires_at: Tick(15),
            }),
            ..ContentionQueue::default()
        };

        assert_eq!(
            queue.enqueue(actor(2), ActionDefId(5), Tick(11), Some(0)),
            Err(ContentionError::QueueFull)
        );
    }

    #[test]
    fn contention_types_round_trip_through_bincode() {
        let policy = ContentionPolicy {
            grant_hold_ticks: NonZeroU32::new(4).unwrap(),
            auto_promote: true,
            max_waiters: Some(2),
        };
        let mut queue = ContentionQueue::default();
        queue
            .enqueue(actor(1), ActionDefId(4), Tick(10), policy.max_waiters)
            .unwrap();
        queue.promote_head(Tick(12), policy.grant_hold_ticks);

        let policy_roundtrip: ContentionPolicy =
            bincode::deserialize(&bincode::serialize(&policy).unwrap()).unwrap();
        let queue_roundtrip: ContentionQueue =
            bincode::deserialize(&bincode::serialize(&queue).unwrap()).unwrap();

        assert_eq!(policy_roundtrip, policy);
        assert_eq!(queue_roundtrip, queue);
    }

    #[test]
    fn contention_agent_side_types_round_trip_through_bincode() {
        let intents = ContentionIntents {
            intents: BTreeMap::from([(
                actor(7),
                QueuedContentionIntent {
                    goal_key: GoalKey::from(GoalKind::Sleep),
                    intended_action: ActionDefId(8),
                },
            )]),
        };
        let disposition = ContentionDispositionProfile {
            queue_patience_ticks: NonZeroU32::new(12),
        };

        let intents_roundtrip: ContentionIntents =
            bincode::deserialize(&bincode::serialize(&intents).unwrap()).unwrap();
        let disposition_roundtrip: ContentionDispositionProfile =
            bincode::deserialize(&bincode::serialize(&disposition).unwrap()).unwrap();

        assert_eq!(intents_roundtrip, intents);
        assert_eq!(disposition_roundtrip, disposition);
    }

    #[test]
    fn contention_status_defaults_to_unmanaged() {
        assert_eq!(ContentionStatus::default(), ContentionStatus::Unmanaged);
    }
}
