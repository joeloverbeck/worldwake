# Architectural Debt Analysis: golden_resilience

**Status**: COMPLETED
**Date**: 2026-04-12
**Input**: crates/worldwake-ai/tests/golden_resilience.rs
**Source modules analyzed**: 206
**Crates touched**: worldwake-core (71), worldwake-sim (48), worldwake-systems (37), worldwake-ai (50)
**Prior reports consulted**: none

## Executive Summary

The golden_resilience test suite (T31 stress + T32 replay consistency) exercises all 206 source modules across 4 crates via `step_once()` simulation loops with periodic disruptions. Two detection lenses (structural scatter and architectural fractures) were applied and their findings validated against source code.

One Medium-severity finding was confirmed: commodity validation helpers (`ensure_accessible_quantity`, `resolve_controlled_lots`) are duplicated across 4 action handler modules within `worldwake-systems`. Three additional signals were investigated and resolved as acceptable architecture. Multiple high-signal claims from initial detection (is_alive/is_dead duplication, boundary inversion in feasibility, GoalKind overloading, CommodityKind scatter) were falsified during validation — they reflected correct trait polymorphism, intentional layering, and normal type usage rather than architectural debt.

The codebase exhibits strong architectural health: clean crate boundaries, zero cross-system coupling in worldwake-systems, well-structured belief/authoritative separation via the `GoalBeliefView` trait, and declarative goal dispatch via `GoalDispatchKey`.

## Scenario Families

| Family | Tests | Domain Concepts | Key Assertions |
|--------|-------|----------------|----------------|
| Disruption injection protocol | T31 | death, destruction, removal, teleportation, WorldTxn | Disruption types cycle deterministically; no panics during injection |
| Conservation enforcement | T31 | CommodityKind, ItemLot, Quantity, archive | `verify_authoritative_conservation` passes every tick; baseline adjusts for destroyed lots |
| Needs bounds enforcement | T31 | HomeostaticNeeds, Permille | All need values stay within [0, 1000] for living agents |
| Dead agent inactivity | T31 | DeadAt, active_action | Dead agents never have active actions |
| Unique placement | T31 | effective_place, topology | Living agents always at valid topology places |
| Deterministic replay / save-load fidelity | T31 (roundtrip), T32 (split replay) | hash_world, hash_event_log, save_to_bytes, load_from_bytes | Continuous run and split run produce identical checkpoint hashes at every 100-tick boundary |

## Traceability Summary

| Module | Scenario Families | Confidence | Strategy |
|--------|------------------|------------|----------|
| worldwake-systems/src/trade_actions.rs | Conservation, Disruption | High | use + temporal |
| worldwake-systems/src/artifact_actions.rs | Conservation, Disruption | High | use + temporal |
| worldwake-systems/src/justice_actions.rs | Conservation, Disruption | High | use + naming |
| worldwake-systems/src/office_actions.rs | Conservation, Disruption | High | use + naming |
| worldwake-core/src/conservation.rs | Conservation | High | use (verify_authoritative_conservation) |
| worldwake-core/src/needs.rs | Needs bounds | High | use (HomeostaticNeeds) |
| worldwake-core/src/world.rs | All families | High | use (effective_place, is_alive, hash_world) |
| worldwake-sim/src/tick_step.rs | All families | High | use (step_once) |
| worldwake-sim/src/save_load.rs | Replay, Save-load | High | use (save_to_bytes, load_from_bytes) |
| worldwake-sim/src/scheduler.rs | All families | High | use (current_tick) |
| worldwake-ai/src/feasibility.rs | Disruption (replanning) | Medium | naming + call graph |
| worldwake-ai/src/agent_tick/mod.rs | Disruption (replanning) | Medium | naming + temporal |
| worldwake-core/src/world/ownership.rs | Conservation, Disruption | Medium | temporal (5 co-changes with AI tests) |
| worldwake-sim/src/belief_view.rs | Disruption (belief-based decisions) | Medium | naming + trait |
| worldwake-sim/src/action_validation.rs | All action families | High | use (constraint evaluation) |

## Findings

### F1: Commodity Validation Helper Duplication in worldwake-systems

**Lens Source**: Lens A (Structural Scatter)
**Kind**: Projection owner
**Severity**: Medium
**Confidence**: High
**Scope**: worldwake-systems (4 action handler modules)

**Owned truth**: "Whether a holder has sufficient accessible quantity of a commodity for an action to proceed, and the resolution of controlled lots to satisfy a transfer."

**Invariants**: The accessible quantity check must agree with `World::controlled_commodity_quantity` semantics. The lot resolution must transfer exactly the requested quantity from controlled lots, never more, never less. Conservation (P4) must hold across the transfer.

**Owner boundary**: A shared `pub(crate)` module in worldwake-systems (e.g., `commodity_support.rs` or similar).

