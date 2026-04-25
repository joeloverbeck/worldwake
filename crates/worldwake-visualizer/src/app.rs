use std::path::{Path, PathBuf};

use clap::Parser;
use eframe::egui;
use worldwake_ai::AgentTickDriver;
use worldwake_cli::scenario::{
    load_scenario_file, spawn_scenario, spawn_scenario_ignoring_lints, ScenarioError,
};
use worldwake_core::{EntityId, Tick, World};
use worldwake_sim::{
    step_tick, ActionTraceSink, AutonomousControllerRuntime, InstitutionalKnowledgeTraceSink,
    PerceptionTraceSink, PoliticalTraceSink, RequestResolutionTraceSink, SimulationState,
    SystemDispatchTable, TickStepError, TickStepServices,
};
use worldwake_systems::ActionRegistries;

use crate::controls::{self, TopBarAction, TopBarState};
use crate::layout::PlaceLayout;
use crate::snapshot::{build_snapshot, FrameSnapshot};
use crate::tabs::DetailTab;
use crate::trace_buffers::AgentTraceBuffers;

pub const MAX_TICKS_PER_FRAME: usize = 100;

#[derive(Clone, Debug, Parser)]
#[command(
    name = "worldwake-visualizer",
    about = "Interactive Worldwake scenario visualizer"
)]
pub struct VisualizerCli {
    /// Path to a RON scenario file.
    pub scenario: Option<PathBuf>,
    /// Bypass scenario lint failures for ad-hoc debugging.
    #[arg(long)]
    pub ignore_lints: bool,
}

