//! Authoritative intention components stored on agents.
//!
//! These components represent causally relevant agent state that must survive
//! save/load: the active goal and facility queue intents.

use crate::{ActionDefId, Component, EntityId, GoalKey, Tick};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A facility use intention: which goal motivated the agent to queue for a
/// facility and what action it plans to perform there.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QueuedFacilityIntent {
    pub goal_key: GoalKey,
    pub intended_action: ActionDefId,
}

/// The agent's currently adopted goal intention.
///
/// Persisted through save/load so goal-switching margins and interrupt
/// thresholds are preserved across representation boundaries.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActiveGoal {
    /// The goal the agent is currently pursuing.
    pub goal_key: GoalKey,
    /// The tick at which this goal was adopted.
    pub adopted_at: Tick,
}

impl Component for ActiveGoal {}

/// Per-agent record of which facilities the agent intends to use and what
/// action it plans to perform there.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct FacilityQueueIntents {
    /// Map from facility `EntityId` to the agent's queued intent.
    pub intents: BTreeMap<EntityId, QueuedFacilityIntent>,
}

impl Component for FacilityQueueIntents {}
