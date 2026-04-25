use eframe::egui;
use worldwake_core::Tick;

use crate::app::{PlayState, TicksPerSecond};

pub struct TopBarState<'a> {
    pub scenario_label: String,
    pub tick: Option<Tick>,
    pub play_state: PlayState,
    pub speed: &'a mut TicksPerSecond,
    pub scenario_loaded: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TopBarAction {
    TogglePlay,
    Step,
    RequestReset,
    LoadScenario,
    ClearSelection,
}

#[must_use]
pub fn draw_top_bar(ui: &mut egui::Ui, state: TopBarState<'_>) -> Option<TopBarAction> {
    let mut action = None;
    ui.horizontal_wrapped(|ui| {
        ui.label(state.scenario_label);
        ui.separator();
        ui.label(format!(
            "Tick: {}",
            state
                .tick
                .map_or_else(|| "-".to_string(), |tick| tick.0.to_string())
        ));
        ui.separator();

        let play_label = match state.play_state {
            PlayState::Paused => "▶",
            PlayState::Playing => "⏸",
        };
        if ui
            .add_enabled(state.scenario_loaded, egui::Button::new(play_label))
            .clicked()
        {
            action = Some(TopBarAction::TogglePlay);
        }
        if ui
            .add_enabled(state.scenario_loaded, egui::Button::new("⏭ Step"))
            .clicked()
        {
            action = Some(TopBarAction::Step);
        }
        if ui
            .add_enabled(state.scenario_loaded, egui::Button::new("↻ Reset"))
            .clicked()
        {
            action = Some(TopBarAction::RequestReset);
        }

        let mut log_speed = state.speed.get().log10();
        if ui
            .add(
                egui::Slider::new(
                    &mut log_speed,
                    TicksPerSecond::MIN.log10()..=TicksPerSecond::MAX.log10(),
                )
                .text(format!("Speed: {:.1}x", state.speed.get())),
            )
            .changed()
        {
            *state.speed = TicksPerSecond::new(10.0_f32.powf(log_speed));
        }

        if ui.button("Load scenario...").clicked() {
            action = Some(TopBarAction::LoadScenario);
        }
    });
    action
}
