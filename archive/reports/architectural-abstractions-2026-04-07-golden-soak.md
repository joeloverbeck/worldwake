**Status**: COMPLETED

# Architectural Abstraction Recovery: golden-soak

**Date**: 2026-04-07
**Input**: crates/worldwake-ai/tests/golden_soak.rs
**Source modules analyzed**: 198
**Crates touched**: worldwake-core (68), worldwake-sim (43), worldwake-systems (39), worldwake-ai (48)
**Prior reports consulted**: reports/missing-abstractions-2026-04-07-golden-soak.md

## Executive Summary

Cross-subsystem fractures are concentrated on a single architectural surface: the `RuntimeBeliefView` / `GoalBeliefView` trait pair in worldwake-sim, which is the primary coupling conduit between the AI planner and the simulation layer. Git temporal coupling analysis confirms this — the top 10 most frequently co-changing file pairs are ALL cross-crate pairs between worldwake-ai planning files and worldwake-sim belief view files, with co-change counts of 33–46 per pair over 6 months. One candidate abstraction (Belief View Domain Decomposition) survived validation with medium confidence. The rest of the codebase — action registration, system dispatch, affordance queries, conservation, perception — shows clean crate boundaries and expected coupling patterns.

The prior missing-abstractions report identified one incomplete single-concept abstraction (`GoalDispatchDeclaration` not carrying all per-goal-kind static metadata). That finding is acknowledged and not re-reported here. This report focuses on cross-subsystem architectural concerns that the single-concept analysis cannot detect.

## Scenario Families

| Family | Tests | Domain Concepts | Key Assertions |
|--------|-------|----------------|----------------|
| Needs-driven survival | 1 (per-tick inv. 2, 3) | needs, death, metabolism | needs ≤ 1000‰; dead agents have no active actions |
| Economic lifecycle | 1 (per-tick inv. 1, per-run inv. 7) | commodity, conservation, trade, production | conservation holds per-tick; acquire/trade events emerge |
| Social interaction | 1 (per-run inv. 7) | tell, social observation, belief sharing | tell events emerge across seeds |
| Political emergence | 1 (cross-run inv. 10) | offices, ClaimOffice, faction | political events reach scaled threshold |
| Criminal emergence | 1 (cross-run inv. 11) | theft, crime, violation | crime events reach scaled threshold |
| Spatial navigation | 1 (per-tick inv. 4, per-run inv. 7) | place, travel, topology | agents at valid places; travel events emerge |
| Causal integrity | 1 (per-tick inv. 5, 6) | event log, cause chain, tick | ticks advance monotonically; all causal refs valid |
| Seed diversity | 1 (cross-run inv. 9) | hash, determinism | not all seeds produce identical hashes |

## Traceability Summary

| Module | Scenario Families | Confidence | Strategy |
|--------|------------------|------------|----------|
| worldwake-sim/belief_view.rs | all (planning reads) | High | use/import + temporal coupling (43–46 co-changes with AI) |
| worldwake-sim/per_agent_belief_view.rs | all (belief implementation) | High | use/import + temporal coupling (46 co-changes with AI) |
| worldwake-ai/planning_state.rs | economic, social, political, criminal | High | use/import + temporal coupling (46 co-changes with SIM) |
| worldwake-ai/planning_snapshot.rs | economic, social, political, criminal | High | use/import + temporal coupling (38 co-changes with SIM) |
| worldwake-ai/candidate_generation.rs | all emergence families | High | use/import + temporal coupling (46 co-changes with SIM) |
| worldwake-ai/goal_model.rs | all emergence families | High | use/import + temporal coupling (40 co-changes with SIM) |
| worldwake-ai/ranking.rs | all emergence families | High | use/import + temporal coupling (39 co-changes with SIM) |
| worldwake-sim/tick_step.rs | all (tick execution) | High | use/import + temporal coupling (27 co-changes with systems) |
| worldwake-sim/affordance_query.rs | all emergence families | High | use/import (RuntimeBeliefView consumer) |
| worldwake-systems/tell_actions.rs | social interaction | High | use/import + temporal coupling (29 co-changes with SIM) |
| worldwake-core/world.rs | all (authoritative state) | High | use/import + temporal coupling (31 co-changes with AI) |