**Evidence**:
- `crates/worldwake-systems/src/artifact_actions.rs:760` — `fn ensure_accessible_quantity(&WorldTxn, holder, commodity, quantity)` checks `txn.controlled_commodity_quantity(holder, commodity) >= quantity`
- `crates/worldwake-systems/src/justice_actions.rs:661` — `fn ensure_accessible_quantity(&World, holder, commodity, quantity)` — identical logic, different first parameter type
- `crates/worldwake-systems/src/trade_actions.rs:1239` — same function, same logic
- `crates/worldwake-systems/src/office_actions.rs:1138` — same function, same logic
- `crates/worldwake-systems/src/artifact_actions.rs:779` — `fn resolve_controlled_lots` duplicated in artifact_actions, justice_actions, office_actions (3 files)

**Modules affected**: artifact_actions.rs, justice_actions.rs, trade_actions.rs, office_actions.rs

**Scenario families explained**: Conservation enforcement (all 4 modules participate in commodity transfers validated by T31's per-tick conservation checks), Disruption injection (item destruction adjusts baselines that these functions check against)

**Expected simplification**: Extract `ensure_accessible_quantity` and `resolve_controlled_lots` into a shared `pub(crate)` module. The `&WorldTxn` vs `&World` parameter difference is resolvable since `WorldTxn` exposes `controlled_commodity_quantity` through the same query path. Eliminates 4 copies of validation logic and 3 copies of lot resolution logic.

**FOUNDATIONS alignment**:
- P4 (Persistent Identity and Explicit Transfer): aligned — the duplication does not violate P4, but consolidation reduces the risk of one copy diverging from conservation semantics
- P26 (Systems Interact Through State, Not Through Each Other): aligned — the duplicated functions read shared state, not each other; consolidation preserves this
- P27 (Derived Summaries Are Caches, Never Truth): aligned — `controlled_commodity_quantity` is a derived query on authoritative state; the finding is about duplicated invocation, not about storing derived state as truth

**Counter-evidence**: The `&WorldTxn` vs `&World` parameter difference could indicate that the functions need access to in-flight transaction state that differs from committed world state. If `WorldTxn::controlled_commodity_quantity` accounts for pending mutations while `World::controlled_commodity_quantity` does not, then the duplication may be intentional. Verify whether `WorldTxn` delegates to `World` for this query or adds transaction-scoped adjustments. If identical, extraction is safe. If divergent, extraction requires a trait or generic parameter.

---

## Acceptable Architecture

### sim/systems Module Independence (zero within-crate hotspots)

The 37 modules in `worldwake-systems` exhibit zero git co-change hotspots (no file pair changed together in 3+ commits over 6 months). Each action handler module (`trade_actions.rs`, `combat.rs`, `patrol_actions.rs`, etc.) is a standalone unit that depends only on `worldwake-core` and `worldwake-sim`, never on sibling modules. This is the design intent of Principle 12 (system decoupling) and the codebase enforces it effectively.

### GoalBeliefView Trait Separation (correct polymorphism, not duplication)

Initial detection flagged 30+ `is_alive`/`is_dead` implementations across 4 crates as "predicate duplication." Validation revealed these are all implementations of the `GoalBeliefView` trait (`crates/worldwake-sim/src/belief_view.rs:69`), which defines a unified interface for querying agent state from different data sources:
- `PerAgentBeliefView` (belief-based, per-agent): reads from `AgentBeliefStore`
- Authoritative implementations: read from `World` directly
- Test stubs: return hardcoded values for isolated unit tests

This is correct Rust trait polymorphism implementing P14 (World State Is Not Belief State). The multiplicity of implementations is architecturally necessary, not accidental scatter.

### Three-Layer Validation Pipeline (stratified, not redundant)

Initial detection flagged the existence of validation logic in three layers as a "split protocol." Validation confirmed these layers serve genuinely different purposes:
1. **AI feasibility** (`feasibility.rs`): Cheap belief-based heuristics before GOAP search. Explicitly operates on `GoalBeliefView`, never authoritative state. Returns `Likely/Uncertain/Unlikely` hints to reorder candidates — never excludes goals.
2. **sim constraints** (`action_validation.rs`): Authoritative constraint evaluation at action start time. Checks actor alive, not incapacitated, has control, at correct place, has required commodities.
3. **systems handlers** (individual `*_actions.rs`): Domain-specific validation within action execution. Checks business rules specific to each action type (e.g., trade negotiation state, escort payload validity).

Each layer has a different data source (beliefs vs. authoritative state), different timing (pre-search vs. action-start vs. mid-execution), and different consequences (reordering vs. rejection vs. abort). This is stratified validation, not authority confusion.

### GoalDispatchKey Declarative Dispatch (anti-scatter mechanism)

Initial detection flagged `GoalKind` (~30 variants) matched across multiple AI files as "scattered match arms." Validation revealed that `GoalDispatchKey` (`crates/worldwake-ai/src/goal_dispatch_key.rs`) provides a declarative dispatch table mapping each `GoalKind` to a `GoalDispatchDeclaration` containing `FeasibilityStrategy`, candidate generation parameters, and planner transition kinds. This centralizes the per-goal-type configuration that would otherwise require exhaustive match blocks in each consumer. The remaining matches in `goal_model.rs`, `ranking.rs`, and `candidate_generation.rs` each serve distinct purposes (state transitions, priority computation, candidate deduplication) with no duplicated logic between them.

### Stable sim Crate (zero internal hotspots, zero cross-crate coupling)

`worldwake-sim` (48 modules) shows zero internal git co-change hotspots and no cross-crate coupling with `worldwake-systems`. The action framework (`ActionDef`, `ActionHandler`, `ActionHandlerRegistry`) provides a clean registration-based architecture that systems modules plug into without direct coupling.

## Needs Investigation

| Signal | Type Suspected | One Signal Found | Second Signal to Look For |
|--------|---------------|-----------------|--------------------------|
| AI/Core git co-change coupling (59 commits) | Hidden seam | `world/ownership.rs` co-changes with AI test and planning files | Check if ownership semantics changes consistently require AI-side adjustments beyond test updates; if so, may indicate ownership API is too low-level for AI's needs |
| `GoalKind::apply_planner_step` dispatch (goal_model.rs) | Overloaded abstraction | Single method on GoalKind handles all ~30 variants' planner state transitions | Check if subsets of variants share identical transition logic that could be factored into shared implementations |
| TODO/FIXME density in worldwake-ai (53 markers across 6 files) | Workaround indicators | `search/tests.rs` (16), `planner_ops.rs` (16), `enterprise.rs` (6) | Classify TODOs: test-only annotations vs. production workarounds; production TODOs may indicate missing abstractions |

## Proposals

### P1: Extract Shared Commodity Validation Helpers in worldwake-systems

**Claim**: The `ensure_accessible_quantity` function is duplicated as a private function in 4 action handler modules within worldwake-systems, and `resolve_controlled_lots` is duplicated in 3. All copies implement the same logic (check controlled commodity quantity against a threshold, then resolve lot transfers). The duplication creates a maintenance risk: a bugfix or semantic change to commodity validation in one module may not propagate to the others.

**Evidence**:
- `crates/worldwake-systems/src/artifact_actions.rs:760` — `ensure_accessible_quantity` (takes `&WorldTxn`)
- `crates/worldwake-systems/src/justice_actions.rs:661` — `ensure_accessible_quantity` (takes `&World`)
- `crates/worldwake-systems/src/trade_actions.rs:1239` — `ensure_accessible_quantity`
- `crates/worldwake-systems/src/office_actions.rs:1138` — `ensure_accessible_quantity`
- `crates/worldwake-systems/src/artifact_actions.rs:779` — `resolve_controlled_lots` (also in justice_actions, office_actions)

**FOUNDATIONS references**: P4 (Persistent Identity and Explicit Transfer), P26 (Systems Interact Through State)

**Proposed change**: A spec should extract `ensure_accessible_quantity` and `resolve_controlled_lots` into a shared `pub(crate)` module in `worldwake-systems`. The spec should resolve the `&WorldTxn` vs `&World` parameter variance (likely via a shared trait or by standardizing on `&WorldTxn`). No cross-crate changes needed.

**Priority**: Medium

## Codebase Health Observations

- **Low workaround density in production code**: The "fallback" references in `planner_ops.rs` are a named planner transition kind (`PlannerTransitionKind::GoalModelFallback`), not workarounds. TODO/FIXME markers concentrate in test files and are mostly test improvement notes, not production debt.
- **Zero cross-system coupling**: The `worldwake-systems` crate's 37 modules show no internal dependency hotspots and no sibling imports — each action handler is fully independent.
- **Clean crate boundary surfaces**: Git co-change analysis found only AI-Core coupling (natural, along the dependency arrow). sim-systems, sim-core, and systems-core show zero coupling hotspots.
- **Effective declarative dispatch**: `GoalDispatchKey` + `GoalDispatchDeclaration` eliminate the need for scattered GoalKind match arms that would otherwise exist across the AI crate.
- **Correct belief/authoritative layering**: The `GoalBeliefView` trait enforces P14 at the type level — AI code cannot accidentally access authoritative state because the trait interface only exposes belief-based queries.

## Outcome

Completed on 2026-04-12.

- The confirmed Medium-severity finding (duplicated commodity validation helpers in `worldwake-systems`) was exploited through `S99` and implemented via the shared `commodity_support` module plus caller migration in `artifact_actions.rs`, `justice_actions.rs`, `trade_actions.rs`, and `office_actions.rs`.
- The corresponding spec and ticket were archived at `archive/specs/S99-commodity-validation-helpers.md` and `archive/tickets/S99COMVALHEL-001.md`.
- No additional findings from this report were promoted into follow-up implementation work during this session because the remaining signals were validated as acceptable architecture or left in the report's explicit "Needs Investigation" bucket.
