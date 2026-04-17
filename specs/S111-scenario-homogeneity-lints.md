# S111: Scenario Profile Homogeneity Lints

## Summary

Add scenario-load-time lints that fail fast when a scenario ships an agent population with suspiciously homogeneous cognitive/utility/belief profiles, or when a profile is internally inconsistent (proactive exploration enabled without a curiosity/information-seeking trait). Also fold in an architecture-lint regression: confirm `PlanningSnapshot` exposes no authoritative-only helpers to planner code (the travel-fence audit — PR-1.11 in the assessment). Lints run at scenario load inside `worldwake-cli` and inside CI-targeted tests, turning "silent herd behavior" into a scenario-author-visible error.

## Phase and Status

Phase 8: Belief-First Continual Planning Foundation. Status: Draft.

## Crates

- `worldwake-cli` — `scenario::lints` module; wire into `spawn_scenario` / `load_scenario` path
- `worldwake-core` — no code changes; lints read agent component state after spawn
- `worldwake-ai` — a lint-only integration test for the `PlanningSnapshot` accessor-only contract (regression test)

## Dependencies

- None.

## Design Goals

- A scenario with all default `CognitiveProfile` / `UtilityProfile` / `BeliefProfile` across a population of >2 agents fails to load. Scenario authors see the error before the simulation runs.
- A scenario that sets `ExplorationProfile::enable_proactive = true` but leaves `curiosity_weight = 0` (or equivalent) fails to load.
- A scenario that inherits `courage`, `patience`, and `memory_fidelity` unchanged across an entire archetype fails to load.
- A compile/CI test confirms `PlanningSnapshot` exposes only accessor functions (no pub authoritative-only fields); new authoritative-only fields added in the future fail the regression.
- Lints are strict — scenarios must explicitly vary; no "close enough" heuristics. Authors can silence individual lints with an explicit override flag (e.g., `scenario_lints_disable: ["profile_homogeneity"]`), but the flag itself makes the intent legible.

## Non-Goals

- Runtime lints. All lints run at scenario load.
- Reporting homogeneity as a warning-only diagnostic. Failures are hard failures.
- Arch lints beyond the travel-fence regression. Broader arch lints (e.g., "contested affordance lacks explicit claim entity") are deferred until the contested-affordance specs (S60, S63) land.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-22 (Agent Diversity Through Concrete Variation) | Homogeneity lints fail builds that ship uniform populations. Agents in the same role must differ. |
| FND-22A (Learning, Habits, Preference Shifts Are Concrete State) | Curiosity-without-exploration-trait lint enforces that learned-state-driven behaviors must have grounding traits. |
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
    pub affected_agents: Vec<AgentName>,
    pub detail: String,
}

pub enum LintRule {
    ProfileHomogeneity,
    ProactiveExplorationWithoutCuriosity,
    ArchetypeInheritedUnchanged,
    AuthoritativeHelperOnSnapshot, // tested separately, not at scenario load
}

pub fn run_lints(scenario: &ScenarioDef) -> LintReport;
```

### D2: Lint rules

**Rule `ProfileHomogeneity`**: For each scenario with >2 AI agents, at least one of `CognitiveProfile`, `UtilityProfile`, `BeliefProfile`, `PerceptionProfile`, or `ExplorationProfile` must differ across agents. "Differ" means at least one field has distinct values between at least two agents. Populations of ≤2 agents are exempt (test scenarios). Scenario authors can opt out via `agent_def.uniform_population_justification: String`.

**Rule `ProactiveExplorationWithoutCuriosity`**: If an agent has `ExplorationProfile::enable_proactive = true` (or the relevant S80/S107 flag), it must also have a non-zero `curiosity_weight` in `UtilityProfile` (or the equivalent field). The pairing prevents silent exploration spam.

**Rule `ArchetypeInheritedUnchanged`**: For agents that reference an archetype (if the scenario system has archetype inheritance at load time; if not, this rule is dormant), fail if `courage`, `patience`, and `memory_fidelity` are all inherited unchanged across the archetype's whole population. At least one per-agent override required.

**Rule `AuthoritativeHelperOnSnapshot`** (D3): Architecture test, not runtime.

### D3: Architecture test — no authoritative helpers on `PlanningSnapshot`

New integration test in `crates/worldwake-ai/tests/planning_snapshot_accessor_contract.rs`:

```rust
#[test]
fn planning_snapshot_exposes_only_accessors() {
    // Use a type-level check or a source-file text scan that
    // confirms every pub/pub(crate) item on PlanningSnapshot is
    // either (a) a constructor, (b) an accessor method, or (c) a
    // test helper behind #[cfg(test)].
    // Fails if any pub field exposes authoritative-only data
    // (e.g., a future reintroduction of raw shortest_travel_ticks).
}
```

Concretely, the test parses the `planning_snapshot.rs` source, enumerates all `pub` items, and asserts that no `pub` or `pub(crate)` field has a type matching the authoritative-matrix allowlist (`DistanceMatrix`, anything ending in `Matrix`, any type annotated `#[authoritative_only]`). A simpler variant: assert `PlanningSnapshot::shortest_travel_ticks` remains `pub(crate)` and is not exported beyond the crate.

