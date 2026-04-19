# S111SCEHOMLIN-002: `scenario::lints` module + rule implementations

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: None

## Problem

Scenario authors today can ship populations of multiple AI agents that share identical default `CognitiveProfile`, `UtilityProfile`, `PerceptionProfile`, etc., or agents whose exploration drive can never fire (zero `ExplorationProfileDef.curiosity_weight` and no `DiversificationProfile`). FND-22 requires concrete agent diversity; FND-22A requires that learning-driven behaviors have grounding traits. The simulation runs anyway, producing herd behavior or silent feature absence — neither failure is loud.

This ticket creates the lint module that detects both classes of misconfiguration. The module compiles standalone but is not yet wired into the scenario load path (that wiring is S111SCEHOMLIN-003).

## Assumption Reassessment (2026-04-19)

1. **Profile types and field names — verified at /reassess-spec time**:
   - `UtilityProfile` lives in `crates/worldwake-core/src/utility_profile.rs:8-39` with `courage`, no `curiosity_weight`. Has `Default + PartialEq + Eq + Clone`.
   - `CognitiveProfile` lives in `crates/worldwake-core/src/cognitive_profile.rs:6-45` with `Copy + Eq + PartialEq + Ord + PartialOrd`. No `courage`/`patience`/`memory_fidelity` here.
   - `PerceptionProfile` derives `Eq + PartialEq` (used in `AgentDef`).
   - `ExplorationProfileDef` lives in `crates/worldwake-cli/src/scenario/types.rs:208-221` with `Eq + PartialEq` and `curiosity_weight: Permille`.
   - `DiversificationProfile` lives in `crates/worldwake-core` (added by archived S107) with `Eq + PartialEq` and `base_curiosity: Permille`.
   - `EpistemicDispositionProfile`, `IntentionDispositionProfile`, `LastSeenMemory` all derive `Eq + PartialEq` (used in `AgentDef`).
2. **Spec/docs reference**: `specs/S111-scenario-homogeneity-lints.md` D1, D2, and D6.1–D6.3 (current revision after `/reassess-spec` 2026-04-19).
3. **Shared abstraction boundary**: the lint reads `ScenarioDef` (`crates/worldwake-cli/src/scenario/types.rs:23-44`) and its `agents: Vec<AgentDef>` field (lines 107-179). The lint's contract is "produce a `LintReport` from a `ScenarioDef` without mutating either". `AgentDef` field equality uses the existing `PartialEq` derives on each profile type — verified above.
13. **Adjacent contradictions**: none surfaced. The previously-proposed `ArchetypeInheritedUnchanged` rule was dropped during reassessment because the codebase has no archetype mechanism; this ticket implements only the two surviving rules.

## Architecture Check

1. The lint module sits entirely outside the tick loop — it reads `ScenarioDef` at scenario load and emits a `LintReport`. It writes nothing to world state, makes no cross-system calls into the simulation, and has no runtime cost after load. This is the cleanest place to enforce FND-22 / FND-22A authoring contracts: as early as possible, before any tick advances.
2. No backwards-compatibility shims. The module is new; existing code paths are untouched until S111SCEHOMLIN-003.

## Verification Layers

1. `ProfileHomogeneity` correctly identifies populations where no profile field varies across agents -> focused unit test on synthetic `ScenarioDef`.
2. `UnreachableExplorationDrive` correctly identifies agents with both pathways nulled -> focused unit test on synthetic `AgentDef`.
3. `LintReport` accumulates failures without short-circuiting -> focused unit test asserting multi-failure case.
4. Single-layer ticket: lints run at scenario load, not in the tick loop, so action-trace, event-log, and decision-trace layers do not apply. Unit tests on synthetic `ScenarioDef` are the canonical proof surface.

## What to Change

### 1. Create `crates/worldwake-cli/src/scenario/lints.rs`

Define the public types from spec D1:

```rust
use std::collections::BTreeMap;
use serde::Deserialize;
use crate::scenario::types::ScenarioDef;
use worldwake_core::ControlSource;

#[derive(Clone, Debug, Default)]
pub struct LintReport {
    pub failures: Vec<LintFailure>,
    pub warnings: Vec<LintWarning>,
}

#[derive(Clone, Debug)]
pub struct LintFailure {
    pub rule: LintRule,
    pub affected_agents: Vec<String>, // AgentDef.name values
    pub detail: String,
}

#[derive(Clone, Debug)]
pub struct LintWarning {
    pub rule: LintRule,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize)]
pub enum LintRule {
    ProfileHomogeneity,
    UnreachableExplorationDrive,
    AuthoritativeHelperOnSnapshot, // doctest-only; never raised by run_lints
}

pub fn run_lints(scenario: &ScenarioDef) -> LintReport {
    let mut report = LintReport::default();
    check_profile_homogeneity(scenario, &mut report);
    check_unreachable_exploration_drive(scenario, &mut report);
    report
}
```

`LintRule::Deserialize` is required so scenario RON files can name overrides as map keys (consumed by S111SCEHOMLIN-003). Even though the override map itself is added in S111SCEHOMLIN-003, the derive belongs on the enum definition here so the dependency is one-way (003 imports the enum, not the other way around).

### 2. Implement `check_profile_homogeneity`

