# S153GOLDGAPSCALE-004: Office-backed patrol duty assignment substrate

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — institutional duty components/records, duty lifecycle systems, patrol candidate/ranking integration, scenario/golden proof
**Deps**: `specs/S153-golden-gaps-ai-architecture-scaling.md`, `archive/tickets/S153GOLDGAPSCALE-002.md`

## Problem

`archive/tickets/S153GOLDGAPSCALE-002.md` was rejected because the requested office-vacancy → patrol-gap golden assumed production substrate that does not exist. Live patrol behavior is driven by `PatrolRoute` / `PatrolProfile`; `ExpectationStore` overdue state drives missing-person search/report motives, not patrol-duty validity. FOUNDATIONS Canonical Scenario F still requires the architecture to produce office vacancy → duty degradation → patrol gap → opportunistic route predation from generic institutional state, not from a hidden scenario flag.

This ticket adds the missing office-backed patrol duty assignment substrate as first-class world state, then lands the S153 D3 golden against that substrate.

## Assumption Reassessment (2026-05-20)

1. Live patrol motive is `crates/worldwake-ai/src/ranking.rs::patrol_motive`: it reads `PatrolProfile`, `PatrolRoute`, unresolved thefts, believed office vacancies, and force contests. It does not read `ExpectationStore`.
2. Live expectation-overdue motive is `crates/worldwake-ai/src/ranking.rs::expectation_response_motive`: it applies to `GoalKind::SearchForMissing`, `GoalKind::ReportMissing`, and `GoalKind::ReportFound`. `ExpectationBasis::DutyAssignment` increases missing-person expectation weight only; it is not a patrol-duty lifecycle.
3. Live patrol assignment state is `crates/worldwake-core/src/patrol.rs::PatrolRoute` plus `PatrolProfile`. It records assigned places and patrol parameters, but no issuing office, assignee duty id, lifecycle state, renewal cadence, vacancy invalidator, or inspectable social artifact.
4. Shared boundary under audit: authoritative institutional duty state in `worldwake-core` / `worldwake-systems` must become the source that AI candidate generation, ranking, and portfolio admission use for office-backed patrol duties. Derived planner views may cache it, but they cannot become truth.
5. Information-path statement: an office-backed patrol duty is a social artifact/record. Agents may learn about it through local office records, notices, direct assignment, testimony, or observation of non-performance. This ticket must not give AI global knowledge of all duties.
6. Canonical end state: a guard patrols because they hold an active duty assignment issued by an office or lawful delegate. When that office is vacant and no successor/delegate renews the assignment, the duty degrades or lapses through explicit lifecycle state, and the guard's `ObligationDuty` slot no longer treats it as a valid patrol obligation.
7. If implementation finds an already-existing duty artifact that satisfies this contract, narrow this ticket to wiring and golden proof, but record the exact existing owner first.

## Architecture Check

1. A concrete duty assignment artifact aligns with FND-23, FND-25, and FND-25A: the duty has issuer, assignee, jurisdiction/route, lifecycle, legal effect, visibility/actionability, invalidators, and causal records.
2. AI integration aligns with FND-20 and FND-21: guards choose patrol because their beliefs and duty state make it a lawful commitment, and they can revise when the duty lapses or its assumptions break.
3. Systems remain state-mediated per FND-26: office lifecycle, duty maintenance, patrol candidate generation, and route-danger aftermath communicate through state and event history, not direct system calls.
4. No backward-compatibility shim: once office-backed duties become the canonical patrol-obligation path, update or remove stale test-only assumptions rather than preserving a parallel fake path.

## Verification Layers

1. Duty assignment creation / renewal / lapse -> authoritative duty component or record state plus append-only event-log delta.
2. Office vacancy or missing successor degrades/lapses office-backed duties -> duty lifecycle state and causal event history.
3. Active duty exposes a patrol candidate in the guard's `ObligationDuty` slot -> candidate-generation and decision-trace proof.
4. Lapsed/degraded duty removes or suppresses that patrol obligation -> decision trace showing `ObligationDuty` no longer ranks the patrol duty.
5. Patrol gap permits route predation only through ordinary actor behavior -> action trace / event log for route traversal or predation, with no hidden scenario flag.
6. Merchant or other observer learns the route danger through local evidence/report path -> `RoutePreferenceEntry` or successor route-danger belief state.
7. Determinism -> same seed produces equal event log hash and equal `ScenarioDiagnosticsReport`.

