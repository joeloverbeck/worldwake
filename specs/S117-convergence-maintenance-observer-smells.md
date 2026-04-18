# S117: Convergence and Maintenance-Cycle Observer Smells

## Summary

Extend the observer binary with four new mechanical anomaly kinds that detect failure modes the current detector suite misses: agent population convergence on a single place, maintenance-need starvation (relief firing too infrequently relative to accumulation rate), single-source commodity dependency (recipe monoculture), and sub-threshold acute spikes (needs ≥ critical for 30–99 consecutive ticks, below the 100-tick sustained-critical bar). All four are read-only over authoritative event-log and component state. The spec also adjusts the existing `STUCK_AGENT` detector's stated tolerance note to match the refined precision behavior delivered by S118.

## Phase and Status

Phase 8 Adjunct: Survival Baseline Under Contention (post-`survival-contested.ron` report). Status: Draft.

## Crates

- `worldwake-cli` — observer binary (`crates/worldwake-cli/src/bin/observer.rs`, where `AnomalyKind`, the `Anomaly` struct, detectors, and the Section 3 renderer currently live as a single module): four new `AnomalyKind` label variants, new optional fields on the outer `Anomaly` struct for multi-agent rendering, four new detector functions, Section 3 rendering additions, Section 2 supplementary subsections, golden tests under `crates/worldwake-cli/tests/`.
- `worldwake-core` — no changes.
- `worldwake-sim` — no changes.
- `worldwake-ai` — no changes.
- `worldwake-systems` — no changes.

## Dependencies

- No simulation-side dependencies.
- `S116` is completed and archived at `archive/specs/S116-drive-escalation-sustained-critical.md`. With S116 landed, a clean `survival-contested.ron` re-run after S117 should show zero `MAINTENANCE_STARVATION` anomalies on the dirtiness need; any that fire signal remaining gaps.
- `S118` (`specs/S118-stuck-agent-detector-active-frame-exclusion.md`) refines the `STUCK_AGENT` detector. S117 and S118 are symmetric siblings: neither blocks the other's landing. S117's prose about `STUCK_AGENT` simply tracks whichever precision behavior is live when S117 merges.
- `.claude/skills/scenario-analysis/references/layer-1-behavioral-smells.md` and `.claude/skills/scenario-analysis/references/layer-3-meta-analysis.md`: once S117 lands, the "Known Pathology Signatures" list (Step 4) gains "Maintenance-cycle starvation" as a catalogued signature, and the "Step 6.4: Proposed New Smell Categories" template notes that smells 11/12/13 graduate from "proposed" to "shipped".

## Motivating Evidence

From `reports/scenario-analysis-report.md` Layer 3:

- **Gap 1 (Geographic Convergence, HIGH)**: 4 agents spent 78–84% of ticks at East Orchard; Spring Basin (the only wash site) saw 0–86 ticks per agent. No current detector surfaces this.
- **Gap 2 (Latrine Abandonment, MEDIUM)** — also surfaces through gap 4 (Wash-Cycle Starvation). 0 toilet actions across all 4 agents; 88 `relieve_wilderness` actions (each adding +200 permille dirtiness).
- **Gap 3 (Single-Source Food Monoculture, MEDIUM)**: Harvest Apples total = 64, Harvest Grain total = 0, despite every agent knowing both recipes.
- **Gap 4 (Wash-Cycle Starvation, MEDIUM)**: `relief_rate < accumulation_rate` over 200+ ticks. Smell 5 (Sustained Critical Needs) flags the symptom once it crosses 100 ticks, but the underlying frequency mismatch is invisible.
- **Gap 5 (Sub-Threshold Acute Spikes, LOW)**: Agent C (`dehydration_tolerance_ticks=220` per `scenarios/survival-contested.ron:386`) hit hunger=950 for 97 ticks, thirst=900 for 37 ticks. Both below the 100-tick sustained-critical threshold, both dangerous proximity to metabolism tolerance limits.

## Design Goals

