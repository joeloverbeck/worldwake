# S177WATSRCQUA-009: Focused goldens — water-quality-on-arrival, dirty-water-tolerance-tradeoff, muddy-basin-refill

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: No — scenario authoring + golden harness only
**Deps**: `archive/tickets/S177WATSRCQUA-001.md`, `archive/tickets/S177WATSRCQUA-002.md`, `archive/tickets/S177WATSRCQUA-003.md`, `archive/tickets/S177WATSRCQUA-004.md`, `archive/tickets/S177WATSRCQUA-005.md`, `archive/tickets/S177WATSRCQUA-006.md`

## Problem

The spec's Scenario Validation section (focused branch goldens) requires three new branch goldens that prove the quality axis end-to-end: (a) an agent's quality belief updates on arrival when a believed-clean source turns out to be muddy; (b) two agents with different `WaterToleranceProfile` make different drink-vs-travel choices given identical world state; (c) basin refill from a muddy source raises basin dirtiness and degrades wash effectiveness. Without these goldens, the quality-axis behavioral contract is unverified — a regression in any of tickets 001-006 could pass `verify.sh` silently. These goldens are mandatory per spec FND-31 alignment.

## Assumption Reassessment (2026-05-31)

1. Existing golden harness pattern: `crates/worldwake-ai/tests/scenarios/survival_*.rs` paired with `scenarios/survival-*.ron`. Precedent: `survival_basin_competition.rs` ↔ `survival-basin-competition-1440.ron`, `survival_baseline.rs` ↔ `survival-baseline.ron`. New scenarios follow the same naming convention.
2. The depletion half of canonical scenario D is proven by existing goldens (named in spec): `ai_decisions.rs::golden_local_depleted_source_regenerates_without_spurious_failure_memory`, `source_composite.rs::empty_but_fresh_observation_demotes_depleted_source`, `survival_preferences.rs::*`. The new goldens complement (not replace) these.
3. Per `docs/golden-e2e-testing.md` (canonical golden authoring guide), new goldens must declare invariants, name the intended branch under test, document scenario isolation choices, and prove the causal reason (not just structural activation).
4. Per `docs/generated/golden-e2e-inventory.md`, after authoring new goldens, run `python3 scripts/golden_inventory.py --write --check-docs` to refresh the inventory.
5. Per ticket 003's universal-profile contract, agents can be authored with explicit `water_tolerance_profile:` overrides in RON — this is essential for the tolerance-tradeoff golden where two agents differ in tolerance.
6. The "muddy on arrival" golden exercises:
   - Initial belief: agent believes source is Clean (test-harness seeded `SourceReliability` with `last_observed_quality: Some(Clean)`, fresh).
   - World truth: source is actually Muddy (RON-authored `ResourceSource.quality: Some(Muddy)`).
   - Travel + arrival + observation: agent travels to source, perception writes `last_observed_quality: Some(Muddy)` on the agent's `SourceReliability`, emits `EventTag::ResourceSourceQualityObserved`.
   - Branch: agent's next-tick choice depends on whether tolerance + cost favor drinking muddy vs. traveling to a fallback. The golden asserts either a deterministic branch (per the authored seed) or both branches lawful with explicit isolation choice.
7. The "dirty-water-tolerance-tradeoff" golden uses two agents with different `WaterToleranceProfile`:
   - Agent A (hardy): `thirst_relief_factor[Muddy] = 1000`, `dirtiness_penalty[Muddy] = 0`; muddy water is neutral for this agent, so lower local/source tiebreakers can choose the colocated muddy source rather than paying travel to a clean fallback.
   - Agent B (fragile): `thirst_relief_factor[Muddy] = 100`, `dirtiness_penalty[Muddy] = 500`; the quality-aware `SourceComposite` discount makes the clean fallback win.
   - Identical world and beliefs: one local Muddy source and one remote Clean source, both fresh in `SourceReliability`.
   - Expected emergent divergence: agent A selects the local muddy source, while agent B selects the clean fallback. The strongest stable proof boundary is the decision-trace selected source and source-composite comparison, not a later action-trace Drink/Travel commit.
8. The "muddy-basin-refill" golden chains tickets 006 + S176:
   - Basin has only a muddy water source colocated.
   - Refill happens (ticket 006 logic), `dirtiness_level` rises by `dirty_water_refill_penalty[Muddy]`.
   - Subsequent wash attempt has reduced effectiveness via S176's `max_effective_dirtiness` gate.
   - Golden asserts the chained behavior.
9. Adjacent contradictions: existing scenarios (`survival-basin-competition-1440.ron`) have water sources without `quality:` authoring, so they default to `quality: None` after ticket 001 — `quality: None` is treated as Clean-equivalent throughout (no penalty, full relief). Existing goldens are unaffected.
10. RON scenario file shape: `scenarios/survival-water-quality-on-arrival.ron` etc. New `WaterQuality` enum variants serialize as `Clean`, `Stale`, `Muddy` via the standard serde enum representation. Cross-reference `scenarios/survival-basin-competition-1440.ron:377-380` for source authoring conventions.

