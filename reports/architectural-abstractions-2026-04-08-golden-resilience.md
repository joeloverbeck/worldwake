# Architectural Abstraction Recovery: golden-resilience

**Date**: 2026-04-08
**Input**: crates/worldwake-ai/tests/golden_resilience.rs
**Source modules analyzed**: 199
**Crates touched**: worldwake-core (67), worldwake-sim (48), worldwake-systems (36), worldwake-ai (48)
**Prior reports consulted**: reports/architectural-abstractions-2026-04-07-golden-soak.md, reports/missing-abstractions-2026-04-07-golden-soak.md

## Executive Summary

No new cross-subsystem fractures were identified from the golden_resilience test suite. The two tests (T31 stress disruptions, T32 replay consistency) exercise the same codebase as golden_soak but with two unique emphases: arbitrary mid-simulation state mutations and save/load serialization roundtrips. The prior golden-soak report's two fractures (RuntimeBeliefView overloaded abstraction, PlanningSnapshot projection drift) remain the only validated cross-subsystem findings. This test's primary architectural value is as a **stress-proof of existing design** — it validates that the disruption injection path, dead agent handling, serialization boundary, and conservation audit all hold under adversarial conditions. One single-signal observation (conservation audit-only enforcement) warrants further investigation but does not meet the two-signal threshold for a fracture finding.

## Scenario Families

| Family | Tests | Domain Concepts | Key Assertions |
|--------|-------|----------------|----------------|
| Disruption resilience | T31 (1 test, 6 per-tick invariants) | death, archival, workstation removal, teleportation, conservation, needs, causality | All 6 invariants hold across 2880 ticks with 28 disruptions; save/load hash match at end |
| Serialization determinism | T32 (1 test, 14 checkpoints) | save/load, state hash, event log hash, registry rebuild, driver serialization | Continuous vs split-run checkpoint hashes match exactly at every 100-tick boundary |

### Scenario Family Detail

**Disruption resilience (T31)**: Reuses T30's 10-place topology and 20-agent population. Every 100 ticks, one deterministically-selected disruption is injected via WorldTxn: kill a random living agent (`set_component_dead_at`), destroy a random ItemLot (`archive_entity`), remove a workstation marker (`clear_component_workstation_marker`), or teleport an agent (`set_ground_location`). Six invariants are checked every tick: commodity conservation, needs bounds (≤1000 Permille), dead agent inactivity, unique placement at valid places, tick monotonicity, and causal link integrity. After 2880 ticks, a save/load roundtrip hash is verified.

**Serialization determinism (T32)**: Same T30 world, no disruptions. A continuous 1440-tick run records (world_hash, log_hash) at 14 checkpoints (every 100 ticks). A split run saves at tick 720, loads the snapshot, and continues for 720 more ticks. The test asserts that checkpoint hashes match exactly — proving that save/load does not alter world meaning (Principle 12).

## Traceability Summary

| Module | Scenario Families | Confidence | Strategy |
|--------|------------------|------------|----------|
| worldwake-core/src/conservation.rs | Disruption resilience (inv. 1) | High | use/import (called every tick in T31) |
| worldwake-core/src/world_txn.rs | Disruption resilience (disruption injection) | High | use/import (WorldTxn creates disruptions) |
| worldwake-core/src/canonical.rs | Both (hash_world, hash_event_log) | High | use/import (checkpoint and roundtrip verification) |
| worldwake-core/src/cause.rs | Disruption resilience (inv. 6) | High | use/import (CauseRef validated per event) |
| worldwake-core/src/numerics.rs | Disruption resilience (inv. 2) | High | structural (Permille type enforces needs bounds) |
| worldwake-sim/src/tick_step.rs | Both (step_once execution) | High | use/import + temporal coupling (160 AI↔SIM co-commits) |
| worldwake-sim/src/save_load.rs | Serialization determinism | High | use/import (save_to_bytes, load_from_bytes) |
| worldwake-sim/src/saveable_runtime.rs | Serialization determinism | High | use/import (SimulationState snapshot) |
| worldwake-ai/src/agent_tick/mod.rs | Both (AI decision + dead agent skip + driver serialization) | High | use/import + temporal coupling (47 co-changes with per_agent_belief_view.rs) |
| worldwake-systems/src/action_registry.rs | Serialization determinism (registry rebuild) | High | use/import (build_full_registries on load) |
| worldwake-ai/tests/golden_harness/mod.rs | Both (step_once, save_load_roundtrip, harness infrastructure) | High | use/import (test infrastructure) |
| worldwake-ai/tests/golden_harness/soak_world.rs | Both (build_t30_world) | High | use/import (T30 world setup) |

