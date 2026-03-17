# Action Execution Trace Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add an opt-in action execution trace layer to `worldwake-sim` that records action lifecycle events (started, committed, aborted, start-failed), closing the Principle 27 debuggability gap for the causal path.

**Architecture:** An `ActionTraceSink` collects `ActionTraceEvent` records during `step_tick()`. The sink is threaded through `TickStepServices` → `TickStepRuntime` and populated at the 5 lifecycle hook points in `tick_step.rs`. Zero-cost when disabled (trace recording gated by `Option::is_some()`). Follows the proven `DecisionTraceSink` pattern from `worldwake-ai`.

**Tech Stack:** Rust, no new dependencies. Lives in `worldwake-sim` since the action framework is sim-level infrastructure.

---

## Reusable Patterns

- `DecisionTraceSink` in `crates/worldwake-ai/src/decision_trace.rs` — the pattern to follow for sink structure, query API, and dump output
- `TickStepRuntime` in `crates/worldwake-sim/src/tick_step.rs:23` — internal runtime struct where trace sink will live
- `TickStepServices` in `crates/worldwake-sim/src/tick_step.rs:15` — public services struct where user passes in the sink
- `GoldenHarness::step_once()` in `crates/worldwake-ai/tests/golden_harness/mod.rs:488` — where services are constructed for tests

## Design Summary

### Trace Types

```rust
// crates/worldwake-sim/src/action_trace.rs

pub struct ActionTraceEvent {
    pub tick: Tick,
    pub actor: EntityId,
    pub def_id: ActionDefId,
    pub action_name: String,
    pub kind: ActionTraceKind,
}

pub enum ActionTraceKind {
    Started {
        targets: Vec<EntityId>,
    },
    Committed {
        instance_id: ActionInstanceId,
        outcome: CommitOutcome,
    },
    Aborted {
        instance_id: ActionInstanceId,
        reason: String,
    },
    StartFailed {
        reason: String,
    },
}

pub struct ActionTraceSink {
    events: Vec<ActionTraceEvent>,
}
```

### Recording Hook Points (all in `tick_step.rs`)

| Event | Location | Data Source |
|-------|----------|-------------|
| `Started` | `apply_input()` after `start_affordance()` succeeds | `actor`, `def_id`, `targets` from `InputKind::RequestAction` |
| `StartFailed` | `apply_input()` at BestEffort failure | `actor`, `def_id` from input, `reason` from error |
| `Committed` | `progress_active_actions()` at `TickOutcome::Committed` | `instance` (cloned before tick), `outcome` from tick result |
| `Aborted` | `progress_active_actions()` at `TickOutcome::Aborted` | `instance`, `reason` from abort |
| `Aborted` | `apply_input()` at `CancelAction` | instance from scheduler lookup |
| `Aborted` | `abort_actions_for_dead_actors()` | dead actor's instance |

### Threading

`TickStepServices` gains `pub action_trace: Option<&'a mut ActionTraceSink>`. At the top of `step_tick()`, this is `take()`d into `TickStepRuntime`. Since `runtime` is always `&mut`, the sink gets mutable access in all internal functions.

---

### Task 1: Create `action_trace.rs` module with types and sink

**Files:**
- Create: `crates/worldwake-sim/src/action_trace.rs`
- Modify: `crates/worldwake-sim/src/lib.rs` — add module declaration and public re-exports

**Step 1: Create the trace module**

