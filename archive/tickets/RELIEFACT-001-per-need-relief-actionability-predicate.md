# RELIEFACT-001: Extract per-need relief-actionability predicate from emit_exploration_candidates

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` candidate generation (refactor of `emit_exploration_candidates`)
**Deps**: archive/specs/S129-place-dirtiness-facility-wear.md, archive/tickets/S129CIREM-002-late-game-stuck-idle.md

## Problem

S129CIREM-002 added a dirtiness-specific branch in
`emit_exploration_candidates`
(`crates/worldwake-ai/src/candidate_generation.rs`, ~line 2813):

```rust
if need_id == HomeostaticNeedId::Dirtiness {
    let wash_access_known = !wash_access_opportunities(ctx).is_empty();
    if wash_access_known {
        continue;
    }
} else {
    let path_reliable = ctx.view.acquisition_exhaustion_count(ctx.agent, need_id)
        < profile.acquisition_failure_threshold;
    if any_local_need_relief(ctx.view, ctx.agent, ctx.place, matches_need)
        || (path_reliable && need_has_known_acquisition_path(ctx, matches_need))
    {
        continue;
    }
}
```

The dirtiness branch was added because dirtiness's relief is gated on
*clean wash basin* state, not on owning a commodity — so the generic
"any local relief or known acquisition path" predicate misclassified
"agent owns water" as "dirtiness path is actionable", which was wrong
and caused contested-scenario stalls. The fix is correct, but it
encodes the per-need relief substrate as an inline `if` rather than
a per-need predicate.

The architectural concern: agents have N needs, each with its own
relief substrate (Eat = own food, Drink = own water, Sleep = bed
in safe place + tiredness, Relieve = latrine OR wilderness with
penalty, Wash = clean basin with units). The predicate "is this
need's relief path actionable from current state?" should be answered
by composing per-need predicates registered with each need, not by
an inline branch that grows one arm per surprising substrate. As
S128 (sleep-quality) and any future needs-with-conditional-relief
land, the same special-casing pattern will be repeated. FND-3
(concrete state) and agent-symmetry across needs both push toward
declarative substrate over enumerated branches.

## Assumption Reassessment (2026-05-01)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. **Live `emit_exploration_candidates`** at
   `crates/worldwake-ai/src/candidate_generation.rs` (~line 2813):
   the dirtiness `if` branch was added by S129CIREM-002 as cited
   above. The default path uses `any_local_need_relief` plus
   `need_has_known_acquisition_path` against `matches_need`.
2. **Live per-need relief surfaces**:
   - `Eat` / `Drink`: relief is "consume owned commodity" via the
     existing `consume_owned_commodity` action; actionability is "do
     I own a relief commodity locally OR is there a known acquisition
     path to one?"
   - `Sleep`: relief is "sleep in a place" (S128); actionability is
     "is there a known sleep-quality place reachable?"
   - `Relieve`: relief is "latrine OR wilderness-with-penalty"; both
     paths are always actionable, so exploration on Relieve is
     unusual.
   - `Wash`: relief is "clean wash basin with `clean_water_units >= units_per_full_wash`";
     actionability is "is there a known clean wash basin?" — exactly
     the dirtiness branch added by CIREM-002.
3. **Live `wash_access_opportunities`** at
   `crates/worldwake-ai/src/candidate_generation.rs` (search for the
   helper) returns the agent's known clean wash-basin opportunity
   set; that is the dirtiness-specific predicate body.
4. **Live `matches_need`** at
   `crates/worldwake-ai/src/candidate_generation.rs` is a closure
   over the homeostatic need id; the generic exploration code uses
   it against `any_local_need_relief` and
   `need_has_known_acquisition_path`. The mismatch CIREM-002 found is
   that for dirtiness, `matches_need` matched water (which can
   *eventually* compose into wash) but the agent could not satisfy
   dirtiness without basin access. The substrate fix is to change
   what "actionable" means per need.
5. **Mismatch + correction**: this ticket extracts the per-need
   predicate without changing observable behavior. Each need's
   actionability check is moved from inline-ish-spread-across-helpers
   to an explicit per-need function, called from a single dispatch.
   The dirtiness branch becomes one declared predicate. Eat/Drink/etc.
   become declared predicates that wrap the existing helpers.
6. **Heuristic Removal Discipline (precision-rules §12)**: the
   refactor preserves the existing predicates by name, just dispatches
   them through a per-need map. No predicate is weakened or removed;
   none is added beyond formalizing what dirtiness already did.
7. **Coverage gap (precision-rules §3)**: existing focused tests
   (`generate_candidates_keeps_dirtiness_exploration_when_only_water_path_is_known`,
   `dirtiness_emits_water_acquisition_when_no_clean_wash_basin_is_known`)
   already exercise the dirtiness predicate. Add a parallel focused
   test per need in the dispatch table to lock per-need predicates
   in place.
8. **Coordination with future need additions**: when a future spec
   adds a new need with conditional relief, this dispatch is the
   single place that declares the new predicate, instead of growing
   the inline `if` arm.

## Architecture Check

1. **No backwards-compatibility shim**: the inline dirtiness `if`
   is removed in favor of the dispatch.
2. **Concrete state, not abstract score (FND-3)**: every per-need
   predicate reads concrete belief state (owned commodities, known
   wash basins, known sleep sites, etc.), not derived scores.
3. **No silent contract relaxation**: the refactor changes *where*
   the per-need relief check lives, not *what* it returns. Each
   predicate is verified against an existing focused test.

## Verification Layers

1. **Dirtiness predicate** -> existing focused tests
   `generate_candidates_keeps_dirtiness_exploration_when_only_water_path_is_known`,
   `dirtiness_emits_water_acquisition_when_no_clean_wash_basin_is_known`
   continue to pass with the predicate moved into the dispatch.
2. **Eat / Drink predicate** -> existing focused tests for hunger /
   thirst exploration suppression continue to pass; if no such
   focused test exists, add one as part of this ticket.
3. **Sleep predicate** -> existing S128 sleep-quality coverage
   continues to pass; the sleep predicate must read sleep-quality
   site beliefs, not just `matches_need`.
4. **Relieve predicate** -> wilderness-relief-with-penalty path
   means relieve is always actionable; the predicate returns true
   unconditionally for this need.
5. **Goldens**: `golden_survival_baseline`, `golden_survival_contested`,
   `golden_survival_scattered`, `golden_place_dirtiness`,
   `golden_sleep_episode` continue to pass under existing budgets.

## What to Change

### 1. Define the per-need predicate signature

Introduce a private trait or enum dispatch in the candidate-generation
module:

```rust
/// Returns true when the agent's known state already exposes an
/// actionable relief path for `need_id`. When true,
/// emit_exploration_candidates should skip emitting a need-driven
/// exploration candidate for this need.
fn relief_path_actionable(
    ctx: &GenerationContext<'_>,
    profile: &ExplorationProfile,
    need_id: HomeostaticNeedId,
) -> bool;
```

### 2. Implement per-need predicates

Move existing logic into per-need bodies:

- `relief_path_actionable_dirtiness(ctx)`:
  `!wash_access_opportunities(ctx).is_empty()`
- `relief_path_actionable_consumable(ctx, profile, need_id, matches_need)`:
  `any_local_need_relief || (path_reliable && need_has_known_acquisition_path)`
  — handles Hunger, Thirst, and any other consumable-need pattern.
- `relief_path_actionable_sleep(ctx)`: known sleep-quality place
  reachable. (If S128 coverage already encodes this, wrap the
  existing helper.)
- `relief_path_actionable_relieve(_ctx)`: `true` (wilderness path is
  always actionable; the penalty is the dampener, not exploration).

The dispatch is one `match need_id` site per file, in a single
location. Adding a new need fails to compile until the dispatch is
extended.

### 3. Refactor `emit_exploration_candidates`

Replace the dirtiness `if`/`else` block with:

```rust
if relief_path_actionable(ctx, profile, need_id) {
    continue;
}
```

The acquisition-failure-threshold path-reliable gating moves into the
consumable predicate body.

### 4. Add focused per-need predicate tests

If any predicate lacks a focused test, add one. Each test asserts
both directions: actionable-when-known-state-exposes-relief and
not-actionable-when-it-doesn't.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify —
  predicate signature, per-need bodies, dispatch, tests)

## Out of Scope

- Changing relief actions or their preconditions.
- Changing the acquisition-failure-threshold semantics.
- Generalizing `matches_need` beyond what the predicates need.
- Adding new needs.

## Acceptance Criteria

### Tests That Must Pass

1. `generate_candidates_keeps_dirtiness_exploration_when_only_water_path_is_known`
2. `dirtiness_emits_water_acquisition_when_no_clean_wash_basin_is_known`
3. `fully_blocked_self_care_source_emits_exploration_fallback`
4. New `relief_path_actionable_dirtiness_returns_true_when_clean_basin_known`
5. New `relief_path_actionable_consumable_returns_true_when_local_or_path_reliable`
6. New `relief_path_actionable_sleep_returns_true_when_sleep_site_known`
7. `cargo test -p worldwake-ai`
8. `cargo test --release -p worldwake-ai --test golden_survival_baseline -- --ignored --test-threads=1`
9. `cargo test --release -p worldwake-ai --test golden_survival_contested -- --ignored --test-threads=1`
10. `cargo test --release -p worldwake-ai --test golden_survival_scattered -- --ignored --test-threads=1`
11. `cargo test --release -p worldwake-ai --test golden_place_dirtiness -- --ignored --test-threads=1`
12. `./scripts/verify.sh`

### Invariants

1. **One dispatch site for need actionability**: there is exactly
   one `match need_id` in `emit_exploration_candidates` (and any
   helper it routes to) that decides actionability per need.
   No other call site replicates the per-need check.
2. **Dirtiness behavior preserved**: the dirtiness predicate must
   return the same actionability decision as the inline `if` branch
   it replaces.
3. **Adding a new need fails to compile until dispatch is extended**:
   the dispatch is exhaustive over `HomeostaticNeedId`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs::tests::relief_path_actionable_dirtiness_returns_true_when_clean_basin_known`
   — new
