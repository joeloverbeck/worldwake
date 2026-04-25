use egui::Ui;
use worldwake_core::{
    AgentBeliefStore, EntityBeliefAspect, EntityId, Permille, PlaceVisitRecord, Tick, World,
};

use crate::app::VisualizerApp;

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BeliefSourcePresence {
    pub agent_belief_store: bool,
    pub last_seen_memory: bool,
    pub expectation_store: bool,
    pub source_reliability: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EntityClaimRow {
    pub subject: EntityId,
    pub aspect: EntityBeliefAspect,
    pub confidence: Permille,
    pub acquired_tick: Tick,
    pub freshness_ticks: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlaceVisitRow {
    pub place: EntityId,
    pub place_name: String,
    pub last_arrival_tick: Tick,
    pub visit_count: u16,
    pub ticks_present: u32,
}

pub fn render(ui: &mut Ui, app: &VisualizerApp, agent_id: EntityId) {
    let Some(world) = app.world() else {
        ui.label("No scenario loaded");
        return;
    };
    let current_tick = app
        .current_snapshot()
        .map_or(Tick(0), |snapshot| snapshot.tick);

    ui.heading("Beliefs");
    render_agent_belief_store(ui, world, agent_id, current_tick);
    render_last_seen_memory(ui, world, agent_id);
    render_expectation_store(ui, world, agent_id);
    render_source_reliability(ui, world, agent_id);
}

#[cfg(test)]
pub(crate) fn source_presence(world: &World, agent_id: EntityId) -> BeliefSourcePresence {
    BeliefSourcePresence {
        agent_belief_store: world.get_component_agent_belief_store(agent_id).is_some(),
        last_seen_memory: world.get_component_last_seen_memory(agent_id).is_some(),
        expectation_store: world.get_component_expectation_store(agent_id).is_some(),
        source_reliability: world.get_component_source_reliability(agent_id).is_some(),
    }
}

pub(crate) fn entity_claim_rows(
    store: &AgentBeliefStore,
    current_tick: Tick,
) -> Vec<EntityClaimRow> {
    let mut rows = store
        .entity_claims
        .iter()
        .flat_map(|(subject, claims)| {
            claims.iter().map(|claim| EntityClaimRow {
                subject: *subject,
                aspect: claim.aspect,
                confidence: claim.confidence,
                acquired_tick: claim.acquired_tick,
                freshness_ticks: current_tick.0.saturating_sub(claim.acquired_tick.0),
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| (row.freshness_ticks, row.subject, row.aspect));
    rows
}

pub(crate) fn place_visit_rows(world: &World, store: &AgentBeliefStore) -> Vec<PlaceVisitRow> {
    let mut rows = store
        .place_visits
        .iter()
        .map(|(place, record)| place_visit_row(world, *place, *record))
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .last_arrival_tick
            .cmp(&left.last_arrival_tick)
            .then_with(|| left.place.cmp(&right.place))
    });
    rows
}

fn place_visit_row(world: &World, place: EntityId, record: PlaceVisitRecord) -> PlaceVisitRow {
    PlaceVisitRow {
        place,
        place_name: worldwake_cli::display::entity_display_name(world, place),
        last_arrival_tick: record.last_arrival_tick,
        visit_count: record.visit_count,
        ticks_present: record.ticks_present,
    }
}

fn render_agent_belief_store(ui: &mut Ui, world: &World, agent_id: EntityId, current_tick: Tick) {
    egui::CollapsingHeader::new("AgentBeliefStore")
        .default_open(true)
        .show(ui, |ui| {
            let Some(store) = world.get_component_agent_belief_store(agent_id) else {
                ui.label("absent");
                return;
            };

            render_entity_claims(ui, store, current_tick);
            render_debug_count(ui, "known_entities", store.known_entities.len());
            render_debug_count(ui, "social_observations", store.social_observations.len());
            render_debug_count(ui, "told_beliefs", store.told_beliefs.len());
            render_debug_count(ui, "heard_beliefs", store.heard_beliefs.len());
            render_debug_count(ui, "asked_witnesses", store.asked_witnesses.len());
            render_place_visits(ui, world, store);
            render_debug_count(
                ui,
                "institutional_beliefs",
                store.institutional_beliefs.len(),
            );
        });
}

fn render_entity_claims(ui: &mut Ui, store: &AgentBeliefStore, current_tick: Tick) {
    let rows = entity_claim_rows(store, current_tick);
    egui::CollapsingHeader::new(format!("entity_claims ({})", rows.len()))
        .default_open(true)
        .show(ui, |ui| {
            if rows.is_empty() {
                ui.label("empty");
                return;
            }
            egui::Grid::new("belief_entity_claims_grid")
                .num_columns(5)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Subject");
                    ui.strong("Aspect");
                    ui.strong("Confidence");
                    ui.strong("Acquired");
                    ui.strong("Freshness");
                    ui.end_row();
                    for row in rows {
                        ui.monospace(row.subject.to_string());
                        ui.label(format!("{:?}", row.aspect));
                        ui.label(row.confidence.to_string());
                        ui.label(row.acquired_tick.0.to_string());
                        ui.label(row.freshness_ticks.to_string());
                        ui.end_row();
                    }
                });
        });
}

fn render_place_visits(ui: &mut Ui, world: &World, store: &AgentBeliefStore) {
    let rows = place_visit_rows(world, store);
    egui::CollapsingHeader::new(format!("place_visits ({})", rows.len()))
        .default_open(false)
        .show(ui, |ui| {
            if rows.is_empty() {
                ui.label("empty");
                return;
            }
            egui::Grid::new("belief_place_visits_grid")
                .num_columns(4)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Place");
                    ui.strong("Last arrival");
                    ui.strong("Visits");
                    ui.strong("Ticks present");
                    ui.end_row();
                    for row in rows {
                        ui.label(row.place_name);
                        ui.label(row.last_arrival_tick.0.to_string());
                        ui.label(row.visit_count.to_string());
                        ui.label(row.ticks_present.to_string());
                        ui.end_row();
                    }
                });
        });
}

