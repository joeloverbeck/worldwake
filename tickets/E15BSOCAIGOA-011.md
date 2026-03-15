# E15BSOCAIGOA-011: Extract shared social relay selection logic for Tell affordances and ShareBelief candidates

**Status**: PENDING
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

1. `candidate_generation.rs` now has local `belief_chain_len()` plus `relayable_social_subjects()` logic added by E15BSOCAIGOA-004.
2. `tell_actions.rs` already had its own local `belief_chain_len()` and relayable-subject sorting/truncation logic before E15BSOCAIGOA-004.
3. No active ticket in `tickets/` currently owns deduplicating this production logic. E15BSOCAIGOA-005 is ranking-only; E15BSOCAIGOA-006 through E15BSOCAIGOA-010 are test/report tickets.
4. The duplicated logic is about subject selection policy, not listener enumeration. Listener filtering remains candidate-generation-specific and should stay local.
5. The clean shared boundary is data-oriented, not trait-object-oriented: both call sites already have access to `(EntityId, BelievedEntityState)` sequences plus `TellProfile` limits.

## Architecture Check

1. The right cleanup is a single shared helper for relayable-subject selection, not a broader trait dependency or a new compatibility wrapper. This keeps the policy authoritative in one place.
2. A helper that operates over belief data and tell-profile limits is cleaner than trying to force both call sites onto the same trait object boundary. It avoids widening AI modules to `RuntimeBeliefView`.
3. This preserves the existing architecture guard that goal-reading AI modules stay on `GoalBeliefView`.
4. No backwards-compatibility shims or alias paths should be introduced. Both callers should migrate directly to the shared helper and delete their local copies.

## What to Change

### 1. Introduce a shared social relay helper

Add a small shared helper in `worldwake-sim` that owns:

- chain length derivation from `PerceptionSource`
- relay-depth filtering
- deterministic recency ordering
- truncation by `max_tell_candidates`

The helper should accept concrete belief data rather than a broad view trait. One acceptable shape:

```rust
pub fn relayable_social_subjects(
    beliefs: impl IntoIterator<Item = (EntityId, BelievedEntityState)>,
    max_relay_chain_len: u8,
    max_tell_candidates: u8,
) -> Vec<EntityId>
```

If a slightly different signature is cleaner in the current module layout, keep the same semantics.

### 2. Replace local copies in Tell affordances

Update `crates/worldwake-systems/src/tell_actions.rs` to use the shared helper for subject selection. Remove its local duplicate chain-length and ordering logic.

### 3. Replace local copies in social candidate generation

Update `crates/worldwake-ai/src/candidate_generation.rs` to use the same shared helper. Keep listener filtering local to candidate generation.

### 4. Add regression tests for policy parity

Add focused tests around the shared helper and retain caller-level tests that prove both AI candidate generation and Tell affordances continue to agree on:

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
- Any widening of AI goal modules from `GoalBeliefView` to `RuntimeBeliefView`

## Acceptance Criteria

### Tests That Must Pass

1. Shared helper returns the same relayable-subject order currently expected by Tell affordance tests.
2. ShareBelief candidate-generation tests still pass while using the shared helper.
3. Tell affordance tests still pass while using the shared helper.
4. There is no remaining production duplicate of social relay subject selection logic across `candidate_generation.rs` and `tell_actions.rs`.
5. Existing suites:
   - `cargo test -p worldwake-ai`
   - `cargo test -p worldwake-systems tell_actions -- --nocapture`

### Invariants

1. Tell affordances and ShareBelief candidate generation derive relayable subjects from one authoritative policy implementation.
2. Goal-reading AI modules continue to depend on `GoalBeliefView`, not `RuntimeBeliefView`.
3. Deterministic ordering remains `observed_tick` descending, then `subject` ascending.
4. No compatibility wrapper or alias path is left behind after the extraction.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/<new social helper module>.rs` — unit tests for relay-depth filtering, deterministic ordering, and truncation.
2. `crates/worldwake-ai/src/candidate_generation.rs` — keep ShareBelief candidate tests passing against the shared helper.
3. `crates/worldwake-systems/src/tell_actions.rs` — keep Tell affordance relay-selection tests passing against the shared helper.

### Commands

1. `cargo test -p worldwake-systems tell_actions -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
