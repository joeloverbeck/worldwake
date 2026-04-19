# S111: Scenario Profile Homogeneity Lints

**Status**: COMPLETED

## Summary

Add scenario-load-time lints that fail fast when a scenario ships an agent population with suspiciously homogeneous cognitive/utility/perception/exploration profiles, or when an agent's exploration configuration is internally inconsistent (curiosity weight zero on every exploration-related profile so no exploration drive can ever fire). Also fold in an architecture-lint regression: confirm `PlanningSnapshot` exposes no authoritative-only fields to planner code outside `worldwake-ai` (the travel-fence audit — PR-1.11 in the assessment). Lints run at scenario load inside `worldwake-cli` and inside CI-targeted tests, turning "silent herd behavior" into a scenario-author-visible error.

## Phase and Status

Phase 8: Belief-First Continual Planning Foundation. Status: Completed.

## Crates

- `worldwake-cli` — `scenario::lints` module; new `scenario_lint_overrides` field on `ScenarioDef`; wire into `spawn_scenario` / `load_scenario_file` path
- `worldwake-ai` — `compile_fail` doctest on `PlanningSnapshot` enforcing the accessor-only contract (regression test)

## Dependencies

- `archive/specs/S80-exploration-drive.md` (archived) — owns `ExplorationProfile` and its `curiosity_weight` field that the lint reads
- `archive/specs/S107-proactive-diversification.md` (archived) — owns `DiversificationProfile` and its `base_curiosity` field; mere presence of this component is what enables proactive exploration (there is no `enable_proactive` boolean)

## Design Goals

- A scenario with all default `CognitiveProfile` / `UtilityProfile` / `PerceptionProfile` / `ExplorationProfileDef` (or absence-only differences) across a population of >2 AI agents fails to load. Scenario authors see the error before the simulation runs.
- A scenario in which **no** exploration drive can ever fire — `ExplorationProfileDef.curiosity_weight == 0` AND no `DiversificationProfile` (or `DiversificationProfile.base_curiosity == 0`) — fails to load. (Replaces the original `enable_proactive`-based rule, which assumed a flag that does not exist.)
- Compile-time doctests on `PlanningSnapshot` confirm the accessor-only boundary from outside `worldwake-ai`: one `compile_fail` snippet proves `shortest_travel_ticks` is not externally readable, and one `compile_fail` snippet proves `worldwake_ai::planning_snapshot::DistanceMatrix` is not externally reachable. Future reintroduction of a public `DistanceMatrix`, or of both public `DistanceMatrix` plus public `shortest_travel_ticks`, fails `cargo test --doc -p worldwake-ai`.
- Lints are strict — scenarios must explicitly vary; no "close enough" heuristics. Authors silence individual lints with a single in-scenario opt-out: `scenario_lint_overrides: { ProfileHomogeneity: "test scenario covering exact-replica regression" }`. The override map keys are `LintRule` variants; values are required justification strings (empty string is rejected as a mis-spelled override).

## Non-Goals

- Runtime lints. All lints run at scenario load.
- Reporting homogeneity as a warning-only diagnostic. Failures are hard failures.
- Per-agent opt-outs. The lint reasons over populations, so the override lives on `ScenarioDef`, not on individual `AgentDef` entries.
- Archetype-inheritance lints. The current scenario system has no archetype mechanism; the previously-proposed `ArchetypeInheritedUnchanged` rule is dropped pending an archetype system spec.
- Arch lints beyond the travel-fence regression. Broader arch lints (e.g., "contested affordance lacks explicit claim entity") are deferred until the contested-affordance specs (S60, S63) land.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-22 (Agent Diversity Through Concrete Variation) | Homogeneity lints fail builds that ship uniform populations. Agents in the same role must differ. |
| FND-22A (Learning, Habits, Preference Shifts Are Concrete State) | The exploration-coherence lint (`UnreachableExplorationDrive`) ensures that any agent whose scenario suggests exploration has at least one concrete drive parameter capable of firing it. |
| FND-7 (Locality of Motion, Interaction, Communication) | The travel-fence accessor-only contract prevents AI code from reading authoritative global distance data directly. |
| FND-31 (Validation and Falsification Are First-Class) | Lints are falsification: the simulation refuses to run configurations that violate declared invariants. |

