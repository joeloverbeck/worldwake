# AGEFOOREP-003: Theft survival has a founded food path under spoilage

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Yes — theft/food acquisition planning or scenario contract, depending on reassessment
**Deps**: Follows from `archive/specs/S178-perishable-food-spoilage.md` and `archive/tickets/AGEFOOREP-001.md`.

## Problem

`scenarios/survival-theft.ron` proves a concealed displayed-lot theft branch: staged visible stock -> theft -> self-consume, with physical aftermath and later suspicion relay. The scenario intentionally gives the thief no coin, no harvestable food source, and no remote food fallback. With S178 spoilage enabled, this finite displayed stock branch exceeds the authored hunger critical-run envelope.

The current `commodity_perish_profile: {}` opt-out in `scenarios/survival-theft.ron` is temporary scenario containment, not completion. The follow-up must either define a founded theft-survival path under spoilage or revise the scenario's survival contract honestly.

## Assumption Reassessment (2026-06-02)

1. The motivating golden is `scenarios::survival_theft::survival_theft_proves_concealed_staged_lot_branch` in `crates/worldwake-ai/tests/scenarios/survival_theft.rs`.
2. The scenario comment explicitly excludes harvestable and remote fallback food for the thief, so direct harvest replenishment from AGEFOOREP-001 cannot make this scenario survive.
3. The live `GoalKind` surfaces under audit are `StealItem { target_item }`, `ConsumeOwnedCommodity`, and any lawful follow-on food acquisition goal that preserves the theft branch.
4. The shared abstraction boundary is ownership/custody/access around displayed sale lots plus thief belief of visible food, theft aftermath, and owner expectation/suspicion.
5. FOUNDATIONS alignment: the fix must preserve FND-4 explicit transfer/source-sink accounting, FND-10 theft aftermath, FND-14B belief-backed planner targets, FND-17 expectation-based missing-stock discovery, FND-20 reusable agent reasoning, and FND-24 ownership/custody/access separation.
6. Reassessment exposed this as a separate bug from AGEFOOREP-001: anonymous harvest-source workstation tagging fixes direct harvest execution, while this ticket owns the intentionally non-harvest theft survival contract.

## Architecture Check

The theft scenario should not be made green by silently adding durable food, hidden fallback supply, or scenario-specific planner rails. A lawful solution must either let the thief repeatedly acquire visible food through ordinary theft/planning as stock remains available, create a concrete local replenishment/supply process that keeps the theft branch truthful, or narrow the golden's survival contract if finite displayed stock is the intended authored limit.

## Verification Layers

1. Thief sees and targets only locally visible or belief-backed displayed food -> decision trace for `StealItem`.
2. Theft transfers concrete item lots and leaves aftermath -> action trace plus authoritative ownership/custody/location state.
3. Owner suspicion still comes from expectation mismatch and evidence, not omniscience -> event/social observation trace.
4. Survival envelope passes or is truthfully narrowed with spoilage enabled -> affected golden and deterministic replay.

## What to Change

### 1. Theft Food Path Reassessment

Audit whether the thief fails because repeated visible-stock theft is not generated/ranked/planned, because finite stock cannot physically sustain the run, or because the scenario contract should not require 1440-tick survival under spoilage.

### 2. Founded Contract Update

Implement the narrow founded behavior or revise the scenario/golden contract so it proves the theft branch without hiding spoilage through an unowned opt-out.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify if theft candidate generation is at fault)
- `crates/worldwake-ai/src/ranking.rs` (modify if theft ranking suppresses survival-relevant theft)
- `crates/worldwake-ai/src/search/tests.rs` (modify/add focused theft planning coverage)
- `crates/worldwake-ai/tests/scenarios/survival_theft.rs` (modify if contract/proof surface changes)
- `scenarios/survival-theft.ron` (modify to remove temporary opt-out after founded behavior lands)

## Out of Scope

- Merchant substitute-market restock/restage; owned by `tickets/AGEFOOREP-002.md`.
- Direct anonymous harvest-source workstation tagging; owned by `archive/tickets/AGEFOOREP-001.md`.

## Acceptance Criteria

### Tests That Must Pass

1. Focused AI/planner coverage proving the selected theft-survival contract at the strongest relevant layer.
2. `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_theft::`
3. Relevant broader AI checks selected during implementation.

### Invariants

1. The thief cannot plan from non-local, non-believed food or ownership facts.
2. Any stolen or consumed food has explicit transfer/source-sink history and preserves theft aftermath.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-ai/src/ranking.rs`, or `crates/worldwake-ai/src/search/tests.rs` — focused theft path proof if engine behavior changes.
2. `crates/worldwake-ai/tests/scenarios/survival_theft.rs` — existing golden should pass or be truthfully narrowed once this ticket lands.

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_theft::`
3. `cargo test -p worldwake-ai`
