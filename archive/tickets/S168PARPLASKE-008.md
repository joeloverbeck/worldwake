# S168PARPLASKE-008: Re-enable info-barrier suspension producer after contested-scenario validation

**Status**: COMPLETED
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

## Resolved Failure Modes

1. **Contested-scenario stuck idle window.** In `survival_contested`, Agent C at tick
   902 entered a 40-tick idle window with max need 384 permille while the producer
   was active. The TickExpiry safety net at +100 ticks is wider than the
   `max_idle_window_ticks_with_elevated_need = 40` contract — Agent C cleared the
   suspension at tick 942 (mechanism unknown — likely a belief change or kill
   evaluation), but the 40-tick window during which it sat
   idle with elevated needs is itself the contract violation. Reproduced by
   `cargo test --release -p worldwake-ai --test golden_ai scenarios::survival_contested::no_stuck_idle_windows_with_elevated_needs -- --ignored --test-threads=1`.

2. **Safety-net window vs contract bound.** The producer's `TickExpiry` uses
   `search_exhaustion_backoff_ticks` (default 100), which is unrelated to the
   per-scenario `max_idle_window_ticks_with_elevated_need` contract (40 in contested,
   per `scenarios/survival-contested.ron`). A safety-net duration that can exceed
   the contract bound on idle windows under elevated need will never be sufficient
   on its own. Either the safety net needs a separate, contract-aware bound, or
   this ticket fixed both by adding a pressure precondition and replacing the
   search-backoff expiry with a transient-block expiry.

3. **Sibling-scenario coverage gap.** S168PARPLASKE-007 verified by enumerating the
   originally-regressed goldens of S168PARPLASKE-006. The contested scenario was not
   in that set even though it shares the survival-health contract surface with
   baseline. This ticket reran the full ignored `golden_ai` matrix, which includes
   the survival matrix and the other gated golden families.

## Architecture Check

1. **Suspending an agent with elevated needs is the trap.** Producer should not
   suspend the primary pursuit while any homeostatic need exceeds the
   scenario-authored elevated-need floor, because suspension is an asymmetric
   commitment: the agent gives up survival action throughput for a chance at
   information that may never arrive. The landed producer reads the actor's
   homeostatic needs through the belief/profile view and declines suspension when
   any need exceeds its profile-authored low threshold.

2. **Safety net must be bounded by the strictest applicable contract.** If the
   producer suspends, its `TickExpiry` is now `tick + cognitive.transient_block_ticks`.
   The default and survival-authored value is 20 ticks, below the strictest survival
   idle-window contract currently exercised by the ignored matrix.

3. **Verification scope is the full survival matrix, not the 006-regressed subset.**
   Re-enable was verified with the full ignored `golden_ai` matrix rather than only
   the originally regressed subset.

## Verified Layers

1. **Need-pressure precondition** -> focused producer test proves suspension is
   declined when an actor's believed/profile-visible homeostatic need exceeds that
   need's profile-authored low threshold. The producer uses `ProfileBeliefView`, not
   an authoritative world-state read.

2. **Contract-aware TickExpiry** -> focused producer test proves the suspended
   entry's `kill_condition` uses `cognitive.transient_block_ticks`, not
   `search_exhaustion_backoff_ticks`. This keeps the safety net on the profile-owned
   transient-block TTL rather than a search-budget retry TTL.

3. **Contested-scenario re-greenness** ->
   `cargo test --release -p worldwake-ai --test golden_ai scenarios::survival_contested -- --ignored --test-threads=1`
   passed with the producer re-enabled.

4. **Full ignored golden matrix re-greenness** ->
   `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1`
   passed with the producer re-enabled after the diagnostics fixture was regenerated.

5. **Originally-regressed goldens re-greenness** -> the S168PARPLASKE-006 regression
   set from S168PARPLASKE-007's verification list passed as part of the full ignored
   matrix.

6. **Diagnostics fixture intentional drift** -> producer reactivation shifted the
   scenario diagnostics fixture. It was regenerated through
   `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` via
   `WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1`.

## Landed Changes

