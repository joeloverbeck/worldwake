use egui::{RichText, Ui};
use worldwake_core::{ControlSource, EntityId, Permille, ThresholdBand};

use crate::need_bar::need_bar;
use crate::snapshot::{AgentPosition, AgentView, FrameSnapshot};

const TOOLTIP_WIDTH: f32 = 240.0;
const TOOLTIP_BAR_WIDTH: f32 = 140.0;

pub fn show_tooltip(ui: &mut Ui, snapshot: &FrameSnapshot, agent: &AgentView) {
    ui.set_min_width(TOOLTIP_WIDTH);
    ui.set_max_width(TOOLTIP_WIDTH);

    ui.horizontal(|ui| {
        ui.label(RichText::new(&agent.name).strong());
        ui.label(control_badge(agent.control));
        ui.label(if agent.alive { "alive" } else { "dead" });
    });
    ui.label(location_text(snapshot, agent));
    ui.label(active_action_text(agent));
    ui.label(active_goal_text(agent));
    ui.add_space(4.0);

    need_rows(ui, agent);
}

fn need_rows(ui: &mut Ui, agent: &AgentView) {
    for row in need_row_specs(agent) {
        need_bar(ui, row.label, row.value, &row.thresholds, TOOLTIP_BAR_WIDTH);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NeedRowSpec {
    label: &'static str,
    value: Permille,
    thresholds: ThresholdBand,
}

fn need_row_specs(agent: &AgentView) -> Vec<NeedRowSpec> {
    let needs = agent.needs;
    let thresholds = agent.drive_thresholds;
    let derived = agent.derived_pressures;
    let mut rows = vec![
        ("Hunger", needs.hunger, thresholds.hunger),
        ("Thirst", needs.thirst, thresholds.thirst),
        ("Fatigue", needs.fatigue, thresholds.fatigue),
        ("Bladder", needs.bladder, thresholds.bladder),
        ("Dirtiness", needs.dirtiness, thresholds.dirtiness),
    ];

    if derived.pain != Permille::ZERO {
        rows.push(("Pain", derived.pain, thresholds.pain));
    }
    if derived.danger != Permille::ZERO {
        rows.push(("Danger", derived.danger, thresholds.danger));
    }

    rows.into_iter()
        .map(|(label, value, thresholds)| NeedRowSpec {
            label,
            value,
            thresholds,
        })
        .collect()
}

fn control_badge(control: ControlSource) -> &'static str {
    match control {
        ControlSource::Human => "Human",
        ControlSource::Ai => "AI",
        ControlSource::None => "None",
    }
}

fn location_text(snapshot: &FrameSnapshot, agent: &AgentView) -> String {
    match agent.position {
        AgentPosition::AtPlace(place_id) => {
            format!("@ {}", place_name(snapshot, place_id))
        }
        AgentPosition::InTransit {
            to, k_of_n: (k, n), ..
        } => {
            format!("-> {} ({k}/{n})", place_name(snapshot, to))
        }
    }
}

fn active_action_text(agent: &AgentView) -> String {
    match (agent.active_action, agent.position) {
        (Some(_), AgentPosition::InTransit { k_of_n: (k, n), .. }) => format!("travel [{k}/{n}]"),
        (Some(action), _) => action.action_def_id.to_string(),
        (None, _) => "-".to_string(),
    }
}

fn active_goal_text(agent: &AgentView) -> String {
    agent.active_goal.as_ref().map_or_else(
        || "no goal".to_string(),
        |goal| format!("{:?} | motive {}", goal.goal_kind, goal.motive_score),
    )
}

fn place_name(snapshot: &FrameSnapshot, place_id: EntityId) -> String {
    snapshot
        .places
        .get(&place_id)
        .map_or_else(|| place_id.to_string(), |place| place.name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::PlaceLayout;
    use crate::snapshot::build_snapshot;
    use worldwake_ai::AgentTickDriver;
    use worldwake_cli::scenario::{load_scenario_file, spawn_scenario_ignoring_lints};
    use worldwake_core::Tick;

    #[test]
    fn tooltip_renders_five_core_need_rows_without_panic() {
        let snapshot = baseline_snapshot();
        let agent = snapshot
            .agents
            .values()
            .next()
            .expect("baseline has an agent");
        let ctx = egui::Context::default();

        let _ = ctx.run_ui(Default::default(), |ui| {
            show_tooltip(ui, &snapshot, agent);
        });
    }

    #[test]
    fn need_row_specs_omit_zero_derived_pressures() {
        let snapshot = baseline_snapshot();
        let agent = snapshot
            .agents
            .values()
            .next()
            .expect("baseline has an agent");

        let labels = need_row_specs(agent)
            .into_iter()
            .map(|row| row.label)
            .collect::<Vec<_>>();

        assert_eq!(
            labels,
            vec!["Hunger", "Thirst", "Fatigue", "Bladder", "Dirtiness"]
        );
    }

    #[test]
    fn need_row_specs_include_non_zero_derived_pressures() {
        let snapshot = baseline_snapshot();
        let mut agent = snapshot
            .agents
            .values()
            .next()
            .expect("baseline has an agent")
            .clone();
        agent.derived_pressures.pain = Permille::new(300).unwrap();
        agent.derived_pressures.danger = Permille::new(600).unwrap();

        let rows = need_row_specs(&agent);
        let labels = rows.iter().map(|row| row.label).collect::<Vec<_>>();

        assert_eq!(
            labels,
            vec![
                "Hunger",
                "Thirst",
                "Fatigue",
                "Bladder",
                "Dirtiness",
                "Pain",
                "Danger"
            ]
        );
        assert_eq!(rows[5].value, Permille::new(300).unwrap());
        assert_eq!(rows[5].thresholds, agent.drive_thresholds.pain);
        assert_eq!(rows[6].value, Permille::new(600).unwrap());
        assert_eq!(rows[6].thresholds, agent.drive_thresholds.danger);
    }

    #[test]
    fn active_goal_text_shows_no_goal_when_absent() {
        let mut snapshot = baseline_snapshot();
        let agent = snapshot
            .agents
            .values_mut()
            .next()
            .expect("baseline has an agent");
        agent.active_goal = None;

        assert_eq!(active_goal_text(agent), "no goal");
    }

    #[test]
    fn location_text_resolves_place_names() {
        let snapshot = baseline_snapshot();
        let agent = snapshot
            .agents
            .values()
            .next()
            .expect("baseline has an agent");

        assert!(location_text(&snapshot, agent).starts_with('@'));
    }

    fn baseline_snapshot() -> FrameSnapshot {
        let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("scenarios/survival-baseline.ron");
        let def = load_scenario_file(&scenario).expect("baseline scenario loads");
        let spawned = spawn_scenario_ignoring_lints(&def).expect("baseline scenario spawns");
        let world = spawned.state.world();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let mut edges = Vec::new();
        for place in world.topology().place_ids() {
            for edge_id in world.topology().outgoing_edges(place) {
                let edge = world.topology().edge(*edge_id).expect("edge resolves");
                edges.push((edge.from(), edge.to(), edge.travel_time_ticks()));
            }
        }
        let layout = PlaceLayout::compute(&places, &edges, 0);
        let driver = AgentTickDriver::new();
        build_snapshot(
            world,
            spawned.state.scheduler(),
            None,
            &driver,
            &layout,
            Tick(0),
        )
    }
}