pub struct VisualizerApp {
    sim: Option<SimulationState>,
    action_registries: Option<ActionRegistries>,
    dispatch_table: SystemDispatchTable,
    driver: AgentTickDriver,
    scenario_path: Option<PathBuf>,
    ignore_lints: bool,
    action_trace: ActionTraceSink,
    perception_trace: PerceptionTraceSink,
    request_resolution_trace: RequestResolutionTraceSink,
    politics_trace: PoliticalTraceSink,
    institutional_knowledge_trace: InstitutionalKnowledgeTraceSink,
    trace_buffers: AgentTraceBuffers,
    layout: Option<PlaceLayout>,
    play_state: PlayState,
    speed: TicksPerSecond,
    tick_carry: f32,
    selected_agent: Option<EntityId>,
    hovered_agent: Option<EntityId>,
    ui_settings: UiSettings,
    canvas_scene_rect: egui::Rect,
    reset_confirmation_open: bool,
    toast: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiSettings {
    pub active_detail_tab: DetailTab,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            active_detail_tab: DetailTab::Overview,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayState {
    Paused,
    Playing,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TicksPerSecond(f32);

impl TicksPerSecond {
    pub const MIN: f32 = 0.5;
    pub const MAX: f32 = 50.0;
    pub const DEFAULT: f32 = 5.0;

    #[must_use]
    pub fn new(value: f32) -> Self {
        Self(value.clamp(Self::MIN, Self::MAX))
    }

    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl Default for TicksPerSecond {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

impl VisualizerApp {
    /// Build the visualizer host and optionally load the startup scenario.
    ///
    /// Startup scenario failures are retained as a toast so the app can still
    /// open in an empty state and accept a later load.
    pub fn new(cli: VisualizerCli) -> Result<Self, ScenarioError> {
        let mut app = Self::empty(cli.ignore_lints);
        if let Some(path) = cli.scenario {
            app.scenario_path = Some(path.clone());
            if let Err(err) = app.load_scenario_from_path(&path) {
                app.toast = Some(format!("Failed to load scenario: {err}"));
            }
        }
        Ok(app)
    }

    fn empty(ignore_lints: bool) -> Self {
        Self {
            sim: None,
            action_registries: None,
            dispatch_table: SystemDispatchTable::canonical_noop(),
            driver: new_tracing_driver(),
            scenario_path: None,
            ignore_lints,
            action_trace: ActionTraceSink::new(),
            perception_trace: PerceptionTraceSink::new(),
            request_resolution_trace: RequestResolutionTraceSink::new(),
            politics_trace: PoliticalTraceSink::new(),
            institutional_knowledge_trace: InstitutionalKnowledgeTraceSink::new(),
            trace_buffers: AgentTraceBuffers::default(),
            layout: None,
            play_state: PlayState::Paused,
            speed: TicksPerSecond::default(),
            tick_carry: 0.0,
            selected_agent: None,
            hovered_agent: None,
            ui_settings: UiSettings::default(),
            canvas_scene_rect: egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::splat(1_100.0),
            ),
            reset_confirmation_open: false,
            toast: None,
        }
    }

    pub fn step_one_tick(&mut self) {
        if let Err(err) = self.try_step_one_tick() {
            self.play_state = PlayState::Paused;
            self.toast = Some(format!("Tick failed: {err}"));
        }
    }

    fn try_step_one_tick(&mut self) -> Result<(), TickStepError> {
        let (Some(sim), Some(action_registries)) =
            (self.sim.as_mut(), self.action_registries.as_ref())
        else {
            return Ok(());
        };

        let mut controllers = AutonomousControllerRuntime::new(vec![&mut self.driver]);
        let (world, event_log, scheduler, controller, rng, recipe_registry) = sim.tick_parts_mut();

        step_tick(
            world,
            event_log,
            scheduler,
            controller,
            rng,
            TickStepServices {
                action_defs: &action_registries.defs,
                action_handlers: &action_registries.handlers,
                recipe_registry,
                systems: &self.dispatch_table,
                input_producer: Some(&mut controllers),
                action_trace: Some(&mut self.action_trace),
                request_resolution_trace: Some(&mut self.request_resolution_trace),
                politics_trace: Some(&mut self.politics_trace),
                perception_trace: Some(&mut self.perception_trace),
                institutional_knowledge_trace: Some(&mut self.institutional_knowledge_trace),
            },
        )?;
        self.drain_trace_sinks();
        Ok(())
    }

    fn drain_trace_sinks(&mut self) {
        if let Some(sink) = self.driver.trace_sink() {
            self.trace_buffers
                .record_decisions(sink.traces().iter().cloned());
        }
        if let Some(sink) = self.driver.trace_sink_mut() {
            sink.clear();
        }

        self.trace_buffers
            .record_actions(self.action_trace.events().iter().cloned());
        self.action_trace.clear();
    }

    pub fn reset_scenario(&mut self) {
        let Some(path) = self.scenario_path.clone() else {
            return;
        };
        if let Err(err) = self.load_scenario_from_path(&path) {
            self.toast = Some(format!("Failed to reset scenario: {err}"));
        }
    }

    fn load_scenario_from_path(&mut self, path: &Path) -> Result<(), ScenarioError> {
        let def = load_scenario_file(path)?;
        let spawned = if self.ignore_lints {
            spawn_scenario_ignoring_lints(&def)?
        } else {
            spawn_scenario(&def)?
        };

        let next_layout = compute_layout(spawned.state.world());
        let layout = match self.layout.take() {
            Some(existing) if existing.topology_fingerprint == next_layout.topology_fingerprint => {
                existing
            }
            _ => next_layout,
        };

        self.sim = Some(spawned.state);
        self.action_registries = Some(spawned.action_registries);
        self.dispatch_table = spawned.dispatch_table;
        self.driver = new_tracing_driver();
        self.action_trace = ActionTraceSink::new();
        self.perception_trace = PerceptionTraceSink::new();
        self.request_resolution_trace = RequestResolutionTraceSink::new();
        self.politics_trace = PoliticalTraceSink::new();
        self.institutional_knowledge_trace = InstitutionalKnowledgeTraceSink::new();
        self.trace_buffers = AgentTraceBuffers::default();
        self.layout = Some(layout);
        self.play_state = PlayState::Paused;
        self.tick_carry = 0.0;
        self.selected_agent = None;
        self.hovered_agent = None;
        self.canvas_scene_rect =
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(1_100.0));
        self.toast = None;
        Ok(())
    }

    fn scenario_label(&self) -> String {
        self.scenario_path.as_ref().map_or_else(
            || "No scenario loaded".to_string(),
            |path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map_or_else(|| path.display().to_string(), str::to_string)
            },
        )
    }

    fn current_tick(&self) -> Option<Tick> {
        self.sim.as_ref().map(|sim| sim.scheduler().current_tick())
    }

    pub(crate) fn current_snapshot(&self) -> Option<FrameSnapshot> {
        let sim = self.sim.as_ref()?;
        let layout = self.layout.as_ref()?;
        Some(build_snapshot(
            sim.world(),
            sim.scheduler(),
            self.action_registries
                .as_ref()
                .map(|registries| &registries.defs),
            &self.driver,
            layout,
            sim.scheduler().current_tick(),
        ))
    }

    pub(crate) fn world(&self) -> Option<&World> {
        self.sim.as_ref().map(SimulationState::world)
    }

    pub(crate) const fn driver(&self) -> &AgentTickDriver {
        &self.driver
    }

    pub(crate) const fn trace_buffers(&self) -> &AgentTraceBuffers {
        &self.trace_buffers
    }

    pub(crate) const fn active_detail_tab(&self) -> DetailTab {
        self.ui_settings.active_detail_tab
    }

    pub(crate) fn set_active_detail_tab(&mut self, tab: DetailTab) {
        self.ui_settings.active_detail_tab = tab;
    }

    pub(crate) fn clear_selected_agent(&mut self) {
        self.selected_agent = None;
    }

    pub(crate) fn selected_modal_agent(&self) -> Option<EntityId> {
        self.sim.as_ref()?;
        self.selected_agent
    }

    fn handle_key_input(&mut self, ui: &egui::Ui) {
        let action = ui.input_mut(|input| {
            if input.consume_key(egui::Modifiers::CTRL, egui::Key::R) {
                Some(TopBarAction::RequestReset)
            } else if input.consume_key(egui::Modifiers::NONE, egui::Key::Space) {
                Some(TopBarAction::Step)
            } else if input.consume_key(egui::Modifiers::NONE, egui::Key::P) {
                Some(TopBarAction::TogglePlay)
            } else if input.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                Some(TopBarAction::ClearSelection)
            } else {
                None
            }
        });

        if let Some(action) = action {
            self.apply_top_bar_action(action);
        }
    }

    fn advance_playback(&mut self, ui: &egui::Ui) {
        if self.play_state != PlayState::Playing {
            return;
        }

        let dt = ui.input(|input| input.stable_dt);
        self.tick_carry += dt * self.speed.get();
        let mut ticks_this_frame = 0;
        while self.tick_carry >= 1.0 && ticks_this_frame < MAX_TICKS_PER_FRAME {
            self.step_one_tick();
            self.tick_carry -= 1.0;
            ticks_this_frame += 1;
            if self.play_state == PlayState::Paused {
                break;
            }
        }
        if ticks_this_frame == MAX_TICKS_PER_FRAME {
            self.tick_carry = self.tick_carry.min(1.0);
        }
        ui.ctx().request_repaint();
    }

    fn apply_top_bar_action(&mut self, action: TopBarAction) {
        match action {
            TopBarAction::TogglePlay => {
                self.play_state = match self.play_state {
                    PlayState::Paused => PlayState::Playing,
                    PlayState::Playing => PlayState::Paused,
                };
            }
            TopBarAction::Step => self.step_one_tick(),
            TopBarAction::RequestReset => {
                if self.sim.is_some() {
                    self.reset_confirmation_open = true;
                }
            }
            TopBarAction::LoadScenario => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("RON scenarios", &["ron"])
                    .pick_file()
                {
                    self.scenario_path = Some(path.clone());
                    if let Err(err) = self.load_scenario_from_path(&path) {
                        self.toast = Some(format!("Failed to load scenario: {err}"));
                    }
                }
            }
            TopBarAction::ClearSelection => {
                self.selected_agent = None;
            }
        }
    }