fn render_debug_count(ui: &mut Ui, label: &str, len: usize) {
    egui::CollapsingHeader::new(format!("{label} ({len})"))
        .default_open(false)
        .show(ui, |ui| {
            if len == 0 {
                ui.label("empty");
            } else {
                ui.label("entries available");
            }
        });
}

fn render_last_seen_memory(ui: &mut Ui, world: &World, agent_id: EntityId) {
    egui::CollapsingHeader::new("LastSeenMemory")
        .default_open(false)
        .show(ui, |ui| {
            let Some(memory) = world.get_component_last_seen_memory(agent_id) else {
                ui.label("absent");
                return;
            };
            ui.label(format!("capacity {}", memory.capacity));
            egui::Grid::new("last_seen_memory_grid")
                .num_columns(4)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Subject");
                    ui.strong("Place");
                    ui.strong("Observed");
                    ui.strong("Provenance");
                    ui.end_row();
                    for record in memory.records.values() {
                        ui.monospace(record.subject.to_string());
                        ui.label(worldwake_cli::display::entity_display_name(
                            world,
                            record.place,
                        ));
                        ui.label(record.observed_tick.0.to_string());
                        ui.label(format!("{:?}", record.provenance));
                        ui.end_row();
                    }
                });
        });
}

fn render_expectation_store(ui: &mut Ui, world: &World, agent_id: EntityId) {
    egui::CollapsingHeader::new("ExpectationStore")
        .default_open(false)
        .show(ui, |ui| {
            let Some(store) = world.get_component_expectation_store(agent_id) else {
                ui.label("absent");
                return;
            };
            egui::Grid::new("expectation_store_grid")
                .num_columns(5)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Id");
                    ui.strong("Subject");
                    ui.strong("Expected place");
                    ui.strong("Deadline");
                    ui.strong("State");
                    ui.end_row();
                    for record in store.records.values() {
                        ui.label(record.id.to_string());
                        ui.monospace(record.subject.to_string());
                        ui.label(worldwake_cli::display::entity_display_name(
                            world,
                            record.expected_place,
                        ));
                        ui.label(record.deadline_tick.0.to_string());
                        ui.label(format!("{:?}", record.state));
                        ui.end_row();
                    }
                });
        });
}