## Fracture Summary

| # | Fracture Type | Location | Evidence Sources | Severity |
|---|--------------|----------|-----------------|----------|
| 1 | Overloaded abstraction (#7) | worldwake-sim/belief_view.rs (RuntimeBeliefView + GoalBeliefView) | trait method count (~100+ spanning 8+ domains) + temporal coupling (46 co-changes) | MEDIUM |
| 2 | Projection drift (#3) | worldwake-ai/planning_snapshot.rs (SnapshotEntity) ↔ worldwake-sim/belief_view.rs | SnapshotEntity ~40 fields manually mirroring trait surface + temporal coupling (38 co-changes) | MEDIUM |

### Fracture 1: Overloaded Abstraction — RuntimeBeliefView

**Evidence source 1 — Structural analysis**: `RuntimeBeliefView` (belief_view.rs:363–741) has ~100+ methods spanning at least 8 conceptual domains: entity lifecycle (alive/dead/incapacitated), spatial (place/transit/topology), inventory (possession/commodity/load), combat (wounds/hostiles/attackers/courage), social (beliefs/observations/institutions/tell), economic (trade/sale/demand/merchandise), temporal (reservations/queue/duration), and agent profiles (needs/metabolism/patrol/justice/etc). `GoalBeliefView` (belief_view.rs:34–362) is a narrower ~80-method subset of the same surface. The `impl_goal_belief_view!` macro mechanically delegates GoalBeliefView → RuntimeBeliefView.

**Evidence source 2 — Temporal coupling**: The top 10 cross-crate co-changing file pairs (over 6 months, 712 commits) are all connections between AI planning files and SIM belief view files:
- `planning_state.rs` ↔ `per_agent_belief_view.rs`: **46** co-changes
- `candidate_generation.rs` ↔ `per_agent_belief_view.rs`: **46** co-changes
- `candidate_generation.rs` ↔ `belief_view.rs`: **43** co-changes
- `planning_state.rs` ↔ `belief_view.rs`: **43** co-changes
- `goal_model.rs` ↔ `per_agent_belief_view.rs`: **40** co-changes
- `goal_model.rs` ↔ `belief_view.rs`: **39** co-changes
- `ranking.rs` ↔ `per_agent_belief_view.rs`: **39** co-changes
- `planning_snapshot.rs` ↔ `per_agent_belief_view.rs`: **38** co-changes

**Mechanism**: Every new domain concept that must be visible to AI planning requires a change cascade:
1. Add method to `RuntimeBeliefView` / `GoalBeliefView` in `belief_view.rs` (worldwake-sim)
2. Implement in `PerAgentBeliefView` in `per_agent_belief_view.rs` (worldwake-sim)
3. Add field to `SnapshotEntity` in `planning_snapshot.rs` (worldwake-ai)
4. Implement in `PlanningState` in `planning_state.rs` (worldwake-ai)
5. Update ~10+ test mock implementations across both crates

This is a 4-file minimum, 2-crate shotgun surgery pattern.

### Fracture 2: Projection Drift — PlanningSnapshot mirrors RuntimeBeliefView

**Evidence source 1 — Structural analysis**: `SnapshotEntity` (planning_snapshot.rs:30–75) has ~40 fields that manually replicate what `RuntimeBeliefView` exposes: `effective_place`, `direct_possessor`, `commodity_quantities`, `homeostatic_needs`, `combat_profile`, `wounds`, `hostile_targets`, `office_data`, `merchandise_profile`, `sale_seller_overrides`, etc. There is no derivation mechanism — each field must be manually populated during snapshot creation and manually read during `impl RuntimeBeliefView for PlanningState`.

**Evidence source 2 — Temporal coupling**: `planning_snapshot.rs` has 38 co-changes with `per_agent_belief_view.rs` and 35 with `belief_view.rs`. When the trait surface grows, the snapshot must grow to match.

**Relationship to Fracture 1**: This is a direct consequence. The snapshot exists because the GOAP planner needs a forkable hypothetical world state for plan search. The snapshot must cache everything the planner might query, which means it mirrors the full trait surface.

## Candidate Abstractions

### Belief View Domain Decomposition

**Kind**: Bounded context
**Scope**: worldwake-sim (trait definition), worldwake-ai (PlanningState + PlanningSnapshot implementation)
**Fractures addressed**: #1 (Overloaded abstraction), #2 (Projection drift)

**Owned truth**: Each sub-trait would own the contract for one query domain — spatial queries, inventory queries, combat queries, social queries, economic queries, profile queries. The RuntimeBeliefView super-trait would compose them. New domain concepts would only require changes to the relevant sub-trait and its implementors, not the entire surface.

**Invariants**:
- Every sub-trait method must return the agent's believed state, never authoritative world state (Principle 14)
- SnapshotEntity fields must correspond 1:1 to sub-trait methods (no orphan fields, no orphan methods)
- The super-trait composition must remain usable as `&dyn RuntimeBeliefView` for dynamic dispatch

**Owner boundary**: worldwake-sim owns the trait definitions and the `PerAgentBeliefView` implementation. worldwake-ai owns `PlanningState` and `PlanningSnapshot` implementations.

**Modules affected**:
- `worldwake-sim/src/belief_view.rs` — split into sub-trait modules or use sub-trait composition
- `worldwake-sim/src/per_agent_belief_view.rs` — implement sub-traits
- `worldwake-ai/src/planning_state.rs` — implement sub-traits
- `worldwake-ai/src/planning_snapshot.rs` — organize SnapshotEntity by sub-trait domain
- ~10 test mock files — implement relevant sub-traits instead of full surface

**Tests explained**: All scenario families — every emergence scenario depends on the belief view for planning

**Expected simplification**:
- Adding a new belief surface method (e.g., new profile type) would require changes to 1 sub-trait + its implementors, not the entire 100+-method surface
- Test mocks could implement only the sub-traits they need, reducing boilerplate
- SnapshotEntity could be decomposed into domain-specific sub-structs, each corresponding to a sub-trait
- Co-change radius would narrow: a combat-domain change would not force recompilation or review of social/economic code

**FOUNDATIONS alignment**:
- P14 (World State ≠ Belief State): **aligned** — sub-traits maintain the belief-only contract
- P26 (Systems Through State): **aligned** — trait decomposition does not introduce cross-system calls
- P7 (Locality): **aligned** — decomposition does not affect information locality
- P12 (Performance May Compress Computation, Never Causality): **strained** — Rust's trait object system may require `dyn RuntimeBeliefView` to remain a single trait for performance. Decomposing into multiple `dyn SubTrait` objects could add virtual dispatch overhead during GOAP plan search, which is performance-critical. This needs careful profiling.
- P28 (No Backward Compatibility): **aligned** — the change would replace the current trait, not add a compatibility layer

**Confidence**: Medium

**Counter-evidence**: 
1. **Rust trait object constraints**: If `RuntimeBeliefView` is used as `dyn RuntimeBeliefView` (which it is — `affordance_query.rs:10` takes `&dyn RuntimeBeliefView`), Rust cannot dynamically dispatch on a super-trait composed of sub-traits. The standard pattern (`trait A: SubA + SubB + SubC`) does not produce a usable `dyn A` if the sub-traits are not object-safe or if the caller needs type erasure. This may make the decomposition impractical without significant refactoring of all call sites to accept generic `impl RuntimeBeliefView` instead of `&dyn RuntimeBeliefView`.
2. **Co-change is expected contract evolution**: Every co-change represents a legitimate API surface growth. The trait IS the contract. When the contract evolves, both sides of the contract change. The temporal coupling may be measuring healthy API growth, not a fracture.
3. **Snapshot coherence requirement**: The GOAP planner's search tree forks hypothetical world state many times per tick. A fragmented snapshot (multiple sub-structs) may have worse cache locality than a single `SnapshotEntity` struct, affecting plan search performance.
4. **Macro already reduces duplication**: The `impl_goal_belief_view!` macro mechanically generates GoalBeliefView from RuntimeBeliefView, preventing the most obvious form of code duplication. Decomposing further may not add proportional benefit.

## Acceptable Architecture

### Action Registration Protocol (worldwake-systems → worldwake-sim)

The action registration pattern — where worldwake-systems modules define `register_*_action()` functions that populate `ActionDefRegistry` and `ActionHandlerRegistry` from worldwake-sim — is clean dependency-direction architecture. worldwake-sim provides the framework contracts (traits, registries, handler interfaces), and worldwake-systems provides the implementations. The temporal coupling between tick_step.rs and individual action modules (26–27 co-changes) reflects normal co-evolution of the execution engine and its action handlers.

### Core-to-AI Type Dependencies

AI planning files (goal_model.rs, planning_state.rs, candidate_generation.rs) each have 28–31 co-changes with worldwake-core/world.rs. This is expected: core defines the authoritative types (EntityId, CommodityKind, HomeostaticNeeds, GoalKind, etc.) that AI reasons about. When core types evolve, all consumers adapt. The dependency direction is correct (AI depends on core, not the reverse).

### System Decoupling (Principle 26 Compliance)

worldwake-systems modules depend only on worldwake-core and worldwake-sim, never on each other. Each action module (tell_actions.rs, trade_actions.rs, combat.rs, etc.) defines its own preconditions, handlers, and commit logic independently. No action module imports another. This is confirmed by import analysis and correctly implements P26 (systems interact through state, not through each other).

### Affordance Query Centralization

`get_affordances()` in worldwake-sim/affordance_query.rs provides a single centralized entry point for action eligibility enumeration. It takes `&dyn RuntimeBeliefView` and evaluates all registered action definitions against the agent's believed state. This correctly owns the "what can this agent do?" truth.

### Conservation and Causal Integrity

Conservation checking (`verify_authoritative_conservation`) and event log causal integrity are implemented in worldwake-core with no cross-crate authority leaks. The soak test validates these per-tick, confirming the invariants hold under extended autonomous play.

## Needs Investigation

### Tell-Actions ↔ Belief View Coupling (Single signal: temporal coupling)

`tell_actions.rs` (worldwake-systems) has 29 co-changes with `per_agent_belief_view.rs` (worldwake-sim) — unusually high for a single action handler compared to other action modules. This may indicate that the social/tell domain is growing faster than other domains and its belief surface is expanding disproportionately. However, only one evidence signal (temporal coupling) supports this — the import structure shows a normal dependency through `worldwake_sim::listener_aware_tell_topic_selection`. A second signal (e.g., duplicated belief predicates, or tell-specific methods dominating recent trait growth) would be needed to upgrade this to a fracture finding. **Second signal to look for**: Count the proportion of recently-added `RuntimeBeliefView` methods that serve tell/social specifically vs. other domains.

### Core belief.rs ↔ Systems perception.rs Coupling (Single signal: temporal coupling)

`worldwake-core/belief.rs` has 28 co-changes with `worldwake-systems/perception.rs`. This is expected (perception creates beliefs), but the frequency suggests the belief representation and perception logic may be evolving together faster than their crate boundary suggests. **Second signal to look for**: Whether perception.rs reaches into belief struct internals rather than using a clean write API.

## Recommendations

- **Needs investigation**: Belief View Domain Decomposition — the candidate has medium confidence with significant counter-evidence (Rust trait object constraints, performance concerns). Recommend a focused feasibility study: (1) audit how many call sites use `&dyn RuntimeBeliefView` vs. `impl RuntimeBeliefView`, (2) profile GOAP search sensitivity to trait object overhead, (3) prototype one sub-trait extraction to measure compile-time and co-change reduction. Only write a spec if the feasibility study shows the trait object constraint is surmountable.
- **Acceptable**: Action registration, system decoupling, affordance query, conservation/causal integrity, core-to-AI type dependencies — all architecturally sound.
- **Needs investigation**: Tell-actions ↔ belief view coupling, core belief ↔ systems perception coupling — single-signal observations needing a second signal before action.
- **Already identified (prior report)**: GoalDispatchDeclaration incomplete consolidation — acknowledged, not re-reported.

## Outcome

- Completion date: 2026-04-08
- What actually changed: Archived this report after its actionable abstraction analysis was exploited.
- Deviations from original plan: None. The report remains as a historical record and is no longer active planning material.
- Verification results: Confirmed the report was marked completed before archival and moved into `archive/reports/`.
