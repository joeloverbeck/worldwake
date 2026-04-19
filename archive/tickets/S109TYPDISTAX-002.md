# S109TYPDISTAX-002: Core discrepancy types and memory components

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `Discrepancy` enum, `BeliefClaimKey`, `DiscrepancyMemory`, `RepairMemory`, `LearnedOpportunityMemory` components registered in the ECS
**Deps**: None (independent of T001; both land before T003)

## Problem

S109 introduces a typed discrepancy taxonomy (`Discrepancy` enum) and three new per-agent memory components (`DiscrepancyMemory` for epistemic failures, `RepairMemory` for successful alternates, `LearnedOpportunityMemory` for opportunities observed en route). These land as additive types with no semantic effect on existing code — emission, reader migration, and variant removal are subsequent tickets. This ticket also lands `BeliefClaimKey`, a new key type referenced by `DiscrepancyClearing::BeliefUpdate` in this spec and by future specs S114 and S115.

## Assumption Reassessment (2026-04-19)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `Discrepancy`, `DiscrepancyMemory`, `DiscrepancyEntry`, `DiscrepancyClearing`, `RepairMemory`, `LearnedOpportunityMemory`, and `BeliefClaimKey` do not exist in the codebase (verified by `grep -rn "Discrepancy\|BeliefClaimKey\|RepairMemory\|LearnedOpportunityMemory" crates/`). `OpportunityKey` is the existing key type at `crates/worldwake-core/src/goal.rs:161` and is reused by `LearnedOpportunityMemory`. `BlockerKey` is the existing key type at `crates/worldwake-core/src/blocker_memory.rs:11` (renamed by T001) and is reused by `DiscrepancyMemory`. `EntityBeliefAspect` exists and is re-exported through `crates/worldwake-core/src/belief.rs:5`; it is the aspect field of the new `BeliefClaimKey`.
2. S109 spec (`specs/S109-typed-discrepancy-taxonomy.md` D1, D2, D4, D9) defines the required shapes. `Discrepancy` has 9 variants (`BeliefStale`, `BeliefContradicted`, `ImproperPlanningState`, `MissingObservation`, `NoLegalBinding`, `NoWillingCounterparty`, `RouteUnknown`, `SearchBudgetExhausted`, `PartialExecutionDrift`). `DiscrepancyClearing` has 4 variants (`TtlExpiry`, `ReobservationOf { target }`, `BeliefUpdate { claim_key }`, `WorldStructureChange`). All types derive `Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize`; memories are `Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize` (non-Copy because they own `BTreeMap`).
3. Shared abstraction boundary: the `with_component_schema_entries!` macro expansion sites at `crates/worldwake-core/src/world.rs`, `delta.rs`, `component_tables.rs`. Per `tickets/README.md` check #13, each new component_schema block requires bare type imports at each expansion site. The 3 new memories (DiscrepancyMemory, RepairMemory, LearnedOpportunityMemory) follow the existing `BlockerMemory` registration pattern verbatim (minus the now-landed `BlockerMemory` block).
13. No adjacent contradictions. `BeliefClaimKey` is defined as `(subject: EntityId, aspect: EntityBeliefAspect)`; this composite is what observation-pipeline updates to `AgentBeliefStore` already carry conceptually (the claim's subject and aspect are visible at every record site in `belief.rs`), so introducing the key type does not widen or narrow the existing claim-identification semantics.
15. Memory capacity: S109 introduces a `MemoryCapacityProfile` with `memory_capacity: u32 = 32` (default, profile-driven). `RepairMemory` and `LearnedOpportunityMemory` eviction follows oldest-`observed_tick` first. This ticket introduces the profile struct with `Default` impl but does NOT wire capacity enforcement into any hot path yet (enforcement lands with the emission ticket T004). Unit tests 5 and 6 from the spec's Validation section exercise the eviction logic directly on the memory structs. The T002 entry shapes for `RepairMemory` / `LearnedOpportunityMemory` do not yet carry expiry metadata, so their `expire(current_tick)` methods land as API-preserving no-ops in this ticket rather than inventing retention state that the spec has not defined.

## Architecture Check

1. Landing the types and component registrations as additive infrastructure (independent of T001's rename) lets the workspace keep building throughout the S109 rollout. Subsequent tickets (T003 belief-view accessors, T004 emission migration, T005 trace replacement) consume these types without re-introducing them. This respects the "workspace builds after every ticket" invariant.
2. No backwards-compatibility aliasing. The new types are defined once; no type aliases, no deprecated shims. `DiscrepancyClearing` is intentionally a simpler shape than `BlockerClearingCondition` because epistemic discrepancies do not compare concrete quantity baselines — they clear on reobservation, belief update, or time. FND-28 compliant.

## Verification Layers

1. Typed-value bincode roundtrip → focused unit test on each new type (`Discrepancy`, `DiscrepancyEntry`, `DiscrepancyClearing`, `BeliefClaimKey`, `RepairEntry`, `OpportunityEntry`). Proof: `serde_roundtrip` style assertions inside each module's `#[cfg(test)]`.
2. Component registration correctness → focused test per memory (`entities_with_discrepancy_memory` returns empty on fresh world, `insert_component_discrepancy_memory` on non-agent returns error, etc.) following the pattern at `blocker_memory.rs:<cfg(test)>` post-rename.
3. Memory behavior → unit tests 1 (record + expire), 2 (ReobservationOf clears), 3 (BeliefUpdate clears), 5 (RepairMemory overwrite), 6 (LearnedOpportunityMemory eviction) from S109 Validation section land in the new modules' `#[cfg(test)]` blocks.
6. Single-layer ticket: no runtime emission, no cross-crate contract; the compiler and per-type unit tests are the proof surface.

## What to Change

### 1. Define `Discrepancy` enum and `BeliefClaimKey`

Create `crates/worldwake-core/src/discrepancy.rs` with the `Discrepancy` enum per spec D1 (9 variants; derives `Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize`). Add a doc comment per variant matching S109 D1's narrative.

Create `crates/worldwake-core/src/belief_claim_key.rs` with:

```rust
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BeliefClaimKey {
    pub subject: EntityId,
    pub aspect: EntityBeliefAspect,
}
```

Both types get `Component` impl only if they are ECS components (they are not — they are value types used inside memories and event payloads). Add bincode roundtrip tests in each file's `#[cfg(test)]` block.

### 2. Define `DiscrepancyMemory` component

Add to `crates/worldwake-core/src/discrepancy.rs` (or a sibling `discrepancy_memory.rs` — choose by file-size balance; spec permits either):

```rust
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscrepancyMemory {
    pub entries: BTreeMap<BlockerKey, DiscrepancyEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscrepancyEntry {
    pub blocker_key: BlockerKey,
    pub discrepancy: Discrepancy,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
    pub clearing_condition: DiscrepancyClearing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DiscrepancyClearing {
    TtlExpiry,
    ReobservationOf { target: EntityId },
    BeliefUpdate { claim_key: BeliefClaimKey },
    WorldStructureChange,
}
```

Add `impl Component for DiscrepancyMemory {}`. Add `record(entry)`, `expire(current_tick)`, `is_suppressed(&BlockerKey, current_tick)`, `clear_for(&BlockerKey)`, and `clear_by_condition(predicate)` methods — same surface shape as `BlockerMemory`'s `record`/`expire`/`is_blocked`/`clear_for`/`sweep_cleared` pair.

### 3. Define `RepairMemory` and `LearnedOpportunityMemory`

Add (co-located with `DiscrepancyMemory` or in a new `repair_memory.rs` / `learned_opportunity_memory.rs`):

```rust
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct RepairKey {
    pub goal_key: GoalKey,
    pub alternate_target: EntityId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepairEntry {
    pub repair_key: RepairKey,
    pub observed_tick: Tick,
    pub success_count: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepairMemory {
    pub repairs: BTreeMap<RepairKey, RepairEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OpportunityEntry {
    pub opportunity: OpportunityKey,
    pub observed_tick: Tick,
    pub observed_at: EntityId,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct LearnedOpportunityMemory {
    pub opportunities: BTreeMap<OpportunityKey, OpportunityEntry>,
}
```

Add `impl Component` for both memories. Each memory gets `record(entry)`, `expire(current_tick)`, and an eviction method driven by `MemoryCapacityProfile::memory_capacity` that removes oldest-`observed_tick` entries when the cap is exceeded.

### 4. Define `MemoryCapacityProfile`

Create `crates/worldwake-core/src/memory_capacity_profile.rs`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemoryCapacityProfile {
    #[serde(default = "default_memory_capacity")]
    pub memory_capacity: u32,
}

impl Default for MemoryCapacityProfile {
    fn default() -> Self { Self { memory_capacity: 32 } }
}

impl Component for MemoryCapacityProfile {}

const fn default_memory_capacity() -> u32 { 32 }
```

### 5. Register all 4 new components via `with_component_schema_entries!`

In `crates/worldwake-core/src/component_schema.rs`, add four new entry blocks following the existing `BlockerMemory` block (post-T001) pattern. Names per component:

- `discrepancy_memories`, `DiscrepancyMemory`, … `"DiscrepancyMemory"`, `|kind| kind == EntityKind::Agent`, `txn_simple_set`.
- `repair_memories`, `RepairMemory`, …
- `learned_opportunity_memories`, `LearnedOpportunityMemory`, …
- `memory_capacity_profiles`, `MemoryCapacityProfile`, …

Each block produces the full accessor suite (`insert_component_*`, `get_component_*`, `entities_with_*`, `query_*`, `count_with_*`, `set_component_*`, `clear_component_*`).

### 6. Update macro expansion sites

Per `tickets/README.md` check #13, update imports and `ComponentKind` arms:

- `crates/worldwake-core/src/world.rs` — add `DiscrepancyMemory, RepairMemory, LearnedOpportunityMemory, MemoryCapacityProfile` to imports.
- `crates/worldwake-core/src/delta.rs` — add the 4 types to imports and any `ComponentKind::*`/`ComponentValue::*` arms generated by the macro.
- `crates/worldwake-core/src/component_tables.rs` — same.
- `crates/worldwake-core/src/lib.rs` — `pub mod discrepancy; pub mod belief_claim_key; pub mod memory_capacity_profile;` plus module declarations for repair/learned-opportunity if split into separate files. Re-export `Discrepancy`, `DiscrepancyMemory`, `DiscrepancyEntry`, `DiscrepancyClearing`, `BeliefClaimKey`, `RepairMemory`, `RepairKey`, `RepairEntry`, `LearnedOpportunityMemory`, `OpportunityEntry`, `MemoryCapacityProfile`.

### 7. Unit tests (Validation section items 1–6)

Inside each new module's `#[cfg(test)]` block:

- `DiscrepancyMemory::record + expire` prunes expired entries (spec test 1).
- `DiscrepancyClearing::ReobservationOf { target }` — test the clearing-condition match helper (spec test 2; the perception-side wiring lands with T004).
- `DiscrepancyClearing::BeliefUpdate { claim_key }` — test the clearing-condition match helper (spec test 3).
- `RepairMemory` overwrite on fresher entry (spec test 5).
- `LearnedOpportunityMemory` evicts oldest on `memory_capacity` exceeded (spec test 6).
- `BlockerMemory` semantics preserved — covered by T001's existing tests; no new assertions here (spec test 4 is satisfied by T001).

Additionally: bincode roundtrip tests per type (derive_value_bounds-style assertions following the `blocker_memory.rs` pattern).

## Files to Touch

- `crates/worldwake-core/src/discrepancy.rs` (new)
- `crates/worldwake-core/src/belief_claim_key.rs` (new)
- `crates/worldwake-core/src/repair_memory.rs` (new)
- `crates/worldwake-core/src/learned_opportunity_memory.rs` (new)
- `crates/worldwake-core/src/memory_capacity_profile.rs` (new)
- `crates/worldwake-core/src/component_schema.rs` (modify — add 4 macro blocks)
- `crates/worldwake-core/src/world.rs` (modify — imports)
- `crates/worldwake-core/src/delta.rs` (modify — imports + macro-generated enum arms)
- `crates/worldwake-core/src/component_tables.rs` (modify — imports + macro-generated enum arms)
- `crates/worldwake-core/src/lib.rs` (modify — module declarations + re-exports)

## Out of Scope

- No belief-view accessors (T003).
- No `CognitiveProfile` TTL fields or `discrepancy_ttl` function (T003).
- No emission-site rewrites; no call-site migration of `BlockingFact::Unknown`/`AssumptionFailed` (T004).
- No `DiscrepancyTrace` or `UnknownBlockerTrace` replacement (T005).
- No removal of `BlockingFact::Unknown`/`AssumptionFailed` variants (T006).
- No scenario RON changes.
- No `SAVE_FORMAT_VERSION` bump (T006).
- No wiring of `MemoryCapacityProfile` into `create_agent` — the profile is defined and registered; runtime seeding matches the existing `BlockerMemory` lazy-init pattern (verified: `create_agent` at `world.rs:164` does not seed `BlockerMemory` either).

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core discrepancy` — new module's unit tests pass.
2. `cargo test -p worldwake-core repair_memory learned_opportunity_memory belief_claim_key memory_capacity_profile` — per-module unit tests pass.
3. `cargo test -p worldwake-core component_schema` (or whichever existing schema-coverage tests run) — new components are registered and accessors are callable.
4. Existing suite: `cargo test --workspace`.

### Invariants

1. `cargo build --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
2. Every new component is registered on `EntityKind::Agent` and reachable via `get_component_*` / `insert_component_*` accessors from other crates.
3. Determinism invariants hold: all new memories use `BTreeMap`, not `HashMap`; no floats introduced; no wall-clock reads.
4. Bincode roundtrip preserves equality for every new serialized type (tested per type).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/discrepancy.rs` `#[cfg(test)]` — bounds, default-empty, record+expire, clearing-condition match helpers. Spec Validation items 1–3.
2. `crates/worldwake-core/src/belief_claim_key.rs` `#[cfg(test)]` — bounds + bincode roundtrip.
3. `crates/worldwake-core/src/repair_memory.rs` `#[cfg(test)]` — overwrite on fresher entry, eviction by `memory_capacity`. Spec Validation item 5.
4. `crates/worldwake-core/src/learned_opportunity_memory.rs` `#[cfg(test)]` — record, expire, eviction by `memory_capacity`. Spec Validation item 6.
5. `crates/worldwake-core/src/memory_capacity_profile.rs` `#[cfg(test)]` — bounds + default.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

Completed on 2026-04-19.

- Added the new S109 core types in `worldwake-core`: `Discrepancy`, `BeliefClaimKey`, `DiscrepancyMemory`, `RepairMemory`, `LearnedOpportunityMemory`, and `MemoryCapacityProfile`, with focused unit coverage for roundtrips and the additive memory behaviors this ticket owns.
- Registered `DiscrepancyMemory`, `RepairMemory`, `LearnedOpportunityMemory`, and `MemoryCapacityProfile` as agent components via `with_component_schema_entries!`, then updated the core macro expansion/import sites and `delta.rs` sample coverage so the generated accessor surface compiles cleanly across the workspace.
- Added deterministic test fixtures in `test_utils.rs` for the new memory/profile components so future shared-surface tests can construct the new component values without re-implementing boilerplate.

## Deviations

- Reassessment found one specification gap inside the ticket itself: `RepairMemory` and `LearnedOpportunityMemory` were required to expose `expire(current_tick)` even though their T002 entry shapes do not yet carry expiry metadata. This ticket now documents and implements those `expire` methods as API-preserving no-ops rather than inventing unsupported retention state.
- The acceptance-criteria command `cargo test -p worldwake-core repair_memory learned_opportunity_memory belief_claim_key memory_capacity_profile` is not a valid single Cargo filter invocation. Verification was run as separate focused `cargo test -p worldwake-core <module>` commands for each new module.

## Verification Result

- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-core discrepancy`
- Passed `cargo test -p worldwake-core repair_memory`
- Passed `cargo test -p worldwake-core learned_opportunity_memory`
- Passed `cargo test -p worldwake-core belief_claim_key`
- Passed `cargo test -p worldwake-core memory_capacity_profile`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
