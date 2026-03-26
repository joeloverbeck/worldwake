# E17CRITHEJUS-012: Golden test — theft creates EntityMissing violation for owner

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E17CRITHEJUS-006 (steal action), E17CRITHEJUS-007 (investigate SuspectedTheft), E17CRITHEJUS-010 (theft candidates), E17CRITHEJUS-015 (typed social-evidence detail)

## Problem

No end-to-end proof exists that the theft-to-discovery pipeline works: steal -> owner visits -> EntityMissing violation -> investigation -> SuspectedTheft recording. This golden test proves the P15 (violated expectation), P7 (local discovery), P3 (concrete evidence), and P12 (belief-state separation) invariants hold across the full system chain.

## Assumption Reassessment (2026-03-25)

1. Golden tests live in `crates/worldwake-ai/tests/golden_*.rs`. The closest structural precedent is `golden_emergent.rs` which contains cross-system E2E scenarios. Crime discovery scenarios belong there or in a new `golden_crime.rs`.
2. The golden test harness (`GoldenHarness` or equivalent) provides `step_once()`, `enable_tracing()`, `enable_action_tracing()`, agent creation with profiles, place graph setup, and assertion helpers.
3. S27's violation detection is already covered by existing golden tests. This test extends the chain: theft event -> owner visit -> EntityMissing -> investigate -> SuspectedTheft.
4. AI-layer ticket: full action registries required (steal + investigate + needs). Decision traces and action traces both relevant for debugging.
5. N/A — no ordering dependency between agents beyond tick sequence.
6. N/A.
7. N/A.
8. N/A.
9. N/A.
10. Isolation: scenario needs Agent A (thief with `TheftDispositionProfile`), Agent B (owner, arrives later), an item lot owned by B at a place. No other agents needed initially. Exclude other goal-generating profiles to isolate theft/investigation behavior.
11. Mismatch: the original ticket only asserted that `AgentBeliefStore` contains `SocialObservation(SuspectedTheft)`. After reassessment, the meaningful contract is stronger and more precise: the stored crime evidence must use explicit typed theft detail from `E17CRITHEJUS-015`, not a tuple convention.
12. Timing: A steals (multi-tick based on `steal_duration_ticks`). B must arrive AFTER steal commits. B's violation detection fires on perception refresh. B's investigation takes `investigation_duration_ticks`. Ensure enough ticks for full pipeline.

## Architecture Check

1. Golden test proves an emergent cross-system chain (theft -> violation -> investigation -> SuspectedTheft). This is the canonical E17 regression: if any link in the chain breaks, this test fails.
2. No backwards-compatibility aliasing. New test file or new scenario in existing golden file.

## Verification Layers

1. Steal commits -> item possession transferred -> action trace check
2. Owner arrives at theft location -> EntityMissing violation fires -> decision trace check
3. Owner investigates -> SuspectedTheft recorded in ViolationMemory -> authoritative state check
4. Typed theft evidence recorded in AgentBeliefStore -> authoritative state check
5. SuspectedTheft.suspect is None (no witness, thief not seen) -> authoritative state check
6. Deterministic replay -> replay companion test

## What to Change

### 1. New golden test scenario

Create a scenario in `golden_emergent.rs` (or new `golden_crime.rs` if the file would be cleaner):

**Setup**:
- Place P1 (theft location), Place P2 (owner's starting location)
- Travel edge between P1 and P2
- Agent A (thief): at P1, has `TheftDispositionProfile`, has needs profiles for survival
- Agent B (owner): at P2, has `ViolationDispositionProfile`, has needs profiles
- Item lot (e.g., 10 units of Gold) at P1, owned by B
- Both agents have `PerceptionProfile`

**Execution**:
- Step ticks until A steals the item (A should generate `StealItem` candidate and execute)
- Then step ticks to have B travel to P1 (may need to manually inject travel or set up motivation)
- After B arrives at P1, step ticks until violation detection fires and investigation completes

**Assertions**:
- After A's steal commits: `possessor_of(item) == A`, `owner_of(item) == B`
- After B arrives at P1: B's `ViolationMemory` contains `EntityMissing` violation
- After B investigates: B's `ViolationMemory` contains `SuspectedTheft { missing_entity: item, expected_place: P1, suspect: None }`
- B's `AgentBeliefStore` contains typed theft evidence observation
- Conservation: `verify_live_lot_conservation()` passes throughout

### 2. Deterministic replay companion

Add a replay test for the scenario (standard pattern: save state, replay, verify hash match).

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` or `crates/worldwake-ai/tests/golden_crime.rs` (new scenario)

## Out of Scope

- Witness-to-accusation chain (E17CRITHEJUS-013)
- Fine/Exile outcomes (E17CRITHEJUS-013)
- Guard response (E19)
- Multiple concurrent thefts
- Theft at occupied locations (witness perception)
- AI planner search debugging (use decision traces if test fails)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_theft_creates_entity_missing_violation` (or similar name): full pipeline from steal -> owner visit -> EntityMissing -> investigate -> SuspectedTheft
2. `golden_theft_creates_entity_missing_violation_replay`: deterministic replay companion
3. Conservation invariant holds throughout the scenario
4. Existing suite: `cargo test -p worldwake-ai --test golden_*`

### Invariants

1. P15: Owner discovers theft through violated expectation (EntityMissing), not through omniscience
2. P7: Discovery requires owner to physically visit the theft location
3. P3: Evidence is concrete world state (SuspectedTheft in ViolationMemory), not abstract score
4. P12: Owner's belief store records the suspicion; world truth is separate
5. P14: Suspect is None (thief identity unknown without witness)
6. Conservation maintained across all ticks

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs` (or `golden_crime.rs`) — golden scenario + replay companion

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent` (or `golden_crime`)
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