```rust
// crates/worldwake-sim/src/action_trace.rs
use crate::{ActionInstanceId, CommitOutcome};
use worldwake_core::{ActionDefId, EntityId, Tick};

/// A single action lifecycle event recorded during `step_tick()`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionTraceEvent {
    pub tick: Tick,
    pub actor: EntityId,
    pub def_id: ActionDefId,
    pub action_name: String,
    pub kind: ActionTraceKind,
}

/// The lifecycle transition that this trace event represents.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionTraceKind {
    /// Action was successfully started and is now active.
    Started {
        targets: Vec<EntityId>,
    },
    /// Action completed successfully via handler commit.
    Committed {
        instance_id: ActionInstanceId,
        outcome: CommitOutcome,
    },
    /// Action was aborted, interrupted, or cancelled.
    Aborted {
        instance_id: ActionInstanceId,
        reason: String,
    },
    /// Action start was requested but failed (BestEffort mode).
    StartFailed {
        reason: String,
    },
}

impl ActionTraceEvent {
    /// One-line human-readable summary (no registry lookups required).
    #[must_use]
    pub fn summary(&self) -> String {
        match &self.kind {
            ActionTraceKind::Started { targets } => {
                format!(
                    "tick {}: {} started '{}' targeting {:?}",
                    self.tick.0, self.actor, self.action_name, targets
                )
            }
            ActionTraceKind::Committed { instance_id, outcome } => {
                let mat_count = outcome.materializations.len();
                format!(
                    "tick {}: {} committed '{}' (instance {}, {} materializations)",
                    self.tick.0, self.actor, self.action_name, instance_id, mat_count
                )
            }
            ActionTraceKind::Aborted { instance_id, reason } => {
                format!(
                    "tick {}: {} aborted '{}' (instance {}, reason: {})",
                    self.tick.0, self.actor, self.action_name, instance_id, reason
                )
            }
            ActionTraceKind::StartFailed { reason } => {
                format!(
                    "tick {}: {} failed to start '{}' (reason: {})",
                    self.tick.0, self.actor, self.action_name, reason
                )
            }
        }
    }
}

/// Append-only collector for action execution traces.
///
/// Zero-cost when not created. When present, `step_tick()` records action
/// lifecycle events here. Query methods enable structured introspection
/// for debugging and golden test assertions.
pub struct ActionTraceSink {
    events: Vec<ActionTraceEvent>,
}

impl ActionTraceSink {
    #[must_use]
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn record(&mut self, event: ActionTraceEvent) {
        self.events.push(event);
    }

    #[must_use]
    pub fn events(&self) -> &[ActionTraceEvent] {
        &self.events
    }

    #[must_use]
    pub fn events_for(&self, actor: EntityId) -> Vec<&ActionTraceEvent> {
        self.events.iter().filter(|e| e.actor == actor).collect()
    }

    #[must_use]
    pub fn events_at(&self, tick: Tick) -> Vec<&ActionTraceEvent> {
        self.events.iter().filter(|e| e.tick == tick).collect()
    }

    #[must_use]
    pub fn events_for_at(&self, actor: EntityId, tick: Tick) -> Vec<&ActionTraceEvent> {
        self.events
            .iter()
            .filter(|e| e.actor == actor && e.tick == tick)
            .collect()
    }

    /// Most recent `Committed` event for an actor, if any.
    #[must_use]
    pub fn last_committed(&self, actor: EntityId) -> Option<&ActionTraceEvent> {
        self.events
            .iter()
            .rev()
            .find(|e| e.actor == actor && matches!(e.kind, ActionTraceKind::Committed { .. }))
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Dump all events for an agent to stderr (for interactive debugging).
    pub fn dump_agent(&self, actor: EntityId) {
        let agent_events = self.events_for(actor);
        if agent_events.is_empty() {
            eprintln!("[ActionTrace] No events for {actor}");
            return;
        }
        eprintln!("[ActionTrace] {} events for {actor}:", agent_events.len());
        for event in agent_events {
            eprintln!("  {}", event.summary());
        }
    }
}

impl Default for ActionTraceSink {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event(tick: u64, kind: ActionTraceKind) -> ActionTraceEvent {
        ActionTraceEvent {
            tick: Tick(tick),
            actor: EntityId { slot: 1, generation: 0 },
            def_id: ActionDefId(0),
            action_name: "eat".to_string(),
            kind,
        }
    }

    #[test]
    fn sink_starts_empty() {
        let sink = ActionTraceSink::new();
        assert!(sink.events().is_empty());
    }

    #[test]
    fn record_and_query_by_actor() {
        let mut sink = ActionTraceSink::new();
        let actor_a = EntityId { slot: 1, generation: 0 };
        let actor_b = EntityId { slot: 2, generation: 0 };

        sink.record(ActionTraceEvent {
            tick: Tick(1),
            actor: actor_a,
            def_id: ActionDefId(0),
            action_name: "eat".to_string(),
            kind: ActionTraceKind::Started { targets: vec![] },
        });
        sink.record(ActionTraceEvent {
            tick: Tick(1),
            actor: actor_b,
            def_id: ActionDefId(1),
            action_name: "loot".to_string(),
            kind: ActionTraceKind::Started { targets: vec![actor_a] },
        });

        assert_eq!(sink.events_for(actor_a).len(), 1);
        assert_eq!(sink.events_for(actor_b).len(), 1);
        assert_eq!(sink.events().len(), 2);
    }

    #[test]
    fn query_by_tick() {
        let mut sink = ActionTraceSink::new();
        sink.record(sample_event(1, ActionTraceKind::Started { targets: vec![] }));
        sink.record(sample_event(2, ActionTraceKind::Committed {
            instance_id: ActionInstanceId(1),
            outcome: CommitOutcome::empty(),
        }));

        assert_eq!(sink.events_at(Tick(1)).len(), 1);
        assert_eq!(sink.events_at(Tick(2)).len(), 1);
        assert_eq!(sink.events_at(Tick(3)).len(), 0);
    }

    #[test]
    fn last_committed_returns_most_recent() {
        let mut sink = ActionTraceSink::new();
        let actor = EntityId { slot: 1, generation: 0 };
        sink.record(ActionTraceEvent {
            tick: Tick(1),
            actor,
            def_id: ActionDefId(0),
            action_name: "eat".to_string(),
            kind: ActionTraceKind::Committed {
                instance_id: ActionInstanceId(1),
                outcome: CommitOutcome::empty(),
            },
        });
        sink.record(ActionTraceEvent {
            tick: Tick(3),
            actor,
            def_id: ActionDefId(1),
            action_name: "loot".to_string(),
            kind: ActionTraceKind::Committed {
                instance_id: ActionInstanceId(2),
                outcome: CommitOutcome::empty(),
            },
        });

        let last = sink.last_committed(actor).unwrap();
        assert_eq!(last.action_name, "loot");
        assert_eq!(last.tick, Tick(3));
    }

    #[test]
    fn summary_format_covers_all_variants() {
        let started = sample_event(1, ActionTraceKind::Started { targets: vec![] });
        assert!(started.summary().contains("started"));

        let committed = sample_event(2, ActionTraceKind::Committed {
            instance_id: ActionInstanceId(1),
            outcome: CommitOutcome::empty(),
        });
        assert!(committed.summary().contains("committed"));

        let aborted = sample_event(3, ActionTraceKind::Aborted {
            instance_id: ActionInstanceId(1),
            reason: "test".to_string(),
        });
        assert!(aborted.summary().contains("aborted"));

        let failed = sample_event(4, ActionTraceKind::StartFailed {
            reason: "precondition".to_string(),
        });
        assert!(failed.summary().contains("failed to start"));
    }

    #[test]
    fn clear_removes_all_events() {
        let mut sink = ActionTraceSink::new();
        sink.record(sample_event(1, ActionTraceKind::Started { targets: vec![] }));
        assert_eq!(sink.events().len(), 1);
        sink.clear();
        assert!(sink.events().is_empty());
    }
}
```

