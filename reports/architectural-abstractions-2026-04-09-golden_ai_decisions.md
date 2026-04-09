# Architectural Abstraction Recovery: golden_ai_decisions

**Date**: 2026-04-09
**Input**: `crates/worldwake-ai/tests/golden_ai_decisions.rs`
**Source modules analyzed**: 203 (short-circuit: all modules in referenced crates)
**Crates touched**: worldwake-core (66 files), worldwake-sim (50 files), worldwake-systems (38 files), worldwake-ai (49 files)
**Prior reports consulted**: none

## Executive Summary

The golden AI decisions test suite exercises the full agent decision pipeline — from need pressure through goal generation, planning, action execution, and failure recovery — across all four crates. Cross-subsystem fractures are **mild**. The belief view protocol stack (GoalBeliefView / RuntimeBeliefView stratification) and the PlanningState shadow state machine are well-architected boundaries. Two validated fractures were found: a projection drift where need band classification is independently reimplemented in three AI files, and an authority leak where BlockingFact is defined in core but populated entirely by the AI crate. One additional low-severity split protocol was detected in the goal dispatch declaration/logic boundary. Two candidate abstractions survived validation; one is spec-worthy.

## Scenario Families

| Family | Tests | Domain Concepts | Key Assertions |
|--------|-------|----------------|----------------|
| Goal invalidation by resource competition | 1 | GoalKind, conservation, multi-agent | Alice eats bread; Bob doesn't acquire bread; bread conservation holds |
| Frontier exhaustion isolation | 2 (+ determinism replay) | ExhaustionEntry, OpportunityKey, invalidation conditions | Unrelated commodity change preserves Bob's exhausted apple entry |
| Exhausted opportunity fallthrough | 2 (+ determinism replay) | OpportunityKey, sibling sources, suppression | Exhausted local opportunity suppressed; remote sibling selected |
| Priority-based interrupt | 1 | Metabolism, interrupt, goal switching | Sleep interrupted when hunger reaches critical; agent eats |
| Blocked intent with TTL expiry | 1 | ResourceSource depletion, regeneration, BlockedIntentMemory | Depleted source eventually regenerates; agent harvests |
| Deprivation cascade (hunger/thirst/wash) | 3 | Metabolism tick, threshold crossing, commodity consumption | Need escalation triggers goal generation and action |
| Three-way need competition | 1 | UtilityProfile weights, multi-need ranking, action ordering | Highest-weighted need (hunger) addressed first; fatigue last |
| Bladder relief with travel | 1 | Travel, latrine facility, waste production, dirtiness | Agent travels to latrine; waste materializes at destination |
| Goal switching during multi-leg travel | 1 | Multi-leg travel, thirst escalation, commitment suspension/reactivation | Travel continues through medium/high thirst; interrupts at critical |
| Multi-hop travel planning | 3 (+ determinism replay) | Spatial planning, travel pruning, harvest lifecycle | Agent navigates place graph to remote orchard; harvests; reduces hunger |
| Utility weight diversity | 1 | UtilityProfile divergence, enterprise restock, Principle 20 | HungerDriven eats locally; EnterpriseDriven leaves for restock |
| Fallback to addressable needs | 1 | Unsatisfiable top need, fallback ranking, zero-step prevention | Agent with no water falls back to eating; no indefinite idle |

## Traceability Summary

Since the short-circuit rule applies (all modules exercised via `step_once()` loops), this table focuses on modules **uniquely relevant** to specific scenario families rather than exhaustively listing all 203 files.

