# S107PRODIV-007: Golden tests — proactive diversification discovery, need-slack veto, cooldown

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — tests only
**Deps**: archive/tickets/S107PRODIV-005.md, archive/tickets/S107PRODIV-006.md

## Problem

The proactive-diversification behavior from tickets 005-006 has focused coverage, but no golden E2E coverage yet proves it through the existing exploration golden ownership surface. Three scenarios are still needed: (1) an agent with `DiversificationProfile` discovers a novel place while a matched control without the profile does not, (2) high need pressure vetoes proactive exploration without suppressing lawful need-driven exploration, and (3) repeated proactive exploration attempts are spaced by the per-agent cooldown.

## Assumption Reassessment (2026-04-17)

1. Existing owner: `crates/worldwake-ai/tests/golden_exploration.rs` already owns `ExploreLocation` golden coverage and passed unchanged on reassessment (`cargo test -p worldwake-ai --test golden_exploration`). Creating a new `golden_proactive_diversification.rs` file would split the same domain across two suites unnecessarily.
2. Scenario infrastructure: `golden_exploration.rs` already provides the right programmatic topology/harness style, decision-trace helpers, action-trace helpers, and scenario metadata format for new exploration scenarios.
3. CLI wiring from ticket 005 is real substrate, but these goldens do not need RON scenario loading. The honest proof surface is the existing programmatic exploration golden harness.
4. Existing generated golden docs mention only the pre-S107 exploration scenarios, so this ticket must refresh the inventory/docs after adding the new scenario metadata.

### Drafted vs Live

- `Already landed`: proactive candidate emission, ranking, cooldown state writes, and focused tests from tickets 001-006
- `Still live`: add three end-to-end exploration goldens for proactive discovery, need-slack veto, and cooldown spacing
- `New fallout`: generated golden inventory/docs need refresh after adding scenario metadata, including global inventory/matrix files and line-number-only detail drift in unrelated generated pages
- `No-change cited files`: no new dedicated `golden_proactive_diversification.rs` file is needed

## Architecture Check

1. Golden tests exercise the full agent decision cycle end-to-end: candidate generation → ranking → goal selection → plan search → action execution → perception. They prove the live exploration contract, not the focused arithmetic already covered in ticket 006.
2. No backward-compatibility shims — extend the existing exploration golden owner rather than creating a parallel suite.

## Verification Layers

1. Proactive diversification discovery → golden E2E: agent with profile visits unvisited place after needs stabilize
2. Need-slack veto → golden E2E: agent with profile + high needs never emits proactive ExploreLocation
3. Cooldown enforcement → golden E2E: exploration attempts spaced by cooldown_ticks
4. Control case → golden E2E: agent WITHOUT profile never visits the unvisited place
5. Multi-dependency golden tests exercise tickets 001-006 simultaneously

## What to Change

### 1. Extend the existing exploration golden owner

Add the new proactive-diversification scenarios to `crates/worldwake-ai/tests/golden_exploration.rs` using its existing helper style and scenario metadata format.

### 2. Scenario: Proactive Diversification Discovery

Setup:
- topology/harness built programmatically inside `golden_exploration.rs`
- 2 otherwise identical comparison runs: Explorer (with `DiversificationProfile`) and Settler control (without)
- comfortable needs and stable metabolism so proactive exploration, not survival pressure, is the differentiator

Assertions:
- Explorer selects and completes proactive travel to the novel target place
- Settler never selects proactive exploration and never reaches that novel target

### 3. Scenario: Need-Slack Veto

Setup:
- hungry exploration setup with `DiversificationProfile`, no known concrete relief path, and frontier beliefs present

Assertions:
- zero `ExploreLocation { motivating_need: Proactive }` emissions or selections across the run
- lawful need-driven exploration still appears, proving the veto is about proactive motivation rather than exploration removal

### 4. Scenario: Cooldown Enforcement

Setup:
- branching topology with multiple novel targets reachable over time
- agent with `DiversificationProfile` configured for a short cooldown and repeated proactive exploration opportunities

Assertions:
- at least two proactive exploration selections occur
- consecutive proactive exploration selections are spaced by at least `exploration_cooldown_ticks`

## Files to Touch

- `crates/worldwake-ai/tests/golden_exploration.rs` — add 3 proactive-diversification scenarios and any file-local helpers they need
- `docs/generated/golden-coverage-matrix.md` — refreshed generated coverage matrix
- `docs/generated/golden-e2e-inventory.md` — refreshed generated inventory
- `docs/generated/golden-scenario-index.md` — refreshed generated scenario index
- `docs/generated/golden-scenario-details/exploration.md` — refreshed exploration scenario details
- `docs/generated/golden-scenario-details/item-decay.md` — line-number-only generated detail drift from the same inventory refresh

## Out of Scope

- Focused unit tests for familiarity/novelty computation (ticket 006)
- RON scenario files for non-test use
- Observer binary integration

## Acceptance Criteria

### Tests That Must Pass

1. `golden_exploration::golden_s107_proactive_diversification_discovers_novel_place` — explorer commits proactive discovery while matched control does not
2. `golden_exploration::golden_s107_need_slack_veto_suppresses_proactive_exploration` — high need pressure blocks proactive motivation without removing lawful need-driven exploration
3. `golden_exploration::golden_s107_cooldown_spaces_proactive_exploration_attempts` — repeated proactive exploration selections respect cooldown spacing
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Proactive exploration is agent-diversity-dependent (FND-22) — identical scenarios produce different behavior based on DiversificationProfile presence/absence
2. Need-slack veto is absolute — no proactive exploration when any need exceeds comfort_threshold
3. Cooldown is per-agent — no interaction between agents' exploration timing

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_exploration.rs` — 3 new golden E2E scenarios validating emergent proactive exploration behavior
2. Generated golden inventory/docs refreshed for the new scenario metadata

### Commands

1. `cargo test -p worldwake-ai --test golden_exploration -- --list`
2. `cargo test -p worldwake-ai --test golden_exploration golden_s107_proactive_diversification_discovers_novel_place -- --exact`
3. `cargo test -p worldwake-ai --test golden_exploration golden_s107_need_slack_veto_suppresses_proactive_exploration -- --exact`
4. `cargo test -p worldwake-ai --test golden_exploration golden_s107_cooldown_spaces_proactive_exploration_attempts -- --exact`
5. `python3 scripts/golden_inventory.py --write --check-docs`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-17.

- Added three proactive-diversification scenarios to `crates/worldwake-ai/tests/golden_exploration.rs` under the existing exploration golden owner: profile-driven proactive discovery, high-need proactive veto, and cooldown spacing.
- Added file-local proactive exploration helpers for diversification-profile setup, calm-agent setup, and proactive selection trace inspection so the new scenarios prove the live end-to-end boundary directly.
- Refreshed generated golden docs after adding the new scenario metadata. The inventory refresh updated the expected exploration inventory/index/detail pages, the global coverage matrix, and the item-decay detail page via source line-number drift only.

## Deviations

- Reassessment narrowed the ticket from a new dedicated `golden_proactive_diversification.rs` file to the live owning suite `crates/worldwake-ai/tests/golden_exploration.rs`.
- The proactive discovery proof uses two otherwise identical comparison runs rather than two co-located agents in one world, which keeps the diversification difference isolated from unrelated social or co-location branching.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_exploration -- --list`
- Passed `cargo test -p worldwake-ai --test golden_exploration golden_s107_proactive_diversification_discovers_novel_place -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_exploration golden_s107_need_slack_veto_suppresses_proactive_exploration -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_exploration golden_s107_cooldown_spaces_proactive_exploration_attempts -- --exact`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
