use egui::Ui;
use worldwake_ai::AgentDecisionTrace;
use worldwake_core::EntityId;
use worldwake_sim::{ActionTraceEvent, ActionTraceKind};

use crate::app::VisualizerApp;

pub fn render(ui: &mut Ui, app: &VisualizerApp, agent_id: EntityId) {
    ui.heading("Traces");
    ui.columns(2, |columns| {
        render_decision_column(&mut columns[0], app, agent_id);
        render_action_column(&mut columns[1], app, agent_id);
    });
}

fn render_decision_column(ui: &mut Ui, app: &VisualizerApp, agent_id: EntityId) {
    ui.strong("Decision");
    let mut count = 0;
    for trace in app.trace_buffers().decisions_for_newest_first(agent_id) {
        count += 1;
        render_decision_trace(ui, trace);
    }
    if count == 0 {
        ui.label("No decision traces recorded");
    }
}

fn render_action_column(ui: &mut Ui, app: &VisualizerApp, agent_id: EntityId) {
    ui.strong("Action");
    let mut count = 0;
    for event in app.trace_buffers().actions_for_newest_first(agent_id) {
        count += 1;
        render_action_trace(ui, event);
    }
    if count == 0 {
        ui.label("No action traces recorded");
    }
}

fn render_decision_trace(ui: &mut Ui, trace: &AgentDecisionTrace) {
    egui::CollapsingHeader::new(format!("{} {}", trace.tick, trace.outcome.summary()))
        .default_open(false)
        .show(ui, |ui| {
            ui.label(format!("{:#?}", trace.outcome));
        });
}

fn render_action_trace(ui: &mut Ui, event: &ActionTraceEvent) {
    egui::CollapsingHeader::new(format!(
        "{} {} {}",
        event.tick,
        action_kind_label(&event.kind),
        event.action_name
    ))
    .default_open(false)
    .show(ui, |ui| {
        ui.label(event.summary());
        if let Some(detail) = &event.detail {
            ui.label(detail.summary());
        }
    });
}

fn action_kind_label(kind: &ActionTraceKind) -> &'static str {
    match kind {
        ActionTraceKind::Started { .. } => "started",
        ActionTraceKind::Committed { .. } => "committed",
        ActionTraceKind::Aborted { .. } => "aborted",
        ActionTraceKind::StartFailed { .. } => "start failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldwake_sim::ActionTraceKind;

    #[test]
    fn action_kind_labels_are_human_readable() {
        assert_eq!(
            action_kind_label(&ActionTraceKind::Started {
                targets: Vec::new()
            }),
            "started"
        );
    }
}