| Module | Scenario Families | Confidence | Strategy |
|--------|------------------|------------|----------|
| `ai/candidate_generation.rs` | All 12 families (goal emission entry point) | High | use + call graph |
| `ai/ranking.rs` | Three-way need competition, utility weight diversity, fallback | High | use + naming |
| `ai/exhaustion.rs` | Frontier exhaustion isolation, exhausted opportunity fallthrough | High | use + assertion pattern |
| `ai/goal_model.rs` | All planning families (satisfaction, binding, payload) | High | use + call graph |
| `ai/plan_revalidation.rs` | Blocked intent, goal switching during travel | High | use + temporal coupling |
| `ai/interrupts.rs` | Priority-based interrupt, goal switching during travel | High | use + naming |
| `ai/failure_handling.rs` | Blocked intent with TTL expiry, fallback to addressable | High | use + naming |
| `ai/agent_tick/planning.rs` | All planning families | High | use + call graph |
| `ai/agent_tick/execution.rs` | All action-producing families | High | use + call graph |
| `sim/per_agent_belief_view.rs` | All families (belief source for AI) | High | temporal coupling (55 commits with candidate_generation) |
| `sim/belief_view.rs` | All families (trait stack definition) | High | temporal coupling (51 commits) |
| `sim/affordance_query.rs` | All action families (constraint checking) | High | temporal coupling (33 commits with planning_state) |
| `sim/tick_step.rs` | All families (tick execution) | High | use + call graph |
| `core/needs.rs` | Deprivation cascade, three-way, priority interrupt | High | use + naming |
| `core/goal.rs` | All families (GoalKind definition) | High | use |
| `systems/needs_actions.rs` | Deprivation cascade, three-way, bladder relief | High | naming + temporal |
| `systems/production_actions.rs` | Blocked intent, multi-hop travel | High | naming + assertion |
| `systems/travel_actions.rs` | Bladder relief, goal switching, multi-hop | High | naming + assertion |

## Fracture Summary

| # | Fracture Type | Location | Evidence Sources | Severity |
|---|--------------|----------|-----------------|----------|
| 1 | Projection drift | `ai/ranking.rs`, `ai/exhaustion.rs`, `ai/pressure.rs` | Code analysis (3 files with `classify_band`/`classify_need_band`/`need_threshold_band`) + temporal coupling (ranking↔exhaustion co-change) | MEDIUM |
| 2 | Authority leak | `core/blocked_intent.rs` (definition), `ai/failure_handling.rs` (population), `systems/trade_actions.rs` (population) | Code analysis (enum in core, variant construction in ai+systems) + grep (18 files reference `BlockingFact::` variants, 15 in ai, 1 in core, 1 in systems) | MEDIUM |
| 3 | Split protocol | `ai/goal_dispatch_decl.rs` (metadata), `ai/exhaustion.rs` (logic) | Code analysis (InvalidationStrategy enum vs derive_invalidation_conditions match) + temporal coupling (both files co-change) | LOW |

## Candidate Abstractions

### Need Band Oracle

**Kind**: Projection owner
**Scope**: worldwake-ai (internal to AI crate; could be promoted to core if sim needs it)
**Fractures addressed**: Fracture #1 (projection drift)

**Owned truth**: The mapping from (HomeostaticNeeds, DriveThresholds) → ThresholdBand for each need dimension. Currently this classification is reimplemented independently in `ranking.rs` (for priority class computation), `exhaustion.rs` (for band boundary crossing detection), and `pressure.rs` (for pressure scoring). Each has slightly different function signatures but performs the same conceptual operation: given a need value and thresholds, return which band (Low/Medium/High/Critical) the value falls in.

**Invariants**:
- Band classification is monotonic: higher need values never map to lower bands
- Band boundaries come exclusively from DriveThresholds (never hardcoded)
- The same (need_value, thresholds) input always produces the same band output across all call sites

**Owner boundary**: A single `need_band_oracle` module within worldwake-ai, or a method on DriveThresholds in worldwake-core if the classification is needed outside AI.

**Modules affected**: `ai/ranking.rs` (remove local classify_band), `ai/exhaustion.rs` (remove classify_need_band/need_threshold_band), `ai/pressure.rs` (remove local band helpers)

**Tests explained**: Deprivation cascade (threshold crossing triggers goal), priority-based interrupt (band escalation triggers interrupt), three-way need competition (relative band positions drive ordering), fallback to addressable needs (band classification determines fallback eligibility)

**Expected simplification**: Three independent band classification implementations → one canonical implementation. Changes to band classification logic (e.g., adding a new band, changing boundary semantics) would require updating one location instead of three. Reduces risk of subtle divergence where ranking classifies a band differently than exhaustion invalidation.

