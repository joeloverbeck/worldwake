# S117: Convergence and Maintenance-Cycle Observer Smells

## Summary

Extend the observer binary with four new mechanical anomaly kinds that detect failure modes the current detector suite misses: agent population convergence on a single place, maintenance-need starvation (relief firing too infrequently relative to accumulation rate), single-source commodity dependency (recipe monoculture), and sub-threshold acute spikes (needs ≥ critical for 30–99 consecutive ticks, below the 100-tick sustained-critical bar). All four are read-only over authoritative event-log and component state. The spec also adjusts the existing `STUCK_AGENT` detector's stated tolerance note to match the refined precision behavior delivered by S118.

## Phase and Status

Phase 8 Adjunct: Survival Baseline Under Contention (post-`survival-contested.ron` report). Status: Draft.

## Crates

- `worldwake-cli` — observer binary (`src/bin/observer.rs` and `src/observer/anomalies.rs` or equivalent): four new detectors, four new `AnomalyKind` variants, Section 3 rendering, Section 2 supplementary subsections
- `worldwake-core` — no changes
- `worldwake-sim` — no changes
- `worldwake-ai` — no changes
- `worldwake-systems` — no changes

## Dependencies

- None on the simulation side.
- Informs `S116` golden validation (a clean `survival-contested.ron` re-run after S116 should show zero `MAINTENANCE_STARVATION` anomalies).
- Informs `.claude/skills/scenario-analysis/SKILL.md`: once S117 lands, Step 4 Known Pathology Signatures gains "Maintenance-cycle starvation" as a catalogued signature, and Step 6.4 proposed smells 11/12/13 graduate from "proposed" to "shipped".

## Motivating Evidence

From `reports/scenario-analysis-report.md` Layer 3:

- **Gap 1 (Geographic Convergence, HIGH)**: 4 agents spent 78–84% of ticks at East Orchard; Spring Basin (the only wash site) saw 0–86 ticks per agent. No current detector surfaces this.
- **Gap 2 (Latrine Abandonment, MEDIUM)** — also surfaces through gap 4 (Wash-Cycle Starvation). 0 toilet actions across all 4 agents; 88 `relieve_wilderness` actions (each adding +200 permille dirtiness).
- **Gap 3 (Single-Source Food Monoculture, MEDIUM)**: Harvest Apples total = 64, Harvest Grain total = 0, despite every agent knowing both recipes.
- **Gap 4 (Wash-Cycle Starvation, MEDIUM)**: `relief_rate < accumulation_rate` over 200+ ticks. Smell 5 (Sustained Critical Needs) flags the symptom once it crosses 100 ticks, but the underlying frequency mismatch is invisible.
- **Gap 5 (Sub-Threshold Acute Spikes, LOW)**: Agent C hunger=950 for 97 ticks, thirst=900 for 37 ticks. Both below the 100-tick sustained-critical threshold, both dangerous proximity to metabolism tolerance limits (`dehydration_tolerance_ticks=220`).

## Design Goals

1. Every detection gap enumerated above has a mechanical detector that fires on the current `survival-contested.ron` dump (for gaps 1, 2, 4, 5) and on any scenario producing the same surface signature.
2. Detectors run entirely inside the observer — no simulation-state writes, no event-log writes. Observer remains passive (FND-26, FND-12).
3. Each detector's threshold is a compile-time constant with a short justification comment, revisable without engine changes. Threshold revisions are observer-tool changes, not world-meaning changes.
4. Anomaly rendering in Section 3 matches the existing format so the `/scenario-analysis` skill can consume it without parser changes.
5. Each detector produces at most one anomaly entry per (agent × window × kind) tuple to prevent Section 3 flooding.

## Non-Goals

- Changing simulation behavior. These are detectors only.
- Auto-remediation. The detector emits an anomaly; responding to it is the analyst's (or `/scenario-analysis` skill's) job.
- LLM-gated detection. Every detector in this spec is mechanical and runs in the Rust observer binary.
- Refactoring the existing anomaly architecture. New detectors slot into the same `AnomalyKind` enum and Section 3 rendering path.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-12 (Performance May Compress Computation, Never Causality) | Observer detectors read the authoritative event log and agent component state. They do not alter world meaning. |
| FND-26 (Systems Interact Through State) | Detectors are pure reads of authoritative state and derived event-log scans. No simulation system depends on them. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | Anomaly entries in Section 3 are derived views over authoritative history. Deleting the observer output does not change the simulation. |
| FND-29 (Debuggability Is a Product Feature) | This spec's entire purpose: surface failure modes that are currently invisible. |
| FND-29A (Causal History Is Authoritative, Append-Only) | Detectors compute over the append-only event log. They do not mutate history. |

## Deliverables

### D1: `AnomalyKind` extensions

In `crates/worldwake-cli/src/observer/anomalies.rs` (or equivalent):

