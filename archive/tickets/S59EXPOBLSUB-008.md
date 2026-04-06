# S59EXPOBLSUB-008: ask_about_person action

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new action in worldwake-systems
**Deps**: S59EXPOBLSUB-002, S59EXPOBLSUB-005

## Problem

Agents searching for missing persons need to gather information from other agents. The `ask_about_person` action lets a searcher query a co-located agent's `LastSeenMemory` for sighting records, propagating information through the existing Tell mechanism rather than omniscient lookup.

## Assumption Reassessment (2026-04-06)

1. The live missing-person substrate does not have a stored `SearchTarget` carrier anywhere on this branch. `SearchTarget` exists only as a shared enum in `crates/worldwake-core/src/expectation.rs`, so this action cannot lawfully gate on “actor has a SearchTarget” as written.
2. `GoalBeliefView::expectation_store()` and `last_seen_memory()` already exist from `S59EXPOBLSUB-004`, and `PlannerOpKind::AskAboutPerson` is already reserved as a lawful operator for `GoalKind::SearchForMissing` in `crates/worldwake-ai/src/goal_model.rs` and `crates/worldwake-ai/src/goal_dispatch_decl.rs`.
3. The exact shared boundary under audit is the runtime epistemic-action surface for missing-person search: authoritative self `ExpectationStore` + `LastSeenMemory` reads, action payload/trace transport in `worldwake-sim`, and action registration in `worldwake-systems`.
4. `ask_witness` already exists in `crates/worldwake-systems/src/epistemic_actions.rs` and uses `AskWitnessMemory` to suppress repeated queries, but it transfers `AgentBeliefStore` entity snapshots rather than `LastSeenRecord`s. This ticket still owns a missing-person-specific runtime action instead of reusing generic `ask_witness` unchanged.
5. `TellTopic` does not currently have a last-seen-specific carrier; the existing `tell` action shares `AgentBeliefStore` snapshots, social observations, and institutional claims only. Extending Tell for `LastSeenRecord` transfer would widen scope beyond this ticket's honest runtime slice.
6. There is no existing stored “target has not seen the subject” carrier. The honest negative branch on this ticket is therefore “no `LastSeenMemory` update, but the ask-memory lane records that the target was queried.” Explicit negative-response memory becomes separate follow-up work.
7. Both actor and target still need the normal colocated alive-agent runtime checks, and target incapacitation should match the existing witness-query boundary.
8. Mismatch + correction: this ticket owns a direct `ask_about_person` action keyed from the actor's overdue expectations, with positive hearsay transfer into `LastSeenMemory` and duplicate-query suppression through the existing ask-memory lane. It does not depend on `SearchTarget`, extend `TellTopic`, or add a new negative-response memory carrier.

## Architecture Check

1. Follows the standard action pattern. Information propagation through Tell mechanism satisfies P15 (knowledge travels physically) and P7 (locality).
2. No backward compatibility shims.

## Verification Layers

1. Actor receives last-seen record from target -> action trace identity + authoritative world state (`LastSeenMemory` update with hearsay provenance)
2. Target without relevant records -> authoritative world state (`LastSeenMemory` unchanged) + ask-memory lane records the query to suppress repeats
3. Payload enumeration only uses overdue expectations and colocated agents -> focused runtime test on affordance payloads
4. Preconditions enforce colocated alive-agent boundary -> focused runtime/unit test
5. Single-layer ticket: candidate generation and planner admission are already reserved elsewhere, so additional decision-trace proof is not applicable here

## What to Change

### 1. Create ask_about_person action

Create `crates/worldwake-systems/src/ask_about_person_actions.rs`:

- Domain: `ActionDomain::Epistemic`
- Preconditions: Actor co-located with target agent. Actor has an overdue `ExpectationRecord` for the requested subject.
- Duration: Short (2-3 ticks, conversation action)
- Affordance payloads: enumerate `(target, subject)` pairs from the actor's overdue expectations and colocated agent targets, skipping self and already-asked memory lanes.
- on_commit: Read target's `LastSeenMemory` for the requested subject. If record exists, update actor's `LastSeenMemory` with a hearsay-transferred sighting, preserving the original observer and incrementing chain depth deterministically. If no record exists, leave `LastSeenMemory` unchanged and only record the query in the ask-memory lane.
- Affordance targets: co-located agents
- Query memory: reuse the existing `AskWitnessMemoryKey` lane keyed by `(target, subject)` so generic and missing-person witness questions do not diverge into duplicate suppression paths.

### 2. Register action

In `crates/worldwake-systems/src/action_registry.rs`, add `register_ask_about_person_action()` call and update the completeness test.

### 3. Extend shared runtime payload/trace transport

Add the new typed payload and action-trace detail in `worldwake-sim` so runtime traces can identify the queried target and missing subject without inventing a new bespoke commit-trace system.

## Files to Touch

- `crates/worldwake-systems/src/ask_about_person_actions.rs` (new)
- `crates/worldwake-sim/src/action_payload.rs` (modify — add ask_about_person payload)
- `crates/worldwake-sim/src/action_trace.rs` (modify — add ask_about_person trace detail)
- `crates/worldwake-sim/src/lib.rs` (modify — re-export payload type)
- `crates/worldwake-systems/src/lib.rs` (modify — add module)
- `crates/worldwake-systems/src/action_registry.rs` (modify — register + test)

## Out of Scope

- TellTopic extension or routing through the existing `tell` action
- Candidate generation for ask_about_person goal — deferred to future ticket if separate goal is needed
- Explicit stored memory that a queried target had no sighting to share
- Hearsay reliability discounting — agents receive the record with chain-depth tracking only

## Acceptance Criteria

### Tests That Must Pass

1. Actor's LastSeenMemory updated with target's sighting record when target has one
2. Provenance correctly set to Hearsay with incremented chain_depth
3. No information shared when target has no record for the subject, but the query memory lane is recorded
4. Affordance enumeration derives subjects from overdue expectations, not from a nonexistent SearchTarget carrier
5. Action rejected when actor and target not co-located
6. Action rejected when actor has no overdue expectation for the payload subject
7. Action registry completeness test includes "ask_about_person"
8. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Information travels through explicit interaction, not telepathy (P7, P15)
2. Hearsay chain_depth increments on each transmission
3. LastSeenMemory keeps the newest sighting per subject and evicts oldest records deterministically when over capacity
4. Duplicate question suppression stays on the existing ask-memory lane rather than introducing a second parallel query-memory path

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/ask_about_person_actions.rs` — unit tests for info sharing + preconditions
2. `crates/worldwake-sim/src/action_payload.rs` / `crates/worldwake-sim/src/action_trace.rs` — payload and trace identity coverage
3. `crates/worldwake-systems/src/action_registry.rs` — updated completeness test

### Commands

1. `cargo test -p worldwake-systems ask_about`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completed on 2026-04-06.
- Corrected the ticket before implementation: `SearchTarget` and Tell-based `LastSeenRecord` transfer were stale assumptions on the live branch, so the landed action keys directly from overdue expectations and transfers positive last-seen records through a dedicated runtime payload instead.
- Added `ask_about_person` in `crates/worldwake-systems/src/ask_about_person_actions.rs`, registered it in the systems action catalog, and wired shared payload/trace transport in `worldwake-sim`.
- Positive commits now copy the witness's `LastSeenRecord` into the actor's `LastSeenMemory` as hearsay with deterministic chain-depth handling and capacity eviction. Negative commits leave `LastSeenMemory` unchanged but record the query in the existing ask-memory lane.
- Created follow-up ticket `S59EXPOBLSUB-014` for the narrowed-out negative-response memory carrier so that work does not remain ownerless.

## Verification Result

- Passed `cargo test -p worldwake-systems ask_about_person`
- Passed `cargo test -p worldwake-sim action_trace`
- Passed `cargo test -p worldwake-sim action_payload`
- Passed `cargo test -p worldwake-systems`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