**FOUNDATIONS alignment**:
- P3 (Concrete State Over Abstract Scores): Aligned — band classification derives from concrete need values and thresholds, not abstract scores
- P27 (Derived Summaries Are Caches, Never Truth): Aligned — band classification is a derived view over concrete HomeostaticNeeds + DriveThresholds
- P26 (Systems Interact Through State): Neutral — this is internal to the AI crate, not a cross-system interaction

**Confidence**: Medium
**Counter-evidence**: If the three implementations are intentionally different (e.g., ranking uses coarser bands than exhaustion for performance reasons, or pressure uses a weighted variant), then centralization would be incorrect. Verify by diffing the actual classification logic in all three files — if the band boundaries are identical, centralization is warranted; if they differ intentionally, each serves a distinct purpose and this is not a fracture but deliberate specialization.

---

### Failure Diagnosis Protocol

**Kind**: Authority boundary
**Scope**: worldwake-core (definition) ↔ worldwake-ai (population) ↔ worldwake-systems (population)
**Fractures addressed**: Fracture #2 (authority leak)

**Owned truth**: The classification of why an action failed to start or was aborted — the `BlockingFact` enum. Currently defined in `worldwake-core/src/blocked_intent.rs` (the enum with ~15 variants like `TargetGone`, `NoKnownPath`, `ResourceDepleted`, etc.), but the actual derivation logic — determining which `BlockingFact` variant matches a given failure — lives in `worldwake-ai/src/failure_handling.rs` and `worldwake-systems/src/trade_actions.rs`.

The issue: AI inspects sim-level failure reasons (`ActionStartFailure`, `AbortReason`, `ExternalAbortReason`) and maps them to `BlockingFact` variants. This means AI must understand the semantics of sim-level failures to produce correct blocking classifications. When sim adds a new failure reason, AI must be updated to classify it correctly.

**Invariants**:
- Every `ActionStartFailure` reason maps to exactly one `BlockingFact` variant (no ambiguous classifications)
- `BlockingFact` derivation does not require access to world state beyond what the failure reason already encodes
- Blocking facts are agent-local beliefs about failure causes, not authoritative world state

**Owner boundary**: The mapping from failure reasons → `BlockingFact` should live at the boundary where failure reasons are produced — either in `worldwake-sim` (as a method on `ActionStartFailure`/`AbortReason`) or as a trait implementation that sim provides and AI consumes.

**Modules affected**: `ai/failure_handling.rs` (remove BlockingFact derivation logic), `sim/action_validation.rs` or `sim/action_execution.rs` (add BlockingFact derivation), `systems/trade_actions.rs` (use sim-provided derivation)

**Tests explained**: Blocked intent with TTL expiry (failure → blocked intent memory → eventual retry), fallback to addressable needs (unsatisfiable goal → blocking → fallback), goal invalidation (competition → failure → re-planning)

**Expected simplification**: AI no longer needs to interpret sim-level failure semantics. New failure reasons added in sim automatically come with their BlockingFact classification. Fewer cross-crate knowledge dependencies for failure handling.

**FOUNDATIONS alignment**:
- P26 (Systems Interact Through State, Not Through Each Other): Strained — currently AI must understand sim internals to classify failures. Moving derivation to sim aligns better with state-mediated interaction.
- P14 (World State Is Not Belief State): Aligned — BlockingFact is agent-local belief about why something failed, not world truth. But the derivation currently requires AI to interpret world-state-level failure reasons.
- P29 (Debuggability): Aligned — centralizing failure classification improves traceability of "why did the agent think this failed?"

**Confidence**: Medium
**Counter-evidence**: If the BlockingFact classification is intentionally agent-subjective (i.e., different agents should classify the same failure differently based on their beliefs), then it correctly belongs in AI. Check whether any `BlockingFact` derivation in `failure_handling.rs` reads agent belief state to determine the classification. If it only reads the failure reason itself (which comes from sim), the derivation belongs in sim. If it also considers agent beliefs or cognitive profile, it belongs in AI as a belief-formation process.

