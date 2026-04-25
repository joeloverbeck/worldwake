use egui::Ui;
use worldwake_ai::{AgentDecisionRuntime, PlanExpectation, PlanGuard, PlannedPlan, PlannedStep};
use worldwake_core::{EntityId, IntentionFrame, World};

use crate::app::VisualizerApp;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IntentionView {
    pub domain: String,
    pub state: String,
    pub goal: String,
}

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlanStepRow {
    pub index: usize,
    pub current: bool,
    pub op_kind: String,
    pub target_place: Option<EntityId>,
    pub estimated_ticks: u32,
    pub guard_required_facts: Option<usize>,
    pub expectation_count: usize,
}

pub fn render(ui: &mut Ui, app: &VisualizerApp, agent_id: EntityId) {
    let Some(world) = app.world() else {
        ui.label("No scenario loaded");
        return;
    };

    ui.heading("Plan");
    render_intention(ui, world, agent_id);
    ui.separator();
    render_current_plan(ui, app.driver().runtime(agent_id));
    ui.separator();
    render_last_replan_reason(ui, app, agent_id);
}

pub(crate) fn intention_view(world: &World, agent_id: EntityId) -> Option<IntentionView> {
    world
        .get_component_intention_frame(agent_id)
        .map(intention_frame_view)
}

#[cfg(test)]
pub(crate) fn plan_step_rows(runtime: &AgentDecisionRuntime) -> Vec<PlanStepRow> {
    let Some(plan) = runtime.current_plan.as_ref() else {
        return Vec::new();
    };
    plan.steps
        .iter()
        .enumerate()
        .map(|(index, step)| PlanStepRow {
            index,
            current: index == runtime.current_step_index,
            op_kind: format!("{:?}", step.op_kind),
            target_place: step.target_place,
            estimated_ticks: step.estimated_ticks,
            guard_required_facts: step.guard.as_ref().map(|guard| guard.required_facts.len()),
            expectation_count: step.expectations.len(),
        })
        .collect()
}

fn intention_frame_view(frame: &IntentionFrame) -> IntentionView {
    IntentionView {
        domain: format!("{:?}", frame.domain),
        state: format!("{:?}", frame.state),
        goal: format!("{:?}", frame.goal.kind),
    }
}

fn render_intention(ui: &mut Ui, world: &World, agent_id: EntityId) {
    egui::CollapsingHeader::new("IntentionFrame")
        .default_open(true)
        .show(ui, |ui| match intention_view(world, agent_id) {
            Some(frame) => {
                egui::Grid::new("plan_intention_grid")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Domain");
                        ui.label(frame.domain);
                        ui.end_row();
                        ui.label("State");
                        ui.label(frame.state);
                        ui.end_row();
                        ui.label("Goal");
                        ui.label(frame.goal);
                        ui.end_row();
                    });
            }
            None => {
                ui.label("no active intention");
            }
        });
}

fn render_current_plan(ui: &mut Ui, runtime: Option<&AgentDecisionRuntime>) {
    egui::CollapsingHeader::new("PlannedPlan")
        .default_open(true)
        .show(ui, |ui| {
            let Some(runtime) = runtime else {
                ui.label("no current plan");
                return;
            };
            let Some(plan) = runtime.current_plan.as_ref() else {
                ui.label("no current plan");
                return;
            };

            render_plan_summary(ui, plan, runtime.current_step_index);
            for (index, step) in plan.steps.iter().enumerate() {
                render_step(ui, index, step, index == runtime.current_step_index);
            }
        });
}

fn render_plan_summary(ui: &mut Ui, plan: &PlannedPlan, current_step_index: usize) {
    egui::Grid::new("plan_summary_grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.label("Goal");
            ui.label(format!("{:?}", plan.goal.kind));
            ui.end_row();
            ui.label("Terminal");
            ui.label(format!("{:?}", plan.terminal_kind));
            ui.end_row();
            ui.label("Estimated ticks");
            ui.label(plan.total_estimated_ticks.to_string());
            ui.end_row();
            ui.label("Current step");
            ui.label(current_step_index.to_string());
            ui.end_row();
        });
}

fn render_step(ui: &mut Ui, index: usize, step: &PlannedStep, current: bool) {
    let title = if current {
        format!("Step {index} * {:?}", step.op_kind)
    } else {
        format!("Step {index} {:?}", step.op_kind)
    };
    egui::CollapsingHeader::new(title)
        .default_open(current)
        .show(ui, |ui| {
            egui::Grid::new(format!("plan_step_{index}_grid"))
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Action");
                    ui.label(step.def_id.to_string());
                    ui.end_row();
                    ui.label("Target place");
                    ui.label(
                        step.target_place
                            .map_or_else(|| "-".to_string(), |place| place.to_string()),
                    );
                    ui.end_row();
                    ui.label("Estimated ticks");
                    ui.label(step.estimated_ticks.to_string());
                    ui.end_row();
                });
            render_guard(ui, step.guard.as_ref());
            render_expectations(ui, &step.expectations);
        });
}

