# S59EXPOBLSUB-008: ask_about_person action

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new action in worldwake-systems
**Deps**: S59EXPOBLSUB-002, S59EXPOBLSUB-005

## Problem

Agents searching for missing persons need to gather information from other agents. The `ask_about_person` action lets a searcher query a co-located agent's `LastSeenMemory` for sighting records, propagating information through the existing Tell mechanism rather than omniscient lookup.

## Assumption Reassessment (2026-04-06)

1. Tell action at `crates/worldwake-systems/src/tell_actions.rs` provides the information-sharing pattern. `ask_about_person` is conceptually the inverse: instead of volunteering information, the actor solicits it.
2. `LastSeenMemory` component (from ticket 002) provides the per-agent sighting records queried by this action.
3. `ActionDomain::Epistemic` exists at `crates/worldwake-core/src/action_domain.rs:10`.
4. `TellTopic` enum at `crates/worldwake-core/src/tell.rs` may need extension for last-seen information, or the action can use the existing Tell mechanism with a new topic variant.
5. Both actor and target must be co-located (same place), alive, and not incapacitated — standard preconditions matching `tell_action_def()`.

## Architecture Check

1. Follows the standard action pattern. Information propagation through Tell mechanism satisfies P15 (knowledge travels physically) and P7 (locality).
2. No backward compatibility shims.

## Verification Layers

1. Actor receives last-seen record from target → action trace + authoritative world state (actor's LastSeenMemory updated)
2. Target without relevant records → no information shared → action trace shows empty result
3. Preconditions enforce co-location → focused unit test

## What to Change

### 1. Create ask_about_person action

Create `crates/worldwake-systems/src/ask_about_person_actions.rs`:

- Domain: `ActionDomain::Epistemic`
- Preconditions: Actor co-located with target agent. Actor has a SearchTarget (missing entity).
- Duration: Short (2-3 ticks, conversation action)
- on_commit: Read target's `LastSeenMemory` for the search subject. If record exists, update actor's `LastSeenMemory` with the sighting (provenance: Hearsay with chain_depth incremented). If no record, actor gains knowledge that target hasn't seen the subject.
- Affordance targets: co-located agents
- Affordance payloads: enumerate from active search targets

### 2. Register action

In `crates/worldwake-systems/src/action_registry.rs`, add `register_ask_about_person_action()` call and update the completeness test.

## Files to Touch

- `crates/worldwake-systems/src/ask_about_person_actions.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify — add module)
- `crates/worldwake-systems/src/action_registry.rs` (modify — register + test)

## Out of Scope

- TellTopic extension (if needed) — assess during implementation; may reuse existing topics
- Candidate generation for ask_about_person goal — deferred to future ticket if separate goal is needed
- Hearsay reliability discounting — agents receive the record as-is with chain_depth tracking

## Acceptance Criteria

### Tests That Must Pass

1. Actor's LastSeenMemory updated with target's sighting record when target has one
2. Provenance correctly set to Hearsay with incremented chain_depth
3. No information shared when target has no record for the subject
4. Action rejected when actor and target not co-located
5. Action rejected when actor has no active SearchTarget
6. Action registry completeness test includes "ask_about_person"
7. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Information travels through explicit interaction, not telepathy (P7, P15)
2. Hearsay chain_depth increments on each transmission
3. LastSeenMemory respects capacity bounds (evict oldest if full)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/ask_about_person_actions.rs` — unit tests for info sharing + preconditions
2. `crates/worldwake-systems/src/action_registry.rs` — updated completeness test

### Commands

1. `cargo test -p worldwake-systems ask_about`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
