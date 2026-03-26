# E17CRITHEJUS-008: Implement accuse action

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new action definition + handler in systems crate
**Deps**: E17CRITHEJUS-003 (needs `InstitutionalClaim::Accusation`, `RecordKind::CrimeRegister`), E17CRITHEJUS-015 (typed social-evidence detail), E17CRITHEJUS-016 (social-evidence relay through Tell)

## Problem

No mechanism exists for agents to formally accuse another agent of theft. The accusation system requires a new action that appends an `Accusation` entry to a `CrimeRegister` record entity, validated against the accuser's concrete evidence in their belief store.

## Assumption Reassessment (2026-03-25)

1. `consult_record_actions.rs` and `office_actions.rs` in worldwake-systems provide the closest structural precedent for actions that interact with institutional records.
2. `RecordData` entities have `append_entry()` for adding `InstitutionalRecordEntry` items. The CrimeRegister follows the same pattern.
3. `AgentBeliefStore` currently stores `SocialObservation`, but the live tuple shape cannot support accused-aware theft evidence reliably. `E17CRITHEJUS-015` must land first so accusation validation reads typed evidence detail instead of tuple conventions.
4. `ActionDomain::Social` exists and is used by Tell and office actions.
5. The accuse action is 1-tick (`Fixed(1)`) — filing an accusation is a brief administrative act.
6. N/A — no heuristic changes.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. Mismatch: the original ticket assumed `SocialObservation(SuspectedTheft)` could be matched directly against an accused agent. The live ticket set records theft observations as `(missing_entity, expected_place)` in `E17CRITHEJUS-007`, which is incompatible. Correct scope is to validate against typed theft evidence from `E17CRITHEJUS-015`, and to allow witness-reported evidence only after `E17CRITHEJUS-016` adds relayable social-evidence topics.
12. Follow-up architectural note: institutional beliefs still travel through Tell as sidecar data on `TellTopic::EntityBelief { subject: office_or_record }` rather than as first-class tell topics. This accuse handler must consume the actor's current belief surfaces without expanding that coupling; cleanup is tracked separately in `E17CRITHEJUS-017`.

## Architecture Check

1. A new section in `justice_actions.rs` (new module) follows the established per-domain module pattern. The accuse handler is structurally similar to `declare_support` (appends an institutional claim to a record entity at a specific place).
2. No backwards-compatibility aliasing. Evidence validation uses the accuser's belief store, not world truth — wrong accusations are architecturally possible (P14).

## Verification Layers

1. Accusation creates `InstitutionalClaim::Accusation` in CrimeRegister -> authoritative record state
2. Duplicate accusation rejected (same accused + same violation) -> start-failure
3. Accusation without evidence rejected -> start-failure
4. Accusation with typed theft evidence accepted -> event log delta
5. Wrong accusation possible when accuser has flawed evidence -> focused test proving P14

## What to Change

### 1. New `justice_actions.rs` module in worldwake-systems

- `register_accuse_action()` following established registration pattern.
- Action definition: name `"accuse"`, domain `ActionDomain::Social`, `TargetSpec::SpecificEntity` (the accused agent), `VisibilitySpec::SamePlace`, tags `[EventTag::Social, EventTag::Crime]`, duration `Fixed(1)`.

### 2. Start handler

Validate authoritatively:
- Actor at same place as a `CrimeRegister` record entity
- Actor's `AgentBeliefStore` contains concrete evidence: (a) typed theft evidence naming the accused, OR (b) witnessed crime event where perpetrator matches accused, OR (c) belief that the stolen item is in the accused's possession
- Accused entity is believed alive
- No existing unresolved `Accusation` against same accused for same violation in the CrimeRegister

### 3. Commit handler

- Append `InstitutionalClaim::Accusation { accuser, accused, violation_id, effective_tick }` to the CrimeRegister
- Emit event with `EventTag::Crime`, `VisibilitySpec::SamePlace`

### 4. Register and export

Wire into `register_all_actions()` in `action_registry.rs`. Export from `lib.rs`.

## Files to Touch

- `crates/worldwake-systems/src/justice_actions.rs` (new)
- `crates/worldwake-systems/src/action_registry.rs` (modify)
- `crates/worldwake-systems/src/lib.rs` (modify)

## Out of Scope

- Fine and Exile actions (E17CRITHEJUS-009)
- Steal action (E17CRITHEJUS-006)
- AI candidate generation for accusation (E17CRITHEJUS-011)
- CrimeRegister entity creation in world setup (handled in test_utils for now; production setup in a future integration ticket)
- Contest or appeal of accusations (future spec)
- Validating evidence accuracy against world truth (accusations use belief store — P12)
- Refactoring institutional Tell topics; handled by `E17CRITHEJUS-017`

## Acceptance Criteria

### Tests That Must Pass

1. Accusation creates `InstitutionalClaim::Accusation` entry in CrimeRegister
2. Accusation entry has correct `accuser`, `accused`, `violation_id`, `effective_tick`
3. Duplicate accusation (same accused + same violation) rejected at start
4. Accusation without any evidence in accuser's belief store rejected at start
5. Accusation with typed theft evidence matching accused succeeds
6. Accusation event emitted with `EventTag::Crime` and `VisibilitySpec::SamePlace`
7. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Evidence validation reads from `AgentBeliefStore`, never from world truth (P12)
2. Wrong accusations are possible — the handler does not verify evidence accuracy (P14)
3. CrimeRegister is append-only — accusation entries are never mutated
4. Accusation is a public act (`VisibilitySpec::SamePlace`) — co-located agents can witness it

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/justice_actions.rs` — focused tests: successful accusation, duplicate rejection, evidence-less rejection, event tags/visibility

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy -p worldwake-systems`
3. `cargo build --workspace`