**Step 2: Register the module in lib.rs**

Add `pub mod action_trace;` to `crates/worldwake-sim/src/lib.rs` and add public re-exports for `ActionTraceSink`, `ActionTraceEvent`, `ActionTraceKind`.

**Step 3: Run unit tests**

Run: `cargo test -p worldwake-sim action_trace`
Expected: All 6 unit tests pass.

**Step 4: Run clippy**

Run: `cargo clippy -p worldwake-sim`
Expected: Clean.

---

### Task 2: Thread the trace sink through `step_tick()`

**Files:**
- Modify: `crates/worldwake-sim/src/tick_step.rs` — add trace sink to `TickStepServices` and `TickStepRuntime`, record events at lifecycle points

**Step 1: Add trace sink field to `TickStepServices`**

```rust
pub struct TickStepServices<'a> {
    pub action_defs: &'a ActionDefRegistry,
    pub action_handlers: &'a ActionHandlerRegistry,
    pub recipe_registry: &'a RecipeRegistry,
    pub systems: &'a SystemDispatchTable,
    pub input_producer: Option<&'a mut dyn TickInputProducer>,
    pub action_trace: Option<&'a mut ActionTraceSink>,
}
```

**Step 2: Add trace sink to `TickStepRuntime`**

```rust
struct TickStepRuntime<'a> {
    world: &'a mut World,
    event_log: &'a mut EventLog,
    scheduler: &'a mut Scheduler,
    rng: &'a mut DeterministicRng,
    action_trace: Option<&'a mut ActionTraceSink>,
}
```

