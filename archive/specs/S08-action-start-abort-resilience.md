**Status**: ✅ COMPLETED

# Action Start Abort Resilience

## Summary

S08 is still needed, but the current codebase changes what the correct fix should be.

Two defects remain live:

1. `AbortRequested` during action start still crashes BestEffort execution.
2. `start_heal` still destroys Medicine before any treatment effect is applied.

The original spec identified both defects correctly, but its proposed heal fix has drifted. In the current code, `tick_heal` applies treatment incrementally each tick and `commit_heal` is empty. That means "move medicine consumption to `commit_heal`" is no longer architecturally correct on its own: it would separate a conserved input from the incremental wound changes that currently happen during `tick_heal`.

The updated implementation direction is:

- treat handler-requested start aborts as recoverable BestEffort start failures;
- make `start_heal` validation-only;
- consume Medicine in the same transaction as the first successful treatment effect, not at start;
- preserve the current gradual-healing model unless the project intentionally redesigns treatment semantics in a separate spec.

## Reassessment Against Current Codebase

### Live drift-confirmed issues

- `crates/worldwake-sim/src/tick_step.rs::is_best_effort_start_failure()` still recognizes only `ReservationUnavailable | PreconditionFailed | InvalidTarget`. `ActionError::AbortRequested(_)` is still excluded, so a lawful world-state change between planning and start can still bubble out as `TickStepError::Action(...)`.
- `crates/worldwake-systems/src/combat.rs::start_heal()` still calls `consume_one_unit_of_commodity(..., Medicine)` during start.
- `crates/worldwake-systems/src/combat.rs::tick_heal()` still owns the real treatment effect by mutating `WoundList` every tick.
- `crates/worldwake-systems/src/combat.rs::commit_heal()` is still a no-op, so moving Medicine consumption there without redesigning heal phase semantics would not keep input consumption aligned with the actual wound mutation.

### Existing architecture that should be preserved

- `worldwake-ai` already has the right downstream plumbing for recoverable action-start failures:
  - `Scheduler::record_action_start_failure()`
  - `agent_tick.rs` drains `action_start_failures()`
  - `handle_plan_failure()` records blockers and clears the failed plan
- Several other `on_start` handlers already mutate world state lawfully:
  - travel start establishes in-transit occupancy and abort restores it;
  - defend start establishes combat stance and abort restores it;
  - craft start stages inputs into a production job.

This matters because S08 should not invent a blanket "no start mutation" rule. The real contract is narrower:

- `on_start` may establish occupancy, reservation, staging, or other reversible in-progress state;
- `on_start` should not destroy conserved resources or apply irreversible outcome state before the action has produced its first real effect.

### Outdated recommendation from the old draft

The old spec suggested removing the `no_recovery_combat_profile()` workaround from `crates/worldwake-ai/tests/golden_emergent.rs`. That is no longer a good blanket requirement.

Those goldens currently use zero natural recovery to assert specific care-driven emergent chains. Keeping that profile is still valid when the scenario needs treatment-caused healing rather than natural recovery noise. S08 should add targeted regression coverage for the race itself, not weaken unrelated goldens by forcing natural recovery back into every care scenario.

## Discovered Via

Originally surfaced by S07 care coverage, where wounds could disappear between planning, start, and early treatment ticks. The crash path is still real in the current code:

- planning produces a lawful `Heal` step;
- world state changes before `start_action()`;
- handler returns `ActionError::AbortRequested(...)`;
- BestEffort request path does not classify that as recoverable;
- `step_tick()` returns a hard error instead of recording a start failure and letting AI replan.

## Foundation Alignment

- **Principle 4**: conserved Medicine cannot disappear before any treatment effect exists.
- **Principle 8**: actions need explicit phase semantics; input consumption belongs to the phase where the action first does real work, not speculative start.
- **Principle 9**: healing remains granular when wound change still happens over time rather than only at the final commit boundary.
- **Principle 19**: plans are revisable commitments; if target wounds vanish before treatment begins, the engine must abort the start gracefully and replan instead of crashing.
- **Principle 24**: the fix should stay inside shared action-framework semantics plus combat state mutation, not AI-specific special casing.

## Phase

Phase 3: Information & Politics (bug fix, no phase dependency)

## Crates

