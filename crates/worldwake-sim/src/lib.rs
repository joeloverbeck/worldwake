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
