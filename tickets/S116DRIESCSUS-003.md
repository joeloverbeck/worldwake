# S116DRIESCSUS-003: Extend needs_system for dirtiness counter and escalation event emission

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `needs_system` (authoritative maintenance of `DeprivationExposure` + emission of `EventTag::Escalation`)
**Deps**: S116DRIESCSUS-001, S116DRIESCSUS-002

## Problem

Spec S116 requires `needs_system` to maintain `DeprivationExposure.dirtiness_critical_ticks` with the same increment-on-above / reset-on-below semantics as the existing 4 counters, and to emit `EventTag::Escalation` begin/end transitions per need when `DeprivationExposure::ticks_at_critical(need)` crosses the agent's `DriveEscalationProfile.params_for(need).start_after_ticks`. No new SystemFn is added; the existing system's counter loop simply extends to 5 needs and gains a transition-emission pass.

## Assumption Reassessment (2026-04-17)

1. `needs_system` signature: `pub fn needs_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError>` at `crates/worldwake-systems/src/needs.rs:12`. `SystemId::Needs` is first in `SystemManifest::canonical()` at `crates/worldwake-sim/src/system_manifest.rs:111-128` — the counter already reflects this tick's need values before any ranking pass. No manifest change required.
2. Existing tests exercising the counter path:
   - `needs_system_increments_deprivation_exposure_at_critical_thresholds` at needs.rs:768 — asserts 4-field counter increments when pressure ≥ critical.
   - `needs_system_resets_deprivation_exposure_when_pressure_drops_below_critical` at needs.rs:849 — asserts reset on sub-critical.
   Both need one dirtiness case added to reach the 5-field invariant.
3. Event emission pattern: `needs_system` already uses two-phase `WorldTxn` emission — see the death event path at needs.rs:53-69 (per-death `WorldTxn::new(...)` with `VisibilitySpec::SamePlace`, `CauseRef::SystemTick(tick)`, tags `{ EventTag::Death, EventTag::System, EventTag::WorldMutation }`, commit per entity). Reuse the same shape for escalation transitions with tags `{ EventTag::System, EventTag::Escalation }` and `VisibilitySpec::Hidden` (escalation is agent-internal, not scene-observable).
4. `DriveEscalationProfile` read via `world.get_component_drive_escalation_profile(agent)` — the bootstrap in ticket 002 guarantees `Some(_)` for every agent. Runtime access via `.expect("agent must have DriveEscalationProfile")` per CLAUDE.md §5 universal-profile contract.
5. Intended layer and harness: authoritative-system layer (`needs_system` owns `DeprivationExposure`); full action registries not required for unit coverage because this path does not call action handlers — only the emission path is exercised. Local needs-only harness is sufficient for new focused tests.
6. `action_name` encoding for transition events: canonical strings `"escalation_begin:{need}:{multiplier_permille}"` and `"escalation_end:{need}:{duration_ticks}"` where `{need}` is `HomeostaticNeedId::as_str()`-style (or `Debug`-derived if no display helper exists). Decision: encode via `action_name` rather than introducing a new `StateDelta` variant — the latter is a larger cross-crate surface change outside this ticket's scope. Document the encoding in a doc-comment above the emission site so ticket 006's goldens can parse it.
7. Shared abstraction boundary under audit: `EventPayload.action_name: Option<String>` + `tags: BTreeSet<EventTag>` as the transport for escalation transition semantics. This ticket makes the encoding canonical; later observer/golden code will decode it.

## Architecture Check

1. Extension stays within one existing system. No new `SystemFn`, no new `SystemId`, no `SystemManifest::canonical()` change — the load-bearing causal ordering commentary in `system_manifest.rs` is untouched.
2. FND-26 preserved: `needs_system` is the single writer of `DeprivationExposure`; `ranking.rs` (ticket 004) is a reader. No cross-system direct call.
3. FND-29A preserved: escalation transitions become authoritative append-only history, queryable via the event log.
4. FND-14 preserved: escalation input is the agent's own components only — no world-state or cross-agent read.

## Verification Layers

1. Dirtiness counter increment → focused test extension of `needs_system_increments_deprivation_exposure_at_critical_thresholds` covering dirtiness.
2. Dirtiness counter reset → focused test extension of `needs_system_resets_deprivation_exposure_when_pressure_drops_below_critical` covering dirtiness.
3. Escalation begin emission → new focused test: drive dirtiness counter past `start_after_ticks` in one tick and assert `EventTag::Escalation` with `action_name == "escalation_begin:Dirtiness:1010"` present at that tick.
4. Escalation end emission → new focused test: after begin, drop dirtiness below critical; assert `EventTag::Escalation` with `action_name == "escalation_end:Dirtiness:N"` at the reset tick.
5. No spurious emission → negative-path test: counter stays at 1..=start_after_ticks over a multi-tick run; assert zero `EventTag::Escalation` events emitted.
6. Multi-need transitions in one tick → focused test: hunger and dirtiness both cross `start_after_ticks` on the same tick; assert two distinct `EventTag::Escalation` begin events with distinct `action_name` need identifiers.

## What to Change

### 1. Dirtiness counter maintenance