2. `crates/worldwake-ai/src/candidate_generation.rs::tests::relief_path_actionable_consumable_returns_true_when_local_or_path_reliable`
   — new
3. `crates/worldwake-ai/src/candidate_generation.rs::tests::relief_path_actionable_sleep_returns_true_when_sleep_site_known`
   — new (depends on the sleep-quality belief surface from S128;
   verify the existing helper before writing)

### Commands

1. `cargo test -p worldwake-ai relief_path_actionable`
2. `cargo test -p worldwake-ai candidate_generation::tests::generate_candidates_keeps_dirtiness_exploration_when_only_water_path_is_known`
3. `cargo test -p worldwake-ai candidate_generation::tests::dirtiness_emits_water_acquisition_when_no_clean_wash_basin_is_known`
4. `cargo test --release -p worldwake-ai --test golden_survival_baseline no_stuck_idle_windows_with_elevated_needs -- --ignored --test-threads=1`
5. `cargo test --release -p worldwake-ai --test golden_survival_contested no_stuck_idle_windows_with_elevated_needs -- --ignored --test-threads=1`
6. `cargo test --release -p worldwake-ai --test golden_survival_scattered no_stuck_idle_windows_with_elevated_needs -- --ignored --test-threads=1`
7. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-01.

- Replaced the inline dirtiness `if`/generic consumable `else` branch
  in `emit_exploration_candidates` with a private exhaustive
  `relief_path_actionable(...)` dispatch over `HomeostaticNeedId`.
