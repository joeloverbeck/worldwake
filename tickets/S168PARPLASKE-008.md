# S168PARPLASKE-008: Re-enable info-barrier suspension producer after contested-scenario validation

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `crates/worldwake-ai/src/agent_tick/planning.rs`, `crates/worldwake-ai/src/agenda_manager.rs`.
**Deps**: `archive/specs/S168-partial-plan-skeleton-reuse.md` D1.b; `archive/specs/S149-partial-plan-segments-and-typed-terminals.md` D7; `archive/tickets/S168PARPLASKE-006.md` (the original producer); `archive/tickets/S168PARPLASKE-007.md` (the first re-enable attempt, reverted on PR #130).

## Problem

`S168PARPLASKE-007` re-enabled `write_information_barrier_partial_plan_segment` with
witness gating (symmetric with `spawn_information_barrier_companions`) and a
`KillCondition::TickExpiry { at_tick: tick + search_exhaustion_backoff_ticks }`
safety net, then verified against the goldens originally regressed by S168PARPLASKE-006:
`survival_baseline::*`, `survival_tell::*`, `planner_pathology_degenerate::*`,
`simulation_gaps::*`, and `scenario_diagnostics_fixture::*`. All passed.

The verification surface missed `survival_contested`, which is the higher-contention
sibling of `survival_baseline` (4 agents instead of 3, tighter resource caps,
chokepoint topology). When the merged branch ran the full `golden-survival.yml`
matrix in CI, `survival_contested::no_stuck_idle_windows_with_elevated_needs` failed
deterministically:

```
StuckIdleWindow {
  agent_name: "Agent C",
  start_tick: 902,
  end_tick: 941,
  max_need_at_start: 384,
}
```

That window is exactly 40 ticks at need 384 permille (above the scenario contract's
300-permille elevated-need floor and the 40-tick stuck-window threshold). With the
default `search_exhaustion_backoff_ticks = 100`, the `TickExpiry` safety net is more
than twice the stuck-window threshold, so the safety net cannot bound the failure
under this contract.

PR #130 reverted just the producer code (`planning.rs`, `agenda_manager.rs`, and the
intentional fixture regeneration in `expected-scenario-diagnostics.json`) to the
pre-S168PARPLASKE-007 state to make CI green again. The skeleton-reuse infrastructure
(carrier on `CandidatePlanSearch`, seeded-search consumer, `PartialPlanResumeTrace`,
budget-exhausted producer) is untouched and remains live — only the info-barrier
producer is dormant again.

## Failure Modes To Resolve Before Re-enable

1. **Contested-scenario stuck idle window.** In `survival_contested`, Agent C at tick
   902 entered a 40-tick idle window with max need 384 permille while the producer
   was active. The TickExpiry safety net at +100 ticks is wider than the
   `max_idle_window_ticks_with_elevated_need = 40` contract — Agent C cleared the
   suspension at tick 942 (mechanism unknown — likely a belief change or kill
   evaluation; needs per-tick trace), but the 40-tick window during which it sat
   idle with elevated needs is itself the contract violation. Reproduced by
   `cargo test --release -p worldwake-ai --test golden_ai scenarios::survival_contested::no_stuck_idle_windows_with_elevated_needs -- --ignored --test-threads=1`.

2. **Safety-net window vs contract bound.** The producer's `TickExpiry` uses
   `search_exhaustion_backoff_ticks` (default 100), which is unrelated to the
   per-scenario `max_idle_window_ticks_with_elevated_need` contract (40 in contested,
   per `scenarios/survival-contested.ron`). A safety-net duration that can exceed
   the contract bound on idle windows under elevated need will never be sufficient
   on its own. Either the safety net needs a separate, contract-aware bound, or
   the producer needs an additional precondition (e.g., no need above the elevated
   floor) before it is allowed to suspend.

3. **Sibling-scenario coverage gap.** S168PARPLASKE-007 verified by enumerating the
   originally-regressed goldens of S168PARPLASKE-006. The contested scenario was not
   in that set even though it shares the survival-health contract surface with
   baseline. The re-enable verification needs to run every gated survival scenario
   (the full `golden-survival.yml` matrix), not just the 006-regressed subset.

## Architecture Check

1. **Suspending an agent with elevated needs is the trap.** Producer should not
   suspend the primary pursuit while any homeostatic need exceeds the
   scenario-authored elevated-need floor, because suspension is an asymmetric
   commitment: the agent gives up survival action throughput for a chance at
   information that may never arrive. Re-enable should add a need-pressure
   precondition (read from the actor's homeostatic-needs belief, in keeping with
   FND-14 belief-only planning).

2. **Safety net must be bounded by the strictest applicable contract.** If the
   producer suspends, its TickExpiry must be ≤ `max_idle_window_ticks_with_elevated_need`
   (or the analogous per-scenario contract). `search_exhaustion_backoff_ticks` is a
   planner-pressure parameter, not an idle-bound parameter; using it as the safety
   net was a category error.

