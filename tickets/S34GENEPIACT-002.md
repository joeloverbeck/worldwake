# S34GENEPIACT-002: Action infrastructure — ActionDomain::Epistemic, payload types, ActionPayload variants

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-sim: new domain variant, new payload structs, new ActionPayload variants
**Deps**: S34GENEPIACT-001 (VerificationSubject type must exist in worldwake-core)

## Problem

The action framework needs `ActionDomain::Epistemic`, `VerifyBeliefPayload`, `AskWitnessPayload`, and corresponding `ActionPayload` variants before action handlers can be implemented. Without these, tickets 003 and 004 cannot compile.

## Assumption Reassessment (2026-03-28)

1. `ActionDomain` is defined in `crates/worldwake-sim/src/action_domain.rs:4-15` with 10 variants (Generic through Corpse). No `Epistemic` variant exists. The `counts_as_combat_engagement()` method (line 19) returns false for non-combat domains and will naturally return false for `Epistemic`.
2. `ActionPayload` is defined in `crates/worldwake-sim/src/action_payload.rs:8-28` with 18 variants. Each variant has a corresponding `as_<name>()` const accessor method. The `InvestigateActionPayload` (line 26) is the closest structural precedent.
3. `InvestigateActionPayload` is defined in `crates/worldwake-sim/src/action_payload.rs` and contains a `ViolationId` and `EntityId` place. The new `VerifyBeliefPayload` contains a `VerificationSubject`, and `AskWitnessPayload` contains target + topic fields.
4. All payload types in the codebase derive `Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize`.
5. Single-layer ticket (sim types only). No handler logic, no AI logic.

## Architecture Check

1. Adding `ActionDomain::Epistemic` is the cleanest way to categorize verify_belief and ask_witness. They don't fit `Social` (which is Tell/conversation) or `Generic`. A new domain keeps the taxonomy honest.
2. Payload structs follow the exact pattern of `InvestigateActionPayload`. No backward-compatibility shims.

## Verification Layers

1. `ActionDomain::Epistemic` compiles and does not count as combat engagement -> focused unit test
2. `VerifyBeliefPayload` and `AskWitnessPayload` serde round-trip -> focused unit test
3. `ActionPayload::VerifyBelief` and `ActionPayload::AskWitness` variants with accessors compile -> compilation + focused unit test
4. Single-layer ticket; no cross-layer mapping needed.

## What to Change

### 1. Add `ActionDomain::Epistemic`

In `crates/worldwake-sim/src/action_domain.rs`, add `Epistemic` variant to the enum. Verify `counts_as_combat_engagement()` returns `false` for it (should be automatic if using a catch-all arm, otherwise add explicit arm).

### 2. Add payload structs

In `crates/worldwake-sim/src/action_payload.rs`, add:

```rust
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct VerifyBeliefPayload {
    pub subject: VerificationSubject,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AskWitnessPayload {
    pub target: EntityId,
    pub topic_entity: Option<EntityId>,
    pub topic_commodity: Option<CommodityKind>,
}
```

### 3. Add ActionPayload variants and accessors

In `crates/worldwake-sim/src/action_payload.rs`:
- Add `VerifyBelief(VerifyBeliefPayload)` and `AskWitness(AskWitnessPayload)` to the enum.
- Add `as_verify_belief()` and `as_ask_witness()` const accessor methods following the existing pattern.

### 4. Re-exports

In `crates/worldwake-sim/src/lib.rs`, add re-exports for `VerifyBeliefPayload` and `AskWitnessPayload`.

## Files to Touch

- `crates/worldwake-sim/src/action_domain.rs` (modify — add Epistemic variant)
- `crates/worldwake-sim/src/action_payload.rs` (modify — add 2 structs, 2 variants, 2 accessors)
- `crates/worldwake-sim/src/lib.rs` (modify — add re-exports)

## Out of Scope

- Action definitions (ActionDef registration) — ticket 003/004
- Action handlers (start/tick/commit/abort functions) — ticket 003/004
- Planner ops, candidate generation, ranking — tickets 005/006/007
- `ActionDomain::Epistemic` usage in `counts_as_combat_engagement` beyond returning false
- Any changes to existing payload types or domains

## Acceptance Criteria

### Tests That Must Pass

1. `ActionDomain::Epistemic` does not count as combat engagement
2. Serde round-trip for `VerifyBeliefPayload` with `EntityLocation` subject
3. Serde round-trip for `VerifyBeliefPayload` with `SupplyAvailability` subject
4. Serde round-trip for `AskWitnessPayload` with both topic fields populated
5. Serde round-trip for `AskWitnessPayload` with only `topic_entity` populated
6. `ActionPayload::VerifyBelief(payload).as_verify_belief()` returns `Some`
7. `ActionPayload::AskWitness(payload).as_ask_witness()` returns `Some`
8. `ActionPayload::VerifyBelief(payload).as_investigate()` returns `None` (cross-variant rejection)
9. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. `ActionPayload` match exhaustiveness — all existing match arms updated for new variants (compiler-enforced)
2. All new types derive `Serialize`/`Deserialize` for save/load compatibility (P11)
3. No floats — all fields use integer types or `Permille`/`Tick`/`EntityId`/`CommodityKind`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_domain.rs` (in-module tests) — Epistemic combat engagement check
2. `crates/worldwake-sim/src/action_payload.rs` (in-module tests) — serde round-trips + accessor correctness

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy -p worldwake-sim`
3. `cargo build --workspace`
