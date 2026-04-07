# Missing Abstraction Analysis: golden-soak

**Date**: 2026-04-07
**Input**: crates/worldwake-ai/tests/golden_soak.rs
**Source modules analyzed**: ~100
**Crates touched**: worldwake-core (28 modules), worldwake-sim (3 modules), worldwake-systems (36 modules), worldwake-ai (40 modules)

## Executive Summary

One incomplete abstraction was found: `GoalDispatchDeclaration` partially centralizes per-goal-kind static data but does not carry goal family policy or base priority class, forcing 3 additional exhaustive matches in separate files. All other concept clusters (location, affordance, inventory, pressure, plan lifecycle, combat, patrol, trade, needs, offices) show expected cross-cutting spread with clean per-crate ownership boundaries. The codebase is structurally healthy — only 3 workaround comments were found across ~100 source modules.

## Cluster Summary

| Cluster | Files | Crates | Scattered Matches | Repeated Predicates | Recomputation | Verdict |
|---------|-------|--------|-------------------|--------------------:|--------------:|---------|
| goal    | 10    | 1 (ai) | 4 source files     | 0                   | 0             | Incomplete |
| location | 60   | 4      | 0                  | 0                   | 0             | Acceptable |
| affordance | 42 | 2      | 0                  | 0                   | 0             | Acceptable |
| inventory | 36  | 3      | 0                  | 0                   | 0             | Acceptable |
| plan    | 8     | 1 (ai) | 0                  | 0                   | 0             | Acceptable |
| combat  | 23    | 3      | 0                  | 0                   | 0             | Acceptable |
| patrol  | 5     | 3      | 0                  | 0                   | 0             | Acceptable |
| trade   | 8     | 3      | 0                  | 0                   | 0             | Acceptable |
| needs   | 6     | 2      | 0                  | 0                   | 0             | Acceptable |
| pressure | 3    | 1 (ai) | 0                  | 0                   | 2 locations   | Acceptable |
| office  | 5     | 3      | 0                  | 0                   | 0             | Acceptable |

## Concept Clusters

### GoalKind Dispatch (Files: 10, Crates: 1)

