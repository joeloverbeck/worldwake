# S71EVELOGDEL-001: Define `BeliefStoreDiff` type and diff/apply logic

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new type in `worldwake-core` belief module
**Deps**: None

## Problem

The append-only event log stores full before+after `ComponentValue` snapshots in every `ComponentDelta::Set` record. For `AgentBeliefStore` (~150 KB at steady state), this means ~300 KB per perception event per agent. With 20 agents the event log grows by ~6 MB/tick, causing OOM within ~1,100 ticks. A compact structural diff type is needed that captures only the mutations (~1-5 KB) while preserving full reconstructability.

## Assumption Reassessment (2026-04-08)

1. `AgentBeliefStore` definition confirmed at `crates/worldwake-core/src/belief.rs:44` with fields: `entity_claims`, `next_claim_id`, `known_entities`, `social_observations`, `told_beliefs`, `heard_beliefs`, `asked_witnesses`, `institutional_beliefs`.
2. All field value types confirmed: `BelievedEntityState` at `belief.rs:1058`, `SocialObservation` at `belief.rs:1783`, `TellMemoryKey` at `belief.rs:1085`, `ToldBeliefMemory` at `belief.rs:1091`, `HeardBeliefMemory` at `belief.rs:1097`, `AskWitnessMemoryKey` at `belief.rs:1104`, `AskWitnessMemory` at `belief.rs:1111`, `EntityBeliefClaim` at `entity_belief_claim.rs:47`, `InstitutionalBeliefKey` at `institutional.rs:194`, `BelievedInstitutionalClaim` at `institutional.rs:221`, `ClaimId` is a newtype struct at `entity_belief_claim.rs:14`.
3. `AgentBeliefStore` implements `Component` trait (`belief.rs:851`), derives `Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize`.
4. This is a single-crate ticket within `worldwake-core`. No cross-system boundary under audit.
5. Not a planner/golden-driven ticket.
6. Not an AI regression ticket.
14. No mismatches found; all 8 fields of `AgentBeliefStore` are accounted for in the proposed `BeliefStoreDiff`.

## Architecture Check

1. A dedicated diff type with `compute(before, after) -> Self` and `apply(self, base) -> AgentBeliefStore` is cleaner than inlining diff logic at the delta-emission site. It keeps the diff algorithm testable in isolation and reusable if other consumers need it.
2. No backward-compatibility shims. The diff type is new; it does not wrap or alias any existing type.

## Verification Layers

1. Roundtrip correctness (`apply(compute(before, after), before) == after`) -> focused unit test with diverse belief store states
2. Empty diff (no changes) -> focused unit test confirming `compute(x, x)` yields an empty diff and `apply(empty, x) == x`
3. Single-layer ticket (new type + unit tests within `worldwake-core`); additional layer mapping not applicable until integration in subsequent tickets.

## What to Change

### 1. Define `BeliefStoreDiff` struct

In `crates/worldwake-core/src/belief.rs`, add:

```rust
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeliefStoreDiff {
    pub next_claim_id: Option<ClaimId>,
    pub known_entities_set: Vec<(EntityId, BelievedEntityState)>,
    pub known_entities_removed: Vec<EntityId>,
    pub social_observations_added: Vec<SocialObservation>,
    pub social_observations_removed_count: u16,
    pub told_beliefs_set: Vec<(TellMemoryKey, ToldBeliefMemory)>,
    pub told_beliefs_removed: Vec<TellMemoryKey>,
    pub heard_beliefs_set: Vec<(TellMemoryKey, HeardBeliefMemory)>,
    pub heard_beliefs_removed: Vec<TellMemoryKey>,
    pub asked_witnesses_set: Vec<(AskWitnessMemoryKey, AskWitnessMemory)>,
    pub asked_witnesses_removed: Vec<AskWitnessMemoryKey>,
    pub entity_claims_set: Vec<(EntityId, Vec<EntityBeliefClaim>)>,
    pub entity_claims_removed: Vec<EntityId>,
    pub institutional_beliefs_set: Vec<(InstitutionalBeliefKey, Vec<BelievedInstitutionalClaim>)>,
    pub institutional_beliefs_removed: Vec<InstitutionalBeliefKey>,
}
```

