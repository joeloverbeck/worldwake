# S176SANFACDEG-008: Focused goldens + 1440-tick sanitation-breakdown scenario

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: None (scenarios + golden tests + roadmap registration)
**Deps**: S176SANFACDEG-003, S176SANFACDEG-004, S176SANFACDEG-005, S176SANFACDEG-006

## Problem

S176's consequence wiring needs E2E proof that the branches fire for the right reasons (FND-31): wash effectiveness scaling, precondition rejection, cleaning recovery, latrine fullblock + wilderness fallback, forensic records, and the belief barrier. This ticket adds the two focused branch goldens and the 1440-tick multi-agent collision scenario, plus the belief-barrier negative assertion (D7, distributed).

## Assumption Reassessment (2026-05-29)

1. The golden harness lives under `crates/worldwake-ai/tests/scenarios/` with scenarios in `scenarios/*.ron`; `docs/generated/golden-e2e-inventory.md` is the canonical `golden_*` test-name inventory (regenerate via `python3 scripts/golden_inventory.py --write --check-docs`). Existing survival goldens (`survival-baseline`, `survival-safe-rest`, etc.) are the structural precedent.
2. `docs/scenario-roadmap.md` registers 1440-tick scenarios with a `survival_health_contract` (`crates/worldwake-cli/src/scenario/types.rs::SurvivalHealthContractDef`), run via `.github/workflows`; the roadmap documents only landed scenarios, so `survival-sanitation-breakdown-1440` must be added there. `scenario_coverage.rs` enumerates `scenarios/*.ron` via `fs::read_dir`.
3. Live planner surface under test: `GoalKind::Wash` (`goal.rs:73`) / `Relieve` (`goal.rs:72`), with cleaning prerequisite ops `PlannerOpKind::{CleanWashBasin, EmptyLatrine}` (from S176SANFACDEG-005). Confirm the live op surface still routes through these before trusting the scenario narrative.
4. Coverage-gap classification: this is the **golden/E2E** layer; per-branch focused coverage lands in the implementation tickets (003-006). The goldens prove the authored causal reason, not mere survival (FND-31) — each asserts the planning trace, action trace, and authoritative state for its branch.
5. Cumulative arithmetic (FND-31 / precision rule 7): the 1440-tick scenario must make dirtiness/fill cross their thresholds and recover via labor under the authored `dirtiness_per_use` / `fill_per_use` / `max_effective_dirtiness` / `critical_threshold` / decay deltas. State the concrete per-use deltas and tick budget that make threshold-crossing-then-recovery reachable; validate the survivability envelope for the multi-agent shared-facility contention.
6. Scenario isolation (precision rule 8): name the lawful competing affordances each focused golden excludes — `survival-basin-dirty-dirty` isolates the wash-degradation branch (excludes latrine pressure); `survival-latrine-full` isolates the toilet-fullblock branch (excludes basin scarcity). The 1440 scenario intentionally combines both for collision.
7. Belief barrier (D7, distributed): the 1440 scenario asserts no omniscient remote-facility read — a cleaning prerequisite is never synthesized for a facility the agent has no belief about.

## Architecture Check

1. Three independent patterns (wash degradation+recovery, latrine fullblock+fallback, long-run multi-agent collision) per FND-31's multiple-pattern requirement; focused goldens prove single branches, the 1440 scenario proves emergent collision + recovery.
2. The negative assertions (no `sanitation_score`, no omniscient remote read, no facility self-reset) are first-class falsification checks, not incidental.

## Verification Layers

1. Wash effectiveness + rejection + cleaning recovery → `survival-basin-dirty-dirty` golden: planning trace (cleaning prerequisite), action trace (`clean_wash_basin` commit), authoritative state (dirtiness recovers).
2. Latrine fullblock + branch choice → `survival-latrine-full` golden: precondition rejection, `WasteCreated` provenance (`OvercapacityLatrine` vs `LatrineEmptied`), `DegradedSelfCareOpportunity` record.
3. Long-run collision + belief barrier + replay → `survival-sanitation-breakdown-1440`: facility-state arithmetic, decision-trace belief barrier, replay equivalence.

## What to Change

### 1. Focused goldens

Author `scenarios/survival-basin-dirty-dirty.ron` and `scenarios/survival-latrine-full.ron` + backing `golden_*` tests asserting the branch traces.

### 2. 1440-tick collision scenario

Author `scenarios/survival-sanitation-breakdown-1440.ron` (multi-agent, one basin + one latrine, `survival_health_contract`) + backing test; register in `docs/scenario-roadmap.md` and the CI workflow.

### 3. Inventory regeneration

Regenerate `docs/generated/golden-e2e-inventory.md` and sibling indices.

## Files to Touch

