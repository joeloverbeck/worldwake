# GOLDENE2E-004: Healing a Wounded Agent

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Unlikely
**Deps**: None

## Problem

The Heal goal, Care action domain, and medicine consumption path are completely untested at the E2E level. The current engine already contains candidate generation for heal goals, planner semantics for `heal`, and a real care action lifecycle, but none of that is covered by the golden AI loop. This is an entire action domain (Care) with zero golden coverage.

**Coverage gap filled**:
- GoalKind: `Heal { target }` (completely untested)
- GoalKind: `AcquireCommodity { purpose: Treatment }` (if medicine must be acquired)
- ActionDomain: Care (completely untested)
- Cross-system chain: Wounds present → AI detects wounded co-located agent → Heal goal → heal action → wound severity reduced

## Assumption Reassessment (2026-03-12)

1. `GoalKind::Heal { target: EntityId }` exists in `crates/worldwake-core/src/goal.rs` (confirmed).
2. `CommodityKind::Medicine` exists (confirmed in `crates/worldwake-core/src/items.rs`).
3. Candidate generation for heal goals exists in `crates/worldwake-ai/src/candidate_generation.rs` and emits `GoalKind::Heal { target }` when the actor has medicine and a local wounded target (confirmed).
4. A heal action handler exists in `crates/worldwake-systems/src/combat.rs` under the Care domain, with unit coverage for affordance gating, medicine consumption, and wound reduction (confirmed).
5. `WoundList` component tracks wounds per agent (confirmed in `crates/worldwake-core/src/wounds.rs`).
6. The generic golden harness already supports medicine setup through `give_commodity(...)`; no harness-level medicine-specific support is required.

## Architecture Check

1. This test exercises a fundamentally different goal type: targeted care for another local agent rather than self-oriented need relief. It validates the AI's ability to generate and complete a goal against another entity through the real loop.
2. A new test file `golden_care.rs` is warranted since Care is a distinct domain.
3. No shims, aliases, or special-case hooks. The scenario should pass by composing the existing candidate-generation, planning, action-registry, and wound-treatment systems.
4. Scope should stay on the same-place healing path. Medicine acquisition for treatment is a separate planning branch and should only be asserted here if the scenario genuinely requires it.

## What to Change

### 1. Add harness helper: `agent_wound_load()`

In `golden_harness/mod.rs`:
```rust
pub fn agent_wound_load(&self, agent: EntityId) -> u32 {
    self.world
        .get_component_wound_list(agent)
        .map_or(0, WoundList::wound_load)
}
```

Rationale: this reuses the existing domain abstraction instead of re-implementing wound aggregation logic in the test harness.

### 2. Create `golden_care.rs` test file

New file: `crates/worldwake-ai/tests/golden_care.rs`

**Test: `golden_healing_wounded_agent`**

Setup:
- Healer agent at Village Square with `Quantity(1)` Medicine, healthy (no wounds, low needs).
- Wounded agent at Village Square with a significant wound (e.g., `severity: pm(400)` starvation wound), wound_capacity `pm(1000)`.
- Both agents have otherwise calm needs so the scenario isolates the care path instead of competing on hunger/fatigue urgency.

Expected emergent chain:
1. Healer's AI detects wounded co-located agent + medicine in inventory.
2. AI generates `Heal { target: wounded_agent }` goal.
3. Heal action executes — medicine consumed, wound severity reduced.
4. The scenario should not rely on medicine acquisition, travel, or bespoke harness shortcuts.

### 3. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P4 from Part 3 to Part 1 once the golden scenario is passing.
- Update Part 2: Care ActionDomain and Heal GoalKind marked as tested.
- Update summary counts so they reflect the actual post-change suite size and coverage totals.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add `agent_wound_load()` if needed by the test)
- `crates/worldwake-ai/tests/golden_care.rs` (new — care domain golden tests)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Self-healing (agent healing itself)
- Medicine acquisition via travel or trade
- Natural wound recovery (clotting/recovery rates)
- Multiple sequential heal actions
- Heal goal priority vs. other goals

## Engine Discovery Protocol

This ticket is a golden e2e test that exercises emergent behavior through the real AI loop.
If implementation reveals that the engine cannot produce the expected emergent behavior,
the following protocol applies:

1. **Diagnose**: Identify the specific engine limitation (missing candidate generation path, planner op gap, action handler deficiency, belief view gap, etc.).
2. **Do not downgrade the test**: The test scenario defines the desired emergent behavior. Do not weaken assertions or remove expected behaviors to work around engine gaps.
3. **Fix forward**: Implement the minimal, architecturally sound engine change that enables the emergent behavior. Document the change in a new "Engine Changes Made" subsection under "What to Change". Each fix must:
   - Follow existing patterns in the affected module
   - Include focused unit tests for the engine change itself
   - Not introduce compatibility shims or special-case logic
   - Prefer strengthening existing abstractions over adding ticket-specific helpers
4. **Scope guard**: If the required engine change exceeds this ticket's effort rating by more than one level (e.g., a Small ticket needs a Large engine change), stop and apply the 1-3-1 rule: describe the problem, present 3 options, recommend one, and wait for user confirmation before proceeding.
5. **Document**: Record all engine discoveries and fixes in the ticket's Outcome section upon completion, regardless of whether fixes were needed.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_healing_wounded_agent` — healer with medicine heals a wounded co-located agent
2. Wounded agent's total wound severity decreases after healing
3. Healer's Medicine quantity decreases (medicine consumed)
4. Both agents remain alive throughout the test
5. Coverage report `reports/golden-e2e-coverage-analysis.md` updated: Care ActionDomain and Heal GoalKind marked as tested, with suite totals corrected
6. Existing suite: `cargo test -p worldwake-ai --test golden_care`
7. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation: Medicine lots never increase
3. Determinism: same seed produces same outcome
4. Wound severity can only decrease via healing, never increase from the heal action

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_care.rs::golden_healing_wounded_agent` — proves heal goal and care action pipeline through the real AI loop

### Commands

1. `cargo test -p worldwake-ai --test golden_care golden_healing_wounded_agent`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- **Completed**: 2026-03-12
- **What changed**:
  - Added `crates/worldwake-ai/tests/golden_care.rs` with a real AI-loop care scenario proving a healer consumes medicine and reduces a co-located patient's wound load.
  - Added a deterministic replay companion test for the healing scenario.
  - Added `agent_wound_load()` to the golden harness, reusing `WoundList::wound_load()` instead of duplicating wound aggregation logic.
  - Updated `reports/golden-e2e-coverage-analysis.md` to record care-domain coverage and the expanded golden suite layout.
- **Deviations from original plan**:
  - No engine changes were required; the existing candidate generation, planner semantics, and heal action lifecycle already supported the scenario.
  - The harness did not need medicine-specific setup support because `give_commodity(...)` already handled it generically.
  - The helper was implemented as `agent_wound_load()` rather than a bespoke permille-summing helper to preserve the existing wound abstraction.
  - Added a replay determinism test beyond the original minimum scenario.
- **Verification**:
  - `cargo test -p worldwake-ai --test golden_care`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
