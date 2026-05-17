# S147HTNMETDEC-002: Discrepancy::MethodFailure variant and exhaustive-match audit

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extends typed-discrepancy taxonomy with `Discrepancy::MethodFailure(MethodFailureContext)`; touches the workspace `Discrepancy`-match surface; bumps `SAVE_FORMAT_VERSION`.
**Deps**: 001 (MethodSchemaId)

## Problem

S147 D6 routes method-failure attribution through the existing typed-discrepancy chain (per FND-29A: causal history is authoritative). Without a new `Discrepancy::MethodFailure(MethodFailureContext)` variant, method failures would only surface in the optional `MethodPlanAttemptTrace` (introduced in ticket 009) and fail FND-29A's test — "later state changes can only be explained by reading ad hoc logs". The chosen mechanism (single new variant rather than payload-widening each existing variant or pure trace-only surfacing) was approved during reassessment via soft-criterion FND lensing — see `specs/S147-htn-method-decomposition.md` D6 rationale.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `Discrepancy` enum lives at `crates/worldwake-core/src/discrepancy.rs:9`. All variants currently cited by S147 D6's mapping table are **unit variants** with no payload: `BeliefStale:11`, `BeliefContradicted:13`, `NoLegalBinding:21`, `SearchBudgetExhausted:27`, `PartialExecutionDrift:29`. Workspace-wide use sites: 474 total `Discrepancy::` references across ~50 files (most are construction sites in action handlers and effect sinks). Genuine destructuring match arms: 16 sites (across `failure_handling.rs`, `agenda_manager.rs`, and others). Existing focused coverage in `crates/worldwake-ai/src/failure_handling.rs` tests exercises every existing variant's TTL/backoff arm at `failure_handling.rs:1522-1534`; this is the most-likely-to-break match.
2. `MethodSchemaId` exists after ticket 001 lands at `crates/worldwake-core/src/method_schema_id.rs`. `SAVE_FORMAT_VERSION` is currently `88` at `crates/worldwake-sim/src/save_load.rs:6`. Adding a new variant to a serde-derived enum is structurally backwards-compatible for *reading* old saves (the new variant simply never appears in pre-bump byte streams), but forward-incompatible: new saves with the new variant fail to load on old code. The project bumps `SAVE_FORMAT_VERSION` on schema additions as a forward-compat signal.
3. Shared boundary: the `Discrepancy` typed channel between authoritative action handlers (which construct `Discrepancy` values) and AI-side blocker memory / agenda manager / failure handler (which destructure `Discrepancy` values to drive backoff and replan decisions). This ticket extends both sides of the channel symmetrically.
4. Existing tests exercising `Discrepancy` match arms (named per Step 2 spot-check (f)): `failure_handling.rs::discrepancy_ttl_resolves_per_variant` (anchor — verify exact name on implementation), the inline tests around `failure_handling.rs:1522-1534`, and `agenda_manager.rs:96+119` arms within `cognitive_reaction_for_discrepancy`. Verify exact test names during implementation via `cargo test -p worldwake-ai -- --list | grep -i discrepancy`.
5. The audit scope is the workspace exhaustive-match audit listed as a D6 deliverable: grep `match` sites that destructure `Discrepancy` (16 candidates) and add a `Discrepancy::MethodFailure(_) => …` arm to each genuinely-exhaustive match. Sites using `_ =>` catch-all do not require a new arm but should be reviewed for whether method-failure context is meaningfully different from the catch-all default.

## Architecture Check

1. A single new variant carrying a `Copy`-safe payload (`MethodFailureContext { method_id, kind, subgoal_index }`) keeps each existing Discrepancy variant single-meaning. Option (c) from the reassessment — widening each existing variant with optional method context — would have given each variant two meanings (method-driven vs not), violating FND-28's parallel-authority test for downstream blocker-memory consumers. Option (b) — trace-only — would have failed FND-29A's authoritative-history test. Option (a) (this choice) is the only one that passes all three lenses.
2. No backwards-compatibility shims. The variant is purely additive in the typed channel. `MethodFailureMode` (ai-side, defined in ticket 004) projects to `MethodFailureKind` (core-side, defined here) via a `From` impl that ticket 004 owns — this ticket only declares the core-side surface.

## Verification Layers

1. New variant exists and is constructible → focused unit test in `discrepancy.rs` tests asserting `Discrepancy::MethodFailure(MethodFailureContext { … })` constructs and round-trips through serde.
2. Existing typed-discrepancy backoff/replan behavior unchanged for all pre-existing variants → existing `failure_handling.rs` tests pass without modification beyond the new arm addition.
3. Workspace exhaustive-match audit complete → `cargo build --workspace --all-targets` succeeds after the variant is added (compile-time check that all exhaustive matches handle the new variant).
4. Save-format bump is consistent → load a save written under version 88 and assert the format-version error is the canonical mismatch (not a silent deserialize failure).

