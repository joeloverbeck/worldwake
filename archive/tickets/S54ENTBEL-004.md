# S54ENTBEL-004: Split entity breadth memory from per-subject claim depth

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `PerceptionProfile`, claim retention, scenario profile plumbing
**Deps**: specs/S54-entity-belief-claims.md, S54ENTBEL-002

## Problem

`S54ENTBEL-002` made the passive perception / witnessed event / Tell lane claim-backed and restored full behavioral equivalence, but it also exposed that `PerceptionProfile.memory_capacity` is doing two different jobs:

- limiting how many entities an agent can retain in `known_entities`
- limiting how many `EntityBeliefClaim`s survive for one subject in `entity_claims`

Those are different substrates. Breadth across subjects is an agent-diversity trait; per-subject claim depth is storage policy for a specific belief representation. Reusing one number for both forced sparse snapshot emission just to avoid dropping canonical facts from a fresh observation. The current code is correct, but the contract is still overloaded and should be split explicitly.

## Assumption Reassessment (2026-04-05)

1. `AgentBeliefStore::enforce_capacity` and `AgentBeliefStore::enforce_entity_claim_capacity` in [belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs) both currently use `PerceptionProfile.memory_capacity`. The former applies it as a global `known_entities` cap; the latter applies it as a per-subject claim-vector cap. Confirmed in live code.
2. `S54ENTBEL-002` is now completed and verified. Its final implementation had to introduce sparse snapshot claim emission so a normal direct observation would not exceed the per-subject claim cap and lose canonical aspects such as `Location` or `Alive`. That bug was implementation-induced, not spec-intended.
3. The shared abstraction boundary under audit is `AgentBeliefStore { entity_claims, known_entities }` plus `PerceptionProfile` as the policy carrier that governs both retention behaviors. This is a mixed core/systems/CLI contract because profile values are scenario-definable and consumed during perception retention.
4. The same fact still has multiple lawful transport paths after `S54ENTBEL-002`: passive perception / witnessed events / Tell are canonical claim-backed paths, while investigation and other explicit refresh lanes may still write `known_entities` directly. This ticket does not change that information-path scope; it only separates retention policy for the already-claim-backed lane.
5. The intended invariant is: a fresh observed snapshot must never lose its canonical aspects because claim storage is more granular than summary storage. The current sparse-emission fix preserves that invariant, but only indirectly.
6. `PerceptionProfile` is scenario-definable through [scenario/types.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/scenario/types.rs) and applied in `spawn_agent()` in [scenario/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/scenario/mod.rs). Any new profile field here must remain scenario-definable per `AGENTS.md` and `docs/spec-drafting-rules.md`.
7. Golden coverage already proved the overloaded-capacity coupling is behaviorally sensitive. `golden_merchant_selling` changed branch selection when the per-subject claim cap interacted with the global entity cap; `golden_combat` changed corpse/death behavior when canonical claim clearing was mishandled. This follow-up should therefore keep both merchant and combat proof surfaces in scope.
8. `S54ENTBEL-003` is still pending and remains independent. Contradictory-claims golden coverage does not need this split to be valid, and this ticket must not silently absorb that proof obligation.
9. Reassessment exposed no requirement to change planner interfaces. The planner should continue reading derived `known_entities`; only the retention-policy substrate changes.
10. `PerceptionProfile` is serialized (`Serialize`, `Deserialize`) and used in world/scenario roundtrip fixtures. Splitting its fields is therefore a current-format persisted-shape change and must sweep world/profile roundtrips, scenario fixtures, and any save/load boundaries that serialize current-format world state.
11. The adjacent contradiction is architectural cleanup made visible by `S54ENTBEL-002`, not a separate bug and not a required consequence of `S54ENTBEL-003`.

## Architecture Check

1. Splitting breadth and depth is cleaner because it maps directly to the two real resources being modeled: how many subjects an agent can keep in working memory, and how many contradictory/provenance-rich claims the substrate keeps for one subject. One overloaded number obscures both.
2. The cleaner end-state is:
   - one profile field for cross-entity memory breadth
   - one profile field for per-subject entity-claim depth
   The claim-depth field may remain per-agent/profile-driven if we want agent diversity there, but it must no longer be an accidental alias of breadth capacity.
3. No backwards-compatibility aliasing or shim policy should be introduced. Rename/split the live profile contract directly and update all scenario/profile plumbing in one pass.

## Verification Layers

1. Per-subject claim retention no longer evicts canonical fresh-snapshot aspects under normal observation load -> focused `worldwake-core` unit tests on claim emission + capacity enforcement
2. Global entity-memory breadth remains unchanged and still evicts oldest subject memories at the agent level -> focused `worldwake-core` unit test on `enforce_capacity`
3. Scenario/profile plumbing exposes the new field(s) through `AgentDef` and `spawn_agent()` without breaking defaults -> focused `worldwake-cli` scenario tests
4. Current-format profile serialization and world roundtrips preserve the new split fields -> focused `worldwake-core` roundtrip/save tests
5. Merchant branch selection remains stable after the capacity split -> `golden_merchant_selling` focused golden
6. Death / corpse / direct-observation correction remains stable after the capacity split -> `golden_combat` focused golden
7. Full no-regression proof surface -> `cargo test --workspace` plus `cargo clippy --workspace --all-targets -- -D warnings`

## What to Change

### 1. Split the retention policy in `PerceptionProfile`

Introduce separate profile fields for:

- global entity working-memory breadth
- per-subject entity-claim depth

