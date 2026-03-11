# Pre-E14 Hardening Spec

## Scope

Harden and clean up the Phase 1+2 foundation (E01–E13) before E14 (Perception & Belief System) changes the landscape. The golden e2e test (`crates/worldwake-ai/tests/golden_e2e.rs`) proves the architecture works end-to-end; this spec addresses extensibility friction, robustness gaps, performance inefficiencies, and test coverage weaknesses discovered during E13 implementation.

**Non-goals**: No new features, no new domain systems, no new agent capabilities. This is purely structural improvement.

## Constraints

- All changes must preserve **identical golden e2e state hashes** (determinism invariant)
- `cargo test --workspace` and `cargo clippy --workspace` must pass after each ticket
- No new external dependencies (except optionally `im` for HARDEN-C03)
- No changes to public API signatures that would break downstream crates unless the ticket explicitly calls for it

## Foundations Alignment

| Principle | Relevance |
|-----------|-----------|
| P12 (System Decoupling) | A01, A02, A03 — search core should not contain domain-specific goal logic; candidate generators for different domains should be independent |
| P3 (Concrete State) | B04 — observation snapshots should track concrete, goal-relevant state changes, not abstract "anything changed" signals |
| P7 (Locality) | B04 — agents should only replan when locally-relevant information changes |

---

## Category A: Architecture Extensibility

### HARDEN-A01: Extract GoalSemantics trait from search.rs

**Problem**: Adding a new `GoalKind` requires editing 3 match blocks deep in the search core: `goal_is_satisfied()` (lines 330–380), `build_payload_override()` (lines 169–234), and `progress_barrier()` (lines 311–328). This violates Principle 12 — the search algorithm should not know domain-specific goal logic.

**Deliverables**:
1. New file `crates/worldwake-ai/src/goal_semantics.rs` defining a `GoalSemantics` trait with methods:
   - `fn is_satisfied(&self, goal: &GroundedGoal, state: &PlanningState<'_>) -> bool`
   - `fn build_payload_override(&self, affordance_payload: Option<&ActionPayload>, state: &PlanningState<'_>, targets: &[EntityId], def: &ActionDef, semantics: &PlannerOpSemantics) -> Result<Option<ActionPayload>, ()>`
   - `fn is_progress_barrier(&self, goal: &GroundedGoal, step: &PlannedStep) -> bool`
2. Per-`GoalKind` implementations (can be match-based internally, but registered externally to search)
3. `search_plan()` accepts `&dyn GoalSemantics` instead of inlining the logic
4. Existing behavior preserved exactly — this is a pure refactor

**Files**: `crates/worldwake-ai/src/search.rs`, new `crates/worldwake-ai/src/goal_semantics.rs`, `crates/worldwake-ai/src/lib.rs` (re-export)

**Verification**: All existing search tests pass unchanged. Golden e2e hashes identical.

---

### HARDEN-A02: Modularize candidate generation

**Problem**: `generate_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs` is a monolithic function that mixes need-based, enterprise, combat, and production candidate logic. Adding a new domain (e.g., social goals in E15+) requires editing this single function.

**Deliverables**:
1. Break the monolithic function into per-domain generators:
   - `generate_need_candidates()` — hunger, thirst, sleep, bladder, dirtiness, wash
   - `generate_enterprise_candidates()` — restock, sell, produce
   - `generate_combat_candidates()` — danger reduction, looting, healing
   - `generate_production_candidates()` — recipe-driven production goals
2. Top-level `generate_candidates()` becomes an orchestrator calling each sub-generator
3. Each sub-generator is a private function in the same module (no new files needed)

**Files**: `crates/worldwake-ai/src/candidate_generation.rs`

**Verification**: All candidate generation tests pass. Golden e2e hashes identical.

---

### HARDEN-A03: Decouple enterprise module from candidate_generation

**Depends on**: HARDEN-A02

**Problem**: `candidate_generation.rs` imports `crate::enterprise::restock_gap` and `crate::enterprise::opportunity_signal` directly. The enterprise module's internal API becomes a coupling point.

**Deliverables**:
1. Define an `EnterpriseSignal` struct (or use existing types) representing the outputs of enterprise analysis
2. `generate_enterprise_candidates()` (from A02) accepts pre-computed enterprise signals as input data (`Vec<EnterpriseSignal>`) rather than calling enterprise functions directly
3. The orchestrator in `generate_candidates()` calls enterprise functions and passes results to the sub-generator

**Files**: `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-ai/src/enterprise.rs`

**Verification**: All candidate generation tests pass. Golden e2e hashes identical.

---

### HARDEN-A04: Document system execution ordering contract