- Preserved the existing exploration fallback emission set as
  Hunger/Thirst/Dirtiness only. Fatigue and Bladder are still declared
  in the actionability dispatch for exhaustiveness, but
  `emit_exploration_candidates` does not iterate them, so this refactor
  does not start resetting non-acquisition exhaustion counters or
  emitting new fatigue/bladder exploration candidates.
- Preserved the live consumable semantics exactly: local relief is
  actionable regardless of acquisition-exhaustion reliability, while a
  known acquisition path is gated by
  `profile.acquisition_failure_threshold`.
- Added focused predicate coverage for Dirtiness, consumables, Sleep,
  and Relieve, plus a regression assertion that fatigue acquisition
  exhaustion is not reset by the exploration fallback loop.

## Deviations

- The ticket's consumable sketch originally grouped local relief under
  `path_reliable`. Live code did not: local relief already bypassed the
  acquisition-failure threshold. The implementation and ticket text now
  preserve the live `local || (path_reliable && known_path)` contract.
- The ticket listed
  `golden_survival_contested::no_stuck_idle_windows_with_elevated_needs`
  as a must-pass command. The command fails with
  `Agent B` stuck idle from tick 349 to 389 with max need 799, and the
  same failure reproduces in a clean `HEAD` worktree at commit
  `564ddcea`. That makes it a pre-existing contested-golden blocker,
  not fallout from this refactor. Follow-up:
  `archive/tickets/CONTESTIDLE-001-pre-existing-survival-contested-stuck-idle.md`.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib relief_path_actionable`.
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::generate_candidates_keeps_dirtiness_exploration_when_only_water_path_is_known -- --exact`.
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::dirtiness_emits_water_acquisition_when_no_clean_wash_basin_is_known -- --exact`.
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::fully_blocked_self_care_source_emits_exploration_fallback -- --exact`.
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::generate_candidates_records_pending_reset_when_need_pressure_drops_below_threshold -- --exact`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo test --release -p worldwake-ai --test golden_survival_baseline no_stuck_idle_windows_with_elevated_needs -- --ignored --test-threads=1`.
- Passed `cargo test --release -p worldwake-ai --test golden_survival_scattered no_stuck_idle_windows_with_elevated_needs -- --ignored --test-threads=1`.
- Passed `cargo test --release -p worldwake-ai --test golden_place_dirtiness -- --test-threads=1`.
- Passed `cargo test --release -p worldwake-ai --test golden_sleep_episode -- --test-threads=1`.
- Passed `./scripts/verify.sh`, whose live gate set is
  `cargo fmt --all -- --check`, `cargo test --workspace`,
  `bash scripts/check_active_goal_removed.sh`,
  `cargo clippy --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
- Failed, pre-existing: `cargo test --release -p worldwake-ai --test golden_survival_contested no_stuck_idle_windows_with_elevated_needs -- --ignored --test-threads=1`. The same command failed in a clean temporary `HEAD` worktree with the same Agent B tick-349..389 stuck-idle window.
