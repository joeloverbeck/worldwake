# S34GENEPIACT-002: Action infrastructure — ActionDomain::Epistemic, payload types, ActionPayload variants

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-sim: new domain variant, new payload structs, new ActionPayload variants
**Deps**: S34GENEPIACT-001 (VerificationSubject type must exist in worldwake-core)

## Problem

The action framework needs `ActionDomain::Epistemic`, `VerifyBeliefPayload`, `AskWitnessPayload`, and corresponding `ActionPayload` variants before action handlers can be implemented. Without these, tickets 003 and 004 cannot compile.

## Assumption Reassessment (2026-03-28)

1. Shared abstraction boundary under audit: the sim-layer action identity contract formed by `crates/worldwake-sim/src/action_domain.rs`, `crates/worldwake-sim/src/action_payload.rs`, and the public re-exports in `crates/worldwake-sim/src/lib.rs`. This ticket is complete when epistemic actions have first-class sim typing that downstream action defs and handlers can name directly.
2. `ActionDomain` is defined in `crates/worldwake-sim/src/action_domain.rs` with 10 variants (`Generic` through `Corpse`). No `Epistemic` variant exists. `counts_as_combat_engagement()` is currently `matches!(self, Self::Combat)`, so adding `Epistemic` should remain a no-combat domain automatically, but the invariant should stay covered by the existing all-domains test table rather than by a one-off assertion.
3. `ActionPayload` is defined in `crates/worldwake-sim/src/action_payload.rs` with 18 variants (`None` plus 17 typed payload variants). The file already follows the intended extensibility pattern for new action kinds: a typed enum variant, a dedicated payload struct, and a typed `as_*` accessor. This ticket should extend that existing contract rather than introduce any aliasing or shared "epistemic payload" wrapper.
4. `InvestigateActionPayload` is the closest live precedent for a focused information-seeking payload, but the ticket narrative was stale about its shape: it currently contains only `violation_id: ViolationId`, not a place field. The new epistemic payloads should therefore be justified by their own data contract, not by a nonexistent structural match to an older investigate shape.
5. `VerificationSubject` already exists in `crates/worldwake-core/src/epistemic.rs`, is re-exported from `crates/worldwake-core/src/lib.rs`, and is already embedded in `GoalKind::VerifyBelief` in `crates/worldwake-core/src/goal.rs`. The live code therefore already committed to `VerificationSubject` as the canonical cross-layer identity for proactive verification; `VerifyBeliefPayload` should reuse that type directly instead of inventing a sim-local alias or decomposed duplicate fields.
6. The existing sim test surface for these files is bincode-focused, not generic serde-json-focused: `action_domain.rs` already verifies trait bounds plus bincode round-trips over a canonical `ALL_DOMAINS` table, and `action_payload.rs` already verifies trait bounds, typed accessors, and bincode round-trips for representative variants. The ticket should extend those established focused tests instead of requiring a new serialization style.
7. This remains a single-layer sim ticket. No handler behavior, affordance enumeration, or AI pipeline logic belongs here. The only cross-crate dependency is reuse of the already-authoritative `VerificationSubject`, `CommodityKind`, and `EntityId` core types.

## Architecture Check

1. Adding `ActionDomain::Epistemic` is cleaner than folding these actions into `Generic` or `Social`. `verify_belief` and `ask_witness` are both information-seeking actions with distinct planner and debugging semantics, and a dedicated domain keeps the action taxonomy explicit for future traceability without coupling them to conversation-only or catch-all buckets.
2. `VerifyBeliefPayload { subject: VerificationSubject }` is cleaner than duplicating entity/place/commodity fields inside `worldwake-sim`. The core layer already established `VerificationSubject` as the canonical representation for this fact path, so reusing it avoids parallel encodings and future drift.
3. `AskWitnessPayload` should stay as a dedicated payload struct with explicit optional topic fields, not as an overloaded `TellTopic` alias. The spec models asking as a distinct action with its own validation surface and memory consequences; preserving a dedicated payload keeps that boundary extensible and avoids backwards-compatibility shims later.

