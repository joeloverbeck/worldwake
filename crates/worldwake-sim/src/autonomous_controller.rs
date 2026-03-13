use crate::{
    ActionDefRegistry, ActionHandlerRegistry, CommittedAction, DeterministicRng, RecipeRegistry,
    ReplanNeeded, Scheduler, TickInputContext, TickInputError, TickInputProducer,
};
use worldwake_core::{ControlSource, EntityId, EventLog, Tick, World};

pub struct AutonomousControllerContext<'a> {
    pub world: &'a mut World,
    pub event_log: &'a mut EventLog,
    pub scheduler: &'a mut Scheduler,
    pub rng: &'a mut DeterministicRng,
    pub action_defs: &'a ActionDefRegistry,
    pub action_handlers: &'a ActionHandlerRegistry,
    pub recipe_registry: &'a RecipeRegistry,
    pub tick: Tick,
}

pub trait AutonomousController {
    fn name(&self) -> &'static str;

    fn claims_agent(&self, world: &World, agent: EntityId, control_source: ControlSource) -> bool;

    fn produce_agent_input(
        &mut self,
        ctx: AutonomousControllerContext<'_>,
        agent: EntityId,
        replan_signals: &[&ReplanNeeded],
        committed_actions: &[CommittedAction],
    ) -> Result<(), TickInputError>;
}

pub struct AutonomousControllerRuntime<'a> {
    controllers: Vec<&'a mut dyn AutonomousController>,
}

impl<'a> AutonomousControllerRuntime<'a> {
    #[must_use]
    pub fn new(controllers: Vec<&'a mut dyn AutonomousController>) -> Self {
        Self { controllers }
    }
}