1. Every detection gap enumerated above has a mechanical detector that fires on the current `survival-contested.ron` dump (for gaps 1, 2, 4, 5) and on any scenario producing the same surface signature.
2. Detectors run entirely inside the observer — no simulation-state writes, no event-log writes. Observer remains passive (FND-26, FND-12).
3. Each detector's threshold is a compile-time constant with a short justification comment, revisable without engine changes. Threshold revisions are observer-tool changes, not world-meaning changes.
4. Anomaly rendering in Section 3 matches the existing format so the `/scenario-analysis` skill can consume it without parser changes.
5. Each detector produces at most one anomaly entry per dedup key (detector-specific; see D2–D5) to prevent Section 3 flooding.
6. `AnomalyKind` remains a `Copy` label enum. Rich per-anomaly data lives on the outer `Anomaly` struct, preserving the existing rendering contract and avoiding migration of the six existing anomaly variants.

## Non-Goals

- Changing simulation behavior. These are detectors only.
- Auto-remediation. The detector emits an anomaly; responding to it is the analyst's (or `/scenario-analysis` skill's) job.
- LLM-gated detection. Every detector in this spec is mechanical and runs in the Rust observer binary.
- Refactoring the existing observer architecture. `bin/observer.rs` stays monolithic; new detectors slot into the same file alongside existing ones. A future extraction to a dedicated `src/observer/` module tree is a separate refactor.
- Adding a shared helper for recipe → need classification. The derivation (`CommodityKind::spec().trade_category` + `consumable_profile`) lives inline in the `RecipeMonoculture` detector. If a second consumer appears later, it can be promoted.
- Adding a `DriveThresholds::medium(need)` helper to `worldwake-core`. The detector performs the per-need match locally.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-12 (Performance May Compress Computation, Never Causality) | Observer detectors read the authoritative event log and agent component state. They do not alter world meaning. |
| FND-26 (Systems Interact Through State) | Detectors are pure reads of authoritative state and derived event-log scans. No simulation system depends on them. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | Anomaly entries in Section 3 are derived views over authoritative history. The recipe → need classification in D4 is derived inline from `RecipeDefinition` + `CommodityConsumableProfile`; it is not stored state. Deleting the observer output does not change the simulation. |
| FND-29 (Debuggability Is a Product Feature) | This spec's entire purpose: surface failure modes that are currently invisible. |
| FND-29A (Causal History Is Authoritative, Append-Only) | Detectors compute over the append-only event log. They do not mutate history. |

## Deliverables

### D1: `AnomalyKind` label variants and `Anomaly` struct extension

In `crates/worldwake-cli/src/bin/observer.rs` (current `AnomalyKind` definition at line ~729, current `Anomaly` struct nearby):

**`AnomalyKind`** — add four unit variants. The enum remains `#[derive(Clone, Copy)]`:

```rust
enum AnomalyKind {
    // ... existing variants: RedundantPerception, ActionLoop, StuckAgent,
    //                         FailedActionSpiral, SustainedCriticalNeed, UnaddressedNeed ...
    GeographicConvergence,
    MaintenanceStarvation,
    RecipeMonoculture,
    AcuteNeedSpike,
}
```

Extend `AnomalyKind::label()` with the four new labels: `"GEOGRAPHIC_CONVERGENCE"`, `"MAINTENANCE_STARVATION"`, `"RECIPE_MONOCULTURE"`, `"ACUTE_NEED_SPIKE"`.

**`Anomaly` struct** — add one new optional field to support multi-agent rendering (used by `GeographicConvergence`). The existing `agent_name: String`, `description: String`, and `tick_range: Option<(Tick, Tick)>` fields are unchanged:

```rust
struct Anomaly {
    kind: AnomalyKind,
    agent_name: String,              // existing; lead agent
    description: String,             // existing; rich quantitative content rendered as body
    tick_range: Option<(Tick, Tick)>,// existing
    // Added by S117:
    additional_agent_names: Option<Vec<String>>, // None for single-agent anomalies;
                                                 // Some(names) for multi-agent (agents sorted by EntityId)
}
```

All quantitative data (permille rates, recipe shares, peak values, commodity counts) is formatted into the `description` string at detection time. This keeps `AnomalyKind` `Copy` and leaves the six existing detectors untouched.

### D2: `GeographicConvergence` detector

**Logic**: Over the full run, for each 200-tick rolling window and each place, compute the share of each agent's ticks spent at that place. If 2+ agents each spend ≥ 60% of the window at the same place, emit one anomaly per `(agent-set, place)` with the window collapsed to the first qualifying window's start and the last qualifying window's end.