## Fracture Summary

| # | Fracture Type | Location | Evidence Sources | Severity |
|---|--------------|----------|-----------------|----------|

No new fractures identified. The prior golden-soak report's two fractures apply to code exercised by this test but are not re-reported:

- **Prior Fracture #1** (Overloaded abstraction, MEDIUM): RuntimeBeliefView ~100+ methods spanning 8+ domains in `worldwake-sim/belief_view.rs`. Temporal coupling: 46-47 co-changes with AI planning files. Still valid.
- **Prior Fracture #2** (Projection drift, MEDIUM): PlanningSnapshot ~40 fields mirroring RuntimeBeliefView in `worldwake-ai/planning_snapshot.rs`. Temporal coupling: 38-39 co-changes. Still valid.
- **Prior candidate**: Belief View Domain Decomposition (Medium confidence, significant counter-evidence around Rust trait object constraints). Status unchanged.
- **Prior incomplete abstraction** (missing-abstractions report): GoalDispatchDeclaration not carrying all per-goal-kind static metadata. Status unchanged.

## Candidate Abstractions

None new. The prior report's Belief View Domain Decomposition candidate remains the only cross-subsystem candidate. This test does not add evidence for or against it.

## Acceptable Architecture

### Dead Agent Inactivity: Defense-in-Depth Across AI/SIM Boundary

The "dead agents don't act" invariant is enforced at two layers with complementary responsibilities:

- **AI layer** (`worldwake-ai/src/agent_tick/mod.rs:285-290`): When death is first detected, clears all frames, plans, and steps, sets `dead_cleanup_done = true`. On subsequent ticks, returns immediately if `dead_cleanup_done` is set — preventing any new goal computation or plan generation.
- **Sim layer** (`worldwake-sim/src/tick_step.rs:723`): `abort_actions_for_dead_actors()` runs each tick, scanning active actions and aborting any whose actor has `dead_at`. This handles the case where a system kills an agent mid-tick while an action is in-flight.

These are not redundant enforcement — they operate at different abstraction levels. The AI layer prevents new work from being scheduled. The sim layer cleans up work-in-progress. Neither can fully substitute for the other. Removing the AI layer would waste planning compute on dead agents before tick_step aborts the results. Removing the sim layer would leave a gap for mid-tick deaths.

T31 validates this holds under random agent kills (disruption type 0). The test checks `agent_has_active_action()` for every dead agent every tick (lines 208-215). No violations across 2880 ticks with up to 7 agent deaths.

**Counter-evidence**: If the dead-cleanup pattern required coordination beyond the `dead_at` component (e.g., if the AI and sim layers needed to agree on a protocol beyond "check dead_at"), this would be a fracture. But `dead_at` is the single shared truth — both layers independently read it and act accordingly. This is state-mediated interaction (Principle 26), not cross-system coupling.

### Serialization Boundary: Registry Rebuild Pattern

The save/load path separates serialized data from rebuilt indexes:

- **Serialized**: `SimulationState` containing World, EventLog, Scheduler, RecipeRegistry, ControllerState, DeterministicRng (via bincode). `AgentTickDriver` runtime state serialized separately.
- **Rebuilt on load**: `ActionDefRegistry` and `ActionHandlerRegistry` rebuilt from serialized `RecipeRegistry` via `build_full_registries()` (`worldwake-systems/src/action_registry.rs:64-72`). Post-load validation via `verify_completeness()`.
- **Validated on load**: `AgentTickDriver::from_saved_runtime()` calls `post_load_validate()` which prunes dead agents from runtime, clears stale caches, resets dirty masks.

This is a textbook "serialize data, rebuild indexes" pattern. Registries are deterministic functions of recipes. Recipes ARE serialized. The rebuild is validated. T32 proves exact hash equivalence across 14 checkpoints spanning the serialization boundary — continuous run and split run produce identical (world_hash, log_hash) pairs at every 100-tick checkpoint.

