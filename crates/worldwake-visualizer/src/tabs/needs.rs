use egui::Ui;
#[cfg(test)]
use worldwake_core::World;
use worldwake_core::{
    DriveThresholds, EntityId, HomeostaticNeedId, HomeostaticNeeds, MetabolismProfile,
    ThresholdBand,
};

use crate::app::VisualizerApp;
use crate::need_bar::need_bar;

const DETAIL_BAR_WIDTH: f32 = 520.0;

pub fn render(ui: &mut Ui, app: &VisualizerApp, agent_id: EntityId) {
    let Some(world) = app.world() else {
        ui.label("No scenario loaded");
        return;
    };

    let Some(needs) = world.get_component_homeostatic_needs(agent_id).copied() else {
        ui.label("No needs component");
        return;
    };
    let thresholds = world
        .get_component_drive_thresholds(agent_id)
        .copied()
        .unwrap_or_default();

    ui.heading("Needs");
    for row in need_rows(needs) {
        need_bar(
            ui,
            row.label,
            row.value,
            threshold_band(&thresholds, row.id),
            DETAIL_BAR_WIDTH,
        );
    }

    ui.separator();
    ui.heading("Metabolism");
    if let Some(profile) = world.get_component_metabolism_profile(agent_id) {
        render_metabolism(ui, profile);
    } else {
        ui.label("No metabolism profile");
    }

    ui.separator();
    ui.heading("Drive escalation");
    if let Some(profile) = world.get_component_drive_escalation_profile(agent_id) {
        egui::Grid::new("drive_escalation_grid")
            .num_columns(4)
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Need");
                ui.strong("Start");
                ui.strong("Growth");
                ui.strong("Cap");
                ui.end_row();
                for need in HomeostaticNeedId::ALL {
                    let params = profile.params_for(need);
                    ui.label(format!("{need:?}"));
                    ui.label(params.start_after_ticks.to_string());
                    ui.label(params.growth_per_tick.to_string());
                    ui.label(params.max_multiplier.value().to_string());
                    ui.end_row();
                }
            });
    } else {
        ui.label("No drive escalation profile");
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NeedRow {
    pub id: HomeostaticNeedId,
    pub label: &'static str,
    pub value: worldwake_core::Permille,
}

#[cfg(test)]
pub(crate) fn rendered_need_count(world: &World, agent_id: EntityId) -> usize {
    world
        .get_component_homeostatic_needs(agent_id)
        .map_or(0, |needs| need_rows(*needs).len())
}

fn need_rows(needs: HomeostaticNeeds) -> [NeedRow; 5] {
    [
        NeedRow {
            id: HomeostaticNeedId::Hunger,
            label: "Hunger",
            value: needs.hunger,
        },
        NeedRow {
            id: HomeostaticNeedId::Thirst,
            label: "Thirst",
            value: needs.thirst,
        },
        NeedRow {
            id: HomeostaticNeedId::Fatigue,
            label: "Fatigue",
            value: needs.fatigue,
        },
        NeedRow {
            id: HomeostaticNeedId::Bladder,
            label: "Bladder",
            value: needs.bladder,
        },
        NeedRow {
            id: HomeostaticNeedId::Dirtiness,
            label: "Dirtiness",
            value: needs.dirtiness,
        },
    ]
}

const fn threshold_band(thresholds: &DriveThresholds, need: HomeostaticNeedId) -> &ThresholdBand {
    match need {
        HomeostaticNeedId::Hunger => &thresholds.hunger,
        HomeostaticNeedId::Thirst => &thresholds.thirst,
        HomeostaticNeedId::Fatigue => &thresholds.fatigue,
        HomeostaticNeedId::Bladder => &thresholds.bladder,
        HomeostaticNeedId::Dirtiness => &thresholds.dirtiness,
    }
}

fn render_metabolism(ui: &mut Ui, profile: &MetabolismProfile) {
    egui::Grid::new("metabolism_grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            for (label, value) in [
                ("Hunger rate", profile.hunger_rate.to_string()),
                ("Thirst rate", profile.thirst_rate.to_string()),
                ("Fatigue rate", profile.fatigue_rate.to_string()),
                ("Bladder rate", profile.bladder_rate.to_string()),
                ("Dirtiness rate", profile.dirtiness_rate.to_string()),
                ("Rest efficiency", profile.rest_efficiency.to_string()),
                (
                    "Starvation tolerance",
                    profile.starvation_tolerance_ticks.get().to_string(),
                ),
                (
                    "Dehydration tolerance",
                    profile.dehydration_tolerance_ticks.get().to_string(),
                ),
                (
                    "Collapse tolerance",
                    profile.exhaustion_collapse_ticks.get().to_string(),
                ),
                (
                    "Bladder tolerance",
                    profile.bladder_accident_tolerance_ticks.get().to_string(),
                ),
                ("Toilet ticks", profile.toilet_ticks.get().to_string()),
                ("Wash ticks", profile.wash_ticks.get().to_string()),
                (
                    "Travel fatigue multiplier",
                    profile.travel_fatigue_multiplier.to_string(),
                ),
                (
                    "Travel thirst multiplier",
                    profile.travel_thirst_multiplier.to_string(),
                ),
                (
                    "Travel bladder multiplier",
                    profile.travel_bladder_multiplier.to_string(),
                ),
                (
                    "Wilderness relief dirtiness",
                    profile.wilderness_relief_dirtiness_penalty.to_string(),
                ),
            ] {
                ui.label(label);
                ui.label(value);
                ui.end_row();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{VisualizerApp, VisualizerCli};

    #[test]
    fn needs_tab_renders_all_five_core_needs() {
        let app = baseline_app();
        let world = app.world().expect("scenario is loaded");
        let agent = world
            .entities_with_name_and_agent_data()
            .find(|agent| world.get_component_homeostatic_needs(*agent).is_some())
            .expect("baseline has an agent with needs");

        assert_eq!(rendered_need_count(world, agent), 5);
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
