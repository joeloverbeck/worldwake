# S43COMTYPDIF-004: Class-aware Tell acceptance + remove TellProfile.acceptance_fidelity

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — Tell handler acceptance logic changed, TellProfile field removed
**Deps**: S43COMTYPDIF-001

## Problem

The Tell handler uses a single `TellProfile.acceptance_fidelity` value for all Tell payload items regardless of content urgency. A listener's skepticism toward idle gossip also blocks acceptance of critical alarms. Per Principle 28, the old uniform acceptance field must be replaced, not wrapped.

## Assumption Reassessment (2026-04-03)

1. Tell commit handler at `tell_actions.rs:578` calls `passes_acceptance_check(listener_profile.acceptance_fidelity.value(), rng)` — a single fidelity check for the entire Tell. Confirmed.
2. `TellProfile` at `belief.rs:1323` has fields: `max_tell_candidates`, `max_relay_chain_len`, `acceptance_fidelity`, `conversation_memory_capacity`, `conversation_memory_retention_ticks`. Derives `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. Confirmed.
3. `TellProfile::default()` sets `acceptance_fidelity: Permille::new_unchecked(800)`. Confirmed.
4. `acceptance_fidelity` is referenced in ~15 test setups in `tell_actions.rs` — all need updating.
5. `CommunicationProfile` (from ticket 001) will provide `alarm_acceptance`, `testimony_acceptance`, `gossip_acceptance` as replacements.
6. `classify_communication(topic, speaker_beliefs)` (from ticket 001) is available in `worldwake-core`, callable from `worldwake-systems`.
7. The Tell commit path gets the speaker from `ActionInstance.actor` and the listener from `TellActionPayload.listener`. It can access `AgentBeliefStore` for the speaker directly at commit time to classify the topic.
8. This is an information-path refactor (precision rule 16): the acceptance check path changes from `TellProfile.acceptance_fidelity` (single uniform) to `CommunicationProfile.{alarm,testimony,gossip}_acceptance` (class-specific). The canonical end-state path is through `CommunicationProfile`. The old `acceptance_fidelity` field is removed in this ticket — no coexistence.
9. `SAVE_FORMAT_VERSION` at `save_load.rs:6` is currently `14`. Must bump to `15` because `TellProfile` serialization changes (field removed).
10. `TellProfile` is a shared serialized shape, so removing a field also forces mechanical fixture updates beyond `belief.rs` and `tell_actions.rs`, including explicit roundtrip/component samples in core, planning snapshot/state fixtures in `worldwake-ai`, and any golden/integration helpers that still build `TellProfile` literals. This fallout is structural, not a scope change.

## Architecture Check

1. Replacing one field with a class-dispatched lookup is a clean substitution. The Tell handler already has all data needed: payload topic, speaker's belief store (to classify), listener's CommunicationProfile (to look up acceptance).
2. No backwards-compatibility shims. `acceptance_fidelity` is removed from `TellProfile`, not deprecated or aliased. Per Principle 28.
3. The `CommunicationProfile` fallback (when absent on listener) uses default values that match or approximate the old `acceptance_fidelity` default (800‰ for testimony, which was the old uniform default). This preserves behavior for agents without explicit profiles.

## Verification Layers

1. Class-specific acceptance check -> focused unit test: alarm topic uses `alarm_acceptance`, gossip topic uses `gossip_acceptance`
2. TellProfile.acceptance_fidelity removed -> compilation: no references to the field remain
3. Fallback behavior when CommunicationProfile absent -> focused unit test: defaults match expected values
4. Save format version bump -> existing save/load tests (if any) or manual verification
5. This is a cross-crate change (worldwake-core TellProfile + worldwake-systems Tell handler) but the interaction is state-mediated (component reads), not cross-system coupling

## What to Change

### 1. Remove acceptance_fidelity from TellProfile

In `crates/worldwake-core/src/belief.rs`, remove the `acceptance_fidelity: Permille` field from `TellProfile`. Update the `Default` impl to remove it.

### 2. Modify Tell commit handler for class-aware acceptance

In `crates/worldwake-systems/src/tell_actions.rs`, in the commit handler:

1. Retrieve the speaker's `AgentBeliefStore` (already accessible via transaction).
2. Call `classify_communication(&payload.topic, speaker_beliefs)` to get the class.
3. Retrieve the listener's `CommunicationProfile` (fall back to `CommunicationProfile::default()` if absent).
4. Select the appropriate acceptance fidelity:
   - `Alarm` → `profile.alarm_acceptance`
   - `Testimony` → `profile.testimony_acceptance`
   - `Gossip` → `profile.gossip_acceptance`
5. Pass the selected fidelity to `passes_acceptance_check`.

### 3. Update all test setups in tell_actions.rs

Every test that sets `TellProfile { acceptance_fidelity: ..., .. }` must be updated:
- Remove `acceptance_fidelity` from `TellProfile` construction.
- Where tests need to control acceptance, set `CommunicationProfile` on the listener entity instead.
- Tests that set `acceptance_fidelity: Permille::new(0)` (rejection tests) should set the appropriate class-specific fidelity to 0 on `CommunicationProfile`.
- Tests that set `acceptance_fidelity: Permille::new(1000)` (guaranteed acceptance) should set all three class fidelities to 1000.

### 4. Bump SAVE_FORMAT_VERSION

In `crates/worldwake-sim/src/save_load.rs`, increment `SAVE_FORMAT_VERSION` from 14 to 15.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify) — remove `acceptance_fidelity` from TellProfile
- `crates/worldwake-core/src/component_tables.rs` (modify) — update explicit TellProfile roundtrip sample
- `crates/worldwake-core/src/delta.rs` (modify) — update explicit TellProfile component-value sample
- `crates/worldwake-core/src/world.rs` (modify) — update explicit TellProfile world sample
- `crates/worldwake-core/src/world_txn.rs` (modify) — update explicit TellProfile delta test fixture
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify) — update TellProfile snapshot fixture
- `crates/worldwake-ai/src/planning_state.rs` (modify) — update TellProfile planning-state fixture
- `crates/worldwake-ai/src/candidate_generation.rs` (modify) — update TellProfile candidate-generation fixture
- `crates/worldwake-ai/tests/*` (modify) — remove stale `TellProfile.acceptance_fidelity` literals; move any acceptance semantics onto `CommunicationProfile`
- `crates/worldwake-systems/src/tell_actions.rs` (modify) — class-aware acceptance, update tests
- `crates/worldwake-systems/tests/e15_information_integration.rs` (modify) — update integration TellProfile helper
- `crates/worldwake-sim/src/save_load.rs` (modify) — bump SAVE_FORMAT_VERSION

## Out of Scope

- Goal policy suppression changes (ticket 003)
- Ranking boost (ticket 003)
- GoalKind::ShareBelief extension (ticket 002)
- Golden test scenarios (ticket 005)
- Save migration logic for old format (not needed — dev-only saves, no production persistence)

## Acceptance Criteria

### Tests That Must Pass

1. Tell commit with Alarm-class topic uses `alarm_acceptance` fidelity from listener's CommunicationProfile
2. Tell commit with Gossip-class topic uses `gossip_acceptance` fidelity
3. Tell commit with no CommunicationProfile on listener falls back to default values
4. `tell_commit_respects_listener_acceptance_fidelity` test updated and passing (or renamed to reflect class-aware behavior)
5. No remaining references to `TellProfile.acceptance_fidelity` in codebase
6. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. `TellProfile` no longer has `acceptance_fidelity` — Principle 28 enforced
2. When `CommunicationProfile` is absent, default acceptance values are used (950/800/600) — no panic or error
3. `SAVE_FORMAT_VERSION` bumped — old saves will not silently load with wrong TellProfile shape

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/tell_actions.rs` — modify `tell_commit_respects_listener_acceptance_fidelity` to test class-specific acceptance
2. `crates/worldwake-systems/src/tell_actions.rs` — add test for CommunicationProfile fallback defaults
3. All existing Tell tests — update TellProfile construction to remove `acceptance_fidelity`

### Commands

1. `cargo test -p worldwake-systems -- tell`
2. `cargo test -p worldwake-core`
3. `cargo test -p worldwake-systems`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`

## Outcome

- **Completion date**: 2026-04-03
- **What changed**:
  - Removed `TellProfile.acceptance_fidelity` from `crates/worldwake-core/src/belief.rs` and updated the shared serialized/sample fixtures that still built explicit `TellProfile` literals.
  - Changed `crates/worldwake-systems/src/tell_actions.rs` so Tell commit classifies communication from the speaker's belief store and reads listener acceptance from `CommunicationProfile.{alarm_acceptance,testimony_acceptance,gossip_acceptance}`, with default fallback when the listener has no explicit communication profile.
  - Updated acceptance-sensitive systems/golden/integration harness setup to express tell acceptance through listener `CommunicationProfile` rather than the removed `TellProfile` field.
  - Bumped `SAVE_FORMAT_VERSION` from `14` to `15` in `crates/worldwake-sim/src/save_load.rs` because the persisted `TellProfile` shape changed.
- **Deviations from original plan**:
  - The ticket correctly anticipated structural shared-shape fallout, but the live implementation also needed explicit golden/harness setup migration where prior tests were using `TellProfile.acceptance_fidelity` to encode listener acceptance behavior.
- **Verification results**:
  - `cargo test -p worldwake-systems -- tell`
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-ai --test golden_emergent -- golden_same_place_office_fact_still_requires_tell`
  - `cargo test -p worldwake-ai --test golden_social`
  - `cargo test -p worldwake-systems`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