## What to Change

### 1. Add `Discrepancy::MethodFailure(MethodFailureContext)` variant

Modify `crates/worldwake-core/src/discrepancy.rs`:

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

Grep `match` sites that destructure `Discrepancy` and add a `Discrepancy::MethodFailure(_) => …` arm to each. Initial target list (verify during implementation):

- `crates/worldwake-ai/src/failure_handling.rs` (TTL/backoff match around line 1522)
- `crates/worldwake-ai/src/agenda_manager.rs` (rejection lifecycle match around lines 96+119)
- Any other site `rg "Discrepancy::\w+\s*=>" crates/` surfaces

For each new arm, choose the semantic default for method failures:
- Backoff: use the median of existing backoff arms (configurable later if method-specific tuning emerges).
- Lifecycle: classify as `RejectionLifecycle::InfeasibleUntil` with a moderate backoff, mirroring `RouteUnknown`.

### 3. Bump `SAVE_FORMAT_VERSION` 88 → 89

Modify `crates/worldwake-sim/src/save_load.rs:6` to bump the constant. Add a brief comment naming S147 D6 as the cause.

## Files to Touch

- `crates/worldwake-core/src/discrepancy.rs` (modify — add variant + MethodFailureContext + MethodFailureKind)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump version)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — add `MethodFailure` arm to TTL/backoff match around line 1522)
- `crates/worldwake-ai/src/agenda_manager.rs` (modify — add `MethodFailure` arm to rejection lifecycle match around lines 96, 119)
- Additional sites: `To be confirmed:` — run `rg -n "Discrepancy::\w+\s*=>" crates/` during the assumption-reassessment phase and enumerate every site needing a new arm. The 16-site count surfaces all destructuring matches; the `Files to Touch` list should grow to enumerate each before edits begin.

Merge note: Ticket 002 bumps `SAVE_FORMAT_VERSION` 88→89 for the new `Discrepancy::MethodFailure` variant. Sibling tickets 003 (`disabled_methods` with `#[serde(default)]`) and 009 (`method_trace: Option<…>`) deliberately avoid additional bumps via additive serde-default surfaces.

## Out of Scope

- The ai-side `MethodFailureMode` enum and its `From<&MethodFailureMode> for MethodFailureKind` impl — those land in ticket 004 (split — variant + audit in core (this ticket), `From` impl in ai (004)).
- Production sites that emit `Discrepancy::MethodFailure` — emission happens from the planner integration (ticket 008) and the method selector (ticket 007); this ticket only adds the variant.
- Per-method TTL/backoff tuning — moderate defaults are sufficient for first ship; per-method tuning is future work.

## Acceptance Criteria

### Tests That Must Pass

1. New focused test in `discrepancy.rs` tests: `MethodFailure(MethodFailureContext { method_id: MethodSchemaId(7), kind: MethodFailureKind::SubgoalUnachievable, subgoal_index: Some(2) })` constructs and serde-round-trips.
2. Existing `failure_handling.rs` and `agenda_manager.rs` discrepancy-arm tests pass with the new arm added.
3. Save round-trip: a state saved under version 89 loads under version 89; a state saved under version 88 fails to load with the canonical version-mismatch error.
4. Existing suite: `cargo test -p worldwake-core` and `cargo test -p worldwake-ai` pass.
5. `cargo build --workspace --all-targets` succeeds (proves exhaustive-match audit is complete).
6. `cargo clippy --workspace --all-targets -- -D warnings` clean.

### Invariants

1. Every workspace site that previously exhaustively matched `Discrepancy` continues to compile after the new variant is added.
2. `MethodFailureContext` and `MethodFailureKind` derive `Copy` — so the variant satisfies the existing `Copy` bound on `Discrepancy`.
3. `SAVE_FORMAT_VERSION` is bumped exactly once across the S147 ticket set — no sibling ticket bumps it again.
4. Each new match arm chooses a documented semantic default (not a placeholder `_ =>` swallowing the new variant when the surrounding match was previously exhaustive).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/discrepancy.rs` inline tests — add construction and serde round-trip cases for `MethodFailure`.
2. `crates/worldwake-ai/src/failure_handling.rs` inline tests — extend the existing TTL/backoff test to cover the new arm.
3. `crates/worldwake-ai/src/agenda_manager.rs` inline tests — extend the existing rejection-lifecycle test to cover the new arm.

### Commands

1. `cargo test -p worldwake-core --lib discrepancy`
2. `cargo test -p worldwake-ai --lib failure_handling`
3. `cargo test -p worldwake-ai --lib agenda_manager`
4. `cargo build --workspace --all-targets`
5. `./scripts/verify.sh`
