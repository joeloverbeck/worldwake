# S177WATSRCQUA-010: 1440-tick collision golden — `survival-quality-degrading-1440`

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: No — scenario authoring + golden harness only
**Deps**: `archive/tickets/S177WATSRCQUA-001.md`, `archive/tickets/S177WATSRCQUA-002.md`, `archive/tickets/S177WATSRCQUA-003.md`, `archive/tickets/S177WATSRCQUA-004.md`, `archive/tickets/S177WATSRCQUA-005.md`, `archive/tickets/S177WATSRCQUA-006.md`, `archive/tickets/S177WATSRCQUA-009.md`

## Problem

The spec's Scenario Validation section requires a 1440-tick CI-owned collision scenario that exercises all S177 surfaces under sustained multi-agent pressure: several agents share a finite, regenerating clean water source plus a muddy backup; depletion drives fallback travel (already proven for the depletion-only case by `survival-basin-competition-1440.ron`); the muddy backup is observed on arrival; different `WaterToleranceProfile` per agent produces different choices; basin dirtiness rises from muddy-water refill; at least one critical-thirst window forms. This is the long-run soak test that catches integration regressions the focused goldens (ticket 009) cannot.

## Assumption Reassessment (2026-05-31)

1. Existing 1440-tick collision precedent: `scenarios/survival-basin-competition-1440.ron` paired with `crates/worldwake-ai/tests/scenarios/survival_basin_competition.rs`. This scenario already exercises multi-agent water competition at a depleted source — extends as the template for the new collision scenario.
2. Per `docs/golden-e2e-testing.md`, 1440-tick goldens are CI-owned (run in the full test suite, not just the cargo test crate filter) and serve as the soak/regression surface.
3. Differentiation from ticket 009's focused goldens: the focused goldens isolate one branch (one agent or one cross-system coupling at a time); the 1440-tick scenario exercises **all** S177 surfaces simultaneously: depletion + quality observation + per-agent tolerance diversity + basin dirtiness coupling + critical-window forensics.
4. Scenario shape:
   - 3-4 agents with diverse `WaterToleranceProfile` (hardy, default, fragile).
   - One Clean regenerating water source ("Riverside Well") with moderate `regeneration_ticks_per_unit` so multi-agent draw drains it under sustained pressure.
   - One Muddy regenerating water source ("Backup Spring") at a different place reachable via travel.
   - One wash-basin colocated with the Muddy source (so muddy-water refill is reachable and the dirtiness coupling fires).
   - 1440 tick duration. CI-owned (full suite).
5. Per `docs/precision-rules.md` Rule 7 (cumulative arithmetic): assertions on this scenario must state concrete deltas, cadences, thresholds, and capacity math that make the intended branches reachable. Authoritative survivability/non-survivability of each agent during the 1440-tick window must be verified before declaring the scenario reachable.
6. Per `docs/precision-rules.md` Rule 8 (scenario isolation): document the isolation choices — what unrelated lawful affordances are intentionally absent (e.g., no trade between agents, no sleep, no combat) to keep the branch under test sharp.
7. Adjacent contradictions: the existing `survival_basin_competition` scenario should be re-run as part of this ticket's verification to ensure no regression on the depletion-only baseline. Sibling scenarios at `scenarios/survival-basin-dirty-dirty.ron`, `scenarios/survival-sanitation-breakdown-1440.ron` may also exercise overlapping surfaces and should pass.
8. Per `docs/golden-e2e-testing.md`, 1440-tick scenarios commonly assert (a) aggregate behavior (counts of events, thresholds), (b) at least one critical window forms, (c) deterministic replay, (d) specific causal patterns (e.g., "the hardy agent drinks Muddy at least once; the fragile agent never drinks Muddy").

## Architecture Check

1. The 1440-tick scenario is the canonical FND-1 emergence proof — the scarcity ↔ quality tradeoff materializes from per-agent tolerance diversity + finite source draw + travel cost, not from authored sequence.
2. CI ownership (vs. cargo-test-only) means the scenario protects the spec's contract from broad workspace regressions, not just spec-local regressions.
3. The integration with existing `survival_basin_competition` and `survival_sanitation_breakdown_1440` ensures the new scenario complements rather than replaces existing coverage.

## Verification Layers

1. The 1440-tick scenario loads and completes — full scenario run produces no panics or assertion failures.
2. Per-agent choice diversity is observable: action-trace shows the hardy agent drinks Muddy at least N times during the run; the fragile agent never (or rarely) does.
3. At least one critical-thirst window forms during the run — `CriticalWindowReport` is produced.
4. Basin dirtiness rises from muddy-water refill — `WashBasinState.dirtiness_level > 0` at some point during the run.
5. Quality beliefs are acquired with provenance — `EventTag::ResourceSourceQualityObserved` events fire at expected ticks.
6. Replay equivalence: running the scenario twice with the same seed produces byte-identical event logs.
7. Existing 1440-tick scenarios still pass: `survival-basin-competition-1440`, `survival-sanitation-breakdown-1440`.