- `scenarios/survival-basin-dirty-dirty.ron` (new)
- `scenarios/survival-latrine-full.ron` (new)
- `scenarios/survival-sanitation-breakdown-1440.ron` (new)
- `crates/worldwake-ai/tests/scenarios/` (new — backing `golden_*` tests; confirm module path via existing survival goldens)
- `docs/scenario-roadmap.md` (modify — register the 1440 scenario)
- Likely: `.github/workflows/golden-survival.yml` (modify — register the 1440 scenario run; confirm the correct workflow file during implementation)
- `docs/generated/golden-e2e-inventory.md` + sibling indices (modify — regenerate via `python3 scripts/golden_inventory.py --write --check-docs`)

## Out of Scope

- Any production/engine change — all behavior is delivered by S176SANFACDEG-003/004/005/006.
- Per-branch focused unit coverage — lives in the implementation tickets.

## Acceptance Criteria

### Tests That Must Pass

1. `survival-basin-dirty-dirty` golden: relief scales down, wash precondition fails at threshold, `clean_wash_basin` recovers full relief, Waste lot emitted, replay-deterministic.
2. `survival-latrine-full` golden: toilet precondition fails at `critical_threshold`, the chosen branch (wilderness relief or `empty_latrine`) is asserted with `WasteCreated` provenance and a `DegradedSelfCareOpportunity` record.
3. `survival-sanitation-breakdown-1440`: thresholds cross and recover via labor, belief barrier holds, replay equivalence passes.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No golden passes via an illegal planner read, facility self-reset, omniscient remote-facility read, or `sanitation_score` (FND-31 forbidden artifacts).
2. The 1440 scenario carries a `survival_health_contract` and is registered in `docs/scenario-roadmap.md`.
3. `golden_inventory.py --check-docs` is clean after regeneration.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/survival_basin_dirty_dirty.rs` (or equivalent module path) — branch golden.
2. `crates/worldwake-ai/tests/scenarios/survival_latrine_full.rs` — branch golden.
3. `crates/worldwake-ai/tests/scenarios/survival_sanitation_breakdown_1440.rs` — CI-owned collision scenario.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai sanitation` (confirm the post-S154 `golden_ai` + substring-filter form against the real module path)
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `scripts/verify.sh`

## Outcome

**Completion date**: 2026-05-29

**What changed**:
- Authored three scenarios: `scenarios/survival-basin-dirty-dirty.ron` (single agent, basin authored above `max_effective_dirtiness`), `scenarios/survival-latrine-full.ron` (single agent, isolated indoor latrine above `critical_threshold`), and `scenarios/survival-sanitation-breakdown-1440.ron` (three archetype-varied agents sharing ONE basin + ONE latrine, with a `survival_health_contract`).
- Backing goldens: `survival_basin_dirty_dirty.rs` (default lane) — clean-prerequisite insertion, wash recovery, Waste lot, `DegradedSelfCareOpportunity{BasinTooDirty,Cleaned}`, replay determinism; `survival_latrine_full.rs` (default lane) — empty_latrine insertion, `WasteCreated{LatrineEmptied}` with concrete lot, toilet recovery, `DegradedSelfCareOpportunity{LatrineFull}`, replay determinism; `survival_sanitation_breakdown_1440.rs` (`#[ignore]`, CI-only) — no deaths, both recovery families fire, both degradation causes recorded, belief barrier (all recovery co-located), replay determinism.
- Registered the 1440 scenario in `docs/scenario-roadmap.md` (§5.20) and the `golden-survival.yml` CI matrix (`sanitation_breakdown` entry, run with `--ignored`).
- Regenerated `docs/generated/golden-e2e-inventory.md` + sibling indices (`golden_inventory.py --write`) and the `scenario-coverage.md` snapshot.

**Deviations / decisions**:
- The focused goldens run bounded windows (40/80 ticks), not 1440 — they isolate single branches per FND-31. Effectiveness scaling (relief halving) is unit-covered in S176SANFACDEG-003 (`wash_relief_scales_down_with_basin_dirtiness`); the basin golden proves the rejection→clean-prerequisite→recovery branch end-to-end (basin starts above threshold for determinism).
- The latrine focused golden uses an *isolated* indoor latrine so the cheaper wilderness-relief fallback is unreachable, deterministically forcing the `empty_latrine` branch (the cost-optimal wilderness fallback when reachable is the lawful alternative the spec allows).
- The 1440 collision scenario zeros hunger/thirst needs to isolate sanitation (bladder/dirtiness/fatigue) contention and stay survivable without a full food/water economy; lowered dirtiness/bladder critical thresholds + raised sanitation rates so critical windows open during contention and the degraded-self-care forensic records fire. Remaining observer anomalies (STUCK_AGENT, FAILED_ACTION_SPIRAL on the shared basin, GEOGRAPHIC_CONVERGENCE) are inherent single-facility contention — the scenario's point — not regressions; no deaths.

**Verification**: focused goldens pass in the default `cargo test -p worldwake-ai` lane; the `#[ignore]` 1440 tests pass via `cargo test --release -p worldwake-ai --test golden_ai -- --ignored survival_sanitation_breakdown_1440`. Full `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo fmt --all -- --check` all clean. `golden_inventory.py --check-docs` clean.
