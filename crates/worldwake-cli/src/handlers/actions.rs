//! Action command handlers: actions, do, cancel.

use worldwake_core::{EntityId, EntityKind, World};
use worldwake_sim::{
    ActionRequestMode, GoalBeliefView, InputKind, PerAgentBeliefRuntime, PerAgentBeliefView,
    SimulationState, get_affordances,
};
use worldwake_systems::ActionRegistries;

use crate::commands::{CommandError, CommandOutcome, CommandResult};
use crate::display::format_quantity;
use crate::repl::ReplState;

/// Action names filtered from the human action menu. Includes internal
/// operations, actions requiring complex payloads the CLI can't construct,
/// and enforcement actions that need institutional context.
pub const HIDDEN_ACTIONS: &[&str] = &[
    "store_stock",
    "collect_display_stock",
    "stage_stock_for_sale",
    "unstage_stock",
    "declare_support",
    "queue_for_facility_use",
    "staff_market",
    "steal",
    "fine",
    "exile",
];

/// List available actions for the controlled agent.
///
/// Queries affordances via `get_affordances()` (same query as AI agents),
/// stores them in `repl_state.last_affordances`, and prints a numbered menu.
pub fn handle_actions(
    sim: &SimulationState,
    registries: &ActionRegistries,
    repl_state: &mut ReplState,
) -> CommandResult {
    let entity = sim
        .controller_state()
        .controlled_entity()
        .ok_or_else(|| CommandError::new("no controlled agent"))?;

    let runtime = PerAgentBeliefRuntime::new(sim.scheduler().active_actions(), &registries.defs);
    let view = PerAgentBeliefView::with_runtime_from_world(entity, sim.world(), runtime);

    let mut affordances = get_affordances(&view, entity, &registries.defs, &registries.handlers);

    // Filter out self-targeting actions (attack self, fine self, exile self).
    affordances.retain(|a| !a.bound_targets.contains(&entity));

    // Filter out internal merchant operations not meaningful as user choices.
    affordances.retain(|a| {
        registries
            .defs
            .get(a.def_id)
            .is_none_or(|def| !HIDDEN_ACTIONS.contains(&def.name.as_str()))
    });

    // Remove duplicates that differ only in payload (keep first variant).
    affordances.dedup_by(|a, b| a.def_id == b.def_id && a.bound_targets == b.bound_targets);

    if affordances.is_empty() {
        println!("no actions available");
        repl_state.last_affordances.clear();
        return Ok(CommandOutcome::Continue);
    }

    println!("Available actions:");
    for (i, affordance) in affordances.iter().enumerate() {
        let action_name = registries
            .defs
            .get(affordance.def_id)
            .map_or("unknown", |def| def.name.as_str());

        let targets_str = if affordance.bound_targets.is_empty() {
            String::new()
        } else {
            let names: Vec<String> = affordance
                .bound_targets
                .iter()
                .map(|t| pov_target_label(&view, sim.world(), entity, *t))
                .collect();
            format!(" ({})", names.join(", "))
        };

        let duration_str = if let Some(def) = registries.defs.get(affordance.def_id) {
            format_duration_estimate(&def.duration)
        } else {
            String::new()
        };

        println!("  {}. {action_name}{targets_str}{duration_str}", i + 1);
    }

    repl_state.last_affordances = affordances;
    Ok(CommandOutcome::Continue)
}

fn pov_target_label(
    view: &PerAgentBeliefView<'_>,
    world: &World,
    actor: EntityId,
    target: EntityId,
) -> String {
    if let Some(place) = world.topology().place(target) {
        return place.name.clone();
    }

    if world.effective_place(actor).is_some()
        && world.effective_place(actor) == world.effective_place(target)
    {
        return directly_observed_physical_label(world, target)
            .unwrap_or_else(|| observed_kind_label(world.entity_kind(target)));
    }

    if let Some((_, belief)) = view
        .known_entity_beliefs(actor)
        .into_iter()
        .find(|(entity, _)| *entity == target)
    {
        return believed_target_label(world, &belief);
    }

    if let Some(place) = view.effective_place(target)
        && let Some(place_name) = world
            .topology()
            .place(place)
            .map(|place| place.name.as_str())
    {
        return match view.entity_kind(target) {
            Some(kind) => format!("{kind:?} last seen at {place_name}"),
            None => format!("last seen at {place_name}"),
        };
    }

    "unknown".to_string()
}

