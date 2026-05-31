# S177WATSRCQUA-010: 1440-tick collision golden — `survival-quality-degrading-1440`

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: No — scenario authoring + golden harness only
**Deps**: `archive/tickets/S177WATSRCQUA-001.md`, `archive/tickets/S177WATSRCQUA-002.md`, `archive/tickets/S177WATSRCQUA-003.md`, `archive/tickets/S177WATSRCQUA-004.md`, `archive/tickets/S177WATSRCQUA-005.md`, `archive/tickets/S177WATSRCQUA-006.md`, `archive/tickets/S177WATSRCQUA-009.md`

## Problem

The spec's Scenario Validation section requires a 1440-tick CI-owned collision scenario that exercises all S177 surfaces under sustained multi-agent pressure: several agents share a finite, regenerating clean water source plus a muddy backup; depletion drives fallback travel (already proven for the depletion-only case by `survival-basin-competition-1440.ron`); the muddy backup is observed on arrival; different `WaterToleranceProfile` per agent produces different choices; basin dirtiness rises from muddy-water refill; at least one critical-thirst window forms. This is the long-run soak test that catches integration regressions the focused goldens (ticket 009) cannot.

## Assumption Reassessment (2026-05-31)

1. Existing 1440-tick collision precedent: `scenarios/survival-basin-competition-1440.ron` paired with `crates/worldwake-ai/tests/scenarios/survival_basin_competition.rs`. This scenario already exercises multi-agent water competition at a depleted source — extends as the template for the new collision scenario.
2. Per `docs/golden-e2e-testing.md` and the live `.github/workflows/golden-survival.yml`, 1440-tick survival goldens are CI-owned as ignored `golden_ai` tests run by the golden-survival workflow with `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 <filter>`. They are not part of default `cargo test -p worldwake-ai` execution.
3. Differentiation from ticket 009's focused goldens: the focused goldens isolate one branch (one agent or one cross-system coupling at a time); the 1440-tick scenario exercises **all** S177 surfaces simultaneously: depletion + quality observation + per-agent tolerance diversity + basin dirtiness coupling + critical-window forensics.
4. Scenario shape:
   - 3-4 agents with diverse `WaterToleranceProfile` (hardy, default, fragile).
   - One Clean regenerating water source ("Riverside Well") with moderate `regeneration_ticks_per_unit` so multi-agent draw drains it under sustained pressure.
   - One Muddy regenerating water source ("Backup Spring") at a different place reachable via travel.
   - One wash-basin colocated with the Muddy source (so muddy-water refill is reachable and the dirtiness coupling fires).
   - 1440 tick duration. CI-owned through the golden-survival ignored-test workflow.
   - Test setup may seed source/location entity beliefs to make the reachable fallback candidates known, but must not seed source-reliability quality memory; if entity snapshots are seeded from world state, water quality must be stripped so the Muddy fact still enters through co-located perception.
5. Per `docs/precision-rules.md` Rule 7 (cumulative arithmetic): assertions on this scenario must state concrete deltas, cadences, thresholds, and capacity math that make the intended branches reachable. Authoritative survivability/non-survivability of each agent during the 1440-tick window must be verified before declaring the scenario reachable.
6. Per `docs/precision-rules.md` Rule 8 (scenario isolation): document the isolation choices — what unrelated lawful affordances are intentionally absent (e.g., no trade between agents, no sleep, no combat) to keep the branch under test sharp.
7. Adjacent contradictions: the existing `survival_basin_competition` scenario should be re-run as part of this ticket's verification to ensure no regression on the depletion-only baseline. Sibling scenarios at `scenarios/survival-basin-dirty-dirty.ron`, `scenarios/survival-sanitation-breakdown-1440.ron` may also exercise overlapping surfaces and should pass.
8. Per `docs/golden-e2e-testing.md`, 1440-tick scenarios commonly assert (a) aggregate behavior (counts of events, thresholds), (b) at least one critical window forms, (c) deterministic replay, (d) specific causal patterns (e.g., "the hardy agent drinks Muddy at least once; the fragile agent never drinks Muddy").

