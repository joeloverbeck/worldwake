# S173SELCARINT-002: `ActionTraceDetail::SelfCareInterrupted` variant

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new variant on `ActionTraceDetail` (trace-sink type; not serialized)
**Deps**: `archive/tickets/S173SELCARINT-001.md` (uses `SelfCareUseKind`), `archive/specs/S173-self-care-interruption-occupancy.md` (D7)

## Problem

Before this ticket, an interrupted self-care action fired `EventTag::ActionAborted` (engine-level, already authoritative) with an unstructured `ActionTraceKind::Aborted { instance_id, reason: String }` payload (`crates/worldwake-sim/src/action_trace.rs:65-86`). "Why didn't this agent wash?" could not be answered from typed evidence — only from the freeform string. The completed change adds the typed `ActionTraceDetail::SelfCareInterrupted { kind, basin }` variant so downstream tickets can populate structured "which use kind, which facility/place" payload alongside the existing authoritative `EventTag::ActionAborted` record. The live population boundary is `tick_step.rs::abort_trace_detail_for_instance` after `archive/tickets/S173SELCARINT-004.md` and `archive/tickets/S173SELCARINT-005.md`, not handler-local trace mutation.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `ActionTraceDetail` was defined at `crates/worldwake-sim/src/action_trace.rs:32-63` with 8 existing variants (`Tell`, `Investigate`, `AskWitness`, `AskAboutPerson`, `SearchPlace`, `ReportMissing`, `ReportFound`, `EscortToSafety`). The original reassessment correctly found no cross-crate exhaustive observer/decision-trace consumers, but focused compile exposed one local exhaustive helper: `ActionTraceDetail::summary()` in the same file. The implementation updated that local summary arm as same-module trace-surface fallout.
2. `ActionTraceDetail::from_payload` (`action_trace.rs:740+`) returns `None` for `ActionPayload::None`. Self-care actions all register with `ActionPayload::None` (`crates/worldwake-systems/src/needs_actions.rs` registration block at L23-58). The new variant is therefore populated by the abort handlers directly (setting `ActionTraceEvent.detail` at emission time), not auto-derived through `from_payload`. No change to `from_payload` is required in this ticket.
3. Shared abstraction boundary: the typed action-trace sink (`ActionTraceSink`), consumed by goldens, the observer binary, and decision traces. `EventTag::ActionAborted` (already firing) remains the authoritative causal anchor (FND-29A); this variant is the enriching trace-detail payload (FND-29 debuggability), not a new authoritative surface.
4. Save format / serialization: `ActionTraceDetail` is **not** serialized to save state — it lives in the trace-sink layer (per-tick borrowed `&mut ActionTraceSink`, not part of `SimulationState`). No `SAVE_FORMAT_VERSION` bump applies to this ticket; the bump landed in 001. Verified by inspecting `crates/worldwake-sim/src/save_load.rs` — no reference to `ActionTraceDetail` in the save path.
5. `SelfCareUseKind` (defined in ticket 001) derives `Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize` — sufficient for inclusion as a field on `ActionTraceDetail` (which derives `Clone, Debug, Eq, PartialEq` per its existing definition).
6. `EntityId` is `Copy + Eq + Hash` (`crates/worldwake-core/src/ids.rs:44`). `Option<EntityId>` payload is compatible with the parent enum's derives.

## Architecture Check

1. Single typed trace surface: `EventTag::ActionAborted` (existing, authoritative) for the causal record; `ActionTraceDetail::SelfCareInterrupted` (new) for the structured payload. Per FND-28 this avoids a parallel `EventTag::SelfCareInterrupted` variant for an event already covered by `ActionAborted`. The structured payload lives where typed trace-sink discrimination already lives (`ActionTraceDetail`), keeping the event log free of redundant variants.
2. Variant placement on `ActionTraceDetail` (rather than a sibling enum or a `Discrepancy` variant) follows the "Discrepancy as Failure-Attribution Surface" option (2) trace-only pattern — surfaces the failure-mode cause for debug/observer inspection without extending the typed failure taxonomy (`Discrepancy`) for a case that doesn't alter handler control flow.
3. No backwards-compatibility aliasing — the variant is additive; no existing variant is renamed or removed.

## Verification Mapping

1. Variant existence and shape → focused unit test: construct an `ActionTraceDetail::SelfCareInterrupted { kind: SelfCareUseKind::Wash, basin: Some(EntityId { slot: 1, generation: 0 }) }`, assert derive surface (`Clone`, `Debug`, `PartialEq`) works.
2. Single-layer ticket (variant definition only): downstream behavior (abort-handler population, observer consumption) is proven by tickets 004, 005, and 007's goldens. This ticket's contract is solely that the variant exists with the correct shape.

## Landed Changes

### 1. Add `SelfCareInterrupted` variant to `ActionTraceDetail`