## Architecture Check

1. Per `docs/golden-e2e-testing.md`: every golden asserts a specific causal branch with declared invariants and isolation choices. The three new goldens each name (a) the intended branch, (b) the isolation choice (e.g., "only one alternative fallback authored to prevent search-space ambiguity"), (c) the proof surface (decision-trace + action-trace + event-log delta).
2. The tolerance-diversity golden is the cleanest possible FND-22 emergence demonstration in test form — same world, different agents, different choices.
3. The chained basin-refill→wash-effectiveness golden proves the cross-ticket coupling without requiring a 1440-tick scenario (which is ticket 010's job).

## Verified Layers

1. Each new RON scenario loads without error — `cargo test scenario_loading` or equivalent existing harness coverage.
2. Each new golden body asserts specific invariants:
   - water-quality-on-arrival: pre-arrival event-log absence proves no omniscient correction; post-arrival `SourceReliability` stores `Some(Muddy)`; `ResourceSourceQualityObserved` event is emitted at/after arrival.
   - dirty-water-tolerance-tradeoff: same world state and source beliefs are seeded; agent A's decision trace selects the local Muddy source; agent B's decision trace selects the Clean fallback and records `RankedGoalComparisonDimension::SourceComposite`.
   - muddy-basin-refill: world-state delta shows `WashBasinState.dirtiness_level` raised by the muddy refill penalty; subsequent Wash commits and leaves partial rather than full dirtiness relief.
3. Replay equivalence: the arrival and tolerance scenario families each include deterministic replay coverage over canonical world + event-log hashes. The basin-refill family is deterministic through the focused refill/wash assertions and the broader `worldwake-ai` crate run.

## Landed Changes

### 1. Author `scenarios/survival-water-quality-on-arrival.ron`

Standard RON shape modeled on `scenarios/survival-basin-competition-1440.ron`. Single agent with default `WaterToleranceProfile`. Two water sources: one named "Believed Well" with `quality: Some(Muddy)`, and one named "Backup Spring" with `quality: Some(Clean)`. The golden harness seeds a recent `SourceReliability` belief claiming "Believed Well" was Clean before the run. Critical thirst pressure rises; minimal other affordances isolate the branch under test.

### 2. Author `scenarios/survival-dirty-water-tolerance-tradeoff.ron`

Two agents at the same place. Agent A's `water_tolerance_profile.thirst_relief_factor[Muddy] = 1000` and `dirtiness_penalty[Muddy] = 0`; agent B's `thirst_relief_factor[Muddy] = 100` and `dirtiness_penalty[Muddy] = 500`. One Muddy water source is colocated; one Clean source is at a distant place. Both agents have critical thirst pressure and the same seeded source beliefs.

### 3. Author `scenarios/survival-muddy-basin-refill.ron`

One wash-basin with `dirtiness_level: Permille(0)`. One Muddy water source colocated. One agent with critical dirtiness need. Scenario lets the basin refill run, then lets the agent attempt to wash, asserts wash effectiveness is reduced.

### 4. Author `crates/worldwake-ai/tests/scenarios/survival_water_quality_on_arrival.rs`

Test body following the precedent of `survival_basin_competition.rs`:

- `golden_survival_water_quality_on_arrival_records_belief_correction()` — runs the scenario, asserts the agent's `SourceReliability.sources[believed_well_key].last_observed_quality == Some(Muddy)` after the arrival tick (not before), and that `EventTag::ResourceSourceQualityObserved` is in the event log at the expected tick.
- `golden_survival_water_quality_on_arrival_emits_no_omniscient_belief_correction()` — negative assertion: pre-arrival ticks do NOT show belief correction.
- Deterministic replay assertion.

### 5. Author `crates/worldwake-ai/tests/scenarios/survival_dirty_water_tolerance_tradeoff.rs`

- `golden_dirty_water_tolerance_tradeoff_hardy_agent_drinks_muddy()` — Agent A's decision trace selects the local Muddy source.
- `golden_dirty_water_tolerance_tradeoff_fragile_agent_travels_to_fallback()` — Agent B's decision trace selects the Clean fallback and the decisive comparison dimension is `SourceComposite`.
- `golden_dirty_water_tolerance_tradeoff_replays_deterministically()`.

### 6. Author `crates/worldwake-ai/tests/scenarios/survival_muddy_basin_refill.rs`

- `golden_muddy_basin_refill_raises_dirtiness_level()` — world-state delta shows `dirtiness_level` raised by `dirty_water_refill_penalty[Muddy]` post-refill.
- `golden_muddy_basin_refill_degrades_wash_effectiveness()` — subsequent wash relief is reduced relative to a clean-water-refilled baseline.

### 7. Regenerate golden inventory

```bash
python3 scripts/golden_inventory.py --write --check-docs
```

Include the regenerated `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and `docs/generated/golden-scenario-details/*` in the ticket's diff.

## Landed Files

- `scenarios/survival-water-quality-on-arrival.ron` (new)
- `scenarios/survival-dirty-water-tolerance-tradeoff.ron` (new)
- `scenarios/survival-muddy-basin-refill.ron` (new)
- `crates/worldwake-ai/tests/scenarios/survival_water_quality_on_arrival.rs` (new)
- `crates/worldwake-ai/tests/scenarios/survival_dirty_water_tolerance_tradeoff.rs` (new)
- `crates/worldwake-ai/tests/scenarios/survival_muddy_basin_refill.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs` or equivalent (modify — register the new scenario modules in the test binary's module tree)
- `docs/generated/golden-e2e-inventory.md` (modify — regenerated by `scripts/golden_inventory.py`)
- `docs/generated/golden-scenario-index.md` (modify — regenerated)
- `docs/generated/golden-scenario-details/` (modify — new per-scenario detail files generated)

## Out of Scope

- The 1440-tick collision scenario — owned by ticket 010.
- Modifying existing scenarios (`scenarios/survival-basin-competition-1440.ron` etc.) to author quality values — out of scope; existing scenarios remain `quality: None` to preserve current behavior.
- Refactoring the golden harness infrastructure — out of scope; new scenarios use the existing harness.

## Acceptance Result

### Tests Passed

1. New: all 8 `golden_*` tests across the three new scenario files (arrival: 3, tolerance tradeoff: 3, muddy basin refill: 2).
2. Existing: `cargo test -p worldwake-ai` passes — no regression in existing goldens.
3. Existing: `cargo test --workspace` passes.

### Invariants

1. Each new golden's authored causal branch is proven, not just structurally activated. Per `docs/golden-e2e-testing.md`, the assertion surface includes decision-trace + action-trace + event-log delta as appropriate.
2. Replay equivalence holds for the arrival and tolerance scenario families; the basin-refill chain is covered by deterministic focused state/action assertions and the affected crate run.
3. No new golden uses omniscient belief correction or short-circuits the perception pipeline.
4. The tolerance-diversity golden proves FND-22 — same world state, different per-agent profile, different choice. This is the decisive emergence demonstration for the spec's headline target pattern.

## Test Plan Result

### Added/Modified Tests

1. Three new RON scenarios + three new scenario harness files (see Files to Touch).
2. Regenerated golden inventory documents.

### Commands Run Or Waived

1. `cargo test -p worldwake-ai golden_survival_water_quality_on_arrival` — targeted.
2. `cargo test -p worldwake-ai golden_dirty_water_tolerance_tradeoff` — targeted.
3. `cargo test -p worldwake-ai golden_muddy_basin_refill` — targeted.
4. `cargo test -p worldwake-ai` — full AI crate suite (includes existing goldens for regression coverage).
5. `python3 scripts/golden_inventory.py --write --check-docs` — refresh inventory.
6. `./scripts/verify.sh` — full workspace.

## Outcome

Completed on 2026-05-31.

1. Added three authored scenario fixtures:
   - `scenarios/survival-water-quality-on-arrival.ron`
   - `scenarios/survival-dirty-water-tolerance-tradeoff.ron`
   - `scenarios/survival-muddy-basin-refill.ron`
2. Added three focused golden modules and registered them in `crates/worldwake-ai/tests/scenarios/mod.rs`:
   - `survival_water_quality_on_arrival.rs` — proves local arrival quality correction, no pre-arrival omniscient correction, and deterministic replay.
   - `survival_dirty_water_tolerance_tradeoff.rs` — proves same-world source-choice divergence from `WaterToleranceProfile`: hardy neutral-tolerance selects local Muddy; fragile discount selects remote Clean through `SourceComposite`.
   - `survival_muddy_basin_refill.rs` — proves muddy refill raises basin dirtiness and the later wash gives partial relief.
3. Regenerated golden inventory artifacts, including new detail pages for the three S177 focused golden files and expected line-reference/coverage-matrix churn.

## Deviations

1. The tolerance-tradeoff proof lands at the decision-trace selected-source boundary rather than waiting for later action commits. This is the stronger stable boundary for the authored invariant because it proves the planner-facing source choice directly.
2. The hardy agent uses neutral Muddy tolerance (`1000‰` relief, `0‰` dirtiness penalty) rather than a positive boost. The live source-composite model discounts lower-quality water; it does not boost Muddy above Clean. Divergence comes from hardy neutrality allowing local/source tiebreakers to choose Muddy while fragile tolerance makes `SourceComposite` choose the Clean fallback.

## Verification Result

1. Passed `cargo test -p worldwake-ai golden_survival_water_quality_on_arrival`.
2. Passed `cargo test -p worldwake-ai golden_dirty_water_tolerance_tradeoff`.
3. Passed `cargo test -p worldwake-ai golden_muddy_basin_refill`.
4. Passed `python3 scripts/golden_inventory.py --write --check-docs`.
5. Passed `cargo test -p worldwake-ai`.
6. Passed `cargo test --workspace --quiet`.
7. Waived `./scripts/verify.sh` for this per-ticket closeout because the `implement-spec-tickets` harness owns the final pre-push verification gate after the full S177 ticket family lands.
