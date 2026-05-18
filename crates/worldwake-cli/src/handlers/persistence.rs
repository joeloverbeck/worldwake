use std::path::Path;

use worldwake_ai::AgentTickDriver;
use worldwake_sim::SimulationState;

use crate::commands::{CommandOutcome, CommandResult};
use crate::repl::ReplState;

/// Save current simulation state to a file.
#[allow(clippy::unnecessary_wraps)] // Must return CommandResult for dispatch interface.
pub fn handle_save(sim: &SimulationState, driver: &AgentTickDriver, path: &str) -> CommandResult {
    let file_path = Path::new(path);
    match worldwake_sim::save(sim, Some(driver), file_path) {
        Ok(()) => {
            println!("Saved to {path}");
            Ok(CommandOutcome::Continue)
        }
        Err(err) => {
            println!("Save failed: {err}");
            Ok(CommandOutcome::Continue)
        }
    }
}

/// Load simulation state from a file, replacing the current state.
///
/// On success, replaces `*sim` with the loaded state, restores the persisted
/// AI driver runtime when present, and clears stale REPL state (last
/// affordances).
///
/// On error, the current state is left unchanged.
#[allow(clippy::unnecessary_wraps)] // Must return CommandResult for dispatch interface.
pub fn handle_load(
    sim: &mut SimulationState,
    driver: &mut AgentTickDriver,
    repl_state: &mut ReplState,
    path: &str,
) -> CommandResult {
    let file_path = Path::new(path);
    match worldwake_sim::load(file_path) {
        Ok((loaded, runtime_bytes)) => {
            let tick = loaded.scheduler().current_tick();
            *sim = loaded;
            *driver = if let Some(bytes) = runtime_bytes.as_deref() {
                match AgentTickDriver::from_saved_runtime(bytes, sim.world()) {
                    Ok(driver) => driver,
                    Err(err) => {
                        println!("Load failed: {err}");
                        return Ok(CommandOutcome::Continue);
                    }
                }
            } else {
                AgentTickDriver::new()
            };
            repl_state.last_affordances.clear();
            println!("Loaded from {path} — tick {}", tick.0);
            Ok(CommandOutcome::Continue)
        }
        Err(err) => {
            println!("Load failed: {err}");
            Ok(CommandOutcome::Continue)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldwake_core::{
        CauseRef, CognitiveProfile, ControlSource, EventLog, ExecutionBudget, Seed, StateHash,
        Tick, VisibilitySpec, WitnessData, World, WorldTxn, build_prototype_world,
    };
    use worldwake_sim::SaveableRuntime;
    use worldwake_sim::{
        ControllerState, DeterministicRng, RecipeRegistry, ReplayRecordingConfig, ReplayState,
        Scheduler, SimulationState, SystemManifest,
    };

    fn build_test_sim() -> SimulationState {
        build_test_sim_with_profiles(None, None)
    }

    fn build_test_sim_with_profiles(
        cognitive_profile: Option<CognitiveProfile>,
        execution_budget: Option<ExecutionBudget>,
    ) -> SimulationState {
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
        let agent_id = txn.create_agent("TestAgent", ControlSource::Ai).unwrap();
        if let Some(cognitive_profile) = cognitive_profile {
            txn.set_component_cognitive_profile(agent_id, cognitive_profile)
                .unwrap();
        }
        if let Some(execution_budget) = execution_budget {
            txn.set_component_execution_budget(agent_id, execution_budget)
                .unwrap();
        }
        let _ = txn.commit(&mut event_log);

        let scheduler = Scheduler::new_with_tick(Tick(7), SystemManifest::canonical());
        let seed = Seed([0u8; 32]);
        let replay = ReplayState::new(
            StateHash([0u8; 32]),
            seed,
            Tick(7),
            ReplayRecordingConfig::disabled(),
        );
        let controller = ControllerState::with_entity(agent_id);
        let rng = DeterministicRng::new(seed);

        SimulationState::new(
            world,
            event_log,
            scheduler,
            RecipeRegistry::new(),
            replay,
            controller,
            rng,
        )
    }

    #[test]
    fn test_save_creates_file() {
        let sim = build_test_sim();
        let driver = AgentTickDriver::new();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.save");
        let path_str = path.to_str().unwrap();

        let result = handle_save(&sim, &driver, path_str);
        assert!(result.is_ok());
        assert!(path.exists(), "save file should exist");
    }

    #[test]
    fn test_save_load_roundtrip() {
        let sim = build_test_sim();
        let original_tick = sim.scheduler().current_tick();
        let original_entity_count = sim.world().entity_count();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("roundtrip.save");
        let path_str = path.to_str().unwrap();

        let saving_driver = AgentTickDriver::new();
        let expected_driver_bytes = saving_driver.save_runtime_state().unwrap();
        handle_save(&sim, &saving_driver, path_str).unwrap();

        let mut loaded_sim = build_test_sim(); // different instance
        let mut driver = AgentTickDriver::new();
        let mut repl_state = ReplState::new();

        handle_load(&mut loaded_sim, &mut driver, &mut repl_state, path_str).unwrap();

        assert_eq!(
            loaded_sim.scheduler().current_tick(),
            original_tick,
            "tick should match after roundtrip"
        );
        assert_eq!(
            loaded_sim.world().entity_count(),
            original_entity_count,
            "entity count should match after roundtrip"
        );
        assert_eq!(
            driver.save_runtime_state().unwrap(),
            expected_driver_bytes,
            "driver runtime bytes should match the saved driver payload after load"
        );
    }

    #[test]
    fn test_save_load_roundtrip_preserves_split_agent_profiles() {
        let cognitive_profile = CognitiveProfile {
            max_plan_depth: 12,
            snapshot_travel_horizon: 9,
            max_node_expansions: 1024,
            switch_margin: worldwake_core::Permille::new(175).unwrap(),
            ..CognitiveProfile::default()
        };
        let execution_budget = ExecutionBudget::new(
            16,
            ExecutionBudget::default().max_prerequisite_locations(),
            ExecutionBudget::default().preferred_operator_boost(),
        );
        let sim = build_test_sim_with_profiles(Some(cognitive_profile), Some(execution_budget));
        let agent = sim
            .controller_state()
            .controlled_entity()
            .expect("test sim should have a controlled agent");

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("roundtrip-reasoning.save");
        let path_str = path.to_str().unwrap();

        let driver = AgentTickDriver::new();
        handle_save(&sim, &driver, path_str).unwrap();

        let mut loaded_sim = build_test_sim();
        let mut loaded_driver = AgentTickDriver::new();
        let mut repl_state = ReplState::new();
        handle_load(
            &mut loaded_sim,
            &mut loaded_driver,
            &mut repl_state,
            path_str,
        )
        .unwrap();

        assert_eq!(
            loaded_sim.world().get_component_cognitive_profile(agent),
            Some(&cognitive_profile),
            "agent cognitive profile should persist across save/load"
        );
        assert_eq!(
            loaded_sim.world().get_component_execution_budget(agent),
            Some(&execution_budget),
            "agent execution budget should persist across save/load"
        );
    }

    #[test]
    fn test_load_nonexistent_file() {
        let mut sim = build_test_sim();
        let original_tick = sim.scheduler().current_tick();
        let mut driver = AgentTickDriver::new();
        let mut repl_state = ReplState::new();

        let result = handle_load(
            &mut sim,
            &mut driver,
            &mut repl_state,
            "/tmp/nonexistent_worldwake.save",
        );
        assert!(
            result.is_ok(),
            "load should return Ok (error is printed, not propagated)"
        );
        assert_eq!(
            sim.scheduler().current_tick(),
            original_tick,
            "state should be unchanged after failed load"
        );
    }

    #[test]
    fn test_load_invalid_file() {
        let mut sim = build_test_sim();
        let original_tick = sim.scheduler().current_tick();
        let mut driver = AgentTickDriver::new();
        let mut repl_state = ReplState::new();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("garbage.save");
        std::fs::write(&path, b"this is not a valid save file").unwrap();
        let path_str = path.to_str().unwrap();

        let result = handle_load(&mut sim, &mut driver, &mut repl_state, path_str);
        assert!(
            result.is_ok(),
            "load should return Ok (error is printed, not propagated)"
        );
        assert_eq!(
            sim.scheduler().current_tick(),
            original_tick,
            "state should be unchanged after invalid file load"
        );
    }

    #[test]
    fn test_load_clears_repl_state() {
        let sim = build_test_sim();
        let driver = AgentTickDriver::new();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("clear_repl.save");
        let path_str = path.to_str().unwrap();

        handle_save(&sim, &driver, path_str).unwrap();

        let mut loaded_sim = build_test_sim();
        let mut driver = AgentTickDriver::new();
        let mut repl_state = ReplState::new();

        // Simulate stale affordances by adding a dummy entry
        repl_state.last_affordances.push(worldwake_sim::Affordance {
            def_id: worldwake_core::ActionDefId(0),
            actor: worldwake_core::EntityId {
                slot: 1,
                generation: 1,
            },
            bound_targets: vec![],
            payload_override: None,
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        });
        assert!(!repl_state.last_affordances.is_empty());

        handle_load(&mut loaded_sim, &mut driver, &mut repl_state, path_str).unwrap();

        assert!(
            repl_state.last_affordances.is_empty(),
            "last_affordances should be cleared after load"
        );
    }
}