In `needs_system`'s existing counter-update loop (around needs.rs:246-300), add dirtiness handling mirroring hunger/thirst/fatigue/bladder: read `thresholds.dirtiness.critical()` via the `DriveThresholds::critical(HomeostaticNeedId::Dirtiness)` accessor from ticket 001 (or direct field access), compare against `needs.dirtiness`, increment `exposure.dirtiness_critical_ticks` on above / reset to 0 otherwise.

Prefer a single keyed loop using `HomeostaticNeedId::ALL` + `ticks_at_critical` + `needs.value` + `thresholds.critical` over five explicit field blocks, to reduce drift risk if a sixth need is ever added.

### 2. Escalation transition computation

After the counter pass, for each `HomeostaticNeedId::ALL`:

```rust
let params = profile.params_for(need);
let was_escalating = prev_exposure.ticks_at_critical(need) > params.start_after_ticks;
let is_escalating = next_exposure.ticks_at_critical(need) > params.start_after_ticks;
let multiplier = escalation_multiplier(next_exposure.ticks_at_critical(need), params);
if is_escalating && !was_escalating {
    emit_escalation_begin(world, event_log, agent, need, multiplier, tick);
} else if !is_escalating && was_escalating {
    let duration = /* prev_exposure.ticks_at_critical(need), i.e. the final counter value before reset */;
    emit_escalation_end(world, event_log, agent, need, duration, tick);
}
```

`prev_exposure` is the `DeprivationExposure` value before the counter update this tick; `next_exposure` is the value written by step 1.

### 3. Event emission helpers

Two new local helpers inside `needs_system.rs`:

```rust
fn emit_escalation_begin(
    world: &mut World,
    event_log: &mut EventLog,
    agent: EntityId,
    need: HomeostaticNeedId,
    multiplier: Permille,
    tick: Tick,
) { /* WorldTxn with tags { System, Escalation }, action_name formatted string, target agent */ }

fn emit_escalation_end(
    world: &mut World,
    event_log: &mut EventLog,
    agent: EntityId,
    need: HomeostaticNeedId,
    duration_ticks: u32,
    tick: Tick,
) { /* analogous */ }
```

Follow the death-event emission style (per-entity `WorldTxn`, `VisibilitySpec::Hidden`, `CauseRef::SystemTick(tick)`). Document the `action_name` encoding format in a doc-comment on each helper.

### 4. Update existing tests

Add one dirtiness case to `needs_system_increments_deprivation_exposure_at_critical_thresholds` (assert `dirtiness_critical_ticks` increments when `dirtiness >= dirtiness.critical()`) and to `needs_system_resets_deprivation_exposure_when_pressure_drops_below_critical` (assert reset on sub-critical dirtiness).

### 5. New focused tests

Add to the same `#[cfg(test)] mod tests` block in `needs.rs`:

- `needs_system_emits_escalation_begin_when_dirtiness_counter_crosses_start_after`
- `needs_system_emits_escalation_end_when_dirtiness_counter_resets`
- `needs_system_does_not_emit_escalation_when_counter_below_start_after`
- `needs_system_emits_distinct_escalation_events_for_multi_need_transitions_same_tick`

Use the existing local harness helpers in `needs.rs` (see `setup_world_with_needs_and_wounds` style if present, or model on the existing counter tests).

## Files to Touch

- `crates/worldwake-systems/src/needs.rs` (modify — counter extension, transition computation, emission helpers, test additions)

## Out of Scope

- `ranking.rs` consumption of the counter or profile — ticket 004.
- Scenario RON integration for profile overrides — ticket 005.
- Golden coverage for end-to-end escalation breaking wash-cycle starvation — ticket 006.
- Introducing a typed `StateDelta::EscalationTransition` variant (larger cross-crate change; left for a future ticket if structured decoding becomes necessary).

## Acceptance Criteria

### Tests That Must Pass

1. Extended `needs_system_increments_deprivation_exposure_at_critical_thresholds` passes with dirtiness case.
2. Extended `needs_system_resets_deprivation_exposure_when_pressure_drops_below_critical` passes with dirtiness case.
3. All 4 new focused emission tests pass.
4. Existing `needs_system_*` suite at `crates/worldwake-systems/src/needs.rs` tests block all pass unchanged otherwise.
5. Existing suite: `cargo test -p worldwake-systems needs`, `cargo test -p worldwake-systems --test e09_needs_integration`.

### Invariants

1. Begin event is emitted exactly on the tick the counter exceeds `start_after_ticks`; no duplicate in subsequent ticks while counter continues to grow.
2. End event is emitted exactly on the tick the counter resets to 0 from a previously-escalating state; no emission when resetting from a non-escalating state.
3. Counter maintenance is symmetric across all 5 needs — no need-kind-specific branching beyond the keyed lookup.
4. `DeprivationExposure` is the single authoritative source of "ticks at critical per need" (FND-28).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs.rs` — extend 2 existing counter tests; add 4 new emission tests listed above.

### Commands

1. `cargo test -p worldwake-systems needs`
2. `cargo test -p worldwake-systems --test e09_needs_integration`
3. `cargo test -p worldwake-core`
4. `cargo clippy --workspace --all-targets -- -D warnings`
