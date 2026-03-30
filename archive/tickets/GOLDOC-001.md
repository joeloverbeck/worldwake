# GOLDOC-001: Add needs-assertion and system-ordering guidance to golden testing docs

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — documentation only
**Deps**: `docs/golden-e2e-testing.md`, `docs/FOUNDATIONS.md` (Principles 3, 9, 26)

## Problem

`docs/golden-e2e-testing.md` provides strong guidance on assertion hierarchy, ordering rules, trace usage, and scenario isolation, but lacks two pieces of guidance discovered during E20COMBEH-007 implementation:

1. **Transient vs. durable needs state**: Needs-based authoritative state (bladder, dirtiness, hunger) is transient — it resets at action commit but immediately re-escalates from basal metabolism. Golden tests that assert `bladder == pm(0)` at an arbitrary tick after relief will fail because basal drift accumulates post-commit. The doc does not address this.

2. **Actions-before-systems ordering trap for deprivation scenarios**: The tick execution order (drain inputs → progress actions → run systems) means an agent can complete a 1-tick travel action BEFORE the needs system fires deprivation consequences in the same tick. Golden tests that expect deprivation to fire at the agent's starting place can fail because the agent travels away first. This is a direct consequence of Principle 9 (scheduling is part of the world model) but isn't surfaced in the golden testing guide.

Both gaps led to test failures during E20COMBEH-007 that required redesign. Documenting them prevents repetition.

## Assumption Reassessment (2026-03-30)

1. `docs/golden-e2e-testing.md` currently has sections on Assertion Hierarchy, Ordering Rules, Trace Guidance, Scenario Isolation, Outdoor Place Affordance Trap, Multi-Hop Travel Observation, and Same-Tick Ordering for 1-Tick Actions. None address needs-state transience or the action-before-system ordering trap for deprivation scenarios specifically.
2. The "Outdoor Place Affordance Trap" section (line 156) covers a related but orthogonal concern: local outdoor affordance preventing travel. The new guidance covers a different trap: agent escaping to another place before system consequences fire.
3. The "Same-Tick Ordering for 1-Tick Actions" section (line 236) covers intra-tick ordering for travel departure and rival observation. The new guidance addresses a different same-tick interaction: travel completion before needs-system deprivation consequences.
4. `docs/FOUNDATIONS.md` Principle 9: "Scheduling, Simultaneity, and Tie-Breaking Are Part of the World Model" — directly supports documenting the actions-before-systems ordering as a world-model fact that golden tests must respect.
5. `docs/FOUNDATIONS.md` Principle 3: "Concrete State Over Abstract Scores" — reinforces that needs values are live concrete state with basal drift, not static post-action snapshots.
6. `docs/FOUNDATIONS.md` Principle 26: "Systems Interact Through State, Not Through Each Other" — the ordering trap exists precisely because needs-system reads state written by completed actions, not because it coordinates with travel directly.
7. No engine changes or test changes needed. This is purely documentation to prevent future golden test authoring mistakes.
8. DEPRTRACE-001 (archived as not-implemented) already rejected a needs-specific trace sink for this area. That decision remains correct — this ticket adds scenario design guidance, not new runtime tracing infrastructure.
9. The concrete E20COMBEH-007 failures that motivate this: `golden_latrine_preferred` initially failed because bladder re-escalated to pm(330) after toilet commit over 30 remaining ticks; `golden_deprivation_accident` initially failed because waste appeared at VillageSquare (1-tick travel destination) instead of CommonHouse (starting place). Both were resolved by scenario redesign, confirming the issues are authoring guidance gaps.

## Architecture Check

1. Documentation-only change. No code paths affected. No new abstractions, sinks, or runtime plumbing. Cleaner than code-level mitigations because the root issue is scenario design knowledge, not missing infrastructure.
2. No backward-compatibility shims or aliases introduced.

## Verification Layers

1. Needs-state transience guidance correctness → verified by existing E20COMBEH-007 golden tests that use the break-at-commit pattern: `golden_latrine_preferred`, `golden_wilderness_fallback`
2. Actions-before-systems ordering guidance correctness → verified by existing `golden_deprivation_accident` test that checks waste at agent's effective_place rather than hardcoded starting place
3. No new runtime trace boundary required → DEPRTRACE-001 archival decision still stands
4. Single-layer (documentation) ticket → no additional layer mapping applicable

