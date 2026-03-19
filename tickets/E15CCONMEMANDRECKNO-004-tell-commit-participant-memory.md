# E15CCONMEMANDRECKNO-004: Tell Commit Participant Memory

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — authoritative Tell commit writes speaker/listener conversation memory
**Deps**: `E15CCONMEMANDRECKNO-001`

## Problem

The current `crates/worldwake-systems/src/tell_actions.rs::commit_tell()` only updates the listener’s `known_entities` on successful acceptance. It never records what the speaker remembers having told, never records what the listener remembers hearing, and has no disposition surface for accepted versus ignored tells. Without that, E15c has no authoritative social-memory source of truth.

## Assumption Reassessment (2026-03-19)

1. `commit_tell()` currently returns early on failed acceptance fidelity and writes nothing at all for the listener in that branch.
2. Existing focused coverage proves only the old behavior: `tell_actions::tests::tell_commit_respects_listener_acceptance_fidelity`, `tell_actions::tests::tell_commit_keeps_listener_newer_belief`, and `tell_actions::tests::tell_commit_transfers_direct_observation_as_report_and_preserves_tick`.
3. There is no current test for speaker-side remembered tells, listener-side remembered hears, or disposition values.
4. The E15c spec requires conversation-memory writes and belief-store writes to happen in the same `WorldTxn`; this is authoritative world-state ordering, not merely event-log ordering.
5. `Rejected` is reserved for concrete future trust/contradiction paths. This ticket should not invent omniscient rejection logic if none exists in current code.

## Architecture Check

1. `commit_tell()` is the correct and only authoritative mutation point because Tell is already the explicit social action that transmits knowledge.
2. Writing speaker and listener memory in the same transaction is cleaner than deriving conversation memory later from event log replay or AI-side heuristics.

## Verification Layers

1. Speaker and listener memory records are written in the same authoritative mutation as Tell commit -> focused runtime/action test in `tell_actions.rs`
2. Accepted vs newer-vs-not-internalized dispositions are represented correctly -> focused unit tests in `tell_actions.rs`
3. Later goldens are out of scope; this ticket proves commit semantics directly.

## What to Change

### 1. Record speaker-side told memory

When Tell commits, record `ToldBeliefMemory` for `(listener, subject)` using the speaker’s shareable belief snapshot.

### 2. Record listener-side heard memory

Write `HeardBeliefMemory` for `(speaker, subject)` with `Accepted`, `AlreadyHeldEqualOrNewer`, or `NotInternalized` as dictated by the concrete branch.

### 3. Keep mutation ordering atomic

Apply conversation-memory writes, optional `known_entities` update, and conversation-memory retention maintenance inside the same authoritative transaction.

## Files to Touch

- `crates/worldwake-systems/src/tell_actions.rs` (modify)

## Out of Scope

- Listener-aware affordance filtering
- AI candidate generation
- Decision-trace omission diagnostics
- Adding trust-based explicit rejection logic without an existing substrate

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-systems tell_actions::tests::tell_commit_records_speaker_told_belief_memory`
2. `cargo test -p worldwake-systems tell_actions::tests::tell_commit_records_listener_heard_belief_with_accepted_disposition`
3. `cargo test -p worldwake-systems tell_actions::tests::tell_commit_records_listener_heard_belief_with_already_held_equal_or_newer`
4. `cargo test -p worldwake-systems tell_actions::tests::tell_commit_records_listener_heard_belief_with_not_internalized`
5. Existing suite: `cargo test -p worldwake-systems tell_actions::tests::tell_commit_keeps_listener_newer_belief`

### Invariants

1. Speaker memory is derived from the speaker’s own belief content, not from later listener state.
2. Listener memory records hearing even when the tell is not internalized into `known_entities`.
3. No branch reads the listener’s live belief store on behalf of the speaker.
4. Tell commit remains deterministic and same-place lawful.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/tell_actions.rs` — add focused commit-semantic tests for speaker memory and heard dispositions.
2. `crates/worldwake-systems/src/tell_actions.rs` — strengthen existing acceptance-fidelity and newer-belief tests to assert memory side effects.
3. `None — broader golden coverage is deferred to E15CCONMEMANDRECKNO-006.`

### Commands

1. `cargo test -p worldwake-systems tell_actions::tests::tell_commit_records_speaker_told_belief_memory`
2. `cargo test -p worldwake-systems tell_actions::tests::tell_commit_records_listener_heard_belief_with_not_internalized`
3. `cargo test -p worldwake-systems`
