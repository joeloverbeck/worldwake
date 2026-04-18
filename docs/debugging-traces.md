# Debugging with Trace Systems

Two complementary trace systems help debug agent behavior and action execution.

## Decision Traces (AI reasoning)

Use when asking "Why did/didn't the agent do X?" — covers the full decision pipeline: candidate generation, ranking, plan search, selection, and execution outcome.

### Quick start in golden tests

```rust
// 1. Enable tracing on the driver before stepping.
h.driver.enable_tracing();

// 2. Run ticks as normal.
for _ in 0..20 { h.step_once(); }

// 3. Query traces.
let sink = h.driver.trace_sink().unwrap();

// Per-agent per-tick lookup:
let trace = sink.trace_at(agent, Tick(5)).unwrap();

// All traces for one agent:
let agent_traces = sink.traces_for(agent);

// Check what candidates were generated:
if let DecisionOutcome::Planning(ref p) = trace.outcome {
    eprintln!("candidates: {:?}", p.candidates.ranked);
    eprintln!("plan search: {:?}", p.planning.attempts);
}

// Human-readable dump to stderr:
sink.dump_agent(agent, &h.defs);

// One-line summary per outcome:
eprintln!("{}", trace.outcome.summary());
```

### Key queries

- `sink.traces_for(agent)` — all traces for one agent
- `sink.trace_at(agent, tick)` — single tick lookup
- `trace.outcome.summary()` — one-line human-readable string
- `DecisionOutcome::Planning(p)` — inspect `p.candidates`, `p.planning.attempts`, `p.selection`

### When to reach for decision traces

- "Why did/didn't agent X do Y?" → check `candidates.generated` and `planning.attempts`
- "Why did the agent switch goals?" → check `InterruptTrace` on `ActiveAction` outcomes
- "Why did plan search fail?" → check `PlanSearchOutcome` variants (`BudgetExhausted`, `FrontierExhausted`, `Unsupported`)

Decision traces are the first stop for AI reasoning, not the only stop. If the trace shows the selected outcome but does not expose the concrete world facts keeping that branch alive, drop to the shared lower-layer state/query tests before adding ad-hoc instrumentation. If that missing provenance is architecturally important rather than just inconvenient for one test, write a follow-up traceability ticket instead of papering over it locally.

Tracing is opt-in and zero-cost when disabled. Do not leave `enable_tracing()` in committed test code unless the test explicitly asserts on trace data.

## Action Traces (execution lifecycle)

Use when asking "Did the action run?", "When did it complete?", "Why was it aborted?" — covers what happened when the action executed.

### Quick start in golden tests

```rust
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
```

Key types: `ActionTraceSink`, `ActionTraceEvent`, `ActionTraceKind` (Started, Committed, Aborted, StartFailed).

Action tracing is opt-in and zero-cost when disabled. Do not leave `enable_action_tracing()` in committed test code unless the test explicitly asserts on trace data.

## Tick alignment

Both trace systems key events to the tick being processed (N), not the post-step tick (N+1).

`step_once()` processes tick N (the value of `scheduler.current_tick()` before stepping) and increments to N+1. To query the trace for a just-processed tick, capture `let processed_tick = h.scheduler.current_tick()` *before* calling `h.step_once()`, then use `sink.trace_at(agent, processed_tick)`. Using `scheduler.current_tick()` *after* `step_once()` queries tick N+1, which has no trace yet.

## Which trace to use

| Question | Use |
|----------|-----|
| "Why did the agent choose to loot?" | Decision trace (`h.driver.enable_tracing()`) |
| "Did the loot action actually execute?" | Action trace (`h.enable_action_tracing()`) |
| "How long did the action take?" | Action trace — compare Started tick vs Committed tick |
| "Why was the action aborted?" | Action trace — check `ActionTraceKind::Aborted { reason }` |
| "What items were created?" | Action trace — check `CommitOutcome::materializations` |
| "Why wasn't the controller installed as holder?" | Politics trace — check `ForceInstallationDeferred { reason }` |

## Golden test observation strategy

- **1-tick actions** (e.g., loot, eat): Complete within a single `step_once()` call. Use **state-delta observation** (check item ownership changes between ticks) or action traces. Do NOT rely on `agent_active_action_name()` — the action won't be visible between ticks.
- **Multi-tick actions** (e.g., harvest, travel, craft): Visible as active between ticks. Use `agent_active_action_name()` or action traces.
- **When in doubt**: Enable action tracing and check `events_for_at(agent, tick)` to see exactly what happened.

For same-tick cross-agent ordering, the contract is the explicit `ActionTraceEvent.sequence_in_tick` key — do not rewrite that contract as "later tick" unless strict tick separation is the intended engine rule.

## System Tick Ordering & Force-Control

### Tick System Execution Order

Systems run in this order each tick (defined in `system_manifest.rs`):

```
Needs → Production → Trade → Combat → FacilityQueue → Politics → Perception
```

The ordering is load-bearing. Key constraint: **Politics runs before Perception** so that institutional state changes (`OfficeController`, contested state, vacancy) are visible to co-located observers in the same tick via `force_control_claims_for_event()`. Without this, Perception cannot project institutional beliefs from political events (violates Principle 7).

### Force-Control Lifecycle

Force claims do not immediately transfer control. The lifecycle has distinct phases:

```
press_force_claim → hostility + ContestsOffice → (vacancy required) → succession processes → controller → holder
```

- `press_force_claim` creates `ContestsOffice` relation and `hostile_to(challenger, incumbent)` if an `office_holder` exists.
- The succession system (`evaluate_office_succession`) returns `OccupiedNoAction` while a living holder exists — force claims are NOT processed until the office vacates.
- After vacancy, the succession system evaluates pending force claims and establishes a controller.
- After uncontested hold for `succession_period_ticks`, the controller is installed as `office_holder`.

Golden tests that need both hostility (requires incumbent) AND controller establishment (requires vacancy) must include an explicit vacancy step between them.

## Critical Window Forensics

Decision traces and action traces remain the primary raw debugging surfaces, but prolonged survival failures usually need one more layer: a stable read-model over an entire authored-critical window rather than a pile of single-tick facts.

Use `worldwake_ai::{CriticalWindowReport, SurvivalForensicExtractor}` from `crates/worldwake-ai/src/survival_forensics.rs` for that composed view. The extractor bundles per-frame decision-trace snapshots, action-trace snapshots, blocker and exhaustion summaries, and authoritative local-place state with bounded capture rules so long windows stay readable.

For golden tests, prefer the shared helpers in `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs` and print `dump_reports_for_debug(&reports)` before inventing a one-off reproducer. For manual scenario inspection, the observer binary now renders the same surface in `## Section 9 — Critical Window Forensics` and exposes `--critical-window-top-n` to bound the frame table.

Cross-reference: `docs/golden-e2e-testing.md` section `Survival Critical-Window Forensics`.
