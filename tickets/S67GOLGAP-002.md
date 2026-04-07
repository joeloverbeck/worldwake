# S67GOLGAP-002: Report missing + institutional record golden test

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — golden test only
**Deps**: S67GOLGAP-001

## Problem

No golden scenario exercises `ReportMissing`, `report_missing`, `InstitutionalClaim::MissingPersonStatus`, or the candidate-generation suppression logic that prevents duplicate missing-person reports. This ticket adds a golden proving the report-then-search emergent chain with institutional record creation and AI suppression.

## Assumption Reassessment (2026-04-07)

1. `emit_search_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs:3241` emits both `SearchForMissing` (line 3286) and `ReportMissing` (line 3312) for each overdue expectation. `ReportMissing` emission is suppressed when `violation_memory.is_recorded(&missing_violation, ctx.current_tick)` returns true (lines 3302-3307).
2. `GoalKind::ReportMissing { subject, to_office }` exists at `crates/worldwake-core/src/goal.rs` with `GoalKey::from` handling. The `to_office` field is emitted as `None` by `emit_search_candidates` (line 3314).
3. `PlannerOpKind::ReportMissing` is wired in `crates/worldwake-ai/src/planner_ops.rs:48` with domain mapping at line 141.
4. `report_missing` action is registered in `crates/worldwake-systems/src/action_registry.rs:55` via `register_report_missing_action`. Implementation in `crates/worldwake-systems/src/report_actions.rs`.
5. `report_missing` creates `ViolationKind::EntityMissing { entity, expected_place }` in `ViolationMemory`. When a local unique `OfficeRegister` exists, it also writes `InstitutionalClaim::MissingPersonStatus` with `MissingPersonReportStatus::Missing { expected_place }`. Verified in `crates/worldwake-systems/src/report_actions.rs`.
6. `InstitutionalClaim::MissingPersonStatus` and `InstitutionalBeliefKey::MissingPersonStatus` exist at `crates/worldwake-core/src/institutional.rs:68-73` and `215-217` respectively.
7. `MissingPersonReportStatus` enum has variants `Missing`, `FoundSafe`, `FoundWounded`, `FoundDead` at `crates/worldwake-core/src/institutional.rs:18-23`.
8. The suppression logic is: after `report_missing` commits and creates the `ViolationMemory` entry, the next candidate generation cycle's `is_recorded` check returns true, so `ReportMissing` is not re-emitted, but `SearchForMissing` still is (it's emitted before the suppression check at line 3283).
9. No existing golden test references `ReportMissing`, `report_missing`, or `MissingPersonStatus`. Confirmed by grep across `crates/worldwake-ai/tests/`.
10. Highest scenario ID after S67GOLGAP-001 will be 120. This scenario will be 121.
11. This scenario needs the subject agent to be absent from the expected place (so the search component can demonstrate the post-report shift to searching, even if the search doesn't find the subject).
12. The scenario isolates `ReportMissing` from competing goals by keeping the reporter fully satiated and without political, trade, or combat affordances. An `OfficeRegister` entity at the reporter's place is required for the institutional record branch.

## Architecture Check

1. Adding to the `golden_expectation.rs` file created by S67GOLGAP-001 keeps S59 behavioral domain goldens cohesive in one file.
2. The scenario demonstrates that `ReportMissing` and `SearchForMissing` self-organize through suppression logic — the agent reports first (when the violation is unrecorded), then the suppression filter kicks in and the agent shifts to searching. This is a 6-system chain tested as one cohesive golden.
3. No backward-compatibility shims.

## Verification Layers

1. AI emits both `ReportMissing` and `SearchForMissing` candidates from overdue expectation -> decision trace (candidate diagnostics show both goal kinds)
2. Agent selects and commits `report_missing` -> action trace (committed action with report_missing domain)
3. `ViolationKind::EntityMissing` created in `ViolationMemory` -> authoritative world state (read `ViolationMemory` after report_missing commit)
4. `InstitutionalClaim::MissingPersonStatus` written to `OfficeRegister` -> authoritative world state (read `RecordData` after report_missing commit)
5. Next candidate generation cycle suppresses `ReportMissing` but still emits `SearchForMissing` -> decision trace (candidate diagnostics show only SearchForMissing, not ReportMissing)
6. Agent proceeds to plan `search_place` -> action trace (committed action with search_place domain)
7. Deterministic replay produces identical event log -> replay fidelity (hash comparison)

## What to Change

### 1. Implement Scenario 121 primary test

Add to `crates/worldwake-ai/tests/golden_expectation.rs`:

`golden_report_missing_creates_violation_and_institutional_record`:

1. Build world with: 2+ places, reporter agent (with `ViolationDispositionProfile`, `PerceptionProfile`, satiated needs, overdue `ExpectationStore` record), an office entity with `OfficeRegister` at the reporter's place, subject agent at a different place (absent from expected place)
2. Tick until `check_overdue_expectations` transitions the record to `Overdue`
3. Assert AI emits both `ReportMissing` and `SearchForMissing` candidates
4. Assert agent selects and commits `report_missing`
5. Assert `ViolationMemory` contains `ViolationKind::EntityMissing` for the subject
6. Assert `OfficeRegister` contains `InstitutionalClaim::MissingPersonStatus` with `MissingPersonReportStatus::Missing { expected_place }`
7. Continue ticking — assert next candidate generation suppresses `ReportMissing` (violation now recorded) but still emits `SearchForMissing`
8. Assert agent proceeds to plan/execute `search_place`

### 2. Implement Scenario 121 replay companion

`golden_report_missing_creates_violation_and_institutional_record_replays_deterministically`:

Standard replay companion: run the same scenario twice with the same seed, compare event log hashes.

## Files to Touch

- `crates/worldwake-ai/tests/golden_expectation.rs` (modify — add Scenario 121 + replay)

## Out of Scope

- `report_found` golden — not autonomously plannable (no `GoalKind::ReportFound`)
- `EscortToSafety` golden — no candidate generation exists
- `consult_record` projection of `MissingPersonStatus` — tested in focused `consult_record_actions.rs` tests
- `ask_about_person` hearsay transfer chain — deferred to future gap spec
- Institutional belief projection of `MissingPersonStatus` through perception/tell — covered by focused tests in `worldwake-systems`

## Acceptance Criteria

### Tests That Must Pass

1. `golden_report_missing_creates_violation_and_institutional_record` — overdue expectation triggers both ReportMissing and SearchForMissing, agent reports first, ViolationMemory and MissingPersonStatus records created, suppression kicks in, agent shifts to search
2. `golden_report_missing_creates_violation_and_institutional_record_replays_deterministically` — deterministic replay fidelity
3. Existing suite: `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `ReportMissing` is emitted only when the corresponding `ViolationKind::EntityMissing` is not yet recorded in `ViolationMemory`
2. `InstitutionalClaim::MissingPersonStatus` is written only when a local unique `OfficeRegister` exists — no omniscient global registry
3. After `report_missing` commits, `SearchForMissing` remains emitted — reporting does not suppress searching
4. Agent never reads world state directly — reports from `ExpectationStore` beliefs
5. Conservation holds — no physical goods created or destroyed

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_expectation.rs::golden_report_missing_creates_violation_and_institutional_record` — proves the 6-system emergent chain: ExpectationCheck -> AI (dual emission) -> ReportMissing -> ViolationMemory + MissingPersonStatus -> AI (suppression) -> SearchForMissing
2. `crates/worldwake-ai/tests/golden_expectation.rs::golden_report_missing_creates_violation_and_institutional_record_replays_deterministically` — replay fidelity for the report+search chain

### Commands

1. `cargo test -p worldwake-ai --test golden_expectation`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
