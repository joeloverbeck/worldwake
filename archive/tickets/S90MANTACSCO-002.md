# S90MANTACSCO-002: Barrier-required explore classification and fail-fast

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — planner search internals (`worldwake-ai`)
**Deps**: S90MANTACSCO-001

## Problem

The live branch has two different meanings collapsed into `TacticalSubGoal::Explore`:
1. exploration that is supposed to install a tactical travel barrier for supported goal families like `AcquireCommodity` and `SearchForMissing`
2. generic exploratory fallback that is allowed to continue without a tactical barrier for other goal families

Because those two meanings share one strategic variant, `search_plan` cannot fail fast on "missing tactical goal" without also breaking lawful generic exploration and lawful no-barrier strategic paths. The ticket needs to separate those meanings first, then attach fail-fast only to the barrier-required slice.

## Assumption Reassessment (2026-04-11)

1. `TacticalGoal::from_strategic_step` call confirmed at `mod.rs:267-270`. After 001, the signature includes `snapshot`.
2. `PlanSearchResult::FrontierExhausted { expansions_used: u16 }` confirmed at `mod.rs:172`. Already has the `expansions_used` field — no variant changes needed.
3. Shared boundary: `crates/worldwake-ai/src/search/strategic.rs::TacticalSubGoal` plus `crates/worldwake-ai/src/search/mod.rs::TacticalGoal::from_strategic_step`.
4. Reassessment correction: the original blanket fail-fast is not lawful on the live branch. `search_accuse_satisfy_goal_does_not_install_travel_barrier` already proves a non-empty strategic `SatisfyGoal` step may lawfully keep `tactical_goal = None`, and broader `cargo test -p worldwake-ai` showed many existing search contracts still rely on generic no-barrier `Explore` fallback.
5. Live architectural gap: the strategic layer currently does not distinguish barrier-required exploration from generic fallback exploration. That missing classification is the actual substrate 002 must land before any fail-fast becomes safe.
6. Only `AcquireCommodity` and `SearchForMissing` currently lawfully use exploration as a tactical barrier target. The existing `exploration_supports_tactical_barrier` function in `search/mod.rs` already encodes that boundary downstream; 002 should move the distinction upstream into the strategic step shape instead of keeping it implicit.

## Architecture Check

1. Splitting strategic exploration into barrier-required vs generic fallback is cleaner than adding special-case guards in `search_plan` against an overloaded `Explore` meaning. It makes the planner output itself say whether a tactical barrier is required.
2. No backwards-compatibility shims. The old overloaded `Explore` meaning is replaced by two explicit strategic variants and one bounded fail-fast check.

## Verification Layers

1. Strategic planner emits the barrier-required exploration variant only for supported goal families → focused strategic/search unit tests
2. Missing tactical goal only fail-fasts for the barrier-required exploration variant → focused unit test on the fail-fast boundary helper/search entry
3. Generic exploration fallback and lawful no-barrier strategic paths remain unaffected → focused search tests plus `cargo test -p worldwake-ai`
4. Single-layer ticket: strategic classification and fail-fast are planner-internal, no cross-system interaction

## What to Change

### 1. Split strategic exploration into explicit barrier-required vs generic fallback variants

**Files**: `crates/worldwake-ai/src/search/strategic.rs`, `crates/worldwake-ai/src/search/mod.rs`

Replace the overloaded `TacticalSubGoal::Explore` meaning with two explicit strategic variants:
- one variant for exploration that requires a tactical barrier
- one variant for generic fallback exploration that may continue without a tactical barrier

Update `strategic::exploration_plan()` so supported goal families (`AcquireCommodity`, `SearchForMissing`) emit the barrier-required variant and other families emit the generic fallback variant.

Update `TacticalGoal::from_strategic_step()` so only the barrier-required variant attempts to produce `TacticalGoal::Explore`; the generic fallback variant must continue to produce `None`.

### 2. Add fail-fast guard after `from_strategic_step` call

**File**: `crates/worldwake-ai/src/search/mod.rs`

After the `TacticalGoal::from_strategic_step()` call (line 267-270, post-001), add a guard keyed to the new barrier-required exploration variant:

```rust
if strategic_plan
    .as_ref()
    .and_then(|plan| plan.steps.first())
    .is_some_and(|step| matches!(step.sub_goal, strategic::TacticalSubGoal::<BARRIER_VARIANT>))
    && tactical_goal.is_none()
{
    return PlanSearchResult::FrontierExhausted {
        expansions_used: 0,
    };
}
```

