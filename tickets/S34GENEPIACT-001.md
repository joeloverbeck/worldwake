# S34GENEPIACT-001: Core types — VerificationSubject, VerificationDispositionProfile, GoalKind::VerifyBelief

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-core: new enum, new component, new goal kind variant
**Deps**: S27 (completed — provides ViolationKind, ViolationDispositionProfile patterns)

## Problem

S34 introduces deliberate epistemic actions but the core types they depend on do not exist yet. Without `VerificationSubject`, `VerificationDispositionProfile`, and `GoalKind::VerifyBelief`, no downstream ticket (action handlers, planner ops, candidate generation) can compile.

## Assumption Reassessment (2026-03-28)

1. `GoalKind` is defined in `crates/worldwake-core/src/goal.rs:16-84` with 20 existing variants. No `VerifyBelief` variant exists. The `InvestigateViolation` variant (line 66-69) is the closest structural precedent.
2. `ViolationDispositionProfile` is defined in `crates/worldwake-core/src/violation.rs:168-177` and registered in `component_schema.rs`. This is the structural pattern for `VerificationDispositionProfile`.
3. `ViolationKind` is defined in `crates/worldwake-core/src/violation.rs:24-43` with `EntityMissing` and `SupplyDepleted` variants. The new `VerificationSubject` enum mirrors these shapes but serves a different purpose (proactive verification targets vs reactive violation records).
4. `CommodityKind` is in `crates/worldwake-core/src/items.rs`. `Permille` is in `crates/worldwake-core/src/numerics.rs`. `Tick` is in `crates/worldwake-core/src/ids.rs`. All needed imports exist.
5. `component_schema.rs` uses the `with_component_schema_entries!` macro for typed storage registration. `ViolationDispositionProfile` registration (lines ~1033-1056) is the pattern to follow.
6. Single-layer ticket (core types only). No AI, planner, or handler logic.

## Architecture Check

1. Placing `VerificationSubject` alongside `ViolationKind` in `violation.rs` would conflate reactive violations with proactive verification. A new `verification.rs` module in worldwake-core is cleaner — it parallels the existing `violation.rs` for the epistemic domain.
2. No backward-compatibility shims. New types only.

## Verification Layers

1. `VerificationSubject` enum compiles with correct variants -> focused unit test (serde round-trip)
2. `VerificationDispositionProfile` registered on `EntityKind::Agent` -> focused unit test (component insert/get on Agent entity)
3. `GoalKind::VerifyBelief` variant exists with correct fields -> compilation + focused unit test (goal key derivation)
4. Single-layer ticket; no cross-layer mapping needed.

## What to Change

### 1. New `verification.rs` module in worldwake-core

Create `crates/worldwake-core/src/verification.rs` containing:

- `VerificationSubject` enum with `EntityLocation { entity: EntityId, place: EntityId }` and `SupplyAvailability { commodity: CommodityKind, source: EntityId, place: EntityId }` variants. Derive `Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize`.
- `VerificationDispositionProfile` struct with fields per spec: `belief_verification_threshold: Permille`, `verify_belief_duration_ticks: NonZeroU32`, `witness_query_duration_ticks: NonZeroU32`, `verification_motive_weight: Permille`, `ask_memory_retention_ticks: u32`. Derive `Debug, Clone, Serialize, Deserialize`. Implement `Component` trait.

### 2. Add `GoalKind::VerifyBelief` variant

In `crates/worldwake-core/src/goal.rs`, add:
```rust
VerifyBelief {
    subject: VerificationSubject,
    generation_tick: Tick,
},
```

Update all existing match arms on `GoalKind` across worldwake-core (e.g., `goal_key()` derivation, Display impl if any, serde coverage).

### 3. Register `VerificationDispositionProfile` in component schema

In `crates/worldwake-core/src/component_schema.rs`, add a `with_component_schema_entries!` entry for `VerificationDispositionProfile` on `EntityKind::Agent`, following the `ViolationDispositionProfile` pattern.

### 4. Wire module and re-exports

- Add `pub mod verification;` to `crates/worldwake-core/src/lib.rs`.
- Re-export `VerificationSubject`, `VerificationDispositionProfile` from `lib.rs`.

## Files to Touch

- `crates/worldwake-core/src/verification.rs` (new)
- `crates/worldwake-core/src/goal.rs` (modify — add VerifyBelief variant, update GoalKey derivation)
- `crates/worldwake-core/src/component_schema.rs` (modify — register VerificationDispositionProfile)
- `crates/worldwake-core/src/lib.rs` (modify — add module, re-exports)

## Out of Scope

- Action payload types (`VerifyBeliefPayload`, `AskWitnessPayload`) — those live in worldwake-sim (ticket 002)
- Action handlers — ticket 003/004
- Planner ops, candidate generation, ranking — tickets 005/006/007
- `GoalKindTag::VerifyBelief` in worldwake-ai — ticket 005
- `ComponentDelta` / `ComponentValue` coverage for `VerificationDispositionProfile` in `delta.rs` — if the existing macro-generated pattern handles it automatically, no additional work; otherwise handle in this ticket
- Any changes to `ViolationKind` or `ViolationDispositionProfile`

## Acceptance Criteria

### Tests That Must Pass

1. Serde round-trip test for `VerificationSubject::EntityLocation` and `VerificationSubject::SupplyAvailability`
2. Serde round-trip test for `VerificationDispositionProfile`
3. Component schema test: insert and retrieve `VerificationDispositionProfile` on an Agent entity
4. Component schema test: inserting `VerificationDispositionProfile` on a non-Agent entity fails (or is rejected per schema rules)
5. `GoalKind::VerifyBelief` serde round-trip for both subject variants
6. `GoalKey` derivation for `VerifyBelief` produces distinct keys for different entities at the same place
7. `GoalKey` derivation for `VerifyBelief` produces distinct keys for `EntityLocation` vs `SupplyAvailability` at the same place
8. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `GoalKind` match exhaustiveness — all existing match arms updated for new variant (compiler-enforced)
2. `VerificationDispositionProfile` is only registrable on `EntityKind::Agent`
3. All new types derive `Serialize`/`Deserialize` for save/load compatibility (P11)
4. No `HashMap`/`HashSet` — only `BTreeMap`/`BTreeSet` in any new collections (determinism invariant)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/verification.rs` (in-module tests) — serde round-trips for both types
2. `crates/worldwake-core/src/goal.rs` (in-module or existing test block) — GoalKey derivation uniqueness for VerifyBelief variants
3. `crates/worldwake-core/src/component_schema.rs` (existing test block) — component registration on Agent

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy -p worldwake-core`
3. `cargo build --workspace`