Update defaults, serde/bincode shape, and all helper constructors/tests that currently assume a single `memory_capacity` field covers both.
Because `PerceptionProfile` is persisted in the current save format, update the current-format serialization boundary accordingly and keep older save versions rejected unless another ticket explicitly owns compatibility work.

### 2. Rewire `AgentBeliefStore` retention to the split policy

In [belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs):

- keep the global `known_entities` cap tied to the breadth field
- use the new depth field for per-subject `entity_claims` truncation
- preserve current retention-window behavior and current summary re-derivation semantics
- keep sparse claim emission unless reassessment proves it is no longer needed; do not silently reintroduce dense claim emission without explicit evidence

### 3. Update scenario/profile plumbing

In the CLI scenario layer:

- extend `AgentDef` profile parsing/serialization to carry the new field(s)
- update `spawn_agent()` application logic
- update sample/default scenario tests so all `EntityKind::Agent` profile components remain scenario-definable

### 4. Add focused regression coverage for the split

Add tests that prove:

- a fresh direct observation with many populated aspects still retains all canonical summary fields when claim depth is low but sufficient by the new contract
- breadth eviction across many subjects still behaves independently from per-subject claim depth
- merchant/combat focused goldens remain unchanged under the split

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify, if exports move)
- `crates/worldwake-core/src/component_tables.rs` (modify, profile fixture updates)
- `crates/worldwake-core/src/world.rs` (modify, profile roundtrip/default tests)
- `crates/worldwake-cli/src/scenario/types.rs` (modify)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify)
- `crates/worldwake-sim/src/save.rs` (modify if current-format save version or save/load validation needs a factual update)
- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify only if focused retention assertions are added; do not rewrite scenario meaning)
- `crates/worldwake-ai/tests/golden_combat.rs` (modify only if focused retention assertions are added; do not rewrite scenario meaning)

## Out of Scope

- `S54ENTBEL-003` contradictory-claims golden scenario
- Migrating additional direct `known_entities` writers outside the passive/witnessed/Tell lane
- Planner API changes
- Institutional-belief capacity changes
- New contradiction lifecycle states such as disputed/retracted claims

## Acceptance Criteria

### Tests That Must Pass

1. A fresh observed snapshot no longer depends on sparse emission alone to preserve canonical aspects; the split capacity contract makes the retention boundary explicit.
2. Global entity-memory eviction still operates independently from per-subject claim truncation.
3. Scenario-defined `PerceptionProfile` values can specify the new field(s) without breaking default agent spawning.
4. Current-format world/profile serialization roundtrips preserve the new split field values with non-default test data.
5. `golden_merchant_selling` remains stable after the split.
6. `golden_combat` remains stable after the split.
7. Existing suite: `cargo test --workspace`

### Invariants

1. Cross-entity memory breadth and per-subject claim depth are distinct policy knobs with distinct enforcement sites.
2. Derived `known_entities` never loses a canonical fresh observation because a per-subject claim vector was truncated by an unrelated breadth limit.
3. The passive/witnessed/Tell claim-backed lane remains canonical for S54; this ticket does not reopen direct-writer coexistence scope.
4. Scenario profile completeness remains intact for any new `PerceptionProfile` field.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — split-capacity unit tests for breadth vs depth and fresh-snapshot preservation
2. `crates/worldwake-core/src/world.rs` and/or current-format save tests — profile/world roundtrip with non-default split fields
3. `crates/worldwake-cli/src/scenario/mod.rs` and/or `crates/worldwake-cli/src/scenario/types.rs` — scenario profile roundtrip/application for the new field(s)
4. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — focused no-regression verification for retention-sensitive merchant behavior
5. `crates/worldwake-ai/tests/golden_combat.rs` — focused no-regression verification for death/corpse/direct-observation behavior

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-cli`
3. `cargo test -p worldwake-ai --test golden_merchant_selling combined_market_trip_selected_for_side_benefit_replays_deterministically -- --nocapture`
4. `cargo test -p worldwake-ai --test golden_combat golden_death_cascade_and_opportunistic_loot -- --nocapture`
5. `cargo test -p worldwake-ai --test golden_combat golden_defend_changed_conditions -- --nocapture`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- **Completed**: 2026-04-05
- **What changed**:
  - Split `PerceptionProfile.memory_capacity` into `entity_memory_capacity` and `entity_claim_capacity`
  - Rewired `AgentBeliefStore::enforce_capacity` and `enforce_entity_claim_capacity` to use the separate breadth and depth controls
  - Updated scenario/profile plumbing, current-format roundtrip fixtures, and helper literals across core, systems, CLI, and AI golden harness setup
  - Added focused core regression coverage proving breadth eviction and per-subject claim truncation are independent
  - Updated the active S54 spec text to reflect the split profile contract
- **Deviations from original plan**:
  - `crates/worldwake-sim/src/save.rs` did not require direct edits; current-format persistence fallout was fully covered by existing world/profile roundtrip surfaces
  - Focused merchant/combat verification passed without adding new assertions in the golden files beyond the shared profile-literal sweep
- **Verification**:
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-cli`
  - `cargo test -p worldwake-ai --test golden_merchant_selling combined_market_trip_selected_for_side_benefit_replays_deterministically -- --nocapture`
  - `cargo test -p worldwake-ai --test golden_combat golden_death_cascade_and_opportunistic_loot -- --nocapture`
  - `cargo test -p worldwake-ai --test golden_combat golden_defend_changed_conditions -- --nocapture`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