**Step 3: Take the sink in `step_tick()` and pass to runtime**

In `step_tick()`, after `let mut runtime = TickStepRuntime { ... }`:

```rust
let action_trace = services.action_trace.take();
let mut runtime = TickStepRuntime {
    world,
    event_log,
    scheduler,
    rng,
    action_trace,
};
```

**Step 4: Add a helper method on `TickStepRuntime`**

```rust
impl TickStepRuntime<'_> {
    fn record_action_trace(&mut self, event: ActionTraceEvent) {
        if let Some(sink) = self.action_trace.as_mut() {
            sink.record(event);
        }
    }
}
```

**Step 5: Record `Started` in `apply_input()` at RequestAction success**

After `start_affordance()` succeeds (around line 247), before returning `Ok(InputOutcome { actions_started: 1, ... })`:

```rust
let action_name = services
    .action_defs
    .get(def_id)
    .map_or_else(|| "unknown".to_owned(), |d| d.name.clone());
runtime.record_action_trace(ActionTraceEvent {
    tick,
    actor,
    def_id,
    action_name,
    kind: ActionTraceKind::Started {
        targets: targets.clone(),
    },
});
```

**Step 6: Record `StartFailed` in `apply_input()` at BestEffort failure**

After `record_action_start_failure()` (around line 242), add:

```rust
let action_name = services
    .action_defs
    .get(def_id)
    .map_or_else(|| "unknown".to_owned(), |d| d.name.clone());
runtime.record_action_trace(ActionTraceEvent {
    tick,
    actor,
    def_id,
    action_name,
    kind: ActionTraceKind::StartFailed {
        reason: format!("{err:?}"),
    },
});
```

**Step 7: Record `Committed` in `progress_active_actions()`**

At `TickOutcome::Committed { outcome }` (around line 401), after `retain_committed_action`:

```rust
let action_name = services
    .action_defs
    .get(instance.def_id)
    .map_or_else(|| "unknown".to_owned(), |d| d.name.clone());
runtime.record_action_trace(ActionTraceEvent {
    tick,
    actor: instance.actor,
    def_id: instance.def_id,
    action_name,
    kind: ActionTraceKind::Committed {
        instance_id,
        outcome: outcome.clone(),
    },
});
```

**Step 8: Record `Aborted` in `progress_active_actions()`**

At `TickOutcome::Aborted { reason, replan }` (around line 415):

```rust
let action_name = services
    .action_defs
    .get(instance.def_id)
    .map_or_else(|| "unknown".to_owned(), |d| d.name.clone());
runtime.record_action_trace(ActionTraceEvent {
    tick,
    actor: instance.actor,
    def_id: instance.def_id,
    action_name,
    kind: ActionTraceKind::Aborted {
        instance_id,
        reason: format!("{reason:?}"),
    },
});
```

**Step 9: Record `Aborted` in `apply_input()` at CancelAction**

