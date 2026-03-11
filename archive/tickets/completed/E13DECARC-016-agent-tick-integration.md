# E13DECARC-016: Agent tick integration and decision loop

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — integrates AI into the tick loop
**Deps**: E13DECARC-007, E13DECARC-008, E13DECARC-012, E13DECARC-013, E13DECARC-014, E13DECARC-015

## Problem

All the pieces need to be wired together into the per-tick decision loop for AI-controlled agents. This is the unified agent tick: derive pressures -> update dirty flag -> evaluate interrupts -> generate candidates -> rank -> plan -> revalidate -> emit `InputKind::RequestAction`. Dead agents, human-controlled agents, and idle states must be handled correctly.

## Assumption Reassessment (2026-03-11)

1. `ControlSource::Ai` marks AI agents — confirmed.
2. `InputEvent` and `InputKind::RequestAction` exist — confirmed.
3. `InputQueue` accepts deterministic `(tick, sequence_no)` ordering — confirmed.
4. `Scheduler` runs systems per tick — confirmed.
5. `SystemFn` signature takes `SystemExecutionContext` — confirmed.
6. `ReplanNeeded` is produced by action abort/interrupt paths — confirmed, but `step_tick()` currently drops those signals instead of retaining them for AI consumption.
7. `worldwake-ai` already implements most E13 planner pieces:
   - candidate generation
   - ranking
   - bounded search
   - plan selection
   - plan revalidation
   - failure handling
   - interrupt evaluation
8. `BlockedIntentMemory` is already an authoritative agent component in `worldwake-core`; it is not runtime-only.
9. `AgentDecisionRuntime` is runtime-only and currently exists, but it does not yet track in-flight/remaining plan progress or observation snapshots for dirty detection.
10. `worldwake-ai/Cargo.toml` already depends on `worldwake-sim`; no dependency fix is needed here.
11. There is no existing `crates/worldwake-ai/src/agent_tick.rs` stub at `HEAD`.
12. `SystemExecutionContext` does not expose scheduler/input-queue mutation, action handlers, recipe registry, or retained replan signals, so the current system-dispatch seam cannot host the full AI loop as written below.
13. `World::create_agent()` does not automatically attach `UtilityProfile` or `BlockedIntentMemory`; integration must handle absent components explicitly or add principled initialization as part of the implementation.

## Architecture Check

1. Decision loop is event-driven: only replan when dirty flag is set.
2. Dirty flag triggers: plan missing, plan finished, plan invalidated, `ReplanNeeded` received, place changed, inventory changed, wounds changed, threshold band changed, blocked intent cleared/expired.
3. AI reads through `&dyn BeliefView` only — constructs `OmniscientBeliefView` at tick start.
4. AI outputs are `InputKind::RequestAction` — same pipeline as human.
5. Dead agents are skipped entirely.
6. No action running + no valid plan = idle (explicit, lawful).
7. Integration belongs at the tick/control boundary, not inside `worldwake-systems`:
   - `worldwake-ai` already depends on `worldwake-sim`
   - `worldwake-ai` test code also depends on `worldwake-systems`
   - registering AI from `worldwake-systems` would invert ownership and create a bad architectural seam
8. The clean seam is a generic tick input producer hook in `worldwake-sim`, with the concrete AI driver implemented in `worldwake-ai`.
9. `ReplanNeeded` must be retained durably between ticks so the AI driver can stay event-driven without reading transient action internals.
10. The cleanest delivery shape is to drain retained replans into the producer context as read-only decision input, rather than requiring producers to mutate scheduler internals directly.

## What to Change

### 1. Add a generic tick-input producer seam in `worldwake-sim`

`step_tick()` currently supports only pre-enqueued external inputs. Add a simulation-owned hook that can inspect tick state and enqueue lawful `InputKind` values before current-tick inputs are drained.

This seam must provide the data the E13 loop actually needs:
- world access for `OmniscientBeliefView`
- event log access for authoritative memory writes when needed
- scheduler/input-queue access
- active action access
- action defs and handlers
- recipe registry
- current tick
- retained `ReplanNeeded` signals for the current decision pass

This should be generic, not AI-specific, so the tick orchestrator stays decoupled from any particular controller implementation.

### 2. Retain and deliver `ReplanNeeded` signals in `worldwake-sim`

Abort/interrupt/failed-commit paths already produce `ReplanNeeded`, but the scheduler/tick step currently throws them away. Persist them in scheduler runtime until the next input-production pass drains them.

### 3. Implement the per-agent decision driver in `worldwake-ai`

Add the missing agent-tick integration layer in `worldwake-ai` on top of the already-implemented planner pieces. A new module/file is acceptable; do not assume a pre-existing `agent_tick.rs` stub.

The driver must own runtime-only state:
- `BTreeMap<EntityId, AgentDecisionRuntime>`
- planning budget
- semantics table cache

`AgentDecisionRuntime` must be extended as needed to support real execution flow, including:
- current plan tracking
- in-flight step tracking or equivalent remaining-step progress
- enough last-observed snapshot data to compute dirty status deterministically

The per-agent flow is:

Per-agent tick flow:
1. Skip dead agents (return `None`)
2. Resolve authoritative `BlockedIntentMemory` and `UtilityProfile`
   - use stored component values when present
   - if missing, apply the implementation choice documented in code/tests for this ticket
