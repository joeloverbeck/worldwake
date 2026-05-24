# S165EPIVERREP-005: RepairAttemptTrace verification anchor

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` decision trace (`decision_trace.rs`), `worldwake-cli` observer
**Deps**: archive/tickets/S165EPIVERREP-003.md

## Problem

The authoritative `RepairApplied` event records the verification witness via
`substitute_target` (ticket 003), but the richer diagnostic surface
(`RepairAttemptTrace`, consumed by `scenario_diagnostics` and the observer) carries no
verification-specific detail. To answer "why did the agent ask this witness to repair
this goal?" during debugging (FND-29), the trace needs the chosen witness anchor and, on
rejection, the missing-affordance cause.

## Assumption Reassessment (2026-05-22)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Before this ticket, `RepairAttemptTrace` was defined at
   `crates/worldwake-ai/src/decision_trace.rs` with fields `breach`, `chosen_kind`,
   `rejected`, `budget_consumed`, `budget_total`. It is the AI-crate diagnostic trace,
   not authoritative saved state (the authoritative surface is
   `RepairApplied(RepairAppliedPayload)`), so adding fields does **not** bump
   `SAVE_FORMAT_VERSION` (confirmed: `SAVE_FORMAT_VERSION = 100`,
   `crates/worldwake-sim/src/save_load.rs:7`).
2. Spec deliverable D7 (`specs/S165-epistemic-verification-repair.md`).
3. Construction sites (struct-literal grep) — all must supply the new field: runtime
   emission at `crates/worldwake-ai/src/agent_tick/execution.rs:144` and `:585`; observer
   at `crates/worldwake-cli/src/bin/observer.rs:8640`; test/aggregator helpers at
   `crates/worldwake-ai/src/decision_trace.rs:2885` and
   `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs:1375-1376`. This is a
   cross-crate field addition (`worldwake-ai` + `worldwake-cli`).
4. Information-path note (precision rule 16): the witness anchor now has two lawful
   surfaces — the authoritative `RepairApplied` event (canonical, ticket 003) and this
   diagnostic trace (debug/observer convenience). The trace is **not** a competing
   authority; it is a derived view. The canonical provenance path remains the event log;
   this ticket does not weaken it.

## Architecture Check

1. Adding the anchor as `Option<EntityId>` (default `None` at non-verification sites)
   keeps every construction site valid with a one-field change and makes the trace
   self-describing for verification repairs without inflating non-verification traces.
2. The trace stays a derived diagnostic view over the authoritative event (FND-27) — no
   second source of truth for the anchor.

## Verified Layers

1. Successful verification repair trace anchor -> `agent_tick::execution::tests::insert_verification_repair_trace_records_witness_anchor`.
2. Rejected verification cause remains explicit through `rejected` entries -> `agent_tick::execution::tests::failed_local_repair_attempt_trace_records_budget_and_rejections`.
3. Observer renders the trace anchor in the existing repair summary -> `tests::render_insert_verification_repair_with_trace_anchor`.

## Landed Changes

### 1. Extend `RepairAttemptTrace`

Added `verification_anchor: Option<EntityId>` to `RepairAttemptTrace`. Successful
`InsertVerification` repair traces populate it from the selected repaired plan step;
failed traces and non-verification helper construction sites use `None`.

### 2. Observer rendering

Rendered `verification_anchor` in the existing `RepairApplied` detail block when the
matched trace is an `InsertVerification` attempt or carries an anchor.

### 3. Spec truth-sync

Updated S165 D7 so the active spec describes the landed trace field rather than the
pre-ticket field set.

## Landed Files

- `crates/worldwake-ai/src/decision_trace.rs` (modified)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modified)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (modified)
- `crates/worldwake-cli/src/bin/observer.rs` (modified)
- `specs/S165-epistemic-verification-repair.md` (modified)

## Out of Scope

- The authoritative `RepairApplied` anchor (ticket 003 — the provenance of record).
- Any `scenario_diagnostics` aggregate metric beyond carrying the new field through its
  helper construction site.

## Acceptance Result

### Proof

1. Passed: verification-repair `RepairAttemptTrace` carries the witness anchor.
2. Passed: rejected-verification trace records the missing-affordance cause.
3. Passed: `cargo test -p worldwake-ai decision_trace` and
   `cargo test -p worldwake-ai scenario_diagnostics`.

### Invariants

1. `RepairAttemptTrace` remains a derived diagnostic view, not authoritative state
   (FND-27); no `SAVE_FORMAT_VERSION` change landed.
2. All construction sites compile with the field; non-verification traces default to
   `None`.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/execution.rs` — added focused anchor extraction coverage and extended failed-trace assertions.
2. `crates/worldwake-ai/src/decision_trace.rs` — updated bincode roundtrip fixture for the new field.
3. `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` — updated helper construction site.
4. `crates/worldwake-cli/src/bin/observer.rs` — added render coverage for the verification anchor line.

## Outcome

Completed on 2026-05-24.

- Added `RepairAttemptTrace.verification_anchor` as a diagnostic-only witness anchor for `InsertVerification`.
- Populated the anchor from the repaired plan at the revalidation seam and left failed/non-verification traces as `None`.
- Rendered the anchor in observer `RepairApplied` details while keeping `RepairApplied.substitute_target` as the authoritative provenance path.
- Truth-synced S165 D7 to the landed diagnostic trace shape.

## Deviations

- The ticket's drafted inline decision-trace test location was replaced with the stronger live seam in `agent_tick::execution`, where the repaired plan step and selected repair kind are both available.
- `./scripts/verify.sh` was not run for this per-ticket iteration; the harness reserves that full pre-PR gate for final spec-family completion before push. The affected `worldwake-ai` and `worldwake-cli` crate suites passed.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib agent_tick::execution::tests::insert_verification_repair_trace_records_witness_anchor -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::execution::tests::failed_local_repair_attempt_trace_records_budget_and_rejections -- --exact`
- Passed `cargo test -p worldwake-cli --bin observer tests::render_insert_verification_repair_with_trace_anchor -- --exact`
- Passed `cargo test -p worldwake-ai decision_trace`
- Passed `cargo test -p worldwake-ai scenario_diagnostics`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo test -p worldwake-ai`
- Waived `./scripts/verify.sh` for this ticket iteration because final harness completion runs the full pre-PR gate before pushing.
