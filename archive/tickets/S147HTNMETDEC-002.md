# S147HTNMETDEC-002: Discrepancy::MethodFailure variant and exhaustive-match audit

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extends typed-discrepancy taxonomy with `Discrepancy::MethodFailure(MethodFailureContext)`; touches the workspace `Discrepancy`-match surface; bumps `SAVE_FORMAT_VERSION`.
**Deps**: `archive/tickets/S147HTNMETDEC-001.md` (MethodSchemaId)

## Problem

S147 D6 routes method-failure attribution through the existing typed-discrepancy chain (per FND-29A: causal history is authoritative). Without a new `Discrepancy::MethodFailure(MethodFailureContext)` variant, method failures would only surface in the optional `MethodPlanAttemptTrace` (introduced in ticket 009) and fail FND-29A's test — "later state changes can only be explained by reading ad hoc logs". The chosen mechanism (single new variant rather than payload-widening each existing variant or pure trace-only surfacing) was approved during reassessment via soft-criterion FND lensing — see `archive/specs/S147-htn-method-decomposition.md` D6 rationale.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `Discrepancy` enum lives in `crates/worldwake-core/src/discrepancy.rs`. All variants cited by S147 D6's mapping table were present before this ticket, while `MethodFailure` was absent. Workspace-wide direct `Discrepancy::` references were mostly construction sites in action handlers and effect sinks. The live explicit match-arm audit via `rg -n 'Discrepancy::[A-Za-z0-9_]+\s*=>' crates` identified result-affecting consumers in `crates/worldwake-ai/src/failure_handling.rs`, `crates/worldwake-ai/src/agenda_manager.rs`, `crates/worldwake-ai/src/agent_tick/mod.rs`, and `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs`.
2. `MethodSchemaId` exists after `archive/tickets/S147HTNMETDEC-001.md` lands at `crates/worldwake-core/src/method_schema_id.rs`. Before this ticket, `SAVE_FORMAT_VERSION` was `88` at `crates/worldwake-sim/src/save_load.rs`; this ticket bumped it to `89` for the new serialized `Discrepancy::MethodFailure` payload.
3. Shared boundary: the `Discrepancy` typed channel between authoritative action handlers (which construct `Discrepancy` values) and AI-side blocker memory / agenda manager / failure handler (which destructure `Discrepancy` values to drive backoff and replan decisions). This ticket extends both sides of the channel symmetrically.
4. Exact live test names were resolved with `cargo test -p worldwake-ai -- --list` before focused proof. The landed AI coverage extends `failure_handling::tests::discrepancy_ttl_uses_class_specific_defaults`, `failure_handling::tests::discrepancy_ttl_respects_profile_override`, and `agenda_manager::tests::classify_rejection_method_failure_uses_tick_elapsed_trigger`.
5. The audit scope was the workspace explicit match-arm audit listed as a D6 deliverable. Sites using `_ =>` catch-all did not require new arms, but `scenario_diagnostics::aggregator::normalize_discrepancy` required explicit normalization so metrics aggregate method failures by class rather than by payload identity.

## Architecture Check

1. A single new variant carrying a `Copy`-safe payload (`MethodFailureContext { method_id, kind, subgoal_index }`) keeps each existing Discrepancy variant single-meaning. Option (c) from the reassessment — widening each existing variant with optional method context — would have given each variant two meanings (method-driven vs not), violating FND-28's parallel-authority test for downstream blocker-memory consumers. Option (b) — trace-only — would have failed FND-29A's authoritative-history test. Option (a) (this choice) is the only one that passes all three lenses.
2. No backwards-compatibility shims. The variant is purely additive in the typed channel. `MethodFailureMode` (ai-side, defined in `archive/tickets/S147HTNMETDEC-004.md`) projects to `MethodFailureKind` (core-side, defined here) via a `From` impl that `archive/tickets/S147HTNMETDEC-004.md` owns — this ticket only declares the core-side surface.

## Verified Layers

1. New variant exists and is constructible → `discrepancy::tests::discrepancy_method_failure_roundtrips_through_bincode` constructs `Discrepancy::MethodFailure(MethodFailureContext { … })` and round-trips it through bincode.
2. Existing typed-discrepancy backoff/replan behavior remained covered → `cargo test -p worldwake-ai --lib failure_handling` passed after adding the method-failure TTL/clearing coverage.
3. Workspace exhaustive-match audit completed → `cargo build --workspace --all-targets` passed after the variant was added.
4. Save-format bump is consistent → `cargo test -p worldwake-sim --lib save_load` passed with version `89` writes and canonical rejection of version `88`.

## Landed Changes

### 1. Add `Discrepancy::MethodFailure(MethodFailureContext)` variant

`crates/worldwake-core/src/discrepancy.rs` now defines:

```rust
pub enum Discrepancy {
    // ... existing unit variants unchanged ...
    MethodFailure(MethodFailureContext),
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct MethodFailureContext {
    pub method_id: MethodSchemaId,
    pub kind: MethodFailureKind,
    pub subgoal_index: Option<u32>,
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum MethodFailureKind {
    PreconditionLost,
    SubgoalUnachievable,
    ArtifactNotProduced,
    ClaimDenied,
    Timeout,
}
```

### 2. Workspace exhaustive-match audit

The explicit match-arm audit added `Discrepancy::MethodFailure(_)` handling to:

- `crates/worldwake-ai/src/failure_handling.rs` (TTL/backoff match around line 1522)
- `crates/worldwake-ai/src/agenda_manager.rs` (rejection lifecycle match around lines 96+119)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (decisive evidence reference extraction)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (payload normalization for aggregate metrics)