**Problem**: `SystemManifest::canonical()` and `SystemId::ALL` define the tick execution order (Needs → Production → Trade → Combat → Perception → Politics), but the rationale for this specific ordering is undocumented. The ordering is load-bearing — changing it could produce different emergent behavior.

**Deliverables**:
1. Add doc comments to `SystemId::ALL` explaining the ordering rationale:
   - Needs first: deprivation wounds must be assessed before production/trade decisions
   - Production before Trade: new goods must exist before they can be traded
   - Trade before Combat: economic actions resolve before violence
   - Combat before Perception: combat outcomes are visible in the same tick
   - Perception before Politics: agents perceive before social systems run
2. Add a compile-time assertion (const fn or static assert) that `SystemId::ALL` length matches the number of enum variants, preventing silent omission when new systems are added
3. Add a doc comment to `SystemManifest::canonical()` stating it is the authoritative tick order

**Files**: `crates/worldwake-sim/src/system_manifest.rs`

**Verification**: `cargo test --workspace` passes. `cargo clippy --workspace` clean. No behavioral change.

---

## Category B: Robustness & Invariants

### HARDEN-B01: Named recipe registry with string-keyed lookup

**Problem**: `RecipeRegistry` only supports positional `RecipeId(n)` lookup. Tests and setup code use fragile `RecipeId(0)`, `RecipeId(1)` references that break if registration order changes.

**Deliverables**:
1. Add `recipe_by_name(&self, name: &str) -> Option<(RecipeId, &RecipeDefinition)>` method to `RecipeRegistry`
2. Add a `by_name: BTreeMap<String, RecipeId>` secondary index (built during `register()`)
3. `register()` returns an error (or panics, matching existing style) if a duplicate name is registered
4. Add tests for name-based lookup, duplicate rejection, and empty registry

**Files**: `crates/worldwake-sim/src/recipe_registry.rs`

**Verification**: All existing recipe registry tests pass. New tests for name-based lookup pass.

---

### HARDEN-B02: Action handler registry completeness check

**Problem**: `ActionHandlerRegistry` and `ActionDefRegistry` are populated independently. If a new `ActionDef` is registered but its `handler: ActionHandlerId` points to an unregistered handler, the mismatch is only discovered at runtime when the action fires.

**Deliverables**:
1. Add `verify_completeness(defs: &ActionDefRegistry, handlers: &ActionHandlerRegistry) -> Result<(), Vec<ActionDefId>>` function that checks every `ActionDef`'s `handler` field points to a valid handler
2. Call this verification in `SimulationState` initialization or provide it as a standalone validation function
3. Add tests: all-valid case, missing-handler case

**Files**: `crates/worldwake-sim/src/action_handler_registry.rs`

**Verification**: New tests pass. Existing tests unaffected.

---

### HARDEN-B03: Prototype world entity accessor API

**Problem**: The golden e2e test (line 47) duplicates the private `prototype_entity()` function from `topology.rs` because there's no public way to get named entity IDs for the prototype world. Other test files that use `build_prototype_world()` face the same issue.

**Deliverables**:
1. Add a `PrototypeWorldEntities` struct with named fields for each prototype place:
   ```rust
   pub struct PrototypeWorldEntities {
       pub village_square: EntityId,
       pub orchard_farm: EntityId,
       pub general_store: EntityId,
       pub common_house: EntityId,
       pub rulers_hall: EntityId,
       pub guard_post: EntityId,
       pub public_latrine: EntityId,
       pub north_crossroads: EntityId,
       pub forest_path: EntityId,
       pub bandit_camp: EntityId,
       pub south_gate: EntityId,
       pub east_field_trail: EntityId,
   }
   ```
2. Add `pub fn prototype_world_entities() -> PrototypeWorldEntities` that returns the entity IDs matching `build_prototype_world()`
3. Make `prototype_entity()` public (or use it internally in the new function)
4. Update golden e2e to use `prototype_world_entities()` instead of its local `prototype_entity()` + manual constants

**Files**: `crates/worldwake-core/src/topology.rs`, `crates/worldwake-ai/tests/golden_e2e.rs`

**Verification**: Golden e2e passes with identical hashes. No behavioral change.

---

### HARDEN-B04: Observation snapshot relevance filtering

**Benefits from**: HARDEN-A01 (GoalSemantics trait could declare relevant observation dimensions)

**Problem**: `observation_snapshot_changed()` in `agent_tick.rs` (lines 475–481) compares the full commodity signature across ALL commodity kinds. This means an agent pursuing a Sleep goal will replan when an unrelated commodity changes in their inventory (e.g., gaining a coin from trade). The replanning is wasted work and can cause goal thrashing.