3. Reconcile in-flight step state against active actions and retained `ReplanNeeded`
4. Derive current pressures (pain, danger)
5. Update dirty flag based on state changes
6. Clear expired / resolved blocked intents
7. Evaluate interrupts against the currently active action, if any
8. If replanning required:
   a. Generate grounded candidates
   b. Suppress blocked candidates
   c. Rank candidates (priority class + motive score)
   d. Plan only top `max_candidates_to_plan`
   e. Select best valid plan
9. If current valid plan exists and no action is already active for the agent:
   a. Revalidate the next step by affordance identity
   b. If invalid: trigger failure handling, return idle
   c. If valid: enqueue/emit `InputKind::RequestAction` for that exact step and mark it in flight
10. If no valid plan: remain idle (explicit, lawful)

### 4. Implement dirty flag logic against the real runtime shape

The helper signature may differ from the original ticket if the real runtime shape requires more state.

Dirty when:
- Plan missing or finished
- Plan invalidated (next step would fail revalidation)
- `ReplanNeeded` received for this agent
- Place changed since last decision
- Inventory/possessions changed
- Wounds changed
- Relevant threshold band crossed
- Blocked intent cleared or expired

### 5. Integrate the AI driver into tick orchestration

Wire the new producer hook into the main simulation tick path so AI-controlled agents can enqueue lawful `RequestAction` inputs through the same input queue/action validation pipeline as humans.

### 6. Manage `AgentDecisionRuntime` storage

`AgentDecisionRuntime` instances remain runtime-only and are stored in a `BTreeMap<EntityId, AgentDecisionRuntime>` owned by the AI driver, not in component tables. This map is initialized lazily.

## Files to Touch

- `crates/worldwake-sim/src/tick_step.rs` (modify — invoke tick-input producer before draining current-tick inputs)
- `crates/worldwake-sim/src/scheduler.rs` (modify — retain/drain `ReplanNeeded` signals)
- `crates/worldwake-sim/src/system_dispatch.rs` or nearby sim-owned orchestration module (modify/add — generic producer seam types)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — real execution/runtime tracking fields)
- `crates/worldwake-ai/src/lib.rs` (modify — export AI driver entry points)
- `crates/worldwake-ai/src/...` (add/modify — concrete per-agent tick integration module)
- `crates/worldwake-systems/src/lib.rs` (do not wire AI here; out of scope for the corrected architecture)

## Out of Scope

- Per-agent belief stores (E14) — uses `OmniscientBeliefView` for now
- CLI / human control interface — E21
- Multi-agent coordination / group planning — Phase 3+
- Exploration motivation from ignorance — Phase 3+

## Acceptance Criteria

### Tests That Must Pass

1. Dead agent produces no `InputEvent`
2. Human-controlled agent (`ControlSource::Human`) is skipped
3. Agent with hunger at critical + owned food -> emits RequestAction for consume
4. Agent with no food + seller nearby -> emits RequestAction for travel or trade
5. Agent with a valid remaining plan and no dirty trigger advances through multi-step execution lawfully
6. Agent receiving retained `ReplanNeeded` on the next decision pass -> sets dirty, replans
7. Agent with invalid next step -> triggers failure handling, records/persists `BlockedIntent`
8. Agent with all candidates blocked -> idles
9. Agent does not thrash between equal plans (switch margin enforced)
10. Agent does not retry blocked target (BlockedIntentMemory suppresses)
11. AI-produced `InputEvent`s use the same input queue and receive correct `scheduled_tick`/monotonic `sequence_no`
12. Interruptible active action can be interrupted for a lawful higher-priority replan trigger
13. `ReplanNeeded` retention/drain is deterministic
14. All AI reads go through `&dyn BeliefView` (no new `&World` planning reads in `worldwake-ai`)
15. Existing suite: `cargo test --workspace`

### Invariants

1. All AI reads through `&dyn BeliefView` — no `&World` access in `worldwake-ai`
2. AI outputs are `InputKind::RequestAction` — same pipeline as human
3. Dead agents produce nothing
4. Idle is explicit behavior, not an error
5. `AgentDecisionRuntime` is NOT in component tables
6. Decision loop is event-driven (dirty flag gated)
7. AI integration does not invert crate ownership by registering `worldwake-ai` through `worldwake-systems`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/...` — agent driver tests covering:
   - dead/human skip
   - candidate -> plan -> queued request
   - multi-step plan progress
   - retained replan handling
   - blocked-memory persistence
   - switch-margin stability
2. `crates/worldwake-sim/src/tick_step.rs` or sim integration tests — producer hook runs before input drain and shares the normal queue pipeline
3. End-to-end integration test: create world with AI agent(s), run several ticks, verify correct `RequestAction` flow and lawful progression

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-sim`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

Implemented the sim-owned tick-input producer seam in `worldwake-sim` and the concrete AI decision driver in `worldwake-ai`, rather than trying to force AI orchestration through `worldwake-systems`.

What changed versus the original draft:
- `ReplanNeeded` retention is drained by `step_tick()` and delivered to producers as read-only `TickInputContext` input, which is cleaner than having producers reach into scheduler state to consume signals.
- `AgentDecisionRuntime` was extended with in-flight plan progress and observation snapshot fields so dirty detection stays runtime-local and deterministic.
- The AI driver emits lawful `RequestAction` inputs through the normal queue, reconciles retained replans and active actions, persists `BlockedIntentMemory`, and defaults missing `UtilityProfile` / `BlockedIntentMemory` components rather than requiring new bootstrap wiring.
- The integration tests were corrected to reflect authoritative action timing: the decision tick must enqueue/start the consume action immediately, but item consumption completes according to the action's resolved duration rather than by assumption in the same tick.