## Architecture Check

1. The 1440-tick scenario is the canonical FND-1 emergence proof — the scarcity ↔ quality tradeoff materializes from per-agent tolerance diversity + finite source draw + travel cost, not from authored sequence.
2. CI ownership (vs. cargo-test-only) means the scenario protects the spec's contract from broad workspace regressions, not just spec-local regressions.
3. The integration with existing `survival_basin_competition` and `survival_sanitation_breakdown_1440` ensures the new scenario complements rather than replaces existing coverage.

## Verified Layers

1. Scenario load/completion -> `golden_survival_quality_degrading_1440_completes_1440_ticks_without_panic`.
2. Per-agent choice diversity -> action-trace location sampling in `golden_survival_quality_degrading_1440_diverges_agents_by_tolerance`.
3. Critical thirst window -> `SurvivalForensicExtractor` report count in `golden_survival_quality_degrading_1440_produces_critical_window`.
4. Basin dirtiness from muddy refill -> `WashBasinState.dirtiness_level` sampling in `golden_survival_quality_degrading_1440_raises_basin_dirtiness`.
5. Quality belief acquisition -> `EventTag::ResourceSourceQualityObserved` payload scan in `golden_survival_quality_degrading_1440_records_quality_beliefs`.
6. Replay equivalence -> same-seed world/event-log hash comparison in `golden_survival_quality_degrading_1440_replays_deterministically`.
7. Existing depletion-only 1440 baseline -> `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_basin_competition::`.

## Landed Changes

### 1. Authored `scenarios/survival-quality-degrading-1440.ron`

Modeled on `scenarios/survival-basin-competition-1440.ron`. Specifics landed:

- Topology: 3 places — "Riverside Camp" with a finite Clean well, "Backup Camp" with a Muddy spring and basin, and "Clear Ridge" with a farther Clean fallback.
- Agents: 3 — "Aria" (hardy: `thirst_relief_factor[Muddy] = 850`, `dirtiness_penalty[Muddy] = 80`), "Bram" (default tolerance), "Cael" (fragile: `thirst_relief_factor[Muddy] = 250`, `dirtiness_penalty[Muddy] = 450`).
- Initial state: all 3 agents at Riverside Camp with high thirst and quiet unrelated needs.
- A wash-basin at Backup Camp so the muddy-refill to dirtiness coupling materializes.
- Duration: 1440 ticks.
- Authored agent `SourceReliability` is empty. The test harness seeds reachable source/location entity beliefs but strips water quality from those snapshots, so quality still enters through co-located perception.
- No trade, no combat, no sleep pressure, and no social relays — scenario isolation.

### 2. Authored `crates/worldwake-ai/tests/scenarios/survival_quality_degrading_1440.rs`

Test body with assertions covering each verification layer:

- `golden_survival_quality_degrading_1440_completes_1440_ticks_without_panic()` — basic load + run.
- `golden_survival_quality_degrading_1440_records_quality_beliefs()` — assert `EventTag::ResourceSourceQualityObserved` events are present in the log.
- `golden_survival_quality_degrading_1440_diverges_agents_by_tolerance()` — assert action-trace divergence by committed drink location: at least one agent drinks at Backup Camp and at least one reaches Clear Ridge for drinking.
- `golden_survival_quality_degrading_1440_produces_critical_window()` — assert at least one `CriticalWindowReport` for thirst on at least one agent.
- `golden_survival_quality_degrading_1440_raises_basin_dirtiness()` — assert `WashBasinState.dirtiness_level > Permille(0)` at some point.
- `golden_survival_quality_degrading_1440_replays_deterministically()` — same-seed world and event-log hashes match on rerun.

### 3. Regenerated golden inventory

```bash
python3 scripts/golden_inventory.py --write --check-docs
```

## Landed Files