    fn draw_reset_confirmation(&mut self, ctx: &egui::Context) {
        if !self.reset_confirmation_open {
            return;
        }

        let response =
            egui::Modal::new(egui::Id::new("visualizer_reset_confirmation")).show(ctx, |ui| {
                ui.heading("Reset scenario");
                ui.label("Reload this scenario from tick 0?");
                ui.horizontal(|ui| {
                    if ui.button("Reset").clicked() {
                        self.reset_scenario();
                        self.reset_confirmation_open = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.reset_confirmation_open = false;
                    }
                });
            });
        if response.should_close() {
            self.reset_confirmation_open = false;
        }
    }

    fn draw_body(&mut self, ui: &mut egui::Ui) {
        if let Some(message) = &self.toast {
            ui.colored_label(egui::Color32::from_rgb(220, 90, 70), message);
        }

        let Some(snapshot) = self.current_snapshot() else {
            ui.centered_and_justified(|ui| {
                ui.label("No scenario loaded");
            });
            return;
        };

        crate::canvas::draw_canvas(
            ui,
            &snapshot,
            &mut self.canvas_scene_rect,
            &mut self.selected_agent,
            &mut self.hovered_agent,
        );
        if let Some(agent_id) = self.selected_modal_agent() {
            crate::modal::show_modal(ui.ctx(), self, agent_id);
        }
    }
}