**Deliverables**:
1. Filter `commodity_signature` comparison to only commodities relevant to the current goal/plan
2. If HARDEN-A01 is implemented, the `GoalSemantics` trait could declare which observation dimensions are relevant; otherwise, use a simpler per-`GoalKind` relevance function
3. Preserve the existing "always dirty if no plan" behavior — filtering only applies when a plan is active
4. Add a unit test showing that commodity changes irrelevant to the current goal do NOT trigger replanning

**Files**: `crates/worldwake-ai/src/agent_tick.rs`, `crates/worldwake-ai/src/decision_runtime.rs`

**Verification**: Golden e2e passes (may produce different tick counts if agents replan less, but final state should be equivalent — verify conservation and death/loot outcomes). If hashes change, document the new expected hashes.

---

## Category C: Performance & Efficiency

### HARDEN-C01: Replace Vec-sort frontier with BinaryHeap in search

**Problem**: `pop_next_node()` (lines 145–153 of `search.rs`) sorts the entire `Vec<SearchNode>` on every pop — O(n log n) per expansion. With larger frontier sizes this becomes a bottleneck.

**Deliverables**:
1. Replace `Vec<SearchNode>` frontier with `BinaryHeap<Reverse<SearchNode>>` (or equivalent)
2. Implement `Ord` for `SearchNode` matching the existing `compare_search_nodes` logic (total_estimated_ticks, then steps.len(), then steps lexicographic)
3. Remove `pop_next_node()` function — use `frontier.pop()` directly
4. Verify determinism: the `Ord` implementation must produce the same node selection order as the current sort

**Files**: `crates/worldwake-ai/src/search.rs`

**Verification**: All search tests pass with identical results. Golden e2e hashes identical (determinism preserved).

---

### HARDEN-C02: Cache OmniscientBeliefView per agent tick

**Problem**: `process_agent()` in `agent_tick.rs` creates up to 9 separate `OmniscientBeliefView::new()` calls during a single agent's tick processing (lines 138, 160, 173, 185, 244, 252, 298, 321, 400). While `OmniscientBeliefView::new()` is cheap (just a reference), the pattern obscures the data flow — it's unclear which view reflects which world state.

**Deliverables**:
1. Create ONE `OmniscientBeliefView` at the top of `process_agent()`
2. Refresh the view (create a new one) only after world mutations (e.g., after `persist_blocked_memory` calls)
3. Document with comments which sections use which view generation

**Primary benefit**: Clarity and correctness (view reflects known state), not raw performance.

**Files**: `crates/worldwake-ai/src/agent_tick.rs`

**Verification**: All agent_tick tests pass. Golden e2e hashes identical.

---

### HARDEN-C03: Reduce PlanningState clone overhead (OPTIONAL)

**Problem**: `PlanningState` is cloned on every search node expansion. The override maps (`BTreeMap`) inside are cloned deeply. At larger beam widths this could become expensive.

**Deliverables**:
1. Investigate `im::OrdMap` or cow-style sharing for override maps in `PlanningState`
2. Benchmark clone cost at beam_width=8 vs beam_width=32
3. If improvement is measurable (>20% clone time reduction), implement; otherwise document findings and close

**Files**: `crates/worldwake-ai/src/planning_state.rs`

**Verification**: All search and planning tests pass. Golden e2e hashes identical.

**Priority**: Low. Only matters at larger beam widths not currently used.

---

## Category D: Test Hardening

### HARDEN-D01: Multi-recipe golden e2e scenario

**Benefits from**: HARDEN-B01 (named recipe IDs)

**Problem**: The golden e2e only tests a single recipe (Harvest Apples via orchard). Multi-recipe interactions (e.g., Harvest Grain → Bake Bread, Chop Wood) are untested at the integration level.

**Deliverables**:
1. Add a new scenario to `golden_e2e.rs` with:
   - Multiple recipes registered: Harvest Apples (orchard), Harvest Grain (field), Bake Bread (mill, requires grain)
   - An agent with `KnownRecipes` including all three
   - Workstations placed at appropriate locations
   - Verify the agent chains harvest → craft when direct food is unavailable
2. Assert conservation invariants throughout
3. Assert deterministic replay produces identical hashes

**Files**: `crates/worldwake-ai/tests/golden_e2e.rs`

**Verification**: New scenario passes. Existing scenarios unaffected.

---

### HARDEN-D02: Budget exhaustion and beam pruning tests

**Problem**: The search module has tests for `max_plan_depth` and `max_node_expansions` exhaustion, but does not test `beam_width` pruning behavior or the interaction between budget parameters.