## Deliverables

### D1: `scenario::lints` module

New module `crates/worldwake-cli/src/scenario/lints.rs`:

```rust
pub struct LintReport {
    pub failures: Vec<LintFailure>,
    pub warnings: Vec<LintWarning>,
}

pub struct LintFailure {
    pub rule: LintRule,
    pub affected_agents: Vec<String>, // agent names from AgentDef.name
    pub detail: String,
}

pub struct LintWarning {
    pub rule: LintRule,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize)]
pub enum LintRule {
    ProfileHomogeneity,
    UnreachableExplorationDrive,
    AuthoritativeHelperOnSnapshot, // tested via D3 compile_fail doctest, not at scenario load
}

pub fn run_lints(scenario: &ScenarioDef) -> LintReport;
```

`LintRule` derives `Deserialize` so the override map (D4) can be parsed directly from RON.

### D2: Lint rules

**Rule `ProfileHomogeneity`**: For each scenario with >2 AI agents, the population must vary along at least one of these `AgentDef` profile fields:

- `cognitive_profile: Option<CognitiveProfile>`
- `utility_profile: Option<UtilityProfile>`
- `perception_profile: Option<PerceptionProfile>`
- `exploration_profile: Option<ExplorationProfileDef>`
- `diversification_profile: Option<DiversificationProfile>`
- `epistemic_disposition: Option<EpistemicDispositionProfile>`
- `intention_disposition: Option<IntentionDispositionProfile>`
- `last_seen_memory: Option<LastSeenMemory>`

"Vary" is defined as: there exists at least one pair of AI agents `(a, b)` such that for at least one profile field above, **either** one agent has `Some(_)` and the other has `None`, **or** both have `Some(x)` and `Some(y)` with `x != y` (uses the existing `PartialEq` derive on each profile type). Populations of ≤2 AI agents and populations entirely composed of `ControlSource::Human | None` agents are exempt.

Recommended fields list is an implementation detail of the lint — adding new agent-profile fields to `AgentDef` should grow this list as part of the field-addition diff so regressions stay caught.

**Rule `UnreachableExplorationDrive`**: For each AI agent, fail if:

- the agent has an `exploration_profile` whose `curiosity_weight == Permille(0)`,
- AND the agent either lacks `diversification_profile` or has `diversification_profile.base_curiosity == Permille(0)`.

Rationale: an agent with both pathways nulled cannot ever generate exploration goals, but the scenario shape (presence of the profiles) suggests the author intended exploration. This is the failure mode the original `ProactiveExplorationWithoutCuriosity` rule was reaching for; the corrected rule is type-checked against the profiles that actually exist.

Agents with no exploration-related profile at all (both fields `None`) are not flagged — exploration is opt-in at the scenario level, and absence is unambiguous.

### D3: Architecture test — no authoritative helpers on `PlanningSnapshot`

Replace the original test-file approach with co-located `compile_fail` doctests on `PlanningSnapshot` in `crates/worldwake-ai/src/planning_snapshot.rs`:

````rust
/// Authoritative travel data is intentionally unreachable from outside this
/// crate. Planner code outside `worldwake-ai` must use the `min_travel_ticks`
/// accessors instead of reading the underlying matrix.
///
/// ```compile_fail
/// use worldwake_ai::PlanningSnapshot;
/// fn read_authoritative(snapshot: &PlanningSnapshot) {
///     let _ = &snapshot.shortest_travel_ticks as *const _ as *const ();
/// }
/// ```
///
/// ```compile_fail
/// fn mention_authoritative_type() {
///     let _: Option<worldwake_ai::planning_snapshot::DistanceMatrix> = None;
/// }
/// ```
pub struct PlanningSnapshot { /* ... */ }
````

