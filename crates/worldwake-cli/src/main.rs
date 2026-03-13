use clap::Parser;
use std::path::PathBuf;
use std::process;
use worldwake_ai::{AgentTickDriver, PlanningBudget};
use worldwake_cli::repl::run_repl;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario};

#[derive(Parser)]
#[command(name = "worldwake", about = "Causality-first emergent micro-world simulation")]
struct Cli {
    /// Path to RON scenario file
    scenario: PathBuf,
}

fn main() {
    let cli = Cli::parse();

    let def = match load_scenario_file(&cli.scenario) {
        Ok(def) => def,
        Err(e) => {
            eprintln!("Failed to load scenario: {e}");
            process::exit(1);
        }
    };

    let spawned = match spawn_scenario(&def) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to spawn scenario: {e}");
            process::exit(1);
        }
    };

    let mut sim = spawned.state;
    let mut driver = AgentTickDriver::new(PlanningBudget::default());

    if let Err(e) = run_repl(&mut sim, &mut driver, &spawned.action_registries, &spawned.dispatch_table) {
        eprintln!("REPL error: {e}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_args_parse() {
        let cli = Cli::parse_from(["worldwake", "scenarios/default.ron"]);
        assert_eq!(cli.scenario, PathBuf::from("scenarios/default.ron"));
    }
}
