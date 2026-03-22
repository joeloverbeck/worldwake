# E16DPOLPLAN-026: Update E16DPOLPLAN-006 and dependent golden scenario tickets after coalition-aware planner

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — ticket corrections only
**Deps**: E16DPOLPLAN-025

## Problem

After E16DPOLPLAN-022 through E16DPOLPLAN-025 land, the GOAP planner's behavior for `ClaimOffice` changes fundamentally. Several existing tickets were written under the assumption that the planner would find Bribe/Threaten plans with the current (broken) search semantics. Now that the planner correctly reasons about coalition-building, these tickets need assumption reassessment and potential corrections.

## Assumption Reassessment (2026-03-18)

1. **E16DPOLPLAN-006** (integration tests: planner finds Bribe/Threaten plans): The test scenarios are now achievable with the coalition-aware planner. However, test setup details need correction:
   - Tests must set up **contested** scenarios (competitor with existing support) so the planner is motivated to Bribe/Threaten rather than just DeclareSupport.
   - Test 1 (Bribe plan): needs a competitor with 1+ supporters so self-declaration alone produces a tie.
   - Test 2 (Threaten plan): same — needs competitor so Threaten is needed for majority.
   - Test 3 (Travel + Bribe): correct as-is (agent not at jurisdiction), but must also have competitor.
   - Test 4 (Threaten rejected): correct as-is — high-courage target means Threaten produces no state change, so planner skips it.

2. **E16DPOLPLAN-010** (Golden Scenario 13: Bribe → support coalition): Needs competitor setup so the AI agent's planner chooses Bribe over solo DeclareSupport.

3. **E16DPOLPLAN-011** (Golden Scenario 14: Threaten with courage diversity): Needs competitor setup so the planner chooses Threaten.

4. **E16DPOLPLAN-020** (Golden Scenario 19: Incumbent defense): Already has an incumbent with support — should work correctly with the coalition-aware planner.

5. **E16DPOLPLAN-008** (Golden Scenario 11: Simple office claim): Uncontested scenario — planner now produces GoalSatisfied instead of ProgressBarrier for terminal kind. Assertions about terminal kind may need updating.

## Architecture Check

1. These are ticket corrections, not code changes. The architecture changes are in E16DPOLPLAN-022 through E16DPOLPLAN-025.
2. No backwards-compatibility concerns — tickets are living documents that should reflect current codebase state.

## What to Change

### 1. Update E16DPOLPLAN-006 test setup descriptions

For tests 1-3, add competitor setup to each scenario:
- A second agent at the jurisdiction who has already declared support for themselves (or a third party). This ensures the actor faces competition and needs coalition-building.
- The test verifies the planner naturally selects Bribe/Threaten because DeclareSupport alone would produce a tie (ProgressBarrier fallback), not a winning coalition (GoalSatisfied).

For test 4 (reject Threaten against high-courage), verify the planner falls back to a different strategy (Bribe if available, or DeclareSupport ProgressBarrier).

### 2. Update E16DPOLPLAN-010, E16DPOLPLAN-011 golden scenario setup

Ensure each scenario has explicit competitor presence so the planner's coalition-building motivation is clearly grounded.

### 3. Update E16DPOLPLAN-008 terminal kind expectations

The uncontested office claim now produces GoalSatisfied (actor has 1 support, 0 competitors) instead of ProgressBarrier. Update assertions about `PlanTerminalKind` if any exist.

### 4. Document the dependency chain

Add a note to each affected ticket referencing E16DPOLPLAN-022 through E16DPOLPLAN-025 as prerequisites.

## Files to Touch

- `tickets/E16DPOLPLAN-006.md` (modify)
- `tickets/E16DPOLPLAN-008.md` (modify — if terminal kind assertions exist)
- `tickets/E16DPOLPLAN-010.md` (modify — competitor setup)
- `tickets/E16DPOLPLAN-011.md` (modify — competitor setup)

## Out of Scope

- Implementation of the updated tests (done when each ticket is implemented)
- Changes to production code (covered by E16DPOLPLAN-022 through E16DPOLPLAN-025)

## Acceptance Criteria

### Tests That Must Pass

1. All affected tickets have correct assumption reassessment sections
2. Test setups include competitor agents where needed for planner motivation
3. Terminal kind expectations match new planner behavior
4. Dependency chains are documented

### Invariants

1. No ticket claims "Engine Changes: None" while depending on engine changes from E16DPOLPLAN-022-025

## Test Plan

### New/Modified Tests

None — this ticket modifies ticket files, not code.

### Commands

1. Review each affected ticket for consistency with the coalition-aware planner

## Outcome

- **Completion date**: 2026-03-18
- **What changed**:
  - E16DPOLPLAN-006: Added competitor setup requirements to tests 1-3; updated test 4 fallback behavior; added deps on 022-025; added dependency chain note
  - E16DPOLPLAN-008: Added terminal kind note (GoalSatisfied for uncontested); added deps on 022-025; added dependency chain note
  - E16DPOLPLAN-010: Added competitor agent to setup and expected behavior with coalition counting; added deps on 022-025; added dependency chain note
  - E16DPOLPLAN-011: Added competitor agent to setup and expected behavior with coalition motivation; added deps on 022-025; added dependency chain note
- **Deviations**: None — all four changes applied as specified
- **Verification**: All tickets have correct deps, competitor setup where needed, terminal kind expectations updated, and dependency chain notes
