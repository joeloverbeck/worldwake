# E15BSOCAIGOA-011: Extract shared social relay selection logic for Tell affordances and ShareBelief candidates

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — shared social selection helper across sim/ai/systems
**Deps**: E15BSOCAIGOA-004

## Problem

`crates/worldwake-systems/src/tell_actions.rs` and `crates/worldwake-ai/src/candidate_generation.rs` now both implement the same relay-subject selection rules for social information:

- compute relay chain length from `PerceptionSource`
- filter by `TellProfile.max_relay_chain_len`
- sort by `observed_tick` descending, then `subject` ascending
- truncate by `TellProfile.max_tell_candidates`

That duplication is small but architecturally wrong. It creates policy drift risk between mechanical Tell affordances and autonomous ShareBelief goal generation. If one side changes and the other does not, the AI can decide to pursue a social goal that the action layer would not naturally enumerate, or vice versa.

## Assumption Reassessment (2026-03-15)

1. `crates/worldwake-ai/src/candidate_generation.rs` currently has local `belief_chain_len()` plus `relayable_social_subjects()` logic. It filters by relay depth and sorts relayable subjects by `observed_tick` descending, then `subject` ascending, but it does not truncate inside the helper. Truncation currently happens at the call site while emitting `ShareBelief` candidates per listener.
2. `crates/worldwake-systems/src/tell_actions.rs` already has its own local `belief_chain_len()` and relayable-subject sorting/truncation logic inside Tell payload enumeration.
3. The duplicated logic is specifically the authoritative relay-subject policy: chain-length derivation, relay-depth filtering, deterministic ordering, and subject truncation. Listener enumeration is separate and remains caller-specific.
4. `GoalBeliefView` already exposes `known_entity_beliefs()` and `tell_profile()` in the current codebase, so this ticket does not need any additional AI belief-boundary widening or trait changes.
5. Both targeted caller areas already have regression coverage today:
   - `crates/worldwake-ai/src/candidate_generation.rs` has ShareBelief candidate-generation tests.
   - `crates/worldwake-systems/src/tell_actions.rs` has Tell affordance subject-selection tests.
   The gap is shared-policy authority, not missing caller tests from scratch.
6. No active ticket in `tickets/` currently owns deduplicating this production logic. E15BSOCAIGOA-005 is ranking-only; E15BSOCAIGOA-006 through E15BSOCAIGOA-010 are test/report tickets.

## Architecture Check

1. The right cleanup is a single shared helper for relayable-subject selection, not a broader trait dependency or a new compatibility wrapper. This keeps the policy authoritative in one place.
2. A helper that operates over belief data and tell-profile limits is cleaner than trying to force both call sites onto the same trait object boundary. It avoids widening AI modules to `RuntimeBeliefView`.
3. This extraction should preserve the current architecture exactly: goal-reading AI modules stay on `GoalBeliefView`, and Tell affordances stay on `RuntimeBeliefView`. The shared policy should live below both callers in `worldwake-sim`, not by pushing either caller across a new boundary.
4. No backwards-compatibility shims or alias paths should be introduced. Both callers should migrate directly to the shared helper and delete their local copies.
5. The extracted helper is architecturally preferable to the current duplication because relayability is domain policy, not per-caller interpretation. Keeping two implementations invites drift between what the AI proposes and what the action layer can enumerate. Centralizing that policy makes the system more robust and extensible without introducing a new abstraction layer.

## What to Change

### 1. Introduce a shared social relay helper

Add a small shared helper in `worldwake-sim` that owns:

- chain length derivation from `PerceptionSource`
- relay-depth filtering
- deterministic recency ordering
- truncation by `max_tell_candidates`

The helper should accept concrete belief data rather than a broad view trait. One acceptable shape is a borrowed slice/input over already-read beliefs:

```rust
pub fn relayable_social_subjects(
    beliefs: impl IntoIterator<Item = (EntityId, BelievedEntityState)>,
    max_relay_chain_len: u8,
    max_tell_candidates: u8,
) -> Vec<EntityId>
```

