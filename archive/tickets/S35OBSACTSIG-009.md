# S35OBSACTSIG-009: Unify direct-local observation bookkeeping in perception

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-systems` perception pipeline internals
**Deps**: `archive/tickets/S35OBSACTSIG-003.md`, `specs/S35-observable-activity-signals.md`

## Problem

`observe_active_actions()` now works and is covered, but the perception pipeline still computes the same direct-local observation fact through multiple internal paths in `crates/worldwake-systems/src/perception.rs`.

Today:
- `observe_passive_local_entities()` decides which colocated entities were directly observed, builds snapshots, and emits missing-entity discoveries.
- `observe_active_actions()` then re-derives "who was directly observed locally this tick" by reading back belief-store entries with `observed_tick == tick` and `source == DirectObservation`.
- departure-driven activity clearing separately re-walks prior local beliefs and authoritative locations.

The current behavior is correct, but the architecture is not ideal. One observer-local fact should have one canonical internal transport path. If later S35 work adds more visible local signals, the current shape will encourage more read-back heuristics and more duplicated fidelity/absence logic.

## Assumption Reassessment (2026-03-29)

1. Focused coverage already exists for the current behavior in `crates/worldwake-systems/src/perception.rs`, including:
   - `co_located_active_action_sets_believed_activity`
   - `active_action_respects_observation_fidelity_gate`
   - `idle_colocated_subject_clears_believed_activity`
   - `departed_subject_clears_believed_activity_when_no_longer_colocated`
   - `active_action_does_not_cross_place_boundaries_or_self_observe`
   - multiple passive discovery tests such as `passive_observation_emits_discovery_for_missing_entity`
   This is not a missing-coverage bug ticket; it is an architecture cleanup ticket with behavior-preservation requirements.
2. `observe_passive_local_entities()` in `crates/worldwake-systems/src/perception.rs` is currently the canonical place where same-place direct observation rolls fidelity, builds `BelievedEntityState` snapshots, calls `record_observed_snapshot(...)`, and computes missing-entity discoveries.
3. `observe_active_actions()` in the same file currently infers "directly observed locally this tick" by scanning the already-updated belief store for entries whose `observed_tick == tick` and `source == PerceptionSource::DirectObservation`. That is a second internal transport path for the same direct-local observation fact.
4. Shared abstraction boundary under audit: one observer-local direct observation cycle at `(observer, place, tick)` should canonically produce the set of directly observed local subjects and the set of directly noticed missing prior-local subjects, and downstream perception substeps should consume that result rather than reconstructing it from belief state.
5. Information-path analysis: the same fact currently travels through two lawful internal paths inside one system step:
   - direct path: local observation loop -> snapshot/discovery writes
   - read-back path: belief-store writes -> `observe_active_actions()` infers who must have been locally observed
   Canonical end-state after this ticket: one ephemeral direct-local observation batch is produced once and consumed by passive snapshot update, missing-entity discovery, and activity overlay/clearing. The read-back path is removed in-scope.
6. This is a single-system refactor in `worldwake-systems`; it should not move responsibility into `worldwake-core`, `worldwake-sim`, or `worldwake-ai`.
7. `docs/FOUNDATIONS.md` requires clean, robust, extensible architecture with no workaround paths. Re-reading belief writes to infer what the observer saw is workable, but it is still a workaround path rather than the clean source-of-truth path.
8. Adjacent S35 tickets do not own this cleanup:
   - `tickets/S35OBSACTSIG-004.md` is belief-view query surface
   - `tickets/S35OBSACTSIG-005.md` is trace data shape
   - `tickets/S35OBSACTSIG-006.md` is ranking arithmetic
   - `tickets/S35OBSACTSIG-007.md` is golden proof
   Those tickets should consume the perception contract, not reshape it.
9. Focused-test reassessment: the named focused perception tests do exist in `crates/worldwake-systems/src/perception.rs` under the real `perception::tests::*` path, and the narrow command surface is `cargo test -p worldwake-systems --lib perception::tests::...`. However, current coverage is still split across separate tests for missing-entity discovery and departure-driven activity clearing; there is not yet one focused test proving both outcomes come from the same observer-local direct observation cycle. Adding that proof remains in-scope for this ticket.
10. Mismatch + correction: the remaining open S35 tickets are not the right place to absorb this cleanup. If pursued, it needs its own perception-scoped ticket to avoid mixed-layer scope creep.

## Architecture Check

1. A single ephemeral direct-local observation batch is cleaner than inferring observation results by re-reading belief state after writes. It gives one canonical internal path for "what the observer directly saw at this place on this tick."
2. The cleanup aligns with `docs/FOUNDATIONS.md`:
   - P3 / concrete state: the batch carries concrete observed subjects and noticed absences, not abstract flags.
   - P7 / locality: the batch is explicitly observer-place scoped.
   - P24 / systems through state: no new cross-system calls are introduced; this remains a perception-internal refactor over existing state inputs.
   - P25 / derived summaries are caches, never truth: the batch is ephemeral and not stored as authoritative or durable world state.
3. This is more robust than merging scheduler activity into `ObservedEntitySnapshot` or `build_believed_entity_state(...)`. Core belief snapshots should stay scheduler-agnostic; the batch belongs in perception internals.
4. No backwards-compatibility aliasing or shim path is needed. The goal is to remove an internal duplicate path, not preserve it.

## Verification Layers

1. Canonical observer-local subject set drives passive belief refresh and activity overlay together -> focused `worldwake-systems` perception tests.
2. Canonical noticed-absence set drives both missing-entity discovery and activity clearing for departed subjects -> focused `worldwake-systems` perception tests plus event-log discovery assertions.
3. No behavior regression in perception crate -> `cargo test -p worldwake-systems`.
4. No workspace regression from perception refactor -> `cargo test --workspace` and `cargo clippy --workspace`.
5. Additional AI/action-trace layer mapping is not applicable because this ticket intentionally preserves external behavior and only cleans up one authoritative perception-system boundary.

## What to Change

### 1. Introduce one canonical direct-local observation batch

In `crates/worldwake-systems/src/perception.rs`, add an internal ephemeral structure and helper for one observer-local direct observation cycle. It should capture, at minimum:
- observer place for this cycle
- directly observed colocated subjects for this tick
- built direct-observation snapshots (or an equivalent canonical subject set plus snapshot data)
- prior-local subjects whose absence was directly noticed this tick

The exact field layout is flexible, but the architecture requirement is not: downstream substeps must consume this batch instead of reconstructing the same fact from belief writes.

### 2. Refactor passive local observation to produce and consume the batch

Restructure the passive/local portion of perception so that:
- direct observation rolls happen once per observer-subject or observer-prior-local-subject decision
- `record_observed_snapshot(...)` consumes the batch’s observed snapshot data
- missing-entity discovery emission consumes the batch’s noticed-absence set

Keep current behavior unchanged from the caller’s point of view.

### 3. Refactor activity overlay/clearing to consume the same batch

Change `observe_active_actions()` so it no longer scans belief entries for `observed_tick == tick && source == DirectObservation`.

Instead:
- activity projection for present colocated subjects uses the canonical directly observed subject set from the batch
- idle clearing for present directly observed subjects with no active action uses that same set
- departure clearing for previously local subjects uses the batch’s noticed-absence set

### 4. Preserve perception’s current external contract

This ticket must not:
- change `BelievedActivity` shape
- change `GoalBeliefView`
- change ranking arithmetic
- move activity into `ObservedEntitySnapshot`
- introduce durable batch state or new components

The visible behavior should remain the same; only the internal transport path becomes canonical.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify)

## Out of Scope

- `GoalBeliefView` or `RuntimeBeliefView` changes
- AI ranking discount changes
- decision-trace or action-trace extensions
- save/load work
- moving scheduler concepts into `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. Existing active-activity focused tests in `crates/worldwake-systems/src/perception.rs` still pass unchanged or with only assertion-surface-preserving edits.
2. A focused perception test proves that a departed previously local subject both emits/retains the expected missing-entity discovery behavior and clears `believed_activity` from the same canonical local observation cycle.
3. Existing suite: `cargo test -p worldwake-systems`, `cargo test --workspace`, and `cargo clippy --workspace`.

