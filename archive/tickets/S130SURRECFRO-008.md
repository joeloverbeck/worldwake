# S130SURRECFRO-008: SurveyMemory decay in evidence_decay_system

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `evidence_decay_system` extended with an agent-iteration pass for `SurveyMemory::enforce_limits`
**Deps**: `archive/tickets/S130SURRECFRO-001.md`, `archive/tickets/S130SURRECFRO-002.md`, `archive/tickets/S130SURRECFRO-004.md`, spec `archive/specs/S130-survey-records-frontier-disconfirmation.md` SystemFn Integration

## Problem

`SurveyMemory` entries must decay through an explicit world process (FND-22A — concrete state with explicit decay) rather than persisting indefinitely. The spec mandates per-tick `SurveyMemory::enforce_limits(current_tick, &cognitive_profile)` calls. The natural host is `evidence_decay_system` (already a per-tick decay-pass SystemFn) — extended here with an agent-iteration pass that runs alongside the existing place-iteration pass for `SceneEvidence`.

## Assumption Reassessment (2026-05-02)

1. `evidence_decay_system` lives at `crates/worldwake-systems/src/evidence_decay.rs:7` and currently iterates places via `world.query_scene_evidence()` (line 36), pruning stale `EvidenceEntry` items. The existing function is the per-tick decay pass — no new SystemFn is introduced.
2. `RouteExperience::enforce_limits` and `SourceReliability::enforce_limits` (the spec's prior reference points) live at `crates/worldwake-core/src/experience.rs:22, 91` and are called from action-context paths (`travel_actions.rs:145`, `experience_recording.rs:27`, `agent_tick/mod.rs:2121`) — not from a unified per-tick maintenance system. Survey decay is the *first* per-tick agent-iteration decay path; consolidating the others is explicitly out of scope per spec.
3. `SurveyMemory::enforce_limits(&mut self, current_tick: Tick, profile: &CognitiveProfile)` (defined in `archive/tickets/S130SURRECFRO-002.md`) reads `profile.survey_memory_retention_ticks` (added in ticket 001).
4. Macro-generated `entities_with_survey_memory` helper (from ticket 004's `with_component_schema_entries!` registration) exposes the agent-iteration surface.
5. Existing `evidence_decay.rs` tests at lines 198-310 exercise the place-iteration path: `evidence_decay_system_keeps_unexpired_entries:198`, `evidence_decay_system_removes_only_expired_entries:232`, `evidence_decay_system_clears_component_when_last_entry_expires:269`. They construct test fixtures with `SceneEvidence` only — adding the agent-iteration pass does not alter their fixtures or assertions (no agents have `SurveyMemory` entries in those fixtures, so the new pass is a no-op for them). This ticket adds two new tests covering the agent-iteration path.
6. No goal-kind, candidate-emission, validation, or affordance-generation surface is touched — this is a maintenance-pass extension only.

## Architecture Check

1. Hosting `SurveyMemory::enforce_limits` in `evidence_decay_system` rather than spawning a new `belief_maintenance_system` SystemFn keeps the per-tick decay-pass invariant (one SystemFn for periodic per-agent / per-place state decay) rather than fragmenting it across multiple systems. The existing place-iteration pass is preserved unchanged.
2. The agent-iteration pass reads `cognitive_profile` per-agent (each agent may have customized retention) and calls `enforce_limits` against that profile — preserves agent-level diversity (FND-22).
3. No backward-compat shim — net-new code path within an existing SystemFn body.
4. Decay is a pure local mutation of an agent's own component — no cross-agent reads, no global state queries (FND-7).

## Verification Layers

1. Agents with stale `SurveyMemory` entries lose them after the system runs once → focused unit/runtime test (set up agent with entries at `tick - retention - 1`; advance to current tick; run system; assert empty).
2. Agents with fresh `SurveyMemory` entries keep them → focused unit test.
3. Existing place-iteration behavior is preserved → existing tests `evidence_decay_system_keeps_unexpired_entries:198`, `evidence_decay_system_removes_only_expired_entries:232`, `evidence_decay_system_clears_component_when_last_entry_expires:269` continue to pass.
4. Single-system ticket — one SystemFn, two iteration passes (places, then agents). No decision-trace, action-trace, or goal-system surface to map.

## What to Change

### 1. Agent-iteration pass

In `crates/worldwake-systems/src/evidence_decay.rs`, after the existing place-iteration block (after line 23, before `Ok(())`), add an agent-iteration block:

```rust
let agents_to_update: Vec<EntityId> = world.entities_with_survey_memory().collect();
for agent in agents_to_update {
    let Some(profile) = world.get_component_cognitive_profile(agent).cloned() else {
        continue;
    };
    let Some(mut memory) = world.get_component_survey_memory(agent).cloned() else {
        continue;
    };
    let before = memory.entries.len();
    memory.enforce_limits(tick, &profile);
    if memory.entries.len() == before {
        continue;
    }
    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        Some(agent),
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.set_component_survey_memory(agent, memory)
        .map_err(|error| SystemError::new(error.to_string()))?;
    let _ = txn.commit(event_log);
}
```

(Helper names — `entities_with_survey_memory` and `get_component_survey_memory` — bind to the macro-generated accessors from ticket 004's registration. Variable names mirror the existing `apply_update` helper pattern at `evidence_decay.rs:59-89`. Confirm signatures during reassessment.)

### 2. Tests

Add to `crates/worldwake-systems/src/evidence_decay.rs` `#[cfg(test)]` block (alongside existing tests at lines 198-310):

- `evidence_decay_system_prunes_stale_survey_records` — agent with entry at `tick - retention - 1`; system runs at current tick; entry is gone.
- `evidence_decay_system_keeps_fresh_survey_records` — agent with entry within retention; system runs; entry preserved.

## Files to Touch

- `crates/worldwake-systems/src/evidence_decay.rs` (modify — agent-iteration pass + 2 new tests)

## Out of Scope

- Consolidating `RouteExperience::enforce_limits` and `SourceReliability::enforce_limits` into the same pass — explicitly out of scope per spec; their action-context placement is intentional
- Renaming `evidence_decay_system` to reflect its broadened scope — out of scope (the function name is documentary; the SystemFn registration in `lib.rs` is unchanged)
- Per-agent decay tuning beyond `survey_memory_retention_ticks` — out of scope
- Golden coverage of the decay behavior end-to-end (ticket 009 — sub-test 2 covers damping fade-out which exercises decay through the timing window)

## Acceptance Criteria

### Tests That Must Pass

1. New: `evidence_decay_system_prunes_stale_survey_records`.
2. New: `evidence_decay_system_keeps_fresh_survey_records`.
3. Existing: `evidence_decay_system_keeps_unexpired_entries` — passes unchanged.
4. Existing: `evidence_decay_system_removes_only_expired_entries` — passes unchanged.
5. Existing: `evidence_decay_system_clears_component_when_last_entry_expires` — passes unchanged.
6. Existing suite: `cargo test -p worldwake-systems evidence_decay`.
7. Existing suite: `cargo test --workspace`.

### Invariants

1. The place-iteration pass for `SceneEvidence` runs first; the agent-iteration pass for `SurveyMemory` runs second. Order is fixed within the SystemFn body but does not matter for correctness (the two passes operate on disjoint component types).
2. Agents without a `SurveyMemory` component or without a `CognitiveProfile` are silently skipped by the agent-iteration pass — defensive against state inconsistencies, consistent with `evidence_decay_system`'s existing skip-if-already-empty convention.
3. The system is idempotent within a single tick — running it twice in a row produces the same world state as running it once.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/evidence_decay.rs` (`#[cfg(test)]` block) — 2 new focused/runtime tests covering the agent-iteration path (per Acceptance Criteria 1-2).

### Commands

1. `cargo test -p worldwake-systems evidence_decay`
2. `cargo test -p worldwake-systems`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-02.

- Extended `evidence_decay_system` with a second per-tick pass that collects agents with `SurveyMemory`, reads each agent's `CognitiveProfile`, applies `SurveyMemory::enforce_limits(tick, profile)`, and writes back only changed memories.
- Added focused runtime coverage for stale survey pruning and fresh survey retention in `crates/worldwake-systems/src/evidence_decay.rs`.
- Preserved the existing `SceneEvidence` place pass and its tests unchanged; survey decay remains in the same SystemFn and no new SystemFn was introduced.

## Deviations

- The landed implementation follows the existing `collect_updates` / `apply_update` shape instead of the inline one-block sketch: survey-memory changes are collected immutably first and then committed through `apply_survey_update`.
- Survey-memory decay commits carry `System` and `WorldMutation` tags and target the updated agent, matching the existing place-evidence mutation event shape.

## Verification Result

- Passed `cargo test -p worldwake-systems --lib evidence_decay::tests::evidence_decay_system_prunes_stale_survey_records -- --exact`.
- Passed `cargo test -p worldwake-systems --lib evidence_decay::tests::evidence_decay_system_keeps_fresh_survey_records -- --exact`.
- Passed `cargo test -p worldwake-systems evidence_decay`.
- Passed `cargo fmt --all`.
- Passed `cargo test -p worldwake-systems`.
- Passed `cargo test --workspace`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
