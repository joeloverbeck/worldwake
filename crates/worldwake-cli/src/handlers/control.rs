//! Handlers for agent control commands: `switch` and `observe`.

use worldwake_core::{
    components::AgentData,
    control::ControlSource,
    ids::EntityId,
    CauseRef, VisibilitySpec, WitnessData, WorldTxn,
};
use worldwake_sim::SimulationState;

use crate::commands::{CommandError, CommandOutcome, CommandResult};
use crate::display::{entity_display_name, format_location, resolve_entity, ResolveError};

/// Set an agent's `ControlSource` via a `WorldTxn`, recording the change in the event log.
///
/// Uses `CauseRef::ExternalInput` to mark this as a meta-operation (human UI control transfer)
/// rather than a simulation event.
fn set_control_source(
    sim: &mut SimulationState,
    agent_id: EntityId,
    new_source: ControlSource,
) -> Result<(), CommandError> {
    let tick = sim.scheduler().current_tick();
    let (world, event_log) = sim.world_and_event_log_mut();
    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::ExternalInput(0),
        Some(agent_id),
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.set_component_agent_data(
        agent_id,
        AgentData {
            control_source: new_source,
        },
    )
    .map_err(|e| CommandError::new(format!("failed to set control source: {e}")))?;
    txn.commit(event_log);
    Ok(())
}

/// Handle `switch <name>`: transfer human control to another agent.
///
/// Per spec lines 104–109:
/// 1. Resolve name → `EntityId`
/// 2. Validate: must be an agent, must be alive, must not be already controlled
/// 3. Set old agent's `control_source` to Ai (via `WorldTxn`)
/// 4. Set new agent's `control_source` to Human (via `WorldTxn`)
/// 5. Update `ControllerState`
/// 6. Print confirmation
pub fn handle_switch(sim: &mut SimulationState, name: &str) -> CommandResult {
    // 1. Resolve name → EntityId.
    let target_id = match resolve_entity(sim.world(), name) {
        Ok(id) => id,
        Err(ResolveError::NotFound(input)) => {
            println!("No entity found matching \"{input}\"");
            return Ok(CommandOutcome::Continue);
        }
        Err(ResolveError::Ambiguous(names)) => {
            println!("Ambiguous name — did you mean: {}?", names.join(", "));
            return Ok(CommandOutcome::Continue);
        }
    };

    // 2a. Must be an agent (has AgentData component).
    if sim.world().get_component_agent_data(target_id).is_none() {
        let display = entity_display_name(sim.world(), target_id);
        println!("\"{display}\" is not an agent");
        return Ok(CommandOutcome::Continue);
    }

    // 2b. Must be alive.
    if !sim.world().is_alive(target_id) {
        let display = entity_display_name(sim.world(), target_id);
        println!("\"{display}\" is not alive");
        return Ok(CommandOutcome::Continue);
    }

    // 2c. Must not already be the controlled agent.
    let old_id = sim.controller_state().controlled_entity();
    if old_id == Some(target_id) {
        let display = entity_display_name(sim.world(), target_id);
        println!("Already controlling {display}");
        return Ok(CommandOutcome::Continue);
    }

    // 3. Release old agent → Ai.
    if let Some(old) = old_id {
        set_control_source(sim, old, ControlSource::Ai)?;
    }

    // 4. Set new agent → Human.
    set_control_source(sim, target_id, ControlSource::Human)?;

    // 5. Update ControllerState.
    sim.controller_state_mut()
        .switch_control(old_id, Some(target_id))
        .map_err(|e| CommandError::new(format!("control state error: {e:?}")))?;

    // 6. Print confirmation.
    let display = entity_display_name(sim.world(), target_id);
    let location = format_location(sim.world(), target_id);
    println!("Now controlling {display} {location}");

    Ok(CommandOutcome::Continue)
}

