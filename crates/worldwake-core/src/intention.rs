//! Authoritative intention components stored on agents.
//!
//! These components represent causally relevant agent state that must survive
//! save/load: the active goal, multi-tick journey commitments, and facility
//! queue intents.

use crate::{ActionDefId, Component, EntityId, GoalKey, Tick};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Whether a journey commitment is actively being executed or temporarily
/// suspended (e.g., the agent is handling an urgent local need).
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum JourneyCommitmentState {
    #[default]
    Active,
    Suspended,
}

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

/// A multi-tick travel commitment that prevents goal thrashing during
/// multi-leg journeys. Persisted so that save/load mid-journey does not
/// cause the agent to re-evaluate and potentially abandon its route.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JourneyCommitment {
    /// The goal that motivated this journey.
    pub committed_goal: GoalKey,
    /// The terminal destination place entity.
    pub destination: EntityId,
    /// Whether the commitment is actively being executed or temporarily
    /// suspended.
    pub state: JourneyCommitmentState,
    /// The tick at which the journey commitment was established.
    pub established_at: Tick,
    /// The last tick at which the agent made forward travel progress.
    /// `None` if no progress has been recorded yet.
    pub last_progress_tick: Option<Tick>,
    /// How many consecutive ticks the current travel leg has been blocked.
    pub consecutive_blocked_leg_ticks: u32,
}

impl Component for JourneyCommitment {}

/// Per-agent record of which facilities the agent intends to use and what
/// action it plans to perform there.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct FacilityQueueIntents {
    /// Map from facility `EntityId` to the agent's queued intent.
    pub intents: BTreeMap<EntityId, QueuedFacilityIntent>,
}

impl Component for FacilityQueueIntents {}
