# S128SLEEPIPLA-004: Sleep action handler refactor and wake-condition synthesis

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — replaces the per-tick re-commit `tick_sleep` handler with a duration-bearing episode, adds wake-condition synthesis, populates `SleepEpisode` at start, emits `SleepEpisodeStarted`/`SleepEpisodeEnded` events. Updates the existing `sleep_reduces_fatigue_without_a_bed` test.
**Deps**: archive/tickets/S128SLEEPIPLA-001.md, S128SLEEPIPLA-002, S128SLEEPIPLA-003

## Problem

The current `tick_sleep` (`crates/worldwake-systems/src/needs_actions.rs:331-349`) reduces fatigue by `MetabolismProfile.rest_efficiency` per tick and returns `ActionProgress::Continue` — the planner re-selects sleep next tick, producing 143–146 separate `sleep` action commits per agent in a 1440-tick run (`reports/proposed-gameplay-mechanic-changes.md:191`). Sleep cannot have intent: it cannot wake on a projected hunger breach, cannot know its place's sleep quality, cannot record a partial-recovery aftermath when interrupted, and produces no causal record beyond the existing `ActionStarted`/`ActionCommitted` per re-commit. This ticket replaces the handler with a duration-bearing sleep episode that holds wake conditions, modulates recovery by place quality, and emits the new lifecycle events.

