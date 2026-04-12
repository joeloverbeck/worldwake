# S96OBLSAT-006: Golden test — obligation does not starve survival needs

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: archive/tickets/S96OBLSAT-001.md, archive/tickets/S96OBLSAT-002.md, archive/tickets/S96OBLSAT-003.md, S96OBLSAT-004, S96OBLSAT-005

## Problem

The simulation observer report showed Guard Theron executing 487 PostNotice actions while starving to death. After implementing satiation, a golden E2E test is needed to prove that the full pipeline (tracker update → ranking dampening → goal selection) allows survival needs to compete with obligation goals.

## Assumption Reassessment (2026-04-12)

1. `golden_planner_pathology.rs` exists at `crates/worldwake-ai/tests/golden_planner_pathology.rs`. Contains `seed_guard_theron` helper at line 169 and `guard_perception_profile` at line 131. Existing tests: `cross_location_water_acquisition_succeeds_without_budget_exhaustion` (line 486), `degenerate_zero_step_loop_blocks_actionable_goals` (line 603).
2. `golden_ai_decisions.rs` has `golden_fallback_to_addressable_need_when_top_need_unsatisfiable` at line 2290 — tests fallback when top need is unsatisfiable, NOT when a non-survival goal outranks survival. This test is distinct from the satiation test.
3. `golden_integration.rs` tests PostNotice selection and commitment but not the interaction with competing survival needs under satiation.
4. GoalKinds exercised: `PostNotice(ThreatWarning)` (goal.rs:90), `AcquireCommodity` (goal.rs:22). Both exist.
5. `NeedDeprivation` death cause exists at `crates/worldwake-core/src/combat.rs:60`.
6. `PerceptionProfile` is required for agents that need to observe post-production output (CLAUDE.md golden test note). The `guard_perception_profile()` helper at line 131 provides this.
12. Scenario isolation: the test intentionally creates a single-agent scenario with co-located food/water to isolate the satiation-vs-survival interaction. Competing agents, trade, and combat are excluded because they are outside the contract under test.

## Architecture Check

1. Testing the emergent interaction between satiation dampening and survival need ranking — neither system alone produces this behavior. This is a cross-system E2E test exercising: commit handler (tracker update) → ranking (satiation dampening) → goal selection (survival wins).
2. No backwards-compatibility shims. New test function in existing file.

## Verification Layers

1. Agent performs eat/drink actions despite active PostNotice obligations → golden E2E assertion on committed action kinds
2. PostNotice executions do not exceed 80% of total actions → golden E2E ratio assertion
3. Agent survives (no NeedDeprivation death) → golden E2E assertion on agent alive state
4. Cross-system ticket: tracker update (worldwake-systems) → ranking dampening (worldwake-ai) → goal selection (worldwake-ai). The golden test is the appropriate proof surface for this end-to-end chain.

## What to Change

### 1. Add golden test function

In `crates/worldwake-ai/tests/golden_planner_pathology.rs`, add:

```rust
#[test]
fn obligation_satiation_allows_survival_needs_to_override_posting()
```

### 2. Test setup

- One guard agent at a location with food and water available (use existing `seed_guard_theron` as a starting point, or a similar seeding function)
- Agent has an active hostile entity belief triggering ThreatWarning at agent's location
- Agent has `notice_posting_weight: Permille::new_unchecked(900)`
- Agent has `ObligationSatiationProfile::default()`
- Agent has `PerceptionProfile` (required for observation)
- Set hunger and thirst to critical levels (>750 permille)
- Run for 200 ticks

### 3. Assertions

- Agent performed at least one `eat` action
- Agent performed at least one `drink` action
- PostNotice executions do not exceed 80% of total committed actions
- Agent is alive at end of simulation (no `DeadAt` component / no `NeedDeprivation` death)

## Files to Touch

- `crates/worldwake-ai/tests/golden_planner_pathology.rs` (modify)

## Out of Scope

- Modifying existing golden tests
- Testing PostBounty satiation in this golden test (covered by unit tests in ticket 005; PostBounty E2E can be a follow-up)
- Multi-agent scenarios with satiation interaction

## Acceptance Criteria

### Tests That Must Pass

1. `obligation_satiation_allows_survival_needs_to_override_posting` passes
2. Existing golden tests in `golden_planner_pathology.rs` continue to pass
3. Full golden suite: `cargo test -p worldwake-ai`

### Invariants

1. Agent survival is not achieved by disabling PostNotice — it is achieved by satiation dampening allowing survival goals to outcompete saturated obligations
2. PostNotice still executes (agent is not forbidden from posting) — satiation only dampens priority, not capability

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_planner_pathology.rs::obligation_satiation_allows_survival_needs_to_override_posting` — proves the full satiation pipeline prevents obligation starvation

### Commands

1. `cargo test -p worldwake-ai -- obligation_satiation_allows_survival`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
