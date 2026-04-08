# S74 — Intention Commitment Under Needs Fluctuation

**Status**: COMPLETED

## Motivation

The current planning path has a structural coupling between metabolism rate and planning frequency. Metabolism changes homeostatic needs by 1–2 permille every tick. The observation snapshot comparison (`observation_snapshot_changed`) detects this as a NEEDS dirty bit on every tick. When the dirty set is non-empty, planning triggers. When goal ranking shifts (which it frequently does, since need levels directly drive goal priority weights), the plan continuation fast path fails and full GOAP search runs.

This creates ~10,000 full planning passes per agent over a 10,000-tick soak — roughly one per tick — even when the agent has a valid, progressing plan and the need change is trivially small. Each full planning pass costs 2–20ms depending on snapshot complexity, making this the dominant CPU cost in long-running simulations.

### Why Tuning Cannot Fix This

- **Band-based needs detection** (comparing at 5–50 permille granularity instead of exact equality): breaks `golden_merchant_selling` at ANY band size. The test depends on exact-tick responsiveness to need-level changes for SellCommodity goal timing.
- **Relaxed plan continuation** (continue plan when goal is anywhere in ranked candidates): causes 88% regression on seed 4 because agents persist on suboptimal plans that lead to failure cascades.
- **Restricting continuation to top-2 candidates**: helps some seeds but regresses others. Seed-dependent because the ranking stability varies with agent behavior.

All three approaches are band-aids that trade correctness for performance. The root problem is architectural: the intention frame has commitment semantics (Principle 21 — "Agents need commitments so they do not thrash between options every tick"), but the planning path has no equivalent commitment mechanism. Active-action agents benefit from `frame_switch_margin` (they only switch if a challenger exceeds the margin), but idle agents planning their next step have no commitment inertia at all.

## Design

### Planning-Phase Switch Margin

Extend the existing `switch_margin` concept — which currently applies only during active-action interrupt evaluation — to also apply during the planning path's snapshot-continuation check.

**Current behavior** (planning path, `try_continue_snapshot_plan`):
1. If dirty set is snapshot-only AND agent has a current plan:
2. If dirty set is needs-only (`is_needs_only()`): check if current goal is in the **top 2** ranked candidates → continue plan (existing heuristic from soak-seed-perf experiments)
3. Otherwise: check if current goal is ranked #1 AND matches the plan's opportunity → continue plan
4. If neither condition holds → fall through to full GOAP search

The top-2 heuristic reduces replanning frequency but is unprincipled — it was tuned empirically and has no cognitive meaning. This spec replaces it with a margin-based mechanism.

**Proposed behavior**:
1. If dirty set is snapshot-only AND agent has a current plan:
2. Check if current goal is ranked #1 AND matches the plan's opportunity → continue (unchanged)
3. **NEW**: If current goal is NOT #1, apply the planning switch margin check:
   - **Cross-class bypass**: If the top-ranked goal has a strictly higher `GoalPriorityClass` than the current goal (e.g., Critical vs. Medium), always fall through to full GOAP search — a priority class upgrade is always compelling enough to abandon a plan
   - **Same-class margin**: If both goals share the same `GoalPriorityClass`, check if the top-ranked goal's `motive_score` exceeds the current goal's `motive_score` by more than `planning_switch_margin`. If the margin is NOT exceeded → continue plan. If exceeded → fall through to full GOAP search
4. If the current goal is NOT present in the ranked candidates at all → fall through (goal became infeasible)
5. In all cases, if the plan's next step fails revalidation → fall through (the plan is invalid regardless of ranking)

### Planning Switch Margin Parameter

Add `planning_switch_margin: Permille` to `CognitiveProfile`:
- **Default**: `Permille(150)` — a 15% priority advantage is needed to dislodge a committed plan
- **Semantics**: within the same `GoalPriorityClass`, the top-ranked goal must have `motive_score >= current_goal_motive_score + planning_switch_margin` to trigger full replanning. If the top-ranked goal has a strictly higher `GoalPriorityClass`, the margin is bypassed (always replan)
- **Interaction with existing `switch_margin`**: `planning_switch_margin` applies during the planning path (no active action). `switch_margin` applies during the active-action interrupt path. They serve the same purpose in different phases.

### Priority Comparison

The comparison uses the existing `RankedGoal` fields: `priority_class: GoalPriorityClass` and `motive_score: u32`. No new ranking logic is needed — the comparison reuses the ranking infrastructure that `rank_candidates` already produces.

