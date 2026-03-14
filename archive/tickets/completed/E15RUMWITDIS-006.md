# E15RUMWITDIS-006: Implement Tell Action Handler Commit Logic

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — Tell commit behavior in `worldwake-systems`
**Deps**: E15RUMWITDIS-003 (TellProfile), E15RUMWITDIS-005 (Tell ActionDef + payload)

## Problem

`crates/worldwake-systems/src/tell_actions.rs` already defines the Tell action, payload validation, relay-limit validation, and start/tick hooks, but `commit_tell()` is still a stub returning `CommitOutcome::empty()`. That leaves the E15 social transfer path non-functional even though the surrounding action plumbing has already landed.

This ticket is only about the missing authoritative commit behavior: read the speaker's current belief, apply Tell-specific source degradation, probabilistically accept or reject on the listener, update the listener belief store under existing newer-wins/capacity rules, and rely on the existing action scheduler transaction path to emit the commit event.

## Assumption Reassessment (2026-03-14)

1. `TellProfile`, `ActionDomain::Social`, `EventTag::Social`, `SocialObservationKind::WitnessedTelling`, and `TellActionPayload` are already implemented. This ticket must not re-add or redesign them.
2. `register_tell_action()` and `tell_action_def()` are already present in `crates/worldwake-systems/src/tell_actions.rs`, including the fixed 2-tick duration, same-place visibility, and social/world-mutation tags.
3. `validate_tell_payload_authoritatively()` already enforces listener/target alignment, self-target rejection, subject-belief existence at action start, and relay-limit filtering from the speaker's `TellProfile`.
4. `commit_tell()` is still a stub. That is the actual missing implementation.
5. `build_believed_entity_state()` in `crates/worldwake-core/src/belief.rs` is not part of Tell commit. Tell transfers a stored `BelievedEntityState`; it does not rebuild one from authoritative world state.
6. `AgentBeliefStore.update_entity()` already enforces newer-wins by `observed_tick`, and `AgentBeliefStore.enforce_capacity()` already performs retention/capacity trimming using `PerceptionProfile`.
7. `DeterministicRng` is available in the handler and already exposes the primitives needed for a deterministic `Permille` acceptance check.
8. Action commit events are emitted by the scheduler after `on_commit()` returns successfully. The Tell handler should stage world mutations on `WorldTxn`; it does not need to manually emit its own event record.
9. The action definition already carries commit conditions for actor alive, target existence, target co-location, target kind, and target alive. The handler should still defend against mutable belief/profile state that can change during the 2-tick action window.

## Architecture Check

1. The Tell handler should remain a pure state-mediated bridge between two `AgentBeliefStore` components. No direct cross-system calls. This aligns with [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md) Principle 12's world/belief separation and the repo rule that systems interact through state, not through each other.
2. Tell-specific source degradation is domain logic for this action. Unless a second caller appears, keep the helper local to `tell_actions.rs` instead of broadening `worldwake-core` with a single-use API.
3. The scheduler already appends `EventTag::ActionCommitted` plus the Tell action definition's `Social` and `WorldMutation` tags when the transaction commits. The handler should stage component writes and targets; it should not manually emit a duplicate event.
4. Bystander social observation is already handled by the perception system's `social_kind()` mapping for `EventTag::Social`. The Tell handler should not attempt to write bystander belief content.
5. No backward-compatibility aliases or shadow paths. The commit logic should define the single authoritative Tell transfer behavior.

## What to Change

### 1. Implement Tell handler commit logic

In `crates/worldwake-systems/src/tell_actions.rs`, replace the stub `commit_tell()` with logic that:

1. Extract `TellActionPayload` (listener, subject_entity) from action instance.
2. Reads the speaker's current `AgentBeliefStore` and current belief for `subject_entity`.
3. If the speaker no longer has a belief about the subject at commit time, exits cleanly without writing listener state.
4. Rechecks the speaker's current relay limit against the current belief chain depth before transfer. This keeps the authoritative write path aligned with the actual commit-time belief, not just action-start validation.
5. Computes Tell-specific degraded source:
   - `DirectObservation` → `Report { from: speaker_id, chain_len: 1 }`
   - `Report { from, chain_len: n }` → `Rumor { chain_len: n + 1 }`
   - `Rumor { chain_len: n }` → `Rumor { chain_len: n + 1 }`
   - `Inference` → `Rumor { chain_len: 1 }`
