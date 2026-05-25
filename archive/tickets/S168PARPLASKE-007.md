# S168PARPLASKE-007: Re-enable info-barrier suspension producer after end-to-end D7 validation

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `crates/worldwake-ai/src/agent_tick/planning.rs`, `crates/worldwake-ai/src/agenda_manager.rs`.
**Deps**: `archive/specs/S168-partial-plan-skeleton-reuse.md` D1.b; `archive/specs/S149-partial-plan-segments-and-typed-terminals.md` D7; `archive/tickets/S168PARPLASKE-006.md` (the disabled producer).

## Problem

Before this ticket, S168PARPLASKE-006 had added `write_information_barrier_partial_plan_segment` to activate the previously-dormant S149 D7 mechanism: when a plan terminal is `InformationBarrier { topic }`, the agent suspends the primary pursuit so `spawn_information_barrier_companions` can spawn an `AskWitness` companion that learns the topic, after which the primary resumes via `BeliefStatusChanged`.

Activating the producer broke 6 gated goldens (planner-pathology degenerate, simulation-gaps multi-agent-convergence, scenario-diagnostics fixture, survival-tell, survival-baseline drink + dirtiness). The fix in this branch disabled the producer; the D7 chain returned to its pre-S168 dormant state (consumer + resume conditions present, but never driven).

The disable was conservative. The D7 mechanism was architecturally meaningful, but the producer could not ship until the chain was validated end-to-end. The specific failure modes the disable papered over needed each to be diagnosed and fixed before the producer was reactivated.

## Failure Modes To Resolve Before Re-enable

1. **Witness unavailable at suspension time.** `select_information_barrier_witness` requires a co-located, alive, agent witness who plausibly knows the topic and has not already been asked. When no witness exists, no companion spawns and the suspended entry sits forever (`revival_trigger: None`, `kill_condition: External`). Reproduced by `planner_pathology_degenerate::degenerate_zero_step_loop_blocks_actionable_goals` (Lina alone in Eldergrove with no co-located agents).
2. **Multi-agent scenarios where witnesses ARE co-located but the chain still fails.** Survival baseline and simulation-gaps had multiple agents in shared places yet still failed. The producer's promise to the consumer (a companion will spawn and resolve the barrier) was not honored in practice. Diagnose whether the companion AskWitness spawns, whether it executes successfully, whether the witness's belief actually contains the topic, and whether `BeliefStatusChanged` fires on the primary afterward.
3. **No safety net for stuck suspensions.** The current `KillCondition::External` makes suspended info-barrier entries unrecoverable except through the companion path. If that path is broken or never fires, the agent is trapped.

## Architecture Check

1. **Witness gating must be symmetric with the consumer.** If the producer suspends only when `spawn_information_barrier_companions` could plausibly spawn a companion (same witness selector applied), suspension and companion creation become guaranteed-symmetric — no orphaned suspensions.
2. **Safety-net kill condition.** Even with a witness present at suspension time, the chain can break later (witness leaves, companion fails). Adding `KillCondition::TickExpiry { at_tick: tick + N }` (or a TickElapsed resume condition) bounds the trap — at worst the agent loses N ticks before re-creating the goal.
3. **Skeleton reuse already in place.** The skeleton-source carrier and the seeded-search consumer are unaffected by the producer's disabled state; budget-exhausted skeleton reuse continues to work. The D7-producer's contribution is purely about activating the suspension/companion/resume cycle.

## Verified Layers

1. **Witness-gate symmetry** → added focused producer tests covering the selected-plan suspension path with and without a plausible witness. The producer now calls the same `select_information_barrier_witness(actor, beliefs, topic)` helper as `spawn_information_barrier_companions`.
2. **Companion → resume chain** → existing S168 partial-plan-terminal goldens still prove the resume trace (`ReusedSeededSearch` / fallback), existing agenda-manager tests still prove companion spawning from suspended information-barrier segments, and the restored producer tests prove the selected-plan producer now creates the suspended segment only when that consumer can plausibly spawn a companion.
3. **No-witness fallback** → added focused producer test proving no suspension when no co-located plausible witness exists; the selected information-barrier plan remains on the normal adoption path.
4. **Safety-net expiry** → added focused producer assertion that produced information-barrier suspensions use `KillCondition::TickExpiry { at_tick: tick + cognitive.search_exhaustion_backoff_ticks }`; existing agenda-manager coverage proves `TickExpiry` clears entries on or after expiry.
5. **Originally-regressed goldens** → reran the named ignored regression surfaces after re-enable. The scenario diagnostics fixture intentionally changed because D7 now produces information-barrier/AskWitness diagnostics again, so `expected-scenario-diagnostics.json` was regenerated through the fixture's built-in update path and then rerun normally.

## Landed Changes

