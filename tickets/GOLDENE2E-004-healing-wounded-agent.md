# GOLDENE2E-004: Healing a Wounded Agent

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Possible
**Deps**: None

## Problem

The Heal goal, Care action domain, and medicine consumption path are completely untested at the E2E level. The `candidate_generation` logic for heal goals requires medicine in inventory and a wounded target at the same location. This is an entire action domain (Care) with zero golden coverage.

**Coverage gap filled**:
- GoalKind: `Heal { target }` (completely untested)
- GoalKind: `AcquireCommodity { purpose: Treatment }` (if medicine must be acquired)
- ActionDomain: Care (completely untested)
- Cross-system chain: Wounds present → AI detects wounded co-located agent → Heal goal → heal action → wound severity reduced

## Assumption Reassessment (2026-03-12)

1. `GoalKind::Heal { target: EntityId }` exists in `crates/worldwake-core/src/goal.rs` (confirmed).
2. `CommodityKind::Medicine` exists (confirmed in `crates/worldwake-core/src/items.rs`).
3. Candidate generation for heal goals exists in `crates/worldwake-ai/src/candidate_generation.rs` — needs verification that it emits Heal goals when agent has medicine + wounded target nearby.
4. A heal action handler exists in `crates/worldwake-systems/` under the Care domain — needs verification during implementation.
5. `WoundList` component tracks wounds per agent (confirmed in `crates/worldwake-core/src/wounds.rs`).

## Architecture Check

1. This test exercises a fundamentally different goal type — healing another agent rather than self-oriented goals. It validates the AI's ability to generate goals targeting other entities.
2. A new test file `golden_care.rs` is warranted since Care is a distinct domain.
3. No shims — uses existing wound and medicine systems.

## What to Change

### 1. Add harness helper: `agent_wound_total()`

In `golden_harness/mod.rs`:
```rust
pub fn agent_wound_total(&self, agent: EntityId) -> Permille {
    self.world
        .get_component_wound_list(agent)
        .map_or(pm(0), |wl| {
            let total: u16 = wl.wounds.iter().map(|w| w.severity.value()).sum();
            pm(total.min(1000))
        })
}
```

### 2. Create `golden_care.rs` test file

New file: `crates/worldwake-ai/tests/golden_care.rs`

**Test: `golden_healing_wounded_agent`**

Setup:
- Healer agent at Village Square with `Quantity(1)` Medicine, healthy (no wounds, low needs).
- Wounded agent at Village Square with a significant wound (e.g., `severity: pm(400)` starvation wound), wound_capacity `pm(1000)`.
- Wounded agent also has some food to prevent death from starvation during the test.

Expected emergent chain:
1. Healer's AI detects wounded co-located agent + medicine in inventory.
2. AI generates `Heal { target: wounded_agent }` goal.
3. Heal action executes — medicine consumed, wound severity reduced.

### 3. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P4 from Part 3 to Part 1.
- Update Part 2: Care ActionDomain now tested, Heal GoalKind now tested.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add `agent_wound_total()`)
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
4. **Scope guard**: If the required engine change exceeds this ticket's effort rating by more than one level (e.g., a Small ticket needs a Large engine change), stop and apply the 1-3-1 rule: describe the problem, present 3 options, recommend one, and wait for user confirmation before proceeding.
5. **Document**: Record all engine discoveries and fixes in the ticket's Outcome section upon completion, regardless of whether fixes were needed.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_healing_wounded_agent` — healer with medicine heals a wounded co-located agent
2. Wounded agent's total wound severity decreases after healing
3. Healer's Medicine quantity decreases (medicine consumed)
4. Both agents remain alive throughout the test
5. Coverage report `reports/golden-e2e-coverage-analysis.md` updated: Care ActionDomain and Heal GoalKind marked as tested
6. Existing suite: `cargo test -p worldwake-ai --test golden_care`
7. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation: Medicine lots never increase
3. Determinism: same seed produces same outcome
4. Wound severity can only decrease via healing, never increase from the heal action

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_care.rs::golden_healing_wounded_agent` — proves heal goal and care action pipeline

### Commands

1. `cargo test -p worldwake-ai --test golden_care golden_healing_wounded_agent`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