The agent-set for dedup is a `BTreeSet<EntityId>` (deterministic) materialized into the `agent_name` (lead = smallest EntityId's name) and `additional_agent_names` (remaining names, sorted by EntityId) fields on the `Anomaly` struct.

**Threshold justification**: 60% × 200 ticks = 120 ticks of shared occupancy. Below 60%, agents are rotating normally; above 60%, they are anchored. Live reassessment on `survival-baseline.ron` showed the threshold alone is not sufficient, because lawful split-support routing can still anchor multiple agents on a food-only node for well above 60% of a window. The landed observer contract therefore keeps the 60% bar for anchored overlap but also suppresses places that expose only one local survival-support family while complementary support clearly exists elsewhere. The threshold still catches anchored-hub patterns like `survival-contested.ron` without relying on scenario-name suppression.

**Dedup key**: `(GeographicConvergence, BTreeSet<EntityId> of agents, place_id)` — at most one anomaly per distinct agent-set × place across the whole run.

### D3: `MaintenanceStarvation` detector

**Logic**: For each agent and each `HomeostaticNeedId`, compute over a rolling 200-tick window:

- `accumulation_permille` — total positive delta of the need value across the window (sum of per-tick increases from metabolism + any action penalties).
- `relief_permille` — total negative delta of the need value across the window (sum of per-tick decreases from relief actions).
- `avg_need_permille` — simple mean of the per-tick need value across the window.

If `relief_permille < accumulation_permille` AND `avg_need_permille > medium` (the per-agent, per-need medium threshold; i.e., the need is chronically elevated), emit anomaly.

`medium` is read per-agent, per-need via the `DriveThresholds` component:

```rust
let thresholds = world
    .get_component_drive_thresholds(agent)
    .copied()
    .unwrap_or_default();
let medium = match need {
    HomeostaticNeedId::Hunger    => thresholds.hunger.medium(),
    HomeostaticNeedId::Thirst    => thresholds.thirst.medium(),
    HomeostaticNeedId::Fatigue   => thresholds.fatigue.medium(),
    HomeostaticNeedId::Bladder   => thresholds.bladder.medium(),
    HomeostaticNeedId::Dirtiness => thresholds.dirtiness.medium(),
};
```

(This mirrors the existing `DriveThresholds::critical(need)` helper's body; a `medium(need)` helper is intentionally not added to `worldwake-core` — the match stays local to the single detector call site.)

**Threshold justification**: Using the agent's own per-need medium threshold (FND-14) avoids global constants and respects per-agent variation. The rolling 200-tick window mirrors `GeographicConvergence` for consistency.

**Dedup key**: `(MaintenanceStarvation, agent, need)` — at most one anomaly per (agent × need) across the whole run. If the starvation persists across multiple windows, merge into a single anomaly with the combined span (first qualifying window start → last qualifying window end).

### D4: `RecipeMonoculture` detector

**Logic**: For each agent, at run end:

1. Read `KnownRecipes` (`crates/worldwake-core/src/production.rs:40`) for the agent.
2. For each known `RecipeId`, resolve the `RecipeDefinition` (`crates/worldwake-sim/src/recipe_def.rs:6`) and classify its primary satisfied need via the derivation below. Recipes with no consumable output (tools, weapons, waste) are excluded.
3. Bucket known-and-classified recipes by `HomeostaticNeedId`. For each bucket with ≥ 2 recipes, compute the per-recipe action-count share across the run (from the action trace / commit log already consumed by the observer). If the top recipe's share ≥ 95%, emit anomaly.
4. Belief-gate cross-check: only emit if at least one alternative recipe's required facility / workstation / resource source is present in the agent's final `AgentBeliefStore` at run end (from the same Section 5 belief data the observer already renders). This rules out the case where the agent genuinely never discovered where to execute the alternative.

**Recipe → need derivation (inline)**:

For a given `RecipeDefinition`, inspect `outputs[0]`. Resolve `CommodityKind::spec()`. If `consumable_profile` is `None`, the recipe has no consumable output and is excluded from monoculture analysis. Otherwise classify by trade category:

- `TradeCategory::Water` → `HomeostaticNeedId::Thirst`
- `TradeCategory::Food` → `HomeostaticNeedId::Hunger`
- all other trade categories → excluded from monoculture analysis

This landed rule is narrower than the original draft's "first non-zero relief field" ordering because live `CommodityKind::Apple` relieves both hunger and thirst; classifying by `TradeCategory` preserves the intended food-monoculture behavior for apples vs grain while still treating water recipes as thirst support. If a recipe has multiple outputs with different classifications, use the first output's classification (matches the current "primary output" convention elsewhere in the codebase). Fatigue and dirtiness reliefs do not come from recipes in the current model; such recipes do not exist today and are not special-cased.

This derivation is a pure read-side computation per FND-27. It lives as a private helper inside the detector module, not in `worldwake-sim`.

**Threshold justification**: 95% is a strict monoculture cutoff; anything above 90% is effectively single-recipe. The belief-gate prevents false positives for agents who know a recipe but never discovered the facility.

**Dedup key**: `(RecipeMonoculture, agent, need)` — at most one anomaly per (agent × need category) per run.

### D5: `AcuteNeedSpike` detector

**Logic**: For each agent and each `HomeostaticNeedId`, scan the per-tick need trajectory (already collected in `AgentStats.needs_samples` for Section 2) for maximal runs where `value >= critical_threshold` of length ≥ 30 and < 100 consecutive ticks. Emit one anomaly per run.

`critical_threshold` is read per-agent, per-need via the existing `DriveThresholds::critical(need)` method (`crates/worldwake-core/src/drives.rs:92`):

```rust
let critical = thresholds.critical(need);
```

**Threshold justification**: The 30-tick minimum filters out single-action transients. Wash takes 12 ticks (per `S118` Motivating Evidence and the wash action registration). Single-unit consume actions vary by `CommodityConsumableProfile.consumption_ticks_per_unit`, typically in the single-digit range. The 100-tick maximum is the existing `SUSTAINED_CRITICAL_NEED` cutoff — `AcuteNeedSpike` and `SUSTAINED_CRITICAL_NEED` are disjoint by construction; no double-flagging.

**Dedup key**: `(AcuteNeedSpike, agent, need, run_start_tick)` — one anomaly per maximal qualifying run. If two runs are separated by a single-tick gap, they remain distinct unless the detector chooses to merge; the reference implementation does not merge (treats gaps as real).

### D6: Section 3 rendering

Each new anomaly renders through the existing single render path (`bin/observer.rs:1696-1709`) with two small changes:

1. **Multi-agent header**: when `anomaly.additional_agent_names` is `Some(names)`, render the header as `### Anomaly N — KIND (agent_name, name1, name2, ...)` instead of the single-agent `### Anomaly N — KIND (agent_name)`. No other existing detectors use this field, so existing rendering is unchanged.
2. **Descriptions are pre-formatted**: all quantitative content (percentages, permille values, tick counts, commodity counts, recipe names) is interpolated into the `description` string at detection time.

Example outputs (body = `description` field):

```
### Anomaly 6 — GEOGRAPHIC_CONVERGENCE (Agent A, Agent B, Agent C, Agent D)

4 agents spent 78.3% of ticks 100–300 at East Orchard.

Tick range: 100–300
```

```
### Anomaly 7 — MAINTENANCE_STARVATION (Agent A)

dirtiness accumulated 385 permille but was relieved only 201 permille over ticks 400–600. Average dirtiness in window: 812 permille (above medium threshold 650).

Tick range: 400–600
```

```
### Anomaly 8 — RECIPE_MONOCULTURE (Agent A)

hunger actions: 100% Harvest Apples (16 actions), 0% Harvest Grain (0 actions). Both recipes known; final belief store includes workstation FieldPlot evidence.

Tick range: 0–1440
```

```
### Anomaly 9 — ACUTE_NEED_SPIKE (Agent C)

thirst above critical threshold (850 permille) for 40 consecutive ticks (ticks 0–39), peak 850 permille. Below the 100-tick sustained-critical bar but within 17% of dehydration tolerance (240 ticks).

Tick range: 0–39
```

### D7: Section 2 supplementary subsections

Add two small supplementary subsections to each agent's Section 2 block, immediately after the existing "Needs trajectory" / "Ticks above 750‰" / "Locations visited" / "Max consecutive idle ticks" blocks:

- **"Maintenance rates"** table: per need, accumulation permille (window-total), relief permille (window-total), net balance. One row per need. Provides the analyst with raw data to check the `MaintenanceStarvation` detector without re-deriving it. Uses the same 200-tick rolling-window convention as D3; the reported row uses the whole-run totals.
- **"Recipe usage"** table: per known recipe (from `KnownRecipes`), count of commits by that agent. Single line per recipe, in `RecipeId` order. If the agent has commits for a registry-backed recipe that is no longer present in current `KnownRecipes`, include a deterministic ` (unknown)` row rather than dropping the historical count.

Neither table requires new per-tick collection; both are aggregations over the `needs_samples` already collected for Section 2 and the action trace the observer already reads.

### D8: Golden coverage

Four new goldens in a new file `crates/worldwake-cli/tests/golden_observer_anomalies.rs`. Each golden drives the observer over a dedicated scenario fixture committed to `crates/worldwake-cli/tests/fixtures/observer_anomalies/` (one `.ron` scenario per golden, mirroring the existing production scenario layout in `scenarios/`). The existing observer `load_scenario_file` + `spawn_scenario` path remains the authoritative scenario-loading substrate, but the honest E2E test seam is the compiled `observer` binary itself, invoked from the integration test via `env!("CARGO_BIN_EXE_observer")` with a temp output path.

1. **`convergence_smell_fires_on_forced_hub_scenario`** — scripted scenario with 3 agents whose profiles, knowledge, and place layout make a single place the only viable option for 200+ consecutive ticks. Assert: Section 3 contains exactly one `GEOGRAPHIC_CONVERGENCE` anomaly covering the expected window and including all three agent names in the header.

2. **`maintenance_starvation_fires_on_wash_gap`** — scripted scenario with `wilderness_relief_dirtiness_penalty=200` (verified scenario-configurable via `MetabolismProfile` at `crates/worldwake-core/src/needs.rs:149` and `crates/worldwake-cli/src/scenario/types.rs`) and the wash facility several travel hops away. Assert: Section 3 contains `MAINTENANCE_STARVATION` for `Dirtiness` on each affected agent, with `accumulation_permille > relief_permille` in the description.

3. **`recipe_monoculture_fires_on_single_food_dependency`** — scripted scenario with an agent that has Harvest Apples + Harvest Grain knowledge plus a West Grainfield facility belief, but all food intake is apples. Assert: Section 3 contains `RECIPE_MONOCULTURE` for `Hunger` on that agent. Control case in the same test: a sibling agent that knows both recipes but never acquires the grainfield belief does NOT trigger the anomaly (belief-gate).

4. **`acute_need_spike_fires_on_bounded_thirst_run`** — scripted scenario forcing one bounded thirst-critical run followed by relief. Assert: Section 3 contains `ACUTE_NEED_SPIKE` with the bounded run rendered in the live report output, and no overlap with any `SUSTAINED_CRITICAL_NEED` entry.

Each fixture scenario is minimal (1–3 agents, 2–4 places) and reuses the existing scenario schema without new fields.

### D9: Skill-side integration (documentation only, no code)

After S117 lands, update the scenario-analysis skill references (NOT `.claude/skills/scenario-analysis/SKILL.md`, which is a thin entry point):

- **`.claude/skills/scenario-analysis/references/layer-1-behavioral-smells.md`** — under the "Known Pathology Signatures" section (line 41), add entries for the four new mechanical detectors so analysts know they fire mechanically rather than via Layer 3 LLM judgment.
- **`.claude/skills/scenario-analysis/references/layer-3-meta-analysis.md`** — under "Step 6.4: Proposed New Smell Categories" (line 79), note that smells 11/12/13 graduate from "proposed" to "shipped"; proposing new smells in a future report now means proposing additions to a successor spec.
- **`.claude/skills/scenario-analysis/references/report-templates.md`** — the "Proposed New Smell Categories" section (line 121) is documentation template only; no change required unless the template phrasing needs tightening.

This change is a post-implementation skill edit, not a code deliverable.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: All four detectors read the authoritative event log (action traces, ActionStarted/Committed pairs) and agent-component state (`HomeostaticNeeds` per-tick trajectory already captured in `AgentStats.needs_samples` for Section 2; `DriveThresholds` via `World::get_component_drive_thresholds`; `KnownRecipes`; belief entries per Section 5). No new information paths. FND-7 and FND-14 preserved — detectors operate on the observer's authoritative read view, not on any planner-facing belief state.

2. **Positive-feedback analysis**: None. Detectors do not modify any simulation state, produce no events, and do not feed back into agent behavior. The report-generation loop is purely read-side.

3. **Concrete dampeners**: Not applicable — no loops to dampen. The per-dedup-key anomaly limits prevent Section 3 output inflation but are output-format concerns, not world-state dampeners.

4. **Stored state vs. derived read-model**:
   - **Stored state**: none. This spec adds no authoritative components.
   - **Derived**: every anomaly entry is derived per-run from the event log + component trajectories. The D4 recipe → need classification is derived inline from `RecipeDefinition.outputs` + `CommodityKind::spec().consumable_profile`; it is not cached, not stored, and re-running the observer on the same dump produces the same anomalies.

## SystemFn Integration

None. No SystemFn changes. Observer is a post-run read tool.

## Component Registration

None. No new ECS components.

## Cross-System Interactions (FND-26)

No simulation system consumes the observer's output. Cross-observer interactions:

- Detectors read the same authoritative event log and agent components that Section 2 already summarizes — shared read substrate, not cross-component coupling.
- `STUCK_AGENT` detector behavior is refined by `specs/S118-stuck-agent-detector-active-frame-exclusion.md`. S117 does not re-specify that detector; it merely tracks whichever precision behavior is live when S117 merges.
- The D4 recipe → need derivation reads `CommodityKind::spec()` (a `const fn` lookup in `worldwake-core/src/items.rs`). This is a static-spec read, not a runtime cross-system call.

## Risks and Open Questions

1. **False positives on `GeographicConvergence` for legitimate trade-hub scenarios**: A market day or festival may legitimately cluster agents. Mitigation: the detector reports the convergence but does not prescribe that convergence is bad. The `/scenario-analysis` skill's Layer 3 evaluates whether the convergence is pathological; future "expected convergence" scenarios can note the smell and dismiss it in the report.

2. **Threshold drift**: 60% / 200-tick window / 95% monoculture / 30-tick acute threshold are all constants. If a future scenario class needs different thresholds, the right response is per-scenario threshold overrides via an observer CLI flag — deferred until a motivating case arises.

3. **Window overlap and dedup**: Implementing maximal-run detection (for `AcuteNeedSpike` and `MaintenanceStarvation`) while avoiding duplicate reports across adjacent 200-tick windows requires care. Reference implementation: compute all qualifying runs first, then merge adjacent/overlapping runs into a single anomaly with the combined span per the D3 / D5 dedup keys.

4. **Inline recipe → need classification**: The D4 derivation is inline and duplicates logic that may later be useful to a planner-side system (e.g., a ranking heuristic that prefers need-relevant recipes). If a second consumer appears, promote the classification to a helper function on `RecipeDefinition` or a free function in `worldwake-sim`. For the S117 scope, inline keeps `worldwake-sim` untouched and avoids speculative API.

5. **Dual-use placement**: The extended `Anomaly` struct and four new detectors live in `bin/observer.rs`, which is not importable from other crates. If a future replay / diagnostic tool needs to consume anomaly structures programmatically, both the `Anomaly` struct and the detector module will need to move from `bin/observer.rs` to `crates/worldwake-cli/src/observer.rs` (or a submodule) with `pub` visibility — an `DecisionTraceSink` / `ActionTraceSink`-style extraction. Not required for S117 to land; flag for a future refactor if the reuse pressure materializes.

## Verification Plan

1. `cargo test -p worldwake-cli --test golden_observer_anomalies` — 4 new goldens pass.
2. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-contested.ron --ticks 1440 --output /tmp/contested-dump.md` — dump contains the new anomalies matching the report (at minimum: `GEOGRAPHIC_CONVERGENCE` for East Orchard, `MAINTENANCE_STARVATION` for dirtiness on all 4 agents, `RECIPE_MONOCULTURE` for hunger on all 4 agents, `ACUTE_NEED_SPIKE` for Agent C hunger). Verify the observer CLI flag names against `bin/observer.rs` main-function argument parsing before scripting; if they differ from `--ticks` / `--output`, update this step.
3. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md` — regression: no false positives in the healthy baseline scenario.
4. `cargo clippy --workspace --all-targets -- -D warnings` — clean.
5. `cargo test -p worldwake-cli` — full crate test suite passes (integration test `test_observer_mode_simulation_runs` at `tests/integration.rs:395` continues to pass).
