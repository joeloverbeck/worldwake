# S177WATSRCQUA-009: Focused goldens — water-quality-on-arrival, dirty-water-tolerance-tradeoff, muddy-basin-refill

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: No — scenario authoring + golden harness only
**Deps**: `archive/tickets/S177WATSRCQUA-001.md`, `archive/tickets/S177WATSRCQUA-002.md`, `archive/tickets/S177WATSRCQUA-003.md`, `archive/tickets/S177WATSRCQUA-004.md`, `archive/tickets/S177WATSRCQUA-005.md`, `tickets/S177WATSRCQUA-006.md`

## Problem

The spec's Scenario Validation section (focused branch goldens) requires three new branch goldens that prove the quality axis end-to-end: (a) an agent's quality belief updates on arrival when a believed-clean source turns out to be muddy; (b) two agents with different `WaterToleranceProfile` make different drink-vs-travel choices given identical world state; (c) basin refill from a muddy source raises basin dirtiness and degrades wash effectiveness. Without these goldens, the quality-axis behavioral contract is unverified — a regression in any of tickets 001-006 could pass `verify.sh` silently. These goldens are mandatory per spec FND-31 alignment.

## Assumption Reassessment (2026-05-31)

1. Existing golden harness pattern: `crates/worldwake-ai/tests/scenarios/survival_*.rs` paired with `scenarios/survival-*.ron`. Precedent: `survival_basin_competition.rs` ↔ `survival-basin-competition-1440.ron`, `survival_baseline.rs` ↔ `survival-baseline.ron`. New scenarios follow the same naming convention.
2. The depletion half of canonical scenario D is proven by existing goldens (named in spec): `ai_decisions.rs::golden_local_depleted_source_regenerates_without_spurious_failure_memory`, `source_composite.rs::empty_but_fresh_observation_demotes_depleted_source`, `survival_preferences.rs::*`. The new goldens complement (not replace) these.
3. Per `docs/golden-e2e-testing.md` (canonical golden authoring guide), new goldens must declare invariants, name the intended branch under test, document scenario isolation choices, and prove the causal reason (not just structural activation).
4. Per `docs/generated/golden-e2e-inventory.md`, after authoring new goldens, run `python3 scripts/golden_inventory.py --write --check-docs` to refresh the inventory.
5. Per ticket 003's universal-profile contract, agents can be authored with explicit `water_tolerance_profile:` overrides in RON — this is essential for the tolerance-tradeoff golden where two agents differ in tolerance.
6. The "muddy on arrival" golden exercises:
   - Initial belief: agent believes source is Clean (RON-authored `SourceReliability` seed with `last_observed_quality: Some(Clean)`, fresh).
   - World truth: source is actually Muddy (RON-authored `ResourceSource.quality: Some(Muddy)`).
   - Travel + arrival + observation: agent travels to source, perception writes `last_observed_quality: Some(Muddy)` on the agent's `SourceReliability`, emits `EventTag::ResourceSourceQualityObserved`.
   - Branch: agent's next-tick choice depends on whether tolerance + cost favor drinking muddy vs. traveling to a fallback. The golden asserts either a deterministic branch (per the authored seed) or both branches lawful with explicit isolation choice.
7. The "dirty-water-tolerance-tradeoff" golden uses two agents with different `WaterToleranceProfile`:
   - Agent A (hardy): `thirst_relief_factor[Muddy] = 800`, `dirtiness_penalty[Muddy] = 100`.
   - Agent B (fragile): `thirst_relief_factor[Muddy] = 300`, `dirtiness_penalty[Muddy] = 300`.
   - Identical world: clean well depleted, only muddy source available; clean fallback exists but requires travel.
   - Expected emergent divergence: agent A drinks muddy, agent B travels to fallback. Deterministic replay required.
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

## Verification Layers

1. Each new RON scenario loads without error — `cargo test scenario_loading` or equivalent existing harness coverage.
2. Each new golden body asserts specific invariants:
   - water-quality-on-arrival: decision-trace shows pre-arrival belief = Clean; post-arrival belief = Muddy; `ResourceSourceQualityObserved` event emitted at arrival tick; agent's subsequent action is either Drink (if tolerance + pressure favors) or Travel (otherwise).
   - dirty-water-tolerance-tradeoff: same world state seeded; agent A's action-trace shows Drink at Muddy; agent B's shows Travel-to-fallback; replay-equivalence asserted.
   - muddy-basin-refill: action-trace shows basin refill from Muddy source; world-state delta shows `dirtiness_level` raised by the penalty; subsequent wash has lower relief.
3. Replay equivalence: each golden runs deterministically with seeded `ChaCha8Rng`; running the same scenario twice produces byte-identical event logs.

