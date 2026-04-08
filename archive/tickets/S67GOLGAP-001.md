# S67GOLGAP-001: Overdue expectation drives search golden test

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — bounded AI feasibility exhaustiveness fix plus golden test coverage
**Deps**: S59 (all S59EXPOBLSUB tickets completed)

## Problem

The entire S59 expectation/search/report behavioral domain has zero golden E2E coverage. No golden scenario exercises `SearchForMissing`, `search_place`, `ExpectationCheck`, or the `ExpectationStore` lifecycle. This ticket adds the baseline golden proving the core emergent chain: overdue expectation -> AI candidate generation -> search -> resolution.

## Assumption Reassessment (2026-04-07)

1. `ExpectationStore` is a universal component on `EntityKind::Agent` with public `records: BTreeMap<ExpectationId, ExpectationRecord>` field — tests can insert expectations directly. Verified at `crates/worldwake-core/src/expectation.rs:73-76`.
2. `check_overdue_expectations` runs as `SystemId::ExpectationCheck` after Perception and before EvidenceDecay. Verified at `crates/worldwake-systems/src/expectation_check.rs:7`.
3. `emit_search_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs:3241` requires `ViolationDispositionProfile` on the agent (line 3246) and reads `ExpectationStore` via `ctx.view.expectation_store(ctx.agent)` (line 3250). It emits `GoalKind::SearchForMissing` for each overdue record (line 3286).
4. `GoalKind::SearchForMissing { subject, last_seen }` exists in `crates/worldwake-core/src/goal.rs:44-47` with `GoalKey::from` handling at line 164.
5. `PlannerOpKind::SearchPlace` is wired in `crates/worldwake-ai/src/planner_ops.rs:46` with domain mapping at line 139. Planner semantics entries exist at lines 1613, 1904, 1950.
6. `search_place` action is registered in `crates/worldwake-systems/src/action_registry.rs:54` via `register_search_place_action`. Implementation in `crates/worldwake-systems/src/search_actions.rs`.
7. `LastSeenMemory` is a universal component on `EntityKind::Agent` with `records: BTreeMap<EntityId, LastSeenRecord>` and default `capacity: 20`. Verified at `crates/worldwake-core/src/expectation.rs:101-115`.
8. No existing golden test references `SearchForMissing`, `search_place`, `ExpectationStore`, or `LastSeenMemory`. Confirmed by grep across `crates/worldwake-ai/tests/`.
9. `golden_emergent.rs` is 8177 lines — a new `golden_expectation.rs` file is warranted for the S59 behavioral domain.
10. Highest existing scenario ID is 119. This scenario will be 120.
11. `PerceptionProfile` is required on the searcher agent for post-search observation of the subject entity and resulting state updates.
12. The scenario isolates `SearchForMissing` from competing goal kinds by keeping the searcher agent fully satiated (no hunger/thirst/sleep pressure) and without political, trade, or combat affordances.
13. Live correction: `docs/generated/golden-scenario-index.md` currently reports 138 total scenario blocks because the inventory includes alphanumeric identifiers such as `S02` and `79b`, but the highest numeric `Scenario N` identifier is still `119`, so `Scenario 120` remains the next free numeric slot. Safe because the generated index and direct grep across `crates/worldwake-ai/tests/` agree on the live numeric boundary.
14. Live correction: adding a new `// Scenario 120:` block requires regenerating the generated golden inventory/docs via `python3 scripts/golden_inventory.py --write --check-docs`. Safe because `tickets/README.md` and `docs/golden-e2e-testing.md` make that mechanical docs-sync step part of the owned golden verification surface.
15. Live correction: the current command list combines `cargo clippy` and `cargo test --workspace` in one shell line. Split them into sequential commands to match repo execution guidance and avoid chained verification ambiguity. Safe because this is a mechanical verification-surface correction only.
16. Live correction from focused verification: the current AI feasibility layer still panics on `GoalKind::SearchForMissing` under `FeasibilityStrategy::NoOpinion` in `crates/worldwake-ai/src/feasibility.rs`, even though `GoalDispatchDeclaration` already reserves `SearchForMissing`, `ReportMissing`, and `EscortToSafety` with `NoOpinion` in `crates/worldwake-ai/src/goal_dispatch_decl.rs`. This ticket therefore cannot remain golden-only; it must absorb the bounded compile/runtime parity fix needed for the new golden to execute the live S59 goal family honestly.