For each scenario:
- Collect the AI-controlled agents only (`agent.control == ControlSource::Ai`).
- If `ai_agents.len() <= 2`, return without emitting (exempt per spec D2).
- Define a "varies" check: a list of profile-field accessors, each returning `Option<&Profile>`. The list:
  - `cognitive_profile`, `utility_profile`, `perception_profile`, `exploration_profile`, `diversification_profile`, `epistemic_disposition`, `intention_disposition`, `last_seen_memory`.
- For at least one accessor in the list, find a pair of AI agents `(a, b)` such that either one has `Some(_)` and the other has `None`, OR both have `Some(x), Some(y)` with `x != y`.
- If no accessor varies across the population, push a single `LintFailure { rule: ProfileHomogeneity, affected_agents: <all AI agent names>, detail: "AI agent population shares default profiles across all checked fields; FND-22 requires concrete per-agent variation" }`.

The accessor list should be encoded once as a slice of closures or a small helper trait — adding a future profile field to `AgentDef` should grow this list to keep the lint covering.

### 3. Implement `check_unreachable_exploration_drive`

For each AI-controlled agent:
- Let `expl_zero` = `agent.exploration_profile.is_some_and(|e| e.curiosity_weight == Permille::new_unchecked(0))`.
- Let `div_absent_or_zero` = `agent.diversification_profile.is_none() || agent.diversification_profile.as_ref().is_some_and(|d| d.base_curiosity == Permille::new_unchecked(0))`.
- If `expl_zero && div_absent_or_zero` → push `LintFailure { rule: UnreachableExplorationDrive, affected_agents: vec![agent.name.clone()], detail: "ExplorationProfileDef.curiosity_weight == 0 and no DiversificationProfile (or base_curiosity == 0); exploration drive can never fire" }`.

Agents with **no** exploration-related profile at all (both `None`) are not flagged — exploration is opt-in at the scenario level, and absence is unambiguous.

### 4. Wire the new module into `scenario/mod.rs` (declaration only)

Add `pub mod lints;` near the existing `pub mod types;` declaration in `crates/worldwake-cli/src/scenario/mod.rs:6`. Do **not** call `run_lints` from `spawn_scenario` in this ticket — the wiring is S111SCEHOMLIN-003.

### 5. Add unit tests in `lints.rs`

Per spec D6.1, D6.2, D6.3:

1. `homogeneous_population_fails_lint` — synthesize a `ScenarioDef` with 3 AI agents whose every profile field is identical (build via direct struct literal or a small test helper). Assert `run_lints` returns `failures` containing a `ProfileHomogeneity` entry.
2. `varied_population_passes_lint` — synthesize a `ScenarioDef` with 3 AI agents where at least one profile field differs. Assert `run_lints` returns empty failures.
3. `zero_curiosity_no_diversification_fails_lint` — synthesize an AI agent with `exploration_profile.curiosity_weight = 0` and no `diversification_profile`. Assert failure with `UnreachableExplorationDrive`.
4. `population_under_three_exempt_from_homogeneity` — synthesize a `ScenarioDef` with 2 AI agents sharing identical default profiles. Assert `ProfileHomogeneity` is **not** raised (per spec exemption for `len() <= 2`).
5. `human_only_population_exempt_from_homogeneity` — synthesize a `ScenarioDef` with 3 `Human` agents sharing identical profiles. Assert no failure (lint applies to AI agents only).

## Files to Touch

- `crates/worldwake-cli/src/scenario/lints.rs` (new — module + rules + unit tests)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — add `pub mod lints;` declaration; no other change)

## Out of Scope

- Wiring `run_lints` into `spawn_scenario` (covered by S111SCEHOMLIN-003).
- Adding `scenario_lint_overrides` to `ScenarioDef` (covered by S111SCEHOMLIN-003).
- Adding `ScenarioError::LintFailure` variant (covered by S111SCEHOMLIN-003).
- The `--ignore-lints` CLI flag (covered by S111SCEHOMLIN-003).
- The CI integration test that sweeps `scenarios/*.ron` (covered by S111SCEHOMLIN-004).
- Fixing or annotating any existing scenario in `scenarios/` (covered by S111SCEHOMLIN-004).
- `ArchetypeInheritedUnchanged` rule (dropped during /reassess-spec — no archetype system exists).
- Any change to `PlanningSnapshot` (covered by S111SCEHOMLIN-001).

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-cli scenario::lints::tests` (the 5 new unit tests pass).
2. `cargo test -p worldwake-cli` (no regression in existing CLI tests).
3. `cargo clippy --workspace --all-targets -- -D warnings` (no new lints).

### Invariants

1. `run_lints` reads `ScenarioDef` immutably and returns a `LintReport`; it never mutates the scenario or world state.
2. `ProfileHomogeneity` is exempt for AI populations of size ≤ 2.
3. `UnreachableExplorationDrive` only flags agents that have at least one exploration-related profile present (no false positives on agents with no exploration profile at all).
4. `LintReport` accumulates all failures across all rules without short-circuiting on the first failure.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/lints.rs` (new file) — 5 unit tests under `#[cfg(test)] mod tests { ... }`. Rationale: each test exercises one branch of one rule (or the exempt branch). Covers spec D6.1–D6.3 plus two boundary conditions surfaced during reassessment (population-size exemption, control-source filter).

### Commands

1. `cargo test -p worldwake-cli scenario::lints` (targeted — runs only the new module's tests)
2. `cargo test -p worldwake-cli` (regression on the CLI crate)
3. `cargo clippy --workspace --all-targets -- -D warnings` (CI parity)