- `worldwake-sim`
- `worldwake-systems`
- `worldwake-ai` verification coverage

## Dependencies

None.

## Architectural Decision

### 1. BestEffort start failures should include handler-requested start aborts

At action start, `AbortRequested` means the affordance was legal when chosen but invalid by the time authoritative execution began. In BestEffort mode that is normal world drift, not a fatal engine error.

This classification should remain narrow:

- recoverable: `ReservationUnavailable`, `PreconditionFailed`, `InvalidTarget`, `AbortRequested`
- not recoverable: `InternalError`, unknown action/handler, invalid status, or other framework corruption

### 2. Heal should keep gradual treatment semantics

Heal is currently modeled as a multi-tick treatment process via `DurationExpr::TargetTreatment` plus incremental `tick_heal()` wound updates. S08 should preserve that.

Therefore:

- do not redesign heal into a pure end-of-action commit unless that redesign is explicitly desired in a later spec;
- instead, move Medicine consumption out of `start_heal()` and into the first successful treatment tick, in the same transaction as the first wound mutation.

### 3. Start-phase mutation contract

This spec establishes the action-lifecycle rule the codebase should follow:

- `on_start` may create reversible "action is now in progress" state;
- conserved-resource destruction belongs to the same transaction as the first irreversible domain effect;
- if an action never reaches that first effect, conserved inputs must remain intact.

Heal is the motivating case, but this rule is general and should guide future multi-tick actions.

## Deliverables

### 1. Extend recoverable BestEffort start-failure classification

**File**: `crates/worldwake-sim/src/tick_step.rs`

Update `is_best_effort_start_failure()` so `ActionError::AbortRequested(_)` is treated the same way as the other recoverable authoritative-start failures.

Effect:

- `step_tick()` records `ActionStartFailure`
- action tracing records `ActionTraceKind::StartFailed`
- AI sees the failure via `scheduler.action_start_failures()`
- `handle_plan_failure()` can record blockers and clear the failed plan
- simulation continues instead of crashing

### 2. Make `start_heal()` validation-only

**File**: `crates/worldwake-systems/src/combat.rs`

`start_heal()` should validate context and allocate no irreversible cost.

Required outcome:

- no Medicine is consumed in `on_start`
- if wounds vanish before the first treatment tick, no resource is lost

### 3. Consume Medicine in the first successful treatment-effect transaction

**File**: `crates/worldwake-systems/src/combat.rs`

Refactor heal so Medicine is consumed together with the first successful wound mutation, not at start.

Required behavior:

- if the target already lost all wounds before treatment begins, `tick_heal()` aborts and Medicine remains untouched;
- if the first treatment tick succeeds, exactly one unit of Medicine is consumed in that same transaction;
- subsequent treatment ticks continue reducing wounds without consuming more Medicine;
- if the action aborts after some successful treatment ticks, Medicine remains spent because the world already received a real treatment effect.

Implementation note:

- If the existing handler API makes "first successful tick" bookkeeping awkward, prefer a small, explicit lifecycle extension over ad-hoc domain leakage.
- A narrow implementation that keys off the scheduler's unique first-tick boundary is acceptable only if it remains explicit in code and tests why that boundary is stable.
- Do not add global caches, magic counters, or AI-side exemptions.

### 4. Preserve the current heal timing model unless intentionally redesigned

S08 should not silently convert heal from gradual treatment into a single end-of-action state flip.

That means at least one of these must remain true after implementation:

- treatment still unfolds incrementally during active ticks; or
- a separately documented redesign explicitly changes healing semantics, AI expectations, and action duration meaning.

For S08, the first option is the intended scope.

### 5. AI pipeline verification per authoritative-to-AI rule

Because this change touches authoritative action-start behavior and heal execution timing, the full AI decision path must still be verified:

1. `get_affordances()` still exposes heal when the agent lawfully sees wounds and controls Medicine.
2. `generate_candidates()` still produces `GoalKind::TreatWounds`.
3. `search_plan()` still finds valid care plans.
4. action start in `step_tick()` records recoverable start failures instead of crashing.
5. `handle_plan_failure()` still records blockers and clears the failed step after start rejection.
6. relevant care golden coverage still passes.

## Tests

### `worldwake-sim`