**Deliverables**:
1. Test `beam_width=1` forces greedy search (only best successor survives)
2. Test `beam_width` pruning discards lower-priority successors
3. Test interaction: small `max_node_expansions` with large `beam_width` still terminates
4. Test `max_plan_depth=0` returns `None` immediately (edge case)

**Files**: `crates/worldwake-ai/src/search.rs` (test module)

**Verification**: New tests pass. Existing tests unaffected.

---

### HARDEN-D03: Strengthen weak assertions in golden e2e

**Problem**: Two assertions in the golden e2e are observational rather than required:
1. Line 696: Blocked intent check is wrapped in `if saw_blocker { eprintln!(...) }` — observational only
2. Line 1023: Loot assertion is `if !b_looted { eprintln!("Note: ...non-fatal") }` — observational only

These weaken the test's value as a regression gate.

**Deliverables**:
1. Convert the loot assertion (line 1023) to a proper `assert!(b_looted, ...)` — if the AI architecture is working correctly, looting should happen deterministically within 100 ticks
2. For the blocked intent check (line 696): if blocked intents are not reliably generated (planner may skip rather than fail), keep it observational but add a doc comment explaining WHY it's observational
3. If converting to hard assertions causes test failures, investigate and fix the underlying AI behavior rather than weakening the assertion

**This ticket should be implemented LAST** to avoid churn from other tickets changing behavior.

**Files**: `crates/worldwake-ai/tests/golden_e2e.rs`

**Verification**: Golden e2e passes with all assertions as hard asserts (where converted).

---

## Implementation Order (Waves)

```
Wave 1 (parallel, no deps, low risk):
  HARDEN-A04  — doc comments + static assert (system_manifest.rs)
  HARDEN-B02  — completeness check (action_handler_registry.rs)
  HARDEN-B03  — prototype entity accessors (topology.rs)
  HARDEN-D02  — budget/beam tests (search.rs tests)
  HARDEN-C01  — BinaryHeap frontier (search.rs)

Wave 2 (parallel, medium risk):
  HARDEN-A02  — modularize candidate generation
  HARDEN-B01  — named recipe registry
  HARDEN-C02  — cache belief view per tick

Wave 3 (has deps on Wave 2):
  HARDEN-A01  — GoalSemantics trait (independent but same area as A02)
  HARDEN-A03  — decouple enterprise (needs A02)
  HARDEN-D01  — multi-recipe e2e (benefits from B01)

Wave 4 (behavioral changes, do last):
  HARDEN-B04  — observation filtering (benefits from A01)
  HARDEN-D03  — strengthen assertions (must be last)

Wave 5 (optional):
  HARDEN-C03  — PlanningState clone optimization
```

## Dependency Graph

```
A04 ──── independent
B02 ──── independent
B03 ──── independent
D02 ──── independent
C01 ──── independent
A02 ──── independent
B01 ──── independent
C02 ──── independent
A01 ──── independent (same area as A02, no hard dep)
A03 ──→ A02 (hard dependency)
D01 ──→ B01 (soft — benefits from named recipes)
B04 ──→ A01 (soft — benefits from GoalSemantics trait)
D03 ──→ all other tickets (must be last)
C03 ──→ optional, no deps
```

## Verification Criteria (All Tickets)

For every ticket in this spec:

1. `cargo test --workspace` passes
2. `cargo clippy --workspace` produces no new warnings
3. Golden e2e produces **identical state hashes** unless the ticket explicitly documents a hash change (only HARDEN-B04 may change hashes)
4. No new external dependencies added (except optionally `im` for C03)
5. No public API breakage unless explicitly called for in the ticket

## Critical Files Summary

| File | Tickets |
|------|---------|
| `crates/worldwake-ai/src/search.rs` | A01, C01, D02 |
| `crates/worldwake-ai/src/candidate_generation.rs` | A02, A03 |
| `crates/worldwake-ai/src/agent_tick.rs` | B04, C02 |
| `crates/worldwake-ai/src/enterprise.rs` | A03 |
| `crates/worldwake-ai/src/goal_semantics.rs` (new) | A01 |
| `crates/worldwake-ai/tests/golden_e2e.rs` | B03, D01, D02, D03 |
| `crates/worldwake-sim/src/recipe_registry.rs` | B01 |
| `crates/worldwake-sim/src/action_handler_registry.rs` | B02 |
| `crates/worldwake-core/src/topology.rs` | B03 |
| `crates/worldwake-sim/src/system_manifest.rs` | A04 |
| `crates/worldwake-ai/src/planning_state.rs` | C03 |
| `crates/worldwake-ai/src/decision_runtime.rs` | B04 |