**Cross-class semantics**: `GoalPriorityClass` is an ordered enum (Background < Low < Medium < High < Critical). When the top-ranked goal's `priority_class` strictly exceeds the current goal's `priority_class`, the margin is bypassed and full replanning triggers immediately. This prevents a commitment margin from blocking genuinely urgent goals (e.g., a Critical survival goal should always dislodge a Medium economic goal).

**Same-class semantics**: When both goals share the same `priority_class`, the margin comparison applies to `motive_score`: `top.motive_score >= current.motive_score + planning_switch_margin`. The `motive_score` is a post-discount priority value already computed by `rank_candidates`, so no additional scoring is needed.

The comparison requires finding the current goal in the ranked candidates list. If the current goal is NOT present in the ranked candidates at all (it was completely filtered or blocked), the plan is abandoned — this preserves responsiveness to goals that become entirely infeasible.

### Interaction with Non-Needs Snapshot Changes

The planning switch margin applies ONLY when the dirty set is snapshot-only. If structural bits (NO_PLAN, PLAN_FINISHED, REPLAN_SIGNAL, QUEUE_TRANSITION, BLOCKER_CLEANUP, QUEUE_PATIENCE) or frame bits (FRAME_BLOCKAGE, FRAME_PATIENCE, ASSUMPTION_FAILED) are set, the current full-planning behavior is unchanged. This ensures responsiveness to meaningful state changes while reducing churn from continuous need fluctuation.

### Why This Aligns With Principle 21

Principle 21 states: "Agents need commitments so they do not thrash between options every tick. But commitments are never rails."

The planning switch margin IS the commitment mechanism for the planning path:
- **Commitment**: agents don't abandon plans over trivial ranking changes
- **Not rails**: agents DO abandon plans when a genuinely higher-priority goal exceeds the margin
- **Assumption monitoring**: structural triggers (assumption failures, replan signals) bypass the margin entirely, preserving full responsiveness to broken assumptions

## FND-01 Section H Analysis

### H.1 Information-Path Analysis

No information paths change. All observations, beliefs, and perceptions remain exact-tick and exact-value. The change affects only the planner's decision of whether to RE-SEARCH a plan, not what it observes or believes. The agent's belief store still reflects exact need values. Goal candidates are still generated from exact beliefs. Ranking still uses exact priorities.

### H.2 Positive-Feedback Analysis

**Identified loop (broken by this spec)**: Need fluctuation → NEEDS dirty → goal ranking shift → full GOAP search → plan abandoned → agent idles → need increases more → more planning churn.

The planning switch margin breaks this loop by preventing trivial ranking shifts from triggering expensive replanning.

**No new loops introduced**: The margin is a static per-agent parameter, not a function of planning outcomes.

### H.3 Concrete Dampeners

The dampener is `planning_switch_margin: Permille`, a concrete per-agent parameter on `CognitiveProfile`. It is not a naked clamp — it expresses the agent's commitment inertia, a cognitively meaningful property (some agents are more stubborn, others more reactive). This parallels the existing `switch_margin` for active actions.

### H.4 Stored State vs. Derived Read-Model

| Item | Category |
|------|----------|
| `HomeostaticNeeds` component | Authoritative stored state (unchanged) |
| `observation_snapshot_changed` NEEDS dirty bit | Derived computation (unchanged — still exact equality) |
| `planning_switch_margin` | Authoritative stored state (CognitiveProfile parameter) |
| RankedGoal `priority_class` + `motive_score` | Derived computation (already computed by `rank_candidates`) |
| Plan continuation decision | Derived computation (now uses margin comparison) |

## Implementation

**Crate: worldwake-core** (cognitive_profile.rs)

1. Add `planning_switch_margin: Permille` to `CognitiveProfile` (default: `Permille(150)`).

**Crate: worldwake-ai** (agent_tick/planning.rs)

2. Extend `try_continue_snapshot_plan` signature to accept `&CognitiveProfile` (or `planning_switch_margin: Permille`). Replace the `is_needs_only()` top-2 heuristic with the margin-based comparison: cross-class bypass on `GoalPriorityClass`, same-class margin check on `motive_score`.
3. Modify the traced variant in `plan_and_validate_next_step_traced` (inline logic at ~lines 786-836) consistently with the same margin-based comparison.
4. Remove the `is_needs_only()` top-2 heuristic from `try_continue_snapshot_plan` (`planning.rs`), which this spec supersedes with a principled mechanism.