- Add or update a unit test proving a BestEffort request whose `on_start` returns `ActionError::AbortRequested(...)` does not fail the tick and is recorded as `ActionStartFailure`.
- Preserve strict-mode behavior: the same start error should still propagate under `ActionRequestMode::Strict`.

### `worldwake-systems`

- Update heal lifecycle tests so Medicine is no longer expected to disappear at start.
- Add a regression test where:
  - heal starts lawfully,
  - the target loses wounds before the first treatment tick,
  - `tick_heal()` aborts,
  - no Medicine is consumed.
- Add a regression test proving Medicine is consumed exactly once on the first successful treatment tick, not once per tick and not at start.
- Preserve coverage that heal still reduces bleeding/severity and eventually commits.

### `worldwake-ai`

- Add a targeted AI regression proving a care plan whose start now returns `AbortRequested(TargetHasNoWounds | TargetLacksWounds)` does not crash the simulation and causes normal plan-failure handling.
- Do not require blanket removal of `no_recovery_combat_profile()` from existing emergent goldens. Only adjust those goldens if a specific assertion truly depends on the old broken behavior.

## Non-Goals

- No new reservation system for Medicine in this spec.
- No redesign of `DurationExpr::TargetTreatment`.
- No broad action-framework rewrite beyond what is needed to express a clean first-effect consumption boundary.
- No AI-side workaround that paper-over fixes authoritative action behavior.

## Risks

- If heal input consumption is keyed to "first successful tick" without explicit action-local bookkeeping, the code can become scheduler-coupled. Keep that dependency visible and tested.
- A broader reusable "mutable local tick state" mechanism may become worthwhile if more multi-tick actions need the same pattern. That is a follow-up candidate, not required scope for S08 unless the current implementation becomes contorted.
- BestEffort recovery must remain narrow. Accidentally swallowing internal framework errors would make debugging harder and weaken determinism guarantees.

## FND-01 Section H Analysis

### Information-Path Analysis

No new belief pathway is introduced. The relevant path is already:

- authoritative start rejection in `worldwake-sim`
- `Scheduler::record_action_start_failure()`
- AI reads `action_start_failures()`
- AI converts failure into blocked-intent / replanning state

S08 preserves that path and widens it to include lawful handler-requested start aborts.

### Positive-Feedback Analysis

The current failure mode is not a growth loop but a crash edge. The main behavioral loop here is:

- agent plans treatment
- world changes
- start fails
- AI records blocker
- AI replans next tick

That loop already has dampening:

- blocked-intent memory TTL
- blocker resolution checks
- plan invalidation and dirty-runtime clearing

S08 should preserve those dampeners and must not introduce retry spam through silent failure swallowing.

### Concrete Dampeners

- `BlockedIntentMemory` expiry
- blocker revalidation through `clear_resolved_blockers()`
- authoritative affordance/precondition checks
- finite treatment duration and finite Medicine inventory

### Stored State vs. Derived

Stored state already involved:

- item lots / Medicine quantities
- wound lists
- scheduler action-start failure records
- current plan / blocked-intent memory

Derived state:

- AI `ActionStartFailureSummary`
- derived blocker classification from abort reasons

S08 should not introduce abstract truth caches. Any added bookkeeping should stay action-local and minimal.

## Outcome

- Completion date: 2026-03-19
- What actually changed:
  - `worldwake-sim` now treats `ActionError::AbortRequested(_)` as a recoverable BestEffort start failure and preserves structured start-failure data for AI/runtime consumption.
  - `worldwake-systems` made `start_heal()` validation-only and now spends Medicine exactly once on the first successful treatment-effect tick via explicit action-local heal state.
  - `worldwake-ai` now consumes start failures through the canonical execution-failure path, and golden care coverage proves the real AI loop handles pre-start wound disappearance as `StartFailed` plus blocked-intent persistence rather than a crash.
- Deviations from original plan:
  - The finished care race rejects at the shared authoritative precondition boundary (`PreconditionFailed(\"TargetHasWounds(0)\")`) when wounds disappear before input drain, rather than forcing the rejection deeper into `start_heal()`. Focused coverage still preserves the lower-layer `AbortRequested(...)` path.
  - A narrow `worldwake-sim` substrate change was required so tick handlers can persist action-local state and convert lawful `on_tick()` aborts into clean action aborts.
- Verification results:
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-systems`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
