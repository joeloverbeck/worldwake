# E15RUMWITDIS-007: Tell Affordance Enumeration

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — affordance generation for Tell action in worldwake-systems
**Deps**: E15RUMWITDIS-005 (Tell ActionDef), E15RUMWITDIS-003 (TellProfile), E15RUMWITDIS-006 (Tell handler)

## Problem

The AI planner needs to know when a Tell action is available. The Tell affordance enumeration function must generate TellActionPayload candidates from the speaker's AgentBeliefStore — filtering by TellProfile.max_relay_chain_len and bounding by TellProfile.max_tell_candidates — for each co-located alive agent target. Without this, the AI cannot choose to tell anything.

## Assumption Reassessment (2026-03-14)

1. Affordance generation pattern: `get_affordances()` in `crates/worldwake-sim/src/affordance_query.rs` iterates action defs and calls each handler's `enumerate_affordances` callback. Confirmed.
2. Each action handler's affordance enumeration is done within the handler definition — see existing patterns in trade_actions, combat, travel_actions.
3. `AgentBeliefStore.known_entities` returns the BTreeMap of entity → BelievedEntityState. Keys are the set of entities the agent has beliefs about.
4. `TellProfile.max_relay_chain_len` limits which beliefs can be relayed (beliefs with source chain_len > max are not offered).
5. `TellProfile.max_tell_candidates` (default 3) limits the number of subjects offered per decision pass, selecting most recently observed beliefs.

## Architecture Check

1. Follows existing affordance enumeration pattern — no new framework needed.
2. The combinatorial explosion guard (max_tell_candidates=3) is spec-mandated. Without it, an agent with 10 beliefs and 5 co-located targets would generate 50 affordances.
3. Selection of "most recently observed beliefs" uses `observed_tick` ordering — deterministic via BTreeMap/sorting.
4. No backwards-compatibility shims.

## What to Change

### 1. Implement Tell affordance enumeration

In `crates/worldwake-systems/src/tell_actions.rs`, implement the affordance enumeration callback:

1. Get actor's `AgentBeliefStore` from belief view.
2. Get actor's `TellProfile` from belief view.
3. Filter `known_entities` keys by chain_len eligibility:
   - For each belief, check if source chain_len ≤ TellProfile.max_relay_chain_len
   - DirectObservation counts as chain_len 0
   - Report { chain_len: n } counts as chain_len n
   - Rumor { chain_len: n } counts as chain_len n
   - Inference counts as chain_len 0 (produces Rumor { chain_len: 1 })
4. Sort eligible beliefs by `observed_tick` descending (most recent first).
5. Take top `max_tell_candidates` subjects.
6. For each co-located alive agent target (excluding self):
   - For each eligible subject:
     - Generate `Affordance` with `TellActionPayload { listener: target, subject_entity: subject }`.

### 2. Wire affordance enumeration into Tell handler

Ensure the Tell action handler definition includes the affordance enumeration function in its handler struct.

## Files to Touch

- `crates/worldwake-systems/src/tell_actions.rs` (modify — add affordance enumeration)

## Out of Scope

- AI goal generation for Tell (deciding WHEN to tell — future work)
- Mismatch detection or discovery events
- belief_confidence() function
- Changes to affordance_query.rs framework
- Changes to other action affordance enumerations

## Acceptance Criteria

### Tests That Must Pass

1. Tell affordances generated for each co-located alive agent × eligible subject
2. Beliefs with chain_len > max_relay_chain_len are excluded from affordances
3. No more than max_tell_candidates subjects offered (most recent by observed_tick)
4. Speaker with max_relay_chain_len=1 does not generate affordances for Rumor beliefs
5. Speaker with no beliefs generates no Tell affordances
6. Dead targets excluded from affordances
7. Self excluded as target
8. Existing suite: `cargo test --workspace`
9. `cargo clippy --workspace`

### Invariants

1. Affordance count bounded by max_tell_candidates × co_located_agent_count
2. Deterministic ordering (BTreeMap + observed_tick sort ensures reproducibility)
3. No belief content leaks to non-listener agents through affordance enumeration

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/tell_actions.rs` — tests for affordance enumeration: verify correct subjects offered, chain_len filtering, candidate limiting, target filtering

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