fn directly_observed_physical_label(world: &World, target: EntityId) -> Option<String> {
    world
        .get_component_item_lot(target)
        .map(|lot| format_quantity(lot.commodity, lot.quantity))
        .or_else(|| {
            world
                .get_component_workstation_marker(target)
                .map(|marker| format!("{:?}", marker.0))
        })
        .or_else(|| {
            world
                .get_component_resource_source(target)
                .map(|source| format!("{:?} source", source.commodity))
        })
}

fn believed_target_label(world: &World, belief: &worldwake_core::BelievedEntityState) -> String {
    if let Some(tag) = belief.workstation_tag {
        return format!("{tag:?}");
    }

    if let Some(source) = &belief.resource_source {
        return format!("{:?} source", source.commodity);
    }

    if let Some(place) = belief.last_known_place
        && let Some(place_name) = world
            .topology()
            .place(place)
            .map(|place| place.name.as_str())
    {
        return match belief.believed_kind {
            Some(kind) => format!("{kind:?} last seen at {place_name}"),
            None => format!("last seen at {place_name}"),
        };
    }

    observed_kind_label(belief.believed_kind)
}

fn observed_kind_label(kind: Option<EntityKind>) -> String {
    kind.map_or_else(|| "unknown".to_string(), |kind| format!("{kind:?}"))
}

/// Execute an action by menu number from the last `actions` output.
///
/// Creates an `InputKind::RequestAction` and enqueues it in the input queue.
/// The action won't execute until the next `tick`.
pub fn handle_do(
    n: usize,
    sim: &mut SimulationState,
    registries: &ActionRegistries,
    repl_state: &ReplState,
) -> CommandResult {
    let _entity = sim
        .controller_state()
        .controlled_entity()
        .ok_or_else(|| CommandError::new("no controlled agent"))?;

    if repl_state.last_affordances.is_empty() {
        return Err(CommandError::new(
            "run 'actions' first to see available actions",
        ));
    }

    if n == 0 || n > repl_state.last_affordances.len() {
        return Err(CommandError::new(
            "invalid action number, run 'actions' first",
        ));
    }

    let affordance = &repl_state.last_affordances[n - 1];

    let action_name = registries
        .defs
        .get(affordance.def_id)
        .map_or("unknown", |def| def.name.as_str());

    let tick = sim.scheduler().current_tick();
    sim.scheduler_mut().input_queue_mut().enqueue(
        tick,
        InputKind::RequestAction {
            actor: affordance.actor,
            def_id: affordance.def_id,
            targets: affordance.bound_targets.clone(),
            payload_override: affordance.payload_override.clone(),
            mode: ActionRequestMode::Strict,
            provenance: worldwake_sim::RequestProvenance::External,
        },
    );

    println!("Requested: {action_name}");
    Ok(CommandOutcome::Continue)
}

/// Cancel the current action for the controlled agent.
///
/// Creates an `InputKind::CancelAction` and enqueues it in the input queue.
pub fn handle_cancel(sim: &mut SimulationState) -> CommandResult {
    let entity = sim
        .controller_state()
        .controlled_entity()
        .ok_or_else(|| CommandError::new("no controlled agent"))?;

    // Find the active action for this agent.
    let active = sim
        .scheduler()
        .active_actions()
        .iter()
        .find(|(_, instance)| instance.actor == entity)
        .map(|(id, _)| *id);

    let Some(action_instance_id) = active else {
        println!("no action to cancel");
        return Ok(CommandOutcome::Continue);
    };

    let tick = sim.scheduler().current_tick();
    sim.scheduler_mut().input_queue_mut().enqueue(
        tick,
        InputKind::CancelAction {
            actor: entity,
            action_instance_id,
        },
    );

    println!("Cancel requested");
    Ok(CommandOutcome::Continue)
}