## What to Change

### 1. Author `scenarios/survival-water-quality-on-arrival.ron`

Standard RON shape modeled on `scenarios/survival-basin-competition-1440.ron`. Single agent with default `WaterToleranceProfile`. Two water sources: one named "Believed Well" with `quality: Some(Muddy)` (but agent's `source_reliability` seed claims Clean), and one named "Backup Spring" with `quality: Some(Clean)`. Authored agent belief seeds `last_observed_quality: Some(Clean)` for "Believed Well" with a recent tick. Critical thirst pressure rising; minimal other affordances to isolate the branch under test.

### 2. Author `scenarios/survival-dirty-water-tolerance-tradeoff.ron`

Two agents at the same place. Agent A's `water_tolerance_profile.thirst_relief_factor[Muddy] = 800`; agent B's = 300. One Muddy water source colocated; one Clean source at a distant place. Both agents have critical thirst pressure.

### 3. Author `scenarios/survival-muddy-basin-refill.ron`

One wash-basin with `dirtiness_level: Permille(0)`. One Muddy water source colocated. One agent with critical dirtiness need. Scenario lets the basin refill run, then lets the agent attempt to wash, asserts wash effectiveness is reduced.

### 4. Author `crates/worldwake-ai/tests/scenarios/survival_water_quality_on_arrival.rs`

Test body following the precedent of `survival_basin_competition.rs`:

- `golden_survival_water_quality_on_arrival_records_belief_correction()` — runs the scenario, asserts the agent's `SourceReliability.sources[believed_well_key].last_observed_quality == Some(Muddy)` after the arrival tick (not before), and that `EventTag::ResourceSourceQualityObserved` is in the event log at the expected tick.
- `golden_survival_water_quality_on_arrival_emits_no_omniscient_belief_correction()` — negative assertion: pre-arrival ticks do NOT show belief correction.
- Deterministic replay assertion.

### 5. Author `crates/worldwake-ai/tests/scenarios/survival_dirty_water_tolerance_tradeoff.rs`

- `golden_dirty_water_tolerance_tradeoff_hardy_agent_drinks_muddy()` — Agent A's action trace shows Drink at Muddy source.
- `golden_dirty_water_tolerance_tradeoff_fragile_agent_travels_to_fallback()` — Agent B's action trace shows Travel to Clean source.
- `golden_dirty_water_tolerance_tradeoff_replays_deterministically()`.

### 6. Author `crates/worldwake-ai/tests/scenarios/survival_muddy_basin_refill.rs`

- `golden_muddy_basin_refill_raises_dirtiness_level()` — world-state delta shows `dirtiness_level` raised by `dirty_water_refill_penalty[Muddy]` post-refill.
- `golden_muddy_basin_refill_degrades_wash_effectiveness()` — subsequent wash relief is reduced relative to a clean-water-refilled baseline.

### 7. Regenerate golden inventory

```bash
python3 scripts/golden_inventory.py --write --check-docs
```

Include the regenerated `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and `docs/generated/golden-scenario-details/*` in the ticket's diff.

## Files to Touch

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

## Acceptance Criteria

### Tests That Must Pass

1. New: all 5+ `golden_*` tests across the three new scenario files (per scenario, 1-2 named branches + 1 replay-equivalence test).
2. Existing: `cargo test -p worldwake-ai` passes — no regression in existing goldens.
3. Existing: `cargo test --workspace` passes.

### Invariants

1. Each new golden's authored causal branch is proven, not just structurally activated. Per `docs/golden-e2e-testing.md`, the assertion surface includes decision-trace + action-trace + event-log delta as appropriate.
2. Replay equivalence holds for every new golden (deterministic with seeded `ChaCha8Rng`).
3. No new golden uses omniscient belief correction or short-circuits the perception pipeline.
4. The tolerance-diversity golden proves FND-22 — same world state, different per-agent profile, different choice. This is the decisive emergence demonstration for the spec's headline target pattern.

## Test Plan

### New/Modified Tests

1. Three new RON scenarios + three new scenario harness files (see Files to Touch).
2. Regenerated golden inventory documents.

### Commands

1. `cargo test -p worldwake-ai golden_survival_water_quality_on_arrival` — targeted.
2. `cargo test -p worldwake-ai golden_dirty_water_tolerance_tradeoff` — targeted.
3. `cargo test -p worldwake-ai golden_muddy_basin_refill` — targeted.
4. `cargo test -p worldwake-ai` — full AI crate suite (includes existing goldens for regression coverage).
5. `python3 scripts/golden_inventory.py --write --check-docs` — refresh inventory.
6. `./scripts/verify.sh` — full workspace.
