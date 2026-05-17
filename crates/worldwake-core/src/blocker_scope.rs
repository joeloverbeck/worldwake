use crate::{ActionDefId, BlockerKey, EntityId, GoalKey};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum BlockerScope {
    Exact(BlockerKey),
    RouteSegment(RouteSegment),
    Counterparty(EntityId),
}

impl BlockerScope {
    #[must_use]
    pub const fn exact(
        goal_key: GoalKey,
        place: Option<EntityId>,
        target: Option<EntityId>,
        action_def: Option<ActionDefId>,
    ) -> Self {
        Self::Exact(BlockerKey {
            goal_key,
            place,
            target,
            action_def,
        })
    }

    #[must_use]
    pub const fn exact_goal_key(&self) -> Option<GoalKey> {
        match self {
            Self::Exact(key) => Some(key.goal_key),
            Self::RouteSegment(_) | Self::Counterparty(_) => None,
        }
    }

    #[must_use]
    pub const fn exact_place(&self) -> Option<EntityId> {
        match self {
            Self::Exact(key) => key.place,
            Self::RouteSegment(_) | Self::Counterparty(_) => None,
        }
    }

    #[must_use]
    pub const fn exact_target(&self) -> Option<EntityId> {
        match self {
            Self::Exact(key) => key.target,
            Self::RouteSegment(_) | Self::Counterparty(_) => None,
        }
    }

    #[must_use]
    pub const fn exact_action_def(&self) -> Option<ActionDefId> {
        match self {
            Self::Exact(key) => key.action_def,
            Self::RouteSegment(_) | Self::Counterparty(_) => None,
        }
    }
}

impl From<BlockerKey> for BlockerScope {
    fn from(value: BlockerKey) -> Self {
        Self::Exact(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct RouteSegment {
    pub from: EntityId,
    pub to: EntityId,
}

impl RouteSegment {
    #[must_use]
    pub fn new(from: EntityId, to: EntityId) -> Self {
        if from <= to {
            Self { from, to }
        } else {
            Self { from: to, to: from }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BlockerScope, RouteSegment};
    use crate::{
        ActionDefId, BlockerKey, EntityId, GoalKey, GoalKind,
        test_utils::{entity_id, sample_goal_key},
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_copy_value_bounds<
        T: Copy + Clone + Eq + Ord + Debug + Serialize + DeserializeOwned,
    >() {
    }

    #[test]
    fn blocker_scope_variants_satisfy_required_bounds() {
        assert_copy_value_bounds::<BlockerScope>();
        assert_copy_value_bounds::<RouteSegment>();
    }

    #[test]
    fn route_segment_canonicalizes_endpoint_order() {
        let a = entity_id(10, 0);
        let b = entity_id(11, 0);

        assert_eq!(RouteSegment::new(a, b), RouteSegment::new(b, a));
    }

    #[test]
    fn blocker_scope_exact_accessors_expose_legacy_key_fields() {
        let target = entity_id(7, 0);
        let scope = BlockerScope::exact(
            GoalKey::from(GoalKind::Sleep),
            Some(entity_id(3, 0)),
            Some(target),
            Some(ActionDefId(9)),
        );

        assert_eq!(scope.exact_goal_key(), Some(GoalKey::from(GoalKind::Sleep)));
        assert_eq!(scope.exact_place(), Some(entity_id(3, 0)));
        assert_eq!(scope.exact_target(), Some(target));
        assert_eq!(scope.exact_action_def(), Some(ActionDefId(9)));
    }

    #[test]
    fn blocker_scope_non_exact_accessors_return_none() {
        let scope = BlockerScope::Counterparty(entity_id(4, 0));

        assert_eq!(scope.exact_goal_key(), None);
        assert_eq!(scope.exact_place(), None);
        assert_eq!(scope.exact_target(), None);
        assert_eq!(scope.exact_action_def(), None);
    }

    #[test]
    fn blocker_scope_variants_roundtrip_through_bincode() {
        let variants = [
            BlockerScope::Exact(BlockerKey {
                goal_key: sample_goal_key(),
                place: Some(entity_id(1, 0)),
                target: Some(entity_id(2, 0)),
                action_def: Some(ActionDefId(3)),
            }),
            BlockerScope::RouteSegment(RouteSegment::new(entity_id(8, 0), entity_id(9, 0))),
            BlockerScope::Counterparty(EntityId {
                slot: 12,
                generation: 0,
            }),
        ];

        for variant in variants {
            let bytes = bincode::serialize(&variant).unwrap();
            let roundtrip: BlockerScope = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, variant);
        }
    }
}
