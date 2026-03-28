# S34GENEPIACT-004: ask_witness action handler — definition, registration, start/tick/commit/abort

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-systems: add ask_witness handler to epistemic_actions.rs
**Deps**: S34GENEPIACT-001 (core types), S34GENEPIACT-002 (payload types), S34GENEPIACT-003 (epistemic_actions.rs module exists)

## Problem

Agents cannot query other co-located agents for information. The `ask_witness` action does not exist. Without it, social information gathering is limited to passive perception and one-directional Tell. Agents have no mechanism to deliberately seek specific knowledge from witnesses.

## Assumption Reassessment (2026-03-28)

1. E15c conversation memory (`ToldBeliefMemory`/`HeardBeliefMemory`) is completed and provides the deduplication infrastructure that `ask_witness` reuses. The Tell action handler in the codebase demonstrates the pattern for two-agent social interactions.
2. The spec requires `ask_witness` to record `HeardBeliefMemory` (asker) and `ToldBeliefMemory` (target) on commit, reusing the E15c `TellMemoryKey` / `enforce_conversation_memory` infrastructure. No separate `AskMemory` system.
3. `AskWitnessPayload` has `target: EntityId`, `topic_entity: Option<EntityId>`, `topic_commodity: Option<CommodityKind>`. Validation must reject payloads where both topic fields are `None`.
4. The handler transfers a subset of the target's `AgentBeliefStore.known_entities` entries to the actor, filtered by topic, with `PerceptionSource::Report { from: target, chain_len: 1 }` provenance.
5. If the target has no relevant beliefs, the commit still succeeds — the absence of information is a result (the agent learns the witness does not know).
6. The handler must check conversation memory to suppress re-asking the same target about the same topic within the retention window. This is the `ask_memory_retention_ticks` field on `VerificationDispositionProfile`.
7. `validate_investigate_payload_authoritatively()` pattern (actor alive, not incapacitated, target alive, co-located, not incapacitated) applies here for the authoritative validator.

## Architecture Check

1. Adding `ask_witness` to the existing `epistemic_actions.rs` (created in ticket 003) keeps all epistemic handlers together. This mirrors how `investigate_actions.rs` contains both investigate and related justice handlers.
2. Reusing `ToldBeliefMemory`/`HeardBeliefMemory` from E15c avoids dual representation (P26). The ask-witness exchange produces the same memory artifacts as telling.

## Verification Layers

1. ask_witness transfers beliefs -> focused handler test: actor gains target's belief with `Report { chain_len: 1 }` provenance
2. ask_witness no-op when target has no relevant beliefs -> focused handler test: commit succeeds, actor belief store unchanged for that topic
3. ask_witness respects conversation memory -> focused handler test: re-ask within retention window is rejected by affordance enumerator
4. ask_witness rejects invalid payload -> focused handler test: both topic fields `None` -> authoritative validation failure
5. ask_witness abort on target movement -> focused handler test: no beliefs transferred, no memory recorded
6. ask_witness records memory entries -> focused handler test: both `HeardBeliefMemory` (asker) and `ToldBeliefMemory` (target) exist after commit

## What to Change

### 1. Add ask_witness handler to epistemic_actions.rs

In `crates/worldwake-systems/src/epistemic_actions.rs`, add:

- `register_ask_witness_action(defs, handlers)`: Registers `ActionDef` with name `"ask_witness"`, domain `ActionDomain::Epistemic`, `FreelyInterruptible`, visibility `SamePlace`. Duration from `VerificationDispositionProfile::witness_query_duration_ticks`.
- `enumerate_ask_witness_payloads(world, actor, defs)`: Returns payloads for co-located agents the actor could ask about specific topics. Checks conversation memory to suppress recently-asked targets. Validates at least one topic field is populated.
- `validate_ask_witness_payload_authoritatively(world, actor, payload)`: Actor alive, not incapacitated. Target alive, co-located with actor, not incapacitated. At least one of `topic_entity`/`topic_commodity` is `Some`.
- `start_ask_witness(world_txn, instance)`: Validates preconditions, starts the two-agent interaction.
- `tick_ask_witness(world_txn, instance)`: Standard duration tick-down returning `ActionProgress`.
- `commit_ask_witness(world_txn, instance, event_log)`:
  - Read target's `AgentBeliefStore.known_entities` entries filtered by topic.
  - For each matching entry, write to actor's belief store with `PerceptionSource::Report { from: target, chain_len: 1 }`.
  - Record `HeardBeliefMemory` entry for the asker with `TellMemoryKey { counterparty: target, topic }`.
  - Record `ToldBeliefMemory` entry for the target with the same key structure.
  - If target has no relevant beliefs, commit still succeeds (no beliefs transferred, memory entries still recorded).
- `abort_ask_witness(world_txn, instance)`: No-op — no beliefs transferred, no memory entries recorded. Spent ticks consumed.

### 2. Register in action_registry.rs

Add `register_ask_witness_action(&mut defs, &mut handlers);` call in `register_all_actions()`.

Add `"ask_witness"` to the required action names list in the catalog test.

## Files to Touch

- `crates/worldwake-systems/src/epistemic_actions.rs` (modify — add ask_witness handler)
- `crates/worldwake-systems/src/action_registry.rs` (modify — add registration call + test name)

## Out of Scope

- `verify_belief` handler — ticket 003
- Planner ops (`PlannerOpKind::AskWitness`) — ticket 005
- Candidate generation (conversation memory suppression in `emit_verify_belief_goals`) — ticket 006
- Ranking integration — ticket 007
- Golden E2E tests — ticket 008
- Changes to `ToldBeliefMemory`/`HeardBeliefMemory` structures (E15c provides these)
- Changes to `AgentBeliefStore` (E14 provides this)
- Affordance payload enumeration for `verify_belief` — already in ticket 003

## Acceptance Criteria

### Tests That Must Pass

1. `ask_witness` transfers target's beliefs with `Report { chain_len: 1 }` provenance
2. `ask_witness` commits successfully when target has no relevant beliefs (no-op transfer)
3. `ask_witness` respects conversation memory (no re-ask within retention window via affordance enumeration)
4. `ask_witness` rejects payload where both `topic_entity` and `topic_commodity` are `None`
5. `ask_witness` aborts if target moves away during action; no beliefs transferred, no memory recorded
6. `ask_witness` records `HeardBeliefMemory` (asker) and `ToldBeliefMemory` (target) on commit
7. `ask_witness` aborts if target dies during action; no beliefs transferred, no memory recorded
8. `ask_witness` aborts if target becomes incapacitated during action; no beliefs transferred, no memory recorded
9. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Epistemic actions update belief stores, never authoritative world state beyond belief store and conversation memory (P12)
2. `ask_witness` occupies both actor and target for the action duration (same occupancy model as Tell — P8)
3. Conversation memory reuses E15c `ToldBeliefMemory`/`HeardBeliefMemory` — no separate ask memory system (P26)
4. Conservation invariant unaffected
5. Determinism — handler uses no `HashMap`/`HashSet`, no floats, no wall-clock time

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/epistemic_actions.rs` (in-module tests) — 8 focused handler tests per spec test list items 7-12
2. `crates/worldwake-systems/src/action_registry.rs` (modify existing test) — add "ask_witness" to required names

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy -p worldwake-systems`
3. `cargo build --workspace`