## Assumption Reassessment (2026-04-27)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Existing focused/unit coverage at `crates/worldwake-systems/src/needs_actions.rs::sleep_reduces_fatigue_without_a_bed` (line 738) asserts that `tick_sleep` reduces `HomeostaticNeeds.fatigue` by `rest_efficiency`. This test must be reframed (or extended) to assert the new episode-based behavior — recovery accumulates across ticks, `SleepEpisode` component is present mid-episode, fatigue reduction equals integrated recovery up to wake. Per `docs/precision-rules.md` Rule 13, do not adapt the test to the new behavior cosmetically — restate the intended invariant first. Existing AI-layer coverage: `crates/worldwake-ai/src/feasibility.rs::test_sleep_always_likely` (line 689) asserts sleep is always feasible — this should remain green; sleep does not gain new preconditions in this ticket.
2. `tick_sleep` handler signature: `fn tick_sleep(_def: &ActionDef, instance: &mut ActionInstance, _context: &worldwake_sim::ActionExecutionContext<'_>, _rng: &mut DeterministicRng, txn: &mut WorldTxn<'_>) -> Result<ActionProgress, ActionError>`. Sleep is registered at `needs_actions.rs:30-35` (handler binding) and `needs_actions.rs:69-75` (`register_def` call) with `DurationExpr::Fixed(NonZeroU32::MIN)` and `Interruptibility::InterruptibleWithPenalty`. Start handler is `start_noop`; commit handler is `commit_noop`. All four handler functions (`start_noop`, `tick_sleep`, `commit_noop`, `abort_noop`) are referenced; `start_noop` and `commit_noop` need bespoke replacements for sleep — `start_sleep_episode`, `commit_sleep_episode`, plus a new `abort_sleep_episode` mirroring commit semantics for unexpected interruptions.
3. Shared boundary under audit: the action lifecycle. The new handler replaces all three of start, tick, and commit (and abort) for the `sleep` action. `Interruptibility` becomes `FreelyInterruptible` (no penalty for wake-condition-driven exit; the wake machinery is internal to the tick handler). `DurationExpr` becomes `DurationExpr::Variable { min: NonZeroU32::new(1).unwrap(), max: NonZeroU32::new(64).unwrap() }` — the per-episode actual bounds come from `SleepEpisode`, but the registration carries placeholder bounds for scheduler bookkeeping.
4. Wake-condition synthesis location (D10 architectural concern): the spec proposes `crates/worldwake-ai/src/agent_tick/sleep_synthesis.rs` (new). However, the synthesis logic reads `FrameAssumption::NeedSafeUntilTick` (in `worldwake-core/src/intention_frame.rs`), agent intention queue (also core), and emits `WakeCondition` (core). The handler at sleep start (in `worldwake-systems`) needs the synthesized vec. Three options: (a) synthesis lives in `worldwake-systems` next to the handler — direct construction at `start_sleep_episode` time; (b) synthesis lives in `worldwake-ai` and writes wake conditions to the agent's `IntentionFrame` (or similar core-layer carrier) before adoption, and the handler reads them off; (c) synthesis lives in `worldwake-ai` and is called as a free function from `worldwake-systems` — illegal under the layer rule. **Recommendation: option (a)** — synthesis at start_sleep_episode time keeps the data flow local and avoids cross-crate coupling. The spec's proposed module path (`worldwake-ai/src/agent_tick/sleep_synthesis.rs`) is corrected to `crates/worldwake-systems/src/sleep_synthesis.rs` (new). Reflect this correction in the implementation; flag if reassessment surfaces a different constraint.
5. Information-path: at start, the handler reads (i) `MetabolismProfile.min_sleep_ticks` and `rest_efficiency` from the agent (authoritative — the agent owns this); (ii) `SleepQualityProfile` from the agent's current place (authoritative — actions execute against world state per FND-14 exception for action handlers; `recovery_modifier` is cached on `SleepEpisode` so the tick loop doesn't re-read); (iii) `FrameAssumption::NeedSafeUntilTick` from the agent's `IntentionFrame` for each non-Fatigue need; (iv) any scheduled commitments in the intention queue for the sleep window. At tick, the handler reads `SleepEpisode.recovery_modifier` (cached), `MetabolismProfile.rest_efficiency`, and current `HomeostaticNeeds.fatigue`. Wake-condition evaluation re-reads `FrameAssumption::NeedSafeUntilTick` each tick (projection may shift). At commit, the handler removes `SleepEpisode` and emits `SleepEpisodeEnded`.
6. Removed path: the per-tick re-commit pattern is removed, not preserved alongside the new path. Per FND-28, no live-authority shim survives. The `Interruptibility::InterruptibleWithPenalty → FreelyInterruptible` change is also a clean transition; no compat layer.
7. Cumulative arithmetic check (Rule 7): per-tick recovery becomes `MetabolismProfile.rest_efficiency × SleepEpisode.recovery_modifier ÷ 1000`. With default values (`rest_efficiency: pm(20)`, `recovery_modifier: pm(1000)`) this equals `pm(20)` per tick, identical to current behavior. With Hillside Shelter's `recovery_modifier: pm(1300)` (per spec), per-tick recovery becomes `pm(26)`. With Fertile Fields' `pm(900)`, recovery becomes `pm(18)`. `accumulated_recovery` is bounded by `Permille::new_unchecked(1000)` (saturates at full recovery). Fatigue cannot drop below `Permille::new_unchecked(0)` per existing saturating subtraction. Survivability envelope: an agent can recover from `fatigue: pm(900)` to `fatigue: pm(0)` in `900/26 ≈ 35` ticks at Hillside Shelter, `900/20 = 45` ticks at default place, `900/18 = 50` ticks at Fertile Fields. With `intended_max_ticks: 64` (placeholder), all three complete fully.
8. Adjacent contradictions check: existing AI behavior assumes sleep is per-tick — the candidate emitter at `crates/worldwake-ai/src/candidate_generation.rs:3228 emit_sleep_goal` emits one untargeted Sleep candidate per tick. After this ticket, the sleep tick handler holds the action open, and the planner sees the action as in-flight — the candidate emitter still emits, but the action lifecycle prevents a parallel sleep. This is consistent with how other duration-bearing actions work. Per-place candidate emission (S128SLEEPIPLA-005) is the orthogonal change; this ticket leaves emission untargeted.

## Architecture Check

1. The episode-based handler aligns with FND-21 (revisable commitments): the episode is the commitment, wake conditions are the assumption surface, and a wake fires when assumptions break. Sleep becomes "rest until the conditions break" rather than "rest one tick at a time."
2. Wake-condition evaluation inside the tick handler avoids introducing a new sim-side scheduling primitive (per spec D5 design goal: "no new sim-side scheduling primitive needed"). The handler returns `ActionProgress::StopAndCommit` (or the existing equivalent that signals end-of-action) when any condition fires; the existing transition machinery handles commit and `SleepEpisodeEnded` emission.
3. Synthesis-in-systems (option a above) keeps the data flow local and respects layer boundaries (`worldwake-systems` reads from core, never from ai). The spec's original D10 location was provisional; this ticket corrects it.
4. `Interruptibility::FreelyInterruptible` honestly reflects the new model: wake-condition-driven exit incurs no penalty, and the partial-recovery aftermath is recorded via `accumulated_recovery` on `SleepEpisode` (FND-10).

