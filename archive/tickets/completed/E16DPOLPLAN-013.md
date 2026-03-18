# E16DPOLPLAN-013: Golden Scenario 16 — Survival pressure suppresses political goals

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: E16DPOLPLAN-007

## Problem

No golden test proves that survival needs (critical hunger) suppress political goals and that political goals emerge after survival pressure is relieved.

## Assumption Reassessment (2026-03-18)

1. `GoalKind::ClaimOffice` is still ranked as `GoalPriorityClass::Medium` — confirmed in `crates/worldwake-ai/src/ranking.rs`.
2. Political suppression is not handled by a standalone `suppression.rs` or `is_suppressed()` goal-generation gate. Current architecture centralizes this in `crates/worldwake-ai/src/goal_policy.rs`, and `rank_candidates()` filters suppressed goals after candidate generation.
3. `GoalKind::ClaimOffice` and `GoalKind::SupportCandidateForOffice` are suppressed when self-care or danger pressure is `High` or above, not only when hunger is `Critical`. Critical hunger remains a valid setup, but the invariant under test is `High-or-above suppresses political goals`.
4. Candidate generation for office claims still depends on the agent having a belief about a visibly vacant office. The scenario must explicitly seed office belief, or no political candidate will exist to suppress.
5. Existing office goldens already cover baseline political execution paths:
   - simple `DeclareSupport` claim
   - competing claims with supporter coalition
   - bribe coalition
   - threaten coalition
   - remote travel to jurisdiction
   What is missing is the cross-family handoff from self-care suppression back into the political claim path.
6. Existing suppression goldens live primarily in `golden_combat.rs`, not `golden_social.rs`. The closest reusable pattern is “suppression under self-care pressure, then lift after eating,” but that pattern is not yet proven for `ClaimOffice`.

## Architecture Check

1. Current architecture is already the correct long-term shape for this behavior:
   - candidate generation stays domain-focused and emits lawful political goals from local beliefs
   - `goal_policy.rs` owns cross-family suppression rules
   - ranking applies suppression uniformly across goal families
   This is cleaner and more extensible than adding political-specific suppression logic or special-case checks in office code.
2. A new golden is still beneficial because it proves the full integration path across systems:
   belief-seeded political candidate -> suppression by self-care pressure -> self-care execution -> suppression lift -> political execution -> succession installation.
3. The scenario should assert behavioral ordering via action traces (`eat` before `declare_support`) and authoritative world state (`office_holder`), instead of depending only on raw event-log ordering.
4. Tick-boundary nuance discovered during implementation: hunger relief and the later `declare_support` commit can be observed on the same simulation tick boundary after the eat commit. The durable invariant is therefore:
   - no `declare_support` commit while hunger remains `High-or-above`
   - `eat` commits before `declare_support`
   rather than requiring a strictly earlier tick number for hunger relief than declaration.
5. No production-code architectural change is justified by this ticket. The value is coverage of the current architecture, not redesign.

## What to Change

### 1. Add to `golden_offices.rs`

- **Setup**: Vacant office at VillageSquare. Hungry agent with `enterprise_weight=pm(800)`, eligible, and owned local bread. Hunger must start at or above the agent's `High` threshold so political goals are suppressed by the shared goal-policy path.
- **Expected**: Agent does not commit `declare_support` while hunger remains `High-or-above`. Agent eats first, hunger relief is observed before or at the same tick boundary as the first `declare_support` commit, then politics proceeds normally and the agent is installed.
- **Assertions**:
  - `eat` commits before any `declare_support` commit for the claimant
  - no `declare_support` commit occurs while hunger is still `High-or-above`
  - hunger falls below the `High` threshold during the scenario
  - agent eventually becomes office holder
  - deterministic replay should remain stable for the scenario, matching the existing golden-suite pattern for new office scenarios

> **Golden E2E documentation**: Review and update `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` as necessary to reflect the new scenario(s) added by this ticket.

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Thirst/fatigue/dirtiness suppression of political goals (same code path)
- Bribe/Threaten scenarios
- Changes to suppression logic
- Changes to production code

## Acceptance Criteria

### Tests That Must Pass

1. `golden_survival_pressure_suppresses_political_goals` — eat before declare, no declare while hunger is `High-or-above`, eventually installed
2. `golden_survival_pressure_suppresses_political_goals_replays_deterministically`
3. Relevant office slice and AI suite remain green

### Invariants

1. `High-or-above` survival pressure suppresses Medium political goals through shared goal-policy evaluation
2. Political goals re-emerge once survival pressure drops below the suppression threshold
3. The suppression handoff is temporal and causal: self-care commits before political declaration, and political declaration does not commit while survival pressure remains at the suppression threshold
4. Office installation still occurs through the ordinary political path; no special suppression bypass exists

## Test Plan

### New/Modified Tests

1. `golden_offices.rs::golden_survival_pressure_suppresses_political_goals`
2. `golden_offices.rs::golden_survival_pressure_suppresses_political_goals_replays_deterministically`

### Commands

1. `cargo test -p worldwake-ai golden_survival_pressure_suppresses_political_goals -- --nocapture`
2. `cargo test -p worldwake-ai --test golden_offices`
3. `cargo test -p worldwake-ai`

## Outcome

- **Completion date**: 2026-03-18
- **What actually changed**:
  - Added `golden_survival_pressure_suppresses_political_goals` to `crates/worldwake-ai/tests/golden_offices.rs`
  - Added deterministic replay coverage via `golden_survival_pressure_suppresses_political_goals_replays_deterministically`
  - Updated `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` to reflect the new office suppression coverage
  - Corrected this ticket's assumptions and command examples before implementation
- **Deviations from original plan**:
  - Assertion strategy uses action traces plus authoritative office state rather than only raw event-log ordering
  - Deterministic replay coverage was added even though the original ticket only named one test, to keep the office slice aligned with the suite's established pattern
  - Tick-boundary behavior required a more precise invariant: no `declare_support` commit while hunger remains `High-or-above`, rather than demanding a strictly earlier tick number for hunger relief than declaration
- **Verification results**:
  - `cargo test -p worldwake-ai golden_survival_pressure_suppresses_political_goals -- --nocapture` ✅
  - `cargo test -p worldwake-ai --test golden_offices` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace` ✅