Today both snippets fail to compile because `DistanceMatrix` is private and `shortest_travel_ticks` is module-private. The split proof is intentional: the type-mention snippet fails open when `DistanceMatrix` becomes public, and the field-access snippet fails open once both the field and its type become public enough for external use.

A positive-case doctest (asserting the public accessor surface compiles) lives next to the negative case so the regression test cannot silently always-pass:

````rust
/// ```
/// use worldwake_ai::PlanningSnapshot;
/// fn read_via_accessor(s: &PlanningSnapshot, from: worldwake_core::EntityId, to: worldwake_core::EntityId) -> Option<u32> {
///     s.min_travel_ticks(from, to)
/// }
/// ```
````

`PlanningSnapshot` is already re-exported from `worldwake-ai/src/lib.rs:96`, so the doctests resolve.

### D4: Integration at scenario load

Add a new `ScenarioError` variant and an override map to `ScenarioDef`:

```rust
// crates/worldwake-cli/src/scenario/types.rs
#[derive(Clone, Debug, Deserialize)]
pub struct ScenarioDef {
    // ... existing fields ...
    #[serde(default)]
    pub scenario_lint_overrides: BTreeMap<lints::LintRule, String>,
}

// crates/worldwake-cli/src/scenario/mod.rs
#[derive(Debug)]
pub enum ScenarioError {
    Io(std::io::Error),
    Parse(ron::error::SpannedError),
    Validation(String),
    World(worldwake_core::WorldError),
    LintFailure(lints::LintReport),
}
```

Wire `run_lints` into `spawn_scenario` immediately after the `ScenarioDef` is in hand and before any topology construction. The override map suppresses individual rules; an empty justification string is rejected as `ScenarioError::Validation("lint override for {rule:?} requires a non-empty justification")` so authors cannot silently dismiss a rule.

```rust
pub fn spawn_scenario(def: &ScenarioDef) -> Result<SpawnedSimulation, ScenarioError> {
    let report = lints::run_lints(def);
    let report = lints::filter_overrides(report, &def.scenario_lint_overrides)?;
    if !report.failures.is_empty() {
        return Err(ScenarioError::LintFailure(report));
    }
    // continue with existing spawn logic (build_topology, World::new, ...)
}
```

`load_scenario_file` is unchanged — it remains a pure deserialize. CLI entry points that previously called `spawn_scenario(&def)` get the lint check for free.

A `--ignore-lints` CLI flag (in `crates/worldwake-cli/src/main.rs` or whichever bin invokes `spawn_scenario`) bypasses lint failures for ad-hoc debugging, emitting a stderr warning that names every suppressed rule. This flag is the only path that ignores lints without an in-scenario override.

### D5: Existing-scenario audit

As part of landing this spec, run the lints against every scenario in `scenarios/`:

- `survival-baseline.ron`
- `survival-scattered.ron`
- `survival-contested.ron`
- `cli-evaluation.ron`
- `drive-escalation-wash-priority.ron`
- any spec-specific scenarios that exist at landing time

Scenarios that fail must be fixed before the spec is considered complete. If a scenario genuinely needs homogeneity (e.g., a regression test targeting a specific behavior), add a `scenario_lint_overrides` entry with a justification string explaining why the homogeneity is load-bearing for that scenario.

### D6: Golden tests — lint-failing scenario is rejected

New unit tests in `scenario/lints.rs`:

