# S169GENLAWVER-001: Verification foundation types and RepairAppliedPayload extension

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core/src/decision_event_payload.rs` (new enum + field on `RepairAppliedPayload`), new `worldwake-ai/src/verification_provider/` module
**Deps**: specs/S169-generalized-lawful-verification-substrate.md

## Problem

S165 wired `RepairKind::InsertVerification` to splice `AskWitness` verification steps but the candidate construction is exclusively `AskWitness`-centric. S169 broadens the substrate to three lawful provider kinds (`AskWitness`, `ConsultRecord`, `SearchPlace`). This ticket lays the typed foundation: introduces `VerificationProviderKind` in core (so `RepairAppliedPayload` can carry it without a crate-layering violation), extends `RepairAppliedPayload` with the new `provider_kind` field, and creates the `worldwake-ai/src/verification_provider/` module with the supporting types (`VerificationNeed`, `VerificationCandidate`, `VerificationTarget`, `VerificationRejection`, `VerificationContext`) that downstream tickets consume.

No behavior change in this ticket — the registry, the seam refactor, and the provider implementations land in S169GENLAWVER-002. This is pure substrate.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `RepairAppliedPayload` is defined in `crates/worldwake-core/src/decision_event_payload.rs` with fields `agent, goal_key, step_index, repair_kind, substitute_target, substitute_recipe`. The new `provider_kind` field is persisted on current-format payloads so provider provenance survives in append-only causal history.
2. Live constructor reassessment found explicit `RepairAppliedPayload { ... }` literals in the drafted files plus additional `worldwake-ai/src/agent_tick/{mod.rs,execution.rs,tests.rs}` sites. All use explicit field enumeration; no spread-syntax escape hatch. Every explicit site needs an explicit `provider_kind:` field added.
3. Mixed-layer boundary under audit: `worldwake-core` cannot depend on higher crates per workspace layering. `VerificationProviderKind` must live in core because `RepairAppliedPayload.provider_kind` will reference it. The enum is payload-free (3 unit variants) — no `Tag` mirror needed per FND-28.
4. Live persistence reassessment disproved the drafted compatibility claim: bincode does not default a missing trailing struct field, and `crates/worldwake-sim/src/save_load.rs` has `SAVE_FORMAT_VERSION`. Per FND-28 and `AGENTS.md` no-backward-compatibility guidance, this ticket owns a current save-format bump from 100 to 101 rather than adding a legacy decode shim.
5. The `provider_kind` field is semantically meaningful only when `repair_kind == InsertVerification`; for other repair kinds the field is set to `AskWitness` as an inert explicit value. This mirrors the existing conditional meaning of `substitute_target`.

## Architecture Check

1. `VerificationProviderKind` is placed in `worldwake-core/src/decision_event_payload.rs` alongside `RepairAppliedPayload` because the payload's field type must be locally resolvable from core. Adding a core-side `*Tag` mirror would create FND-28 parallel-authority for what is semantically a single 3-variant payload-free enum.
2. `worldwake-sim::SAVE_FORMAT_VERSION` is bumped because `provider_kind` changes the current bincode save/runtime shape. Old versioned saves are rejected by the existing save-version boundary instead of normalized through a compatibility shim.
3. The `worldwake-ai/src/verification_provider/` module is laid out as a single `mod.rs` with the supporting types. Provider submodules (`ask_witness_provider.rs`, `consult_record_provider.rs`, `search_place_provider.rs`) are added by downstream tickets — this ticket does not create them.

## Verified Layers

1. `VerificationProviderKind` serializes/deserializes correctly on current-format payloads -> bincode roundtrip unit coverage in `decision_event_payload.rs`.
2. Old save-format bytes are rejected at the existing `SAVE_FORMAT_VERSION` boundary -> focused save/load version tests in `worldwake-sim`.
3. `worldwake-ai/src/verification_provider/mod.rs` compiles and re-exports through `worldwake-ai/src/lib.rs` -> compile-only verification by `cargo check -p worldwake-ai`.

## Landed Changes

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
    pub provider_kind: VerificationProviderKind,
}
```

### 3. Update all `RepairAppliedPayload { ... }` construction sites

Add explicit `provider_kind: VerificationProviderKind::AskWitness` field to all explicit sites. Most are tests/observer/save_load/agent_tick fallout; the value is `AskWitness` for all existing call sites (pre-S169 verification repairs were exclusively AskWitness; non-verification repairs carry an inert `AskWitness` value).

Sites:
- `crates/worldwake-core/src/decision_event_payload.rs:741` (inline test)
- `crates/worldwake-core/src/decision_event_payload.rs:909` (inline test)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (repair memory completion payload)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (emit path + inline tests)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (repair-memory payload assertion)
- `crates/worldwake-ai/tests/scenarios/plan_repair.rs:272, :355, :734`
- `crates/worldwake-cli/src/bin/observer.rs:6732, :8665, :8745`
- `crates/worldwake-sim/src/save_load.rs:1292`

### 4. Bump save format

Update `crates/worldwake-sim/src/save_load.rs` `SAVE_FORMAT_VERSION` from 100 to 101 and refresh the focused version tests. No legacy decode path is added; old versioned saves remain rejected at the existing boundary.

### 5. Create `worldwake-ai/src/verification_provider/mod.rs`

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

archive/tickets/S169GENLAWVER-002.md finalized the exact `VerificationContext` field list once the seam-side data passed into the registry was known. This ticket landed the type with `actor`, `belief_view`, `effective_place` as the minimum required surface.

