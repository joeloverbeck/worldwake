use std::path::PathBuf;

use clap::Parser;
use eframe::egui;
use worldwake_ai::{AgentTickDriver, DecisionTraceSink};
use worldwake_cli::scenario::{
    load_scenario_file, spawn_scenario, spawn_scenario_ignoring_lints, ScenarioError,
};
use worldwake_core::EntityId;
use worldwake_sim::{
    ActionTraceSink, InstitutionalKnowledgeTraceSink, PerceptionTraceSink, PoliticalTraceSink,
    RequestResolutionTraceSink, SimulationState, SystemDispatchTable,
};
use worldwake_systems::{dispatch_table, ActionRegistries};

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
    action_trace: ActionTraceSink,
    decision_trace: DecisionTraceSink,
    perception_trace: PerceptionTraceSink,
    request_resolution_trace: RequestResolutionTraceSink,
    politics_trace: PoliticalTraceSink,
    institutional_knowledge_trace: InstitutionalKnowledgeTraceSink,
    play_state: PlayState,
    speed: TicksPerSecond,
    tick_carry: f32,
    selected_agent: Option<EntityId>,
    hovered_agent: Option<EntityId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayState {
    Paused,
    Playing,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TicksPerSecond(f32);

impl VisualizerApp {
    /// Build the visualizer shell and optionally load the startup scenario.
    ///
    /// The simulation state remains inert in this ticket. T01DEBVIS-004 wires
    /// the tick loop and borrows these persistent artifacts into `step_tick`.
    pub fn new(cli: VisualizerCli) -> Result<Self, ScenarioError> {
        let mut driver = AgentTickDriver::new();
        driver.enable_tracing();

        let mut sim = None;
        let mut action_registries = None;
        let scenario_path = cli.scenario.clone();

        if let Some(path) = &cli.scenario {
            let def = load_scenario_file(path)?;
            let spawned = if cli.ignore_lints {
                spawn_scenario_ignoring_lints(&def)?
            } else {
                spawn_scenario(&def)?
            };
            sim = Some(spawned.state);
            action_registries = Some(spawned.action_registries);
        }

        Ok(Self {
            sim,
            action_registries,
            dispatch_table: dispatch_table(),
            driver,
            scenario_path,
            action_trace: ActionTraceSink::new(),
            decision_trace: DecisionTraceSink::new(),
            perception_trace: PerceptionTraceSink::new(),
            request_resolution_trace: RequestResolutionTraceSink::new(),
            politics_trace: PoliticalTraceSink::new(),
            institutional_knowledge_trace: InstitutionalKnowledgeTraceSink::new(),
            play_state: PlayState::Paused,
            speed: TicksPerSecond(1.0),
            tick_carry: 0.0,
            selected_agent: None,
            hovered_agent: None,
        })
    }

    pub fn step_one_tick(&mut self) {
        unimplemented!("per-tick stepping lands in T01DEBVIS-004");
    }

    fn scenario_label(&self) -> String {
        self.scenario_path.as_ref().map_or_else(
            || "No scenario loaded".to_string(),
            |path| path.display().to_string(),
        )
    }

    fn draw_shell(&self, ui: &mut egui::Ui) {
        let _staged_runtime = (
            &self.action_registries,
            &self.dispatch_table,
            &self.driver,
            &self.action_trace,
            &self.decision_trace,
            &self.perception_trace,
            &self.request_resolution_trace,
            &self.politics_trace,
            &self.institutional_knowledge_trace,
            self.tick_carry,
            self.selected_agent,
            self.hovered_agent,
        );

        ui.heading("Worldwake Visualizer");
        ui.label(self.scenario_label());
        if self.sim.is_none() {
            ui.label("Load scenario...");
            return;
        }

        ui.label(match self.play_state {
            PlayState::Paused => "Paused",
            PlayState::Playing => "Playing",
        });
        ui.separator();
        egui::Grid::new("visualizer_shell_state").show(ui, |ui| {
            ui.label("Speed");
            ui.label(format!("{:.1} ticks/s", self.speed.0));
            ui.end_row();
        });
    }
}

impl eframe::App for VisualizerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw_shell(ui);
    }
}
