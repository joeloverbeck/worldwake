# S67: Golden E2E Gaps — S59 Expectation and Obligation Substrate

## Summary

S59 introduced a complete expectation/search/report behavioral domain (3 GoalKind variants, 5 actions, 1 system function, candidate generation, institutional missing-person records) with zero golden E2E coverage. This spec proposes scenarios that demonstrate the core emergent chains.

## Analysis Date

2026-04-07

## What S59 Introduced

- **Components**: `ExpectationStore`, `LastSeenMemory` (both universal on Agent)
- **GoalKind variants**: `SearchForMissing`, `ReportMissing`, `EscortToSafety`
- **Actions**: `search_place`, `ask_about_person`, `report_missing`, `report_found`, `escort_to_safety`
- **PlannerOpKind variants**: `SearchPlace`, `AskAboutPerson`, `ReportMissing`, `EscortToSafety`, `ReportFound`
- **SystemFn**: `check_overdue_expectations` (`SystemId::ExpectationCheck`)
- **Candidate generation**: `emit_search_candidates()` emitting `SearchForMissing` and `ReportMissing`
- **Institutional**: `InstitutionalClaim::MissingPersonStatus`, `InstitutionalBeliefKey::MissingPersonStatus`

## Coverage Status

The entire S59 behavioral domain has zero golden scenarios. No GoalKind, ActionDomain, or System entry related to S59 appears in the coverage matrix.

## Implementation Constraints

- `EscortToSafety` has no candidate generation — AI cannot plan for it autonomously. Not testable as a golden E2E scenario.
- `GoalKind::ReportFound` does not exist — `report_found` is exercised through affordance-based start, not through the planner goal system.
- `report_missing` office-register propagation requires a local `OfficeRegister` at the actor's place.

---

## Proposed Scenarios

### Scenario S67-A: Overdue Expectation Drives Search at Expected Place

**Identifier**: S67-A

**Description**: An agent holds an `ExpectationRecord` for a subject expected at a specific place. The system ticks past the deadline + grace period. `check_overdue_expectations` transitions the record to `Overdue`. On the next AI tick, `emit_search_candidates()` produces a `SearchForMissing` goal. The agent plans and executes `search_place` at the expected place. The subject is present — the search finds them alive. The agent's `ExpectationRecord` is resolved with `FoundSafe`, and `LastSeenMemory` is updated.

**GoalKinds exercised**: `SearchForMissing`

**ActionDomains exercised**: `Epistemic` (search_place), `Travel` (if expected place is remote)

**Systems exercised**: ExpectationCheck, AI (candidate generation, planning), search_place action, LastSeenMemory, ExpectationStore lifecycle

**Setup requirements**:
- 2 agents: searcher (with ExpectationStore containing an expectation) and subject
- 2+ places connected by travel edges
- Subject placed at the expected place
- ExpectationRecord with deadline in the past relative to starting tick, grace_ticks configured so overdue triggers within a few ticks
- Searcher has `ViolationDispositionProfile` (required by `emit_search_candidates`)
- Searcher has `PerceptionProfile` (required to observe search results)

**What emergence it demonstrates**: Overdue detection is a global clock tick, but the search response is fully agent-driven: the AI generates the goal from stored expectation state, plans travel + search, and the search outcome updates local memory. Five systems chain (ExpectationCheck -> AI -> Travel -> SearchPlace -> ExpectationStore resolution) with none calling any other directly. Removing any system breaks the chain.

**Foundation principle alignment**:
- P1 (Maximal Emergence): Search behavior emerges from expectation violation, not scripted
- P3 (Concrete State): ExpectationRecord with concrete fields, not abstract scores
- P7 (Information Locality): Agent must physically travel to search location
- P8 (Preconditions and Duration): Search action has duration and occupies the searcher
- P10 (Belief-Only Planning): Agent plans from expectation records and last-seen beliefs
- P12 (System Decoupling): All interactions are state-mediated
- P17 (Violated Expectation): Directly exercises this principle

**Why it is not a duplicate**: No existing golden scenario exercises SearchForMissing, search_place, ExpectationCheck, or ExpectationStore lifecycle. Scenario 57 exercises patrol-driven `EntityMissing` investigation via `InvestigateViolation`, which is a different code path (stale belief -> ViolationKind::EntityMissing -> investigate) from the S59 expectation-driven search path (ExpectationRecord -> Overdue -> SearchForMissing -> search_place).

