use std::collections::BTreeSet;
use worldwake_core::{BodyCostPerTick, EventTag, VisibilitySpec, WorldTxn};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefId, ActionDefRegistry, ActionError, ActionHandler,
    ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress,
    ActionState, Constraint, DurationExpr, Interruptibility, Precondition,
};

pub fn register_defend_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(ActionHandler::new(
        start_defend,
        tick_defend,
        commit_defend,
        abort_defend,
    ));
    defs.register(defend_action_def(ActionDefId(defs.len() as u32), handler))
}

fn defend_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "defend".to_string(),
        actor_constraints: vec![
            Constraint::ActorAlive,
            Constraint::ActorNotDead,
            Constraint::ActorNotIncapacitated,
            Constraint::ActorNotInTransit,
            Constraint::ActorHasControl,
        ],
        targets: Vec::new(),
        preconditions: vec![Precondition::ActorAlive],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::Indefinite,
        body_cost_per_tick: BodyCostPerTick::zero(),
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: vec![Precondition::ActorAlive],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::ActionStarted]),
        payload: ActionPayload::None,
        handler,
    }
}

#[allow(clippy::unnecessary_wraps)]
fn start_defend(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    Ok(None)
}

#[allow(clippy::unnecessary_wraps)]
fn tick_defend(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

#[allow(clippy::unnecessary_wraps)]
fn commit_defend(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_defend(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _reason: &AbortReason,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::register_defend_action;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, CauseRef, CombatProfile, ControlSource, DeadAt, EventLog,
        Permille, Tick, VisibilitySpec, WitnessData, World, WorldTxn, WoundList,
    };
    use worldwake_sim::{
        abort_action, get_affordances, start_action, tick_action, ActionDuration,
        ActionError, ActionExecutionAuthority, ActionExecutionContext, ActionHandlerRegistry,
        ActionInstanceId, ActionPayload, ActionStatus, Affordance, DurationExpr,
        Interruptibility, OmniscientBeliefView, TickOutcome,
    };

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        )
    }

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    fn defend_profile() -> CombatProfile {
        CombatProfile::new(
            pm(1000),
            pm(700),
            pm(600),
            pm(550),
            pm(75),
            pm(20),
            pm(15),
            pm(120),
            pm(30),
            nz(6),
        )
    }

    fn spawn_guard(world: &mut World, tick: u64, control: ControlSource) -> worldwake_core::EntityId {
        let place = world.topology().place_ids().next().unwrap();
        let actor = {
            let mut txn = new_txn(world, tick);
            let actor = txn.create_agent("Guard", control).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_component_combat_profile(actor, defend_profile()).unwrap();
            txn.set_component_wound_list(actor, WoundList::default()).unwrap();
            commit_txn(txn);
            actor
        };
        actor
    }

    #[test]
    fn register_defend_action_creates_indefinite_public_defend_definition() {
        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let defend_id = register_defend_action(&mut defs, &mut handlers);
        let defend = defs.get(defend_id).unwrap();

        assert_eq!(defend.name, "defend");
        assert_eq!(defend.duration, DurationExpr::Indefinite);
        assert_eq!(defend.interruptibility, Interruptibility::FreelyInterruptible);
        assert_eq!(defend.visibility, VisibilitySpec::SamePlace);
        assert_eq!(defend.payload, ActionPayload::None);
        assert!(defend.targets.is_empty());
    }

    #[test]
    fn defend_affordance_starts_and_stays_active_until_cancelled() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let actor = spawn_guard(&mut world, 1, ControlSource::Ai);
        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let defend_id = register_defend_action(&mut defs, &mut handlers);
        let view = OmniscientBeliefView::new(&world);
        let affordance = get_affordances(&view, actor, &defs)
            .into_iter()
            .find(|affordance| affordance.def_id == defend_id)
            .unwrap();
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);

        let action_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(5),
            },
        )
        .unwrap();

        assert_eq!(
            active.get(&action_id).unwrap().remaining_duration,
            ActionDuration::Indefinite
        );
        assert_eq!(active.get(&action_id).unwrap().status, ActionStatus::Active);

        let outcome = tick_action(
            action_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(6),
            },
        )
        .unwrap();

        assert_eq!(outcome, TickOutcome::Continuing);
        assert_eq!(
            active.get(&action_id).unwrap().remaining_duration,
            ActionDuration::Indefinite
        );

        let replan = abort_action(
            action_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(7),
            },
            "cancel defend".to_string(),
        )
        .unwrap();

        assert_eq!(replan.agent, actor);
        assert!(!active.contains_key(&action_id));
    }

    #[test]
    fn defend_is_not_offered_to_dead_or_incapacitated_agents() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let dead = spawn_guard(&mut world, 1, ControlSource::Ai);
        let incapacitated = spawn_guard(&mut world, 2, ControlSource::Ai);
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_dead_at(dead, DeadAt(Tick(3))).unwrap();
            txn.set_component_wound_list(
                incapacitated,
                WoundList {
                    wounds: vec![worldwake_core::Wound {
                        body_part: worldwake_core::BodyPart::Torso,
                        cause: worldwake_core::WoundCause::Deprivation(
                            worldwake_core::DeprivationKind::Starvation,
                        ),
                        severity: pm(700),
                        inflicted_at: Tick(3),
                        bleed_rate_per_tick: pm(0),
                    }],
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let defend_id = register_defend_action(&mut defs, &mut handlers);

        let dead_affordances = get_affordances(&OmniscientBeliefView::new(&world), dead, &defs);
        let incapacitated_affordances =
            get_affordances(&OmniscientBeliefView::new(&world), incapacitated, &defs);

        assert!(!dead_affordances
            .iter()
            .any(|affordance| affordance.def_id == defend_id));
        assert!(!incapacitated_affordances
            .iter()
            .any(|affordance| affordance.def_id == defend_id));
    }

    #[test]
    fn defend_start_gate_rejects_dead_and_incapacitated_actors_authoritatively() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let dead = spawn_guard(&mut world, 1, ControlSource::Ai);
        let incapacitated = spawn_guard(&mut world, 2, ControlSource::Ai);
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_dead_at(dead, DeadAt(Tick(3))).unwrap();
            txn.set_component_wound_list(
                incapacitated,
                WoundList {
                    wounds: vec![worldwake_core::Wound {
                        body_part: worldwake_core::BodyPart::Torso,
                        cause: worldwake_core::WoundCause::Deprivation(
                            worldwake_core::DeprivationKind::Starvation,
                        ),
                        severity: pm(700),
                        inflicted_at: Tick(3),
                        bleed_rate_per_tick: pm(0),
                    }],
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        let mut defs = worldwake_sim::ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let defend_id = register_defend_action(&mut defs, &mut handlers);
        let mut active = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_id = ActionInstanceId(0);

        let dead_err = start_action(
            &Affordance {
                def_id: defend_id,
                actor: dead,
                bound_targets: Vec::new(),
                payload_override: None,
                explanation: None,
            },
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(5),
            },
        )
        .unwrap_err();
        assert_eq!(
            dead_err,
            ActionError::ConstraintFailed("ActorNotDead".to_string())
        );

        let incap_err = start_action(
            &Affordance {
                def_id: defend_id,
                actor: incapacitated,
                bound_targets: Vec::new(),
                payload_override: None,
                explanation: None,
            },
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(5),
            },
        )
        .unwrap_err();
        assert_eq!(
            incap_err,
            ActionError::ConstraintFailed("ActorNotIncapacitated".to_string())
        );
    }
}
