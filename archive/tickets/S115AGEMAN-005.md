# S115AGEMAN-005: agenda_tick_system SystemFn + S74 margin integration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — wires `agenda_tick_system` into the agent-tick phase between perception and ranking; redirects S74 switch-margin read from rank-derived current to `AgendaState.committed.motive_score`; emits S110 `GoalCommitted`/`GoalSuspended`/`GoalAbandoned` events from `AgendaTransitions`.
**Deps**: [archive/tickets/S115AGEMAN-003](../archive/tickets/S115AGEMAN-003.md), [archive/tickets/S115AGEMAN-004](../archive/tickets/S115AGEMAN-004.md)

## Problem

With `tick_agenda` implemented (ticket 003) and `classify_rejection` landed (ticket 004), the agenda manager still sits un-invoked in the agent-tick flow. Ticket 004 truthfully routed committed-feasibility rejections through the current planner seam in `agent_tick/planning.rs`; this ticket owns migrating the remaining dormant agenda lifecycle into the intended `agenda_tick_system` path. S74's switch-margin logic at `active_action.rs:180-199` also still reads the rank-derived "current" goal rather than `AgendaState.committed`. This ticket ties both surfaces together: inserts a new `agenda_tick_system` SystemFn at the correct phase boundary (after belief-update, before candidate generation), routes the resulting `AgendaTransitions` into the event log (S110 event tags), and updates the switch-margin check so commitment decisions read the authoritative agenda state. Without this ticket, the agenda manager is dormant infrastructure.

## Assumption Reassessment (2026-04-22)

1. Current agent-tick flow in `crates/worldwake-ai/src/agent_tick/mod.rs` performs perception/belief-update in `refresh_runtime_for_read_phase_with_memories` (~line 930), candidate generation plus ranking into `read_result.ranked` (line 1034), then downstream planning/active-action consumers. Ticket 003 landed `tick_agenda(actor, ..., fresh_candidates: Vec<AgendaEntry>, ...)`, so the truthful insertion point is AFTER fresh candidate ranking exists and BEFORE downstream commitment consumers (`build_candidate_plans`, active-action interruption checks, and event emission).
2. S74 switch-margin read site: `crates/worldwake-ai/src/agent_tick/active_action.rs:180-199`. Currently reads `cognitive.switch_margin` (from `CognitiveProfile` at `crates/worldwake-core/src/cognitive_profile.rs:39`) alongside a rank-derived current candidate. Post-ticket, the switch-margin comparison reads `runtime.agenda_state.committed.as_ref().map(|e| e.motive_score).unwrap_or(0)` as the baseline.
3. S110 event emission path: `EventTag::GoalCommitted` / `GoalSuspended` / `GoalAbandoned` at `crates/worldwake-core/src/event_tag.rs:37-39` with payload structs `GoalCommittedPayload`, `GoalSuspendedPayload`, `GoalAbandonedPayload` at `crates/worldwake-core/src/decision_event_payload.rs:80,107,114`. These are already the target payload shapes for agenda transitions.
4. The shared boundary under audit is the caller's `AgendaTransitions` → event-log write loop. `tick_agenda` returns transitions; the caller walks them and writes one event per transition. Keeping emission caller-side (not inside `tick_agenda`) preserves the manager's I/O purity from ticket 003.
5. Existing integration harness for agent-tick: `crates/worldwake-ai/src/agent_tick/tests.rs` has extensive coverage. After this ticket, harness tests that step `agent_tick` must observe agenda state updates. Specifically `cargo_satisfaction_at_destination_while_carrying` (line 4710) now transitions the committed goal into `suspended` via the classifier — it should still observe the committed key via `runtime.agenda_state.committed`, with `phase: Suspended`.
6. Intended invariant under audit: a committed goal that remains viable (no margin-exceeding challenger, no kill-condition met, no Satisfied/Dead classification) persists across ticks in `AgendaState.committed` with the same `key`. The two-tick integration test (ticket 006) validates this directly.
7. Ordering contract: the system function `agenda_tick_system` runs STRICTLY BEFORE the existing candidate-generation + ranking call. The agent-tick scheduler currently runs systems in registration order; this ticket registers `agenda_tick_system` before the existing candidate-generation system hook. Per precision-rules.md §4, the ordering driver is explicit system-registration order, not ranking or suppression.
8. `AgendaProfile` access: `agenda_tick_system` reads `profile = world.get_component_agenda_profile(agent).expect("universal component")`. Universal-profile `expect()` is the correct access pattern (ticket 001 seeds the default in `create_agent`).