Add `pub mod verification_provider;` to `worldwake-ai/src/lib.rs`.

## Landed Files

- `crates/worldwake-core/src/decision_event_payload.rs` (modify — add enum + extend struct + tests)
- `crates/worldwake-core/src/lib.rs` (modify — re-export `VerificationProviderKind`)
- `crates/worldwake-ai/src/verification_provider/mod.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — add `pub mod verification_provider;`)
- `crates/worldwake-ai/tests/scenarios/plan_repair.rs` (modify — update 3 construction sites)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — constructor fallout)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — constructor fallout)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — constructor fallout)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — update 3 construction sites)
- `crates/worldwake-sim/src/save_load.rs` (modify — update 1 construction site + save version)

## Out of Scope

- `try_build_verification_candidate` registry function — archive/tickets/S169GENLAWVER-002.md.
- AskWitness / ConsultRecord / SearchPlace provider submodules and `try_build` implementations — S169GENLAWVER-002, -003, -004.
- Seam refactor at `agent_tick/execution.rs:452` — archive/tickets/S169GENLAWVER-002.md.
- `RepairAttemptTrace` extension with `verification_provider` / `verification_rejections` fields — archive/tickets/S169GENLAWVER-002.md.
- New goldens — S169GENLAWVER-003, -004, -005.

## Acceptance Result

### Verified Commands

1. `cargo test -p worldwake-core decision_event_payload` passed with current-format payload roundtrip coverage, all `VerificationProviderKind` variants, and derive-bound proof.
2. `cargo test -p worldwake-sim save_format_version` passed with `SAVE_FORMAT_VERSION = 101`, proving the existing old-version rejection boundary remains active.
3. `cargo check -p worldwake-ai` passed with the staged verification-provider module exported.
4. `cargo test --workspace` passed with all construction sites updated.

### Landed Invariants

1. `VerificationProviderKind` lives in `worldwake-core/src/decision_event_payload.rs`; no `Tag` mirror added (FND-28: single authoritative form).
2. `RepairAppliedPayload` has exactly one added field (`provider_kind`); no added `target` field landed (the existing `substitute_target` continues to carry the target entity).
3. All explicit construction sites set `provider_kind`; no site relies on `..Default::default()` spread syntax.
4. `worldwake-ai/src/verification_provider/mod.rs` has no provider submodules at the end of this ticket (mod-declaration only, populated by 002+).

## Test Plan Result

### Focused Tests

1. `crates/worldwake-core/src/decision_event_payload.rs` inline `#[cfg(test)]` — extended payload roundtrip coverage to include non-default `provider_kind`.
2. `crates/worldwake-core/src/decision_event_payload.rs` inline `#[cfg(test)]` — added `verification_provider_kind_derives_satisfy_event_payload_bounds` and `verification_provider_kind_variants_roundtrip_through_bincode`.
3. `crates/worldwake-sim/src/save_load.rs` inline tests — refreshed `SAVE_FORMAT_VERSION` assertions to 101.

### Commands Run

1. `cargo test -p worldwake-core decision_event_payload` — focused validation of payload roundtrips.
2. `cargo test -p worldwake-sim save_format_version` — focused validation of save-format version assertions.
3. `cargo check -p worldwake-ai` — verify new `verification_provider` module compiles.
4. `cargo test --workspace` — full suite to catch any missed construction sites.

Merge note: Ticket 001 introduced a serialization-format change (added field on `RepairAppliedPayload`) and bumped `SAVE_FORMAT_VERSION` from 100 to 101. Old versioned saves remain rejected by the existing save-version boundary; no compatibility shim was introduced.

## Outcome

Completed on 2026-05-25.

- Added `VerificationProviderKind` to `worldwake-core`, re-exported it, and persisted `provider_kind` on `RepairAppliedPayload`.
- Added the staged `worldwake-ai::verification_provider` module with `VerificationNeed`, `VerificationCandidate`, `VerificationTarget`, `VerificationRejection`, and `VerificationContext`.
- Updated all explicit `RepairAppliedPayload` construction sites, including live `agent_tick` constructor fallout that reassessment found beyond the drafted file list.
- Rendered `provider_kind` in observer decision history and updated the affected observer assertion.
- Bumped `worldwake-sim::SAVE_FORMAT_VERSION` from 100 to 101 so the new persisted causal-history field is a current-format change.

## Deviations

- Replaced the drafted legacy-byte serde-default claim with the FOUNDATIONS-aligned current-format path. Live focused proof showed bincode returns `UnexpectedEof` for missing trailing struct fields, and the repository already has `SAVE_FORMAT_VERSION`; old versioned saves remain rejected instead of decoded through a compatibility shim.
- Added constructor fallout in `crates/worldwake-ai/src/agent_tick/{mod.rs,execution.rs,tests.rs}` because the live shared payload surface was broader than the ticket's original constructor count.
- Waived `./scripts/verify.sh` for this ticket iteration because the `implement-spec-tickets` harness owns the final pre-push gate after the S169 family lands.

## Verification Result

- Passed `cargo test -p worldwake-core decision_event_payload`.
- Passed `cargo test -p worldwake-sim save_format_version`.
- Passed `cargo check -p worldwake-ai`.
- Passed `cargo test -p worldwake-cli --bin observer render_decision_history_section_covers_all_variants` after observer row-count fallout.
- Passed `cargo test --workspace`.
- Waived `./scripts/verify.sh` for this ticket iteration; the harness final branch phase still owns the full pre-PR verification gate before push.
