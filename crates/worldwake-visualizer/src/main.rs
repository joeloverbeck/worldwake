use clap::Parser;
use worldwake_visualizer::app::{VisualizerApp, VisualizerCli};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = VisualizerCli::parse();
    let app = VisualizerApp::new(cli)?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title("Worldwake Visualizer"),
        ..Default::default()
    };

    eframe::run_native(
        "worldwake-visualizer",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )?;
    Ok(())
}