### 1. Re-enabled producer in `crates/worldwake-ai/src/agent_tick/planning.rs`

`write_information_barrier_partial_plan_segment` is active again at the selected-plan
boundary. It restores the S168PARPLASKE-007 witness-gated producer shape with the
additional pressure and expiry protections landed here:

- **Need-pressure precondition.** Suspension is declined when any actor homeostatic
  need read from `ProfileBeliefView` exceeds that need's profile-authored low
  threshold. Missing needs remain "no active pressure" because the producer cannot
  infer survival pressure without a need carrier; missing thresholds fall back to
  `DriveThresholds::default()`.
- **Contract-aware kill condition.** Produced suspensions use
  `KillCondition::TickExpiry { at_tick: tick + cognitive.transient_block_ticks }`,
  so the safety net is tied to the transient-block retry window instead of
  `search_exhaustion_backoff_ticks`.

The restored `InformationBarrierProducerContext` carries actor, belief view, tick,
and cognitive profile into the producer. Both selected-plan call sites pass the
fresh post-observation belief view, avoiding stale borrows across world mutation.

### 2. Restored `pub(crate)` on `select_information_barrier_witness`

`crates/worldwake-ai/src/agenda_manager.rs` exposes the existing witness selector at
`pub(crate)` visibility so the producer and companion consumer share one witness
admission rule.

### 3. Restored and extended producer focused tests

The focused producer family now covers:

- `write_information_barrier_partial_plan_segment_suspends_selected_goal_with_skeleton`
- `write_information_barrier_partial_plan_segment_skips_when_no_witness_is_available`
- `write_information_barrier_partial_plan_segment_allows_missing_skeleton_source`
- `write_information_barrier_partial_plan_segment_does_not_suspend_ask_witness_companion`
- `write_information_barrier_partial_plan_segment_skips_under_homeostatic_pressure`

The first test also proves `TickExpiry` uses `transient_block_ticks` even when
`search_exhaustion_backoff_ticks` is 100.

### 4. Regenerated diagnostics fixture

The first full ignored matrix run exposed only the expected
`scenario_diagnostics_fixture` drift. The fixture was regenerated through the
canonical update environment variable, rerun normally, and then covered by the final
full ignored matrix.

## Landed Files

- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `crates/worldwake-ai/src/agenda_manager.rs`
- `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json`
- `archive/tickets/S168PARPLASKE-008.md`

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

## Outcome

Completed on 2026-05-25.

- Re-enabled information-barrier suspension production with the same witness selector
  used by `spawn_information_barrier_companions`.
- Added a belief/profile-view pressure gate so agents with active homeostatic pressure
  keep the selected survival path instead of suspending it for an information query.
- Replaced the former `search_exhaustion_backoff_ticks` expiry with
  `transient_block_ticks`.
- Regenerated the scenario diagnostics fixture to reflect the reactivated producer.

## Deviations

- The pressure gate does not read the scenario-only
  `survival_health_contract.elevated_need_floor`, because that contract is a golden
  assertion surface, not runtime planner state. The landed runtime gate uses each
  actor's profile-authored low threshold as the lawful live pressure floor.
- The expiry source is the existing cognitive-profile `transient_block_ticks`; no new
  persisted profile field or scenario contract field was added.
- The final full ignored golden matrix was run before a later removal of an incidental
  unit-test fixture seed. That late edit touched only non-release unit-test setup; the
  focused producer tests, `cargo test -p worldwake-ai`, and CI-shaped clippy were
  rerun after it.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib write_information_barrier_partial_plan_segment`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo test --release -p worldwake-ai --test golden_ai scenarios::survival_contested -- --ignored --test-threads=1`.
- Passed `WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1 cargo test --release -p worldwake-ai --test golden_ai scenarios::scenario_diagnostics_fixture::golden_scenario_diagnostics_survival_baseline_fixture_is_stable -- --ignored --exact`.
- Passed `cargo test --release -p worldwake-ai --test golden_ai scenarios::scenario_diagnostics_fixture::golden_scenario_diagnostics_survival_baseline_fixture_is_stable -- --ignored --exact`.
- Passed `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
