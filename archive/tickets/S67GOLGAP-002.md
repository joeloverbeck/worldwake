# S67GOLGAP-002: Report missing + institutional record golden test

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — bounded `worldwake-core` + `worldwake-ai` planner fix required
**Deps**: S67GOLGAP-001

## Problem

No golden scenario exercises `ReportMissing`, `report_missing`, `InstitutionalClaim::MissingPersonStatus`, or the candidate-generation suppression logic that prevents duplicate missing-person reports. This ticket adds a golden proving the report-then-search emergent chain with institutional record creation and AI suppression.

## Assumption Reassessment (2026-04-07)

1. `emit_search_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs:3241` emits both `SearchForMissing` (line 3286) and `ReportMissing` (line 3312) for each overdue expectation. `ReportMissing` emission is suppressed when `violation_memory.is_recorded(&missing_violation, ctx.current_tick)` returns true (lines 3302-3307).
2. `GoalKind::ReportMissing { subject, to_office }` exists at `crates/worldwake-core/src/goal.rs` with `GoalKey::from` handling. The `to_office` field is emitted as `None` by `emit_search_candidates` (line 3314).
3. `PlannerOpKind::ReportMissing` is wired in `crates/worldwake-ai/src/planner_ops.rs:48` with domain mapping at line 141.
4. `report_missing` action is registered in `crates/worldwake-systems/src/action_registry.rs:55` via `register_report_missing_action`. Implementation in `crates/worldwake-systems/src/report_actions.rs`.
5. `report_missing` creates `ViolationKind::EntityMissing { entity, expected_place }` in `ViolationMemory`. When a unique `OfficeRegister` exists at the expectation record's `expected_place`, it also writes `InstitutionalClaim::MissingPersonStatus` with `MissingPersonReportStatus::Missing { expected_place }`. Verified in `crates/worldwake-systems/src/report_actions.rs`.
6. `InstitutionalClaim::MissingPersonStatus` and `InstitutionalBeliefKey::MissingPersonStatus` exist at `crates/worldwake-core/src/institutional.rs:68-73` and `215-217` respectively.
7. `MissingPersonReportStatus` enum has variants `Missing`, `FoundSafe`, `FoundWounded`, `FoundDead` at `crates/worldwake-core/src/institutional.rs:18-23`.
8. The suppression logic is: after `report_missing` commits and creates the `ViolationMemory` entry, the next candidate generation cycle's `is_recorded` check returns true, so `ReportMissing` is not re-emitted, but `SearchForMissing` still is (it's emitted before the suppression check at line 3283).
9. No existing golden test references `ReportMissing`, `report_missing`, or `MissingPersonStatus`. Confirmed by grep across `crates/worldwake-ai/tests/`.
10. Highest scenario ID after S67GOLGAP-001 will be 120. This scenario will be 121.
11. This scenario needs the subject agent to be absent from the expected place so `report_missing` produces a real missing-person violation and the post-report search shift remains meaningful.
12. The scenario isolates `ReportMissing` from competing goals by keeping the reporter fully satiated and without political, trade, or combat affordances. To exercise the institutional-record branch honestly, the scenario should make the expectation's `expected_place` equal the reporter's current place and seed a unique `OfficeRegister` there.
13. Because `SearchForMissing` and `ReportMissing` are both expectation-response goals, a deterministic "report first, then search" golden should bias the reporter's utility profile so `ReportMissing` outranks `SearchForMissing` at the opening decision tick, rather than assuming a stable default tie-break.
14. Reassessment during implementation exposed a live planner contradiction: `GoalKind::ReportMissing` did not carry the `ExpectationId`, so GOAP could not synthesize the required `ReportMissingActionPayload` from planning state. The goal also treated `report_missing` as a non-terminal leaf, so even a payload-bearing root step could not satisfy the goal. This ticket must own that bounded planner fix instead of pretending the gap is golden-only.

## Architecture Check

1. Adding to the `golden_expectation.rs` file created by S67GOLGAP-001 keeps S59 behavioral domain goldens cohesive in one file.
2. The scenario demonstrates that `ReportMissing` and `SearchForMissing` self-organize through suppression logic — the agent reports first while the violation is still unrecorded, then the suppression filter kicks in and the agent shifts to searching. The golden should seed remote last-seen evidence so the post-report search has a lawful place target instead of stalling on a purely local miss.
3. No backward-compatibility shims.
4. Canonical carrier after the fix: `GoalKind::ReportMissing { expectation_id: Some(...) }` is now the planner-visible transport for the live report payload. No duplicate side-channel was introduced.

## Verification Layers

1. AI emits both `ReportMissing` and `SearchForMissing` candidates from one overdue expectation -> decision trace
2. Agent selects and commits `report_missing` first -> decision trace + action trace
3. `ViolationKind::EntityMissing` created in `ViolationMemory` -> authoritative world state
4. `InstitutionalClaim::MissingPersonStatus` written to the `OfficeRegister` at the expectation's `expected_place` -> authoritative world state
5. Next candidate generation cycle suppresses `ReportMissing` but still emits `SearchForMissing` -> decision trace
6. Agent proceeds into the search path and commits `search_place` for the same subject -> action trace
7. Deterministic replay produces identical event log and world hash -> replay fidelity

## What to Change

### 1. Implement Scenario 121 primary test

Add to `crates/worldwake-ai/tests/golden_expectation.rs`:

`golden_report_missing_creates_violation_and_institutional_record`:

1. Build world with: 2+ places, reporter agent (with `ViolationDispositionProfile`, `PerceptionProfile`, satiated needs, and a utility profile that prefers reporting over searching on the opening tick), an overdue `ExpectationStore` record whose `expected_place` equals the reporter's place, a unique `OfficeRegister` at that same place, and a subject agent at a different place with remote `LastSeenMemory` evidence
2. Tick until `check_overdue_expectations` transitions the record to `Overdue`
3. Assert AI emits both `ReportMissing` and `SearchForMissing` candidates
4. Assert agent selects and commits `report_missing`
5. Assert `ViolationMemory` contains `ViolationKind::EntityMissing` for the subject
6. Assert the `OfficeRegister` at `expected_place` contains `InstitutionalClaim::MissingPersonStatus` with `MissingPersonReportStatus::Missing { expected_place }`
7. Continue ticking — assert the next candidate generation suppresses `ReportMissing` (violation now recorded) but still emits `SearchForMissing`
8. Assert the agent shifts onto the search path and commits `search_place` for the same subject

### 2. Implement Scenario 121 replay companion

`golden_report_missing_creates_violation_and_institutional_record_replays_deterministically`:

Standard replay companion: run the same scenario twice with the same seed, compare event log hashes.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify — add planner-visible `expectation_id` carrier to `GoalKind::ReportMissing`)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — emit `ReportMissing` with the live `ExpectationId`)
- `crates/worldwake-ai/src/goal_model.rs` (modify — synthesize `ReportMissingActionPayload`, current-place guidance, root-target synthesis, and terminal satisfaction)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — fill the new `expectation_id` field for non-expectation dispatch surfaces with `None`)
- `crates/worldwake-ai/src/goal_dispatch_key.rs` (modify — compile-safe constructor fallout for the new field)
- `crates/worldwake-ai/src/feasibility.rs` (modify — constructor fallout for the new field)
- `crates/worldwake-ai/src/ranking.rs` (modify — constructor fallout for the new field)
- `crates/worldwake-ai/tests/golden_expectation.rs` (modify — add Scenario 121 + replay and isolate Scenario 120 from the new report path)
- `crates/worldwake-cli/src/display.rs` (modify — compile-safe pattern fallout for the new field)

## Out of Scope

- `report_found` golden — not autonomously plannable (no `GoalKind::ReportFound`)
- `EscortToSafety` golden — no candidate generation exists
- `consult_record` projection of `MissingPersonStatus` — tested in focused `consult_record_actions.rs` tests
- `ask_about_person` hearsay transfer chain — deferred to future gap spec
- Institutional belief projection of `MissingPersonStatus` through perception/tell — covered by focused tests in `worldwake-systems`

## Acceptance Criteria

### Tests That Must Pass

1. `golden_report_missing_creates_violation_and_institutional_record` — overdue expectation triggers both ReportMissing and SearchForMissing, agent reports first, ViolationMemory and MissingPersonStatus records are created at the expectation's `expected_place`, suppression kicks in, and the agent shifts to search
2. `golden_report_missing_creates_violation_and_institutional_record_replays_deterministically` — deterministic replay fidelity
3. Focused planner proof: `GoalKind::ReportMissing` with an `ExpectationId` builds a lawful `ReportMissingActionPayload`
4. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `ReportMissing` is emitted only when the corresponding `ViolationKind::EntityMissing` is not yet recorded in `ViolationMemory`
2. `InstitutionalClaim::MissingPersonStatus` is written only when a unique `OfficeRegister` exists at the expectation's `expected_place` — no omniscient global registry
3. After `report_missing` commits, `SearchForMissing` remains emitted — reporting does not suppress searching
4. Agent never reads world state directly — reports from `ExpectationStore` beliefs
5. Conservation holds — no physical goods created or destroyed
6. Planner-visible missing-person reports must carry enough identity to synthesize the authoritative `ReportMissingActionPayload` without reading authoritative world state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_expectation.rs::golden_report_missing_creates_violation_and_institutional_record` — proves the 6-system emergent chain: ExpectationCheck -> AI (dual emission) -> ReportMissing -> ViolationMemory + MissingPersonStatus -> AI (suppression) -> SearchForMissing
2. `crates/worldwake-ai/tests/golden_expectation.rs::golden_report_missing_creates_violation_and_institutional_record_replays_deterministically` — replay fidelity for the report+search chain
3. `crates/worldwake-ai/src/goal_model.rs::report_missing_builds_payload_override_from_expectation_id` — focused proof that the planner can synthesize the required `ReportMissingActionPayload`

### Commands

1. `cargo test -p worldwake-ai report_missing_builds_payload_override_from_expectation_id`
2. `cargo test -p worldwake-ai --test golden_expectation`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-04-07
- What changed: Implemented Scenario 121 in `golden_expectation.rs` and corrected the live `ReportMissing` planner path it exposed. `GoalKind::ReportMissing` now carries an optional `ExpectationId`, candidate generation emits that carrier for overdue expectations, and the goal model uses it to synthesize `ReportMissingActionPayload` plus treat `report_missing` as a terminal goal-satisfying step. The golden proves the honest contract: overdue expectation -> dual emission -> `report_missing` commit -> `ViolationMemory` + `MissingPersonStatus` institutional record -> duplicate-report suppression -> committed `search_place` follow-up.
- Deviations from original plan: Scenario 120 was re-isolated with a search-favoring utility profile so the new report path does not distort the earlier search-only golden. The ticket also widened from "golden only" to include the bounded planner fix required to make `ReportMissing` truly live.
- Verification results:
  - `cargo test -p worldwake-ai report_missing_builds_payload_override_from_expectation_id`
  - `cargo test -p worldwake-ai --test golden_expectation`
  - `python3 scripts/golden_inventory.py --write --check-docs`
  - `cargo clippy --workspace --all-targets -- -D warnings`