## Acceptable Architecture

### Belief View Protocol Stack
The stratified trait hierarchy (`GoalBeliefView` for narrow candidate generation, `RuntimeBeliefView` for full planning) is a well-designed boundary. The narrowing prevents AI candidate generation from accidentally depending on queue/reservation state. The automatic impl coercions allow composability without coupling. Temporal coupling (55 commits) reflects expected co-evolution of interface and implementation, not a design flaw. **No intervention needed.**

### PlanningState Shadow State Machine
The layered override architecture (`PlanningSnapshot` as immutable base + `SharedMap` override layers for speculative mutations) correctly separates authoritative truth from hypothetical exploration. AI cannot modify sim state; only `PlanningState` instances mutate their own shadows. The `RuntimeBeliefView` trait implementation allows search code to use the same interface for both real and speculative state. **No intervention needed.**

### Action Request → Validation → Execution Protocol
The flow from AI enqueuing `RequestAction` → sim resolving affordances → authoritative constraint validation → handler execution is clean. AI makes no validation claims; sim is the sole validator. The naming of `enqueue_valid_step_or_handle_failure()` is misleading (it doesn't validate), but this is cosmetic, not architectural. **No intervention needed.**

### Interrupt Decision / Execution Separation
`evaluate_interrupt()` in AI is a pure function returning `InterruptDecision`. Execution is delegated to `Scheduler.interrupt_active_action()` in sim. Decision authority is clean: AI decides when to interrupt based on beliefs, sim executes the interruption. **No intervention needed.**

### Goal Lifecycle Pipeline
The `GoalKind` (core) → `GoalDispatchKey` (ai) → `GoalDispatchDeclaration` (ai) → `GroundedGoal` → `RankedGoal` pipeline is complex but each stage adds genuine information. Core owns the kind taxonomy; AI owns dispatch, ranking, and planning semantics. The split is load-bearing: core should not depend on AI planning concepts. **No intervention needed.**

### Exhaustion Cache with Invalidation Conditions
The `ExhaustionEntry` → `ExhaustionInvalidationCondition` → baseline comparison system is well-designed for its purpose: preventing re-planning of known-exhausted opportunities while allowing invalidation when relevant state changes. The isolation test (Scenario 1b) directly validates this boundary. **No intervention needed.**

## Needs Investigation

| Signal | Fracture Type Suspected | One Signal Found | Second Signal to Look For |
|--------|------------------------|-----------------|--------------------------|
| `GoalPriorityClass` lives entirely in AI (21 files) | Possible boundary inversion | GoalPriorityClass defined in `ai/goal_model.rs` but semantically encodes survival-level criticality that could be a core/sim concern | Check if any sim-level system ever needs to know an agent's goal priority class. If sim only sees action requests and never priority, it's correctly AI-local. |
| `goal_model.rs` is ~4000+ LOC with 30+ match arms per method | Possible overloaded abstraction | Single file carries satisfaction, binding, availability, payload override, spatial guidance — 6+ lifecycle roles | Check if any lifecycle role could be extracted to a dedicated module without increasing cross-module coupling. If match arms share significant context, splitting increases complexity. |
| `tell_actions.rs` (systems) co-changes with 5+ AI files (35 commits with planning_state) | Possible hidden seam | High temporal coupling between systems/tell_actions.rs and ai/ | Check whether tell_actions introduces new action types that require corresponding goal dispatch declarations. If so, the coupling is inherent to the action registration protocol, not a seam. |

## Recommendations

- **Spec-worthy**: Failure Diagnosis Protocol — the authority leak is a genuine cross-crate boundary issue that will compound as new failure reasons are added
- **Worth consolidating**: Need Band Oracle — not spec-worthy on its own (it's internal to worldwake-ai), but worth a focused cleanup when next touching ranking or exhaustion code
- **Acceptable**: Belief view protocol, PlanningState, action execution flow, interrupt separation, goal lifecycle pipeline, exhaustion cache
- **Needs investigation**: GoalPriorityClass boundary placement, goal_model.rs complexity, tell_actions temporal coupling
