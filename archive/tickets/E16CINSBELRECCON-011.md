# E16CINSBELRECCON-011: ConsultRecord Mid-Plan Prerequisite Integration For Political Goals

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planner/snapshot integration for consult-record prerequisites
**Deps**: E16CINSBELRECCON-001, E16CINSBELRECCON-005, E16CINSBELRECCON-010

## Problem

For agents to autonomously seek out institutional knowledge, the AI must be able to insert `ConsultRecord` as a lawful prerequisite step inside planning for existing political goals. The GOAP search must be able to plan multi-step sequences like `Travel to record place -> ConsultRecord -> Travel to jurisdiction -> PoliticalAction` when institutional beliefs are `Unknown`. S12's prerequisite-aware planning must integrate with this so political goals expose the record's home place as a prerequisite location.

## Assumption Reassessment (2026-03-22)

1. `GoalKind` in [`crates/worldwake-core/src/goal.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs) currently ends at `SupportCandidateForOffice`; there is no `ConsultRecord` goal variant.
2. `GoalKindTag` in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) also has no `ConsultRecord` tag.
3. `PlannerOpKind::ConsultRecord` already exists in [`crates/worldwake-ai/src/planner_ops.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs), and `classify_action_def()` already recognizes the registered `consult_record` action from ticket `-005`.
4. Mismatch + correction: this ticket should not add `GoalKind::ConsultRecord` or `GoalKindTag::ConsultRecord`. In this architecture, `ConsultRecord` is a means step, not a desired world condition. Adding a top-level consult goal would violate the repo rule that goals name world conditions while plans supply enabling substeps.
5. The correct scope is: make existing political goals (`ClaimOffice`, `SupportCandidateForOffice`) able to use `PlannerOpKind::ConsultRecord` mid-plan when their required institutional beliefs are `Unknown`.
6. `PlannerOpKind::ConsultRecord` currently has placeholder semantics: `may_appear_mid_plan: false`, `relevant_goal_kinds: &[]`, and no hypothetical effect. Those semantics prevent it from ever participating in search for any goal.
7. `PlanningSnapshot` and `PlanningState` currently capture office-holder and support-declaration belief reads, but they do not yet expose record contents or consultation timing to hypothetical planning. In particular, `PlanningState` does not implement `RuntimeBeliefView::record_data()` or `consultation_speed_factor()`, so consult durations and consult-state transitions cannot currently be simulated.
8. `GoalKind::ClaimOffice` and `GoalKind::SupportCandidateForOffice` currently expose only travel/bribe/threaten/declare-support op families. They do not include `ConsultRecord`, and their `prerequisite_places()` implementation is currently empty.
9. `apply_planner_step()` currently treats `PlannerOpKind::ConsultRecord` as a no-op. No hypothetical institutional belief is learned during search.
10. The live architectural gap is not "missing consult action registration" and not "missing consult goal family". It is "existing political goals cannot model consult-record as a prerequisite step because planner semantics, snapshot/state data, and political-goal prerequisite logic are incomplete."
11. `candidate_generation.rs` still relies on live office-holder/support declaration reads for political candidate emission. That larger belief-boundary migration remains outside this ticket except where search needs to consume already-captured institutional belief reads.
12. Ticket `-012` currently assumes `GoalKind::ConsultRecord` emission. If this ticket lands with the corrected architecture, `-012` will need follow-up reassessment before implementation.

## Architecture Check

1. The cleaner architecture is to keep `ConsultRecord` as a planner operation only. That matches Principle 18: goals are desired world conditions, while travel/consult/etc. are enabling steps.
2. Reusing `ClaimOffice` and `SupportCandidateForOffice` avoids introducing a second consult-specific goal family that ranking, policy, tracing, switching, and failure handling would all need to special-case.
3. S12 prerequisite integration should attach to the goal that is actually blocked. That keeps search explanations honest: the agent is trying to claim/support an office and consults records only because that goal currently lacks required knowledge.
4. The necessary extension is to enrich planning snapshot/state with consultable record data and consultation-speed reads, then apply a hypothetical institutional-belief override on consult. No alias path, no compatibility shim.

## Verification Layers

1. Consult action registration/classification -> `planner_ops` unit test on `classify_action_def` and semantics table
2. Snapshot/state support for consult duration + record reads -> `planning_snapshot` / `planning_state` focused unit tests
3. Unknown institutional belief -> record home place exposed as prerequisite guidance -> `goal_model` unit test
4. Hypothetical consult updates institutional belief reads inside planning only -> `goal_model` or `planning_state` focused unit test
5. Search produces `Travel -> ConsultRecord -> ... -> DeclareSupport` for a political goal when the blocking institutional belief is `Unknown` -> `search` integration test
6. Search does not require consult when belief is already `Certain` -> focused `search` or `goal_model` regression test

## What to Change

### 1. Upgrade `PlannerOpKind::ConsultRecord` in `planner_ops.rs`

`PlannerOpKind::ConsultRecord` and basic classification already exist. Upgrade its semantics in `semantics_for()` from registry-integrity placeholder behavior to planning behavior:
- `may_appear_mid_plan: true`
- `relevant_goal_kinds` includes the political goals that can be blocked on institutional knowledge today
- no alternate consult representation or planner-only alias path

### 2. Extend planning snapshot/state for consult reasoning