## Architecture Check

1. Placing agenda mutation in a dedicated SystemFn rather than inlining it in the existing candidate-generation path keeps the ai tick scheduler traceable (each system is independently orderable and debuggable) and matches the existing SystemFn pattern (`perception_system`, `needs_system`, etc.). No cross-system direct calls (FND-26).
2. Redirecting the switch-margin baseline to `AgendaState.committed.motive_score` removes the redundant "rank-derived current" concept — post-ticket, commitment is stored, not re-derived. This fulfills the spec D7 cleanup.
3. Caller-side event emission keeps `tick_agenda` pure; writing to the event log from inside the manager would couple the manager to event-log internals and prevent unit-testability without an event-log fixture (which is expensive).

## Verification Layers

1. System ordering — runtime test asserting `agenda_tick_system` runs before `candidate_generation_system` (or its equivalent) by inspecting system-registration order or by observing state mutations in a test harness that logs per-system effects.
2. Switch-margin redirect — unit test in `active_action.rs`: set `AgendaState.committed.motive_score = 500`, set a fresh candidate with `motive_score = 499`, set `switch_margin = 10` → current stays committed. Increment fresh candidate to `motive_score = 520` → switch fires.
3. Event emission — integration test: trigger a commit transition in a harness; assert one `EventTag::GoalCommitted` event in the log with matching `goal_key` and `motive_score` payload fields.
4. Cross-layer invariant — after a full tick: (a) decision-trace sees `agenda_tick_system` ran, (b) event log contains transition events, (c) `AgendaState.committed` matches what the event payload records. Covered by ticket 006's integration tests; this ticket ensures the wiring is correct.
5. Authoritative-mutation ordering — event-log delta: `GoalCommitted` precedes any subsequent `PlanAdopted` in the same tick (agenda commit happens before plan selection consumes the committed slot).

## What to Change

### 1. Create `agenda_tick_system`

Add `pub fn agenda_tick_system(world: &mut World, tick: Tick, event_log: &mut EventLog)` (exact signature per existing SystemFn conventions) in `crates/worldwake-ai/src/agent_tick/mod.rs` or a new file `crates/worldwake-ai/src/agent_tick/agenda_step.rs` (choose based on module layout; new file is cleaner for a new phase).

Per-agent loop:
1. Read `profile = world.get_component_agenda_profile(agent).expect(...)`.
2. Read `discrepancy_memory = world.get_component_discrepancy_memory(agent).cloned().unwrap_or_default()`.
3. Build `fresh_candidates: Vec<AgendaEntry>` from the current tick's ranked candidate output (`read_result.ranked`), preserving the actor-scoped agenda-entry shape ticket 003 actually landed.
4. Call `transitions = tick_agenda(agent, &mut runtime.agenda_state, fresh_candidates, &beliefs, &discrepancy_memory, &profile, tick)`.
5. Emit events from `transitions`:
   - For each `CommitTransition::Committed { new_key, previous_key }` → write `GoalCommitted` with payload (+ `GoalSuspended` for the previous commitment if non-None and viable).
   - For each `demoted_to_suspended` entry → write `GoalSuspended`.
   - For each `killed` entry → write `GoalAbandoned`.
6. For `Dead`-classified rejections, also call `discrepancy_memory.record(entry)` with the synthesized `BlockerKey` (the caller holds the write handle, completing the classifier-as-pure-function contract from ticket 004).

### 2. Register `agenda_tick_system` in agent-tick scheduler

Find the scheduler registration (typically `crates/worldwake-ai/src/agent_tick/mod.rs` or a scheduler module) and insert `agenda_tick_system` between perception/belief-update and candidate generation. Preserve existing system order otherwise.

