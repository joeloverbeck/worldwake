use egui::Ui;
use worldwake_core::EntityId;

use crate::app::VisualizerApp;

pub mod beliefs;
pub mod inventory;
pub mod needs;
pub mod overview;
pub mod plan;
pub mod traces;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DetailTab {
    Overview,
    Needs,
    Beliefs,
    Inventory,
    Plan,
    Traces,
}

impl DetailTab {
    pub const ALL: [Self; 6] = [
        Self::Overview,
        Self::Needs,
        Self::Beliefs,
        Self::Inventory,
        Self::Plan,
        Self::Traces,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Needs => "Needs",
            Self::Beliefs => "Beliefs",
            Self::Inventory => "Inventory",
            Self::Plan => "Plan",
            Self::Traces => "Traces",
        }
    }
}

pub fn render_tab(ui: &mut Ui, tab: DetailTab, app: &VisualizerApp, agent_id: EntityId) {
    match tab {
        DetailTab::Overview => overview::render(ui, app, agent_id),
        DetailTab::Needs => needs::render(ui, app, agent_id),
        DetailTab::Inventory => inventory::render(ui, app, agent_id),
        DetailTab::Beliefs => beliefs::render(ui, app, agent_id),
        DetailTab::Plan => plan::render(ui, app, agent_id),
        DetailTab::Traces => traces::render(ui, app, agent_id),
    }
}
