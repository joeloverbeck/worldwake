# HARPLASNAP-001: Define and enforce planner snapshot fidelity for institutional domain data

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` planning snapshot/state contract, focused fidelity tests, documentation note
**Deps**: `docs/FOUNDATIONS.md`, `archive/tickets/completed/E16BFORLEGJURCON-009.md`, `archive/tickets/completed/E16DPOLPLAN-023.md`

## Problem

The planner snapshot boundary has been evolving field-by-field, which makes it easy to drop semantics that live runtime belief views still expose. The recent force-law regression showed that partial copying of office fields let the planner lose institutional office meaning even while the live `PerAgentBeliefView` still exposed lawful affordances. Worldwake needs a cleaner snapshot fidelity contract so planning remains a truthful local compression of belief state rather than a lossy alternate model.

## Assumption Reassessment (2026-03-22)

1. The current snapshot architecture already carries many domain objects forward:
   - `PlanningSnapshot::build()` and `build_snapshot_entity()` live in `crates/worldwake-ai/src/planning_snapshot.rs`
   - `PlanningState` reads snapshot state through `impl RuntimeBeliefView for PlanningState` in `crates/worldwake-ai/src/planning_state.rs`
   - focused coverage already exists at `planning_snapshot::tests::snapshot_captures_institutional_belief_reads` and `planning_state::tests::planning_state_implements_goal_and_runtime_surfaces`
2. Reassessment against the finished force-law work showed a concrete mismatch: before the fix, snapshot entities stored only `jurisdiction` and `succession_law`, while `RuntimeBeliefView::office_data()` on `PlanningState` returned `None`. The live belief layer in `crates/worldwake-sim/src/per_agent_belief_view.rs` still returned full `OfficeData`, so planner and live affordance surfaces diverged.
3. The live goal family affected was `GoalKind::ClaimOffice`, and the exact operator/affordance surface was `PlannerOpKind::PressForceClaim` via `get_affordances()` and `office_actions::enumerate_press_force_claim_payloads()`.
4. This is a planner/runtime boundary ticket, not a golden ticket. The intended layer is focused snapshot/planning-state coverage plus narrow runtime parity assertions where useful.
5. The ordering contract is not event-log or action lifecycle ordering. The contract is semantic parity between live belief reads and snapshot belief reads for domain data the planner depends on.
6. No heuristic is being removed. The problem is a missing substrate: there is no explicit rule for which authoritative domain components must survive snapshotting intact versus which may be projected into derived summaries.
7. The first failure boundary in the motivating regression was runtime affordance reproduction inside the planner snapshot. The shared symbols already checked were `PlanningSnapshot::build()`, `build_snapshot_entity()`, `PlanningState::office_data()`, and `get_affordances()`.
8. The political closure boundary under discussion is the AI-layer affordance/planning boundary before any support declaration, succession resolution, or office-holder mutation.
9. No `ControlSource`, driver reset, or queued-input behavior is in scope.
10. Scenario isolation for the motivating regression intentionally excluded unrelated lawful branches. The contract here is not "all planning data must be identical to the world"; it is "all domain data required for lawful planner reasoning must survive snapshotting in semantically complete form."
11. Mismatch corrected: the problem is not office-specific business logic drift. It is a generic snapshot fidelity contract gap that happened to surface first through offices.
12. The survivability envelope is qualitative rather than cumulative arithmetic: any future domain whose planner semantics depend on richer component state than a hand-copied subset is at risk unless the boundary contract is made explicit.

## Architecture Check

1. The cleaner architecture is to define planner snapshot fidelity by domain contract: if planner logic depends on a component as semantic truth, the snapshot should preserve that component in semantically complete form, not hand-copy a few fields opportunistically.
2. This aligns with Principle 3 and Principle 25. Derived summaries are acceptable caches, but they must not silently become lower-fidelity truth for planner reasoning.
3. No backwards-compatibility aliasing or duplicate shadow office model should be introduced. The snapshot should remain a compressed view of the same belief-accessible world, not a parallel institution system.

## Verification Layers

1. Snapshot preserves semantically complete institutional domain data -> focused `planning_snapshot.rs` tests
2. `PlanningState` exposes the same domain data through `RuntimeBeliefView` as the snapshot preserved -> focused `planning_state.rs` tests
3. Live affordance surface and planning affordance surface agree for institution-driven actions -> focused parity test using `get_affordances()`
4. Golden E2E is not the primary proof surface; it only validates that the parity contract matters downstream
5. Additional authoritative world-state mapping is not primary because this ticket governs the planner-local compression boundary

## What to Change

### 1. Write a planner snapshot fidelity contract for semantic domain data

Document, in code comments and a short design note, when snapshot code may store derived fields versus when it must preserve a whole domain component. At minimum, cover:

- institutional domain data such as `OfficeData`
- record metadata when planner prerequisites depend on it
- any future domain component whose planner semantics depend on more than one field

### 2. Add explicit parity tests for live vs snapshot belief reads

Add focused tests that build a live `PerAgentBeliefView`, then build a `PlanningSnapshot`/`PlanningState`, and assert parity for the exact domain reads the planner depends on. Start with office/institutional data and add at least one non-office example if current code exposes another suitable semantic component.

### 3. Add affordance parity coverage for institution-driven actions

Add a focused test proving that if a live belief view exposes an institution-driven affordance such as `press_force_claim`, the corresponding `PlanningState` exposes the same affordance under the same local beliefs.

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `docs/FOUNDATIONS.md` (do not modify in this ticket; cite only)
- `docs/` design note or nearby planner documentation file (new or modify, as appropriate)

## Out of Scope

- Reworking planner search or action semantics
- Expanding snapshot preservation to unrelated heavy runtime caches that the planner does not read
- Office-law feature work itself

## Acceptance Criteria

### Tests That Must Pass

1. Focused test proves `PlanningState` returns semantically complete `OfficeData` when the live belief view exposes it
2. Focused parity test proves institution-driven affordances match between live belief view and planning state
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Planner snapshot fidelity rules must preserve semantic truth for planner-required domain data
2. Derived snapshot summaries may remain as caches, but must not become lower-fidelity replacements for components the planner reads semantically

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs` — add snapshot fidelity coverage for semantically complete institutional domain data
2. `crates/worldwake-ai/src/planning_state.rs` — add live-vs-snapshot parity coverage for domain reads and institution-driven affordances

### Commands

1. `cargo test -p worldwake-ai --lib planning_state::tests::planning_state_implements_goal_and_runtime_surfaces -- --exact`
2. `cargo test -p worldwake-ai --test golden_offices golden_force_claim_ai_installation -- --exact`
3. `cargo test -p worldwake-ai`
