use std::collections::BTreeMap;

use egui::Pos2;
use worldwake_ai::{AgentTickDriver, RankedGoalProvenance};
use worldwake_cli::display::entity_display_name;
use worldwake_core::{
    ActionDefId, ControlSource, DriveThresholds, EntityId, GoalKind, HomeostaticNeeds, Permille,
    PlaceTag, Tick, World,
};
use worldwake_sim::{ActionInstance, ActionState, Scheduler};

use crate::layout::PlaceLayout;

#[derive(Clone, Debug, PartialEq)]
pub struct FrameSnapshot {
    pub tick: Tick,
    pub places: BTreeMap<EntityId, PlaceView>,
    pub edges: Vec<EdgeView>,
    pub agents: BTreeMap<EntityId, AgentView>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlaceView {
    pub name: String,
    pub tags: Vec<PlaceTag>,
    pub position: Pos2,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EdgeView {
    pub from: EntityId,
    pub to: EntityId,
    pub travel_ticks: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentView {
    pub name: String,
    pub control: ControlSource,
    pub position: AgentPosition,
    pub alive: bool,
    pub active_action: Option<ActiveActionView>,
    pub active_goal: Option<CommittedGoalView>,
    pub needs: HomeostaticNeeds,
    pub drive_thresholds: DriveThresholds,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentPosition {
    AtPlace(EntityId),
    InTransit {
        from: EntityId,
        to: EntityId,
        progress: Permille,
        k_of_n: (u32, u32),
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActiveActionView {
    pub action_def_id: ActionDefId,
    pub ticks_in: u32,
    pub ticks_total: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedGoalView {
    pub goal_kind: GoalKind,
    pub motive_score: u32,
    pub provenance: Option<RankedGoalProvenance>,
}

#[must_use]
pub fn build_snapshot(
    world: &World,
    scheduler: &Scheduler,
    driver: &AgentTickDriver,
    layout: &PlaceLayout,
    current_tick: Tick,
) -> FrameSnapshot {
    FrameSnapshot {
        tick: current_tick,
        places: build_places(world, layout),
        edges: build_edges(world),
        agents: build_agents(world, scheduler, driver, current_tick),
    }
}

fn build_places(world: &World, layout: &PlaceLayout) -> BTreeMap<EntityId, PlaceView> {
    world
        .topology()
        .place_ids()
        .map(|id| {
            let place = world
                .topology()
                .place(id)
                .expect("place_ids only yields resolvable places");
            let position = *layout
                .positions
                .get(&id)
                .expect("layout must contain every topology place");
            (
                id,
                PlaceView {
                    name: place.name.clone(),
                    tags: place.tags.iter().copied().collect(),
                    position,
                },
            )
        })
        .collect()
}

fn build_edges(world: &World) -> Vec<EdgeView> {
    let topology = world.topology();
    let mut edges = Vec::new();
    for place in topology.place_ids() {
        for edge_id in topology.outgoing_edges(place) {
            let edge = topology
                .edge(*edge_id)
                .expect("outgoing edge IDs must resolve");
            edges.push(EdgeView {
                from: edge.from(),
                to: edge.to(),
                travel_ticks: edge.travel_time_ticks(),
            });
        }
    }
    edges
}

fn build_agents(
    world: &World,
    scheduler: &Scheduler,
    driver: &AgentTickDriver,
    current_tick: Tick,
) -> BTreeMap<EntityId, AgentView> {
    world
        .query_name_and_agent_data()
        .map(|(id, _, agent_data)| {
            let active_instance = scheduler
                .active_actions()
                .values()
                .find(|instance| instance.actor == id);
            (
                id,
                AgentView {
                    name: entity_display_name(world, id),
                    control: agent_data.control_source,
                    position: agent_position(world, active_instance, id, current_tick),
                    alive: world.get_component_dead_at(id).is_none(),
                    active_action: active_instance
                        .map(|instance| active_action_view(instance, current_tick)),
                    active_goal: active_goal_view(driver, id),
                    needs: world
                        .get_component_homeostatic_needs(id)
                        .copied()
                        .unwrap_or_default(),
                    drive_thresholds: world
                        .get_component_drive_thresholds(id)
                        .copied()
                        .unwrap_or_default(),
                },
            )
        })
        .collect()
}

fn agent_position(
    world: &World,
    active_instance: Option<&ActionInstance>,
    agent: EntityId,
    current_tick: Tick,
) -> AgentPosition {
    if let Some(ActionState::Travel {
        edge_id: _,
        origin,
        destination,
        departure_tick,
        arrival_tick,
    }) = active_instance.and_then(|instance| instance.local_state)
    {
        debug_assert!(
            current_tick.0 != 0,
            "agent cannot be in transit at tick 0 under scenario spawn invariants"
        );
        let (progress, k_of_n) = transit_progress(current_tick, departure_tick, arrival_tick);
        return AgentPosition::InTransit {
            from: origin,
            to: destination,
            progress,
            k_of_n,
        };
    }

    AgentPosition::AtPlace(
        world
            .effective_place(agent)
            .expect("snapshot agent must have an effective place when not in transit"),
    )
}

fn active_action_view(instance: &ActionInstance, current_tick: Tick) -> ActiveActionView {
    let ticks_in = tick_delta(current_tick, instance.start_tick);
    let ticks_total = ticks_in.saturating_add(instance.remaining_duration.ticks());
    ActiveActionView {
        action_def_id: instance.def_id,
        ticks_in,
        ticks_total,
    }
}

fn active_goal_view(driver: &AgentTickDriver, agent: EntityId) -> Option<CommittedGoalView> {
    driver
        .runtime(agent)?
        .agenda_state
        .committed
        .as_ref()
        .map(|entry| CommittedGoalView {
            goal_kind: entry.offer.key.kind,
            motive_score: entry.motive_score,
            provenance: entry.provenance.clone(),
        })
}

fn transit_progress(
    current_tick: Tick,
    departure_tick: Tick,
    arrival_tick: Tick,
) -> (Permille, (u32, u32)) {
    let elapsed = current_tick.0.saturating_sub(departure_tick.0);
    let total = arrival_tick.0.saturating_sub(departure_tick.0);
    let progress = if total == 0 {
        Permille::new_unchecked(1000)
    } else {
        let scaled = elapsed.saturating_mul(1000).saturating_add(total / 2) / total;
        Permille::new(scaled.min(1000) as u16).expect("progress is clamped to Permille range")
    };

    (
        progress,
        (u64_to_u32_saturating(elapsed), u64_to_u32_saturating(total)),
    )
}

fn tick_delta(current_tick: Tick, start_tick: Tick) -> u32 {
    u64_to_u32_saturating(current_tick.0.saturating_sub(start_tick.0))
}

fn u64_to_u32_saturating(value: u64) -> u32 {
    value.min(u64::from(u32::MAX)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldwake_cli::scenario::{load_scenario_file, spawn_scenario_ignoring_lints};

    #[test]
    fn transit_progress_three_of_seven() {
        let (progress, k_of_n) = transit_progress(Tick(103), Tick(100), Tick(107));

        assert_eq!(progress, Permille::new(429).unwrap());
        assert_eq!(k_of_n, (3, 7));
    }

    #[test]
    fn transit_progress_zero_duration_clamps_to_max() {
        let (progress, k_of_n) = transit_progress(Tick(100), Tick(100), Tick(100));

        assert_eq!(progress, Permille::new_unchecked(1000));
        assert_eq!(k_of_n, (0, 0));
    }

    #[test]
    fn snapshot_baseline_scenario_smoke() {
        let scenario_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("scenarios/survival-baseline.ron");
        let def = load_scenario_file(&scenario_path).expect("baseline scenario loads");
        let spawned = spawn_scenario_ignoring_lints(&def).expect("baseline scenario spawns");
        let world = spawned.state.world();
        let scheduler = spawned.state.scheduler();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let edges = build_edges(world)
            .into_iter()
            .map(|edge| (edge.from, edge.to, edge.travel_ticks))
            .collect::<Vec<_>>();
        let layout = PlaceLayout::compute(&places, &edges, 0);
        let driver = AgentTickDriver::new();

        let snapshot = build_snapshot(world, scheduler, &driver, &layout, scheduler.current_tick());

        assert!(!snapshot.places.is_empty());
        for place in snapshot.places.keys() {
            assert!(world.topology().place(*place).is_some());
        }
        for agent in snapshot.agents.values() {
            assert!(matches!(
                agent.position,
                AgentPosition::AtPlace(_) | AgentPosition::InTransit { .. }
            ));
        }
    }
}