## Verification Layers

1. Episode lifecycle (one start, one end, no intermediate re-commits) → action trace assertion (existing `ActionTraceSink` infrastructure).
2. `SleepEpisode` component present mid-episode, removed at commit → focused unit test asserting component presence at tick T (mid-episode) and absence at tick T+N (post-commit).
3. Per-tick recovery applied with `recovery_modifier` modulation → focused unit test asserting `HomeostaticNeeds.fatigue` decreases by exactly `(rest_efficiency × recovery_modifier ÷ 1000)` per tick.
4. Wake-condition evaluation fires the correct `WakeReason` → focused unit tests covering each `WakeCondition` → `WakeReason` mapping.
5. `SleepEpisodeStarted` and `SleepEpisodeEnded` emitted to event log with correct payloads → event-log delta assertion (existing `EventLog` test infrastructure).
6. Authoritative state (`HomeostaticNeeds.fatigue` after commit equals integrated recovery) → world-state assertion.
7. Layered separation: tick-handler logic at `crates/worldwake-systems/src/needs_actions.rs` and `crates/worldwake-systems/src/sleep_synthesis.rs` (new); event payloads at core; wake-condition evaluation reads belief from core (`FrameAssumption`). No reads of `worldwake-ai` types from systems.

## What to Change

### 1. New module `crates/worldwake-systems/src/sleep_synthesis.rs`

Create with the wake-condition synthesis function:

```rust
pub fn synthesize_wake_conditions(
    agent: EntityId,
    intended_max_ticks: NonZeroU32,
    target_recovery: Permille,
    current_tick: Tick,
    txn: &WorldTxn<'_>,
) -> Vec<WakeCondition>
```

Behavior per S128 spec D10:

1. Always push `WakeCondition::IntendedDurationReached`.
2. For each `HomeostaticNeedId` in `HomeostaticNeedId::ALL` except `Fatigue`: read the agent's `IntentionFrame` for `FrameAssumption::NeedSafeUntilTick { need: <this need>, until_tick }`. If `until_tick < current_tick + intended_max_ticks.get()`, push `WakeCondition::ProjectedNeedBreach { need }`. (S126's projection is recomputed at evaluation time inside the tick handler — this synthesis only declares which needs to monitor.)
3. Read the agent's intention queue for scheduled commitments due within `[current_tick, current_tick + intended_max_ticks.get()]`. For each, push `WakeCondition::ScheduledCommitmentDue { tick }`.
4. Always push `WakeCondition::LocalDisturbance`.
5. If `target_recovery < Permille::new_unchecked(1000)`, push `WakeCondition::TargetRecoveryReached`.

### 2. Refactor `tick_sleep` and add start/commit/abort handlers in `crates/worldwake-systems/src/needs_actions.rs`

Replace the existing `tick_sleep` and the `start_noop`/`commit_noop`/`abort_noop` bindings for sleep with bespoke handlers:

