use std::collections::BTreeSet;
use worldwake_core::{
    is_wound_load_fatal, BodyCostPerTick, CauseRef, ComponentDelta, ComponentKind, DeadAt,
    EvidenceRef, EventLog, EventTag, StateDelta, VisibilitySpec, WitnessData, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefId, ActionDefRegistry, ActionError, ActionHandler,
    ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress,
    ActionState, Constraint, DurationExpr, Interruptibility, Precondition, SystemError,
    SystemExecutionContext,
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

pub fn combat_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let SystemExecutionContext {
        world,
        event_log,
        rng: _rng,
        active_actions: _active_actions,
        action_defs: _action_defs,
        tick,
        system_id: _system_id,
    } = ctx;
    let fatalities = collect_fatalities(world, event_log, tick);

    for fatality in fatalities {
        let place = world.effective_place(fatality.entity);
        let mut txn = WorldTxn::new(
            world,
            tick,
            fatality.cause,
            Some(fatality.entity),
            place,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        txn.add_tag(EventTag::System)
            .add_tag(EventTag::WorldMutation)
            .add_tag(EventTag::Combat)
            .add_target(fatality.entity);
        txn.set_component_dead_at(fatality.entity, DeadAt(tick))
            .map_err(|error| SystemError::new(error.to_string()))?;
        let pending = txn.into_pending_event().with_evidence(fatality.evidence);
        let _ = event_log.emit(pending);
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Fatality {
    entity: worldwake_core::EntityId,
    evidence: Vec<EvidenceRef>,
    cause: CauseRef,
}

fn collect_fatalities(
    world: &worldwake_core::World,
    event_log: &EventLog,
    tick: worldwake_core::Tick,
) -> Vec<Fatality> {
    world
        .query_wound_list()
        .filter_map(|(entity, wounds)| {
            let profile = world.get_component_combat_profile(entity)?;
            if world.get_component_dead_at(entity).is_some() {
                return None;
            }
            is_wound_load_fatal(wounds, profile).then_some(Fatality {
                entity,
                evidence: wounds
                    .wound_ids()
                    .into_iter()
                    .map(|wound_id| EvidenceRef::Wound { entity, wound_id })
                    .collect(),
                cause: latest_wound_event(event_log, entity)
                    .map_or(CauseRef::SystemTick(tick), CauseRef::Event),
            })
        })
        .collect()
}

fn latest_wound_event(
    event_log: &EventLog,
    entity: worldwake_core::EntityId,
) -> Option<worldwake_core::EventId> {
    (0..event_log.len())
        .rev()
        .map(|index| worldwake_core::EventId(index as u64))
        .find(|event_id| {
            event_log.get(*event_id).is_some_and(|record| {
                record.state_deltas.iter().any(|delta| {
                    matches!(
                        delta,
                        StateDelta::Component(ComponentDelta::Set {
                            entity: changed,
                            component_kind: ComponentKind::WoundList,
                            ..
                        }) if *changed == entity
                    )
                })
            })
        })
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
    use super::{combat_system, register_defend_action};
    use crate::dispatch_table;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, CauseRef, CombatProfile, ControlSource, DeadAt, EvidenceRef,
        EventLog, EventTag, Permille, Quantity, Seed, Tick, VisibilitySpec, WitnessData, World,
        WorldTxn, WoundId, WoundList,
    };
    use worldwake_sim::{
        abort_action, get_affordances, start_action, tick_action, ActionDuration,
        ActionError, ActionExecutionAuthority, ActionExecutionContext, ActionHandlerRegistry,
        ActionInstanceId, ActionPayload, ActionStatus, Affordance, DeterministicRng,
        DurationExpr, Interruptibility, OmniscientBeliefView, SystemExecutionContext, SystemId,
        TickOutcome,
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
                        id: WoundId(1),
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
                        id: WoundId(1),
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

    #[test]
    fn combat_system_attaches_dead_at_and_emits_combat_event_for_fatal_wounds() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = spawn_guard(&mut world, 1, ControlSource::Ai);
        let place = world.topology().place_ids().next().unwrap();
        let mut log = EventLog::new();
        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_component_wound_list(
                guard,
                WoundList {
                    wounds: vec![worldwake_core::Wound {
                        id: WoundId(7),
                        body_part: worldwake_core::BodyPart::Head,
                        cause: worldwake_core::WoundCause::Deprivation(
                            worldwake_core::DeprivationKind::Starvation,
                        ),
                        severity: pm(1000),
                        inflicted_at: Tick(2),
                        bleed_rate_per_tick: pm(0),
                    }],
                },
            )
            .unwrap();
            let bread = txn
                .create_item_lot(worldwake_core::CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(bread, place).unwrap();
            txn.set_possessor(bread, guard).unwrap();
            let _ = txn.commit(&mut log);
        }
        let place_before = world.effective_place(guard);
        let mut rng = DeterministicRng::new(Seed([7; 32]));
        let active_actions = BTreeMap::new();
        let defs = worldwake_sim::ActionDefRegistry::new();

        combat_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &defs,
            tick: Tick(5),
            system_id: SystemId::Combat,
        })
        .unwrap();

        assert_eq!(world.get_component_dead_at(guard), Some(&DeadAt(Tick(5))));
        assert_eq!(world.effective_place(guard), place_before);
        assert_eq!(
            world.controlled_commodity_quantity(guard, worldwake_core::CommodityKind::Bread),
            Quantity(1)
        );
        assert!(!world.is_archived(guard));
        assert_eq!(log.events_by_tag(EventTag::Combat).len(), 1);
        let record = log.get(log.events_by_tag(EventTag::Combat)[0]).unwrap();
        assert_eq!(record.actor_id, Some(guard));
        assert!(matches!(record.cause, CauseRef::Event(_)));
        assert_eq!(
            record.evidence,
            vec![EvidenceRef::Wound {
                entity: guard,
                wound_id: WoundId(7),
            }]
        );
        assert!(record.tags.contains(&EventTag::System));
        assert!(record.tags.contains(&EventTag::WorldMutation));
    }

    #[test]
    fn combat_system_does_not_reemit_death_for_already_dead_agents() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = spawn_guard(&mut world, 1, ControlSource::Ai);
        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_component_wound_list(
                guard,
                WoundList {
                    wounds: vec![worldwake_core::Wound {
                        id: WoundId(8),
                        body_part: worldwake_core::BodyPart::Head,
                        cause: worldwake_core::WoundCause::Deprivation(
                            worldwake_core::DeprivationKind::Starvation,
                        ),
                        severity: pm(1000),
                        inflicted_at: Tick(2),
                        bleed_rate_per_tick: pm(0),
                    }],
                },
            )
            .unwrap();
            txn.set_component_dead_at(guard, DeadAt(Tick(3))).unwrap();
            commit_txn(txn);
        }
        let mut log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([5; 32]));
        let active_actions = BTreeMap::new();
        let defs = worldwake_sim::ActionDefRegistry::new();

        combat_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &defs,
            tick: Tick(6),
            system_id: SystemId::Combat,
        })
        .unwrap();

        assert_eq!(world.get_component_dead_at(guard), Some(&DeadAt(Tick(3))));
        assert!(log.is_empty());
    }

    #[test]
    fn dispatch_table_uses_combat_system_for_combat_slot() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = spawn_guard(&mut world, 1, ControlSource::Ai);
        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_component_wound_list(
                guard,
                WoundList {
                    wounds: vec![worldwake_core::Wound {
                        id: WoundId(9),
                        body_part: worldwake_core::BodyPart::Head,
                        cause: worldwake_core::WoundCause::Deprivation(
                            worldwake_core::DeprivationKind::Starvation,
                        ),
                        severity: pm(1000),
                        inflicted_at: Tick(2),
                        bleed_rate_per_tick: pm(0),
                    }],
                },
            )
            .unwrap();
            commit_txn(txn);
        }
        let systems = dispatch_table();
        let mut log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([11; 32]));
        let active_actions = BTreeMap::new();
        let defs = worldwake_sim::ActionDefRegistry::new();

        systems
            .get(SystemId::Combat)(SystemExecutionContext {
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
                active_actions: &active_actions,
                action_defs: &defs,
                tick: Tick(7),
                system_id: SystemId::Combat,
            })
            .unwrap();

        assert_eq!(world.get_component_dead_at(guard), Some(&DeadAt(Tick(7))));
        assert_eq!(log.events_by_tag(EventTag::Combat).len(), 1);
    }
}
