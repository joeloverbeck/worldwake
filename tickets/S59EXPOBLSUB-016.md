# S59EXPOBLSUB-016: institutional missing/found-person report carriers

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — shared institutional claim/record shape for missing/found-person reporting
**Deps**: S59EXPOBLSUB-013

## Problem

After `S59EXPOBLSUB-013` lands the first lawful direct-agent `report_found` propagation slice, the institutional/office reporting half of the S59 roadmap remains unowned. The live branch still has no lawful institutional claim type for missing-person or found-person reports, so office-facing propagation would otherwise remain implicit cleanup.

## Assumption Reassessment (2026-04-07)

1. `InstitutionalClaim` in `crates/worldwake-core/src/institutional.rs` currently supports office, faction, force-control, accusation, and verdict records only; there is no missing/found-person claim shape.
2. `RecordKind` currently includes `OfficeRegister`, `FactionRoster`, `SupportLedger`, and `CrimeRegister`; no record kind is dedicated to missing/found-person reporting.
3. `S59EXPOBLSUB-013` is being narrowed to the first lawful current-branch `report_found` slice backed by resolved `ExpectationStore` outcomes plus `LastSeenMemory`, without office-record propagation.
4. Existing office/institutional read paths (`consult_record`, institutional belief projection, justice record handling) are the exact shared abstraction boundary under audit for any future office-facing missing/found-person report carrier.
5. This ticket should not be implemented until the direct-agent `report_found` slice lands, because that slice establishes the canonical non-institutional propagation path the institutional branch must not duplicate.

## Architecture Check

1. Separating institutional record design from the first runtime `report_found` slice keeps the current branch on one canonical direct report path instead of forcing speculative shared-record architecture into an otherwise local action ticket.
2. Any future office-facing propagation must introduce one lawful institutional carrier, not parallel ad hoc writes into `ViolationMemory`, `ExpectationStore`, and record tables.

## Verification Layers

1. Missing/found-person office report admission -> focused runtime/action test at the final action boundary
2. Institutional record mutation -> authoritative `RecordData` world-state proof
3. Consultation/projection fallout -> focused institutional belief / consult-record proof

## What to Change

### 1. Add a lawful institutional carrier

- Introduce the minimum shared claim/record shape needed for office-facing missing/found-person reporting
- Choose the canonical record home and consultation semantics

### 2. Wire office-facing report propagation

- Extend the appropriate report action surface to write and read that institutional carrier
- Keep direct-agent `report_found` propagation and office-facing propagation on one non-duplicated authority path

## Files to Touch

- Reassess from `crates/worldwake-core/src/institutional.rs`, `crates/worldwake-systems/src/report_actions.rs`, and any directly required consult/projection files once `S59EXPOBLSUB-013` is complete.

## Out of Scope

- Direct colocated agent `report_found` propagation backed by resolved expectations — owned by `S59EXPOBLSUB-013`
- New AI goal-family work unless reassessment shows the institutional carrier must become planner-visible in the same slice

## Acceptance Criteria

### Tests That Must Pass

1. Office-facing missing/found-person reporting writes one canonical institutional record path
2. Consultation/projection reads that record through the existing institutional substrate
3. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. No duplicate authority path for the same office-facing missing/found-person fact
2. No omniscient global registry for missing/found persons

## Test Plan

### New/Modified Tests

1. Focused runtime and institutional-record tests at the final shared carrier boundary chosen during implementation

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
