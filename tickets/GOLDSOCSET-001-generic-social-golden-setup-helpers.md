# GOLDSOCSET-001: Generic Social Golden Setup Helpers

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` test harness and social/emergent golden test setup only
**Deps**: `archive/tickets/completed/S14CONMEMEME-001-same-place-office-fact-still-requires-tell.md`, `archive/tickets/completed/S14CONMEMEME-002-already-told-recent-subject-does-not-crowd-out-untold-office-fact.md`, `specs/S14-conversation-memory-emergence-golden-suites.md`, `docs/golden-e2e-testing.md`

## Problem

Social and social-to-political goldens currently repeat fragile setup patterns for three distinct concerns:
- seeding the speaker’s belief about the intended listener so `Tell` can materialize
- seeding the speaker’s shareable subject belief
- seeding conversation-memory state to suppress unrelated lawful repeat-tell branches

Those steps are currently expressed ad hoc in individual tests, which makes scenario construction error-prone and obscures which setup is architectural versus incidental.

## Assumption Reassessment (2026-03-19)

1. The current generic golden harness in `crates/worldwake-ai/tests/golden_harness/mod.rs` exposes `seed_actor_beliefs()`, `seed_belief()`, `set_agent_tell_profile()`, and `set_agent_perception_profile()`, but it does not expose a higher-level helper for common social-golden setup patterns such as “speaker knows listener and subject” or “speaker has preexisting told-memory for this listener/subject”.
2. Existing social goldens in `crates/worldwake-ai/tests/golden_social.rs` repeat explicit listener-belief seeding through `build_believed_entity_state(...listener...)` + `seed_belief(...)` in multiple scenarios, including the autonomous tell path and the social retell fixture. The same setup pattern also now appears in `crates/worldwake-ai/tests/golden_emergent.rs` for the crowd-out scenario.
3. Current focused/runtime coverage already proves the underlying belief-store and seeding primitives are correct: `golden_harness::tests::seed_belief_accessors_and_count_reflect_seeded_state`, `seed_belief_replaces_same_subject_when_tick_is_equal_or_newer`, and `seed_belief_preserves_newer_existing_belief_against_older_input` in `crates/worldwake-ai/tests/golden_harness/mod.rs`.
4. This is a testing-architecture ticket, not a production AI ticket. The intended layer is golden/E2E setup and harness ergonomics. Full action registries remain required for the affected goldens because they exercise real social and political actions.
5. Ordering-sensitive scenarios that use these helpers will still rely on a mixed-layer combination: social suppression/filtering in decision traces, lifecycle ordering in action traces, and durable office outcomes in authoritative state. The helper work must not hide or collapse that distinction.
6. Scenario isolation remains explicit and ticket-owned. The helper layer should not encode “subject A vs subject B” or “office should win” logic. It should only remove repeated boilerplate around lawful setup.
7. Mismatch correction: the gap is not missing test coverage of seeding primitives. The gap is missing reusable test setup vocabulary for common social-golden preconditions and conversation-memory isolation.

## Architecture Check

1. A small set of generic social setup helpers is cleaner than continuing to hand-roll belief-store mutations in each new social/emergent golden. It lowers test brittleness while keeping scenario ownership in the test body.
2. The helpers must stay world-state-based and generic. No ticket-specific helpers, no hidden scenario outcomes, and no compatibility wrapper around old helper names.

## Verification Layers

1. Listener-belief setup helper preserves existing seeding semantics -> focused harness unit tests
2. Told-memory setup helper writes the intended conversation-memory state without bypassing runtime semantics -> focused harness unit tests
3. Existing social/emergent goldens still prove real runtime behavior through the live stack -> `worldwake-ai` golden E2E tests
4. Additional layer mapping is not applicable beyond those boundaries because this ticket changes test setup architecture, not production causal semantics

## What to Change

### 1. Add generic social setup helpers to the golden harness

Extend `crates/worldwake-ai/tests/golden_harness/mod.rs` with a minimal generic surface for:
- seeding a speaker belief about the intended listener
- seeding a speaker belief about one or more shareable subjects
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

### Invariants

1. The helper layer remains generic and world-state-based; it does not encode scenario-specific outcomes or cross-system assertions.
2. Social goldens still express their isolation choices explicitly in test bodies even after helper extraction.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_harness/mod.rs` — add focused tests for listener-belief and told-memory helper behavior.
2. `crates/worldwake-ai/tests/golden_social.rs` — migrate repeated setup to the generic helpers without changing social runtime assertions.
3. `crates/worldwake-ai/tests/golden_emergent.rs` — migrate the S14 social-political setup to the same helper vocabulary.

### Commands

1. `cargo test -p worldwake-ai --test golden_social`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo test -p worldwake-ai`