**Modules**:
- `crates/worldwake-core/src/goal.rs` — defines `GoalKind` enum (34 variants)
- `crates/worldwake-ai/src/goal_dispatch_key.rs` — defines `GoalDispatchKey` (maps GoalKind → dispatch identity)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` — defines `GoalDispatchDeclaration` (centralizes trace_label, provenance_family, relevant_ops, invalidation_strategy, feasibility_strategy)
- `crates/worldwake-ai/src/goal_policy.rs` — defines `goal_family_policy()` (exhaustive match: suppression, penalty_interrupt, free_interrupt)
- `crates/worldwake-ai/src/goal_model.rs` — implements `GoalKindPlannerExt` trait methods: `is_progress_barrier()`, `relevant_observed_commodities()`, `build_payload_override()` (all exhaustive matches)
- `crates/worldwake-ai/src/ranking.rs` — `rank_goal_base_priority()` and `rank_provenance()` (exhaustive matches mapping GoalKind → GoalPriorityClass)
- `crates/worldwake-ai/src/candidate_generation.rs` — deduplication checks via `matches!` (4 instances)
- `crates/worldwake-ai/src/decision_trace.rs` — `PoliticalGoalFamily`, `BanditGoalFamily` enums for trace recording

**Key symbols**: `GoalKind`, `GoalDispatchKey`, `GoalDispatchDeclaration`, `GoalFamilyPolicy`, `GoalKindPlannerExt`, `GoalPriorityClass`

**Scattered match arms**:
- `goal_policy.rs:104-211` — exhaustive match mapping GoalKind → GoalFamilyPolicy (suppression/interrupt rules)
- `ranking.rs:290-461` — exhaustive match mapping GoalKind → GoalPriorityClass (base priority) and provenance
- `goal_model.rs:1041-1133` — exhaustive match mapping (GoalKind, PlannerOpKind) → bool for progress barriers
- `goal_model.rs:467-506` — exhaustive match mapping GoalKind → commodity set for observation

All four files match on GoalKind exhaustively, but each computes a **different static property** of the goal variant. The properties are: (1) policy (suppression/interrupt eligibility), (2) base priority class, (3) progress barrier identification, (4) observed commodities.

**FOUNDATIONS alignment**:
- P1 (Maximal Emergence): satisfied — goal dispatch does not prevent emergent composition; each concern is genuinely distinct
- P26 (Systems Through State): satisfied — goal properties are static lookup tables, not cross-system calls
- P27 (Derived Summaries Are Caches): satisfied — these are static properties of goal variants, not derived state
- P28 (No Backward Compatibility): satisfied — no shims or deprecated paths
- P19 (Agent Symmetry): satisfied — goal dispatch applies equally to all agents
- P20 (Resource-Bounded Reasoning): strained — adding a new GoalKind variant requires updating 4-5 files, increasing the risk of an incomplete addition; GoalDispatchDeclaration already centralizes 5 properties but leaves 3 others scattered

**Diagnosis**: Incomplete abstraction

**Rationale**: `GoalDispatchDeclaration` already centralizes 5 per-goal-kind static properties (trace_label, provenance_family, relevant_ops, invalidation_strategy, feasibility_strategy). Three additional static properties — `GoalFamilyPolicy`, base `GoalPriorityClass`, and progress barrier rules — remain in separate exhaustive matches. Since all are static functions of GoalKind (no runtime state required for the base values), they could be fields on `GoalDispatchDeclaration`, reducing the "new variant checklist" from 4-5 files to 2 (goal.rs for the variant, goal_dispatch_decl.rs for all static metadata). The Rust exhaustive-match compiler enforces completeness, which mitigates but does not eliminate the maintenance cost. The severity is moderate because the current design works correctly and the compiler prevents silent omissions.

---

### Location (Files: 60, Crates: 4)

Location checks (`effective_place`, `ground_location`, `get_component_location`) appear in 60 files across all 4 crates. This is expected: location is the most fundamental spatial property in a place-graph simulation. Every action handler must check co-location (P7 — locality of interaction), every perception system must know where entities are, and every planning module must reason about travel. The spread is wide but each usage is contextually appropriate — no repeated derived computation or scattered match logic on location state. This is architecturally correct for a locality-first simulation.

---

### Affordance / Action Eligibility (Files: 42, Crates: 2)

Affordance and eligibility checks appear across 42 files, primarily in `worldwake-systems` (25 files) and `worldwake-ai` (17 files). Each action module defines its own preconditions: `needs_actions.rs` checks hunger thresholds, `travel_actions.rs` checks edge existence, `trade_actions.rs` checks inventory and co-location. This is correct per P26 — systems interact through state, not through each other. Each action's preconditions are unique to that action; there is no repeated "is eligible" predicate computed identically in multiple locations. The `affordance_query.rs` module in `worldwake-sim` already centralizes the generic affordance enumeration interface.

---

### Inventory / Possession (Files: 36, Crates: 3)

Inventory checks (`possessor`, `commodity_quantity`, `has_commodity`) appear in 36 files. Like location, inventory is a fundamental property that nearly every action and planning module must access. Actions check whether agents have required items; planning simulates hypothetical inventory changes; perception observes what is present. Each access serves a different domain purpose. No repeated derived computation was found — modules query the raw component and apply their own domain-specific logic.

---

### Plan Lifecycle (Files: 8, Crates: 1)

The "plan" concept spans 8 files in worldwake-ai: `plan_selection.rs`, `planning_snapshot.rs`, `plan_revalidation.rs`, `planning_state.rs`, `planner_ops.rs`, `planner_duration_contract.rs`, `agent_tick/planning.rs`. Types include `PlannedPlan`, `PlannedStep`, `SelectionCandidatePlan`, `PlanningSnapshot`, `PlanningState`. These serve genuinely different lifecycle phases: search output (`PlannedPlan`), selection input (`SelectionCandidatePlan`), hypothetical world state during search (`PlanningState`), context snapshot (`PlanningSnapshot`). No scattered match arms or repeated predicates exist — each type has a clear role in the planning pipeline. The separation prevents coupling between search internals and selection logic.

---

### Pressure (Files: 3, Crates: 1)

`derive_danger_pressure()` and `derive_pain_pressure()` are called from `candidate_generation.rs` and `route_threat.rs`. Both compute pressure from the agent's current belief state. This is derived state recomputation in 2 locations — technically meeting the threshold, but correct per P27 (derived summaries are caches, never truth). Pressure depends on dynamic belief state that changes between calls, and caching would require invalidation tracking that exceeds the cost of recomputation. The functions are utility-style computations in a single file (`pressure.rs`), called from 2 consumers. This is clean functional decomposition, not a missing abstraction.

---

### Combat (Files: 23, Crates: 3)

`CombatProfile` access appears in 23 files (14 in AI, 9 in systems). The AI crate reads combat profiles during planning to assess threats; the systems crate reads them during combat resolution. Each crate owns its own concern. No repeated match arms or derived state recomputation — each module uses combat profile data for its specific purpose (planning cost estimation vs. actual damage calculation).

---

### Patrol, Trade, Needs, Offices (Files: 5-8 each, Crates: 2-3)

Each of these concepts follows the same clean pattern: types defined in `worldwake-core`, action handlers in `worldwake-systems`, and goal generation/ranking in `worldwake-ai`. Each crate owns its own concern, with no cross-crate function calls (P26 compliance). Types are shared through `worldwake-core` as the common dependency.

---

## Proposals

### P1: Consolidate GoalKind Static Metadata into GoalDispatchDeclaration

**Claim**: `GoalDispatchDeclaration` centralizes 5 per-goal-kind static properties but leaves 3 others (`GoalFamilyPolicy`, base `GoalPriorityClass`, progress barrier rules) in separate exhaustive matches across 3 files. All properties are static functions of the GoalKind variant with no runtime state dependency for their base values.

**Evidence**:
- `crates/worldwake-ai/src/goal_dispatch_decl.rs:42-48` — `GoalDispatchDeclaration` struct with 5 fields
- `crates/worldwake-ai/src/goal_policy.rs:104-211` — separate exhaustive match for `GoalFamilyPolicy`
- `crates/worldwake-ai/src/ranking.rs:290-461` — separate exhaustive match for base `GoalPriorityClass`
- `crates/worldwake-ai/src/goal_model.rs:1041-1133` — separate exhaustive match for `is_progress_barrier`

**FOUNDATIONS references**: P20 (Resource-Bounded Reasoning — adding goal variants should be tractable), P28 (No Backward Compatibility — if the centralized pattern exists, extend it rather than maintaining parallel dispatch paths)

**Proposed change**: Extend `GoalDispatchDeclaration` with three additional fields: `family_policy: GoalFamilyPolicy`, `base_priority_class: GoalPriorityClass`, and `progress_barrier_ops: &'static [PlannerOpKind]`. Move the exhaustive matches from `goal_policy.rs`, `ranking.rs`, and `goal_model.rs` into the per-key declaration table in `goal_dispatch_decl.rs`. Consumers would call `GoalDispatchKey::from_goal_kind(kind).declaration().family_policy` instead of `goal_family_policy(kind)`. The `relevant_observed_commodities` and `build_payload_override` methods should remain on `GoalKindPlannerExt` since they require runtime inputs beyond the static variant.