## Verification Layers

1. `ActionDomain::Epistemic` is part of the canonical domain set and still only `Combat` counts as combat engagement -> focused `action_domain` unit test over the full domain table
2. `VerifyBeliefPayload` and `AskWitnessPayload` satisfy the existing sim payload trait/serialization contract -> focused `action_payload` unit tests with bincode round-trips
3. `ActionPayload::VerifyBelief` and `ActionPayload::AskWitness` integrate into the typed accessor surface without variant bleed -> focused `action_payload` accessor tests
4. Public `worldwake-sim` consumers can name the new payload types directly -> compile-time proof via `lib.rs` re-exports plus crate test compilation

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
- Update the existing grouped accessor tests so the new variants participate in the same exhaustive cross-variant rejection coverage as the rest of the enum.

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
- Any sim-local alias for `VerificationSubject` or any backwards-compatibility compatibility path that duplicates the new payload contract
- Any change to existing non-epistemic payload semantics beyond the exhaustiveness updates required by the new enum variants

## Acceptance Criteria

### Tests That Must Pass

1. `ActionDomain::Epistemic` participates in the all-domains table and still only `Combat` counts as combat engagement
2. `VerifyBeliefPayload` bincode round-trip with `VerificationSubject::EntityLocation`
3. `VerifyBeliefPayload` bincode round-trip with `VerificationSubject::SupplyAvailability`
4. `AskWitnessPayload` bincode round-trip with both topic fields populated
5. `AskWitnessPayload` bincode round-trip with only `topic_entity` populated
6. `ActionPayload::VerifyBelief(payload).as_verify_belief()` returns `Some`
7. `ActionPayload::AskWitness(payload).as_ask_witness()` returns `Some`
8. `ActionPayload::VerifyBelief(payload).as_investigate()` returns `None` and `ActionPayload::AskWitness(payload).as_tell()` returns `None`
9. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. `ActionPayload` match exhaustiveness — all existing match arms updated for new variants (compiler-enforced)
2. All new types derive `Serialize`/`Deserialize` for save/load compatibility (P11)
3. No floats — all fields use integer types or `Permille`/`Tick`/`EntityId`/`CommodityKind`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_domain.rs` (in-module tests) — extend the canonical `ALL_DOMAINS` table and the existing combat-engagement invariant coverage
2. `crates/worldwake-sim/src/action_payload.rs` (in-module tests) — add epistemic payload round-trips and extend grouped accessor coverage to the new variants

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy -p worldwake-sim`
3. `cargo build --workspace`

## Outcome

- Completed: 2026-03-28
- What changed:
  - Added `ActionDomain::Epistemic` to the canonical sim action-domain set.
  - Added `VerifyBeliefPayload` and `AskWitnessPayload` to `crates/worldwake-sim/src/action_payload.rs`.
  - Added `ActionPayload::VerifyBelief` and `ActionPayload::AskWitness` plus `as_verify_belief()` / `as_ask_witness()` accessors.
  - Re-exported the new payload types from `crates/worldwake-sim/src/lib.rs`.
  - Extended focused sim tests for domain coverage, payload round-trips, and accessor exhaustiveness.
  - Updated `ActionTraceDetail::from_payload()` exhaustiveness so the new variants are explicitly ignored until epistemic trace details are intentionally designed.
- Deviations from original plan:
  - The ticket was corrected before implementation to match live code: `InvestigateActionPayload` no longer has a place field, and the real sim serialization proof surface is bincode-based focused tests rather than generic serde wording.
  - No sim-local alias or compatibility wrapper was introduced; `VerifyBeliefPayload` reuses the already-canonical `VerificationSubject` from `worldwake-core`.
- Verification results:
  - `cargo test -p worldwake-sim` passed
  - `cargo clippy -p worldwake-sim --all-targets -- -D warnings` passed
  - `cargo build --workspace` passed