impl eframe::App for VisualizerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_key_input(ui);
        self.advance_playback(ui);

        let top_bar = TopBarState {
            scenario_label: self.scenario_label(),
            tick: self.current_tick(),
            play_state: self.play_state,
            speed: &mut self.speed,
            scenario_loaded: self.sim.is_some(),
        };
        if let Some(action) = controls::draw_top_bar(ui, top_bar) {
            self.apply_top_bar_action(action);
        }

        ui.separator();
        self.draw_body(ui);
        self.draw_reset_confirmation(ui.ctx());
    }
}

fn new_tracing_driver() -> AgentTickDriver {
    let mut driver = AgentTickDriver::new();
    driver.enable_tracing();
    driver
}

fn compute_layout(world: &worldwake_core::World) -> PlaceLayout {
    let places = world.topology().place_ids().collect::<Vec<_>>();
    let mut edges = Vec::new();
    for place in world.topology().place_ids() {
        for edge_id in world.topology().outgoing_edges(place) {
            let edge = world
                .topology()
                .edge(*edge_id)
                .expect("outgoing edge IDs must resolve");
            edges.push((edge.from(), edge.to(), edge.travel_time_ticks()));
        }
    }
    PlaceLayout::compute(&places, &edges, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_one_tick_advances_scheduler_tick() {
        let mut app = baseline_app();
        let before = app.current_tick().expect("scenario is loaded");

        app.step_one_tick();

        assert_eq!(app.current_tick(), Some(Tick(before.0 + 1)));
    }

    #[test]
    fn step_one_tick_advances_100_ticks_without_panic() {
        let mut app = baseline_app();
        let before = app.current_tick().expect("scenario is loaded");

        for _ in 0..100 {
            app.step_one_tick();
        }

        assert_eq!(app.current_tick(), Some(Tick(before.0 + 100)));
    }

    #[test]
    fn reset_reloads_at_tick_zero() {
        let mut app = baseline_app();
        for _ in 0..5 {
            app.step_one_tick();
        }
        assert_eq!(app.current_tick(), Some(Tick(5)));

        app.reset_scenario();

        assert_eq!(app.current_tick(), Some(Tick(0)));
        assert!(app.tick_carry == 0.0);
    }

    #[test]
    fn missing_startup_scenario_opens_empty_with_toast() {
        let missing = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("scenarios/does-not-exist.ron");

        let app = VisualizerApp::new(VisualizerCli {
            scenario: Some(missing),
            ignore_lints: true,
        })
        .expect("load errors are retained in-app");

        assert!(app.sim.is_none());
        assert!(app.toast.as_deref().is_some_and(|msg| {
            msg.contains("Failed to load scenario") && msg.contains("I/O error")
        }));
    }

    #[test]
    fn modal_opens_on_agent_select() {
        let mut app = baseline_app();
        let agent = app
            .world()
            .expect("scenario is loaded")
            .entities_with_name_and_agent_data()
            .next()
            .expect("baseline has an agent");

        app.selected_agent = Some(agent);

        assert_eq!(app.selected_modal_agent(), Some(agent));

        app.clear_selected_agent();

        assert_eq!(app.selected_modal_agent(), None);
    }

    #[test]
    fn traces_populated_after_steps() {
        let mut app = baseline_app();

        for _ in 0..20 {
            app.step_one_tick();
        }

        let agent = app
            .world()
            .expect("scenario is loaded")
            .entities_with_name_and_agent_data()
            .find(|agent| {
                app.trace_buffers().decisions_for(*agent).next().is_some()
                    && app.trace_buffers().actions_for(*agent).next().is_some()
            })
            .expect("stepping baseline should record decision and action traces for an agent");

        assert!(app
            .trace_buffers()
            .decisions_for(agent)
            .all(|trace| trace.agent == agent && trace.tick.0 < 20));
        assert!(app
            .trace_buffers()
            .actions_for(agent)
            .all(|event| event.actor == agent && event.tick.0 < 20));
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
