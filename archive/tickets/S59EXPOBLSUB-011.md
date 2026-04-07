# S59EXPOBLSUB-011: Search candidate generation

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — candidate generation and ranking in worldwake-ai
**Deps**: S59EXPOBLSUB-004, S59EXPOBLSUB-005, S59EXPOBLSUB-006, S59EXPOBLSUB-010

## Problem

Overdue expectations should drive agents to search for missing persons and report their absence. On this branch, that requires both candidate generation and minimal ranking support: emitted `SearchForMissing` / `ReportMissing` goals must survive the existing zero-motive filter in `rank_candidates()` before the behavior is live.

## Assumption Reassessment (2026-04-06)

1. `generate_candidates` at `crates/worldwake-ai/src/candidate_generation.rs:187-206` calls `generate_candidates_with_travel_horizon()` which calls individual `emit_*_candidates()` functions (lines 246-258).
2. New function: `emit_search_candidates()` called from `generate_candidates_with_travel_horizon()`.
3. `GoalBeliefView::expectation_store()` (from ticket 004) provides access to the agent's overdue expectations.
4. `GoalKind::SearchForMissing` and `GoalKind::ReportMissing` (from ticket 005) are the emitted goal variants.
5. Priority scaling uses overdue duration (current_tick - deadline_tick) and basis type (DutyAssignment > RoutineReturn > SocialPromise).
6. `GroundedGoal` is the return type with priority weight — pattern from existing `emit_*_candidates()` functions.
7. `BlockedIntentMemory` parameter filters recently-failed goals to prevent thrashing.
8. `S59EXPOBLSUB-010` now lands the real route-aware `escort_to_safety` action, so later contextual `EscortToSafety` goal emission can rely on a live authoritative action boundary instead of a reserved planner-only symbol.
9. Live mismatch: `crates/worldwake-ai/src/ranking.rs:647-649` still assigns `SearchForMissing`, `ReportMissing`, and `EscortToSafety` zero motive. Candidate generation alone would emit these goals and then immediately drop them during ranking.
10. Honest scope correction: this ticket must own the minimal ranking support needed for overdue search/report goals to become behaviorally selectable. Contextual `EscortToSafety` emission remains separate until the candidate layer proves it is part of the same live slice.

## Architecture Check

1. Follows the existing `emit_*_candidates()` pattern for generation, plus the existing `goal_motive_score()` / priority-class path for ranking. No new infrastructure.
2. Reads only from GoalBeliefView (belief state, not world state) — satisfies P14.
3. No backward compatibility shims.

## Verification Layers

1. Overdue expectation emits SearchForMissing goal → decision trace (candidate list contains the goal)
2. Overdue expectation emits ReportMissing goal → decision trace
3. Active (non-overdue) expectations do NOT emit goals → decision trace (absence check)
4. Priority scales with overdue duration → focused unit test on priority computation
5. Blocked intent memory suppresses recently-failed search goals → focused unit test
6. Emitted search/report goals survive ranking with non-zero motive and appear in the ranked candidate set.
7. Minimal mixed-layer AI ticket: prove both generation and ranking, not just raw emission.

## What to Change

### 1. Create emit_search_candidates function

In `crates/worldwake-ai/src/candidate_generation.rs`, add:

```rust
fn emit_search_candidates(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    current_tick: Tick,
    blocked: &BlockedIntentMemory,
    candidates: &mut Vec<GroundedGoal>,
) {
    let store = match view.expectation_store(agent) {
        Some(s) => s,
        None => return,
    };

    let last_seen = view.last_seen_memory(agent);

    for record in store.records.values() {
        if record.state != ExpectationState::Overdue {
            continue;
        }

        // Compute priority from overdue duration and basis type
        let overdue_ticks = current_tick.0.saturating_sub(
            record.deadline_tick.0 + record.grace_ticks
        );
        let basis_weight = match &record.basis {
            ExpectationBasis::DutyAssignment { .. } => 3,
            ExpectationBasis::DeliveryCommitment { .. } => 2,
            ExpectationBasis::EscortObligation { .. } => 3,
            ExpectationBasis::RoutineReturn => 1,
            ExpectationBasis::SocialPromise => 1,
        };

        // Emit SearchForMissing
        let last_seen_place = last_seen.as_ref()
            .and_then(|m| m.records.get(&record.subject))
            .map(|r| r.place);

        let search_goal = GoalKind::SearchForMissing {
            subject: record.subject,
            last_seen: last_seen_place,
        };
        // ... push to candidates with priority

        // Emit ReportMissing
        let report_goal = GoalKind::ReportMissing {
            subject: record.subject,
            to_office: None, // office discovery happens during planning
        };
        // ... push to candidates with priority
    }
}
```

