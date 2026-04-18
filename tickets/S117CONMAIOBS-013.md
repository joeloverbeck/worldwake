# S117CONMAIOBS-013: Planner split-support survival preparation for baseline acute-window recovery

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — planner-side survival candidate/ranking behavior in `worldwake-ai`
**Deps**: `archive/tickets/S117CONMAIOBS-011.md`, `archive/specs/S104-survival-baseline-recovery.md`, `docs/planner-contracts.md`, `archive/tickets/S117CONMAIOBS-014.md`

## Problem

Archived `S117CONMAIOBS-011.md` concluded that the remaining `survival-baseline.ron` acute and maintenance anomalies are not observer bugs and not the best target for scenario retuning. The live contradiction is planner-side: on split-support topology where `Fertile Fields` provides food only and `Riverside Camp` provides water/wash/sleep, `Agent B` repeatedly selects reactive one-need survival goals and oscillates between those places until hunger/thirst enter acute and sustained-critical windows. The baseline contract still says all authored agents should survive 1440 ticks with bounded critical runs, so the planner needs a bounded mechanism to prepare complementary self-care support before or during remote split-support travel without violating local-belief planning or introducing an omniscient safety score.

## Assumption Reassessment (2026-04-18)

1. `archive/tickets/S117CONMAIOBS-011.md` and archived `S117CONMAIOBS-012.md` proved the baseline acute and maintenance windows are real symptoms of the same split-support failure mode. The current ticket owns the first implementation path chosen by that disposition: planner/AI behavior, not observer suppression and not scenario-name cleanup.
2. Shared abstraction boundary under audit: self-care candidate generation and selection for split-support travel in `worldwake-ai`, specifically the interaction between `emit_need_driven_candidates()` (`crates/worldwake-ai/src/candidate_generation.rs`), `rank_candidates()` (`crates/worldwake-ai/src/ranking.rs`), and `build_candidate_plans()` / selection in `crates/worldwake-ai/src/agent_tick/planning.rs`.
3. Live baseline traces show the current planner is reactive and one-need-at-a-time in the failing windows. `Agent B` repeatedly selects `AcquireCommodity { commodity: Apple, purpose: SelfConsume }`, `ConsumeOwnedCommodity { commodity: Apple }`, `ConsumeOwnedCommodity { commodity: Water }`, `Relieve`, and `Sleep` as separate competing survival goals while local summaries alternate between `Fertile Fields: food=yes, water=no, wash=no, sleep=no` and `Riverside Camp: food=no, water=yes, wash=yes, sleep=yes`.
4. `rank_candidates()` in `crates/worldwake-ai/src/ranking.rs` derives self-care motive score from the maximum per-goal drive input and sorts goals independently; it does not represent a coupled “prepare for remote split-support trip” contract. `emit_need_driven_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs` likewise emits per-need `AcquireCommodity` / `ConsumeOwnedCommodity` goals, not a bundled complementary-support preparation branch.
5. `build_candidate_plans()` in `crates/worldwake-ai/src/agent_tick/planning.rs` then plans and reselects one opportunity at a time, with same-goal continuation scoped to the active goal family. This is consistent with the baseline trace summaries showing repeated `SearchSelection` for apple-travel plans rather than any preparatory water-buffering step before leaving the camp.
6. `docs/planner-contracts.md` confirms the planner consumes grounded goals and current strategic output one active step at a time; there is no existing multi-step strategic fallback layer that already carries “prepare complementary support, then travel for food” semantics for self-care.
7. This ticket must stay belief-local. Any split-support preparation signal must derive from planner-visible local state, believed place support, carried inventory, and existing travel knowledge. Global scenario-name suppressions or omniscient “future safety score” shortcuts are out of bounds under `docs/FOUNDATIONS.md` principles 14, 20, 21, and 27.
8. The ticket does not need to make `AcuteNeedSpike` disappear in every scenario. The owned invariant is narrower: on the authored healthy baseline, the planner should stop producing the known split-support oscillation that currently drives the baseline acute and maintenance windows.
9. The earlier proof-surface contradiction in the ignored survival golden has now been isolated and corrected by `S117CONMAIOBS-014`. The exact selector `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact` is once again a lawful behavior-level oracle for this planner ticket.

