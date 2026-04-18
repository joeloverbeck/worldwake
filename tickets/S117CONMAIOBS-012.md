# S117CONMAIOBS-012: Baseline `MaintenanceStarvation` disposition after detector correction

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — investigation, trace analysis, and roadmap/spec disposition only
**Deps**: `archive/tickets/S117CONMAIOBS-010.md`, `S117CONMAIOBS-007`, `specs/S117-convergence-maintenance-observer-smells.md`

## Problem

After correcting `MAINTENANCE_STARVATION` to report one real strongest 200-tick window per `(agent, need)` and tightening the predicate to high-band, majority-unrelieved cadence failure, `scenarios/survival-baseline.ron` still emits three maintenance anomalies:

- Agent A hunger: accumulated `410`, relieved `0`, avg `667`, high threshold `650`, ticks `841–1040`
- Agent B hunger: accumulated `603`, relieved `0`, avg `700`, high threshold `620`, ticks `1141–1340`
- Agent B thirst: accumulated `595`, relieved `234`, avg `698`, high threshold `650`, ticks `838–1037`

These are no longer merged-span contradictions or tiny-drift artifacts. Before suppressing them, the project needs an explicit disposition ticket to decide whether the baseline scenario is no longer honestly healthy, whether planner/coupled-need behavior is wrong, or whether the observer contract should still be narrowed further for a principled reason.

## Assumption Reassessment (2026-04-18)

1. `S117CONMAIOBS-010` now owns the detector correctness fix only: one strongest real 200-tick window per `(agent, need)` plus a tighter starvation gate in [`crates/worldwake-cli/src/bin/observer.rs`](../crates/worldwake-cli/src/bin/observer.rs). The remaining baseline maintenance anomalies survive that correction and therefore cannot be dismissed as the original merge bug.
2. Shared abstraction boundary under audit: the healthy-baseline survival behavior across scenario substrate, planner action selection/revision, and observer maintenance diagnostics. This is intentionally mixed-layer; the ticket's job is to decide which layer owns the contradiction.
3. The motivating invariant is not "maintenance starvation must never fire." It is: if `survival-baseline.ron` remains the project's intended healthy baseline, then high-band 200-tick hunger/thirst windows with severe relief lag require a concrete explanation and a specific owning fix boundary.
4. The strongest current evidence is already local and concrete: the corrected observer report on `survival-baseline.ron` shows the three windows above, all with rendered numbers that satisfy the tightened detector predicate exactly.
5. Existing repo expectations still treat `survival-baseline.ron` as healthy. The scenario declares a `survival_health_contract`, and `crates/worldwake-ai/tests/golden_survival_baseline.rs` still expects bounded critical runs on a full-day baseline.
6. Adjacent contradictions remain separate:
   - `GEOGRAPHIC_CONVERGENCE` lawful split-support false positives were fixed in archived `S117CONMAIOBS-009.md`
   - `ACUTE_NEED_SPIKE` baseline disposition belongs to `S117CONMAIOBS-011`
7. Per `docs/FOUNDATIONS.md`, this ticket must not solve the contradiction by adding scenario-name suppression or hiding a real high-need window inside observer-only filtering. The disposition must stay grounded in concrete local state, agent-belief planning, and post-hoc traceability.

## Architecture Check

1. A dedicated disposition ticket is cleaner than continuing to tighten `MAINTENANCE_STARVATION` until the baseline goes quiet. The detector is now mechanically honest; if baseline still fails it, the remaining question is architectural ownership, not detector arithmetic.
2. Keeping this investigation separate from `010` preserves a clean detector contract and makes any eventual scenario/planner/observer follow-up explicit rather than hidden behind another threshold tweak.

## Verification Layers

1. Remaining baseline maintenance windows are real under the corrected detector contract -> observer Section 3 output plus focused detector tests
2. Baseline still claims a healthy authored envelope -> `scenarios/survival-baseline.ron` plus `golden_survival_baseline.rs`
3. Owning contradiction layer (scenario vs planner vs observer semantics) -> decision traces, action traces, local survival summaries, and scenario substrate audit
4. This is an investigation/disposition ticket, so no stronger mutation-layer proof applies until an owning implementation ticket is created
5. Mixed-layer ticket: traces prove the outcome, and the follow-up implementation ticket should be opened only after the layer ownership is explicit

## What to Change

### 1. Audit the corrected baseline maintenance windows end-to-end

Use the corrected observer output, decision traces, action traces, and local survival summaries to explain exactly why the three baseline hunger/thirst starvation windows happen and whether they are lawful-but-unhealthy, scenario-authored, or planner-induced.

### 2. Evaluate remedy classes against `FOUNDATIONS.md`

Compare at least these remedy classes explicitly:

- keep detector unchanged; repair baseline scenario substrate
- keep detector unchanged; repair planner coupled-need behavior
- keep underlying behavior unchanged; narrow observer semantics further only if a principled false-positive argument still survives the corrected detector contract

For each class, record why it does or does not satisfy local-belief planning, concrete state, and traceability.

### 3. Produce a concrete owning follow-up

At closeout, create or update the exact owning implementation path:

- scenario repair ticket, or
- planner/AI behavior ticket, or
- bounded observer semantic-refinement ticket

If more than one layer truly needs work, split them explicitly and document dependency order.

## Files to Touch

- `tickets/S117CONMAIOBS-007.md` (modify if the owning baseline-regression handoff needs a factual note)
- `specs/S117-convergence-maintenance-observer-smells.md` (modify only if the maintenance detector spec claim is proven factually wrong after the disposition)
- `tickets/` (new follow-up implementation ticket(s) created from this disposition if warranted)

## Out of Scope

- Implementing the eventual scenario/planner/observer fix in this ticket
- Reopening the corrected detector arithmetic in `S117CONMAIOBS-010`
- Weakening `MAINTENANCE_STARVATION` just to preserve the current baseline narrative

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-cli --test golden_observer_anomalies`
2. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
3. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. The ticket concludes with an explicit owning layer for the remaining baseline maintenance contradiction, backed by code/trace/scenario evidence.
2. Any recommended fix path remains aligned with `docs/FOUNDATIONS.md` and does not rely on scenario-name suppression or detector-only narrative cleanup.

## Test Plan

### New/Modified Tests

1. `None — investigation/disposition ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-cli --test golden_observer_anomalies`
2. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
3. `cargo test -p worldwake-cli`
