//! Event log command handlers: events, event, trace.
//!
//! All handlers are read-only — zero world mutation.

use worldwake_core::{
    cause::CauseRef,
    event_log::EventLog,
    ids::EventId,
};
use worldwake_sim::SimulationState;

use crate::commands::{CommandError, CommandOutcome, CommandResult};
use crate::display::entity_display_name;

/// Format a single event as a summary line.
///
/// Example: `[E42] tick 15 — ActionCommitted by Kael`
fn format_event_summary(sim: &SimulationState, event_id: EventId) -> Option<String> {
    let record = sim.event_log().get(event_id)?;
    let world = sim.world();

    let tags: Vec<String> = record.tags.iter().map(|t| format!("{t:?}")).collect();
    let tag_str = if tags.is_empty() {
        "(no tags)".to_string()
    } else {
        tags.join(", ")
    };

    let actor_str = match record.actor_id {
        Some(actor) => format!(" by {}", entity_display_name(world, actor)),
        None => String::new(),
    };

    Some(format!(
        "[E{}] tick {} — {}{actor_str}",
        event_id.0, record.tick.0, tag_str
    ))
}

/// Format a `CauseRef` for display.
fn format_cause(cause: &CauseRef) -> String {
    match cause {
        CauseRef::Event(id) => format!("event E{}", id.0),
        CauseRef::SystemTick(tick) => format!("system tick {}", tick.0),
        CauseRef::Bootstrap => "bootstrap".to_string(),
        CauseRef::ExternalInput(seq) => format!("external input {seq}"),
    }
}

/// Show the last `n` events from the event log.
///
/// Defaults to 10 if `n` is None. Displays newest first.
#[allow(clippy::unnecessary_wraps)] // Must return CommandResult for dispatch interface.
pub fn handle_events(sim: &SimulationState, n: Option<usize>) -> CommandResult {
    let count = n.unwrap_or(10);
    let log = sim.event_log();
    let total = log.len();

    if total == 0 {
        println!("No events recorded.");
        return Ok(CommandOutcome::Continue);
    }

    let start = total.saturating_sub(count);

    println!("Events ({} of {total}):", total - start);
    // Show newest first (reverse order).
    for i in (start..total).rev() {
        let event_id = EventId(u64::try_from(i).expect("event index fits u64"));
        if let Some(line) = format_event_summary(sim, event_id) {
            println!("  {line}");
        }
    }

    Ok(CommandOutcome::Continue)
}

/// Show full details for a single event by ID.
pub fn handle_event(sim: &SimulationState, id: u64) -> CommandResult {
    let event_id = EventId(id);
    let log = sim.event_log();

    let Some(record) = log.get(event_id) else {
        return Err(CommandError::new(format!(
            "Event E{id} not found (log has {} events)",
            log.len()
        )));
    };

    let world = sim.world();

    println!("Event [E{id}]");
    println!("  tick: {}", record.tick.0);

    // Tags
    let tags: Vec<String> = record.tags.iter().map(|t| format!("{t:?}")).collect();
    println!(
        "  tags: {}",
        if tags.is_empty() {
            "(none)".to_string()
        } else {
            tags.join(", ")
        }
    );

    // Cause
    println!("  cause: {}", format_cause(&record.cause));

    // Actor
    match record.actor_id {
        Some(actor) => println!("  actor: {}", entity_display_name(world, actor)),
        None => println!("  actor: (none)"),
    }

    // Place
    match record.place_id {
        Some(place) => println!("  place: {}", entity_display_name(world, place)),
        None => println!("  place: (none)"),
    }

    // Targets
    if record.target_ids.is_empty() {
        println!("  targets: (none)");
    } else {
        let names: Vec<String> = record
            .target_ids
            .iter()
            .map(|id| entity_display_name(world, *id))
            .collect();
        println!("  targets: {}", names.join(", "));
    }

    // Witnesses
    let direct = &record.witness_data.direct_witnesses;
    if direct.is_empty() {
        println!("  witnesses: (none)");
    } else {
        let names: Vec<String> = direct
            .iter()
            .map(|id| entity_display_name(world, *id))
            .collect();
        println!("  witnesses: {}", names.join(", "));
    }

    // State deltas
    if record.state_deltas.is_empty() {
        println!("  deltas: (none)");
    } else {
        println!("  deltas ({}):", record.state_deltas.len());
        for delta in &record.state_deltas {
            println!("    {delta:?}");
        }
    }

    Ok(CommandOutcome::Continue)
}

/// Trace the causal chain backward from an event to its root.
///
/// Walks `CauseRef::Event` links backward, printing each event with
/// increasing indentation. Caps at 100 hops with a warning.
pub fn handle_trace(sim: &SimulationState, id: u64) -> CommandResult {
    let event_id = EventId(id);
    let log = sim.event_log();

    if log.get(event_id).is_none() {
        return Err(CommandError::new(format!(
            "Event E{id} not found (log has {} events)",
            log.len()
        )));
    }

    let chain = trace_cause_chain_capped(log, event_id);
    let capped = chain.len() >= MAX_TRACE_HOPS;

    println!("Causal trace from [E{id}]:");
    for (depth, eid) in chain.iter().enumerate() {
        let indent = "  ".repeat(depth + 1);
        let prefix = if depth == 0 { "" } else { "<- " };
        if let Some(line) = format_event_summary(sim, *eid) {
            println!("{indent}{prefix}{line}");
        }
    }

    if capped {
        println!(
            "  ... trace capped at {MAX_TRACE_HOPS} hops (chain may be longer)"
        );
    }

    Ok(CommandOutcome::Continue)
}

/// Maximum hops for causal trace to guard against unreasonably long chains.
const MAX_TRACE_HOPS: usize = 100;