3. **Verification scope is the full survival matrix, not the 006-regressed subset.**
   Re-enable cannot ship until every scenario in `.github/workflows/golden-survival.yml`
   passes locally with the producer active. The same goes for `golden-simulation-gaps.yml`,
   `golden-planner-pathology.yml`, `golden-cognitive-archetypes.yml`,
   `golden-drive-escalation.yml`, `golden-observer-anomalies.yml`,
   `golden-scenario-diagnostics.yml`, and `golden-item-decay.yml` — every gated
   golden family CI runs.

## Verification Layers

1. **Need-pressure precondition** → focused producer test proving suspension is
   declined when any homeostatic need exceeds the elevated-need floor (read via
   `GoalBeliefView::homeostatic_needs` or equivalent belief surface; do NOT read
   world state — FND-14).

2. **Contract-aware TickExpiry** → focused producer test proving the suspended
   entry's `kill_condition` uses a bound ≤ the scenario's
   `max_idle_window_ticks_with_elevated_need` rather than
   `search_exhaustion_backoff_ticks`. The bound source is a design choice —
   options are a per-scenario contract field, a cognitive-profile field, or a
   FND-conformant derived value. Specify and justify the chosen source in the
   ticket implementation.

3. **Contested-scenario re-greenness** →
   `cargo test --release -p worldwake-ai --test golden_ai scenarios::survival_contested -- --ignored --test-threads=1`
   passes with the producer re-enabled.

4. **Full survival matrix re-greenness** →
   `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1`
   passes with the producer re-enabled (all 17 scenarios in `golden-survival.yml`
   plus every other gated golden family).

5. **Originally-regressed goldens re-greenness** → the S168PARPLASKE-006 regression
   set from S168PARPLASKE-007's verification list must remain green when re-run.

6. **Diagnostics fixture intentional drift** → if and only if the producer is
   re-enabled and any scenario diagnostics shifts, regenerate
   `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` via
   `WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1` and cite the producer
   reactivation in the commit message (FND-1 truth-adjustment row).

## What to Change

### 1. Re-enable producer in `crates/worldwake-ai/src/agent_tick/planning.rs`

`write_information_barrier_partial_plan_segment` is currently the pre-S168PARPLASKE-006
no-op (returns `false`, params prefixed with `_`, comment cites this ticket as the
re-enable gate). Restore the active producer with the same shape S168PARPLASKE-007
used (witness-gated, kill-condition-safety-netted) PLUS:

- **Need-pressure precondition.** Decline suspension when any homeostatic need read
  from the actor's belief view exceeds the elevated-need floor (or whichever
  precondition the implementer derives from Architecture Check item 1).
- **Contract-aware kill condition.** Replace
  `KillCondition::TickExpiry { at_tick: tick + search_exhaustion_backoff_ticks }`
  with a bound ≤ the strictest applicable idle contract (see Architecture Check item 2).

Restore the `InformationBarrierProducerContext` struct, the two call-site context
constructions, and the early `view` placement in the traced call site.

### 2. Restore `pub(crate)` on `select_information_barrier_witness`

`crates/worldwake-ai/src/agenda_manager.rs` currently has the witness selector
private. The producer needs it; restore the `pub(crate)` visibility.

### 3. Restore producer focused tests

Re-add the four S168PARPLASKE-007 producer tests
(`_suspends_selected_goal_with_skeleton`, `_skips_when_no_witness_is_available`,
`_allows_missing_skeleton_source`, `_does_not_suspend_ask_witness_companion`).
Add the two new tests required by Verification Layers 1 and 2 (need-pressure
precondition and contract-aware TickExpiry).

### 4. Regenerate diagnostics fixture if drift

After producer re-enable, run the fixture stability golden; if it reports drift,
regenerate via `WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1`. Cite producer
reactivation in the commit message.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/agenda_manager.rs` (modify — visibility)
- `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` (modify — if drift)

## Out of Scope

- Skeleton reuse for budget-exhausted suspensions (already working).
- Other barrier kinds (Coordination, Resource, Jurisdiction).
- Replacing the AskWitness companion mechanism with a different information-acquisition primitive.
- Diagnosing why Agent C's suspension cleared at tick 942 specifically (the contract
  violation is the existence of the 40-tick window, not the specific clearance
  mechanism — fixing the architecture is the goal).

## Notes

- The skeleton-source carrier on `CandidatePlanSearch` and the seeded-search consumer
  remain unaffected by the producer's dormant state.
- This ticket's existence reflects the spec's own caveat (`archive/specs/S168-partial-plan-skeleton-reuse.md`):
  D7 is "explicitly the **lowest-benefit** of the accepted set — an optimization over
  an already-working resume path, not a correctness fix … the population scope can be
  narrowed further at ticket time without affecting the other Adjunct Wave specs."
