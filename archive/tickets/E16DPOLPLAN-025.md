# E16DPOLPLAN-025: Deferred ProgressBarrier in GOAP search — prefer GoalSatisfied across expansion levels

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — search.rs
**Deps**: E16DPOLPLAN-024

## Problem

The GOAP search in `search_plan()` (search.rs:103-205) greedily returns the first terminal successor found during node expansion. When a ProgressBarrier terminal (e.g., 1-step DeclareSupport) is found at root expansion alongside mid-plan candidates (Bribe, Threaten), the search immediately returns the ProgressBarrier without exploring whether a multi-step GoalSatisfied plan exists.

This prevents the planner from discovering that Bribe+DeclareSupport produces GoalSatisfied (winning coalition) even though a solo DeclareSupport only produces ProgressBarrier (declare and hope). The planner should prefer plans that achieve the goal over plans that merely make progress.

The existing ConsumeOwnedCommodity special-case (search.rs:176-189) already prefers GoalSatisfied over ProgressBarrier at the same expansion level. This ticket generalizes that principle: **GoalSatisfied is always preferred over ProgressBarrier, even across expansion levels.**

## Assumption Reassessment (2026-03-18)

1. `search_plan` returns immediately when `terminal_successors` is non-empty (search.rs:172-191) — confirmed.
2. ConsumeOwnedCommodity has a special sort preferring GoalSatisfied over ProgressBarrier at same level (search.rs:176-189) — confirmed.
3. CombatCommitment is a third terminal kind alongside GoalSatisfied and ProgressBarrier — confirmed (search.rs:619-620).
4. `PlanSearchResult` has variants: `Found`, `BudgetExhausted`, `FrontierExhausted`, `Unsupported` — confirmed (search.rs:80-100).
5. Budget limits search with `max_node_expansions` and `max_plan_depth` — confirmed.

## Architecture Check

1. **Principle 18 (Resource-Bounded Practical Reasoning)**: The planner still operates within its budget. ProgressBarrier deferral doesn't add unbounded computation — the search is still bounded by `max_node_expansions`. It just continues searching within the existing budget instead of short-circuiting at the first barrier.
2. **Principle 19 (Intentions Are Revisable Commitments)**: ProgressBarrier plans represent "commit and replan" — a fallback when full goal satisfaction can't be determined. GoalSatisfied plans represent "the goal is achievable." Preferring the latter is a direct application of this principle.
3. **Generalization over special-casing**: The current ConsumeOwnedCommodity sort is a special case of a general principle. This ticket replaces it with a general mechanism that works for all goal kinds, eliminating goal-specific branching in the search loop.
4. **CombatCommitment preserved**: Attack/Defend terminals are returned immediately (no deferral), preserving the existing combat commitment semantics.
5. No backwards-compatibility shims.

## What to Change

### 1. Modify terminal successor handling in `search_plan` (search.rs)

Replace the current terminal handling block (lines 172-191) with deferred ProgressBarrier logic:

```rust
// Track the best ProgressBarrier plan as a fallback.
let mut best_barrier: Option<PlannedPlan> = None;

while let Some(node) = frontier.pop().map(FrontierEntry::into_node) {
    if goal.key.kind.is_satisfied(&node.state) {
        return PlanSearchResult::Found(PlannedPlan::new(
            goal.key,
            node.steps,
            PlanTerminalKind::GoalSatisfied,
        ));
    }
    // ... budget check, expansion ...

    let mut terminal_successors = Vec::new();
    let mut successors = Vec::new();
    // ... candidate evaluation (unchanged) ...

    if !terminal_successors.is_empty() {
        // Partition: GoalSatisfied / CombatCommitment are returned immediately.
        // ProgressBarrier is stored as a fallback.
        terminal_successors.sort_by(|left, right| compare_search_nodes(&left.1, &right.1));

        for (terminal_kind, successor) in terminal_successors {
            match terminal_kind {
                PlanTerminalKind::GoalSatisfied | PlanTerminalKind::CombatCommitment => {
                    return PlanSearchResult::Found(PlannedPlan::new(
                        goal.key,
                        successor.steps,
                        terminal_kind,
                    ));
                }
                PlanTerminalKind::ProgressBarrier => {
                    if best_barrier.is_none() {
                        best_barrier = Some(PlannedPlan::new(
                            goal.key,
                            successor.steps,
                            terminal_kind,
                        ));
                    }
                    // Continue searching — don't return
                }
            }
        }
    }

    // ... successor truncation and frontier push (unchanged) ...
}

// Frontier exhausted. Return best barrier if available, otherwise report exhaustion.
if let Some(barrier_plan) = best_barrier {
    return PlanSearchResult::Found(barrier_plan);
}
PlanSearchResult::FrontierExhausted { expansions_used: expansions }
```