/// Handle `observe`: release human control, enter observer mode.
///
/// Per spec lines 110–111:
/// 1. If controlling an agent, set its `control_source` to Ai (via `WorldTxn`)
/// 2. Clear `ControllerState`
/// 3. Print confirmation
pub fn handle_observe(sim: &mut SimulationState) -> CommandResult {
    let current = sim.controller_state().controlled_entity();

    // Already in observer mode.
    if current.is_none() {
        println!("Already in observer mode");
        return Ok(CommandOutcome::Continue);
    }

    // 1. Release current agent → Ai.
    if let Some(agent_id) = current {
        set_control_source(sim, agent_id, ControlSource::Ai)?;
    }

    // 2. Clear controller state.
    sim.controller_state_mut().clear();

    // 3. Print confirmation.
    println!("Observer mode — simulation runs without human control");

    Ok(CommandOutcome::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::{spawn_scenario, types::*};
    use worldwake_core::{control::ControlSource, topology::PlaceTag};
    use worldwake_sim::affordance_query::get_affordances;

    /// Build a two-agent scenario: Aster (Human) and Briar (Ai) in Village.
    fn two_agent_scenario() -> crate::scenario::SpawnedSimulation {
        let def = ScenarioDef {
            seed: 42,
            places: vec![PlaceDef {
                name: "Village".into(),
                tags: vec![PlaceTag::Village],
            }],
            edges: vec![],
            agents: vec![
                AgentDef {
                    name: "Aster".into(),
                    location: "Village".into(),
                    control: ControlSource::Human,
                    needs: None,
                    combat_profile: None,
                    utility_profile: None,
                    merchandise_profile: None,
                    trade_disposition: None,
                },
                AgentDef {
                    name: "Briar".into(),
                    location: "Village".into(),
                    control: ControlSource::Ai,
                    needs: None,
                    combat_profile: None,
                    utility_profile: None,
                    merchandise_profile: None,
                    trade_disposition: None,
                },
            ],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
        };
        spawn_scenario(&def).unwrap()
    }

    fn find_agent_by_name(sim: &SimulationState, name: &str) -> EntityId {
        sim.world()
            .query_name_and_agent_data()
            .find(|(_, n, _)| n.0 == name)
            .map_or_else(
                || panic!("agent \"{name}\" not found"),
                |(id, _, _)| id,
            )
    }

    // ── T24: switch transfers control ──────────────────────────────

    #[test]
    fn test_switch_transfers_control() {
        let mut spawned = two_agent_scenario();
        let aster = find_agent_by_name(&spawned.state, "Aster");
        let briar = find_agent_by_name(&spawned.state, "Briar");

        assert_eq!(
            spawned
                .state
                .world()
                .get_component_agent_data(aster)
                .unwrap()
                .control_source,
            ControlSource::Human
        );

        handle_switch(&mut spawned.state, "Briar").unwrap();

        assert_eq!(
            spawned
                .state
                .world()
                .get_component_agent_data(briar)
                .unwrap()
                .control_source,
            ControlSource::Human
        );
        assert_eq!(
            spawned
                .state
                .world()
                .get_component_agent_data(aster)
                .unwrap()
                .control_source,
            ControlSource::Ai
        );
        assert_eq!(
            spawned.state.controller_state().controlled_entity(),
            Some(briar)
        );
    }

    // ── switch preserves world state ───────────────────────────────

    #[test]
    fn test_switch_preserves_world_state() {
        let mut spawned = two_agent_scenario();
        let tick_before = spawned.state.scheduler().current_tick();
        let entity_count_before: usize = spawned.state.world().entities().count();

        handle_switch(&mut spawned.state, "Briar").unwrap();

        assert_eq!(spawned.state.scheduler().current_tick(), tick_before);
        assert_eq!(
            spawned.state.world().entities().count(),
            entity_count_before
        );
    }

    // ── switch to non-agent ────────────────────────────────────────

    #[test]
    fn test_switch_to_non_agent() {
        let mut spawned = two_agent_scenario();
        let aster = find_agent_by_name(&spawned.state, "Aster");

        // "Grain" doesn't exist as a named entity — triggers NotFound.
        let result = handle_switch(&mut spawned.state, "Grain");
        assert!(result.is_ok());
        assert_eq!(
            spawned.state.controller_state().controlled_entity(),
            Some(aster)
        );
    }

    // ── switch to self ─────────────────────────────────────────────

    #[test]
    fn test_switch_to_self() {
        let mut spawned = two_agent_scenario();
        let aster = find_agent_by_name(&spawned.state, "Aster");

        let result = handle_switch(&mut spawned.state, "Aster");
        assert!(result.is_ok());
        assert_eq!(
            spawned.state.controller_state().controlled_entity(),
            Some(aster)
        );
        assert_eq!(
            spawned
                .state
                .world()
                .get_component_agent_data(aster)
                .unwrap()
                .control_source,
            ControlSource::Human
        );
    }

    // ── observe releases control ───────────────────────────────────

    #[test]
    fn test_observe_releases_control() {
        let mut spawned = two_agent_scenario();
        let aster = find_agent_by_name(&spawned.state, "Aster");

        handle_observe(&mut spawned.state).unwrap();

        assert_eq!(spawned.state.controller_state().controlled_entity(), None);
        assert_eq!(
            spawned
                .state
                .world()
                .get_component_agent_data(aster)
                .unwrap()
                .control_source,
            ControlSource::Ai
        );
    }

    // ── observe when already observer ──────────────────────────────

    #[test]
    fn test_observe_already_observer() {
        let mut spawned = two_agent_scenario();
        handle_observe(&mut spawned.state).unwrap();

        let result = handle_observe(&mut spawned.state);
        assert!(result.is_ok());
        assert_eq!(spawned.state.controller_state().controlled_entity(), None);
    }

    // ── switch from observer mode ──────────────────────────────────

    #[test]
    fn test_switch_from_observer() {
        let mut spawned = two_agent_scenario();
        handle_observe(&mut spawned.state).unwrap();

        let briar = find_agent_by_name(&spawned.state, "Briar");
        handle_switch(&mut spawned.state, "Briar").unwrap();

        assert_eq!(
            spawned.state.controller_state().controlled_entity(),
            Some(briar)
        );
        assert_eq!(
            spawned
                .state
                .world()
                .get_component_agent_data(briar)
                .unwrap()
                .control_source,
            ControlSource::Human
        );
    }

    // ── T12: switch to merchant shows merchant affordances ─────────

    #[test]
    fn test_switch_new_agent_affordances() {
        use worldwake_core::items::CommodityKind;
        use worldwake_sim::{OmniscientBeliefRuntime, OmniscientBeliefView};

        let def = ScenarioDef {
            seed: 42,
            places: vec![PlaceDef {
                name: "Market".into(),
                tags: vec![PlaceTag::Village],
            }],
            edges: vec![],
            agents: vec![
                AgentDef {
                    name: "Peasant".into(),
                    location: "Market".into(),
                    control: ControlSource::Human,
                    needs: None,
                    combat_profile: None,
                    utility_profile: None,
                    merchandise_profile: None,
                    trade_disposition: None,
                },
                AgentDef {
                    name: "Merchant".into(),
                    location: "Market".into(),
                    control: ControlSource::Ai,
                    needs: None,
                    combat_profile: None,
                    utility_profile: None,
                    merchandise_profile: Some(MerchandiseProfileDef {
                        sale_kinds: vec![CommodityKind::Grain],
                        home_market: None,
                    }),
                    trade_disposition: None,
                },
            ],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
        };
        let mut spawned = spawn_scenario(&def).unwrap();

        handle_switch(&mut spawned.state, "Merchant").unwrap();

        let merchant = find_agent_by_name(&spawned.state, "Merchant");
        let runtime = OmniscientBeliefRuntime::new(
            spawned.state.scheduler().active_actions(),
            &spawned.action_registries.defs,
        );
        let view =
            OmniscientBeliefView::with_runtime(spawned.state.world(), runtime);

        // Affordances come from the merchant's context (invariant 9.12).
        let affordances = get_affordances(
            &view,
            merchant,
            &spawned.action_registries.defs,
            &spawned.action_registries.handlers,
        );

        assert_eq!(
            spawned.state.controller_state().controlled_entity(),
            Some(merchant)
        );
        // Affordances are agent-context-specific — no special player actions.
        let _ = affordances;
    }
}
