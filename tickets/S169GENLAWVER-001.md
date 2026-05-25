# S169GENLAWVER-001: Verification foundation types and RepairAppliedPayload extension

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core/src/decision_event_payload.rs` (new enum + field on `RepairAppliedPayload`), new `worldwake-ai/src/verification_provider/` module
**Deps**: specs/S169-generalized-lawful-verification-substrate.md

## Problem

S165 wired `RepairKind::InsertVerification` to splice `AskWitness` verification steps but the candidate construction is exclusively `AskWitness`-centric. S169 broadens the substrate to three lawful provider kinds (`AskWitness`, `ConsultRecord`, `SearchPlace`). This ticket lays the typed foundation: introduces `VerificationProviderKind` in core (so `RepairAppliedPayload` can carry it without a crate-layering violation), extends `RepairAppliedPayload` with the new `provider_kind` field, and creates the `worldwake-ai/src/verification_provider/` module with the supporting types (`VerificationNeed`, `VerificationCandidate`, `VerificationTarget`, `VerificationRejection`, `VerificationContext`) that downstream tickets consume.

No behavior change in this ticket — the registry, the seam refactor, and the provider implementations land in S169GENLAWVER-002. This is pure substrate.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `RepairAppliedPayload` is defined at `crates/worldwake-core/src/decision_event_payload.rs:435` with fields `agent, goal_key, step_index, repair_kind, substitute_target, substitute_recipe`. The existing `#[serde(default)]` precedent on `substitute_recipe` (line ~440) is the pattern to follow for the new `provider_kind` field.
2. There are 10 `RepairAppliedPayload { ... }` construction sites across 4 files: `worldwake-core/src/decision_event_payload.rs` (struct def + 2 inline tests), `worldwake-ai/tests/scenarios/plan_repair.rs` (3 test sites), `worldwake-cli/src/bin/observer.rs` (3 sites), `worldwake-sim/src/save_load.rs` (1 site). All use explicit field enumeration; no spread-syntax escape hatch. Every site needs an explicit `provider_kind:` field added.
3. Mixed-layer boundary under audit: `worldwake-core` cannot depend on higher crates per workspace layering. `VerificationProviderKind` must live in core because `RepairAppliedPayload.provider_kind` will reference it. The enum is payload-free (3 unit variants) — no `Tag` mirror needed per FND-28.
4. Adjacent contradictions classified: none. The `provider_kind` field is semantically meaningful only when `repair_kind == InsertVerification`; for other repair kinds the field defaults to `AskWitness` but is unused. This is the existing pattern (`substitute_target` is only meaningful for certain repair kinds too).

## Architecture Check

1. `VerificationProviderKind` is placed in `worldwake-core/src/decision_event_payload.rs` alongside `RepairAppliedPayload` because the payload's field type must be locally resolvable from core. Adding a core-side `*Tag` mirror would create FND-28 parallel-authority for what is semantically a single 3-variant payload-free enum.
2. Backward-compat for serialized event-log streams is handled by `#[serde(default)]` + `impl Default for VerificationProviderKind { fn default() -> Self { Self::AskWitness } }`. Old saved events (where `provider_kind` was not yet serialized) deserialize with `AskWitness` — semantically correct because pre-S169 verification repairs were exclusively `AskWitness`. No `SAVE_FORMAT_VERSION` bump needed; the project has no such constant.
3. The `worldwake-ai/src/verification_provider/` module is laid out as a single `mod.rs` with the supporting types. Provider submodules (`ask_witness_provider.rs`, `consult_record_provider.rs`, `search_place_provider.rs`) are added by downstream tickets — this ticket does not create them.

## Verification Layers

1. `VerificationProviderKind` serializes/deserializes correctly with `#[serde(default)]` semantics -> bincode roundtrip unit test in `decision_event_payload.rs` inline `#[cfg(test)]` block.
2. Old `RepairAppliedPayload` byte streams (without `provider_kind` field) deserialize to `provider_kind = AskWitness` -> dedicated roundtrip test asserting empty-trailing-bytes deserialization.
3. New types in `worldwake-ai/src/verification_provider/mod.rs` compile and re-export through `worldwake-ai/src/lib.rs` -> compile-only verification by `cargo check -p worldwake-ai`.

## What to Change

### 1. Add `VerificationProviderKind` enum in `worldwake-core/src/decision_event_payload.rs`

Place near `RepairAppliedPayload` (around line 430):

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum VerificationProviderKind {
    AskWitness,
    ConsultRecord,
    SearchPlace,
}

impl Default for VerificationProviderKind {
    fn default() -> Self {
        Self::AskWitness
    }
}
```

Add the export to `worldwake-core/src/lib.rs` (alongside other `decision_event_payload` re-exports).

### 2. Extend `RepairAppliedPayload` with `provider_kind` field

```rust
pub struct RepairAppliedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub step_index: u16,
    pub repair_kind: RepairKind,
    pub substitute_target: Option<EntityId>,
    #[serde(default)]
    pub substitute_recipe: Option<RecipeId>,
    #[serde(default)]
    pub provider_kind: VerificationProviderKind,
}
```

### 3. Update all `RepairAppliedPayload { ... }` construction sites

Add explicit `provider_kind: VerificationProviderKind::AskWitness` field to all 10 sites. Most are tests/observer/save_load; the value is `AskWitness` for all existing call sites (pre-S169 verification repairs were exclusively AskWitness; non-verification repairs have unused `provider_kind` defaulting to `AskWitness`).

Sites:
- `crates/worldwake-core/src/decision_event_payload.rs:741` (inline test)
- `crates/worldwake-core/src/decision_event_payload.rs:909` (inline test)
- `crates/worldwake-ai/tests/scenarios/plan_repair.rs:272, :355, :734`
- `crates/worldwake-cli/src/bin/observer.rs:6732, :8665, :8745`
- `crates/worldwake-sim/src/save_load.rs:1292`

### 4. Create `worldwake-ai/src/verification_provider/mod.rs`

```rust
use worldwake_core::{
    BeliefRef, EntityBeliefAspect, EntityId, ExpectationId, RecordTopic,
    VerificationProviderKind,
};
use worldwake_sim::PerAgentBeliefView;
use crate::plan_repair::RepairPlanCandidate;