---

### Scenario S67-B: Report Missing Creates Violation and Institutional Record

**Identifier**: S67-B

**Description**: An agent holds an overdue `ExpectationRecord`. `emit_search_candidates()` produces both `SearchForMissing` and `ReportMissing`. The agent selects `ReportMissing` (because the violation has not yet been recorded). The agent executes `report_missing` which creates a `ViolationKind::EntityMissing` entry in `ViolationMemory`. If a local `OfficeRegister` exists, `report_missing` also writes an `InstitutionalClaim::MissingPersonStatus` record. After the report, the same agent's next candidate generation cycle suppresses the duplicate `ReportMissing` (because the violation is now recorded) but still emits `SearchForMissing`.

**GoalKinds exercised**: `ReportMissing`, `SearchForMissing`

**ActionDomains exercised**: `Social` (report_missing), `Epistemic` (search_place)

**Systems exercised**: ExpectationCheck, AI (candidate generation with suppression logic), report_missing action, ViolationMemory, InstitutionalClaim::MissingPersonStatus, search_place action

**Setup requirements**:
- 2 agents: reporter and subject (subject absent from expected place)
- An office entity with `OfficeRegister` at the reporter's place
- ExpectationRecord overdue at start
- ViolationDispositionProfile on reporter
- PerceptionProfile on reporter

**What emergence it demonstrates**: The report-then-search sequence emerges from candidate generation priorities and suppression logic: the agent reports first (creating the institutional record), then the duplicate-report filter kicks in and the agent shifts to searching. This is a 6-system chain (ExpectationCheck -> AI -> ReportMissing -> ViolationMemory + InstitutionalClaim -> AI suppression -> SearchForMissing) demonstrating how reporting and searching self-organize without coordination.

**Foundation principle alignment**:
- P1 (Maximal Emergence): Report-then-search sequence emerges from suppression logic
- P3 (Concrete State): ViolationMemory entry, InstitutionalClaim::MissingPersonStatus
- P7 (Information Locality): Report happens at the reporter's local office
- P12 (System Decoupling): ExpectationCheck, AI, ViolationMemory, InstitutionalClaim are all separate systems interacting through state
- P17 (Violated Expectation): Directly exercises this principle
- P18 (Records Are World State): Institutional missing-person record is inspectable world state
- P26 (Systems Interact Through State): All cross-system interactions are state-mediated

**Why it is not a duplicate**: No existing golden scenario exercises ReportMissing, report_missing action, or InstitutionalClaim::MissingPersonStatus. The candidate-generation suppression logic (violation already recorded -> skip ReportMissing) is a distinct AI behavior path not tested anywhere.

---

## Ticket Breakdown

### S67-001: Implement Scenario S67-A golden test

**File**: `crates/worldwake-ai/tests/golden_emergent.rs` (or a new `golden_expectation.rs` if the emergent file is too large)

**Tasks**:
1. Build scenario with 2 agents, 2+ places, overdue ExpectationRecord
2. Assert `check_overdue_expectations` transitions record to Overdue
3. Assert AI emits `SearchForMissing` candidate
4. Assert agent plans and executes `search_place`
5. Assert ExpectationRecord resolved with `FoundSafe` and LastSeenMemory updated
6. Add `// Scenario S67-A` metadata header
7. Add deterministic replay companion (`*_replays_deterministically`)

### S67-002: Implement Scenario S67-B golden test

**File**: Same as S67-001

**Tasks**:
1. Build scenario with 2 agents, office with OfficeRegister, overdue ExpectationRecord, subject absent
2. Assert AI emits both `ReportMissing` and `SearchForMissing` candidates
3. Assert agent executes `report_missing` creating ViolationMemory entry
4. Assert `InstitutionalClaim::MissingPersonStatus` written to OfficeRegister
5. Assert next candidate generation cycle suppresses `ReportMissing` but still emits `SearchForMissing`
6. Assert agent proceeds to search_place
7. Add `// Scenario S67-B` metadata header
8. Add deterministic replay companion

## Replay and Conservation Requirements

- Each primary golden scenario MUST have a `*_replays_deterministically` companion
- Conservation verification: S59 introduces no physical goods, so `verify_conservation` should pass unchanged. ExpectationRecords and LastSeenRecords are informational, not conserved quantities.
- All scenarios must use `ChaCha8Rng` seeded determinism
