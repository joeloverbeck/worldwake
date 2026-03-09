//! # worldwake-sim
//!
//! Event log, action framework, scheduler, and replay engine.
//! Depends on `worldwake-core`.

pub mod action_def;
pub mod action_def_registry;
pub mod action_handler;
pub mod action_handler_registry;
pub mod action_ids;
pub mod action_instance;
pub mod action_semantics;
pub mod action_state;
pub mod action_status;
pub mod affordance;
pub mod affordance_query;
pub mod knowledge_view;
pub mod start_gate;
pub mod tick_action;
pub mod world_knowledge_view;

pub use action_def::ActionDef;
pub use action_def_registry::ActionDefRegistry;
pub use action_handler::{
    AbortReason, ActionAbortFn, ActionCommitFn, ActionError, ActionHandler, ActionProgress,
    ActionStartFn, ActionTickFn,
};
pub use action_handler_registry::ActionHandlerRegistry;
pub use action_ids::{ActionDefId, ActionHandlerId, ActionInstanceId};
pub use action_instance::ActionInstance;
pub use action_semantics::{
    Constraint, DurationExpr, Interruptibility, Precondition, ReservationReq, TargetSpec,
};
pub use action_state::ActionState;
pub use action_status::ActionStatus;
pub use affordance::Affordance;
pub use affordance_query::{
    enumerate_targets, evaluate_constraint, evaluate_precondition, get_affordances,
};
pub use knowledge_view::KnowledgeView;
pub use start_gate::{start_action, StartActionAuthority, StartActionContext};
pub use tick_action::{tick_action, TickActionAuthority, TickActionContext, TickOutcome};
pub use world_knowledge_view::WorldKnowledgeView;