If a slightly different signature is cleaner in the current module layout, keep the same semantics. Avoid introducing a new trait or compatibility wrapper just to share this policy.

### 2. Replace local copies in Tell affordances

Update `crates/worldwake-systems/src/tell_actions.rs` to use the shared helper for subject selection. Remove its local duplicate chain-length and ordering logic.

### 3. Replace local copies in social candidate generation

Update `crates/worldwake-ai/src/candidate_generation.rs` to use the same shared helper. Keep listener filtering local to candidate generation, and keep the AI module on `GoalBeliefView` without additional trait changes.

### 4. Add regression tests for policy parity

Add focused tests around the shared helper and retain or minimally adapt the existing caller-level tests that prove both AI candidate generation and Tell affordances continue to agree on:

- relay-depth filtering
- ordering
- truncation

## Files to Touch

- `crates/worldwake-sim/src/lib.rs` (modify)
- `crates/worldwake-sim/src/<new social helper module>.rs` (new)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-systems/src/tell_actions.rs` (modify)

## Out of Scope

- ShareBelief ranking and `social_weight` scoring
- Golden social E2E tests
- Listener enumeration policy beyond preserving current behavior
- New social goal kinds such as `InvestigateMismatch`
- Any widening or modification of AI goal-module trait boundaries unless the extraction proves impossible without it

## Acceptance Criteria

### Tests That Must Pass

1. Shared helper returns the same relayable-subject order currently expected by Tell affordance tests.
2. ShareBelief candidate-generation tests still pass while using the shared helper.
3. Tell affordance tests still pass while using the shared helper.
4. There is no remaining production duplicate of social relay subject selection logic across `candidate_generation.rs` and `tell_actions.rs`.
5. Existing suites:
   - `cargo test -p worldwake-sim`
   - `cargo test -p worldwake-ai`
   - `cargo test -p worldwake-systems tell_actions -- --nocapture`
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Tell affordances and ShareBelief candidate generation derive relayable subjects from one authoritative policy implementation.
2. Goal-reading AI modules continue to depend on `GoalBeliefView`, not `RuntimeBeliefView`.
3. Deterministic ordering remains `observed_tick` descending, then `subject` ascending.
4. Subject truncation remains governed by `max_tell_candidates` in the shared helper, not reimplemented separately per caller.
5. No compatibility wrapper or alias path is left behind after the extraction.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/<new social helper module>.rs` — unit tests for relay-depth filtering, deterministic ordering, and truncation.
2. `crates/worldwake-ai/src/candidate_generation.rs` — retain or minimally adjust existing ShareBelief candidate tests so they still prove listener filtering plus shared subject-policy parity.
3. `crates/worldwake-systems/src/tell_actions.rs` — retain or minimally adjust existing Tell affordance relay-selection tests so they still prove shared subject-policy parity.

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test -p worldwake-ai`
3. `cargo test -p worldwake-systems tell_actions -- --nocapture`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-15
- What actually changed:
  - Added `crates/worldwake-sim/src/social_relay.rs` with the shared `belief_chain_len()` and `relayable_social_subjects()` policy plus focused unit tests.
  - Re-exported the shared relay helpers from `worldwake-sim` and migrated `crates/worldwake-ai/src/candidate_generation.rs` and `crates/worldwake-systems/src/tell_actions.rs` to use them directly.
  - Kept listener enumeration local to each caller while centralizing the relay-subject policy in one authoritative location.
  - Corrected this ticket before implementation so it matched the actual codebase and existing test coverage.
- Deviations from original plan:
  - No AI trait-boundary changes were needed because `GoalBeliefView` already exposed the subjective reads social candidate generation requires.
  - Existing caller-level tests were retained rather than replaced; only the new shared-helper tests were added.
  - `cargo clippy` required a targeted `#[allow(clippy::too_many_lines)]` on one pre-existing ShareBelief candidate test after the extraction pushed it over the threshold.
- Verification results:
  - `cargo test -p worldwake-sim` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo test -p worldwake-systems tell_actions -- --nocapture` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