### 3. Redirect S74 switch-margin read

At `crates/worldwake-ai/src/agent_tick/active_action.rs:180-199`, change:

```rust
let current_motive = /* previously: some rank-derived baseline */;
let switch_margin = cognitive.switch_margin;
if top_candidate.motive_score > current_motive + switch_margin.value() {
    // switch
}
```

to read from `AgendaState.committed`:

```rust
let current_motive = runtime.agenda_state.committed
    .as_ref()
    .map(|entry| entry.motive_score)
    .unwrap_or(0);
let switch_margin = cognitive.switch_margin;
if top_candidate.motive_score > current_motive + switch_margin.value() {
    // switch
}
```

(Exact surrounding code depends on current call site; preserve signed/unsigned arithmetic semantics.)

### 4. Update ticket 003's placeholder `commit_or_keep`

Replace the placeholder in `agenda_manager::commit_or_keep` with the margin-aware logic: compare top-ranked candidate's `motive_score` against `state.committed.as_ref().map(|e| e.motive_score).unwrap_or(0) + profile-margin-hint` (or thread the `switch_margin` through the `tick_agenda` signature from the caller). Simpler: thread `switch_margin: Permille` as an additional parameter to `tick_agenda` (caller passes `cognitive.switch_margin`), and `commit_or_keep` uses it directly.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/agenda_step.rs` (new — `agenda_tick_system` function + event-emission helper)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — register new system, import)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify — switch-margin redirect at lines 180-199)
- `crates/worldwake-ai/src/agenda_manager.rs` (modify — extend `tick_agenda` signature with `switch_margin`, replace `commit_or_keep` placeholder)

## Out of Scope

- Unit tests for `agenda_tick_system` wiring (ticket 006 covers two-tick commit persistence and event-log assertions)
- Golden scenario (ticket 007)
- Changes to candidate-generation algorithms — this ticket consumes the existing ranked candidate output as-is
- Changes to event-log infrastructure — payload types already exist (S110)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai -- active_action` — switch-margin unit tests pass with the new `AgendaState.committed.motive_score` baseline.
2. `cargo test -p worldwake-ai -- agent_tick` — broader agent-tick unit tests pass with the new system in the pipeline.
3. `crates/worldwake-ai/src/agent_tick/tests.rs::cargo_satisfaction_at_destination_while_carrying` passes — the test observes the committed goal in `AgendaState.committed` and correctly transitions through the classifier → suspended flow.
4. `crates/worldwake-ai/tests/golden_portfolio_planning.rs::portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` passes.
5. Existing suite: `cargo test --workspace` passes.

### Invariants

1. System ordering: `agenda_tick_system` runs BEFORE candidate-generation + ranking in every agent's tick. Violation would mean agenda state is consumed before it's updated — regression.
2. Switch-margin comparison reads ONLY `AgendaState.committed.motive_score` — no rank-derived "current" fallback.
3. Every `CommitTransition::Committed` produces exactly one `EventTag::GoalCommitted` with matching `goal_key`. Every `demoted_to_suspended` entry produces one `EventTag::GoalSuspended`. Every `killed` entry produces one `EventTag::GoalAbandoned`.
4. `DiscrepancyMemory::record` write for the `Dead` branch happens via the caller (this ticket's system), not via the classifier.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify inline `#[cfg(test)]`) — rewrite or extend switch-margin tests to use `AgendaState.committed.motive_score` baseline; assert switch fires at margin boundary.
2. `crates/worldwake-ai/src/agent_tick/agenda_step.rs` (new inline `#[cfg(test)]`) — small wiring test: call `agenda_tick_system` on a harness, observe that `AgendaState` was mutated and that emitted events match.
3. No golden changes in this ticket — existing goldens must pass unchanged.

### Commands

1. `cargo test -p worldwake-ai -- active_action agenda_step`
2. `cargo test -p worldwake-ai -- agent_tick`
3. `cargo test -p worldwake-ai -- cargo_satisfaction golden_portfolio_planning`
4. `cargo test --workspace`
5. `./scripts/verify.sh`