## Architecture Check

1. A dedicated `golden_expectation.rs` is cleaner than further expanding the 8177-line `golden_emergent.rs`. The S59 behavioral domain (expectations, search, missing-person reports) is a distinct cross-system interaction surface that deserves its own test file.
2. Absorbing the local `FeasibilityStrategy::NoOpinion` parity fix is cleaner than weakening the golden or papering over the runtime panic. The dispatch table already claims these S59 goals are lawful; feasibility must stay exhaustively consistent with that shared contract.
3. No backward-compatibility shims — the feasibility fix is direct exhaustive handling, and the new golden file adds no legacy path.

## Verification Layers

1. `ExpectationState::Active` -> `ExpectationState::Overdue` transition after deadline + grace -> authoritative world state (read `ExpectationStore` component after system tick)
2. `SearchForMissing` candidate emission from overdue expectation -> decision trace (candidate diagnostics)
3. Agent plans and commits `search_place` at the expected place -> action trace (committed action with search_place domain)
4. `ExpectationRecord` resolved with `FoundSafe` outcome -> authoritative world state (read `ExpectationStore` after search commit)
5. `LastSeenMemory` updated with subject sighting -> authoritative world state (read `LastSeenMemory` after search commit)
6. Deterministic replay produces identical event log -> replay fidelity (hash comparison)
7. `SearchForMissing` no longer panics at feasibility dispatch -> focused runtime/golden execution proof plus direct AI feasibility code path

## What to Change

### 1. Create `golden_expectation.rs` test file

Create a new golden test file at `crates/worldwake-ai/tests/golden_expectation.rs` with:

- Standard golden test imports and helpers (matching patterns from existing golden files)
- Scenario 120 test setup helper building: 2 places connected by travel edge, searcher agent (with `ViolationDispositionProfile`, `PerceptionProfile`, satiated needs, `ExpectationStore` containing one overdue expectation for the subject), subject agent at the expected place
- The `ExpectationRecord` should use `ExpectationBasis::RoutineReturn` with a deadline in the past and small grace_ticks so overdue triggers within the first few system ticks

### 0. Repair S59 goal feasibility exhaustiveness

- Extend `crates/worldwake-ai/src/feasibility.rs` so `FeasibilityStrategy::NoOpinion` handles the currently live S59 goal families reserved in `goal_dispatch_decl.rs` (`SearchForMissing`, `ReportMissing`, `EscortToSafety`) instead of panicking on reachable runtime input
- Keep the fix bounded to parity with the existing dispatch contract; do not widen planner behavior beyond making the declared `NoOpinion` strategy actually return `None`

### 2. Implement Scenario 120 primary test

`golden_overdue_expectation_drives_search`:

1. Build world with scenario setup
2. Tick until `check_overdue_expectations` transitions the record to `Overdue`
3. Assert the `ExpectationStore` record state is now `Overdue`
4. Continue ticking — assert AI generates `SearchForMissing` candidate
5. Assert agent plans and executes `search_place` at the expected place
6. Assert `ExpectationRecord` is resolved (state is `Resolved { outcome: FoundSafe { at_place } }`)
7. Assert `LastSeenMemory` contains an entry for the subject at the expected place

### 3. Implement Scenario 120 replay companion

`golden_overdue_expectation_drives_search_replays_deterministically`:

Standard replay companion: run the same scenario twice with the same seed, compare event log hashes.

## Files to Touch

- `crates/worldwake-ai/tests/golden_expectation.rs` (new — Scenario 120 + replay)
- `crates/worldwake-ai/src/feasibility.rs` (modify — keep `NoOpinion` feasibility exhaustive for live S59 goal families)
- `docs/generated/golden-e2e-inventory.md` (modify — generated inventory refresh after new scenario metadata)
- `docs/generated/golden-scenario-index.md` (modify — generated scenario index refresh after new scenario metadata)
- `docs/generated/golden-scenario-details/expectation.md` (new — generated per-file scenario details)

## Out of Scope

- `ReportMissing` golden coverage — owned by S67GOLGAP-002
- `report_found` or institutional record testing — not autonomously plannable (no `GoalKind::ReportFound`)
- `EscortToSafety` golden — no candidate generation exists
- `ask_about_person` hearsay transfer — deferred to future gap spec
- New test helpers beyond what this scenario requires