fn render_guard(ui: &mut Ui, guard: Option<&PlanGuard>) {
    egui::CollapsingHeader::new("guard")
        .default_open(false)
        .show(ui, |ui| match guard {
            Some(guard) => {
                ui.label(format!("required_facts {}", guard.required_facts.len()));
                ui.label(format!("min_confidence {}", guard.min_confidence));
                ui.label(format!("invalidators {}", guard.invalidators.len()));
                for fact in &guard.required_facts {
                    ui.label(format!("{fact:?}"));
                }
            }
            None => {
                ui.label("none");
            }
        });
}

fn render_expectations(ui: &mut Ui, expectations: &[PlanExpectation]) {
    egui::CollapsingHeader::new(format!("expectations ({})", expectations.len()))
        .default_open(false)
        .show(ui, |ui| {
            for expectation in expectations {
                ui.label(format!(
                    "{:?} | observe_by {}",
                    expectation.kind,
                    expectation
                        .observe_by
                        .map_or_else(|| "-".to_string(), |tick| tick.0.to_string())
                ));
            }
        });
}

pub(crate) fn last_replan_reason_text(app: &VisualizerApp, agent_id: EntityId) -> String {
    app.trace_buffers()
        .last_replan_summary(agent_id)
        .unwrap_or_else(|| "no replan recorded".to_string())
}

fn render_last_replan_reason(ui: &mut Ui, app: &VisualizerApp, agent_id: EntityId) {
    egui::CollapsingHeader::new("Last replan reason")
        .default_open(false)
        .show(ui, |ui| {
            ui.label(last_replan_reason_text(app, agent_id));
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{VisualizerApp, VisualizerCli};
    use worldwake_ai::{
        AgentDecisionRuntime, ExpectedMaterialization, PlanTerminalKind, PlannerOpKind,
        RequiredFact,
    };
    use worldwake_core::{
        ActionDefId, CommodityKind, CommodityPurpose, GoalKey, GoalKind, OpportunityAnchor,
        OpportunityKey, Permille, Quantity,
    };
    use worldwake_sim::ActionPayload;

    #[test]
    fn plan_tab_renders_active_intention_when_present() {
        let app = baseline_app_with_intention();
        let world = app.world().expect("scenario is loaded");
        let agent_with_frame = world
            .entities_with_name_and_agent_data()
            .find(|agent| intention_view(world, *agent).is_some())
            .expect("baseline produces an intention frame after stepping");

        assert!(intention_view(world, agent_with_frame).is_some());

        let missing = world
            .entities_with_name_and_agent_data()
            .find(|agent| world.get_component_intention_frame(*agent).is_none());
        if let Some(agent_without_frame) = missing {
            assert_eq!(intention_view(world, agent_without_frame), None);
        }
    }

    #[test]
    fn plan_tab_step_guards_visible() {
        let runtime = AgentDecisionRuntime {
            current_plan: Some(sample_plan_with_guard()),
            current_step_index: 0,
            ..AgentDecisionRuntime::default()
        };

        let rows = plan_step_rows(&runtime);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].guard_required_facts, Some(1));
        assert!(rows[0].current);
    }

    fn sample_plan_with_guard() -> PlannedPlan {
        let place = entity(2);
        let goal = GoalKey::new(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        PlannedPlan::new(
            OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::Place(place),
            },
            goal,
            vec![PlannedStep {
                def_id: ActionDefId(1),
                targets: Vec::new(),
                target_place: Some(place),
                payload_override: Some(ActionPayload::None),
                op_kind: PlannerOpKind::Travel,
                estimated_ticks: 3,
                is_materialization_barrier: false,
                expected_materializations: Vec::<ExpectedMaterialization>::new(),
                guard: Some(PlanGuard {
                    required_facts: vec![RequiredFact::CommodityAvailable {
                        place,
                        kind: CommodityKind::Bread,
                        min_quantity: Quantity(1),
                    }],
                    min_confidence: Permille::new(700).expect("valid permille"),
                    invalidators: Vec::new(),
                }),
                expectations: Vec::new(),
            }],
            PlanTerminalKind::ProgressBarrier,
        )
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn baseline_app_with_intention() -> VisualizerApp {
        let mut app = baseline_app();
        for _ in 0..200 {
            if app
                .world()
                .expect("scenario is loaded")
                .entities_with_name_and_agent_data()
                .any(|agent| {
                    app.world()
                        .expect("scenario is loaded")
                        .get_component_intention_frame(agent)
                        .is_some()
                })
            {
                return app;
            }
            app.step_one_tick();
        }
        app
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
