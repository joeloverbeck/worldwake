# S96OBLSAT-006: Golden test — obligation does not starve survival needs

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: archive/tickets/S96OBLSAT-001.md, archive/tickets/S96OBLSAT-002.md, archive/tickets/S96OBLSAT-003.md, archive/tickets/S96OBLSAT-004.md, archive/tickets/S96OBLSAT-005.md

## Problem

The simulation observer report showed Guard Theron executing 487 PostNotice actions while starving to death. After implementing satiation, a golden E2E test is needed to prove that the full pipeline (tracker update → ranking dampening → goal selection) allows survival needs to compete with obligation goals.

## Assumption Reassessment (2026-04-12)

1. `golden_planner_pathology.rs` exists at `crates/worldwake-ai/tests/golden_planner_pathology.rs` and currently owns two scenario blocks (`Scenario 142`, `Scenario 143`) for observer-reported pathologies. That is the strongest existing owning file for a new observer-pathology golden.
2. `golden_ai_decisions.rs` has `golden_fallback_to_addressable_need_when_top_need_unsatisfiable` at line 2290 — tests fallback when top need is unsatisfiable, NOT when a non-survival goal outranks survival. This test is distinct from the satiation test.
3. `golden_integration.rs` already proves autonomous `PostNotice` selection/commitment in `golden_s58_autonomous_notice_reroutes_later_travel`, so this ticket should reuse that remembered-threat substrate pattern rather than reopening notice-path ownership at the lower layer.
4. GoalKinds exercised: `PostNotice(ThreatWarning)` (goal.rs:90), `AcquireCommodity` (goal.rs:22). Both exist.
5. `NeedDeprivation` death cause exists at `crates/worldwake-core/src/combat.rs:60`.
6. `PerceptionProfile` is required for durable local observation and the existing `guard_perception_profile()` helper in `golden_planner_pathology.rs` is suitable for the guard actor in this slice.
7. A co-located live hostile would lawfully introduce combat and danger-driven branches that are outside the contract under test. The clean golden isolation is: the guard first directly observes a hostile at one place, then starts the scenario window at a different place with local food/water while retaining a remembered threat belief for `PostNotice`.
8. Golden conventions in `docs/golden-e2e-testing.md` expect a deterministic replay companion for new scenarios unless explicitly justified otherwise, and generator-backed scenario metadata/doc refresh is part of the broadened proof when a new `// Scenario N:` block is added.

## Architecture Check

1. Testing the emergent interaction between obligation satiation and survival ranking remains the right E2E boundary, but the golden should isolate that branch with a remembered remote threat instead of a co-located live hostile so combat/pathfinding do not become alternate explanations.
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

- One guard agent who first directly observes a hostile at a warned place, then starts the golden window at a different place with local food and water available
- The guard retains a remembered hostile belief triggering `ThreatWarning` for the warned place while no live combat target is co-located in the self-care location
- Agent has `notice_posting_weight: Permille::new_unchecked(900)`
- Agent has `ObligationSatiationProfile::default()`
- Agent has `PerceptionProfile` (required for observation)
- Set hunger and thirst to critical levels (>750 permille)
- Run for 200 ticks

### 3. Assertions

- Agent commits `post_notice` repeatedly enough for satiation to matter (more than the default threshold)
- Agent performed at least one `eat` action
- Agent performed at least one `drink` action
- After a sustained `post_notice` streak, a later committed self-care action still appears, proving obligation posting does not dominate indefinitely
- PostNotice executions do not exceed 80% of total committed actions
- Agent is alive at end of simulation (no `DeadAt` component / no `NeedDeprivation` death)

## Files to Touch

- `crates/worldwake-ai/tests/golden_planner_pathology.rs` (modify)
- `docs/generated/golden-e2e-inventory.md` (generated)
- `docs/generated/golden-scenario-index.md` (generated)
- `docs/generated/golden-scenario-details/planner-pathology.md` (generated)
- `docs/generated/golden-coverage-matrix.md` (generated expected fallout if refreshed)

## Out of Scope

- Modifying existing golden tests
- Testing PostBounty satiation in this golden test (covered by unit tests in ticket 005; PostBounty E2E can be a follow-up)
- Multi-agent scenarios with satiation interaction

## Acceptance Criteria

### Tests That Must Pass

1. `obligation_satiation_allows_survival_needs_to_override_posting` passes
2. Deterministic replay companion for the new scenario passes
3. Existing golden tests in `golden_planner_pathology.rs` continue to pass
4. Full golden suite: `cargo test -p worldwake-ai`

### Invariants

1. Agent survival is not achieved by disabling PostNotice — it is achieved by satiation dampening allowing survival goals to outcompete saturated obligations
2. PostNotice still executes (agent is not forbidden from posting) — satiation only dampens priority, not capability

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_planner_pathology.rs::obligation_satiation_allows_survival_needs_to_override_posting` — proves the full satiation pipeline prevents obligation starvation
2. `crates/worldwake-ai/tests/golden_planner_pathology.rs::<replay companion>` — proves the new pathology scenario replays deterministically

### Commands

1. `cargo test -p worldwake-ai --test golden_planner_pathology obligation_satiation_allows_survival_needs_to_override_posting -- --exact`
2. `cargo test -p worldwake-ai --test golden_planner_pathology obligation_satiation_allows_survival_needs_to_override_posting_replays_deterministically -- --exact`
3. `cargo test -p worldwake-ai --test golden_planner_pathology`
4. `cargo test -p worldwake-ai`
5. `python3 scripts/golden_inventory.py --write --check-docs`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-04-12
- Added Scenario 144 in `crates/worldwake-ai/tests/golden_planner_pathology.rs` with a remembered-remote-threat setup that proves repeated `PostNotice` still happens, but later `eat` and `drink` commits reappear and the guard survives.
- Added the required deterministic replay companion for the new golden scenario.
- Refreshed generated golden inventory/index/details/matrix docs after introducing the new `// Scenario 144:` block.

## Deviations From Draft

- The original draft described an active hostile at the guard's location. Live reassessment showed that a co-located hostile would lawfully introduce combat and danger branches, so the landed scenario instead uses a directly observed hostile at `DustyTrail` plus a later self-care window at `HearthstoneInn` with a remembered combat belief.
- The honest runtime contract is not "notice posting wins the first action." In the landed scenario, early self-care can occur first; the proved invariant is that after a sustained `post_notice` streak, committed self-care actions still reappear and posting does not dominate indefinitely.
- Generated golden docs widened beyond the per-file scenario details as expected: the inventory/index/matrix files also changed because the generator recomputes global counts and coverage tables.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_planner_pathology obligation_satiation_allows_survival_needs_to_override_posting -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_planner_pathology obligation_satiation_allows_survival_needs_to_override_posting_replays_deterministically -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_planner_pathology`
- Passed `cargo test -p worldwake-ai`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
