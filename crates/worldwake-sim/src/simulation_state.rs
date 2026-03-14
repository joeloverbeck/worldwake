use crate::{ControllerState, DeterministicRng, RecipeRegistry, ReplayState, Scheduler};
use serde::{Deserialize, Serialize};
use worldwake_core::{hash_serializable, CanonicalError, EventLog, StateHash, World};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimulationState {
    world: World,
    event_log: EventLog,
    scheduler: Scheduler,
    recipe_registry: RecipeRegistry,
    replay_state: ReplayState,
    controller_state: ControllerState,
    rng_state: DeterministicRng,
}

impl SimulationState {
    #[must_use]
    pub fn new(
        world: World,
        event_log: EventLog,
        scheduler: Scheduler,
        recipe_registry: RecipeRegistry,
        replay_state: ReplayState,
        controller_state: ControllerState,
        rng_state: DeterministicRng,
    ) -> Self {
        Self {
            world,
            event_log,
            scheduler,
            recipe_registry,
            replay_state,
            controller_state,
            rng_state,
        }
    }

    #[must_use]
    pub const fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    #[must_use]
    pub const fn event_log(&self) -> &EventLog {
        &self.event_log
    }

    pub fn event_log_mut(&mut self) -> &mut EventLog {
        &mut self.event_log
    }

    #[must_use]
    pub const fn scheduler(&self) -> &Scheduler {
        &self.scheduler
    }

    pub fn scheduler_mut(&mut self) -> &mut Scheduler {
        &mut self.scheduler
    }

    #[must_use]
    pub const fn recipe_registry(&self) -> &RecipeRegistry {
        &self.recipe_registry
    }

    pub fn recipe_registry_mut(&mut self) -> &mut RecipeRegistry {
        &mut self.recipe_registry
    }

    #[must_use]
    pub const fn replay_state(&self) -> &ReplayState {
        &self.replay_state
    }

    pub fn replay_state_mut(&mut self) -> &mut ReplayState {
        &mut self.replay_state
    }

    #[must_use]
    pub const fn controller_state(&self) -> &ControllerState {
        &self.controller_state
    }

    pub fn controller_state_mut(&mut self) -> &mut ControllerState {
        &mut self.controller_state
    }

    #[must_use]
    pub const fn rng_state(&self) -> &DeterministicRng {
        &self.rng_state
    }

    pub fn rng_state_mut(&mut self) -> &mut DeterministicRng {
        &mut self.rng_state
    }

    /// Borrow the world and event log mutably at the same time.
    ///
    /// This is necessary for constructing a `WorldTxn` (which borrows `&mut World`)
    /// and then committing it (which borrows `&mut EventLog`) from within a single
    /// `SimulationState`.
    pub fn world_and_event_log_mut(&mut self) -> (&mut World, &mut EventLog) {
        (&mut self.world, &mut self.event_log)
    }

    pub fn hash(&self) -> Result<StateHash, CanonicalError> {
        hash_serializable(self)
    }

    pub fn replay_bootstrap_hash(&self) -> Result<StateHash, CanonicalError> {
        Self::replay_bootstrap_hash_parts(
            &self.world,
            &self.event_log,
            &self.scheduler,
            &self.recipe_registry,
            &self.controller_state,
            &self.rng_state,
        )
    }

    pub(crate) fn replay_bootstrap_hash_parts(
        world: &World,
        event_log: &EventLog,
        scheduler: &Scheduler,
        recipe_registry: &RecipeRegistry,
        controller_state: &ControllerState,
        rng_state: &DeterministicRng,
    ) -> Result<StateHash, CanonicalError> {
        hash_serializable(&(
            world,
            event_log,
            scheduler,
            recipe_registry,
            controller_state,
            rng_state,
        ))
    }

    /// Split borrow: return mutable references to tick-relevant fields
    /// plus an immutable reference to `RecipeRegistry`.
    ///
    /// This exists because Rust cannot split borrows through individual
    /// `&mut self` accessor methods in a single expression.
    pub fn tick_parts_mut(
        &mut self,
    ) -> (
        &mut World,
        &mut EventLog,
        &mut Scheduler,
        &mut ControllerState,
        &mut DeterministicRng,
        &RecipeRegistry,
    ) {
        (
            &mut self.world,
            &mut self.event_log,
            &mut self.scheduler,
            &mut self.controller_state,
            &mut self.rng_state,
            &self.recipe_registry,
        )
    }

    pub(crate) fn runtime_parts_mut(
        &mut self,
    ) -> (
        &mut World,
        &mut EventLog,
        &mut Scheduler,
        &mut ControllerState,
        &mut DeterministicRng,
    ) {
        (
            &mut self.world,
            &mut self.event_log,
            &mut self.scheduler,
            &mut self.controller_state,
            &mut self.rng_state,
        )
    }

    #[cfg(test)]
    pub(crate) fn scheduler_and_replay_mut(&mut self) -> (&mut Scheduler, &mut ReplayState) {
        (&mut self.scheduler, &mut self.replay_state)
    }
}

