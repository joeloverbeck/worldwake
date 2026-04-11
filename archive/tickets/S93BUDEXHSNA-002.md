# S93BUDEXHSNA-002: TreatWounds budget exhaustion snapshot golden tests

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — tests only
**Deps**: S93BUDEXHSNA-001

## Problem

TreatWounds goals generate the worst candidate explosions in the cli-evaluation simulation (5739 candidates at depth 3 for Merchant Vara, 4151 for Kael). These require Medicine at Hearthstone Inn (2 hops away), and the planner cannot decompose the multi-hop acquisition chain within the expansion budget. No golden test captures this pattern. This ticket adds 2 golden tests reproducing the exact TreatWounds budget-exhaustion snapshots.

## Assumption Reassessment (2026-04-11)

1. **Ticket says** S93BUDEXHSNA-001 already landed `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` and its shared helpers. **Live code has** no such file yet; `tickets/S93BUDEXHSNA-001.md` is still an untracked draft. **Correction applied**: this ticket now owns creating `golden_budget_exhaustion_snapshots.rs` plus only the local cli-evaluation topology/belief/item/workstation helper scaffolding required for its two TreatWounds snapshots. This is safe because the blocker is a small local test-only substrate required to land the current ticket, while S93BUDEXHSNA-001 still owns the four AcquireCommodity cases.
2. **`GoalKind::TreatWounds`** exists — grep confirms `TreatWounds` in `crates/worldwake-ai/src/goals/mod.rs`. The goal takes a `patient: EntityId` parameter.
3. This is a single-layer ticket (AI planner tests only). The shared boundary is `search_plan`.
4. Existing care-domain tests already prove the lawful lower-layer contract for remote medicine acquisition and TreatWounds search depth in `crates/worldwake-ai/tests/golden_care.rs`; this ticket remains a golden snapshot reproduction ticket, not a production contradiction ticket.
5. Live `GoalKind` under test: `TreatWounds { patient }`. The planner routes this through operators including `queue_for_care_target`, `pick_up` (Medicine), `travel`, and the care action itself. The snapshot data shows 4151-5739 candidates from these operators × known entities (9-16 Waste inflating the set).

## Architecture Check

1. Adds tests and the narrow local snapshot scaffolding into `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs`, following the same snapshot seeding and dual-phase assertion patterns already used by nearby AI golden tests. No new architectural patterns introduced.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. Budget exhaustion reproduction → `search_plan` returns `BudgetExhausted` (planner output)
2. Phase 2 fix verification → `search_plan` returns `Found`, treatment chain executes (planner output + world state)
3. Single-layer ticket — additional layer mapping not applicable.

## What to Change

### 1. Test 5: `merchant_vara_treat_wounds_at_dusty_trail_budgets_exhaust`

Snapshot tick 456. Merchant Vara at Dusty Trail, goal `TreatWounds { patient: self }`.

Setup per spec:
- Needs: hunger=214, thirst=1000, fatigue=294, bladder=120, dirtiness=557
- Inventory: 5×Grain
- Cognitive profile: Merchant Vara's custom (max_node_expansions=300, max_plan_depth=10, max_candidates_per_expansion=150)
- Execution budget: beam_width=10, preferred_operator_boost=3, max_prerequisite_locations=3
- 12 known entities: Dusty Trail, self, Kael, 9×Waste — all believed at Dusty Trail
- Place contents at Dusty Trail: Merchant Vara, Guard Theron, 1×Bow, 5×Grain, 1×Sword, 15×Waste
- Adjacent: Thornwall Village (Kael, Mill, Loom, Well, 20×Coin)
- Agent must have wounds to trigger TreatWounds goal

Because the dependency file is absent on the live branch, this ticket also creates the local test-only helper substrate in `golden_budget_exhaustion_snapshots.rs`: cli-evaluation topology builder, focused belief seeding helpers, and place/item/workstation setup helpers used by these TreatWounds snapshots.

Phase 1 assertion: `search_plan` returns `BudgetExhausted`.
Phase 2 (`#[ignore]`): `search_plan` returns `Found`.

### 2. Test 6: `kael_treat_wounds_vara_at_dusty_trail_budgets_exhaust`

Snapshot tick 471. Kael at Dusty Trail, goal `TreatWounds { patient: Merchant Vara }`.

Setup per spec:
- Needs: hunger=162, thirst=591, fatigue=304, bladder=8, dirtiness=572
- Inventory: 20×Coin
- Cognitive profile: default (max_node_expansions=224, max_plan_depth=8, max_candidates_per_expansion=200)
- Execution budget: default (beam_width=8, preferred_operator_boost=2, max_prerequisite_locations=3)
- 16 known entities: Thornwall Village, Dusty Trail, self, Merchant Vara, 9×Waste, Mill, Loom, Well
- Believed locations: self + Merchant Vara + 9×Waste at Dusty Trail; Mill + Loom + Well at Thornwall Village
- Place contents at Dusty Trail: Kael, Merchant Vara, Guard Theron, 1×Bow, 20×Coin, 5×Grain, 1×Sword, 16×Waste
- Adjacent: Thornwall Village (Mill, Loom, Well)
- Merchant Vara must have wounds (she is the patient)

Phase 1 assertion: `search_plan` returns `BudgetExhausted`.
Phase 2 (`#[ignore]`): `search_plan` returns `Found`.

## Files to Touch

- `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` (new — add local snapshot helpers plus 2 TreatWounds tests)

## Out of Scope

- Planner changes — this ticket captures failures, not fixes
- AcquireCommodity tests (ticket 001)
- Phase 2 fix verification (un-ignoring `#[ignore]` tests — done when planner fix lands)
- Wound system mechanics — tests only need the patient to have a non-empty `WoundList` component

## Acceptance Criteria

### Tests That Must Pass

1. `merchant_vara_treat_wounds_at_dusty_trail_budgets_exhaust` — asserts `BudgetExhausted` with ≥300 expansions
2. `kael_treat_wounds_vara_at_dusty_trail_budgets_exhaust` — asserts `BudgetExhausted` with ≥224 expansions
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Each test reproduces the exact budget-exhaustion signature from the simulation snapshot (goal, location, cognitive profile, entity density match)
2. Phase 2 `#[ignore]` tests compile but do not run by default

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` — 2 new golden tests capturing TreatWounds budget exhaustion from cli-evaluation simulation snapshots

### Commands

1. `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-11.

- Created `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` with the local cli-evaluation topology builder and focused helper scaffolding needed to reproduce TreatWounds budget-exhaustion snapshots without pulling in unrelated planner changes.
- Added `merchant_vara_treat_wounds_at_dusty_trail_budgets_exhaust` and `kael_treat_wounds_vara_at_dusty_trail_budgets_exhaust`, both asserting `search_plan` returns `BudgetExhausted` at the expected node-budget floor.
- Added matching ignored follow-up tests that compile the eventual `Found` path for the planner-fix phase without running by default.

## Deviations

- S93BUDEXHSNA-001 had not landed on the live branch, so this ticket absorbed the small local test-only substrate needed to create `golden_budget_exhaustion_snapshots.rs` instead of only modifying an existing file. The remaining AcquireCommodity snapshot cases stay owned by S93BUDEXHSNA-001.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots -- --nocapture`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
