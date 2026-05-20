# S153GOLDGAPSCALE-005: Office-vacancy patrol-gap golden

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: `archive/tickets/S153GOLDGAPSCALE-004.md`, `specs/S153-golden-gaps-ai-architecture-scaling.md`

## Problem

`archive/tickets/S153GOLDGAPSCALE-004.md` lands the office-backed patrol duty substrate, but S153 D3 still needs the end-to-end golden that proves the authored office vacancy chain. The remaining coverage gap is scenario/test ownership, not another production duty-state change: the golden must demonstrate magistrate death or vacancy, duty degradation/lapse, `ObligationDuty` patrol suppression, ordinary route predation or traversal through the resulting patrol gap, route-danger observation, and deterministic replay.

## Assumption Reassessment (2026-05-20)

1. The substrate owner is `archive/tickets/S153GOLDGAPSCALE-004.md`: `OfficePatrolDuty` is an agent component with issuing office, assignee, route places, renewal/grace policy, lifecycle, actionability, and provenance.
2. The maintenance boundary is `worldwake-systems::patrol::office_patrol_duty_lifecycle_system`, run through the patrol system slot; it degrades/lapses office duties from `OfficeData.vacancy_since` and records append-only world mutation events.
3. The AI boundary is `worldwake-ai::candidate_generation::extract_patrol_candidates` and `worldwake-ai::ranking::patrol_motive`; a lapsed office duty suppresses `GoalKind::Patrol` candidate emission and zeroes patrol motive.
4. The golden must not introduce a hidden scenario flag for the patrol gap. It must use the duty component and the live patrol candidate/ranking path from the prerequisite.
5. If full route predation is not reachable with existing combat/bandit substrate, narrow this ticket to the strongest honest authored route-traversal/route-danger observation proof and create a follow-up for the predation-specific actor behavior rather than weakening the duty-lapse assertion.

## Architecture Check

1. This ticket is test/golden-only because the production substrate and AI suppression path are owned by the prerequisite.
2. The proof remains state-mediated: office lifecycle, duty lifecycle, portfolio/ranking behavior, and route-danger observation communicate through stored state and event history.
3. No compatibility shim or parallel fake patrol-obligation path is allowed.

## Verification Layers

1. Office vacancy degrades/lapses duties -> authoritative `OfficePatrolDuty.lifecycle` plus event-log delta.
2. Lapsed duty suppresses patrol obligation -> decision trace and/or candidate-generation assertion for absent `GoalKind::Patrol`.
3. Patrol gap route outcome -> action trace / event log for route traversal or predation without guard interception.
4. Route danger learned locally -> `RoutePreferenceEntry` or successor route-danger belief state.
5. Determinism -> same seed produces equal event log/report outputs.

## What to Change

### 1. Add `office_vacancy` golden scenario module

Create `crates/worldwake-ai/tests/scenarios/office_vacancy.rs` and register it from `crates/worldwake-ai/tests/scenarios/mod.rs`.

### 2. Assert the full S153 D3 chain

Build the smallest authored fixture that exercises the prerequisite substrate and proves vacancy -> duty lapse -> no obligation patrol -> patrol gap route event -> local route-danger learning.

### 3. Refresh generated golden docs

Run `python3 scripts/golden_inventory.py --write --check-docs` after adding source metadata.

## Files to Touch

- `crates/worldwake-ai/tests/scenarios/office_vacancy.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs` (modify)
- `docs/generated/golden-e2e-inventory.md` (modify)
- `docs/generated/golden-scenario-index.md` (modify)
- `docs/generated/golden-scenario-details/*.md` (new/modify as generated)
- `specs/S153-golden-gaps-ai-architecture-scaling.md` (truth-sync after landing)

## Out of Scope

- Additional production duty lifecycle state beyond the prerequisite.
- The scaled-contention golden (`tickets/S153GOLDGAPSCALE-003.md`).
- A hidden scenario flag or authored script that directly suppresses patrols.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_ai office_vacancy`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test -p worldwake-ai`

### Invariants

1. The patrol gap emerges from concrete office-backed duty state and ordinary AI selection.
2. The golden proves absence of patrol obligation through the duty lifecycle path, not through removed `PatrolProfile`/`PatrolRoute` fixture shortcuts.
3. Route-danger learning remains local and evidence-backed.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/office_vacancy.rs` — S153 D3 end-to-end golden.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai office_vacancy`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test -p worldwake-ai`
