//! Authoritative facility queue state and queue-domain dispositions.

use crate::{ActionDefId, Component, EntityId, Tick};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::num::NonZeroU32;

/// Authoritative queue/grant policy for exclusive-use facilities.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExclusiveFacilityPolicy {
    pub grant_hold_ticks: NonZeroU32,
}

impl Component for ExclusiveFacilityPolicy {}

/// Per-agent tolerance for waiting in a facility queue.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FacilityQueueDispositionProfile {
    pub queue_patience_ticks: Option<NonZeroU32>,
}

impl Component for FacilityQueueDispositionProfile {}

/// Stored queue state for a single exclusive-use facility.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct FacilityUseQueue {
    pub next_ordinal: u32,
    pub waiting: BTreeMap<u32, QueuedFacilityUse>,
    pub granted: Option<GrantedFacilityUse>,
}

impl Component for FacilityUseQueue {}

/// A queued request to perform one exclusive facility action.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QueuedFacilityUse {
    pub actor: EntityId,
    pub intended_action: ActionDefId,
    pub queued_at: Tick,
}

/// The currently active one-operation grant for a facility.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GrantedFacilityUse {
    pub actor: EntityId,
    pub intended_action: ActionDefId,
    pub granted_at: Tick,
    pub expires_at: Tick,
}

/// Typed queue-state errors for duplicate joins or exhausted ordinals.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FacilityQueueError {
    DuplicateActor(EntityId),
    OrdinalOverflow,
}

impl FacilityUseQueue {
    pub fn enqueue(
        &mut self,
        actor: EntityId,
        intended_action: ActionDefId,
        tick: Tick,
    ) -> Result<u32, FacilityQueueError> {
        if self.has_actor(actor) {
            return Err(FacilityQueueError::DuplicateActor(actor));
        }

        let ordinal = self.next_ordinal;
        self.next_ordinal = self
            .next_ordinal
            .checked_add(1)
            .ok_or(FacilityQueueError::OrdinalOverflow)?;
        self.waiting.insert(
            ordinal,
            QueuedFacilityUse {
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
    ) -> Option<&GrantedFacilityUse> {
        if self.granted.is_some() {
            return self.granted.as_ref();
        }

        let (&ordinal, queued) = self.waiting.iter().next()?;
        let granted = GrantedFacilityUse {
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
        ExclusiveFacilityPolicy, FacilityQueueDispositionProfile, FacilityQueueError,
        FacilityUseQueue, GrantedFacilityUse,
    };
    use crate::{
        test_utils::{entity_id, sample_facility_queue_disposition_profile},
        traits::Component,
        ActionDefId, Tick,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;
    use std::num::NonZeroU32;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn actor(slot: u32) -> crate::EntityId {
        entity_id(slot, 0)
    }

    #[test]
    fn queue_types_satisfy_component_and_value_bounds() {
        assert_component_bounds::<ExclusiveFacilityPolicy>();
        assert_component_bounds::<FacilityQueueDispositionProfile>();
        assert_component_bounds::<FacilityUseQueue>();
        assert_value_bounds::<ExclusiveFacilityPolicy>();
        assert_value_bounds::<FacilityQueueDispositionProfile>();
        assert_value_bounds::<FacilityUseQueue>();
        assert_value_bounds::<GrantedFacilityUse>();
    }

    #[test]
    fn enqueue_appends_and_returns_incrementing_ordinals() {
        let mut queue = FacilityUseQueue::default();

        assert_eq!(queue.enqueue(actor(1), ActionDefId(4), Tick(10)), Ok(0));
        assert_eq!(queue.enqueue(actor(2), ActionDefId(5), Tick(11)), Ok(1));
        assert_eq!(queue.next_ordinal, 2);
        assert_eq!(queue.waiting.len(), 2);
    }

    #[test]
    fn enqueue_rejects_duplicate_actor_membership() {
        let mut queue = FacilityUseQueue::default();

        queue.enqueue(actor(1), ActionDefId(4), Tick(10)).unwrap();

        assert_eq!(
            queue.enqueue(actor(1), ActionDefId(5), Tick(11)),
            Err(FacilityQueueError::DuplicateActor(actor(1)))
        );
    }

    #[test]
    fn position_of_is_zero_indexed_from_queue_head() {
        let mut queue = FacilityUseQueue::default();
        queue.enqueue(actor(1), ActionDefId(4), Tick(10)).unwrap();
        queue.enqueue(actor(2), ActionDefId(5), Tick(11)).unwrap();
        queue.enqueue(actor(3), ActionDefId(6), Tick(12)).unwrap();

        assert_eq!(queue.position_of(actor(1)), Some(0));
        assert_eq!(queue.position_of(actor(2)), Some(1));
        assert_eq!(queue.position_of(actor(3)), Some(2));
        assert_eq!(queue.position_of(actor(4)), None);
    }

    #[test]
    fn has_actor_and_remove_actor_cover_waiting_and_granted_entries() {
        let mut queue = FacilityUseQueue::default();
        queue.enqueue(actor(1), ActionDefId(4), Tick(10)).unwrap();
        queue.enqueue(actor(2), ActionDefId(5), Tick(11)).unwrap();
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
    fn promote_head_moves_head_into_granted_with_expiry() {
        let mut queue = FacilityUseQueue::default();
        queue.enqueue(actor(1), ActionDefId(4), Tick(10)).unwrap();
        queue.enqueue(actor(2), ActionDefId(5), Tick(11)).unwrap();

        let granted = queue
            .promote_head(Tick(20), NonZeroU32::new(3).unwrap())
            .unwrap();

        assert_eq!(granted.actor, actor(1));
        assert_eq!(granted.intended_action, ActionDefId(4));
        assert_eq!(granted.granted_at, Tick(20));
        assert_eq!(granted.expires_at, Tick(23));
        assert_eq!(queue.position_of(actor(2)), Some(0));
    }

    #[test]
    fn promote_head_returns_none_when_queue_is_empty() {
        let mut queue = FacilityUseQueue::default();
        assert_eq!(
            queue.promote_head(Tick(20), NonZeroU32::new(3).unwrap()),
            None
        );
    }

    #[test]
    fn clear_grant_and_grant_expiration_track_grant_state() {
        let mut queue = FacilityUseQueue::default();
        queue.enqueue(actor(1), ActionDefId(4), Tick(10)).unwrap();
        queue.promote_head(Tick(20), NonZeroU32::new(3).unwrap());

        assert!(!queue.grant_expired(Tick(22)));
        assert!(queue.grant_expired(Tick(23)));

        queue.clear_grant();
        assert_eq!(queue.granted, None);
        assert!(!queue.grant_expired(Tick(23)));
    }

    #[test]
    fn queue_domain_profiles_roundtrip_through_bincode() {
        let policy = ExclusiveFacilityPolicy {
            grant_hold_ticks: NonZeroU32::new(5).unwrap(),
        };
        let disposition = sample_facility_queue_disposition_profile();

        let policy_bytes = bincode::serialize(&policy).unwrap();
        let policy_roundtrip: ExclusiveFacilityPolicy =
            bincode::deserialize(&policy_bytes).unwrap();
        assert_eq!(policy_roundtrip, policy);

        let disposition_bytes = bincode::serialize(&disposition).unwrap();
        let disposition_roundtrip: FacilityQueueDispositionProfile =
            bincode::deserialize(&disposition_bytes).unwrap();
        assert_eq!(disposition_roundtrip, disposition);
    }
}
