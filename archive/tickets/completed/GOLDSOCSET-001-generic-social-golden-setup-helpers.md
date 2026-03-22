# GOLDSOCSET-001: Generic Social Golden Setup Helpers

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` test harness and social/emergent golden test setup only
**Deps**: `archive/tickets/completed/S14CONMEMEME-001-same-place-office-fact-still-requires-tell.md`, `archive/tickets/completed/S14CONMEMEME-002-already-told-recent-subject-does-not-crowd-out-untold-office-fact.md`, `specs/S14-conversation-memory-emergence-golden-suites.md`, `docs/golden-e2e-testing.md`

## Problem

Social and social-to-political goldens still repeat low-level belief-store mutation for three distinct concerns:
- seeding the speaker’s belief about the intended listener so `Tell` can materialize
- seeding the speaker’s shareable subject belief from authoritative world state
- seeding conversation-memory state to suppress unrelated lawful repeat-tell branches

The underlying scenarios are already correct, but the setup vocabulary is still too close to `AgentBeliefStore` internals. That makes golden construction noisier than it needs to be and encourages tests to open-code storage details instead of expressing the social preconditions they actually care about.

## Assumption Reassessment (2026-03-19)

1. The current generic golden harness in `crates/worldwake-ai/tests/golden_harness/mod.rs` exposes `seed_actor_beliefs()`, `seed_belief()`, `set_agent_tell_profile()`, and `set_agent_perception_profile()`, but it still requires tests to manually compose `build_believed_entity_state(...)`, `seed_belief(...)`, `to_shared_belief_snapshot(...)`, and direct `AgentBeliefStore::record_told_belief(...)` writes for common social setup.
2. The S14 cross-system goldens referenced by this cleanup ticket already exist in `crates/worldwake-ai/tests/golden_emergent.rs` as `golden_same_place_office_fact_still_requires_tell` and `golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact`. The remaining gap is helper reuse, not missing scenario coverage.
3. Existing social goldens in `crates/worldwake-ai/tests/golden_social.rs` still repeat explicit listener/subject belief seeding in multiple scenarios, including `build_social_retell_fixture`, `golden_agent_autonomously_tells_colocated_peer`, and `run_skeptical_listener_scenario`. `crates/worldwake-ai/tests/golden_emergent.rs` still open-codes speaker-side told-memory seeding in the crowd-out scenario.
4. Current focused coverage already proves the underlying belief-store primitives are correct: `golden_harness::tests::seed_belief_accessors_and_count_reflect_seeded_state`, `seed_belief_replaces_same_subject_when_tick_is_equal_or_newer`, and `seed_belief_preserves_newer_existing_belief_against_older_input` in `crates/worldwake-ai/tests/golden_harness/mod.rs`. The missing coverage is helper-specific behavior for building a belief from live world state and recording told-memory through a harness API.
5. This is a testing-architecture ticket, not a production AI ticket. The intended layer is focused harness coverage plus golden/E2E migration. Full action registries remain required for the affected goldens because they exercise real social and political actions.
6. Ordering-sensitive scenarios that use these helpers still rely on mixed verification layers: social omission status in decision traces, lifecycle ordering in action traces, and office outcomes in authoritative state. Helper extraction must not blur those boundaries or move assertions into the harness.
7. Scenario isolation remains explicit and ticket-owned. The helper layer must not encode “office should win”, “subject A should crowd out B”, or other scenario outcomes; it should only expose lawful, composable setup operations.
8. Mismatch correction: the ticket previously read as if the S14 golden scenarios were still the main deliverable. They are already implemented. The corrected scope is reusable social-golden setup vocabulary plus focused tests proving those helpers preserve current semantics.

## Architecture Check

1. A small set of generic social setup helpers is cleaner than continuing to hand-roll belief-store mutations in each golden. It keeps tests phrased in social terms such as “speaker knows listener”, “speaker knows subject”, and “speaker already told this belief” instead of leaking storage details into every scenario.
2. The helpers must stay world-state-based and composable. No scenario builders, no ticket-specific helpers, and no compatibility aliasing around old call sites.

## Verification Layers

1. Build-and-seed helper preserves existing belief-store replacement semantics -> focused harness unit tests
2. Told-memory setup helper writes the intended `(listener, subject)` memory with the provided shared snapshot and tick -> focused harness unit tests
3. Migrated social/emergent scenarios still prove real runtime behavior through the live stack -> `worldwake-ai` golden E2E tests
4. Additional mixed-layer mapping is not applicable because this ticket changes test setup architecture, not production causal semantics

## What to Change

### 1. Add generic social setup helpers to the golden harness

Extend `crates/worldwake-ai/tests/golden_harness/mod.rs` with a minimal generic surface for:
- seeding an actor belief about a subject by building the snapshot from current authoritative world state
- seeding a speaker belief about one or more shareable subjects without open-coding snapshot construction in each test
- seeding speaker-side told-memory for `(listener, subject)` using an explicit shared snapshot and tick

These helpers should be composable rather than “scenario builder” abstractions.

### 2. Migrate repeated social setup call sites

Use the new helpers in the most repetitive current call sites in:
- `crates/worldwake-ai/tests/golden_social.rs`
- `crates/worldwake-ai/tests/golden_emergent.rs`

The migration should reduce duplication but keep each scenario’s isolation choices readable in the test body.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)
- `crates/worldwake-ai/tests/golden_social.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)