## What to Change

### 1. Author `scenarios/survival-quality-degrading-1440.ron`

Modeled on `scenarios/survival-basin-competition-1440.ron`. Specifics:

- Topology: 2 places — "Riverside Camp" (with Clean Well, regen interval ~12 ticks) and "Backup Camp" (with Muddy Spring, regen interval ~8 ticks) — connected by a travel edge of 30-50 ticks.
- Agents: 3 — "Aria" (hardy: `thirst_relief_factor[Muddy] = 800`, `dirtiness_penalty[Muddy] = 100`), "Bram" (default tolerance), "Cael" (fragile: `thirst_relief_factor[Muddy] = 300`, `dirtiness_penalty[Muddy] = 300`).
- Initial state: all 3 agents at Riverside Camp with high thirst.
- A wash-basin at Backup Camp so the muddy-refill→dirtiness coupling materializes.
- Duration: 1440 ticks.
- Authored agent beliefs: initial `SourceReliability` is empty (agents discover sources through perception).
- No trade, no combat, no sleep authored — scenario isolation.

### 2. Author `crates/worldwake-ai/tests/scenarios/survival_quality_degrading_1440.rs`

Test body with assertions covering each verification layer:

- `golden_survival_quality_degrading_1440_completes_1440_ticks_without_panic()` — basic load + run.
- `golden_survival_quality_degrading_1440_records_quality_beliefs()` — assert `EventTag::ResourceSourceQualityObserved` events are present in the log.
- `golden_survival_quality_degrading_1440_diverges_agents_by_tolerance()` — assert per-agent action-trace divergence (hardy drinks Muddy N+ times; fragile drinks Muddy ≤ M times where N >> M; document concrete N, M values per `docs/precision-rules.md` Rule 7).
- `golden_survival_quality_degrading_1440_produces_critical_window()` — assert at least one `CriticalWindowReport` for thirst on at least one agent.
- `golden_survival_quality_degrading_1440_raises_basin_dirtiness()` — assert `WashBasinState.dirtiness_level > Permille(0)` at some point.
- `golden_survival_quality_degrading_1440_replays_deterministically()` — byte-identical event log on rerun.

### 3. Regenerate golden inventory

```bash
python3 scripts/golden_inventory.py --write --check-docs
```

## Files to Touch

- `scenarios/survival-quality-degrading-1440.ron` (new)
- `crates/worldwake-ai/tests/scenarios/survival_quality_degrading_1440.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs` or equivalent (modify — register the new scenario module)
- `docs/generated/golden-e2e-inventory.md` (modify — regenerated)
- `docs/generated/golden-scenario-index.md` (modify — regenerated)
- `docs/generated/golden-scenario-details/` (modify — new detail file generated)

## Out of Scope

- Modifying existing 1440-tick scenarios — out of scope; this is a new scenario.
- Adding quality authoring to existing scenarios — out of scope; existing scenarios remain `quality: None`.
- Refactoring the golden harness — out of scope.
- Cross-scenario composition testing — out of scope; the 1440-tick soak is the broadest test surface in this spec.

## Acceptance Criteria

### Tests That Must Pass

1. New: 6 `golden_survival_quality_degrading_1440_*` tests covering completion, quality beliefs, agent divergence, critical window, basin dirtiness, replay equivalence.
2. Existing: `cargo test -p worldwake-ai golden_survival_basin_competition` passes — depletion-only baseline preserved.
3. Existing: `cargo test -p worldwake-ai` passes — full AI crate.
4. Existing: `./scripts/verify.sh` passes — full workspace including the 1440-tick scenario.

### Invariants

1. The 1440-tick scenario is deterministic — same seed produces same event log on every run.
2. Per-agent tolerance diversity produces lawful action-trace divergence — same world state, different choices.
3. At least one critical-thirst window forms during the 1440-tick run.
4. Basin dirtiness rises at least once during the run from muddy-water refill.
5. The scenario isolation choices (no trade, no combat, no sleep) are documented in the test body's preamble per `docs/precision-rules.md` Rule 8.

## Test Plan

### New/Modified Tests

1. One new RON scenario + one new scenario harness file with 6 golden tests.
2. Regenerated golden inventory documents.

### Commands

1. `cargo test -p worldwake-ai golden_survival_quality_degrading_1440` — targeted.
2. `cargo test -p worldwake-ai golden_survival_basin_competition` — regression coverage.
3. `cargo test -p worldwake-ai` — full AI crate.
4. `python3 scripts/golden_inventory.py --write --check-docs` — refresh inventory.
5. `./scripts/verify.sh` — full workspace.