- Capture enough record data in `PlanningSnapshot` for consult planning to inspect record kind, home place, entries, and consult duration inputs
- Implement `RuntimeBeliefView::record_data()` and `consultation_speed_factor()` for `PlanningState`
- Reuse the existing office/support institutional belief override surface in `PlanningState` rather than introducing a second generic compatibility layer

### 3. Political goal integration in `goal_model.rs`

- Add `PlannerOpKind::ConsultRecord` to the relevant op families for `ClaimOffice` and `SupportCandidateForOffice`
- Build consult payload overrides from the chosen record target
- Apply hypothetical consult transitions by overriding the relevant institutional belief read from `Unknown` to the record-derived result
- Keep `ConsultRecord` a non-terminal planner step for these goals

### 4. S12 prerequisite integration

Extend `prerequisite_places()` to return the record's home place when:
- The current political goal requires institutional belief knowledge
- The actor's captured institutional belief is `Unknown`
- A matching record is present in the planning snapshot

### 5. Search integration

Ensure `search_plan()` can produce multi-step plans such as `Travel(to record place) -> ConsultRecord -> Travel(to jurisdiction) -> DeclareSupport` when the belief gap blocks the political step.

Hypothetical consult in search must update the existing planning-state institutional belief override surface so the subsequent political step becomes viable without mutating authoritative world state.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — upgrade existing `PlannerOpKind::ConsultRecord` semantics for real planning use)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — snapshot consultable record data and consultation inputs)
- `crates/worldwake-ai/src/planning_state.rs` (modify — expose record data / consultation speed to search and support consult belief overrides)
- `crates/worldwake-ai/src/goal_model.rs` (modify — political-goal consult op family, payload override, prerequisite places, hypothetical consult belief effect)
- `crates/worldwake-ai/src/search.rs` (modify if needed — only if search needs small integration beyond semantics/state support)

## Out of Scope

- Candidate generation reassessment for Unknown/Conflicted political beliefs (ticket -012)
- Ranking/failure-handling follow-up for institutional-belief-aware political behavior (ticket -013)
- Failure handling for stale/conflicted beliefs (ticket -013)
- Live helper seam removal (ticket -014)
- ConsultRecord action def/handler (ticket -005 — must already exist)

## Acceptance Criteria

### Tests That Must Pass

1. `classify_action_def` recognizes `consult_record` as `PlannerOpKind::ConsultRecord`
2. `PlannerOpKind::ConsultRecord` semantics has `may_appear_mid_plan: true`
3. Political goals include `ConsultRecord` in their relevant op families without introducing a new goal kind
4. `PlanningState` can supply consult duration inputs and record data during search
5. Hypothetical consult updates the relevant institutional belief read inside planning without mutating authoritative state
6. `prerequisite_places()` returns the record home place when the relevant institutional belief is `Unknown`
7. Search can produce a lawful political plan containing `ConsultRecord`
8. Existing focused AI tests plus `cargo test -p worldwake-ai` pass

### Invariants

1. `ConsultRecord` is a mid-plan prerequisite operation, not a top-level goal family
2. Search does not require consult when the relevant institutional belief is already `Certain`
3. Hypothetical consult mutates planning-only belief reads, never authoritative world state
4. S12 prerequisite chains remain finite and do not introduce consult loops

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — consult classification/semantics for political-goal relevance
2. `crates/worldwake-ai/src/planning_snapshot.rs` and/or `crates/worldwake-ai/src/planning_state.rs` — record-data + consultation-speed support and planning-only belief overrides
3. `crates/worldwake-ai/src/goal_model.rs` — political prerequisite places and hypothetical consult progression
4. `crates/worldwake-ai/src/search.rs` — multi-step political plan with consult prerequisite, plus no-consult regression when belief is already certain

### Commands

1. `cargo test -p worldwake-ai planner_ops::`
2. `cargo test -p worldwake-ai goal_model::`
3. `cargo test -p worldwake-ai planning_state::`
4. `cargo test -p worldwake-ai search::`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace`
7. `cargo test --workspace`

## Outcome

- Completed: 2026-03-22
- Corrected the ticket scope before implementation. The original implied direction of introducing a consult-specific goal shape was rejected after reassessing the code and E16c architecture. `ConsultRecord` remained a planner operation, and the implementation instead taught existing political goals to use it as a mid-plan prerequisite.
- `PlanningSnapshot` and `PlanningState` now carry consult-relevant record data and consultation-speed inputs so hypothetical search can reason about consult timing and consulted record contents.
- `PlannerOpKind::ConsultRecord` semantics now allow mid-plan use for `ClaimOffice` and `SupportCandidateForOffice`, and `goal_model` now applies planning-only institutional belief updates after consult so downstream political steps can become valid without mutating authoritative state.
- `prerequisite_places()` now exposes record home places for blocked political goals when the relevant vacancy belief is unknown and a matching office register exists in the snapshot.
- Search coverage now verifies both directions: consult is inserted when a political goal is blocked on unknown vacancy knowledge, and consult is skipped when vacancy belief is already certain.
- Implementation refinement versus original plan: political actions are only blocked on missing vacancy knowledge when a matching consultable office register is present in the planning snapshot. This preserves current non-record political scenarios while enabling the cleaner consult-driven path where institutional records exist.
- Verification completed:
  - `cargo test -p worldwake-ai planner_ops::`
  - `cargo test -p worldwake-ai goal_model::`
  - `cargo test -p worldwake-ai planning_state::`
  - `cargo test -p worldwake-ai search::`
  - `cargo test -p worldwake-ai --test golden_emergent`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