## Architecture Check

1. A planner-side split-support preparation rule is cleaner than retuning the scenario around the current reactive behavior. The authored baseline already proves the topology is lawful for two agents; the failure is that one profile repeatedly leaves or returns without enough complementary support to survive the trip cleanly.
2. The fix should stay concrete and local: prefer preparation using planner-visible current-place support and believed destination support, rather than a global scalar “survival safety” heuristic. This keeps the behavior explainable in decision traces and aligned with `FOUNDATIONS.md`.
3. Separating the planner fix from the golden-harness `effective_place` contradiction kept the abstraction boundary honest. `S117CONMAIOBS-013` owns behavior; `S117CONMAIOBS-014` restored the ignored survival golden as a lawful oracle.

## Verification Layers

1. Candidate/selection behavior changes at the intended planner boundary -> focused `worldwake-ai` unit/runtime coverage around self-care ranking/selection for split-support travel
2. The authored baseline acute/maintenance contradiction is removed at the behavior layer -> focused planner/runtime proof plus observer baseline rerun in this ticket, followed by `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
3. The observer anomaly surface stays honest rather than tuned away -> `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
4. No omniscient or snapshot-illegal carrier is introduced -> focused planner-boundary proof against `docs/planner-contracts.md` plus normal `worldwake-ai` crate verification

## What to Change

### 1. Add a bounded planner-side split-support preparation path

Teach the self-care planner to recognize when the actor is about to pursue a remote self-care goal at a destination that lacks complementary local support the actor can still satisfy where it stands. The concrete contract should stay belief-local and target the baseline failure mode:

- current place has planner-visible support for a complementary survival need (for example water at camp before traveling to food-only fields)
- believed destination for the selected self-care goal lacks that complementary support
- the complementary need is already high enough that leaving unprepared is likely to create a critical-band trip

The planner should then prefer a bounded preparation step (consume locally owned stock and/or acquire local support that can be carried) before or as part of committing to the remote split-support trip.

### 2. Prove the behavior at the planner boundary

Add focused `worldwake-ai` coverage for the chosen contract using the narrowest truthful boundary. The proof should show that under split-support survival pressure, the planner no longer refreshes the same remote apple-travel branch while ignoring locally available complementary water support that it can still lawfully prepare before leaving.

### 3. Revalidate the authored baseline and observer handoff

Rerun the observer baseline report, the focused planner proof, and the exact ignored survival-baseline golden selector in this ticket. If the planner fix removes the split-support oscillation cleanly, update `S117CONMAIOBS-007` and archived `S117CONMAIOBS-011.md` closeout references factually.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/tests/golden_survival_baseline.rs` (modify if the proving contract needs focused baseline assertions beyond the current ignored selector)
- `tickets/S117CONMAIOBS-014.md` (new proof-surface dependency; created during reassessment)
- `tickets/S117CONMAIOBS-007.md` (modify if baseline blocker ownership changes after the fix)
- `archive/tickets/S117CONMAIOBS-011.md` (modify on closeout if this ticket resolves the owning contradiction)

## Out of Scope

- Weakening or suppressing `ACUTE_NEED_SPIKE` / `MAINTENANCE_STARVATION`
- Retuning `survival-baseline.ron` to hide the planner failure
- Global omniscient safety scoring or new observer-only heuristics
- Rewriting the survival golden harness unless a separate traceability blocker remains after the planner behavior is fixed

## Acceptance Criteria

### Tests That Must Pass

1. Focused `worldwake-ai` proof for the new split-support preparation behavior
2. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
3. `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The fix uses only planner-visible local state, carried inventory, believed destination support, and existing travel knowledge; no omniscient future-safety carrier is introduced.
2. On the authored healthy baseline, the split-support oscillation that currently produces the known acute and maintenance windows is removed or reduced enough that the baseline contract is again honestly satisfiable.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` or nearby focused planner unit coverage — prove the bounded split-support preparation contract at the real selection boundary.
2. `crates/worldwake-ai/tests/golden_survival_baseline.rs` — use the exact ignored baseline selector as the behavior-level proof.

### Commands

1. Focused `cargo test -p worldwake-ai <exact focused selector>`
2. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
3. `cargo test -p worldwake-ai`
4. `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