6. Reads listener `TellProfile` and performs a deterministic `acceptance_fidelity` check.
7. If the listener rejects the report, exits cleanly without mutating the listener belief store.
8. Builds the transferred `BelievedEntityState`:
   - Copy all fields from speaker's belief
   - `observed_tick` = speaker's observed_tick (NOT current tick)
   - `source` = degraded source from step 5
9. Applies `update_entity()` to the listener store so existing newer-wins behavior remains the only overwrite rule.
10. Applies `enforce_capacity()` using the listener's `PerceptionProfile`.
11. Writes the updated listener store through `WorldTxn`.

### 2. Add small Tell-local helpers

Add small pure helpers inside `tell_actions.rs` for:

```rust
fn degrade_source(speaker: EntityId, source: PerceptionSource) -> PerceptionSource { ... }
fn passes_acceptance_check(fidelity: u16, rng: &mut DeterministicRng) -> bool { ... }
```

Do not move this into `worldwake-core` unless a second non-Tell caller appears.

## Files to Touch

- `crates/worldwake-systems/src/tell_actions.rs` (modify)

## Out of Scope

- Re-implementing Tell action definition, payload types, TellProfile, Social tags, or social observation kinds that already exist
- Tell affordance enumeration (E15RUMWITDIS-007)
- Mismatch detection or discovery events (E15RUMWITDIS-008, E15RUMWITDIS-009)
- AI goal generation for Tell (future work)
- `belief_confidence()` derivation function (E15RUMWITDIS-010)
- Any broader refactor of belief infrastructure beyond what the commit path strictly needs

## Acceptance Criteria

### Tests That Must Pass

1. Tell transmits belief: speaker DirectObservation → listener gets Report { from: speaker, chain_len: 1 }
2. Tell chain degrades: speaker Report { chain_len: n } → listener Rumor { chain_len: n+1 }
3. Tell from Rumor degrades: speaker Rumor { chain_len: n } → listener Rumor { chain_len: n+1 }
4. Tell from Inference: listener gets Rumor { chain_len: 1 }
5. Tell aborts if speaker has no belief about subject
6. Tell does not commit listener state if listener rejects the transfer via `acceptance_fidelity`
7. Tell preserves observed_tick (speaker's original, NOT current tick)
8. Newer-wins: listener keeps more recent belief, Tell does not overwrite
9. Memory capacity enforced after Tell (enforce_capacity called)
10. TellProfile.acceptance_fidelity: listener with `acceptance_fidelity = 0` never accepts told beliefs
11. Tell rechecks relayability at commit time and does not write beliefs that exceed the speaker's current relay limit
12. Tell action still emits a single scheduler-driven commit event with same-place visibility and Tell causal tags
13. Existing suite: `cargo test --workspace`
14. `cargo clippy --workspace`

### Invariants

1. Source degradation is deterministic for the same inputs
2. Tell never creates beliefs out of nothing — it only transfers/degrades existing beliefs
3. observed_tick is NEVER set to current tick — always the speaker's original observed_tick
4. Tell commit reuses the scheduler's action-commit transaction/event path rather than bypassing it
5. No cross-system calls; Tell writes to `AgentBeliefStore` via `WorldTxn`
6. Conservation invariants unaffected (Tell does not move items)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/tell_actions.rs` — extend the existing Tell tests with commit-path coverage instead of creating a separate test module or wider integration harness.

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace`
4. `cargo test --workspace`

## Outcome

Implemented the missing Tell commit path in `crates/worldwake-systems/src/tell_actions.rs` and kept the Tell-specific source degradation and acceptance logic local to that module. The listener now receives transferred beliefs through the existing `AgentBeliefStore` update/capacity path, and the action continues to rely on the scheduler's standard commit transaction/event flow instead of manual event emission inside the handler.

Compared with the original ticket draft, no `worldwake-core` helper was added because Tell is still the only caller for this degradation rule. The ticket was also narrowed before implementation to reflect that Tell action registration, payload validation, TellProfile, Social tags, and witnessed-telling perception support were already present and did not need rework.
