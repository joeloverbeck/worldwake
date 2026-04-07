# S59EXPOBLSUB-016: institutional missing/found-person report carriers

**Status**: COMPLETED
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
6. Live correction: `consult_record` in `crates/worldwake-systems/src/consult_record_actions.rs` already projects arbitrary `InstitutionalClaim` values through `InstitutionalBeliefKey` lanes. The missing slice is the claim/key family plus the report-action writes, not a new consultation action.
7. Live correction: `OfficeRegister` is the clean current-branch canonical home for office-facing missing/found-person records. Reusing it is narrower and cleaner than introducing a new `RecordKind` before there is any separate consultation or ownership behavior that would justify a dedicated record family.
8. Live correction: `report_missing` already binds the actor's current place and can append to a colocated office register without widening its target model. `report_found` currently targets only co-located agents in `crates/worldwake-systems/src/report_actions.rs`; office-facing found reports therefore require widening that action to support a local office-register target in addition to the already-landed direct-agent branch.
9. Live correction: the new claim family will widen exhaustive institutional mappings in `crates/worldwake-core/src/belief.rs`, `crates/worldwake-sim/src/institutional_knowledge_trace.rs`, `crates/worldwake-systems/src/consult_record_actions.rs`, `crates/worldwake-systems/src/tell_actions.rs`, and `crates/worldwake-systems/src/perception.rs`. Those consumers can stay compile-safe and substrate-only; no new planner-visible behavior is implied by this ticket.

## Architecture Check

1. Reusing `OfficeRegister` keeps office-facing missing/found-person reporting on one existing inspectable institutional path instead of adding a new record family without distinct semantics.
2. Extending the existing report actions is cleaner than inventing a second office-report action family: `report_missing` remains place-bound, and `report_found` gains a bounded record-target branch while preserving the already-landed direct-agent branch.
3. The office-facing carrier must remain institutional-record state, not parallel ad hoc writes into `ViolationMemory`, `ExpectationStore`, or bespoke world metadata.

## Verification Layers

1. Office-facing `report_missing` / record-target `report_found` admission -> focused runtime/action tests at the final action boundaries
2. Missing/found-person claim writes supersede one canonical `OfficeRegister` lane per subject -> authoritative `RecordData` world-state proof
3. `consult_record` projects the new claim family through the existing institutional substrate -> focused institutional belief / consult-record proof

## What to Change

### 1. Add a lawful institutional carrier

- Introduce the minimum shared `InstitutionalClaim` / `InstitutionalBeliefKey` shape needed for office-facing missing/found-person reporting
- Reuse `OfficeRegister` as the canonical record home and wire the new claim family through existing consultation / institutional-belief projection

### 2. Wire office-facing report propagation

- Extend `report_missing` to append or supersede the local office-register claim for the missing subject when a unique `OfficeRegister` exists at the actor's place
- Extend `report_found` with a bounded office-register target path while preserving the already-landed direct-agent propagation branch
- Keep direct-agent `report_found` propagation and office-facing propagation on one non-duplicated authority path

## Files to Touch

- `crates/worldwake-core/src/institutional.rs` (modify — add missing/found-person institutional claim and belief-key shapes)
- `crates/worldwake-core/src/belief.rs` (modify — extend institutional memory-lane and topic-key handling)
- `crates/worldwake-systems/src/report_actions.rs` (modify — write office-register claims and widen `report_found` to support a record target path)
- `crates/worldwake-systems/src/consult_record_actions.rs` (modify — project the new claim family through `consult_record`)
- `crates/worldwake-systems/src/tell_actions.rs` (modify — extend institutional claim-key mapping)
- `crates/worldwake-systems/src/perception.rs` (modify — extend institutional claim-key mapping)
- `crates/worldwake-sim/src/institutional_knowledge_trace.rs` (modify — summarize the new institutional belief lane cleanly)

## Out of Scope

- Direct colocated agent `report_found` propagation backed by resolved expectations — owned by `S59EXPOBLSUB-013`
- New AI goal-family work unless reassessment shows the institutional carrier must become planner-visible in the same slice

## Acceptance Criteria

### Tests That Must Pass

1. `report_missing` writes or supersedes one canonical office-register missing-person claim for the reported subject when a local `OfficeRegister` exists
2. Office-targeted `report_found` writes or supersedes that same office-register lane with the found outcome for the subject
3. `consult_record` projects the new claim family through the existing institutional substrate
3. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. No duplicate authority path for the same office-facing missing/found-person fact
2. No omniscient global registry for missing/found persons

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/report_actions.rs` — focused office-register write / supersession tests for `report_missing` and office-targeted `report_found`
2. `crates/worldwake-systems/src/consult_record_actions.rs` — focused projection test for the new institutional claim family

### Commands

1. `cargo test -p worldwake-systems report_actions`
2. `cargo test -p worldwake-systems consult_record_actions`
3. `cargo test -p worldwake-systems`

## Outcome

Completed on 2026-04-07.

- Added the shared institutional missing/found-person carrier on the existing `OfficeRegister` path via `InstitutionalClaim::MissingPersonStatus` plus `InstitutionalBeliefKey::MissingPersonStatus`.
- Extended `report_missing` to append or supersede one canonical office-register lane for the reported subject when a unique local `OfficeRegister` exists, while preserving the existing `ViolationMemory` behavior.
- Extended `report_found` with an office-register target branch that writes the found outcome into that same lane, while preserving the already-landed direct-agent propagation branch.
- Wired the new claim family through institutional belief projection, consultation, perception/tell mappings, trace summarization, and relay ordering exhaustiveness.

## Verification Result

- Passed `cargo test -p worldwake-systems report_actions`
- Passed `cargo test -p worldwake-systems consult_record_actions`
- Passed `cargo test -p worldwake-systems`