After `abort_active_action()` succeeds (around line 274), look up the instance before it's aborted. Actually, the instance is already removed by the abort. Instead, look up the def_id from the scheduler before calling abort. Capture instance info before the abort call:

```rust
InputKind::CancelAction { actor, action_instance_id } => {
    validate_cancel_actor(runtime.scheduler, actor, action_instance_id)?;
    // Capture instance info before abort removes it.
    let cancel_def_id = runtime.scheduler.active_actions()
        .get(&action_instance_id)
        .map(|i| i.def_id);
    let replan = runtime.scheduler.abort_active_action(...)?;
    runtime.scheduler.retain_replan(replan);
    if let Some(def_id) = cancel_def_id {
        let action_name = services.action_defs
            .get(def_id)
            .map_or_else(|| "unknown".to_owned(), |d| d.name.clone());
        runtime.record_action_trace(ActionTraceEvent {
            tick,
            actor,
            def_id,
            action_name,
            kind: ActionTraceKind::Aborted {
                instance_id: action_instance_id,
                reason: format!("CancelledByInput {{ sequence_no: {sequence_no} }}"),
            },
        });
    }
    ...
}
```

**Step 10: Record `Aborted` in `abort_actions_for_dead_actors()`**

Before each `abort_active_action()` call, capture instance info:

```rust
for instance_id in action_ids {
    let dead_instance = runtime.scheduler.active_actions()
        .get(&instance_id)
        .cloned();
    let replan = runtime.scheduler.abort_active_action(...)?;
    runtime.scheduler.retain_replan(replan);
    if let Some(inst) = dead_instance {
        let action_name = services.action_defs
            .get(inst.def_id)
            .map_or_else(|| "unknown".to_owned(), |d| d.name.clone());
        runtime.record_action_trace(ActionTraceEvent {
            tick,
            actor: inst.actor,
            def_id: inst.def_id,
            action_name,
            kind: ActionTraceKind::Aborted {
                instance_id,
                reason: "ActorMarkedDead".to_string(),
            },
        });
    }
    aborted += 1;
}
```

**Step 11: Fix all callers that construct `TickStepServices`**

Search the codebase for `TickStepServices {` and add `action_trace: None` to each. Key locations:
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (GoldenHarness::step_once)
- Any other test or production code constructing TickStepServices

**Step 12: Compile and test**

Run: `cargo test --workspace`
Expected: All existing tests pass (action_trace is None everywhere, so zero behavioral change).

Run: `cargo clippy --workspace`
Expected: Clean.

---

### Task 3: Integrate into `GoldenHarness`

**Files:**
- Modify: `crates/worldwake-ai/tests/golden_harness/mod.rs` — add trace support to harness

**Step 1: Add trace field to GoldenHarness**

```rust
pub struct GoldenHarness {
    pub world: World,
    pub event_log: EventLog,
    pub scheduler: Scheduler,
    pub controller: ControllerState,
    pub rng: DeterministicRng,
    pub defs: ActionDefRegistry,
    pub handlers: ActionHandlerRegistry,
    pub recipes: RecipeRegistry,
    pub driver: AgentTickDriver,
    pub action_trace: Option<ActionTraceSink>,
}
```

Initialize as `None` in both `new()`, `with_recipes()`, and `from_simulation_state()`.

**Step 2: Add enable/query methods**

```rust
pub fn enable_action_tracing(&mut self) {
    self.action_trace = Some(ActionTraceSink::new());
}

pub fn action_trace_sink(&self) -> Option<&ActionTraceSink> {
    self.action_trace.as_ref()
}
```

**Step 3: Thread through `step_once()`**

```rust
pub fn step_once(&mut self) -> TickStepResult {
    let mut controllers = AutonomousControllerRuntime::new(vec![&mut self.driver]);
    step_tick(
        &mut self.world,
        &mut self.event_log,
        &mut self.scheduler,
        &mut self.controller,
        &mut self.rng,
        TickStepServices {
            action_defs: &self.defs,
            action_handlers: &self.handlers,
            recipe_registry: &self.recipes,
            systems: &dispatch_table(),
            input_producer: Some(&mut controllers),
            action_trace: self.action_trace.as_mut(),
        },
    )
    .unwrap()
}
```

