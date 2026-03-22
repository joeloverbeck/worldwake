# S16S09GOLVAL-005: Golden — Spatial Awareness Enables Multi-Hop Plan at Hub Node

**Status**: NOT IMPLEMENTED
**Priority**: HIGH
**Effort**: None
**Engine Changes**: None
**Deps**: `specs/S16-s09-golden-validation.md`, `docs/golden-e2e-testing.md`, `archive/tickets/completed/S16S09GOLVAL-004.md`

## Problem

This active ticket claims the S16/S09 spatial-hub golden is still missing and proposes adding it to `crates/worldwake-ai/tests/golden_combat.rs`.

That assumption is no longer true. The spatial golden was already implemented earlier, then strengthened and refactored in follow-up tickets. Keeping this ticket active would duplicate delivered coverage and point future work at the wrong file and the wrong assertion boundary.

## Assumption Reassessment (2026-03-21)

1. The core scenario already exists. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_spatial_multi_hop_plan` and `golden_spatial_multi_hop_plan_replays_deterministically` are present and passing.
2. The current owning file is `crates/worldwake-ai/tests/golden_ai_decisions.rs`, not `crates/worldwake-ai/tests/golden_combat.rs`. That is the correct architecture because the behavior under test is planner-guided needs/travel acquisition, not combat.
3. The current golden proves a stronger contract than this ticket describes. It asserts the tick-0 decision-trace selection boundary via `selection.selected_plan`, `selection.selected_plan_source`, `next_step`, and selected-plan search provenance, then separately proves downstream travel, harvest, and hunger relief.
4. The current docs already reflect the delivered architecture:
   - `docs/golden-e2e-scenarios.md` lists the VillageSquare spatial golden under `golden_ai_decisions.rs`
   - `docs/golden-e2e-testing.md` cites `archive/tickets/completed/S16S09GOLVAL-004.md` as the canonical travel-planning example for proving the selected path boundary directly
   - `docs/generated/golden-e2e-inventory.md` includes both spatial tests
5. This ticket duplicates already archived implementation history:
   - `archive/tickets/completed/S16S09GOLVAL-004.md` delivered the spatial golden itself
   - `archive/tickets/completed/S16S09GOLVAL-006.md` enriched the selected-plan trace with winning-search provenance and strengthened the spatial assertions
   - `archive/tickets/completed/S16S09GOLVAL-008.md` refactored the spatial helper boundaries without changing behavior
6. `golden_death_while_traveling` still exists in `crates/worldwake-ai/tests/golden_combat.rs`, and it is still insufficient for the branchy-hub contract because it is a combat/travel scenario, not the dedicated VillageSquare hub proof.
7. The claimed coverage gap is false. There is no missing focused/unit coverage and no missing golden/E2E coverage for this scenario in the current codebase.
8. The proposed file changes and new tests should not be implemented. Re-adding the scenario in another file would weaken discoverability, duplicate coverage, and create competing maintenance surfaces.
9. Corrected scope: archive this ticket as a stale duplicate of already delivered work instead of modifying code or tests.

## Architecture Check

1. The current architecture is cleaner than the proposal in this ticket. The spatial route proof belongs in `golden_ai_decisions.rs`, where needs-driven travel and planner-selection goldens already live.
2. The current assertion shape is also better than the proposal. The correct earliest proof boundary is the decision trace for selected path choice, with action trace and authoritative world state only as downstream confirmation. Reverting to a weaker "eventually arrived and harvested" proof would be a regression in test architecture.
3. No production or test-code changes are beneficial here. The ideal architecture is the one already in place: one canonical spatial golden, one canonical selected-plan trace surface, and no duplicate scenario copies in unrelated files.
4. No backwards-compatibility aliasing or duplicate helper paths should be introduced to preserve this stale ticket's original plan. If something needed to change, the existing canonical test should be updated directly.

## Verification Layers

1. Initial route selection from the live AI pipeline -> decision trace in `crates/worldwake-ai/tests/golden_ai_decisions.rs::assert_spatial_multi_hop_initial_selection`
2. Downstream OrchardFarm travel / harvest / hunger-relief chain -> authoritative world state plus action trace in `crates/worldwake-ai/tests/golden_ai_decisions.rs::assert_spatial_multi_hop_execution_outcomes`
3. Deterministic replay -> `golden_spatial_multi_hop_plan_replays_deterministically`
4. Docs alignment for the canonical scenario and proof surface -> `docs/golden-e2e-scenarios.md`, `docs/golden-e2e-testing.md`, and `docs/generated/golden-e2e-inventory.md`

## What To Change

1. Update this ticket to match the current repository state.
2. Do not modify production code.
3. Do not add or duplicate any tests.
4. Archive this ticket under a non-active archive folder because the requested work was already delivered elsewhere.

## Files To Touch

- `tickets/S16S09GOLVAL-005.md` (modify)

## Out Of Scope

- Any production planner/runtime changes
- Any new golden tests
- Moving the existing spatial goldens to another file
- Weakening the current decision-trace assertion boundary
- Updating `docs/golden-e2e-*` unless verification finds an actual mismatch

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan -- --exact`
2. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan_replays_deterministically -- --exact`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `python3 scripts/golden_inventory.py --write --check-docs`
6. `scripts/verify.sh`

### Invariants

1. The canonical spatial golden remains singular and lives in `golden_ai_decisions.rs`
2. The selected-path proof remains decision-trace-first rather than arrival-only
3. No duplicate tests or alternate compatibility paths are introduced
4. Golden docs remain aligned with the live inventory

## Tests

### New/Modified Tests

1. None. Reassessment confirmed the needed tests already exist and already cover the scenario at the correct architectural boundary.

### Commands

1. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan -- --exact`
2. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan_replays_deterministically -- --exact`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `python3 scripts/golden_inventory.py --write --check-docs`
6. `scripts/verify.sh`

## Outcome

- Outcome date: 2026-03-21
- What actually changed:
  - corrected the ticket assumptions to match the current repository state
  - narrowed the scope from "implement missing spatial golden" to "archive stale duplicate ticket"
  - confirmed the existing spatial golden, deterministic replay companion, docs inventory, and repo verification baseline all pass
- Deviations from original plan:
  - no code or test changes were made because the requested coverage was already delivered by `S16S09GOLVAL-004`, then refined by `-006` and `-008`
  - the ticket is archived as `NOT IMPLEMENTED` rather than `COMPLETED` to avoid falsely claiming new implementation work happened under this ticket
- Verification results:
  - `cargo test -p worldwake-ai golden_spatial_multi_hop_plan -- --exact` passed
  - `cargo test -p worldwake-ai golden_spatial_multi_hop_plan_replays_deterministically -- --exact` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `python3 scripts/golden_inventory.py --write --check-docs` passed
  - `scripts/verify.sh` passed