pub enum VerificationNeed {
    StaleEntityBelief { subject: EntityId, aspect: EntityBeliefAspect },
    StaleInstitutionalClaim { record_topic: RecordTopic },
    OverdueExpectationAtPlace { expectation: ExpectationId, place: EntityId },
}

pub struct VerificationCandidate {
    pub provider_kind: VerificationProviderKind,
    pub target: VerificationTarget,
    pub repair_candidate: RepairPlanCandidate,
    pub source_belief: Option<BeliefRef>,
}

pub enum VerificationTarget {
    Witness(EntityId),
    Record(EntityId),
    Place(EntityId),
}

pub enum VerificationRejection {
    BreachClassMismatch,
    NoLawfulLocalTarget,
    PayloadValidationFailed,
    RecentlyFailedAtTarget,
}

pub struct VerificationContext<'a> {
    pub actor: EntityId,
    pub belief_view: &'a PerAgentBeliefView<'a>,
    pub effective_place: EntityId,
    // breach context + seam repair scaffolding added in S169GENLAWVER-002
}
```

The exact `VerificationContext` field list will be finalized in S169GENLAWVER-002 once the seam-side data passed into the registry is known. This ticket lands the type with `actor`, `belief_view`, `effective_place` as the minimum required surface.

Add `pub mod verification_provider;` to `worldwake-ai/src/lib.rs`.

## Files to Touch

- `crates/worldwake-core/src/decision_event_payload.rs` (modify — add enum + extend struct + tests)
- `crates/worldwake-core/src/lib.rs` (modify — re-export `VerificationProviderKind`)
- `crates/worldwake-ai/src/verification_provider/mod.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — add `pub mod verification_provider;`)
- `crates/worldwake-ai/tests/scenarios/plan_repair.rs` (modify — update 3 construction sites)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — update 3 construction sites)
- `crates/worldwake-sim/src/save_load.rs` (modify — update 1 construction site)

## Out of Scope

- `try_build_verification_candidate` registry function — S169GENLAWVER-002.
- AskWitness / ConsultRecord / SearchPlace provider submodules and `try_build` implementations — S169GENLAWVER-002, -003, -004.
- Seam refactor at `agent_tick/execution.rs:452` — S169GENLAWVER-002.
- `RepairAttemptTrace` extension with `verification_provider` / `verification_rejections` fields — S169GENLAWVER-002.
- New goldens — S169GENLAWVER-003, -004, -005.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core decision_event_payload_variants_roundtrip_through_bincode` — existing roundtrip test passes with new field.
2. New test `repair_applied_payload_provider_kind_defaults_to_ask_witness_on_legacy_bytes` — confirms backward-compat default deserialization.
3. New test `verification_provider_kind_derives_satisfy_event_payload_bounds` — confirms `Copy + Hash + Ord + Serialize + Deserialize` derives.
4. Existing suite: `cargo test --workspace` passes with all construction sites updated.

### Invariants

1. `VerificationProviderKind` lives in `worldwake-core/src/decision_event_payload.rs`; no `Tag` mirror added (FND-28: single authoritative form).
2. `RepairAppliedPayload` has exactly one new field (`provider_kind`); no new `target` field added (the existing `substitute_target` continues to carry the target entity).
3. All 10 construction sites explicitly set `provider_kind`; no site relies on `..Default::default()` spread syntax.
4. `worldwake-ai/src/verification_provider/mod.rs` has no provider submodules at the end of this ticket (mod-declaration only, populated by 002+).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/decision_event_payload.rs` inline `#[cfg(test)]` — add `repair_applied_payload_provider_kind_defaults_to_ask_witness_on_legacy_bytes` testing serde-default deserialization on a pre-S169-format byte stream.
2. `crates/worldwake-core/src/decision_event_payload.rs` inline `#[cfg(test)]` — extend `decision_event_payload_variants_roundtrip_through_bincode` to cover `provider_kind = ConsultRecord` and `provider_kind = SearchPlace` variants.

### Commands

1. `cargo test -p worldwake-core decision_event_payload` — focused validation of payload roundtrips.
2. `cargo check -p worldwake-ai` — verify new `verification_provider` module compiles.
3. `cargo test --workspace` — full suite to catch any missed construction sites.
4. `./scripts/verify.sh` — pre-PR gate.

Merge note: Ticket 001 introduces a serialization-format change (new field on `RepairAppliedPayload`); backward-compat handled by `#[serde(default)]` + `impl Default for VerificationProviderKind`. No `SAVE_FORMAT_VERSION` bump required (project has no such constant; bincode + serde defaults are the project's compatibility surface).