### Invariants

1. One observer-local direct observation cycle has one canonical internal result. `observe_active_actions()` no longer infers direct observation by reading back belief-store writes.
2. No new stored state, component, alias enum, or compatibility path is introduced.
3. The canonical path for this fact after the change is:
   direct local observation batch -> passive belief update / missing discovery / activity overlay-clearing.
4. Perception remains scheduler-agnostic at the core belief snapshot boundary: scheduler activity is still layered in perception, not embedded into `ObservedEntitySnapshot`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` — extend focused departure/missing-discovery coverage so one test proves the shared canonical local observation cycle still clears `believed_activity` and preserves the expected discovery behavior.
2. `crates/worldwake-systems/src/perception.rs` — keep existing active-activity and passive-local focused tests as regression proof for the refactor boundary.

### Commands

1. `cargo test -p worldwake-systems --lib perception::tests::`
2. `cargo test -p worldwake-systems`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-29
- What actually changed:
  - Added a perception-internal `DirectLocalObservationBatch` in `crates/worldwake-systems/src/perception.rs` so one observer-local direct observation cycle now canonically produces directly observed snapshots and directly noticed missing subjects.
  - Refactored passive same-place perception to collect that batch once, apply snapshot updates from it, and emit missing-entity discoveries from it.
  - Refactored `observe_active_actions()` to consume the same batch for activity projection, idle clearing, and departure-driven clearing instead of re-deriving direct observation by scanning belief writes.
  - Added focused regression coverage proving one departure observation cycle both clears stale `believed_activity` and emits the expected missing-entity discovery.
- Deviations from original plan:
  - The refactor stayed entirely inside `crates/worldwake-systems/src/perception.rs`; no additional helper modules or cross-crate contract changes were needed.
  - The concrete batch stores `BelievedEntityState` snapshots plus missing-subject ids rather than a more abstract subject list, because the passive snapshot writer already needs the full snapshot payload and this kept the refactor smaller.
- Verification results:
  - `cargo test -p worldwake-systems --lib perception::tests::` passed.
  - `cargo test -p worldwake-systems` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace` passed.