1. Synthetic scenario with 3 AI agents sharing identical default `CognitiveProfile + UtilityProfile + PerceptionProfile + ExplorationProfileDef + DiversificationProfile` → lint fails with `ProfileHomogeneity`.
2. Synthetic scenario with 3 AI agents whose profiles differ in at least one field → lint passes.
3. Synthetic scenario with an AI agent whose `exploration_profile.curiosity_weight == 0` and no `diversification_profile` → lint fails with `UnreachableExplorationDrive`.
4. Synthetic scenario with the same shape as (1) but with `scenario_lint_overrides: { ProfileHomogeneity: "covers identical-twin regression" }` → lint passes.
5. Synthetic scenario with `scenario_lint_overrides: { ProfileHomogeneity: "" }` (empty justification) → returns `ScenarioError::Validation`.
6. CI-targeted integration test that iterates `scenarios/*.ron`, calls `load_scenario_file` then `lints::run_lints`, and asserts every committed scenario passes (or has an override + justification).

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Not applicable. Lints run at scenario load; no simulation information flows through them.
2. **Positive-feedback analysis**: Not applicable.
3. **Concrete dampeners**: Not applicable.
4. **Stored state vs. derived read-model**: `LintReport` is derived from `ScenarioDef` at load; nothing persists to world state. The new `scenario_lint_overrides` field is authoritative scenario configuration (RON-loaded), not a runtime cache.

## SystemFn Integration

None — lints run outside the tick loop, at scenario load.

## Component Registration

None.

## Cross-System Interactions

- **Scenario loader ↔ lint module**: Pure read of `ScenarioDef` (incl. the new `scenario_lint_overrides` field), no cross-system call into the sim.

## Profile-Driven Parameters

Not applicable. Lint rules are static; the tunable surface is the per-scenario `scenario_lint_overrides` map.

## Validation and Falsification

### Unit tests

Listed in D6 (1)–(5).

### Integration tests

6. (D6) Every committed scenario in `scenarios/` passes lints (enforced via CI test that iterates the scenarios directory and calls `run_lints` on each).
7. `PlanningSnapshot` accessor-only contract — D3's `compile_fail` + positive-case doctests verified by `cargo test --doc -p worldwake-ai`.

### Regression guard

8. A new scenario contributor adds a homogeneous AI population without an override → CI blocks the PR.
9. A future PR that makes `worldwake_ai::planning_snapshot::DistanceMatrix` public causes the type-mention `compile_fail` doctest to compile, and a future PR that makes both `DistanceMatrix` and `shortest_travel_ticks` public causes the field-access `compile_fail` doctest to compile, failing `cargo test --doc -p worldwake-ai`.

## Outcome

Completed on 2026-04-19.

- Landed the `worldwake-cli::scenario::lints` module with `LintReport`, `LintFailure`, `LintWarning`, `LintRule`, `run_lints`, and the two shipped rules: `ProfileHomogeneity` and `UnreachableExplorationDrive`.
- Added `ScenarioDef.scenario_lint_overrides`, `ScenarioError::LintFailure`, load-time lint enforcement in `spawn_scenario`, `spawn_scenario_ignoring_lints`, and explicit `--ignore-lints` support in both CLI entry bins.
- Landed the `PlanningSnapshot` accessor-fence regression as co-located doctests in `crates/worldwake-ai/src/planning_snapshot.rs`, using two `compile_fail` snippets plus a positive accessor-surface doctest.
- Added the CI sweep test at `crates/worldwake-cli/tests/scenario_lint_sweep.rs`, which iterates `scenarios/*.ron`, loads each scenario, runs lints, applies override filtering, and fails on unsuppressed lint reports.
- Audited the committed `scenarios/` corpus on the live branch; all current scenarios already passed the lint contract without needing scenario-file edits or new overrides.

### Deviations

- The draft summary and `IMPLEMENTATION-ORDER.md` shorthand still referenced the older broader rule sketch ("proactive-exploration-without-curiosity" and archetype inheritance). The landed spec family narrowed to the type-checked `UnreachableExplorationDrive` rule and kept archetype-inheritance lints as a non-goal.
- The existing-scenario audit did not require the expected scenario rewrites or scenario-root overrides; the final S111 family landed the enforcement and CI guard without modifying committed scenario files.

### Verification Result

- Passed `cargo test --doc -p worldwake-ai`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test -p worldwake-cli scenario::lints`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo test -p worldwake-cli --test scenario_lint_sweep`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