Derive `Default` so an empty diff (no mutations) is representable.

### 2. Implement `BeliefStoreDiff::compute`

```rust
impl BeliefStoreDiff {
    pub fn compute(before: &AgentBeliefStore, after: &AgentBeliefStore) -> Self { ... }
}
```

For each BTreeMap field: diff keys present in `after` but not `before` (or with different values) into `*_set`; keys in `before` but not `after` into `*_removed`.

For `social_observations` (a `Vec`): compare by computing which entries were added (present in `after` tail) and how many were removed from the front (count difference).

For `next_claim_id`: store `Some(after.next_claim_id)` only if it differs from `before.next_claim_id`.

### 3. Implement `BeliefStoreDiff::apply`

```rust
impl BeliefStoreDiff {
    pub fn apply(self, base: &AgentBeliefStore) -> AgentBeliefStore { ... }
}
```

Clone `base`, then apply all set/remove operations, update `next_claim_id` if `Some`. Return the modified store.

### 4. Implement `BeliefStoreDiff::is_empty`

Return true if all Vecs are empty and `next_claim_id` is `None`. Useful for callers that want to skip emitting a delta when nothing changed (though `replace_simple_component` already short-circuits on equality).

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — add `BeliefStoreDiff` struct + `compute`, `apply`, `is_empty` methods + unit tests)
- `crates/worldwake-core/src/lib.rs` (modify — add `BeliefStoreDiff` to crate-root re-export)

## Out of Scope

- Modifying `ComponentDelta` enum (ticket 002)
- Wiring diff into `WorldTxn` commit path (ticket 003)
- Updating verification or other consumers (tickets 004, 005)
- Diff types for non-belief-store components

## Acceptance Criteria

### Tests That Must Pass

1. `BeliefStoreDiff::compute` + `apply` roundtrip: `apply(compute(before, after), before) == after` for diverse store states (empty, populated, partial overlap)
2. Empty diff: `compute(x, x)` yields `BeliefStoreDiff::default()` and `apply(default(), x) == x`
3. Each field mutation type tested in isolation: add-only, remove-only, update (set with changed value)
4. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `apply(compute(before, after), before) == after` for all valid `AgentBeliefStore` pairs (roundtrip correctness)
2. `BeliefStoreDiff` must satisfy `Clone + Debug + Eq + PartialEq + Serialize + Deserialize` (same trait bounds as `ComponentValue` variants)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — roundtrip tests for `BeliefStoreDiff::compute` and `apply` covering empty stores, populated stores with partial overlap, add-only mutations, remove-only mutations, mixed mutations, and `next_claim_id` changes
2. `crates/worldwake-core/src/belief.rs` — serialization roundtrip for `BeliefStoreDiff` (bincode encode/decode produces equal value)

### Commands

1. `cargo test -p worldwake-core belief_store_diff` (targeted)
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-08.

- Added `BeliefStoreDiff` struct with `compute`, `apply`, and `is_empty` methods in `crates/worldwake-core/src/belief.rs`
- Added `BeliefStoreDiff` to crate-root re-export in `crates/worldwake-core/src/lib.rs`
- Added 3 private helper functions: `diff_btree_map_set`, `diff_btree_map_removed`, `diff_social_observations`
- Social observation diff uses suffix-matching to handle the append+evict access pattern
- `ClaimId` is a newtype struct (not a type alias as originally noted in the ticket) — no impact on implementation

## Verification Result

- Passed `cargo test -p worldwake-core belief_store_diff` — 12 tests (empty, identity, per-field roundtrips, mixed mutations, full replacement, serialization)
- Passed `cargo test -p worldwake-core` — 1027 tests
- Passed `cargo clippy -p worldwake-core --all-targets -- -D warnings`
