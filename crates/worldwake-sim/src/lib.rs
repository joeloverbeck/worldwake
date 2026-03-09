//! # worldwake-sim
//!
//! Event log, action framework, scheduler, and replay engine.
//! Depends on `worldwake-core`.

mod action_termination;

pub mod action_def;
pub mod action_def_registry;
pub mod action_execution;
pub mod action_handler;
pub mod action_handler_registry;
pub mod action_ids;
pub mod action_instance;
pub mod action_semantics;
pub mod action_state;
pub mod action_status;
pub mod affordance;
pub mod affordance_query;
pub mod controller_state;
pub mod deterministic_rng;
pub mod input_event;
pub mod input_queue;
pub mod interrupt_abort;
pub mod knowledge_view;
pub mod replay_state;
pub mod replay_execution;
pub mod replan_needed;
pub mod scheduler;
pub mod start_gate;
pub mod system_manifest;
pub mod system_dispatch;
pub mod tick_step;
pub mod tick_action;
pub mod world_knowledge_view;

pub use action_def::ActionDef;
pub use action_def_registry::ActionDefRegistry;
pub use action_execution::{ActionExecutionAuthority, ActionExecutionContext};
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
pub use controller_state::{ControlError, ControllerState};
pub use deterministic_rng::DeterministicRng;
pub use input_event::{InputEvent, InputKind};
pub use input_queue::{InputQueue, InputQueueError};
pub use interrupt_abort::{abort_action, interrupt_action};
pub use knowledge_view::KnowledgeView;
pub use replay_execution::{
    record_tick_checkpoint, replay_and_verify, seed_replay_inputs_from_scheduler,
    ReplayCheckpointError, ReplayError,
};
pub use replay_state::{ReplayCheckpoint, ReplayRecordingConfig, ReplayState, ReplayStateError};
pub use replan_needed::ReplanNeeded;
pub use scheduler::Scheduler;
pub use start_gate::start_action;
pub use system_manifest::{SystemId, SystemManifest, SystemManifestError};
pub use system_dispatch::{SystemDispatchTable, SystemError, SystemExecutionContext, SystemFn};
pub use tick_step::{step_tick, TickStepError, TickStepResult, TickStepServices};
pub use tick_action::{tick_action, TickOutcome};
pub use world_knowledge_view::WorldKnowledgeView;
