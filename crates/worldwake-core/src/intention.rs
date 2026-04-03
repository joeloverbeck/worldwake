//! Authoritative intention components stored on agents.
//!
//! These components represent causally relevant agent state that must survive
//! save/load: the active goal.

use crate::{Component, GoalKey, Tick};
use serde::{Deserialize, Serialize};

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