**Crate: worldwake-ai** (dirty_set.rs)

5. The `is_needs_only()` method can be removed or retained for diagnostics. It is no longer used in the planning decision path.

**Golden test updates:**

6. `golden_merchant_selling` (`loose_home_stock_is_staged_before_sell_goal_settles`): this test depends on the agent switching goals when needs shift priorities. With `planning_switch_margin = 150`, the test may need to either:
   - Set a lower `planning_switch_margin` for the test agent (e.g., 50) to preserve rapid switching behavior
   - OR adjust the scenario's need levels so the priority shift exceeds 150, triggering the switch naturally
   
   The choice depends on what the test is proving: if it proves that agents CAN switch goals under need pressure, adjusting the margin is correct. If it proves timing-specific behavior, the scenario needs stronger need pressure.

## Principle Alignment

| Principle | Alignment |
|-----------|-----------|
| P12 (Performance May Compress Computation) | The margin compresses planning frequency, not world causality. The agent's beliefs, observations, and goal candidates are unchanged. Only the replanning decision is throttled. |
| P20 (Resource-Bounded Reasoning) | The margin IS resource-bounded reasoning — the agent doesn't reconsider its entire plan over trivially small priority changes, analogous to bounded rationality in decision theory |
| P21 (Intentions Are Revisable Commitments) | Directly implements the commitment semantics Principle 21 calls for: "stable intentions held under assumptions" with margin-gated revision |
| P22 (Agent Diversity) | The margin is per-agent via CognitiveProfile, allowing reactive agents (low margin) and committed agents (high margin) |
| P26 (Systems Through State) | No cross-system coupling; the margin is read from a component |

## Validation

1. All existing golden tests pass; no margin-specific scenario override was needed in the live post-S74 validation.
2. Soak telemetry on seeds 0–4 shows the intended reduction in late-window replanning churn, but wall-clock comparisons must use the refreshed per-seed baseline set in `campaigns/soak-seed-perf/seed-baselines.tsv` rather than the earlier mixed legacy entries.
3. Agents still switch goals when a genuinely higher-priority goal emerges (margin is not infinite)
4. Agents with `planning_switch_margin = 0` trigger full replanning on every ranking shift within the same priority class (no commitment inertia), confirming the margin is a pure additive mechanism
5. Decision traces include the margin comparison result for debuggability (Principle 29)

## Scenario Profile Contract

New field `planning_switch_margin: Permille` on `CognitiveProfile`:
- **Universal**: yes (every agent has a CognitiveProfile)
- **Default**: `Permille(150)`
- **Scenario-definable**: yes (already part of `AgentDef` via CognitiveProfile)

## Outcome

Completed on 2026-04-08.

- Added `planning_switch_margin: Permille` to `CognitiveProfile` with a default of `Permille(150)` and made the field part of the live per-agent planning substrate.
- Replaced the old planning-path top-2 continuation heuristic with a margin-based same-class commitment check and aligned traced/untraced planning paths plus decision-trace provenance around that contract.
- Fixed the exposed same-goal merchant continuity regression by preserving the committed same-goal branch through in-progress replanning at the planner-visible boundary.
- Completed the validation and campaign handoff truthfully: no margin-specific golden override was required, the generated golden docs were refreshed, and the soak baseline/spec validation surface was corrected to match the live post-S74 evidence.

## Deviations

- The original spec anticipated that a timing-sensitive merchant golden might need either a lower per-agent margin or stronger need pressure. Live implementation and validation showed no scenario override was needed.
- The original soak-validation claim of seed-wide wall-clock improvement did not survive reassessment. The shipped handoff keeps the validated behavioral claim on reduced late-window replanning churn and refreshes the campaign baseline surface instead of forcing a speculative engine optimization.

## Verification Result

- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-systems report_found_`
- Passed `cargo test -p worldwake-ai snapshot_filter_includes_records_for_report_found`
- Passed `cargo test -p worldwake-ai --test golden_expectation`
- Passed `cargo test -p worldwake-ai planning`
- Passed `cargo test -p worldwake-ai --test golden_ai_decisions golden_goal_switching_during_multi_leg_travel`
- Passed `cargo test -p worldwake-ai -- golden_`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0`
- Passed `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 1`
- Passed `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 2`
- Passed `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 3`
- Passed repeated `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 3` variance check
- Passed `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 4`
