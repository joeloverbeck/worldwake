use crate::{
    ActionDef, ActionDefRegistry, ActionHandlerId, ActionInstance, ActionInstanceId, ActionPayload,
    ActionState, ActionStatus, BeliefView, DeterministicRng, Interruptibility, Precondition,
    TradeAcceptance,
};
use serde::{Deserialize, Serialize};
use worldwake_core::{ActionDefId, CommodityKind, EntityId, Quantity, World, WorldTxn};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommitOutcome {
    pub materializations: Vec<Materialization>,
}

impl CommitOutcome {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Materialization {
    pub tag: MaterializationTag,
    pub entity: EntityId,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum MaterializationTag {
    SplitOffLot,
}

pub type ActionStartFn = for<'w> fn(
    &ActionDef,
    &ActionInstance,
    &mut DeterministicRng,
    &mut WorldTxn<'w>,
) -> Result<Option<ActionState>, ActionError>;
pub type ActionTickFn = for<'w> fn(
    &ActionDef,
    &ActionInstance,
    &mut DeterministicRng,
    &mut WorldTxn<'w>,
) -> Result<ActionProgress, ActionError>;
pub type ActionCommitFn = for<'w> fn(
    &ActionDef,
    &ActionInstance,
    &mut DeterministicRng,
    &mut WorldTxn<'w>,
) -> Result<CommitOutcome, ActionError>;
pub type ActionAbortFn = for<'w> fn(
    &ActionDef,
    &ActionInstance,
    &AbortReason,
    &mut DeterministicRng,
    &mut WorldTxn<'w>,
) -> Result<(), ActionError>;
pub type AffordancePayloadFn =
    fn(&ActionDef, EntityId, &[EntityId], &dyn BeliefView) -> Vec<ActionPayload>;
pub type PayloadOverrideValidatorFn =
    fn(&ActionDef, EntityId, &[EntityId], &ActionPayload, &dyn BeliefView) -> bool;
pub type AuthoritativePayloadValidatorFn = fn(
    &ActionDef,
    &ActionDefRegistry,
    EntityId,
    &[EntityId],
    &ActionPayload,
    &World,
) -> Result<(), ActionError>;

#[derive(Copy, Clone)]
pub struct ActionHandler {
    pub on_start: ActionStartFn,
    pub on_tick: ActionTickFn,
    pub on_commit: ActionCommitFn,
    pub on_abort: ActionAbortFn,
    pub affordance_payloads: AffordancePayloadFn,
    pub payload_override_is_valid: PayloadOverrideValidatorFn,
    pub authoritative_payload_is_valid: AuthoritativePayloadValidatorFn,
}

impl ActionHandler {
    #[must_use]
    pub const fn new(
        on_start: ActionStartFn,
        on_tick: ActionTickFn,
        on_commit: ActionCommitFn,
        on_abort: ActionAbortFn,
    ) -> Self {
        Self {
            on_start,
            on_tick,
            on_commit,
            on_abort,
            affordance_payloads: no_affordance_payloads,
            payload_override_is_valid: no_payload_override_validator,
            authoritative_payload_is_valid: no_authoritative_payload_validator,
        }
    }

    #[must_use]
    pub const fn with_affordance_payloads(
        mut self,
        affordance_payloads: AffordancePayloadFn,
    ) -> Self {
        self.affordance_payloads = affordance_payloads;
        self
    }

    #[must_use]
    pub const fn with_payload_override_validator(
        mut self,
        payload_override_is_valid: PayloadOverrideValidatorFn,
    ) -> Self {
        self.payload_override_is_valid = payload_override_is_valid;
        self
    }

    #[must_use]
    pub const fn with_authoritative_payload_validator(
        mut self,
        authoritative_payload_is_valid: AuthoritativePayloadValidatorFn,
    ) -> Self {
        self.authoritative_payload_is_valid = authoritative_payload_is_valid;
        self
    }
}