**Counter-evidence**: If registries carried state that could diverge from recipes (e.g., runtime-accumulated statistics or learned dispatch preferences), this pattern would be fragile. But registries are pure lookups populated from declarative recipe/action definitions. The determinism guarantee is structural.

### Causal Link Integrity: Dual-Layer Enforcement

Causal link integrity is the best-guarded invariant in the codebase:

- **Write-time enforcement** (`worldwake-core/src/event_log.rs:66-75`): When appending an event with `CauseRef::Event(cause_id)`, the EventLog asserts that `cause_id < event_id` (ordering) and that `cause_id` exists (referential integrity). These are hard panics — no event can be appended with a dangling causal reference.
- **Post-hoc verification** (`worldwake-core/src/verification.rs:43-91`): `verify_completeness()` provides comprehensive structural auditing including dangling reference detection, future cause detection, and root traceability (every event must trace back to Bootstrap, SystemTick, or ExternalInput).
- **Test-time incremental validation** (`golden_resilience.rs:238-257`): T31 checks every new event's causal reference every tick — incremental because the append-only log guarantees previously-checked events never change.

This dual-layer approach (write-time assert + test-time audit) provides both prevention and detection. The append-only EventLog is the single mutation gate — no subsystem can modify or delete events, only append.

**Counter-evidence**: If multiple subsystems could append to the EventLog independently (without going through `WorldTxn::commit()`), the write-time enforcement could be bypassed. But `EventLog::append()` is the single entry point, and it performs the causal integrity checks unconditionally.

## Needs Investigation

### Conservation Audit-Only Enforcement (Single signal: structural analysis)

`verify_authoritative_conservation()` and `total_authoritative_commodity_quantity()` in `worldwake-core/src/conservation.rs` (lines 11-48) are read-only audit functions. They compute commodity totals from live item lots and resource sources, but enforce no mutation-time constraints. Item lots can be created by `create_item_lot()`, destroyed by `archive_entity()`, split by `split_lot()`, and merged by `merge_lots()` — all accessible through any `WorldTxn` across multiple subsystems (production, trade, combat, needs, artifact lifecycle, etc.).

**Single signal**: Structural analysis shows no write-time conservation guard. The conservation functions are test-time validators only (called in golden_resilience.rs:176-187 and golden_soak.rs).

**Counter-evidence preventing escalation to fracture**:
1. `split_lot()` and `merge_lots()` are quantity-preserving by construction — split checks `amount < available`, merge sums quantities. These cover the vast majority of commodity movements.
2. `create_item_lot()` and `archive_entity()` are explicit creation/destruction operations. Conservation is a closed-system property that legitimately changes when commodities enter (harvest, regeneration) or exit (destruction, consumption). There is no "correct total" enforceable at write-time because the correct total depends on which operations are legitimate in context.
3. The `Permille` type provides numeric bounds on all rate parameters, preventing overflow-based violations.
4. T31 demonstrates that conservation holds under adversarial conditions — when the test deliberately destroys items (disruption type 1), it correctly adjusts the conservation baseline downward. The test at lines 129-130 shows: `*total = total.saturating_sub(qty)`.

**Second signal to look for**: Temporal coupling — does `conservation.rs` frequently co-change with files that call `create_item_lot()` or `archive_entity()`? If conservation bugs have historically required coordinated fixes across creation/destruction call sites, that would be the second signal. Also check git blame for any conservation-related bug fixes that required changes in multiple crates.

## Recommendations

- **Spec-worthy**: None from this analysis. The prior report's Belief View Domain Decomposition candidate remains the only cross-subsystem candidate requiring a feasibility study.
- **Acceptable**: Dead agent inactivity defense-in-depth, serialization boundary registry rebuild, causal link integrity dual-layer enforcement — all architecturally sound and proven under stress by this test.
- **Needs investigation**: Conservation audit-only enforcement — single-signal observation. Check temporal coupling for conservation.rs and verify no historical conservation bugs required multi-crate fixes.
- **Already identified (prior reports)**: RuntimeBeliefView overloaded abstraction (#7), PlanningSnapshot projection drift (#3), Belief View Domain Decomposition candidate, GoalDispatchDeclaration incomplete consolidation — all acknowledged, not re-reported.