## What to Change

### 1. Add office-backed duty assignment state

Introduce a concrete duty assignment carrier with at least: issuing office/delegate, assignee, duty kind (`PatrolRoute` initially), covered places/route, created tick, renewal or deadline/grace policy, lifecycle state, legal effect/actionability, and provenance/cause. Choose component vs. record placement during implementation by inspecting current office/artifact/record storage.

### 2. Add lifecycle maintenance

Add or extend a system that degrades, suspends, lapses, or expires office-backed patrol duties when the issuing office is vacant, suspended, destroyed, or lacks a lawful successor/delegate to renew them. Lifecycle changes must emit ordinary append-only causal records.

### 3. Wire AI patrol obligation consumption

Update candidate generation/ranking/portfolio admission so active office-backed patrol duties are the source of `ObligationDuty` patrol candidates. Lapsed/degraded duties must not remain valid obligation candidates, though records may remain visible/history-bearing.

### 4. Land the S153 D3 golden

Add the office-vacancy → patrol-gap golden once the substrate exists. The golden should prove the full chain from magistrate death/vacancy through duty lapse, slot revision, unpatrolled route exploitation, route-danger observation, and determinism.

### 5. Truth-sync generated golden docs

Regenerate golden inventory/docs after the golden lands.

## Files to Touch

- `crates/worldwake-core/src/*` (new/modified duty assignment types and component/record registration)
- `crates/worldwake-systems/src/*` (duty lifecycle maintenance)
- `crates/worldwake-ai/src/candidate_generation.rs` and/or adjacent AI ranking/portfolio surfaces (consume active duty assignments)
- `crates/worldwake-ai/tests/scenarios/office_vacancy.rs` (new golden after substrate lands)
- `crates/worldwake-ai/tests/scenarios/mod.rs` (register golden module)
- `docs/generated/golden-*.md` (regenerated golden inventory/docs)
- `specs/S153-golden-gaps-ai-architecture-scaling.md` (truth-sync once the golden lands)

## Out of Scope

- The scaled-contention golden and route-blocker helper (`tickets/S153GOLDGAPSCALE-003.md`).
- A general duty system for every possible institutional duty kind beyond the patrol duty path needed for Canonical Scenario F.
- Omniscient town-manager code, hidden scenario flags, or planner-only fake duty state.
- Backward-compatibility support for a parallel non-duty patrol-obligation path once the new path is canonical.

## Acceptance Criteria

### Tests That Must Pass

1. Focused core/system tests prove duty assignment creation, renewal/lapse, and office-vacancy invalidation/degradation.
2. Focused AI test proves active duty assignments generate/rank `GoalKind::Patrol` through `SlotKind::ObligationDuty`, while lapsed/degraded duties do not.
3. `cargo test -p worldwake-ai --test golden_ai office_vacancy` passes once the golden lands.
4. `python3 scripts/golden_inventory.py --write --check-docs` passes after scenario metadata lands.
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Patrol-gap behavior emerges from concrete office/duty lifecycle state, not a hidden scenario flag.
2. Duty state remains inspectable and causally reconstructable after lapse/degradation.
3. AI reads only lawful belief/local/institutional record paths; no global duty omniscience.
4. Planner/ranking views may derive summaries from duty state but cannot become authoritative truth.

## Test Plan

### New/Modified Tests

1. Focused duty lifecycle tests in the owning core/system module.
2. Focused AI candidate/ranking/portfolio tests for active vs. lapsed office-backed patrol duties.
3. `crates/worldwake-ai/tests/scenarios/office_vacancy.rs` — S153 D3 golden after substrate lands.

### Commands

1. `cargo test -p worldwake-core <focused duty test>`
2. `cargo test -p worldwake-systems <focused duty lifecycle test>`
3. `cargo test -p worldwake-ai <focused patrol duty AI test>`
4. `cargo test -p worldwake-ai --test golden_ai office_vacancy`
5. `python3 scripts/golden_inventory.py --write --check-docs`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace --all-targets -- -D warnings`