### 1. Witness gate in producer

`write_information_barrier_partial_plan_segment` now receives the actor and live belief view through a small producer context. Before suspending, it calls `select_information_barrier_witness` (made `pub(crate)` in `agenda_manager.rs`) and returns `false` when no plausible witness is available.

### 2. Kill-condition safety net

Produced information-barrier suspensions now use `KillCondition::TickExpiry { at_tick: tick + cognitive.search_exhaustion_backoff_ticks }`, so the suspended primary is bounded even if the companion path subsequently breaks.

### 3. Companion-chain proof

No duplicate `partial_plan_terminals.rs` scenario block was added. The live proof split is:

- producer creates the suspended segment only when the companion consumer can plausibly spawn (`agent_tick/planning.rs` focused producer tests);
- consumer still spawns the `AskWitness` companion from suspended information-barrier segments (`agenda_manager.rs` focused tests);
- resume trace behavior remains covered by existing S168 partial-plan-terminal goldens;
- long-run ignored scenario goldens cover the end-to-end survival and diagnostics surfaces that previously regressed when the producer was enabled.

### 4. Restore producer call sites

The two call sites in `plan_and_validate_next_step_*_with_opportunity_index` now pass the actor and belief view into the producer. When suspension succeeds, they keep the original early-return-with-trace behavior and clear the adopted runtime plan state.

### 5. Restore the focused producer unit tests

Restored and expanded the producer tests with real `PerAgentBeliefView` fixtures:

- `write_information_barrier_partial_plan_segment_suspends_selected_goal_with_skeleton`
- `write_information_barrier_partial_plan_segment_skips_when_no_witness_is_available`
- `write_information_barrier_partial_plan_segment_allows_missing_skeleton_source`
- `write_information_barrier_partial_plan_segment_does_not_suspend_ask_witness_companion`

## Out of Scope

- Skeleton reuse for budget-exhausted suspensions (already working).
- Other barrier kinds (Coordination, Resource, Jurisdiction).
- Replacing the AskWitness companion mechanism with a different information-acquisition primitive.

## Notes

- The consumer (`spawn_information_barrier_companions`) is live again through the restored producer path.
- The skeleton-source carrier on `CandidatePlanSearch` remains shared with the budget-exhausted producer.
- `information_barrier_partial_plan_segment` (the constructor in `partial_plan.rs`) remains the producer-owned segment builder; its focused construction test in `partial_plan.rs` remains green.

## Outcome

Completed on 2026-05-24.

Outcome amended: 2026-05-25.

- Re-enabled information-barrier partial-plan production at the selected-plan boundary.
- Made producer suspension symmetric with the companion consumer by sharing the live witness selector.
- Added `TickExpiry` as the safety net for produced information-barrier suspensions.
- Kept `AskWitness` / `ShareBelief` social plans executable instead of suspending the companion path itself.
- Regenerated the scenario diagnostics fixture to reflect the reactivated D7 information-barrier and AskWitness diagnostics.
- This implementation was later reverted on PR #130 after the full survival matrix
  exposed a contested-scenario stuck-idle regression. The final re-enable contract
  landed in `archive/tickets/S168PARPLASKE-008.md`.

## Deviations

- The drafted "new golden" was not added as a duplicate scenario block. Existing S168 partial-plan-terminal goldens already cover the resume trace, existing agenda-manager tests cover companion spawning, and this ticket added the missing producer-side witness/no-witness/safety-net proof. The originally regressed ignored goldens were rerun to cover the long-run end-to-end surfaces after re-enable.
- `KillCondition::TickExpiry` uses `search_exhaustion_backoff_ticks` exactly as drafted. That is a bounded retry window for this producer, not a new information-barrier-specific profile field.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib write_information_barrier_partial_plan_segment`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::planner_pathology_degenerate::degenerate_zero_step_loop_blocks_actionable_goals -- --ignored --exact`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::simulation_gaps::golden_multi_agent_convergence -- --ignored --exact`.
- Passed `WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1 cargo test -p worldwake-ai --test golden_ai scenarios::scenario_diagnostics_fixture::golden_scenario_diagnostics_survival_baseline_fixture_is_stable -- --ignored --exact` to regenerate the diagnostics fixture.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::scenario_diagnostics_fixture::golden_scenario_diagnostics_survival_baseline_fixture_is_stable -- --ignored --exact`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_tell::survival_tell_lands_row_five -- --ignored --exact`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_tell::listener_with_critical_dirtiness_breaks_off_tell_to_wash -- --ignored --exact`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_baseline::all_agents_perform_survival_actions -- --ignored --exact`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_baseline::no_stuck_idle_windows_with_elevated_needs -- --ignored --exact`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_baseline::all_agents_survive_1440_ticks -- --ignored --exact`.
