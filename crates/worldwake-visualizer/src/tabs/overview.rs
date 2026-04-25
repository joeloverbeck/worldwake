use egui::Ui;
use worldwake_ai::AgendaEntry;
use worldwake_core::EntityId;

use crate::app::VisualizerApp;
use crate::snapshot::{AgentPosition, AgentView};

pub fn render(ui: &mut Ui, app: &VisualizerApp, agent_id: EntityId) {
    let Some(world) = app.world() else {
        ui.label("No scenario loaded");
        return;
    };

    ui.heading(worldwake_cli::display::entity_display_name(world, agent_id));
    ui.monospace(agent_id.to_string());
    if let Some(snapshot) = app.current_snapshot() {
        if let Some(agent) = snapshot.agents.get(&agent_id) {
            render_summary(ui, &snapshot, agent);
        }
    }

    ui.separator();
    ui.heading("Agenda");
    if let Some(runtime) = app.driver().runtime(agent_id) {
        render_committed(ui, runtime.agenda_state.committed.as_ref());
        render_entries(ui, "Pending", runtime.agenda_state.pending.values());
        render_entries(ui, "Suspended", runtime.agenda_state.suspended.values());
        render_top_candidates(ui, runtime.agenda_state.pending.values());
    } else {
        ui.label("idle");
    }
}

fn render_summary(ui: &mut Ui, snapshot: &crate::snapshot::FrameSnapshot, agent: &AgentView) {
    egui::Grid::new("overview_summary_grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.label("Control");
            ui.label(format!("{:?}", agent.control));
            ui.end_row();
            ui.label("State");
            ui.label(if agent.alive { "alive" } else { "dead" });
            ui.end_row();
            ui.label("Location");
            ui.label(location_text(snapshot, agent));
            ui.end_row();
            ui.label("Action");
            ui.label(agent.active_action.map_or_else(
                || "-".to_string(),
                |action| {
                    format!(
                        "{} ({}/{})",
                        action.action_def_id, action.ticks_in, action.ticks_total
                    )
                },
            ));
            ui.end_row();
            ui.label("Goal");
            ui.label(agent.active_goal.as_ref().map_or_else(
                || "idle".to_string(),
                |goal| format!("{:?} | motive {}", goal.goal_kind, goal.motive_score),
            ));
            ui.end_row();
        });
}

fn render_committed(ui: &mut Ui, committed: Option<&AgendaEntry>) {
    egui::CollapsingHeader::new("Committed")
        .default_open(true)
        .show(ui, |ui| match committed {
            Some(entry) => render_entry(ui, entry),
            None => {
                ui.label("idle");
            }
        });
}

fn render_entries<'a>(
    ui: &mut Ui,
    label: &str,
    entries: impl ExactSizeIterator<Item = &'a AgendaEntry>,
) {
    egui::CollapsingHeader::new(format!("{label} ({})", entries.len()))
        .default_open(false)
        .show(ui, |ui| {
            for entry in entries {
                render_entry(ui, entry);
                ui.separator();
            }
        });
}

fn render_top_candidates<'a>(ui: &mut Ui, entries: impl Iterator<Item = &'a AgendaEntry>) {
    let mut ranked = entries.collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .priority_class
            .cmp(&left.priority_class)
            .then_with(|| right.motive_score.cmp(&left.motive_score))
            .then_with(|| left.key.cmp(&right.key))
    });

    egui::CollapsingHeader::new("Top candidates")
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("overview_top_candidates_grid")
                .num_columns(3)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Goal");
                    ui.strong("Priority");
                    ui.strong("Motive");
                    ui.end_row();
                    for entry in ranked.into_iter().take(5) {
                        ui.label(format!("{:?}", entry.offer.key.kind));
                        ui.label(format!("{:?}", entry.priority_class));
                        ui.label(entry.motive_score.to_string());
                        ui.end_row();
                    }
                });
        });
}

fn render_entry(ui: &mut Ui, entry: &AgendaEntry) {
    egui::Grid::new(format!("agenda_entry_{:?}", entry.key))
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.label("Goal");
            ui.label(format!("{:?}", entry.offer.key.kind));
            ui.end_row();
            ui.label("Motive");
            ui.label(entry.motive_score.to_string());
            ui.end_row();
            ui.label("Priority");
            ui.label(format!("{:?}", entry.priority_class));
            ui.end_row();
            ui.label("Provenance");
            ui.label(format!("{:?}", entry.provenance));
            ui.end_row();
        });
}

fn location_text(snapshot: &crate::snapshot::FrameSnapshot, agent: &AgentView) -> String {
    match agent.position {
        AgentPosition::AtPlace(place) => snapshot
            .places
            .get(&place)
            .map_or_else(|| place.to_string(), |place| format!("@ {}", place.name)),
        AgentPosition::InTransit {
            to, k_of_n: (k, n), ..
        } => snapshot.places.get(&to).map_or_else(
            || format!("-> {to} ({k}/{n})"),
            |place| format!("-> {} ({k}/{n})", place.name),
        ),
    }
}
