# HARPREE14-013: Strengthen weak assertions in golden e2e

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: ALL other HARPREE14 tickets (must be implemented LAST)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-D03

## Problem

Two assertions in the golden e2e are observational rather than required:
1. Line 696: Blocked intent check is wrapped in `if saw_blocker { eprintln!(...) }` -- observational only
2. Line 1023: Loot assertion is `if !b_looted { eprintln!("Note: ...non-fatal") }` -- observational only

These weaken the test's value as a regression gate.

## Assumption Reassessment (2026-03-11)

1. The specific line references in the original spec are stale. In the current file:
   - the blocked-intent observation sits around lines 936-940
   - the loot observation sits around lines 1306-1310
2. The blocked-intent check is still observational -- confirmed.
3. The loot check is still observational -- confirmed.
4. The blocked-intent rationale is ALREADY documented in `golden_e2e.rs`:
   - blocked intents are only recorded when an action fails through `handle_plan_failure()`
   - in this scenario, the planner may simply withhold a harvest plan while the source is depleted
   - therefore "no blocked intent observed" is consistent with the current architecture
5. The death-and-loot scenario currently passes deterministically on the seeded golden harness, and the non-fatal loot note did not fire in targeted runs. Under the current architecture and seed, looting is behaving as an invariant worth asserting.
6. Because (4) is already true, the remaining ticket scope is narrower than originally written: this is primarily a test-hardening ticket for the loot outcome plus any additional deterministic coverage needed to keep that assertion robust.
7. The golden e2e remains a high-value regression gate, but it is not the only one; unit tests around planning, failure handling, and candidate generation also carry architectural invariants. This ticket should strengthen the golden gate without pretending it is the entire safety net.

## Architecture Check

1. Hard assertions are beneficial when they lock in behavior the architecture already treats as deterministic for the seeded scenario.
2. Promoting opportunistic loot to a hard assertion is cleaner than keeping a non-fatal note because Scenario 8 is explicitly named `Death Cascade and Opportunistic Loot`; the current test should assert the behavior it claims to cover.
3. Keeping the blocked-intent check observational is architecturally correct. Making it a hard assert would overfit the test to a specific planner failure path instead of the real invariant, which is only that the agent eventually harvests after regeneration.
4. Extra deterministic coverage around Scenario 8 is more valuable than forcing blocked-intent recording. It hardens a true end-to-end invariant without pushing the planner toward artificial failure behavior.
5. If the loot hard assert fails after strengthening, the right response is to fix the AI or scenario setup, not to re-weaken the assertion.

## What to Change

### 1. Convert loot assertion to hard assert

Change the observational loot check (currently around line 1309) to:
```rust
assert!(b_looted, "Agent B should have looted within 100 ticks after killing Agent A");
```

If this fails, investigate and fix the underlying AI behavior rather than keeping it observational.

### 2. Keep the blocked intent check observational, but treat the existing rationale as sufficient

No new behavior change is required for the blocked-intent portion unless the surrounding comment is stale. The current rationale already matches the architecture and should remain observational.

### 3. Add deterministic coverage for the death-and-loot scenario

Strengthen Scenario 8 beyond a single boolean assert by adding or extending replay-style coverage so the same seed proves the death-and-loot outcome is repeatable, not accidental.

## Files to Touch

- `crates/worldwake-ai/tests/golden_e2e.rs` (modify)

## Out of Scope

- Changing AI planning logic to force deterministic blocked intents
- Rewriting blocked-intent generation semantics
- Modifying existing hard assertions
- Changes to any production code

## Acceptance Criteria

### Tests That Must Pass

1. Golden e2e passes with the loot assertion as a hard `assert!`
2. Blocked intent check remains observational with accurate rationale
3. Scenario 8 has deterministic coverage strong enough to justify the hard loot assertion
4. `cargo test --workspace` passes
5. `cargo clippy --workspace` -- no new warnings

### Invariants

1. No production code changes
2. All existing hard assertions still pass
3. The test must not assert blocked-intent recording unless the architecture truly guarantees an action-failure path
4. If loot hard assert fails, the AI behavior or scenario assumptions must be fixed (never weaken the test)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_e2e.rs` -- Scenario 8 strengthened with a hard loot assertion
2. `crates/worldwake-ai/tests/golden_e2e.rs` -- Scenario 8 deterministic replay / repeatability coverage

### Commands

1. `cargo test -p worldwake-ai --test golden_e2e` (targeted)
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-12
- What actually changed:
  - Reassessed the ticket against the live code before implementation
  - Corrected the ticket's stale line references and narrowed the scope to the remaining real work
  - Confirmed the blocked-intent rationale was already correctly documented and kept that scenario observational
  - Converted Scenario 8's opportunistic-loot note into a hard assertion
  - Added deterministic replay coverage for the death-and-loot scenario so the stronger assertion is backed by repeatability
- Deviations from original plan:
  - Did not change the blocked-intent scenario because its existing rationale already matched the current planner/failure-handling architecture
  - Added an extra replay-strengthening test for Scenario 8 because a single hard assert was weaker than the architecture warranted
- Verification results:
  - `cargo test -p worldwake-ai --test golden_e2e golden_death_cascade_and_opportunistic_loot -- --exact`
  - `cargo test -p worldwake-ai --test golden_e2e golden_death_cascade_and_opportunistic_loot_replays_deterministically -- --exact`
  - `cargo test -p worldwake-ai --test golden_e2e`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