- `start_sleep_episode`: derive `intended_min_ticks` from `MetabolismProfile.min_sleep_ticks`. Derive `intended_max_ticks` from agent fatigue and `rest_efficiency` (e.g., `min(max_cap_ticks, current_fatigue / rest_efficiency × 1100/1000)` rounded up — pick a sensible cap such as `NonZeroU32::new(64).unwrap()` and document the formula in code; the formula must produce a value ≥ `intended_min_ticks`). Read agent's current place via `txn.effective_place(agent)`. Read the place's `SleepQualityProfile` via `txn.get_component_sleep_quality_profile(place)` (authoritative; default if absent — which after S128SLEEPIPLA-006 won't happen because every place will have one). Cache `recovery_modifier` on the new `SleepEpisode`. Set `target_recovery` to `Permille::new_unchecked(0)` initially (full recovery target — the agent wakes only if `WakeCondition::IntendedDurationReached` fires, unless overridden). Call `synthesize_wake_conditions` to populate the vec. Insert `SleepEpisode` via `txn.set_component_sleep_episode(agent, episode)`. Emit `SleepEpisodeStarted` event with the full payload.
- `tick_sleep` (refactored): read `SleepEpisode` for the actor. Compute per-tick recovery `delta = rest_efficiency × recovery_modifier ÷ 1000`. Update `accumulated_recovery = saturating_add(delta)` capped at `Permille::new_unchecked(1000)`. Update `HomeostaticNeeds.fatigue = saturating_sub(delta)`. Re-evaluate each `WakeCondition`:
  - `IntendedDurationReached`: fires when `current_tick - start_tick >= intended_max_ticks.get()`.
  - `TargetRecoveryReached`: fires when `accumulated_recovery >= (Permille::new_unchecked(1000) - target_recovery)` (i.e., enough has been recovered to hit the target fatigue).
  - `ProjectedNeedBreach { need }`: re-read `FrameAssumption::NeedSafeUntilTick` for that need; fires if `until_tick <= current_tick`.
  - `ScheduledCommitmentDue { tick }`: fires when `current_tick >= tick`.
  - `LocalDisturbance`: hooks into the existing local-perception channel; fires when a disturbance is perceived. (Confirm the perception hook during reassessment; if the channel doesn't exist as a clean signal, emit a minimal stub that never fires and document as a known limitation — this preserves the variant for future S60-style use without blocking the ticket.)
  - First condition that fires determines `WakeReason`. Return `ActionProgress::StopAndCommit` (or the existing equivalent — confirm the exact return type during reassessment; if the framework uses a different signal for "end action this step," use that).
  - If no condition fires, return `ActionProgress::Continue`.
- `commit_sleep_episode`: read `SleepEpisode` for the actor (it was either just terminated by tick handler or hit `intended_max_ticks`). Determine `WakeReason` from the firing condition (passed via `instance.local_state` if needed, or recomputed from current state). Read final `HomeostaticNeeds.fatigue`. Emit `SleepEpisodeEnded` event with full payload (`accumulated_recovery`, `final_fatigue`, `end_reason`). Remove the `SleepEpisode` component via `txn.clear_component_sleep_episode(agent)`. Return `CommitOutcome` with the recovery delta.
- `abort_sleep_episode`: same as commit but with `end_reason = WakeReason::LocalDisturbance` (or a synthetic abort reason if the framework distinguishes — confirm during reassessment). Removes `SleepEpisode`, emits `SleepEpisodeEnded`.

### 3. Update sleep registration

In `crates/worldwake-systems/src/needs_actions.rs:30-35` (handler binding):

```rust
let sleep_handler = handlers.register(ActionHandler::new(
    start_sleep_episode,
    tick_sleep,
    commit_sleep_episode,
    abort_sleep_episode,
));
```

In `crates/worldwake-systems/src/needs_actions.rs:69-75` (`register_def` call), change:

- `DurationExpr::Fixed(NonZeroU32::MIN)` → `DurationExpr::Variable { min: NonZeroU32::new(1).unwrap(), max: NonZeroU32::new(64).unwrap() }`.
- `Interruptibility::InterruptibleWithPenalty` → `Interruptibility::FreelyInterruptible`.
- `Precondition::ActorAlive` (line 166-ish) is unchanged.
- `ActionPayload::None` is unchanged (sleep stays untargeted at the action-registration level; per-place selection happens in S128SLEEPIPLA-005's candidate emitter).

### 4. Update `sleep_reduces_fatigue_without_a_bed` test

The existing test at `needs_actions.rs:738` asserts `tick_sleep` reduces fatigue by `rest_efficiency` per tick. Reframe as `sleep_episode_reduces_fatigue_at_default_place`: invoke `start_sleep_episode`, run `tick_sleep` for several ticks, assert (a) `SleepEpisode` is present after start, (b) `HomeostaticNeeds.fatigue` decreases by `rest_efficiency × 1` per tick (default place has `recovery_modifier: 1000`, so unchanged behavior at default), (c) `SleepEpisodeStarted` event is in the log, (d) after commit, `SleepEpisode` is removed and `SleepEpisodeEnded` is in the log. This is the canonical replacement for the old test's invariant.

## Files to Touch

- `crates/worldwake-systems/src/sleep_synthesis.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify — add `pub mod sleep_synthesis;` if module is publicly visible, or just `mod` if internal; confirm pattern from sibling modules)
- `crates/worldwake-systems/src/needs_actions.rs` (modify — replace handler bindings, refactor `tick_sleep`, add `start_sleep_episode` / `commit_sleep_episode` / `abort_sleep_episode`, update registration, reframe `sleep_reduces_fatigue_without_a_bed` test)
- `Likely: crates/worldwake-systems/src/needs_actions.rs` test module — additional focused unit tests for each `WakeCondition` → `WakeReason` mapping and for partial-recovery aftermath

## Out of Scope

- Per-place sleep candidate emission and ranking — handled by S128SLEEPIPLA-005
- Scenario authoring of `SleepQualityProfile` per place — handled by S128SLEEPIPLA-006
- Golden E2E tests for sleep episodes — handled by S128SLEEPIPLA-007
- `WakeCondition::PlaceNoLongerSafe` — out of scope per spec Non-Goals (deferred until S60)
- Decision-trace observer rendering for `SleepEpisodeStarted` / `SleepEpisodeEnded` — already handled as shared payload fallout in archive/tickets/S128SLEEPIPLA-001.md; no additional observer code is owned here

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-systems sleep_episode_reduces_fatigue_at_default_place` — reframed lifecycle test passes.
2. `cargo test -p worldwake-systems test_sleep_synthesis` (new test module function) — wake-condition synthesis returns the expected vec for each scenario (no projections, projection-with-breach, scheduled commitment, target recovery).
3. `cargo test -p worldwake-systems` — all existing tests pass; specifically `register_needs_actions_adds_all_six_defs_and_handlers` (line ~720) still passes after handler swap.
4. `cargo test -p worldwake-ai test_sleep_always_likely` — feasibility unchanged (sleep gains no preconditions).
5. `cargo test -p worldwake-ai` — existing AI tests pass (per-tick re-commit is gone, but per-tick decision behavior is ticket-005's territory; this ticket only changes the action lifecycle).
6. Existing suite: `cargo test --workspace`.

### Invariants

1. One `SleepEpisode` insertion per sleep adoption; one removal per commit.
2. Per-tick recovery equals `(rest_efficiency × recovery_modifier ÷ 1000)`, never exceeds current fatigue (saturating subtraction), never increases fatigue.
3. `SleepEpisodeStarted` and `SleepEpisodeEnded` are emitted exactly once per episode, with payloads matching component state.
4. The first matching `WakeCondition` determines `WakeReason`; ties resolved by enum declaration order (deterministic per FND-29A).
5. `accumulated_recovery` saturates at `Permille::new_unchecked(1000)`; cannot drive fatigue below `Permille::new_unchecked(0)`.
6. Sleep registration uses `DurationExpr::Variable` and `Interruptibility::FreelyInterruptible`; the old `Fixed(NonZeroU32::MIN)` and `InterruptibleWithPenalty` are not present (FND-28, no shim).
7. Synthesis lives in `worldwake-systems`; no `worldwake-ai` import from `worldwake-systems` (FND-26 / crate boundary).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` test module (modify — reframe `sleep_reduces_fatigue_without_a_bed` to `sleep_episode_reduces_fatigue_at_default_place`; add new tests for wake-on-projection, wake-on-target-recovery, partial-recovery aftermath, episode lifecycle one-shot).
2. `crates/worldwake-systems/src/sleep_synthesis.rs` (new — module-internal `#[cfg(test)]` tests for `synthesize_wake_conditions` per spec D10 cases).
3. Action-trace and event-log assertions piggyback on the existing test infrastructure (`ActionTraceSink`, `EventLog`); no new infrastructure needed.

### Commands

1. `cargo test -p worldwake-systems sleep_episode sleep_synthesis`
2. `cargo test -p worldwake-systems` (full crate; catches the registration sweep)
3. `cargo test -p worldwake-ai feasibility candidate_generation` (ensures the existing AI tests still pass while sleep stays untargeted)
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `./scripts/verify.sh`