fn no_affordance_payloads(
    _def: &ActionDef,
    _actor: EntityId,
    _targets: &[EntityId],
    _view: &dyn BeliefView,
) -> Vec<ActionPayload> {
    Vec::new()
}

fn no_payload_override_validator(
    _def: &ActionDef,
    _actor: EntityId,
    _targets: &[EntityId],
    _payload: &ActionPayload,
    _view: &dyn BeliefView,
) -> bool {
    false
}

#[allow(clippy::unnecessary_wraps)]
fn no_authoritative_payload_validator(
    _def: &ActionDef,
    _registry: &ActionDefRegistry,
    _actor: EntityId,
    _targets: &[EntityId],
    _payload: &ActionPayload,
    _world: &World,
) -> Result<(), ActionError> {
    Ok(())
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum ActionProgress {
    Continue,
    Complete,
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum ActionError {
    UnknownActionInstance(ActionInstanceId),
    UnknownActionDef(ActionDefId),
    UnknownActionHandler(ActionHandlerId),
    InvalidActionStatus {
        instance_id: ActionInstanceId,
        status: ActionStatus,
    },
    InterruptBlocked {
        instance_id: ActionInstanceId,
        interruptibility: Interruptibility,
    },
    ConstraintFailed(String),
    PreconditionFailed(String),
    ReservationUnavailable(EntityId),
    InvalidTarget(EntityId),
    AbortRequested(ActionAbortRequestReason),
    InternalError(String),
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum ActionAbortRequestReason {
    PayloadEntityMismatch {
        role: PayloadEntityRole,
        expected: EntityId,
        actual: EntityId,
    },
    ActorAlreadyHasCombatStance {
        actor: EntityId,
    },
    ActorNotPlaced {
        actor: EntityId,
    },
    TargetNotColocated {
        actor: EntityId,
        target: EntityId,
    },
    TargetNotDead {
        target: EntityId,
    },
    TargetNotAlive {
        target: EntityId,
    },
    TargetLacksWounds {
        target: EntityId,
    },
    TargetHasNoWounds {
        target: EntityId,
    },
    SelfTargetForbidden {
        actor: EntityId,
        action: SelfTargetActionKind,
    },
    ActorMissingWeaponCommodity {
        actor: EntityId,
        commodity: CommodityKind,
    },
    CommodityNotCombatWeapon {
        commodity: CommodityKind,
    },
    ActorMissingCombatProfile {
        actor: EntityId,
    },
    TargetMissingCombatProfile {
        target: EntityId,
    },
    HolderLacksAccessibleCommodity {
        holder: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    },
    TradeBundleRejected {
        participant: EntityId,
        acceptance: TradeAcceptance,
    },
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum PayloadEntityRole {
    Counterparty,
    Target,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum SelfTargetActionKind {
    Attack,
    Heal,
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum AbortReason {
    CommitConditionFailed {
        condition: Precondition,
    },
    Interrupted {
        kind: InterruptReason,
        detail: Option<String>,
    },
    ExternalAbort {
        kind: ExternalAbortReason,
        detail: Option<String>,
    },
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum InterruptReason {
    DangerNearby,
    Reprioritized,
    Other,
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum ExternalAbortReason {
    CancelledByInput { sequence_no: u64 },
    ActorMarkedDead,
    TargetDestroyed,
    HandlerRequested { reason: ActionAbortRequestReason },
    Other,
}

impl AbortReason {
    #[must_use]
    pub const fn commit_condition_failed(condition: Precondition) -> Self {
        Self::CommitConditionFailed { condition }
    }

    #[must_use]
    pub const fn interrupted(kind: InterruptReason) -> Self {
        Self::Interrupted { kind, detail: None }
    }

    #[must_use]
    pub fn interrupted_with_detail(kind: InterruptReason, detail: impl Into<String>) -> Self {
        Self::Interrupted {
            kind,
            detail: Some(detail.into()),
        }
    }

    #[must_use]
    pub const fn external_abort(kind: ExternalAbortReason) -> Self {
        Self::ExternalAbort { kind, detail: None }
    }

    #[must_use]
    pub fn external_abort_with_detail(
        kind: ExternalAbortReason,
        detail: impl Into<String>,
    ) -> Self {
        Self::ExternalAbort {
            kind,
            detail: Some(detail.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AbortReason, ActionAbortRequestReason, ActionError, ActionHandler, ActionProgress,
        CommitOutcome, ExternalAbortReason, InterruptReason, Materialization, MaterializationTag,
        PayloadEntityRole, SelfTargetActionKind,
    };
    use crate::{
        ActionDef, ActionDomain, ActionDuration, ActionHandlerId, ActionInstance, ActionInstanceId,
        ActionPayload, ActionState, ActionStatus, Constraint, DeterministicRng, DurationExpr,
        Interruptibility, Precondition, ReservationReq, TargetSpec,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use std::collections::BTreeSet;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, ActionDefId, BodyCostPerTick, CauseRef, ControlSource, EntityId,
        EventTag, ReservationId, Seed, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
    };

    fn sample_instance() -> ActionInstance {
        ActionInstance {
            instance_id: ActionInstanceId(9),
            def_id: ActionDefId(2),
            payload: ActionPayload::None,
            actor: EntityId {
                slot: 3,
                generation: 1,
            },
            targets: vec![EntityId {
                slot: 7,
                generation: 1,
            }],
            start_tick: Tick(12),
            remaining_duration: ActionDuration::Finite(3),
            status: ActionStatus::Active,
            reservation_ids: vec![ReservationId(5)],
            local_state: Some(ActionState::Empty),
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    fn sample_def() -> ActionDef {
        ActionDef {
            id: ActionDefId(2),
            name: "sample".to_string(),
            domain: ActionDomain::Generic,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: vec![TargetSpec::SpecificEntity(EntityId {
                slot: 7,
                generation: 1,
            })],
            preconditions: vec![Precondition::TargetExists(0)],
            reservation_requirements: vec![ReservationReq { target_index: 0 }],
            duration: DurationExpr::Fixed(NonZeroU32::new(2).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![Precondition::ActorAlive],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::from([EventTag::WorldMutation]),
            payload: ActionPayload::None,
            handler: ActionHandlerId(1),
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_start(
        _def: &ActionDef,
        _instance: &ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<Option<ActionState>, ActionError> {
        Ok(Some(ActionState::Empty))
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_tick(
        _def: &ActionDef,
        _instance: &ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<ActionProgress, ActionError> {
        Ok(ActionProgress::Continue)
    }

    fn create_agent_on_commit(
        _def: &ActionDef,
        _instance: &ActionInstance,
        _rng: &mut DeterministicRng,
        txn: &mut WorldTxn<'_>,
    ) -> Result<CommitOutcome, ActionError> {
        txn.create_agent("Aster", ControlSource::Ai)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
        Ok(CommitOutcome::empty())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_abort(
        _def: &ActionDef,
        _instance: &ActionInstance,
        _reason: &AbortReason,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    fn assert_copy_traits<T: Copy + Clone + Eq + Ord + std::hash::Hash + std::fmt::Debug>() {}

    fn assert_clone_traits<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn action_supporting_types_satisfy_required_traits() {
        assert_copy_traits::<ActionProgress>();
        assert_clone_traits::<ActionError>();
        assert_clone_traits::<AbortReason>();
        assert_clone_traits::<ActionAbortRequestReason>();
        assert_clone_traits::<CommitOutcome>();
    }

    #[test]
    fn commit_outcome_empty_has_no_materializations() {
        assert!(CommitOutcome::empty().materializations.is_empty());
    }

    #[test]
    fn commit_outcome_tracks_materializations() {
        let entity = EntityId {
            slot: 17,
            generation: 2,
        };
        let outcome = CommitOutcome {
            materializations: vec![Materialization {
                tag: MaterializationTag::SplitOffLot,
                entity,
            }],
        };

        assert_eq!(outcome.materializations.len(), 1);
        assert_eq!(
            outcome.materializations[0].tag,
            MaterializationTag::SplitOffLot
        );
        assert_eq!(outcome.materializations[0].entity, entity);
    }

    #[test]
    fn action_handler_hooks_are_callable() {
        let handler = ActionHandler::new(noop_start, noop_tick, create_agent_on_commit, noop_abort);
        let mut world = World::new(build_prototype_world()).unwrap();
        let instance = sample_instance();
        let def = sample_def();
        let mut rng = DeterministicRng::new(Seed([0x11; 32]));
        let mut txn = WorldTxn::new(
            &mut world,
            Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );

        assert_eq!(
            (handler.on_start)(&def, &instance, &mut rng, &mut txn).unwrap(),
            Some(ActionState::Empty)
        );
        assert_eq!(
            (handler.on_tick)(&def, &instance, &mut rng, &mut txn).unwrap(),
            ActionProgress::Continue
        );
        (handler.on_abort)(
            &def,
            &instance,
            &AbortReason::external_abort_with_detail(ExternalAbortReason::Other, "test"),
            &mut rng,
            &mut txn,
        )
        .unwrap();
    }

    #[test]
    fn abort_reason_helpers_preserve_structured_semantics_and_optional_detail() {
        assert_eq!(
            AbortReason::commit_condition_failed(Precondition::ActorAlive),
            AbortReason::CommitConditionFailed {
                condition: Precondition::ActorAlive,
            }
        );
        assert_eq!(
            AbortReason::interrupted(InterruptReason::DangerNearby),
            AbortReason::Interrupted {
                kind: InterruptReason::DangerNearby,
                detail: None,
            }
        );
        assert_eq!(
            AbortReason::interrupted_with_detail(InterruptReason::Other, "danger nearby"),
            AbortReason::Interrupted {
                kind: InterruptReason::Other,
                detail: Some("danger nearby".to_string()),
            }
        );
        assert_eq!(
            AbortReason::external_abort(ExternalAbortReason::ActorMarkedDead),
            AbortReason::ExternalAbort {
                kind: ExternalAbortReason::ActorMarkedDead,
                detail: None,
            }
        );
        assert_eq!(
            ActionError::AbortRequested(ActionAbortRequestReason::PayloadEntityMismatch {
                role: PayloadEntityRole::Target,
                expected: EntityId {
                    slot: 1,
                    generation: 0,
                },
                actual: EntityId {
                    slot: 2,
                    generation: 0,
                },
            }),
            ActionError::AbortRequested(ActionAbortRequestReason::PayloadEntityMismatch {
                role: PayloadEntityRole::Target,
                expected: EntityId {
                    slot: 1,
                    generation: 0,
                },
                actual: EntityId {
                    slot: 2,
                    generation: 0,
                },
            })
        );
        assert_eq!(
            ActionError::AbortRequested(ActionAbortRequestReason::SelfTargetForbidden {
                actor: EntityId {
                    slot: 9,
                    generation: 0,
                },
                action: SelfTargetActionKind::Heal,
            }),
            ActionError::AbortRequested(ActionAbortRequestReason::SelfTargetForbidden {
                actor: EntityId {
                    slot: 9,
                    generation: 0,
                },
                action: SelfTargetActionKind::Heal,
            })
        );
    }

    #[test]
    fn action_handler_on_commit_can_mutate_world_through_world_txn() {
        let handler = ActionHandler::new(noop_start, noop_tick, create_agent_on_commit, noop_abort);
        let mut world = World::new(build_prototype_world()).unwrap();
        let before = world
            .entities_of_kind(worldwake_core::EntityKind::Agent)
            .count();
        let instance = sample_instance();
        let def = sample_def();
        let mut rng = DeterministicRng::new(Seed([0x22; 32]));
        let mut txn = WorldTxn::new(
            &mut world,
            Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );

        let outcome = (handler.on_commit)(&def, &instance, &mut rng, &mut txn).unwrap();

        let after = txn.query_agent_data().count();
        assert_eq!(after, before + 1);
        assert_eq!(outcome, CommitOutcome::empty());
    }
}