## What to Change

### 1. Add "Needs-State Assertion Guidance" section to `docs/golden-e2e-testing.md`

Place after the "Assertion Hierarchy" section. Content should cover:

- Needs values (bladder, hunger, thirst, fatigue, dirtiness) are transient concrete state: they reset at action commit (e.g., `toilet` sets bladder to `pm(0)`) but immediately re-escalate from basal metabolism rates.
- Asserting `need == pm(0)` at an arbitrary tick count after relief will fail if basal drift has accumulated.
- Preferred patterns:
  - **Break at commit**: loop until action trace shows the relief action committed, then assert state immediately.
  - **Action trace proof**: assert the commit happened (proving the need was reset) rather than sampling authoritative state at a later tick.
  - **Bounded tolerance**: if testing post-commit state, use `value <= basal_rate * max_ticks_since_commit` rather than exact equality.
- The transient/durable distinction: waste creation is durable (entity persists), dirtiness penalty is durable (only removed by wash), but the need value itself is transient and continues evolving.

### 2. Add "Deprivation Ordering Trap" section to `docs/golden-e2e-testing.md`

Place after the "Outdoor Place Affordance Trap" section. Content should cover:

- Tick execution order: drain inputs → progress actions → run systems (Needs runs first among systems, but ALL actions complete before ANY system runs).
- Consequence: an agent can complete a 1-tick travel action within the same tick before the needs system fires deprivation consequences. The deprivation accident's waste is created at the agent's effective_place at the time the needs system runs, which may not be the starting place.
- This applies to any golden test where a system-level consequence must fire at a specific place. The agent's starting location is not guaranteed to be their location when systems execute.
- Preferred patterns:
  - **Check agent's actual location**: use `h.world.effective_place(agent)` for waste/consequence location assertions rather than hardcoding the starting place.
  - **Race the accident**: set `bladder_accident_tolerance_ticks` to 1 so the accident fires on the first critical tick, but accept that travel may still complete first.
  - **No fully isolated indoor place exists**: every indoor prototype place is within planner budget of PublicLatrine. Deprivation accident tests cannot rely on isolation alone; they must use low tolerance values.
- Reference `system_manifest.rs` for the canonical tick execution order and Principle 9 (scheduling is part of the world model).

## Files to Touch

- `docs/golden-e2e-testing.md` (modify — two new sections)

## Out of Scope

- Engine changes to needs system or tick ordering
- New trace sinks (rejected by DEPRTRACE-001)
- Changes to existing golden tests (E20COMBEH-007 tests already use correct patterns)
- Changes to `docs/precision-rules.md` (the guidance is specific to golden scenario design, not general precision rules)

## Acceptance Criteria

### Tests That Must Pass

1. Existing suite: `cargo test --workspace`
2. Documentation inventory: `python3 scripts/golden_inventory.py --write --check-docs`

### Invariants

1. New sections must cite Principle 9 (scheduling) and Principle 3 (concrete state) from `docs/FOUNDATIONS.md`
2. New sections must reference the canonical tick execution order in `system_manifest.rs`
3. No code changes introduced
4. Existing "Outdoor Place Affordance Trap" section unchanged — new section is complementary, not a replacement

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo test --workspace`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `scripts/verify.sh`

## Outcome

- **Completion date**: 2026-03-30
- **What changed**:
  - Added "Needs-State Assertion Guidance" section to `docs/golden-e2e-testing.md` after Assertion Hierarchy, covering transient vs. durable needs state, break-at-commit pattern, action-trace proof, and bounded tolerance.
  - Added "Deprivation Ordering Trap" section after Outdoor Place Affordance Trap, covering actions-before-systems tick ordering, `effective_place` assertions, and Principle 9/26 references.
  - Renumbered `golden_travel_physiology.rs` scenario identifiers from 1–7 to 58–64 to resolve duplicate scenario ID errors in `golden_inventory.py`.
- **Deviations**: Scenario renumbering was not in the original ticket but was required to clear pre-existing `golden_inventory.py` duplicate-ID errors introduced by the same file.
- **Verification**: `cargo test --workspace` passes; `python3 scripts/golden_inventory.py --write --check-docs` reports clean (79 scenario blocks, 0 duplicates).