In `crates/worldwake-sim/src/action_trace.rs`, extend the `ActionTraceDetail` enum (current definition at L32-63):

```rust
pub enum ActionTraceDetail {
    Tell { … },
    Investigate { … },
    AskWitness { … },
    AskAboutPerson { … },
    SearchPlace { … },
    ReportMissing { … },
    ReportFound { … },
    EscortToSafety { … },
    /// Self-care action was interrupted before commit. Fires alongside the
    /// existing `EventTag::ActionAborted` engine-level record; this variant
    /// carries the structured "which use kind, which basin/latrine" payload
    /// that the generic `ActionTraceKind::Aborted { reason: String }` does not
    /// type. `basin` is `Some(entity)` for `Wash` and `LatrineRelief` (the
    /// occupancy-bearing kinds); `None` for `Eat`, `Drink`, and
    /// `WildernessRelief` (atomic actions with no facility target).
    SelfCareInterrupted {
        kind: worldwake_core::SelfCareUseKind,
        basin: Option<worldwake_core::EntityId>,
    },
}
```

Path note: import `SelfCareUseKind` from `worldwake_core` (already a dependency of `worldwake-sim`).

### 2. No change to `from_payload`

`ActionTraceDetail::from_payload(&ActionPayload::None)` continues to return `None`. Downstream tickets (004, 005) set `ActionTraceEvent.detail` through `tick_step.rs::abort_trace_detail_for_instance` at emission time rather than routing through `from_payload`. This preserves the existing `from_payload` contract (payload-derived auto-discrimination for non-`None` payloads).

## Files Touched

- `crates/worldwake-sim/src/action_trace.rs` (modify — add variant to `ActionTraceDetail` enum; possibly extend the existing inline `#[cfg(test)]` block at L557+ with one round-trip / equality test for the new variant)

## Out of Scope

- Populating the variant from abort handlers — owned by tickets 004 (wash/toilet) and 005 (eat/drink/wilderness/sleep).
- Extending `ActionTraceDetail::from_payload` — `from_payload` continues to return `None` for `ActionPayload::None`; abort-side population is the explicit path.
- Adding a new `EventTag` variant — per spec D7 and FND-28, the existing `EventTag::ActionAborted` is reused as the authoritative causal anchor.
- Observer-side rendering of the new variant — `worldwake-cli/src/bin/observer.rs` consumes `ActionTraceDetail` via specific-variant patterns; absence of an observer match arm for `SelfCareInterrupted` is acceptable (the variant simply won't render in observer dumps until ticket 007 or a follow-up extends it). No silent observer breakage results.

## Acceptance Criteria

### Required Proof

1. Added unit test: `self_care_interrupted_variant_constructs_and_derives` — basic construction in inline `#[cfg(test)]` block in `action_trace.rs`.
2. Existing suite: `cargo test -p worldwake-sim action_trace`.
3. Workspace builds and existing scenarios pass: `cargo test --workspace`.

### Invariants

1. The variant's payload types (`SelfCareUseKind`, `Option<EntityId>`) satisfy `ActionTraceDetail`'s derives (`Clone`, `Debug`, `Eq`, `PartialEq`).
2. No existing `if let Some(ActionTraceDetail::X { … })` consumer match site is broken — verified by full workspace test pass.

## Verification Result

### New/Modified Test Coverage

1. Passed `crates/worldwake-sim/src/action_trace.rs` inline test `self_care_interrupted_variant_constructs_and_derives` — constructs, clones, compares, and debug-formats the new detail variant.

### Commands Run

1. Passed `cargo test -p worldwake-sim action_trace` — 27 action-trace/tick-step trace tests passed, including `self_care_interrupted_variant_constructs_and_derives`.
2. Passed `cargo test --workspace` — full workspace test suite passed.
3. Passed `./scripts/verify.sh` — ran `cargo fmt --all -- --check`, `cargo test --workspace`, active-goal/artifact/debug-view checks, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.

## Outcome

Completed: 2026-05-26

The ticket added `ActionTraceDetail::SelfCareInterrupted { kind: SelfCareUseKind, basin: Option<EntityId> }` to `crates/worldwake-sim/src/action_trace.rs`, imported `SelfCareUseKind`, and added the focused construction/derive unit test. The implementation also updated the local `ActionTraceDetail::summary()` helper after focused compile showed it was an exhaustive same-module consumer.

Deviations from plan: the ticket's original reassessment said no exhaustive consumer match arms required updating. That was too broad; the local summary helper did require an arm. No cross-crate observer rendering was added, `ActionTraceDetail::from_payload(ActionPayload::None)` remains unchanged, and no new `EventTag` variant or save-format change was introduced.

Verification: passed `cargo test -p worldwake-sim action_trace`, `cargo test --workspace`, and `./scripts/verify.sh`.
