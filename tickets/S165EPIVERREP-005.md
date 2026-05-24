# S165EPIVERREP-005: RepairAttemptTrace verification anchor

**Status**: PENDING
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

1. `RepairAttemptTrace` is defined at `crates/worldwake-ai/src/decision_trace.rs:197` with
   fields `breach`, `chosen_kind`, `rejected`, `budget_consumed`, `budget_total`. It is the
   AI-crate diagnostic trace, not authoritative saved state (the authoritative surface is
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

## Verification Layers

1. A successful verification repair produces a `RepairAttemptTrace` whose anchor field
   carries the chosen witness → decision-trace assertion (focused test).
2. A rejected verification (no lawful witness) records the missing-affordance cause /
   `NoEpistemicSubstrate` distinct from a bare placeholder → decision-trace assertion.
3. Observer renders the anchor in the existing repair-attempt summary → headless render
   smoke check (observer test surface).

## What to Change

### 1. Extend `RepairAttemptTrace`

Add a verification anchor field (e.g. `verification_anchor: Option<EntityId>`) to
`RepairAttemptTrace` (`decision_trace.rs:197`). Populate it at the runtime emission sites
(`execution.rs:144`, `:585`) from the chosen verification candidate; default `None` at the
observer and test/aggregator construction sites.

### 2. Observer rendering

Render the witness anchor in the existing repair-attempt summary path in
`crates/worldwake-cli/src/bin/observer.rs` (no new section).

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (modify — construction site)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — construction site + rendering)

## Out of Scope

- The authoritative `RepairApplied` anchor (ticket 003 — the provenance of record).
- Any `scenario_diagnostics` aggregate metric beyond carrying the new field through its
  helper construction site.

## Acceptance Criteria

### Tests That Must Pass

1. New: verification-repair `RepairAttemptTrace` carries the witness anchor.
2. New: rejected-verification trace records the missing-affordance cause.
3. Existing suite: `cargo test -p worldwake-ai decision_trace` and
   `cargo test -p worldwake-ai scenario_diagnostics`.

### Invariants

1. `RepairAttemptTrace` remains a derived diagnostic view, not authoritative state
   (FND-27); no `SAVE_FORMAT_VERSION` change.
2. All construction sites compile with the new field; non-verification traces default to
   `None`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` (inline `#[cfg(test)]`) — anchor populated /
   `None` cases.
2. `crates/worldwake-cli/src/bin/observer.rs` (or its test surface) — render smoke check.

### Commands

1. `cargo test -p worldwake-ai decision_trace`
2. `cargo test -p worldwake-cli`
3. `./scripts/verify.sh`