**Step 4: Compile**

Run: `cargo test -p worldwake-ai --test golden_combat -- --list`
Expected: All tests listed, no compile errors.

---

### Task 4: Write a golden test exercising the trace API

**Files:**
- Modify: `crates/worldwake-ai/tests/golden_combat.rs` — add a test that validates trace output

This test proves the trace system works end-to-end by verifying that the multi-corpse loot scenario produces the expected trace events. It re-uses the existing `build_multi_corpse_loot_binding_scenario()`.

**Step 1: Add the test**

```rust
#[test]
fn golden_action_trace_records_loot_lifecycle() {
    let (mut h, _corpse_a, _corpse_b, looter, _, _) =
        build_multi_corpse_loot_binding_scenario(Seed([30; 32]));
    h.enable_action_tracing();

    for _ in 0..10 {
        h.step_once();
    }

    let sink = h.action_trace_sink().expect("action tracing should be enabled");
    let looter_events = sink.events_for(looter);

    // The looter should have at least 2 Started + 2 Committed events (one per corpse loot).
    let started_count = looter_events
        .iter()
        .filter(|e| matches!(e.kind, ActionTraceKind::Started { .. }))
        .count();
    let committed_count = looter_events
        .iter()
        .filter(|e| matches!(e.kind, ActionTraceKind::Committed { .. }))
        .count();

    assert!(
        started_count >= 2,
        "Looter should have at least 2 Started trace events (one per corpse); got {started_count}"
    );
    assert!(
        committed_count >= 2,
        "Looter should have at least 2 Committed trace events; got {committed_count}"
    );

    // Every Started event should have a matching Committed event at the same or later tick.
    for event in &looter_events {
        if let ActionTraceKind::Started { .. } = &event.kind {
            let has_commit = looter_events.iter().any(|e| {
                matches!(e.kind, ActionTraceKind::Committed { .. })
                    && e.action_name == event.action_name
                    && e.tick >= event.tick
            });
            assert!(
                has_commit,
                "Started '{}' at tick {} should have a matching Committed event",
                event.action_name, event.tick.0
            );
        }
    }

    // Verify loot actions specifically complete in the same tick they start
    // (this is the key insight that motivated the trace system).
    let loot_starts: Vec<_> = looter_events
        .iter()
        .filter(|e| e.action_name == "loot" && matches!(e.kind, ActionTraceKind::Started { .. }))
        .collect();
    let loot_commits: Vec<_> = looter_events
        .iter()
        .filter(|e| e.action_name == "loot" && matches!(e.kind, ActionTraceKind::Committed { .. }))
        .collect();

    assert_eq!(
        loot_starts.len(),
        loot_commits.len(),
        "Every loot start should have a corresponding commit"
    );

    for start in &loot_starts {
        let same_tick_commit = loot_commits.iter().any(|c| c.tick == start.tick);
        assert!(
            same_tick_commit,
            "Loot action started at tick {} should commit in the same tick (1-tick action)",
            start.tick.0
        );
    }
}
```

**Step 2: Run the test**

Run: `cargo test -p worldwake-ai --test golden_combat golden_action_trace_records_loot_lifecycle`
Expected: PASS.

**Step 3: Run full golden combat suite**

Run: `cargo test -p worldwake-ai --test golden_combat`
Expected: All 26 tests pass (25 existing + 1 new).

---

### Task 5: Update CLAUDE.md and AGENTS.md documentation

**Files:**
- Modify: `CLAUDE.md` — add action execution trace section
- Modify: `AGENTS.md` — add action execution trace section

**Step 1: Add section to CLAUDE.md**

Add after the "Debugging AI Decisions with Decision Traces" section:

