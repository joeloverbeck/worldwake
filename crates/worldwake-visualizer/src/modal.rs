use eframe::egui;
use worldwake_core::EntityId;

use crate::app::VisualizerApp;
use crate::tabs::{self, DetailTab};

const MODAL_SIZE: egui::Vec2 = egui::vec2(820.0, 640.0);

pub fn show_modal(ctx: &egui::Context, app: &mut VisualizerApp, agent_id: EntityId) {
    let response =
        egui::Modal::new(egui::Id::new(("agent_detail_modal", agent_id))).show(ctx, |ui| {
            egui::Resize::default()
                .id_salt(("agent_detail_modal_resize", agent_id))
                .default_size(MODAL_SIZE)
                .min_size(egui::vec2(520.0, 360.0))
                .show(ui, |ui| {
                    modal_header(ui, app, agent_id);
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            tabs::render_tab(ui, app.active_detail_tab(), app, agent_id);
                        });
                });
        });

    if response.should_close() {
        app.clear_selected_agent();
    }
}

fn modal_header(ui: &mut egui::Ui, app: &mut VisualizerApp, agent_id: EntityId) {
    ui.horizontal(|ui| {
        ui.heading(agent_title(app, agent_id));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Close").clicked() {
                app.clear_selected_agent();
            }
        });
    });
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        for tab in DetailTab::ALL {
            let selected = app.active_detail_tab() == tab;
            if egui::CollapsingHeader::new(tab.label())
                .id_salt(("agent_detail_tab", tab))
                .default_open(selected)
                .show(ui, |ui| {
                    if ui.button("Open").clicked() {
                        app.set_active_detail_tab(tab);
                    }
                })
                .header_response
                .clicked()
            {
                app.set_active_detail_tab(tab);
            }
        }
    });
}

fn agent_title(app: &VisualizerApp, agent_id: EntityId) -> String {
    app.world().map_or_else(
        || format!("Agent {agent_id}"),
        |world| {
            format!(
                "{} ({agent_id})",
                worldwake_cli::display::entity_display_name(world, agent_id)
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{VisualizerApp, VisualizerCli};

    #[test]
    fn modal_agent_title_uses_display_name() {
        let app = baseline_app();
        let agent = app
            .world()
            .expect("scenario is loaded")
            .entities_with_name_and_agent_data()
            .next()
            .expect("baseline has an agent");

        let title = agent_title(&app, agent);

        assert!(title.contains(&agent.to_string()));
        assert!(title.contains('('));
    }

    fn baseline_app() -> VisualizerApp {
        let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("scenarios/survival-baseline.ron");
        VisualizerApp::new(VisualizerCli {
            scenario: Some(scenario),
            ignore_lints: true,
        })
        .expect("visualizer app constructs")
    }
}