fn render_source_reliability(ui: &mut Ui, world: &World, agent_id: EntityId) {
    egui::CollapsingHeader::new("SourceReliability")
        .default_open(false)
        .show(ui, |ui| {
            let Some(reliability) = world.get_component_source_reliability(agent_id) else {
                ui.label("absent");
                return;
            };
            egui::Grid::new("source_reliability_grid")
                .num_columns(5)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Source");
                    ui.strong("Commodity");
                    ui.strong("Successes");
                    ui.strong("Failures");
                    ui.strong("Last attempt");
                    ui.end_row();
                    for (source, record) in &reliability.sources {
                        ui.label(worldwake_cli::display::entity_display_name(
                            world,
                            source.entity,
                        ));
                        ui.label(format!("{:?}", source.commodity));
                        ui.label(record.successful_acquisitions.to_string());
                        ui.label(record.failed_attempts.to_string());
                        ui.label(record.last_attempt_tick.0.to_string());
                        ui.end_row();
                    }
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{VisualizerApp, VisualizerCli};

    #[test]
    fn beliefs_tab_renders_each_source_section() {
        let app = baseline_app();
        let world = app.world().expect("scenario is loaded");
        let agent = first_agent(world);

        assert_eq!(
            source_presence(world, agent),
            BeliefSourcePresence {
                agent_belief_store: world.get_component_agent_belief_store(agent).is_some(),
                last_seen_memory: world.get_component_last_seen_memory(agent).is_some(),
                expectation_store: world.get_component_expectation_store(agent).is_some(),
                source_reliability: world.get_component_source_reliability(agent).is_some(),
            }
        );
    }

    #[test]
    fn beliefs_tab_entity_claims_render_aspect_and_confidence() {
        let app = baseline_app_with_claims();
        let world = app.world().expect("scenario is loaded");
        let agent = world
            .entities_with_name_and_agent_data()
            .find(|agent| {
                world
                    .get_component_agent_belief_store(*agent)
                    .is_some_and(|store| !store.entity_claims.is_empty())
            })
            .expect("baseline produces an agent with entity claims");
        let store = world
            .get_component_agent_belief_store(agent)
            .expect("agent has belief store");
        let current_tick = app.current_snapshot().expect("snapshot exists").tick;
        let rows = entity_claim_rows(store, current_tick);
        let first_claim = store
            .entity_claims
            .values()
            .flat_map(|claims| claims.iter())
            .next()
            .expect("store has at least one claim");

        assert!(rows.iter().any(|row| {
            row.aspect == first_claim.aspect && row.confidence == first_claim.confidence
        }));
    }

    fn first_agent(world: &World) -> EntityId {
        world
            .entities_with_name_and_agent_data()
            .next()
            .expect("baseline has an agent")
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

    fn baseline_app_with_claims() -> VisualizerApp {
        let mut app = baseline_app();
        for _ in 0..200 {
            if app
                .world()
                .expect("scenario is loaded")
                .entities_with_name_and_agent_data()
                .any(|agent| {
                    app.world()
                        .expect("scenario is loaded")
                        .get_component_agent_belief_store(agent)
                        .is_some_and(|store| !store.entity_claims.is_empty())
                })
            {
                return app;
            }
            app.step_one_tick();
        }
        app
    }
}
