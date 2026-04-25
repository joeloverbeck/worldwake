use std::collections::BTreeMap;

use egui::Ui;
use worldwake_core::{CommodityKind, EntityId, Quantity, Tick, World};

use crate::app::VisualizerApp;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InventoryRow {
    pub lot_entity: EntityId,
    pub commodity: CommodityKind,
    pub quantity: Quantity,
    pub ground_since: Option<Tick>,
}

pub fn render(ui: &mut Ui, app: &VisualizerApp, agent_id: EntityId) {
    let Some(world) = app.world() else {
        ui.label("No scenario loaded");
        return;
    };

    let rows = inventory_rows(world, agent_id);
    ui.heading("Inventory");
    if rows.is_empty() {
        ui.label("No carried item lots");
        return;
    }

    egui::Grid::new("inventory_lots_grid")
        .num_columns(4)
        .striped(true)
        .show(ui, |ui| {
            ui.strong("Commodity");
            ui.strong("Quantity");
            ui.strong("LotEntity");
            ui.strong("GroundSince");
            ui.end_row();
            for row in &rows {
                ui.label(format!("{:?}", row.commodity));
                ui.label(row.quantity.to_string());
                ui.monospace(row.lot_entity.to_string());
                ui.label(
                    row.ground_since
                        .map_or_else(|| "-".to_string(), |tick| tick.0.to_string()),
                );
                ui.end_row();
            }
        });

    ui.separator();
    ui.heading("Totals");
    egui::Grid::new("inventory_totals_grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            for (commodity, quantity) in inventory_totals(&rows) {
                ui.label(format!("{commodity:?}"));
                ui.label(quantity.to_string());
                ui.end_row();
            }
        });
}

pub(crate) fn inventory_rows(world: &World, agent_id: EntityId) -> Vec<InventoryRow> {
    world
        .possessions_of(agent_id)
        .into_iter()
        .filter_map(|lot_entity| {
            let lot = world.get_component_item_lot(lot_entity)?;
            Some(InventoryRow {
                lot_entity,
                commodity: lot.commodity,
                quantity: lot.quantity,
                ground_since: world
                    .get_component_ground_since(lot_entity)
                    .map(|ground_since| ground_since.0),
            })
        })
        .collect()
}

fn inventory_totals(rows: &[InventoryRow]) -> BTreeMap<CommodityKind, Quantity> {
    let mut totals = BTreeMap::new();
    for row in rows {
        totals
            .entry(row.commodity)
            .and_modify(|quantity: &mut Quantity| *quantity = *quantity + row.quantity)
            .or_insert(row.quantity);
    }
    totals
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{VisualizerApp, VisualizerCli};

    #[test]
    fn inventory_tab_renders_possessions() {
        let app = baseline_app_with_possession();
        let world = app.world().expect("scenario is loaded");
        let agent = world
            .entities_with_name_and_agent_data()
            .find(|agent| !world.possessions_of(*agent).is_empty())
            .expect("baseline has an agent with possessions");

        assert_eq!(
            inventory_rows(world, agent).len(),
            world.possessions_of(agent).len()
        );
    }

    #[test]
    fn inventory_totals_group_by_commodity() {
        let app = baseline_app_with_possession();
        let world = app.world().expect("scenario is loaded");
        let agent = world
            .entities_with_name_and_agent_data()
            .find(|agent| !world.possessions_of(*agent).is_empty())
            .expect("baseline has an agent with possessions");
        let rows = inventory_rows(world, agent);

        let summed = inventory_totals(&rows);

        assert_eq!(
            summed.values().map(|quantity| quantity.0).sum::<u32>(),
            rows.iter().map(|row| row.quantity.0).sum::<u32>()
        );
    }

    fn baseline_app_with_possession() -> VisualizerApp {
        let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("scenarios/survival-baseline.ron");
        let mut app = VisualizerApp::new(VisualizerCli {
            scenario: Some(scenario),
            ignore_lints: true,
        })
        .expect("visualizer app constructs");
        for _ in 0..200 {
            if app
                .world()
                .expect("scenario is loaded")
                .entities_with_name_and_agent_data()
                .any(|agent| !app.world().unwrap().possessions_of(agent).is_empty())
            {
                return app;
            }
            app.step_one_tick();
        }
        app
    }
}