Alternative implementation: a build-time cargo-semver-check-style rule, or a compile_fail test that attempts to construct a `PlanningSnapshot` and access `shortest_travel_ticks` from outside the crate — must fail to compile.

### D4: Integration at scenario load

In `crates/worldwake-cli/src/scenario/mod.rs`, wire `run_lints` into the scenario-load path:

```rust
pub fn load_scenario(path: &Path) -> Result<Scenario, ScenarioError> {
    let def = parse_ron(path)?;
    let report = lints::run_lints(&def);
    if !report.failures.is_empty() {
        return Err(ScenarioError::LintFailure(report));
    }
    // continue with existing spawn logic
}
```

An explicit `--ignore-lints` CLI flag disables lints for ad-hoc debugging scenarios but emits a visible warning.

### D5: Existing-scenario audit

As part of landing this spec, run the lints against every scenario in `scenarios/` (`survival-baseline.ron`, `survival-scattered.ron`, `survival-contested.ron`, `cli-evaluation.ron`, all CLI eval scenarios, all spec-specific scenarios). Scenarios that currently fail the lints must be fixed before the spec is considered complete. If a scenario genuinely needs homogeneity (e.g., a regression test targeting a specific behavior), add the `uniform_population_justification` opt-out.

### D6: Golden test — lint-failing scenario is rejected

New unit test in `scenario/lints.rs`: a synthetic scenario with 3 agents sharing identical default profiles is rejected by `run_lints` with `LintRule::ProfileHomogeneity` failure. A second synthetic scenario with varied profiles passes.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Not applicable. Lints run at scenario load; no simulation information flows through them.
2. **Positive-feedback analysis**: Not applicable.
3. **Concrete dampeners**: Not applicable.
4. **Stored state vs. derived read-model**: `LintReport` is derived from `ScenarioDef` at load; nothing persists to world state.

## SystemFn Integration

None — lints run outside the tick loop, at scenario load.

## Component Registration

None.

## Cross-System Interactions

- **Scenario loader ↔ lint module**: Pure read of `ScenarioDef`, no cross-system call into the sim.

## Profile-Driven Parameters

Not applicable. Lint rules are static; the tunable surface is the per-scenario `uniform_population_justification` opt-out.

## Validation and Falsification

### Unit tests

1. Synthetic homogeneous scenario (3 agents, identical default profiles) → lint fails with `ProfileHomogeneity`.
2. Synthetic scenario with varied profiles → lint passes.
3. Synthetic scenario with `enable_proactive = true` and `curiosity_weight = 0` → lint fails with `ProactiveExplorationWithoutCuriosity`.
4. Synthetic scenario with `--ignore-lints` → lints emit a warning but scenario loads.

### Integration tests

5. Every committed scenario in `scenarios/` passes lints (enforced via CI test that iterates the scenarios directory and calls `run_lints` on each).
6. `PlanningSnapshot` accessor-only contract test (D3) — confirms no new `pub` authoritative fields have been added.

### Regression guard

7. A new scenario contributor adds a homogeneous population without the opt-out → CI blocks the PR.
8. A future PR that adds a `pub shortest_travel_ticks: DistanceMatrix` to `PlanningSnapshot` fails the arch test.

## Outcome

To be filled in at completion.