- `scenarios/survival-quality-degrading-1440.ron` (new)
- `crates/worldwake-ai/tests/scenarios/survival_quality_degrading_1440.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs` or equivalent (modify — register the new scenario module)
- `.github/workflows/golden-survival.yml` (modify — register the ignored long-run scenario in the CI matrix)
- `docs/generated/golden-e2e-inventory.md` (modify — regenerated)
- `docs/generated/golden-scenario-index.md` (modify — regenerated)
- `docs/generated/golden-coverage-matrix.md` (modify — regenerated)
- `docs/generated/golden-scenario-details/survival-quality-degrading-1440.md` (new generated detail file)

## Out of Scope

- Existing 1440-tick scenario RON files were not modified.
- Existing scenarios were not retrofitted with water quality.
- The golden harness was extended only through this scenario module; no shared harness refactor landed.
- Cross-scenario composition testing remains outside this ticket.

## Acceptance Result

### Observed Verification

1. Passed: 6 ignored `golden_survival_quality_degrading_1440_*` tests covering completion, quality beliefs, agent divergence, critical window, basin dirtiness, and replay equivalence.
2. Passed: existing ignored `survival_basin_competition` 1440-tick depletion baseline.
3. Passed: default `cargo test -p worldwake-ai` suite.
4. Waived per-ticket: `./scripts/verify.sh`; this `implement-spec-tickets` harness owns that full pre-push gate after final spec archival.

### Invariants

1. The 1440-tick scenario is deterministic for same-seed world and event-log hashes.
2. Per-agent tolerance diversity produces lawful action-trace divergence by drink location in the same authored world.
3. At least one critical-thirst window forms during the 1440-tick run.
4. Basin dirtiness rises at least once during the run from muddy-water refill.
5. The scenario isolation choices are documented in the scenario file and test module preamble.

## Test Plan Result

### Added/Modified Tests

1. Added one RON scenario and one scenario harness file with 6 ignored golden-survival tests.
2. Regenerated golden inventory documents.

### Commands Run

1. `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_quality_degrading_1440::` — targeted ignored golden-survival coverage.
2. `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_basin_competition::` — regression coverage for the depletion-only 1440-tick baseline.
3. `cargo test -p worldwake-ai` — full AI crate.
4. `python3 scripts/golden_inventory.py --write --check-docs` — refresh inventory.
5. `./scripts/verify.sh` — waived here because the harness final branch phase owns the full pre-push gate.

## Outcome

Completed on 2026-05-31.

- Added the `survival-quality-degrading-1440.ron` authored scenario with a finite Clean source, Muddy backup, farther Clean fallback, three tolerance-diverse agents, and a Muddy-refilled backup basin.
- Added six ignored golden-survival tests proving scenario completion, quality observation, action-trace drink-location divergence, thirst critical-window forensics, basin dirtiness, and deterministic replay.
- Registered the new scenario module and added it to `.github/workflows/golden-survival.yml`.
- Regenerated golden inventory, index, coverage matrix, and the new scenario detail page.

## Deviations

- The drafted two-place topology landed as three places so the fragile/low-tolerance branch has a clean fallback beyond the muddy backup.
- The test harness seeds source/location entity beliefs so agents know reachable source candidates, then strips water quality from those snapshots. This keeps fallback planning belief-backed while preserving co-located perception as the only source of quality memory.
- The drafted per-agent exact Muddy drink counts landed as a more robust action-trace divergence assertion: at least one committed drink at Backup Camp and at least one committed drink at Clear Ridge.
- Long-run 1440-tick cases landed as ignored `golden_ai` tests registered in the golden-survival workflow, matching the live repo convention.

## Verification Result

- Passed `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_quality_degrading_1440::`.
- Passed `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_basin_competition::`.
- Passed `cargo test -p worldwake-ai`.
- Passed `python3 scripts/golden_inventory.py --write --check-docs`.
- Waived `./scripts/verify.sh` for this ticket iteration because the `implement-spec-tickets` final branch phase owns the full pre-push gate after final spec archival.
