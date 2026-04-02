use clap::Parser;
use std::path::{Path, PathBuf};
use std::process;
use worldwake_ai::{AgentTickDriver, PlanningBudget};
use worldwake_cli::repl::{run_repl, run_single_command};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario};

fn default_scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .map_or_else(
            || PathBuf::from("scenarios/default.ron"),
            |workspace_root| workspace_root.join("scenarios/default.ron"),
        )
}

#[derive(Parser)]
#[command(
    name = "worldwake",
    about = "Causality-first emergent micro-world simulation"
)]
struct Cli {
    /// Path to RON scenario file
    #[arg(default_value_os_t = default_scenario_path())]
    scenario: PathBuf,

    /// Execute a single command and exit (non-interactive mode).
    /// Use with --state to persist between invocations.
    #[arg(long)]
    exec: Option<String>,

    /// Path to state file for persisting between --exec invocations.
    /// On each invocation: loads state if file exists, saves state after command.
    #[arg(long)]
    state: Option<PathBuf>,
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

    if let Some(ref command) = cli.exec {
        // Single-command execution mode.
        // Load persisted state if --state file exists.
        if let Some(ref state_path) = cli.state {
            if state_path.exists() {
                load_state_file(state_path, &mut sim, &mut driver);
            }
        }

        // Run the single command.
        if let Err(e) = run_single_command(
            &mut sim,
            &mut driver,
            &spawned.action_registries,
            &spawned.dispatch_table,
            command,
        ) {
            eprintln!("exec error: {e}");
            process::exit(1);
        }

        // Save state if --state provided.
        if let Some(ref state_path) = cli.state {
            if let Err(e) = worldwake_sim::save(&sim, Some(&driver), state_path) {
                eprintln!("state save failed: {e}");
                process::exit(1);
            }
        }
    } else {
        // Normal REPL mode.
        if let Err(e) = run_repl(
            &mut sim,
            &mut driver,
            &spawned.action_registries,
            &spawned.dispatch_table,
        ) {
            eprintln!("REPL error: {e}");
            process::exit(1);
        }
    }
}

fn load_state_file(
    state_path: &Path,
    sim: &mut worldwake_sim::SimulationState,
    driver: &mut AgentTickDriver,
) {
    match worldwake_sim::load(state_path) {
        Ok((loaded, runtime_bytes)) => {
            *sim = loaded;
            if let Some(bytes) = runtime_bytes.as_deref() {
                match AgentTickDriver::from_saved_runtime(bytes, sim.world()) {
                    Ok(restored) => *driver = restored,
                    Err(err) => {
                        eprintln!("warning: could not restore AI driver: {err}");
                        *driver = AgentTickDriver::new(PlanningBudget::default());
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("state load failed: {e}");
            process::exit(1);
        }
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

    #[test]
    fn test_cli_args_default_to_bundled_scenario() {
        let cli = Cli::parse_from(["worldwake"]);
        assert_eq!(cli.scenario, default_scenario_path());
    }
}