```rust
pub enum AnomalyKind {
    // ... existing kinds ...
    GeographicConvergence {
        agents: Vec<EntityId>,
        place: EntityId,
        window_start_tick: Tick,
        window_end_tick: Tick,
        overlap_permille: Permille, // fraction of window ticks agents co-occupied
    },
    MaintenanceStarvation {
        agent: EntityId,
        need: NeedKind,
        window_start_tick: Tick,
        window_end_tick: Tick,
        accumulation_permille_per_tick: Permille,
        relief_permille_per_tick: Permille,
    },
    RecipeMonoculture {
        agent: EntityId,
        need: NeedKind,
        used_recipe: RecipeId,
        unused_recipes: Vec<RecipeId>,
        used_share_permille: Permille,
    },
    AcuteNeedSpike {
        agent: EntityId,
        need: NeedKind,
        start_tick: Tick,
        end_tick: Tick,
        peak_permille: Permille,
    },
}
```

### D2: `GeographicConvergence` detector

**Logic**: Over the full run, for each 200-tick rolling window and each place, compute the share of each agent's ticks spent at that place. If 2+ agents each spend ≥ 60% of the window at the same place, emit one anomaly per (agent-set × place × window).

**Threshold justification**: 60% × 200 ticks = 120 ticks of shared occupancy. Below 60%, agents are rotating normally; above 60%, they are anchored. Survival-scattered's historical runs show natural 2-agent overlap peaks around 35–45%; 60% is well above the baseline. Deduplicated: if the same convergence persists across overlapping 200-tick windows, emit only the first occurrence and record `window_end_tick` at the last qualifying window's end.

### D3: `MaintenanceStarvation` detector

**Logic**: For each agent and each homeostatic need, compute over a rolling 200-tick window:
- `accumulation_permille` — total increase of the need value across the window (sum of positive deltas from metabolism + any action penalties)
- `relief_permille` — total decrease of the need value across the window (sum of negative deltas from relief actions)

If `relief_permille < accumulation_permille` AND `avg_need_value_in_window > medium_threshold` (i.e., the need is chronically elevated), emit anomaly.

**Threshold justification**: The medium threshold already exists in `DriveThresholds` (e.g., dirtiness medium = 550). Using the agent's own medium threshold (FND-14) avoids global constants. Rolling 200-tick window mirrors `GeographicConvergence` for consistency. The detector emits at most one `MaintenanceStarvation` per (agent × need × run) — if the starvation persists across multiple windows, merge into a single anomaly with the longest span.

### D4: `RecipeMonoculture` detector

**Logic**: For each agent, at run end, enumerate known recipes by need category (food, water, etc. — recipe metadata exposes which need a recipe satisfies). For each category with ≥ 2 known recipes, compute the per-recipe action-count share across the whole run. If the top recipe's share ≥ 95%, emit anomaly. Cross-check: only emit if the alternative recipe's required facility/resource was known to the agent (from Section 5 belief data) — rules out the case where the agent genuinely never knew where to get the alternative.

**Threshold justification**: 95% is a strict monoculture cutoff; anything above 90% is effectively single-recipe. The belief-gate prevents false positives for agents who know the recipe but never discovered the facility.

### D5: `AcuteNeedSpike` detector

**Logic**: For each agent and each need, scan the per-tick need trajectory (already collected for Section 2) for maximal runs where `value >= critical_threshold` of length ≥ 30 and < 100 consecutive ticks. Emit one anomaly per run.

**Threshold justification**: 30-tick minimum filters out single-action transients (wash takes 12 ticks; eat takes ~5). 100-tick maximum is the existing sustained-critical cutoff — avoids double-flagging with `SUSTAINED_CRITICAL_NEED`.

### D6: Section 3 rendering

Each new anomaly renders under the existing `### Anomaly N — KIND (agent[s])` header. Examples:

```
### Anomaly 6 — GEOGRAPHIC_CONVERGENCE (Agent A, Agent B, Agent C, Agent D)

4 agents spent 78.3% of ticks 100–300 at East Orchard.

Tick range: 100–300
```

```
### Anomaly 7 — MAINTENANCE_STARVATION (Agent A)

Dirtiness accumulated 385 permille but was relieved only 201 permille over ticks 400–600. Average dirtiness in window: 812 permille.

Tick range: 400–600
```

```
### Anomaly 8 — RECIPE_MONOCULTURE (Agent A)

Food actions: 100% Harvest Apples (16 actions), 0% Harvest Grain (0 actions). Both recipes known; West Grainfield facility belief present at tick 412.

Tick range: 0–1440
```

```
### Anomaly 9 — ACUTE_NEED_SPIKE (Agent C)

hunger above 750 permille for 97 consecutive ticks (ticks 99–195), peak 950 permille. Below the 100-tick sustained-critical bar but within 41% of starvation tolerance (480 ticks).

Tick range: 99–195
```

### D7: Section 2 supplementary subsections (optional, same spec)

Add two small supplementary sections to each agent's Section 2 block:

- **"Maintenance rates"** table: per need, accumulation permille/tick, relief permille/tick, net balance. One row per need. Provides the analyst with raw data to check the `MaintenanceStarvation` detector without re-deriving it.
- **"Recipe usage"** table: per known recipe, count of commits. Single line per recipe.