impl TickInputProducer for AutonomousControllerRuntime<'_> {
    fn produce_inputs(&mut self, ctx: TickInputContext<'_>) -> Result<(), TickInputError> {
        let TickInputContext {
            world,
            event_log,
            scheduler,
            rng,
            action_defs,
            action_handlers,
            recipe_registry,
            pending_replans,
            tick,
        } = ctx;

        let agents = world
            .query_agent_data()
            .map(|(agent, data)| (agent, data.control_source))
            .collect::<Vec<_>>();

        for (agent, control_source) in agents {
            let matching = self
                .controllers
                .iter()
                .enumerate()
                .filter_map(|(index, controller)| {
                    controller
                        .claims_agent(world, agent, control_source)
                        .then_some(index)
                })
                .collect::<Vec<_>>();

            match matching.as_slice() {
                [] => {}
                [index] => {
                    let agent_replans = pending_replans
                        .iter()
                        .filter(|signal| signal.agent == agent)
                        .collect::<Vec<_>>();
                    let agent_commits = scheduler.take_committed_actions_for(agent);
                    self.controllers[*index].produce_agent_input(
                        AutonomousControllerContext {
                            world,
                            event_log,
                            scheduler,
                            rng,
                            action_defs,
                            action_handlers,
                            recipe_registry,
                            tick,
                        },
                        agent,
                        &agent_replans,
                        &agent_commits,
                    )?;
                }
                _ => {
                    let names = matching
                        .iter()
                        .map(|index| self.controllers[*index].name())
                        .collect::<Vec<_>>();
                    return Err(TickInputError::new(format!(
                        "multiple autonomous controllers claimed agent {agent}: {names:?}"
                    )));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{AutonomousController, AutonomousControllerContext, AutonomousControllerRuntime};
    use crate::{
        ActionDefRegistry, ActionHandlerRegistry, DeterministicRng, RecipeRegistry, Scheduler,
        SystemManifest, TickInputContext, TickInputProducer,
    };
    use worldwake_core::{
        build_prototype_world, ActionDefId, CauseRef, ControlSource, EntityId, EventLog, Seed,
        Tick, VisibilitySpec, WitnessData, World, WorldTxn,
    };

    struct RecordingController {
        name: &'static str,
        claimed_source: ControlSource,
        seen_agents: Vec<EntityId>,
        seen_replan_counts: Vec<usize>,
    }

    impl AutonomousController for RecordingController {
        fn name(&self) -> &'static str {
            self.name
        }

        fn claims_agent(
            &self,
            _world: &World,
            _agent: EntityId,
            control_source: ControlSource,
        ) -> bool {
            control_source == self.claimed_source
        }

        fn produce_agent_input(
            &mut self,
            _ctx: AutonomousControllerContext<'_>,
            agent: EntityId,
            replan_signals: &[&crate::ReplanNeeded],
            committed_actions: &[crate::CommittedAction],
        ) -> Result<(), crate::TickInputError> {
            self.seen_agents.push(agent);
            self.seen_replan_counts.push(replan_signals.len());
            assert!(committed_actions.is_empty());
            Ok(())
        }
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

    fn build_world_with_agents() -> (World, EntityId, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (ai_agent, human_agent) = {
            let mut txn = new_txn(&mut world, 1);
            let ai_agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let human_agent = txn.create_agent("Bram", ControlSource::Human).unwrap();
            txn.set_ground_location(ai_agent, place).unwrap();
            txn.set_ground_location(human_agent, place).unwrap();
            let _ = txn.commit(&mut EventLog::new());
            (ai_agent, human_agent)
        };
        (world, ai_agent, human_agent)
    }

    #[test]
    fn runtime_dispatches_only_claimed_agents_and_passes_agent_specific_replans() {
        let (mut world, ai_agent, human_agent) = build_world_with_agents();
        let mut event_log = EventLog::new();
        let mut scheduler = Scheduler::new(SystemManifest::canonical());
        scheduler.retain_replan(crate::ReplanNeeded {
            agent: ai_agent,
            failed_action_def: ActionDefId(2),
            failed_instance: crate::ActionInstanceId(3),
            reason: crate::AbortReason::external_abort(crate::ExternalAbortReason::Other),
            tick: Tick(0),
        });
        let mut rng = DeterministicRng::new(Seed([7; 32]));
        let defs = ActionDefRegistry::new();
        let handlers = ActionHandlerRegistry::new();
        let recipes = RecipeRegistry::new();
        let mut ai = RecordingController {
            name: "ai",
            claimed_source: ControlSource::Ai,
            seen_agents: Vec::new(),
            seen_replan_counts: Vec::new(),
        };
        let mut human = RecordingController {
            name: "human",
            claimed_source: ControlSource::Human,
            seen_agents: Vec::new(),
            seen_replan_counts: Vec::new(),
        };
        let mut runtime = AutonomousControllerRuntime::new(vec![&mut ai, &mut human]);

        let pending_replans = scheduler.pending_replans().to_vec();
        runtime
            .produce_inputs(TickInputContext {
                world: &mut world,
                event_log: &mut event_log,
                scheduler: &mut scheduler,
                rng: &mut rng,
                action_defs: &defs,
                action_handlers: &handlers,
                recipe_registry: &recipes,
                pending_replans: &pending_replans,
                tick: Tick(0),
            })
            .unwrap();

        assert_eq!(ai.seen_agents, vec![ai_agent]);
        assert_eq!(ai.seen_replan_counts, vec![1]);
        assert_eq!(human.seen_agents, vec![human_agent]);
        assert_eq!(human.seen_replan_counts, vec![0]);
    }

    #[test]
    fn runtime_rejects_overlapping_controller_claims() {
        let (mut world, ai_agent, _human_agent) = build_world_with_agents();
        let mut event_log = EventLog::new();
        let mut scheduler = Scheduler::new(SystemManifest::canonical());
        let mut rng = DeterministicRng::new(Seed([9; 32]));
        let defs = ActionDefRegistry::new();
        let handlers = ActionHandlerRegistry::new();
        let recipes = RecipeRegistry::new();
        let mut first = RecordingController {
            name: "first",
            claimed_source: ControlSource::Ai,
            seen_agents: Vec::new(),
            seen_replan_counts: Vec::new(),
        };
        let mut second = RecordingController {
            name: "second",
            claimed_source: ControlSource::Ai,
            seen_agents: Vec::new(),
            seen_replan_counts: Vec::new(),
        };
        let mut runtime = AutonomousControllerRuntime::new(vec![&mut first, &mut second]);

        let error = runtime
            .produce_inputs(TickInputContext {
                world: &mut world,
                event_log: &mut event_log,
                scheduler: &mut scheduler,
                rng: &mut rng,
                action_defs: &defs,
                action_handlers: &handlers,
                recipe_registry: &recipes,
                pending_replans: &[],
                tick: Tick(0),
            })
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            format!(
                "multiple autonomous controllers claimed agent {ai_agent}: [\"first\", \"second\"]"
            )
        );
    }
}