The generic fallback exploration variant and lawful no-barrier strategic paths such as `Accuse` must remain unaffected.

### 3. Add focused proof for the new classification and fail-fast boundary

**Files**: `crates/worldwake-ai/src/search/strategic.rs`, `crates/worldwake-ai/src/search/tests.rs`

Add focused tests proving:
1. supported exploration-barrier goals emit the barrier-required strategic variant
2. generic exploration fallback goals emit the generic fallback variant
3. the fail-fast helper / search boundary triggers only for the barrier-required variant when no tactical goal is produced
4. generic exploration fallback and lawful no-barrier `Accuse` planning still succeed

## Files to Touch

- `crates/worldwake-ai/src/search/strategic.rs` (modify)
- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)

## Out of Scope

- Modifying `PlanSearchResult` variants (both already have `expansions_used`)
- Changing local goal behavior

## Acceptance Criteria

### Tests That Must Pass

1. `test_empty_beliefs_exploration_fallback_uses_barrier_required_variant_for_supported_goal`
2. `test_empty_beliefs_exploration_fallback_uses_generic_variant_for_unsupported_goal`
3. `search_fail_fast_when_barrier_required_explore_has_no_tactical_goal`
4. `search_generic_explore_without_tactical_goal_still_finds_plan`
5. `search_accuse_search_without_tactical_barrier_still_finds_plan`
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Barrier-required strategic exploration is explicit in the strategic step shape rather than inferred later from goal kind
2. Only barrier-required exploration fails fast when a tactical goal is missing
3. Generic exploration fallback and lawful no-barrier strategic plans remain searchable

## Outcome

Completion date: 2026-04-11

Implemented on the live branch by making the strategic exploration boundary explicit instead of relying on the old overloaded `Explore` meaning.

Delivered behavior:
1. `search/strategic.rs` now distinguishes `ExploreWithBarrier` from `ExploreFallback`
2. `strategic::exploration_plan()` emits `ExploreWithBarrier` only for the currently lawful barrier-owning families (`AcquireCommodity`, `SearchForMissing`)
3. `TacticalGoal::from_strategic_step()` only constructs `TacticalGoal::Explore` from `ExploreWithBarrier`; generic fallback exploration continues to produce `None`
4. `search_plan_with_trace_metadata()` now fail-fasts only when the first strategic step is `ExploreWithBarrier` and tactical-goal construction still returns `None`
5. The old downstream goal-kind inference helper was removed because the strategic step shape now carries the boundary directly

Ticket correction relative to the original D2 draft:
1. The original blanket fail-fast was not lawful on the live branch because it broke existing generic exploration fallback and lawful no-barrier strategic paths like `Accuse`
2. The landed change therefore broadened 002 slightly at the same planner boundary: classify strategic exploration first, then fail-fast only on the barrier-required slice

Verification completed:
1. `cargo test -p worldwake-ai -- test_empty_beliefs_exploration_fallback`
2. `cargo test -p worldwake-ai -- search_fail_fast_when_barrier_required_explore_has_no_tactical_goal`
3. `cargo test -p worldwake-ai -- search_generic_explore_without_tactical_goal_still_finds_plan`
4. `cargo test -p worldwake-ai -- search_accuse_search_without_tactical_barrier_still_finds_plan`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/strategic.rs` focused exploration-variant classification tests
2. `crates/worldwake-ai/src/search/tests.rs::search_fail_fast_when_barrier_required_explore_has_no_tactical_goal`
3. `crates/worldwake-ai/src/search/tests.rs::search_generic_explore_without_tactical_goal_still_finds_plan`
4. `crates/worldwake-ai/src/search/tests.rs::search_accuse_search_without_tactical_barrier_still_finds_plan`

### Commands

1. `cargo test -p worldwake-ai -- test_empty_beliefs_exploration_fallback`
2. `cargo test -p worldwake-ai -- search_fail_fast_when_barrier_required_explore_has_no_tactical_goal`
3. `cargo test -p worldwake-ai -- search_generic_explore_without_tactical_goal_still_finds_plan`
4. `cargo test -p worldwake-ai -- search_accuse_search_without_tactical_barrier_still_finds_plan`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`