#[cfg(test)]
mod tests {
    use super::SimulationState;
    use crate::{
        ControllerState, DeterministicRng, InputEvent, InputKind, RecipeDefinition, RecipeRegistry,
        ReplayCheckpoint, ReplayRecordingConfig, ReplayState, Scheduler, SystemManifest,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use std::num::NonZeroU64;
    use worldwake_core::{
        build_prototype_world, ActionDefId, BodyCostPerTick, CauseRef, CommodityKind,
        ControlSource, EntityId, EventLog, EventPayload, PendingEvent, Quantity, Seed,
        StateHash, Tick, UniqueItemKind, VisibilitySpec, WitnessData, WorkstationTag, World,
        WorldTxn,
    };

    fn assert_traits<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    const fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    const fn hash(byte: u8) -> StateHash {
        StateHash([byte; 32])
    }

    fn recordable_input(tick: u64, sequence_no: u64) -> InputEvent {
        InputEvent {
            scheduled_tick: Tick(tick),
            sequence_no,
            kind: InputKind::RequestAction {
                actor: entity(1),
                def_id: ActionDefId(7),
                targets: vec![entity(2)],
                payload_override: None,
                mode: crate::ActionRequestMode::Strict,
            },
        }
    }

    fn populated_world_and_event_log() -> (World, EventLog) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut event_log = EventLog::new();
        let mut txn = WorldTxn::new(
            &mut world,
            Tick(0),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::Hidden,
            WitnessData::default(),
        );
        let _ = txn
            .create_agent("simulation-root-agent", ControlSource::Ai)
            .unwrap();
        let _ = txn.commit(&mut event_log);
        (world, event_log)
    }