## Out of Scope

- Any production `worldwake-ai`, `worldwake-sim`, or `worldwake-systems` behavior change
- Scenario-builder helpers that hardcode office or crowd-out logic
- Hiding scenario-isolation choices inside helper internals

## Acceptance Criteria

### Tests That Must Pass

1. Focused harness tests proving the new social setup helpers preserve belief-store and told-memory semantics
2. `cargo test -p worldwake-ai --test golden_social`
3. `cargo test -p worldwake-ai --test golden_emergent`
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing lint boundary: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The helper layer remains generic and world-state-based; it does not encode scenario-specific outcomes or cross-system assertions.
2. Social goldens still express their isolation choices explicitly in test bodies even after helper extraction.

## Test Plan

### New/Modified Tests

1. `golden_harness::tests::seed_belief_from_world_builds_and_stores_snapshot` in `crates/worldwake-ai/tests/golden_harness/mod.rs` — proves the new helper builds a snapshot from authoritative world state and stores it without changing belief semantics.
2. `golden_harness::tests::seed_told_belief_memory_records_requested_entry_and_preserves_beliefs` in `crates/worldwake-ai/tests/golden_harness/mod.rs` — proves the told-memory helper writes the exact `(listener, subject, tick)` entry while preserving previously seeded beliefs in the same store.
3. `crates/worldwake-ai/tests/golden_social.rs` — migrated repeated listener/subject seeding in the social retell fixture, autonomous tell path, skeptical-listener path, bystander path, and social-diversity path to the generic helpers without changing runtime assertions.
4. `crates/worldwake-ai/tests/golden_emergent.rs` — migrated the S14 crowd-out setup to the same helper vocabulary, including speaker-side told-memory seeding.

### Commands

1. `cargo test -p worldwake-ai --test golden_social`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-19
- What actually changed:
  - Added `seed_belief_from_world(...)` and `seed_told_belief_memory(...)` to `crates/worldwake-ai/tests/golden_harness/mod.rs`.
  - Added focused harness coverage for both helpers.
  - Migrated the most repetitive social-golden and emergent-golden setup call sites to the new helper vocabulary.
  - Reassessed and corrected the ticket scope so it matches the current codebase: the S14 goldens already existed, and the real gap was reusable setup vocabulary.
- Deviations from original plan:
  - No production `worldwake-ai`, `worldwake-sim`, or `worldwake-systems` changes were needed.
  - The helper surface stayed smaller than the initial ticket wording implied. Existing `seed_actor_beliefs()` already covered multi-subject bulk seeding, so the missing ergonomic gap was a single-subject build-and-seed helper plus a told-memory helper, not a larger scenario-builder layer.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_social`
  - `cargo test -p worldwake-ai --test golden_emergent`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