**Priority**: Medium — the Rust compiler enforces exhaustive match completeness, which prevents silent errors when adding new variants. The maintenance cost is real but bounded. The benefit is reducing the "add a new goal kind" checklist from 4-5 files to 2 files.

---

## Acceptable Clusters

### Location

Location appears in 60 files because co-location is the fundamental precondition for all physical interaction in a place-graph simulation (P7). Every action, perception event, and planning operation must reason about where entities are. The spread reflects correct application of the locality principle, not scattered logic.

### Affordance / Action Eligibility

Affordance checks appear in 42 files because each action defines its own unique preconditions. This is correct per P26 — systems interact through state, not through each other. The generic affordance enumeration interface in `affordance_query.rs` already provides the centralized discovery mechanism.

### Inventory / Possession

Inventory access appears in 36 files because items with persistent identity and explicit transfer (P4) are central to trade, production, combat loot, theft, and consumption. Each module accesses inventory for its own domain-specific purpose.

### Plan Lifecycle

8 files with distinct plan types serve genuinely different pipeline stages: search, selection, execution. No type confusion or repeated logic exists between stages.

### Pressure

Pressure recomputation in 2 locations is correct per P27 — derived summaries that depend on dynamic belief state should be recomputed, not cached as truth.

### Combat / Patrol / Trade / Needs / Offices

Each follows the clean 3-crate pattern: types in core, handlers in systems, AI reasoning in ai. Crate boundaries match concern boundaries.