### 2. Remove the ConsumeOwnedCommodity special-case sort (search.rs:176-189)

The ConsumeOwnedCommodity GoalSatisfied-over-ProgressBarrier preference is subsumed by the general deferred barrier mechanism. Remove the `if matches!(goal.key.kind, GoalKind::ConsumeOwnedCommodity { .. })` branch. All goals now benefit from the same preference.

### 3. Update `BudgetExhausted` return to also check for deferred barriers

At the budget exhaustion check (search.rs:132-133), if a ProgressBarrier has been found, return it instead of BudgetExhausted:

```rust
if expansions >= budget.max_node_expansions {
    if let Some(barrier_plan) = best_barrier {
        return PlanSearchResult::Found(barrier_plan);
    }
    return PlanSearchResult::BudgetExhausted { expansions_used: expansions };
}
```

## Files to Touch

- `crates/worldwake-ai/src/search.rs` (modify — terminal handling in `search_plan`)

## Out of Scope

- Changes to `is_satisfied`, `is_progress_barrier`, or goal_model.rs (E16DPOLPLAN-024)
- Changes to PlanningSnapshot/PlanningState (E16DPOLPLAN-023)
- Changes to belief view (E16DPOLPLAN-022)
- Integration tests (E16DPOLPLAN-006)

## Acceptance Criteria

### Tests That Must Pass

1. **Uncontested ClaimOffice**: 1-step DeclareSupport GoalSatisfied plan found (regression: same outcome as before but with GoalSatisfied terminal kind instead of ProgressBarrier)
2. **Contested ClaimOffice with bribable target**: Multi-step plan found containing Bribe + DeclareSupport with GoalSatisfied terminal (NEW behavior)
3. **Contested ClaimOffice without viable coalition**: 1-step DeclareSupport ProgressBarrier plan found as fallback (regression: fallback preserved)
4. **ConsumeOwnedCommodity regression**: Eat (GoalSatisfied) still preferred over pick_up (ProgressBarrier) when both available at same expansion
5. **ConsumeOwnedCommodity regression**: Pick_up ProgressBarrier still found when eat is not available
6. **CombatCommitment**: Attack/Defend still return immediately (no deferral)
7. **Budget exhaustion with deferred barrier**: If budget runs out but a ProgressBarrier was found, it is returned instead of BudgetExhausted
8. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. GoalSatisfied and CombatCommitment terminals are always returned immediately (no deferral)
2. ProgressBarrier terminals are stored as fallbacks, not returned immediately
3. The best (lowest total_estimated_ticks) ProgressBarrier is kept as fallback
4. If no GoalSatisfied/CombatCommitment is found within budget, the fallback ProgressBarrier is returned
5. If no terminal is found at all, FrontierExhausted/BudgetExhausted is returned (unchanged)
6. Search is still bounded by `max_node_expansions` — deferral does not create unbounded work

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search.rs` — test deferred barrier: ProgressBarrier found at depth 1, GoalSatisfied found at depth 2, GoalSatisfied plan returned
2. `crates/worldwake-ai/src/search.rs` — test fallback: ProgressBarrier found, no GoalSatisfied possible, ProgressBarrier returned after frontier exhaustion
3. `crates/worldwake-ai/src/search.rs` — test budget exhaustion with barrier: barrier found, budget runs out, barrier returned
4. `crates/worldwake-ai/src/search.rs` — regression: existing ConsumeOwnedCommodity tests pass without the removed special-case
5. `crates/worldwake-ai/src/search.rs` — regression: CombatCommitment tests unchanged

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-18
- **What changed**: Modified `search_plan()` in `crates/worldwake-ai/src/search.rs` to defer ProgressBarrier terminals instead of returning them immediately. GoalSatisfied and CombatCommitment terminals still return immediately. Deferred barriers are returned as fallback on frontier or budget exhaustion. Removed the ConsumeOwnedCommodity special-case sort (subsumed by general mechanism).
- **Deviations from plan**: None. All three changes implemented as specified.
- **Verification**: 431 unit tests + 149 golden tests pass (0 failures). `cargo clippy --workspace` clean. 3 new tests added covering: GoalSatisfied preference across expansion levels, fallback after frontier exhaustion, fallback on budget exhaustion.
