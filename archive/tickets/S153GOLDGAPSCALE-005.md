# S153GOLDGAPSCALE-005: Office-vacancy patrol-gap golden

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: `archive/tickets/S153GOLDGAPSCALE-004.md`, `specs/S153-golden-gaps-ai-architecture-scaling.md`

## Problem

Before this ticket, `archive/tickets/S153GOLDGAPSCALE-004.md` had landed the office-backed patrol duty substrate, but S153 D3 still lacked the end-to-end golden proving the authored office vacancy chain. The remaining coverage gap was scenario/test ownership, not another production duty-state change: the golden needed to demonstrate vacancy-driven duty degradation/lapse, `ObligationDuty` patrol suppression, ordinary route traversal through the resulting patrol gap, route-danger observation, and deterministic replay.

## Assumption Reassessment (2026-05-20)

1. The substrate owner is `archive/tickets/S153GOLDGAPSCALE-004.md`: `OfficePatrolDuty` is an agent component with issuing office, assignee, route places, renewal/grace policy, lifecycle, actionability, and provenance.
2. The maintenance boundary is `worldwake-systems::patrol::office_patrol_duty_lifecycle_system`, run through the patrol system slot; it degrades/lapses office duties from `OfficeData.vacancy_since` and records append-only world mutation events.
3. The AI boundary is `worldwake-ai::candidate_generation::extract_patrol_candidates` and `worldwake-ai::ranking::patrol_motive`; a lapsed office duty suppresses `GoalKind::Patrol` candidate emission and zeroes patrol motive.
4. The golden must not introduce a hidden scenario flag for the patrol gap. It must use the duty component and the live patrol candidate/ranking path from the prerequisite.
5. Full autonomous route predation was not required for this ticket because the corrected ticket and S153 D3 allowed route predation or traversal. The landed golden uses the strongest honest route-traversal/route-danger observation proof: a merchant completes ordinary travel on the live Village Square -> South Gate edge after both duties lapse, and the fixture records concrete `RouteExperience` danger memory tied to an observed hostile event.

## Architecture Check

1. This ticket is test/golden-only because the production substrate and AI suppression path are owned by the prerequisite.
2. The proof remains state-mediated: office lifecycle, duty lifecycle, portfolio/ranking behavior, and route-danger observation communicate through stored state and event history.
3. No compatibility shim or parallel fake patrol-obligation path is allowed.

## Verified Layers

1. Office vacancy degrades/lapses duties -> authoritative `OfficePatrolDuty.lifecycle` plus event-log delta.
2. Lapsed duty suppresses patrol obligation -> candidate-generation assertion for absent `GoalKind::Patrol`.
3. Patrol gap route outcome -> action trace / event log for ordinary route traversal without guard patrol commits.
4. Route danger learned locally -> successor `RouteExperience` danger state after an observed hostile event.
5. Determinism -> same seed produces equal observation, world hash, and event-log hash.

## Landed Changes

### 1. Added `office_vacancy` golden scenario module

Added `crates/worldwake-ai/tests/scenarios/office_vacancy.rs` and registered it from `crates/worldwake-ai/tests/scenarios/mod.rs`.

### 2. Asserted the corrected S153 D3 chain

Built the smallest programmatic fixture that exercises the prerequisite substrate and proves vacancy -> duty lapse -> no patrol candidate -> no guard patrol commit -> ordinary route traversal -> local route-danger memory.

### 3. Refreshed generated golden docs

Regenerated the golden inventory, scenario index, coverage matrix, and new office-vacancy detail page from the source metadata.

## Landed Files

- `crates/worldwake-ai/tests/scenarios/office_vacancy.rs`
- `crates/worldwake-ai/tests/scenarios/mod.rs`
- `docs/generated/golden-coverage-matrix.md`
- `docs/generated/golden-e2e-inventory.md`
- `docs/generated/golden-scenario-index.md`
- `docs/generated/golden-scenario-details/office-vacancy.md`
- `specs/S153-golden-gaps-ai-architecture-scaling.md`

## Out of Scope

- Additional production duty lifecycle state beyond the prerequisite.
- The scaled-contention golden (`tickets/S153GOLDGAPSCALE-003.md`).
- A hidden scenario flag or authored script that directly suppresses patrols.

## Acceptance Result

### Tests Passed

1. `cargo test -p worldwake-ai --test golden_ai office_vacancy`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test -p worldwake-ai`

### Invariants

1. The patrol gap emerges from concrete office-backed duty state and ordinary AI selection.
2. The golden proves absence of patrol obligation through the duty lifecycle path, not through removed `PatrolProfile`/`PatrolRoute` fixture shortcuts.
3. Route-danger learning remains local and evidence-backed.

## Test Plan Result

### Added Tests

1. `crates/worldwake-ai/tests/scenarios/office_vacancy.rs` — S153 D3 end-to-end golden.

### Commands Run

1. `cargo test -p worldwake-ai --test golden_ai office_vacancy`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test -p worldwake-ai`

## Outcome

Completed on 2026-05-20.

- Added Scenario 444, `S153 Office Vacancy Patrol Gap`, as a programmatic `golden_ai` scenario.
- Registered `office_vacancy.rs` and regenerated the golden inventory, scenario index, coverage matrix, and new scenario detail page.
- Proved both guards' office-backed patrol duties lapse at `Tick(2)` when the issuing office is vacant past renewal/grace.
- Proved lapsed duties suppress patrol candidate generation and that neither guard commits patrol while the merchant completes the ordinary Village Square -> South Gate traversal.
- Recorded concrete route-danger aftermath as `RouteExperience` on the merchant after an observed hostile event and proved same-seed replay equality.

## Deviations

- The landed route uses the live direct Village Square -> South Gate prototype edge, not an Orchard Farm edge, because the prototype topology does not directly connect Village Square to Orchard Farm.
- The landed route outcome is ordinary traversal plus route-danger memory, not autonomous bandit predation. This matches the corrected ticket's route "predation or traversal" boundary and keeps the patrol-duty lapse proof honest.
- No RON scenario was added; the S153 spec permits inline golden fixtures, and this seam needs direct component/candidate assertions against the office-duty substrate.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_ai office_vacancy`.
- Passed `python3 scripts/golden_inventory.py --write --check-docs`.
- Passed `cargo test -p worldwake-ai`.