/// Format a duration estimate from a `DurationExpr` for display.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn format_duration_estimate(duration: &worldwake_sim::DurationExpr) -> String {
    use worldwake_sim::DurationExpr;
    match duration {
        DurationExpr::Fixed(n) => format!(" — {} ticks", n.get()),
        DurationExpr::Variable { min, max } => {
            format!(" — {}-{} ticks", min.get(), max.get())
        }
        DurationExpr::TravelToTarget { .. } | DurationExpr::EscortRouteTravel => {
            " — travel time".to_string()
        }
        DurationExpr::TargetConsumable { .. } => " — per unit".to_string(),
        DurationExpr::ActorMetabolism { .. }
        | DurationExpr::ActorTradeDisposition
        | DurationExpr::ActorMarketPresence
        | DurationExpr::ActorPatrolProfile
        | DurationExpr::ActorTheftDisposition
        | DurationExpr::ActorInvestigationDisposition
        | DurationExpr::ActorWitnessQueryDisposition
        | DurationExpr::BanditCampEstablishmentProfile
        | DurationExpr::ActorDefendStance
        | DurationExpr::CombatWeapon
        | DurationExpr::ConsultRecord { .. }
        | DurationExpr::TargetTreatment { .. } => " — varies".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repl::ReplState;
    use crate::scenario::{SpawnedSimulation, spawn_scenario, types::*};
    use worldwake_ai::AgentTickDriver;
    use worldwake_core::{
        ActionDefId, Tick,
        control::ControlSource,
        ids::EntityId,
        items::CommodityKind,
        needs::HomeostaticNeeds,
        numerics::{Permille, Quantity},
        topology::PlaceTag,
    };
    use worldwake_sim::{
        ActionDuration, ActionInstance, ActionInstanceId, ActionPayload, ActionStatus, Affordance,
        InputKind,
    };

    fn pm(v: u16) -> Permille {
        Permille::new(v).unwrap()
    }

    /// Scenario with a human agent at a village with food available (enables eat affordance).
    fn human_with_food_scenario() -> (SpawnedSimulation, EntityId) {
        let def = ScenarioDef {
            seed: 42,
            places: vec![PlaceDef {
                name: "Village".into(),
                tags: vec![PlaceTag::Village],
                visibility_profile: None,
                sleep_quality: None,
                place_dirtiness: None,
                latrine_fullness: None,
            }],
            edges: vec![],
            bandit_camps: Vec::new(),
            offices: vec![],
            artifacts: vec![],
            agents: vec![AgentDef {
                name: "Aster".into(),
                location: "Village".into(),
                control: ControlSource::Human,
                needs: Some(HomeostaticNeeds::new(
                    pm(600),
                    pm(600),
                    pm(600),
                    pm(600),
                    pm(600),
                )),
                combat_profile: None,
                utility_profile: None,
                artifact_posting_profile: None,
                merchandise_profile: None,
                trade_disposition: None,
                perception_profile: None,
                tell_profile: None,
                cognitive_profile: None,
                portfolio_weights_profile: None,
                agent_schema_context_profile: None,
                risk_weight_profile: None,
                law_abiding_profile: None,
                agenda_profile: None,
                execution_budget: None,
                epistemic_disposition: None,
                intention_disposition: None,
                communication_profile: None,
                preference_profile: None,
                expectation_store: None,
                last_seen_memory: None,
                social_observations: None,
                obligation_satiation_profile: None,
                drive_thresholds: None,
                drive_escalation_profile: None,
                metabolism_profile: None,
                disposal_profile: None,
                exploration_profile: None,
                diversification_profile: None,
                carry_capacity: None,
                theft_disposition: None,
                justice_disposition: None,
                violation_disposition: None,
                patrol_profile: None,
                patrol_route: None,
                pursuit_profile: None,
                contention_disposition: None,
                commodity_valuation: None,
                substitute_preferences: None,
                testimony_trust_profile: None,
                route_preference_profile: None,
                archetype: None,
                known_recipes: None,
            }],
            items: vec![ItemDef {
                commodity: CommodityKind::Apple,
                quantity: Quantity(5),
                location: "Aster".into(),
                container: false,
            }],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],

            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: std::collections::BTreeMap::new(),
            harvest_trace_retention_ticks: None,
            archetype_assignment_policy: None,
        };
        let spawned = spawn_scenario(&def).unwrap();
        let agent_id = spawned
            .state
            .world()
            .entities_with_name_and_agent_data()
            .next()
            .unwrap();
        (spawned, agent_id)
    }

    /// Scenario with an observer (no controlled agent).
    fn observer_scenario() -> SpawnedSimulation {
        let def = ScenarioDef {
            seed: 42,
            places: vec![PlaceDef {
                name: "Village".into(),
                tags: vec![PlaceTag::Village],
                visibility_profile: None,
                sleep_quality: None,
                place_dirtiness: None,
                latrine_fullness: None,
            }],
            edges: vec![],
            agents: vec![AgentDef {
                name: "Kael".into(),
                location: "Village".into(),
                control: ControlSource::Ai,
                needs: None,
                combat_profile: None,
                utility_profile: None,
                artifact_posting_profile: None,
                merchandise_profile: None,
                trade_disposition: None,
                perception_profile: None,
                tell_profile: None,
                cognitive_profile: None,
                portfolio_weights_profile: None,
                agent_schema_context_profile: None,
                risk_weight_profile: None,
                law_abiding_profile: None,
                agenda_profile: None,
                execution_budget: None,
                epistemic_disposition: None,
                intention_disposition: None,
                communication_profile: None,
                preference_profile: None,
                expectation_store: None,
                last_seen_memory: None,
                social_observations: None,
                obligation_satiation_profile: None,
                drive_thresholds: None,
                drive_escalation_profile: None,
                metabolism_profile: None,
                disposal_profile: None,
                exploration_profile: None,
                diversification_profile: None,
                carry_capacity: None,
                theft_disposition: None,
                justice_disposition: None,
                violation_disposition: None,
                patrol_profile: None,
                patrol_route: None,
                pursuit_profile: None,
                contention_disposition: None,
                commodity_valuation: None,
                substitute_preferences: None,
                testimony_trust_profile: None,
                route_preference_profile: None,
                archetype: None,
                known_recipes: None,
            }],
            bandit_camps: Vec::new(),
            offices: vec![],
            artifacts: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],

            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: std::collections::BTreeMap::new(),
            harvest_trace_retention_ticks: None,
            archetype_assignment_policy: None,
        };
        spawn_scenario(&def).unwrap()
    }

    fn two_place_pov_scenario(last_seen: bool) -> (SpawnedSimulation, EntityId, EntityId) {
        let def = ScenarioDef {
            seed: 42,
            places: vec![
                PlaceDef {
                    name: "Village".into(),
                    tags: vec![PlaceTag::Village],
                    visibility_profile: None,
                    sleep_quality: None,
                    place_dirtiness: None,
                    latrine_fullness: None,
                },
                PlaceDef {
                    name: "Market".into(),
                    tags: vec![PlaceTag::Store],
                    visibility_profile: None,
                    sleep_quality: None,
                    place_dirtiness: None,
                    latrine_fullness: None,
                },
            ],
            edges: vec![EdgeDef {
                from: "Village".into(),
                to: "Market".into(),
                travel_ticks: 1,
                bidirectional: true,
            }],
            agents: vec![
                AgentDef {
                    name: "Bram".into(),
                    location: "Market".into(),
                    control: ControlSource::Ai,
                    needs: None,
                    combat_profile: None,
                    utility_profile: None,
                    artifact_posting_profile: None,
                    merchandise_profile: None,
                    trade_disposition: None,
                    perception_profile: None,
                    tell_profile: None,
                    cognitive_profile: None,
                    portfolio_weights_profile: None,
                    agent_schema_context_profile: None,
                    risk_weight_profile: None,
                    law_abiding_profile: None,
                    agenda_profile: None,
                    execution_budget: None,
                    epistemic_disposition: None,
                    intention_disposition: None,
                    communication_profile: None,
                    preference_profile: None,
                    expectation_store: None,
                    last_seen_memory: None,
                    social_observations: None,
                    obligation_satiation_profile: None,
                    drive_thresholds: None,
                    drive_escalation_profile: None,
                    metabolism_profile: None,
                    disposal_profile: None,
                    exploration_profile: None,
                    diversification_profile: None,
                    carry_capacity: None,
                    theft_disposition: None,
                    justice_disposition: None,
                    violation_disposition: None,
                    patrol_profile: None,
                    patrol_route: None,
                    pursuit_profile: None,
                    contention_disposition: None,
                    commodity_valuation: None,
                    substitute_preferences: None,
                    testimony_trust_profile: None,
                    route_preference_profile: None,
                    archetype: None,
                    known_recipes: None,
                },
                AgentDef {
                    name: "Aster".into(),
                    location: "Village".into(),
                    control: ControlSource::Human,
                    needs: None,
                    combat_profile: None,
                    utility_profile: None,
                    artifact_posting_profile: None,
                    merchandise_profile: None,
                    trade_disposition: None,
                    perception_profile: None,
                    tell_profile: None,
                    cognitive_profile: None,
                    portfolio_weights_profile: None,
                    agent_schema_context_profile: None,
                    risk_weight_profile: None,
                    law_abiding_profile: None,
                    agenda_profile: None,
                    execution_budget: None,
                    epistemic_disposition: None,
                    intention_disposition: None,
                    communication_profile: None,
                    preference_profile: None,
                    expectation_store: None,
                    last_seen_memory: last_seen.then(|| LastSeenMemoryDef {
                        records: vec![LastSeenRecordDef {
                            subject: "Bram".into(),
                            place: "Market".into(),
                            observed_kind: Some(EntityKind::Agent),
                            observed_tick: 3,
                            source: "Bram".into(),
                            provenance: LastSeenProvenanceDef::DirectObservation,
                        }],
                        capacity: 20,
                    }),
                    social_observations: None,
                    obligation_satiation_profile: None,
                    drive_thresholds: None,
                    drive_escalation_profile: None,
                    metabolism_profile: None,
                    disposal_profile: None,
                    exploration_profile: None,
                    diversification_profile: None,
                    carry_capacity: None,
                    theft_disposition: None,
                    justice_disposition: None,
                    violation_disposition: None,
                    patrol_profile: None,
                    patrol_route: None,
                    pursuit_profile: None,
                    contention_disposition: None,
                    commodity_valuation: None,
                    substitute_preferences: None,
                    testimony_trust_profile: None,
                    route_preference_profile: None,
                    archetype: None,
                    known_recipes: None,
                },
            ],
            bandit_camps: Vec::new(),
            offices: vec![],
            artifacts: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],

            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: std::collections::BTreeMap::new(),
            harvest_trace_retention_ticks: None,
            archetype_assignment_policy: None,
        };
        let spawned = spawn_scenario(&def).unwrap();
        let (actor, remote) = {
            let named = spawned
                .state
                .world()
                .query_name()
                .map(|(entity, name)| (name.0.as_str(), entity))
                .collect::<std::collections::BTreeMap<_, _>>();
            (*named.get("Aster").unwrap(), *named.get("Bram").unwrap())
        };
        (spawned, actor, remote)
    }

    fn view_for(spawned: &SpawnedSimulation, actor: EntityId) -> PerAgentBeliefView<'_> {
        let runtime = PerAgentBeliefRuntime::new(
            spawned.state.scheduler().active_actions(),
            &spawned.action_registries.defs,
        );
        PerAgentBeliefView::with_runtime_from_world(actor, spawned.state.world(), runtime)
    }

    fn filtered_menu_affordances(spawned: &SpawnedSimulation, actor: EntityId) -> Vec<Affordance> {
        let view = view_for(spawned, actor);
        let mut affordances = get_affordances(
            &view,
            actor,
            &spawned.action_registries.defs,
            &spawned.action_registries.handlers,
        );
        affordances.retain(|a| !a.bound_targets.contains(&actor));
        affordances.retain(|a| {
            spawned
                .action_registries
                .defs
                .get(a.def_id)
                .is_none_or(|def| !HIDDEN_ACTIONS.contains(&def.name.as_str()))
        });
        affordances.dedup_by(|a, b| a.def_id == b.def_id && a.bound_targets == b.bound_targets);
        affordances
    }

    fn source_between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
        let start_index = source
            .find(start)
            .unwrap_or_else(|| panic!("missing source marker: {start}"));
        let remaining = &source[start_index..];
        let end_index = remaining
            .find(end)
            .unwrap_or_else(|| panic!("missing source marker: {end}"));
        &remaining[..end_index]
    }

    #[test]
    fn action_menu_play_surface_avoids_omniscient_display_helpers() {
        let source = include_str!("actions.rs");
        let handle_actions_body =
            source_between(source, "pub fn handle_actions", "\nfn pov_target_label");
        let handle_do_body = source_between(source, "pub fn handle_do", "\n/// Cancel");

        for forbidden in ["entity_display_name", "resolve_entity", "format_location"] {
            assert!(
                !handle_actions_body.contains(forbidden),
                "handle_actions must not call omniscient display helper {forbidden}"
            );
            assert!(
                !handle_do_body.contains(forbidden),
                "handle_do must not call omniscient display helper {forbidden}"
            );
        }
    }

    #[test]
    fn pov_target_label_uses_local_physical_item_label() {
        let (spawned, actor) = human_with_food_scenario();
        let item = spawned
            .state
            .world()
            .entities_of_kind(EntityKind::ItemLot)
            .next()
            .unwrap();
        let view = view_for(&spawned, actor);

        assert_eq!(
            pov_target_label(&view, spawned.state.world(), actor, item),
            "5× Apple"
        );
    }

    #[test]
    fn pov_target_label_uses_last_seen_label_without_remote_name() {
        let (spawned, actor, remote) = two_place_pov_scenario(true);
        let view = view_for(&spawned, actor);

        let label = pov_target_label(&view, spawned.state.world(), actor, remote);

        assert_eq!(label, "Agent last seen at Market");
        assert!(!label.contains("Bram"));
    }

    #[test]
    fn pov_target_label_hides_unknown_remote_name() {
        let (spawned, actor, remote) = two_place_pov_scenario(false);
        let view = view_for(&spawned, actor);

        let label = pov_target_label(&view, spawned.state.world(), actor, remote);

        assert_eq!(label, "unknown");
        assert!(!label.contains("Bram"));
    }

    #[test]
    fn test_actions_lists_affordances() {
        let (spawned, _agent_id) = human_with_food_scenario();
        let sim = spawned.state;
        let mut repl_state = ReplState::new();

        let result = handle_actions(&sim, &spawned.action_registries, &mut repl_state);
        assert_eq!(result.unwrap(), CommandOutcome::Continue);
        // Agent with food should have at least an eat affordance.
        assert!(
            !repl_state.last_affordances.is_empty(),
            "should have at least one affordance"
        );
    }

    #[test]
    fn test_actions_stores_in_repl_state() {
        let (spawned, _agent_id) = human_with_food_scenario();
        let sim = spawned.state;
        let mut repl_state = ReplState::new();

        assert!(repl_state.last_affordances.is_empty());
        let _ = handle_actions(&sim, &spawned.action_registries, &mut repl_state);
        assert!(!repl_state.last_affordances.is_empty());
    }

    #[test]
    fn test_do_enqueues_input() {
        let (spawned, _agent_id) = human_with_food_scenario();
        let mut sim = spawned.state;
        let mut repl_state = ReplState::new();

        // First populate affordances.
        let _ = handle_actions(&sim, &spawned.action_registries, &mut repl_state);
        assert!(!repl_state.last_affordances.is_empty());

        let queue_before = sim.scheduler().input_queue().len();
        let result = handle_do(1, &mut sim, &spawned.action_registries, &repl_state);
        assert_eq!(result.unwrap(), CommandOutcome::Continue);

        // Input queue should have one more event.
        assert_eq!(sim.scheduler().input_queue().len(), queue_before + 1);

        // Verify it's a RequestAction.
        let tick = sim.scheduler().current_tick();
        let events = sim.scheduler().input_queue().peek_tick(tick);
        let last = events.last().unwrap();
        assert!(
            matches!(last.kind, InputKind::RequestAction { .. }),
            "expected RequestAction, got {:?}",
            last.kind
        );
    }

    #[test]
    fn test_do_out_of_range() {
        let (spawned, _agent_id) = human_with_food_scenario();
        let mut sim = spawned.state;
        let mut repl_state = ReplState::new();

        let _ = handle_actions(&sim, &spawned.action_registries, &mut repl_state);
        let n = repl_state.last_affordances.len() + 1;

        let result = handle_do(n, &mut sim, &spawned.action_registries, &repl_state);
        let err = result.unwrap_err();
        assert!(err.message.contains("invalid action number"));
    }

    #[test]
    fn test_do_zero_out_of_range() {
        let (spawned, _agent_id) = human_with_food_scenario();
        let mut sim = spawned.state;
        let mut repl_state = ReplState::new();

        let _ = handle_actions(&sim, &spawned.action_registries, &mut repl_state);

        let result = handle_do(0, &mut sim, &spawned.action_registries, &repl_state);
        let err = result.unwrap_err();
        assert!(err.message.contains("invalid action number"));
    }

    #[test]
    fn test_do_before_actions() {
        let (spawned, _agent_id) = human_with_food_scenario();
        let mut sim = spawned.state;
        let repl_state = ReplState::new();

        let result = handle_do(1, &mut sim, &spawned.action_registries, &repl_state);
        let err = result.unwrap_err();
        assert!(err.message.contains("run 'actions' first"));
    }

    #[test]
    fn test_cancel_enqueues_input() {
        let (spawned, _agent_id) = human_with_food_scenario();
        let mut sim = spawned.state;
        let mut repl_state = ReplState::new();

        // Start an action first: get affordances, do one, then tick to start it.
        let _ = handle_actions(&sim, &spawned.action_registries, &mut repl_state);
        if repl_state.last_affordances.is_empty() {
            // No affordances — cancel should say "no action to cancel".
            let result = handle_cancel(&mut sim);
            assert_eq!(result.unwrap(), CommandOutcome::Continue);
            return;
        }

        let _ = handle_do(1, &mut sim, &spawned.action_registries, &repl_state);

        // Tick to process the request and start the action.
        let mut driver = AgentTickDriver::new();
        let _ = crate::handlers::tick::handle_tick(
            1,
            &mut sim,
            &mut driver,
            &spawned.action_registries,
            &spawned.dispatch_table,
        );

        // Now check if there's an active action for the agent.
        let entity = sim.controller_state().controlled_entity().unwrap();
        let has_active = sim
            .scheduler()
            .active_actions()
            .values()
            .any(|a| a.actor == entity);

        if has_active {
            let queue_before = sim.scheduler().input_queue().len();
            let result = handle_cancel(&mut sim);
            assert_eq!(result.unwrap(), CommandOutcome::Continue);
            assert_eq!(sim.scheduler().input_queue().len(), queue_before + 1);

            let tick = sim.scheduler().current_tick();
            let events = sim.scheduler().input_queue().peek_tick(tick);
            let last = events.last().unwrap();
            assert!(
                matches!(last.kind, InputKind::CancelAction { .. }),
                "expected CancelAction, got {:?}",
                last.kind
            );
        }
    }

    #[test]
    fn test_cancel_ignores_other_agents_active_action() {
        let (spawned, actor, remote) = two_place_pov_scenario(false);
        let mut sim = spawned.state;
        let other_action_id = ActionInstanceId(99);
        sim.scheduler_mut().insert_action(ActionInstance {
            instance_id: other_action_id,
            def_id: ActionDefId(0),
            payload: ActionPayload::None,
            actor: remote,
            targets: Vec::new(),
            start_tick: Tick(0),
            remaining_duration: ActionDuration::new(3),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
            body_cost_override: None,
        });

        assert_eq!(sim.controller_state().controlled_entity(), Some(actor));
        assert!(
            sim.scheduler()
                .active_actions()
                .contains_key(&other_action_id)
        );
        assert!(
            sim.scheduler()
                .active_actions()
                .values()
                .all(|instance| instance.actor != actor)
        );

        let queue_before = sim.scheduler().input_queue().len();
        let result = handle_cancel(&mut sim);

        assert_eq!(result.unwrap(), CommandOutcome::Continue);
        assert_eq!(sim.scheduler().input_queue().len(), queue_before);
        let tick = sim.scheduler().current_tick();
        assert!(
            sim.scheduler()
                .input_queue()
                .peek_tick(tick)
                .iter()
                .all(|event| !matches!(event.kind, InputKind::CancelAction { .. })),
            "cancel must not enqueue a CancelAction for another agent"
        );
        assert!(
            sim.scheduler()
                .active_actions()
                .contains_key(&other_action_id)
        );
    }

    #[test]
    fn action_menu_matches_ai_affordances_and_pov_labels() {
        let (spawned, actor) = human_with_food_scenario();
        let mut repl_state = ReplState::new();
        let expected_affordances = filtered_menu_affordances(&spawned, actor);

        let result = handle_actions(&spawned.state, &spawned.action_registries, &mut repl_state);

        assert_eq!(result.unwrap(), CommandOutcome::Continue);
        assert_eq!(repl_state.last_affordances, expected_affordances);

        let runtime = PerAgentBeliefRuntime::new(
            spawned.state.scheduler().active_actions(),
            &spawned.action_registries.defs,
        );
        let view =
            PerAgentBeliefView::with_runtime_from_world(actor, spawned.state.world(), runtime);
        for target in repl_state
            .last_affordances
            .iter()
            .flat_map(|affordance| affordance.bound_targets.iter().copied())
        {
            let label = pov_target_label(&view, spawned.state.world(), actor, target);
            assert_ne!(label, "Bram");
        }

        let (remote_spawned, remote_actor, remote_target) = two_place_pov_scenario(false);
        let remote_view = view_for(&remote_spawned, remote_actor);
        let remote_label = pov_target_label(
            &remote_view,
            remote_spawned.state.world(),
            remote_actor,
            remote_target,
        );
        assert_eq!(remote_label, "unknown");
        assert!(!remote_label.contains("Bram"));
    }

    #[test]
    fn test_actions_no_controlled_agent() {
        let spawned = observer_scenario();
        let sim = spawned.state;
        let mut repl_state = ReplState::new();

        let result = handle_actions(&sim, &spawned.action_registries, &mut repl_state);
        let err = result.unwrap_err();
        assert!(err.message.contains("no controlled agent"));
    }

    #[test]
    fn test_do_no_controlled_agent() {
        let spawned = observer_scenario();
        let mut sim = spawned.state;
        let repl_state = ReplState::new();

        let result = handle_do(1, &mut sim, &spawned.action_registries, &repl_state);
        let err = result.unwrap_err();
        assert!(err.message.contains("no controlled agent"));
    }

    #[test]
    fn test_cancel_no_controlled_agent() {
        let spawned = observer_scenario();
        let mut sim = spawned.state;

        let result = handle_cancel(&mut sim);
        let err = result.unwrap_err();
        assert!(err.message.contains("no controlled agent"));
    }
}