### 2. Call from generate_candidates_with_travel_horizon

Add `emit_search_candidates(view, agent, current_tick, blocked, &mut candidates)` call in `generate_candidates_with_travel_horizon()` alongside existing emit functions.

### 3. Add minimal motive support in ranking

In `crates/worldwake-ai/src/ranking.rs`, replace the current zero-motive fallback for:

- `GoalKind::SearchForMissing { .. }`
- `GoalKind::ReportMissing { .. }`

with a minimal non-zero motive path derived from the same overdue expectation / last-seen basis the candidate layer exposes. The goal of this ticket is not deep policy tuning; it is making overdue missing-person response behaviorally live on the existing ranking pipeline. `EscortToSafety` remains out of scope unless reassessment during implementation proves it is required for the same live slice.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add function + call site)
- `crates/worldwake-ai/src/ranking.rs` (modify — minimal motive support for search/report goals)

## Out of Scope

- EscortToSafety goal emission — this is triggered by search_place finding a wounded person, not by overdue expectations directly. The escort goal is generated contextually after search, not in candidate generation.
- Priority tuning — initial weights are starting points, tunable later
- Integration with UtilityProfile or pressure system — uses direct priority scoring
- Broad ranking-policy redesign — this ticket only adds the minimum motive support needed so newly emitted overdue search/report goals are not discarded as zero-motive.

## Acceptance Criteria

### Tests That Must Pass

1. Agent with overdue DutyAssignment expectation emits SearchForMissing goal
2. Agent with overdue SocialPromise emits SearchForMissing with lower priority than DutyAssignment
3. Agent with active (not overdue) expectation emits no search goals
4. Agent with no ExpectationStore emits no search goals
5. ReportMissing goal emitted alongside SearchForMissing for overdue records
6. Blocked intent memory suppresses recently-failed search goals
7. LastSeenMemory provides last_seen_place hint to SearchForMissing goal
8. Ranked candidates retain non-zero-motive SearchForMissing and ReportMissing goals when their overdue expectations are present
9. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Only overdue expectations (not active or resolved) produce search goals
2. Priority scales monotonically with overdue duration
3. Duty-based expectations outrank social expectations
4. Reads only from GoalBeliefView (P14 — belief state, not world state)
5. Newly emitted overdue search/report goals are not filtered out by the ranking zero-motive path

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — focused tests for `emit_search_candidates` with mock GoalBeliefView
2. `crates/worldwake-ai/src/ranking.rs` or existing ranked-candidate tests — focused proof that overdue search/report goals survive ranking with non-zero motive

### Commands

1. `cargo test -p worldwake-ai candidate_generation`
2. `cargo test -p worldwake-ai ranking`
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

- Completed: 2026-04-07
- What changed:
  - Added overdue-expectation-driven `SearchForMissing` / `ReportMissing` candidate emission in `crates/worldwake-ai/src/candidate_generation.rs`.
  - Added focused candidate-generation coverage for overdue, active, blocked, and duplicate-report-suppressed missing-response cases.
  - Replaced the zero-motive ranking fallback for `SearchForMissing` / `ReportMissing` with expectation-backed motive scoring in `crates/worldwake-ai/src/ranking.rs`.
  - Added focused ranking coverage proving overdue missing-response goals survive zero-motive filtering and preserve basis/overdue ordering.
- Deviations from original plan:
  - Scope was corrected during reassessment to include minimal ranking support, because candidate generation alone would have emitted goals that `rank_candidates()` immediately dropped as zero motive.
  - The implemented candidate layer deduplicates overdue records by subject and selects the strongest overdue expectation for emission instead of emitting one goal per raw record.
- Verification results:
  - Passed `cargo test -p worldwake-ai candidate_generation`
  - Passed `cargo test -p worldwake-ai ranking`
  - Passed `cargo test -p worldwake-ai`