For method failures:
- Backoff uses `CognitiveProfile::structural_block_ticks`, matching other structural typed discrepancies.
- Lifecycle classifies as `RejectionLifecycle::InfeasibleUntil` with the standard revival cooldown.
- Decision evidence extraction does not invent a belief or observation reference for method-internal failures.
- Scenario diagnostics normalize payload-bearing method failures to one aggregate class.

### 3. Bump `SAVE_FORMAT_VERSION` 88 → 89

`crates/worldwake-sim/src/save_load.rs` now records `SAVE_FORMAT_VERSION: 89` with an S147 D6 comment and updated version-mismatch coverage.

## Landed Files

- `crates/worldwake-core/src/discrepancy.rs` (modify — add variant + MethodFailureContext + MethodFailureKind)
- `crates/worldwake-core/src/lib.rs` (modify — re-export `MethodFailureContext` and `MethodFailureKind` beside `Discrepancy`)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump version)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — add `MethodFailure` arm to TTL/backoff match around line 1522)
- `crates/worldwake-ai/src/agenda_manager.rs` (modify — add `MethodFailure` arm to rejection lifecycle match around lines 96, 119)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — add `MethodFailure` to the no-decisive-evidence branch)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (modify — normalize method-failure payloads for metrics)
- `archive/tickets/S147HTNMETDEC-002.md` (modify — closeout truthing and archival)

Merge note: Ticket 002 bumps `SAVE_FORMAT_VERSION` 88→89 for the new `Discrepancy::MethodFailure` variant. Sibling ticket 003 (`disabled_methods` with `#[serde(default)]`) deliberately avoids an additional bump via an additive serde-default surface. Ticket 009's `method_trace: Option<…>` lands on the in-memory decision-trace model rather than the save-format surface, so it also does not require a save-version bump.

## Out of Scope

- The ai-side `MethodFailureMode` enum and its `From<&MethodFailureMode> for MethodFailureKind` impl — those landed in `archive/tickets/S147HTNMETDEC-004.md` (split — variant + audit in core (this ticket), `From` impl in ai (004)).
- Production sites that emit `Discrepancy::MethodFailure` — emission happens from the planner integration (ticket 008) and the method selector (ticket 007); this ticket only adds the variant.
- Per-method TTL/backoff tuning — moderate defaults are sufficient for first ship; per-method tuning is future work.

## Acceptance Result

### Passed Criteria

1. `Discrepancy::MethodFailure(MethodFailureContext { method_id: MethodSchemaId(7), kind: MethodFailureKind::SubgoalUnachievable, subgoal_index: Some(2) })` constructs and bincode-round-trips.
2. `failure_handling` and `agenda_manager` discrepancy-arm tests pass with the new arm added.
3. Save round-trip writes version 89 and version 88 is rejected with `SaveError::UnsupportedVersion`.
4. `cargo test -p worldwake-core` and `cargo test -p worldwake-ai` pass.
5. `cargo build --workspace --all-targets` succeeds.
6. `cargo clippy --workspace --all-targets -- -D warnings` passes.
7. `./scripts/verify.sh` passes.

### Invariants

1. Every workspace site that previously exhaustively matched `Discrepancy` continues to compile after the new variant is added.
2. `MethodFailureContext` and `MethodFailureKind` derive `Copy` — so the variant satisfies the existing `Copy` bound on `Discrepancy`.
3. `SAVE_FORMAT_VERSION` is bumped exactly once across the S147 ticket set — no sibling ticket bumps it again.
4. Each new match arm chooses a documented semantic default (not a placeholder `_ =>` swallowing the new variant when the surrounding match was previously exhaustive).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/discrepancy.rs` inline tests — add construction and serde round-trip cases for `MethodFailure`.
2. `crates/worldwake-ai/src/failure_handling.rs` inline tests — extend the existing TTL/backoff test to cover the new arm.
3. `crates/worldwake-ai/src/agenda_manager.rs` inline tests — extend the existing rejection-lifecycle test to cover the new arm.
4. `crates/worldwake-sim/src/save_load.rs` inline tests — update version assertions and pre-S147 version rejection to version 88.

## Outcome

Completed on 2026-05-17.

- Added `Discrepancy::MethodFailure(MethodFailureContext)` plus `MethodFailureKind` in core, and re-exported the new payload types at the existing `worldwake_core::*` consumer boundary.
- Bumped `SAVE_FORMAT_VERSION` from 88 to 89 for the new serialized discrepancy payload.
- Extended AI discrepancy consumers for TTL/backoff, agenda rejection lifecycle, decisive-evidence extraction, and scenario-diagnostics normalization.
- Kept production emission of `Discrepancy::MethodFailure` out of scope for later S147 planner/method-selector tickets.

## Deviations

- The live explicit match-arm audit found four affected AI consumer surfaces, not only the initially listed `failure_handling` and `agenda_manager` sites. `agent_tick/mod.rs` and `scenario_diagnostics/aggregator.rs` were included as required shared-channel fallout.
- Method-failure backoff uses the existing structural-block TTL rather than a new method-specific tuning knob. Per-method tuning remains out of scope.

## Verification Result

- Passed `cargo test -p worldwake-ai -- --list` for selector discovery.
- Passed `cargo test -p worldwake-core --lib discrepancy`.
- Passed `cargo test -p worldwake-ai --lib failure_handling`.
- Passed `cargo test -p worldwake-ai --lib agenda_manager`.
- Passed `cargo test -p worldwake-sim --lib save_load`.
- Passed `cargo build --workspace --all-targets`.
- Passed `cargo test -p worldwake-core`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `./scripts/verify.sh`.