/// Walk the cause chain backward, capped at `MAX_TRACE_HOPS`.
///
/// Similar to `EventLog::trace_cause_chain` but with a hop limit.
fn trace_cause_chain_capped(log: &EventLog, start: EventId) -> Vec<EventId> {
    let mut chain = Vec::new();
    let mut current = start;

    for _ in 0..MAX_TRACE_HOPS {
        let Some(record) = log.get(current) else {
            break;
        };
        chain.push(current);

        match record.cause {
            CauseRef::Event(cause_id) => current = cause_id,
            CauseRef::Bootstrap | CauseRef::SystemTick(_) | CauseRef::ExternalInput(_) => break,
        }
    }

    chain
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::{spawn_scenario, types::*};
    use worldwake_core::{
        cause::CauseRef,
        control::ControlSource,
        event_record::PendingEvent,
        event_tag::EventTag,
        ids::EventId,
        topology::PlaceTag,
        visibility::VisibilitySpec,
        witness::WitnessData,
    };
    use std::collections::BTreeSet;

    fn minimal_scenario() -> ScenarioDef {
        ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Village".into(),
                tags: vec![PlaceTag::Village],
            }],
            edges: vec![],
            agents: vec![AgentDef {
                name: "Kael".into(),
                location: "Village".into(),
                control: ControlSource::Ai,
                needs: None,
                combat_profile: None,
                utility_profile: None,
                merchandise_profile: None,
                trade_disposition: None,
            }],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
        }
    }

    fn emit_bootstrap(sim: &mut SimulationState) -> EventId {
        let pending = PendingEvent::new(
            sim.scheduler().current_tick(),
            CauseRef::Bootstrap,
            None,
            vec![],
            None,
            Vec::new(),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
            BTreeSet::from([EventTag::System]),
        );
        sim.event_log_mut().emit(pending)
    }

    fn emit_caused(sim: &mut SimulationState, cause: EventId) -> EventId {
        let pending = PendingEvent::new(
            sim.scheduler().current_tick(),
            CauseRef::Event(cause),
            None,
            vec![],
            None,
            Vec::new(),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
            BTreeSet::from([EventTag::ActionStarted]),
        );
        sim.event_log_mut().emit(pending)
    }

    #[test]
    fn test_events_shows_recent() {
        let spawned = spawn_scenario(&minimal_scenario()).unwrap();
        let mut sim = spawned.state;
        emit_bootstrap(&mut sim);

        let result = handle_events(&sim, None);
        assert!(result.is_ok());
        assert!(!sim.event_log().is_empty());
    }

    #[test]
    fn test_events_default_count() {
        let spawned = spawn_scenario(&minimal_scenario()).unwrap();
        let mut sim = spawned.state;
        let baseline = sim.event_log().len();

        // Emit 3 events (total will be fewer than default 10).
        for _ in 0..3 {
            emit_bootstrap(&mut sim);
        }

        // Should show all events (fewer than default 10).
        let result = handle_events(&sim, None);
        assert!(result.is_ok());
        assert_eq!(sim.event_log().len(), baseline + 3);
        assert!(sim.event_log().len() <= 10);
    }

    #[test]
    fn test_events_custom_count() {
        let spawned = spawn_scenario(&minimal_scenario()).unwrap();
        let mut sim = spawned.state;

        for _ in 0..10 {
            emit_bootstrap(&mut sim);
        }

        // Requesting 3 when 10 exist — should succeed.
        let result = handle_events(&sim, Some(3));
        assert!(result.is_ok());
    }

    #[test]
    fn test_event_details() {
        let spawned = spawn_scenario(&minimal_scenario()).unwrap();
        let mut sim = spawned.state;
        let eid = emit_bootstrap(&mut sim);

        let result = handle_event(&sim, eid.0);
        assert!(result.is_ok());

        // Verify event record exists with expected fields.
        let record = sim.event_log().get(eid).unwrap();
        assert_eq!(record.tick, sim.scheduler().current_tick());
        assert!(record.tags.contains(&EventTag::System));
    }

    #[test]
    fn test_event_not_found() {
        let spawned = spawn_scenario(&minimal_scenario()).unwrap();
        let sim = spawned.state;

        let result = handle_event(&sim, 9999);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.message.contains("not found"),
            "error should mention 'not found': {err}"
        );
    }

    #[test]
    fn test_trace_walks_backward() {
        let spawned = spawn_scenario(&minimal_scenario()).unwrap();
        let mut sim = spawned.state;

        let root = emit_bootstrap(&mut sim);
        let child = emit_caused(&mut sim, root);
        let grandchild = emit_caused(&mut sim, child);

        // Trace from grandchild should walk backward through the chain.
        let result = handle_trace(&sim, grandchild.0);
        assert!(result.is_ok());

        // Verify the chain is correct via EventLog API.
        let chain = sim.event_log().trace_cause_chain(grandchild);
        assert_eq!(chain, vec![grandchild, child, root]);
    }

    #[test]
    fn test_trace_root_event() {
        let spawned = spawn_scenario(&minimal_scenario()).unwrap();
        let mut sim = spawned.state;

        let root = emit_bootstrap(&mut sim);

        // Trace from root should show single event (no cause).
        let result = handle_trace(&sim, root.0);
        assert!(result.is_ok());

        let chain = sim.event_log().trace_cause_chain(root);
        assert_eq!(chain, vec![root]);
    }

    #[test]
    fn test_trace_not_found() {
        let spawned = spawn_scenario(&minimal_scenario()).unwrap();
        let sim = spawned.state;

        let result = handle_trace(&sim, 9999);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.message.contains("not found"),
            "error should mention 'not found': {err}"
        );
    }
}