```markdown
## Debugging Action Execution with Action Traces

When debugging action lifecycle issues (e.g., "Did this action start?", "When did it complete?", "Why was it aborted?"), use the **action execution trace system** in `worldwake-sim`. This complements the AI decision trace (which covers *why* an agent chose an action) by covering *what happened when the action ran*.

### How to use in golden tests

\```rust
// 1. Enable tracing on the harness before stepping.
h.enable_action_tracing();

// 2. Run ticks as normal.
for _ in 0..20 { h.step_once(); }

// 3. Query traces.
let sink = h.action_trace_sink().unwrap();

// Per-agent lookup:
let agent_events = sink.events_for(agent);

// Per-tick lookup:
let tick_events = sink.events_at(Tick(5));

// Combined:
let agent_tick_events = sink.events_for_at(agent, Tick(5));

// Last completed action for an agent:
let last = sink.last_committed(agent);

// Human-readable dump to stderr:
sink.dump_agent(agent);

// One-line summary per event:
for event in sink.events() {
    eprintln!("{}", event.summary());
}
\```

### When to use action traces vs decision traces

| Question | Use |
|----------|-----|
| "Why did the agent choose to loot?" | Decision trace (`h.driver.enable_tracing()`) |
| "Did the loot action actually execute?" | Action trace (`h.enable_action_tracing()`) |
| "How long did the action take?" | Action trace — compare Started tick vs Committed tick |
| "Why was the action aborted?" | Action trace — check `ActionTraceKind::Aborted { reason }` |
| "What items were created?" | Action trace — check `CommitOutcome::materializations` |

### Golden test observation strategy

- **1-tick actions** (e.g., loot, eat): Complete within a single `step_once()` call. Use **state-delta observation** (check item ownership changes between ticks) or action traces. Do NOT rely on `agent_active_action_name()` — the action won't be visible between ticks.
- **Multi-tick actions** (e.g., harvest, travel, craft): Visible as active between ticks. Use `agent_active_action_name()` or action traces.
- **When in doubt**: Enable action tracing and check `events_for_at(agent, tick)` to see exactly what happened.

Action tracing is opt-in and zero-cost when disabled. Do not leave `enable_action_tracing()` in committed test code unless the test explicitly asserts on trace data.
```

**Step 2: Add section to AGENTS.md**

Add after the "Debugging AI Decisions with Decision Traces" section:

```markdown
## Debugging Action Execution with Action Traces

For action lifecycle questions ("Did the action run?", "When did it complete?", "Why was it aborted?"), use the action execution trace system in `worldwake-sim`. Enable with `h.enable_action_tracing()` in golden tests. Query with `h.action_trace_sink().unwrap().events_for(agent)`.

Key types: `ActionTraceSink`, `ActionTraceEvent`, `ActionTraceKind` (Started, Committed, Aborted, StartFailed).

**Important**: Some actions (e.g., loot, eat) complete within a single tick. They are invisible to inter-tick `agent_active_action_name()` observation. Use action traces or state-delta checks for these.

See CLAUDE.md for detailed usage examples and the decision-trace vs action-trace guidance table.
```

**Step 3: Verify docs don't break any tooling**

Run: `cargo test --workspace && cargo clippy --workspace`
Expected: All pass (docs changes are markdown-only).

---

### Task 6: Final verification

**Step 1: Full workspace test**

Run: `cargo test --workspace`
Expected: All tests pass.

**Step 2: Clippy clean**

Run: `cargo clippy --workspace`
Expected: No warnings.

**Step 3: Verify trace is zero-cost when disabled**

Confirm by inspection: all `record_action_trace()` calls are gated by `if let Some(sink) = self.action_trace.as_mut()`. When `action_trace` is `None`, no trace objects are constructed.

---

## Verification Commands

```bash
# Unit tests for trace types
cargo test -p worldwake-sim action_trace

# Golden test exercising trace API
cargo test -p worldwake-ai --test golden_combat golden_action_trace_records_loot_lifecycle

# Full golden combat suite (regression)
cargo test -p worldwake-ai --test golden_combat

# Full workspace
cargo test --workspace
cargo clippy --workspace
```