    fn populated_recipe_registry() -> RecipeRegistry {
        let mut registry = RecipeRegistry::new();
        registry.register(RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: std::num::NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: vec![UniqueItemKind::SimpleTool],
            body_cost_per_tick: BodyCostPerTick::zero(),
        });
        registry
    }

    fn populated_state() -> SimulationState {
        let (world, event_log) = populated_world_and_event_log();
        let mut scheduler = Scheduler::new_with_tick(Tick(4), SystemManifest::canonical());
        let _ = scheduler.input_queue_mut().enqueue(
            Tick(4),
            InputKind::SwitchControl {
                from: None,
                to: Some(entity(3)),
            },
        );

        let mut replay_state = ReplayState::new(
            hash(0x10),
            Seed([0x22; 32]),
            Tick(4),
            ReplayRecordingConfig::every(NonZeroU64::new(3).unwrap()),
        );
        replay_state.record_input(recordable_input(4, 0)).unwrap();
        replay_state
            .record_checkpoint(ReplayCheckpoint {
                tick: Tick(6),
                event_log_hash: hash(0x44),
                world_state_hash: hash(0x55),
            })
            .unwrap();

        let mut rng_state = DeterministicRng::new(Seed([0x33; 32]));
        let _ = rng_state.next_u32();

        SimulationState::new(
            world,
            event_log,
            scheduler,
            populated_recipe_registry(),
            replay_state,
            ControllerState::with_entity(entity(3)),
            rng_state,
        )
    }

    #[test]
    fn simulation_state_satisfies_required_traits() {
        assert_traits::<SimulationState>();
    }

    #[test]
    fn new_and_accessors_expose_owned_simulation_roots() {
        let state = populated_state();
        let (expected_world, expected_event_log) = populated_world_and_event_log();
        let mut expected_rng = DeterministicRng::new(Seed([0x33; 32]));
        let _ = expected_rng.next_u32();

        assert_eq!(state.world(), &expected_world);
        assert_eq!(state.event_log(), &expected_event_log);
        assert_eq!(state.scheduler().current_tick(), Tick(4));
        assert_eq!(state.recipe_registry().len(), 1);
        assert_eq!(state.replay_state().terminal_tick(), Tick(4));
        assert_eq!(state.replay_state().input_log(), &[recordable_input(4, 0)]);
        assert_eq!(
            state.controller_state().controlled_entity(),
            Some(entity(3))
        );
        assert_eq!(state.rng_state(), &expected_rng);
    }

    #[test]
    fn mutable_accessors_allow_in_place_updates() {
        let mut state = populated_state();

        state.scheduler_mut().increment_tick();
        let recipe_id = state.recipe_registry_mut().register(RecipeDefinition {
            name: "Rest".to_string(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            work_ticks: std::num::NonZeroU32::new(1).unwrap(),
            required_workstation_tag: None,
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::zero(),
        });
        state.controller_state_mut().clear();
        let _ = state.rng_state_mut().next_u32();
        state.replay_state_mut().set_terminal_tick(Tick(5)).unwrap();
        let _ = state.event_log_mut().emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(5),
            cause: CauseRef::SystemTick(Tick(5)),
            actor_id: None,
            target_ids: Vec::new(),
            evidence: Vec::new(),
            place_id: None,
            state_deltas: Vec::new(),
            observed_entities: std::collections::BTreeMap::new(),
            visibility: VisibilitySpec::Hidden,
            witness_data: WitnessData::default(),
            tags: std::collections::BTreeSet::from([worldwake_core::EventTag::System]),
        }));

        assert_eq!(state.scheduler().current_tick(), Tick(5));
        assert_eq!(recipe_id.0, 1);
        assert_eq!(state.recipe_registry().len(), 2);
        assert_eq!(state.controller_state().controlled_entity(), None);
        assert_eq!(state.replay_state().terminal_tick(), Tick(5));
        assert_eq!(state.event_log().len(), 2);
    }

    #[test]
    fn bincode_roundtrip_preserves_full_simulation_state() {
        let state = populated_state();

        let bytes = bincode::serialize(&state).unwrap();
        let roundtrip: SimulationState = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, state);
    }

    #[test]
    fn hash_is_stable_for_identical_states() {
        let left = populated_state();
        let right = left.clone();

        assert_eq!(left.hash().unwrap(), right.hash().unwrap());
    }

    #[test]
    fn replay_bootstrap_hash_ignores_replay_state_but_changes_for_runtime_roots() {
        let original = populated_state();

        let mut changed_replay = original.clone();
        changed_replay
            .replay_state_mut()
            .set_terminal_tick(Tick(9))
            .unwrap();

        let mut changed_rng = original.clone();
        let _ = changed_rng.rng_state_mut().next_u32();

        assert_eq!(
            original.replay_bootstrap_hash().unwrap(),
            changed_replay.replay_bootstrap_hash().unwrap()
        );
        assert_ne!(
            original.replay_bootstrap_hash().unwrap(),
            changed_rng.replay_bootstrap_hash().unwrap()
        );

        let mut changed_recipe_registry = original.clone();
        changed_recipe_registry
            .recipe_registry_mut()
            .register(RecipeDefinition {
                name: "Harvest Apples".to_string(),
                inputs: Vec::new(),
                outputs: vec![(CommodityKind::Apple, Quantity(2))],
                work_ticks: std::num::NonZeroU32::new(2).unwrap(),
                required_workstation_tag: Some(WorkstationTag::OrchardRow),
                required_tool_kinds: Vec::new(),
                body_cost_per_tick: BodyCostPerTick::zero(),
            });
        assert_ne!(
            original.replay_bootstrap_hash().unwrap(),
            changed_recipe_registry.replay_bootstrap_hash().unwrap()
        );
    }

    #[test]
    fn hash_changes_when_any_owned_field_changes() {
        let original = populated_state();

        let mut changed_world = original.clone();
        {
            let world = &mut changed_world.world;
            let event_log = &mut changed_world.event_log;
            let mut txn = WorldTxn::new(
                world,
                Tick(1),
                CauseRef::Bootstrap,
                None,
                None,
                VisibilitySpec::Hidden,
                WitnessData::default(),
            );
            let _ = txn.create_agent("world-change", ControlSource::Ai).unwrap();
            let _ = txn.commit(event_log);
        }

        let mut changed_event_log = original.clone();
        let _ = changed_event_log
            .event_log_mut()
            .emit(PendingEvent::from_payload(EventPayload {
                tick: Tick(7),
                cause: CauseRef::SystemTick(Tick(7)),
                actor_id: None,
                target_ids: Vec::new(),
                evidence: Vec::new(),
                place_id: None,
                state_deltas: Vec::new(),
                observed_entities: std::collections::BTreeMap::new(),
                visibility: VisibilitySpec::Hidden,
                witness_data: WitnessData::default(),
                tags: std::collections::BTreeSet::from([worldwake_core::EventTag::System]),
            }));

        let mut changed_scheduler = original.clone();
        changed_scheduler.scheduler_mut().increment_tick();

        let mut changed_recipe_registry = original.clone();
        changed_recipe_registry
            .recipe_registry_mut()
            .register(RecipeDefinition {
                name: "Harvest Apples".to_string(),
                inputs: Vec::new(),
                outputs: vec![(CommodityKind::Apple, Quantity(2))],
                work_ticks: std::num::NonZeroU32::new(2).unwrap(),
                required_workstation_tag: Some(WorkstationTag::OrchardRow),
                required_tool_kinds: Vec::new(),
                body_cost_per_tick: BodyCostPerTick::zero(),
            });

        let mut changed_replay = original.clone();
        changed_replay
            .replay_state_mut()
            .set_terminal_tick(Tick(5))
            .unwrap();

        let mut changed_controller = original.clone();
        changed_controller.controller_state_mut().clear();

        let mut changed_rng = original.clone();
        let _ = changed_rng.rng_state_mut().next_u32();

        let original_hash = original.hash().unwrap();
        assert_ne!(changed_world.hash().unwrap(), original_hash);
        assert_ne!(changed_event_log.hash().unwrap(), original_hash);
        assert_ne!(changed_scheduler.hash().unwrap(), original_hash);
        assert_ne!(changed_recipe_registry.hash().unwrap(), original_hash);
        assert_ne!(changed_replay.hash().unwrap(), original_hash);
        assert_ne!(changed_controller.hash().unwrap(), original_hash);
        assert_ne!(changed_rng.hash().unwrap(), original_hash);
    }
}