## Acceptance Criteria

### Tests That Must Pass

1. `golden_overdue_expectation_drives_search` — overdue expectation triggers SearchForMissing, agent searches at expected place, finds subject, expectation resolved FoundSafe, LastSeenMemory updated
2. `golden_overdue_expectation_drives_search_replays_deterministically` — deterministic replay fidelity
3. `GoalKind::SearchForMissing` no longer panics under `FeasibilityStrategy::NoOpinion`
4. Generated golden inventory/docs refresh cleanly via `python3 scripts/golden_inventory.py --write --check-docs`
5. Existing suite: `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Agent never reads world state directly — plans from `ExpectationStore` beliefs and `LastSeenMemory`, not authoritative entity positions
2. Search outcome updates local agent memory only — no global broadcast
3. `ExpectationStore` lifecycle transitions are authoritative state changes, not belief-side
4. Conservation holds — no physical goods created or destroyed by expectation/search operations

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_expectation.rs::golden_overdue_expectation_drives_search` — proves the baseline 5-system emergent chain: ExpectationCheck -> AI -> Travel -> SearchPlace -> ExpectationStore resolution
2. `crates/worldwake-ai/tests/golden_expectation.rs::golden_overdue_expectation_drives_search_replays_deterministically` — replay fidelity for the expectation/search chain

### Commands

1. `cargo test -p worldwake-ai --test golden_expectation`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completion date: 2026-04-07

Added `crates/worldwake-ai/tests/golden_expectation.rs` with `Scenario 120: Overdue Expectation Drives Search` plus deterministic replay coverage. The scenario proves the live S59 expectation/search chain: an overdue expectation emits `SearchForMissing`, the agent travels toward the expected place, commits `search_place`, resolves the `ExpectationRecord` to `FoundSafe`, and updates `LastSeenMemory`.

Focused verification exposed two bounded planner-contract contradictions that this ticket absorbed instead of weakening the golden:

1. `FeasibilityStrategy::NoOpinion` still panicked on live S59 goal kinds (`SearchForMissing`, `ReportMissing`, `EscortToSafety`) even though dispatch already reserved them there.
2. `SearchForMissing` planning could emit the goal but still fail to realize the route because the planner lacked a lawful synthesized `search_place` step/payload/progress barrier path and had no fallback use of the grounded expected-place evidence when `last_seen` was absent.

The implementation now keeps those planner surfaces aligned with the declared S59 contract:

- `crates/worldwake-ai/src/feasibility.rs` treats the live S59 `NoOpinion` goal family exhaustively instead of panicking.
- `crates/worldwake-ai/src/goal_model.rs` synthesizes colocated `search_place` roots for `SearchForMissing`, builds the required payload override, and treats `search_place` as the lawful progress barrier for that goal family.
- `crates/worldwake-ai/src/search/heuristic.rs` uses grounded evidence-place fallback only for `SearchForMissing` when the goal has no intrinsic place guidance, preserving existing routing behavior for unrelated goal families.
- `docs/generated/*` golden inventory files were refreshed, including the new `docs/generated/golden-scenario-details/expectation.md`.

## Verification Result

Passed sequentially:

1. `cargo test -p worldwake-ai test_s59_no_opinion_goals_fall_through_to_uncertain`
2. `cargo test -p worldwake-ai search_for_missing_builds_search_place_payload_override_when_colocated`
3. `cargo test -p worldwake-ai grounded_goal_synthesizes_search_place_root_targets_only_when_colocated`
4. `cargo test -p worldwake-ai combined_places_include_grounded_evidence_place_when_goal_has_no_intrinsic_place`
5. `cargo test -p worldwake-ai --test golden_expectation`
6. `cargo test -p worldwake-ai --test golden_t22_bandit_camp_destruction`
7. `cargo test -p worldwake-ai --test golden_integration`
8. `python3 scripts/golden_inventory.py --write --check-docs`
9. `cargo clippy --workspace --all-targets -- -D warnings`

Additional note:

- A prior broad `cargo test -p worldwake-ai` attempt during concurrent heavy-session pressure was killed by `SIGKILL` inside `golden_integration`. The affected high-signal suites were then rerun in isolation and passed cleanly under single-process execution.