### D8: Golden coverage

Four new goldens in a new `crates/worldwake-cli/tests/golden_observer_anomalies.rs` (observer binary is in `worldwake-cli`):

1. **`convergence_smell_fires_on_forced_hub_scenario`** — scripted scenario with 3 agents converging on one place for 200 consecutive ticks. Assert: Section 3 contains exactly one `GEOGRAPHIC_CONVERGENCE` anomaly covering the expected window.

2. **`maintenance_starvation_fires_on_wash_gap`** — scripted scenario with `wilderness_relief_dirtiness_penalty=200` and wash 5 hops away. Assert: Section 3 contains `MAINTENANCE_STARVATION` for dirtiness on each affected agent.

3. **`recipe_monoculture_fires_on_single_food_dependency`** — scripted scenario with an agent that has Harvest Apples + Harvest Grain + Spring Basin beliefs, but all food intake is Apples. Assert: Section 3 contains `RECIPE_MONOCULTURE` for food on that agent. Control case in the same test: an agent that doesn't know the grainfield facility does NOT trigger the anomaly.

4. **`acute_need_spike_fires_on_40_tick_thirst`** — scripted scenario forcing 40 consecutive ticks of thirst ≥ 900 followed by relief. Assert: Section 3 contains `ACUTE_NEED_SPIKE` with start/end/peak matching the scenario setup.

### D9: Skill-side integration (documentation only, no code)

Update `.claude/skills/scenario-analysis/SKILL.md` Step 4 Known Pathology Signatures to reference the new mechanical detectors. Update Step 6.4 Proposed New Smell Categories template to note that smells 11/12/13 (the ones this spec implements) graduate from "proposed" to "shipped" — proposing smells in a report now means proposing additions to this spec or a successor. This change is a post-implementation skill edit, not a code deliverable.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: All four detectors read the authoritative event log (action traces, ActionStarted/Committed pairs) and agent-component state (HomeostaticNeeds per-tick trajectory already captured for Section 2). No new information paths. FND-7 and FND-14 preserved — detectors operate on the observer's authoritative read view, not on any planner-facing belief state.

2. **Positive-feedback analysis**: None. Detectors do not modify any simulation state, produce no events, and do not feed back into agent behavior. The report-generation loop is purely read-side.

3. **Concrete dampeners**: Not applicable — no loops to dampen. The per-run anomaly-dedup logic (one anomaly per agent × need × run or window) prevents Section 3 output inflation but is an output-format concern, not a world-state dampener.

4. **Stored state vs. derived read-model**:
   - **Stored state**: none. This spec adds no authoritative components.
   - **Derived**: every anomaly entry is derived per-run from the event log + component trajectories. Re-running the observer on the same dump produces the same anomalies.

## SystemFn Integration

None. No SystemFn changes. Observer is a post-run read tool.

## Component Registration

None. No new ECS components.

## Cross-System Interactions (FND-26)

No simulation system consumes the observer's output. Cross-observer interactions:
- Detectors read the same authoritative event log and agent components that Section 2 already summarizes — shared read substrate, not cross-component coupling.
- `STUCK_AGENT` detector behavior is refined by S118 (active-frame exclusion). S117 does not re-specify that detector; it merely relies on its revised behavior for the Section 3 ordering and false-positive counts.

## Risks and Open Questions

1. **False positives on `GeographicConvergence` for legitimate trade-hub scenarios**: A market day or festival may legitimately cluster agents. Mitigation: the detector reports the convergence but does not prescribe that convergence is bad. The `/scenario-analysis` skill's Layer 3 evaluates whether the convergence is pathological; future "expected convergence" scenarios can note the smell and dismiss it in the report.

2. **Threshold drift**: 60% / 200-tick window / 95% monoculture / 30-tick acute threshold are all constants. If a future scenario class needs different thresholds, the right response is per-scenario threshold overrides via an observer CLI flag — deferred until a motivating case arises.

3. **Window overlap and dedup**: Implementing maximal-run detection (for `AcuteNeedSpike` and `MaintenanceStarvation`) while avoiding duplicate reports across adjacent 200-tick windows requires care. Reference implementation: compute all qualifying runs first, then merge adjacent/overlapping runs into a single anomaly with the combined span.

## Verification Plan

1. `cargo test -p worldwake-cli --test golden_observer_anomalies` — 4 new goldens pass
2. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-contested.ron --ticks 1440 --output /tmp/contested-dump.md` — dump contains the new anomalies matching the report (at minimum: `GEOGRAPHIC_CONVERGENCE` for East Orchard, `MAINTENANCE_STARVATION` for dirtiness on all 4 agents, `RECIPE_MONOCULTURE` for food on all 4 agents, `ACUTE_NEED_SPIKE` for Agent C hunger)
3. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md` — regression: no false positives in the healthy baseline scenario
4. `cargo clippy --workspace --all-targets -- -D warnings` — clean
