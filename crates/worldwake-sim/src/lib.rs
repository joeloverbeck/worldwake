//! # worldwake-sim
//!
//! Event log, action framework, scheduler, and replay engine.
//! Depends on `worldwake-core`.

mod action_termination;
mod action_validation;

pub mod action_def;
pub mod action_def_registry;
pub mod action_domain;
pub mod action_duration;
pub mod action_execution;
pub mod action_handler;
pub mod action_handler_registry;
pub mod action_ids;
pub mod action_instance;
pub mod action_payload;
pub mod action_semantics;
pub mod action_state;
pub mod action_status;
pub mod affordance;
pub mod affordance_query;
pub mod autonomous_controller;
pub mod belief_view;
pub mod controller_state;
pub mod deterministic_rng;
pub mod input_event;
pub mod input_queue;
pub mod interrupt_abort;
pub mod per_agent_belief_view;
pub mod recipe_def;
pub mod recipe_registry;
pub mod replan_needed;
pub mod replay_execution;
pub mod replay_state;
pub mod save_load;
pub mod scheduler;
pub mod simulation_state;
pub mod social_relay;
pub mod start_gate;
pub mod system_dispatch;
pub mod system_manifest;
pub mod tick_action;
pub mod tick_input_producer;
pub mod tick_step;
pub mod trade_valuation;

pub use action_def::ActionDef;
pub use action_def_registry::ActionDefRegistry;
pub use action_domain::ActionDomain;
pub use action_duration::ActionDuration;
pub use action_execution::{ActionExecutionAuthority, ActionExecutionContext};
pub use action_handler::{
    AbortReason, ActionAbortFn, ActionAbortRequestReason, ActionCommitFn, ActionError,
    ActionHandler, ActionProgress, ActionStartFn, ActionTickFn, AuthoritativePayloadValidatorFn,
    CommitOutcome, ExternalAbortReason, InterruptReason, Materialization, MaterializationTag,
    PayloadEntityRole, PayloadOverrideValidatorFn, SelfTargetActionKind,
};
pub use action_handler_registry::ActionHandlerRegistry;
pub use action_ids::{ActionHandlerId, ActionInstanceId};
pub use action_instance::ActionInstance;
pub use action_payload::{
    ActionPayload, BribeActionPayload, CombatActionPayload, CraftActionPayload,
    DeclareSupportActionPayload, HarvestActionPayload, LootActionPayload,
    QueueForFacilityUsePayload, TellActionPayload, ThreatenActionPayload, TradeActionPayload,
    TransportActionPayload,
};
pub use action_semantics::{
    Constraint, ConsumableEffect, DurationExpr, Interruptibility, MetabolismDurationKind,
    Precondition, ReservationReq, TargetSpec,
};
pub use action_state::ActionState;
pub use action_status::ActionStatus;
pub use action_validation::validate_action_def_authoritatively;
pub use affordance::Affordance;
pub use affordance_query::{
    evaluate_constraint, evaluate_precondition, get_affordances, requested_affordance_matches,
};
pub use autonomous_controller::{
    AutonomousController, AutonomousControllerContext, AutonomousControllerRuntime,
};
pub use belief_view::{estimate_duration_from_beliefs, GoalBeliefView, RuntimeBeliefView};
pub use controller_state::{ControlError, ControllerState};
pub use deterministic_rng::DeterministicRng;
pub use input_event::{ActionRequestMode, InputEvent, InputKind};
pub use input_queue::{InputQueue, InputQueueError};
pub use interrupt_abort::{abort_action, interrupt_action};
pub use per_agent_belief_view::{PerAgentBeliefRuntime, PerAgentBeliefView};
pub use recipe_def::RecipeDefinition;
pub use recipe_registry::RecipeRegistry;
pub use replan_needed::ReplanNeeded;
pub use replay_execution::{
    record_tick_checkpoint, replay_and_verify, seed_replay_inputs_from_scheduler,
    ReplayCheckpointError, ReplayError,
};
pub use replay_state::{ReplayCheckpoint, ReplayRecordingConfig, ReplayState, ReplayStateError};
pub use save_load::{
    load, load_from_bytes, save, save_to_bytes, SaveError, SAVE_FORMAT_VERSION, SAVE_MAGIC,
};
pub use scheduler::{CommittedAction, Scheduler, SchedulerActionRuntime};
pub use simulation_state::SimulationState;
pub use social_relay::{belief_chain_len, relayable_social_subjects};
pub use start_gate::start_action;
pub use system_dispatch::{SystemDispatchTable, SystemError, SystemExecutionContext, SystemFn};
pub use system_manifest::{SystemId, SystemManifest, SystemManifestError};
pub use tick_action::{tick_action, TickOutcome};
pub use tick_input_producer::{TickInputContext, TickInputError, TickInputProducer};
pub use tick_step::{step_tick, TickStepError, TickStepResult, TickStepServices};
pub use trade_valuation::{evaluate_trade_bundle, TradeAcceptance, TradeRejectionReason};
