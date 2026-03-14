# E15RUMWITDIS-006: Implement Tell Action Handler (Commit Logic)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — Tell handler commit logic in worldwake-systems
**Deps**: E15RUMWITDIS-005 (Tell ActionDef + payload), E15RUMWITDIS-003 (TellProfile)

## Problem

The Tell action's commit handler must implement the core social belief transmission: speaker retrieves their belief about the subject, degrades the PerceptionSource chain, checks the listener's acceptance_fidelity, transfers the belief with preserved observed_tick, and applies newer-wins and memory capacity rules. This is the core E15 mechanism that makes information travel through explicit social channels.

## Assumption Reassessment (2026-03-14)

1. `build_believed_entity_state()` in `crates/worldwake-core/src/belief.rs` — confirmed. This is the canonical snapshot builder, but for Tell we are NOT re-projecting from world state. We are transferring the speaker's existing belief with degraded source. So Tell does NOT call build_believed_entity_state(); it reads the speaker's stored belief and creates a modified copy.
2. `AgentBeliefStore.update_entity()` uses newer-wins (by observed_tick) — confirmed in belief.rs.
3. `AgentBeliefStore.enforce_capacity()` takes PerceptionProfile and current tick — confirmed.
4. `DeterministicRng` is available in handler context for acceptance_fidelity check.
5. Action handler pattern: handlers have `on_start`, `on_tick`, `on_commit`, `on_abort` — confirmed from existing handlers (trade_actions, combat, etc.).
6. Commit conditions per spec: actor alive, listener still at actor's place.

## Architecture Check

1. Tell handler reads speaker's AgentBeliefStore (from world state), builds a degraded-source copy of the belief, writes to listener's AgentBeliefStore. This is purely state-mediated (Principle 24).
2. Source degradation is deterministic and follows spec exactly: DirectObservation→Report, Report→Rumor(n+1), Rumor→Rumor(n+1), Inference→Rumor(1).
3. The handler emits a commit event with VisibilitySpec::SamePlace and EventTag::Social + EventTag::WorldMutation.
4. Bystanders observe WitnessedTelling (handled by perception system via E15RUMWITDIS-002's social_kind mapping) but do NOT receive belief content.
5. No backwards-compatibility shims.

## What to Change

### 1. Implement Tell handler commit logic

In `crates/worldwake-systems/src/tell_actions.rs`, replace the stub commit handler with:

1. Extract `TellActionPayload` (listener, subject_entity) from action instance.
2. Read speaker's `AgentBeliefStore`, look up `subject_entity`.
3. If speaker has no belief about subject → abort cleanly (no error, just skip).
4. Compute degraded source:
   - `DirectObservation` → `Report { from: speaker_id, chain_len: 1 }`
   - `Report { from, chain_len: n }` → `Rumor { chain_len: n + 1 }`
   - `Rumor { chain_len: n }` → `Rumor { chain_len: n + 1 }`
   - `Inference` → `Rumor { chain_len: 1 }`
5. Read listener's `TellProfile`. Check `acceptance_fidelity` via RNG — if check fails, skip belief write.
6. Build transferred `BelievedEntityState`:
   - Copy all fields from speaker's belief
   - `observed_tick` = speaker's observed_tick (NOT current tick)
   - `source` = computed degraded source from step 4
7. Call listener's `AgentBeliefStore.update_entity()` — newer-wins applies.
8. Call listener's `enforce_capacity()` with their PerceptionProfile.
9. Write updated listener belief store back via WorldTxn.
10. Emit commit event with `VisibilitySpec::SamePlace`, tags `{Social, WorldMutation}`.

### 2. Implement commit precondition checks

Before commit logic:
- Verify actor is alive
- Verify listener is still at actor's place
- If either fails, abort cleanly

### 3. Source degradation helper

Add a pure helper function (can be in tell_actions.rs or belief.rs):

```rust
pub fn degrade_source(speaker: EntityId, source: &PerceptionSource) -> PerceptionSource { ... }
```

## Files to Touch

- `crates/worldwake-systems/src/tell_actions.rs` (modify — implement commit handler)
- `crates/worldwake-core/src/belief.rs` (modify — add `degrade_source()` helper if placed here)

## Out of Scope

- Tell affordance enumeration (E15RUMWITDIS-007)
- Mismatch detection or discovery events (E15RUMWITDIS-008, E15RUMWITDIS-009)
- AI goal generation for Tell (future work)
- belief_confidence() derivation function (E15RUMWITDIS-010)
- Bystander social observation recording (already handled by perception system + E15RUMWITDIS-002)

## Acceptance Criteria

### Tests That Must Pass

1. Tell transmits belief: speaker DirectObservation → listener gets Report { from: speaker, chain_len: 1 }
2. Tell chain degrades: speaker Report { chain_len: n } → listener Rumor { chain_len: n+1 }
3. Tell from Rumor degrades: speaker Rumor { chain_len: n } → listener Rumor { chain_len: n+1 }
4. Tell from Inference: listener gets Rumor { chain_len: 1 }
5. Tell aborts if speaker has no belief about subject
6. Tell requires co-location: aborts if listener moves mid-action
7. Tell preserves observed_tick (speaker's original, NOT current tick)
8. Newer-wins: listener keeps more recent belief, Tell does not overwrite
9. Memory capacity enforced after Tell (enforce_capacity called)
10. TellProfile.acceptance_fidelity: listener with acceptance_fidelity=0 never accepts told beliefs
11. Bystanders do NOT receive the belief content (only listener does)
12. Existing suite: `cargo test --workspace`
13. `cargo clippy --workspace`

### Invariants

1. Source degradation is deterministic for the same inputs
2. Tell never creates beliefs out of nothing — it only transfers/degrades existing beliefs
3. observed_tick is NEVER set to current tick — always the speaker's original observed_tick
4. Principle 24: no cross-system calls; Tell writes to AgentBeliefStore via WorldTxn
5. Conservation invariants unaffected (Tell does not move items)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/tell_actions.rs` — comprehensive test suite covering all 11 acceptance criteria behaviors. Each test sets up world state, starts/commits a Tell action, and verifies listener belief store state.

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace`
4. `cargo test --workspace`
