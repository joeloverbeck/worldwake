# GOLDDOCSOC-001: Social Golden Preconditions And Trace-Surface Guidance

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: `tickets/S14CONMEMEME-003-golden-e2e-docs-catch-up.md`, `archive/tickets/completed/S14CONMEMEME-001-same-place-office-fact-still-requires-tell.md`, `archive/tickets/completed/S14CONMEMEME-002-already-told-recent-subject-does-not-crowd-out-untold-office-fact.md`, `specs/S14-conversation-memory-emergence-golden-suites.md`, `docs/golden-e2e-testing.md`

## Problem

The current golden-testing docs explain trace-layer selection well at a high level, but they do not yet document several social-golden specifics that matter in practice:
- the speaker may need an explicit belief about the intended listener for the `Tell` branch to materialize
- using an agent as the shareable subject can introduce extra lawful social branches that must be isolated deliberately
- subject-specific `tell` commit ordering is now available in action traces, but omission/generation questions still belong to decision traces

Without that guidance, future tickets can easily overstate what action traces can prove or accidentally design malformed social scenarios.

## Assumption Reassessment (2026-03-19)

1. `docs/golden-e2e-testing.md` already distinguishes decision traces, action traces, authoritative state, and scenario isolation, but it does not currently document the social-specific setup requirement that the speaker may need an explicit belief about the intended listener before `ShareBelief` can materialize.
2. `docs/golden-e2e-scenarios.md` documents the social and emergent slices broadly, but it does not currently call out that some social scenarios deliberately avoid agent subjects because they can create extra lawful `ShareBelief` branches unrelated to the contract under test.
3. Mismatch correction: `crates/worldwake-sim/src/action_trace.rs` already exposes typed `ActionTraceDetail::Tell { listener, subject }`. The ticket's original claim that action traces could not encode subject-specific `tell` payloads is stale and must not drive scope.
4. Current shipped coverage that should be cited in the docs is verified by real test names: `golden_agent_does_not_repeat_same_unchanged_tell_to_same_listener`, `golden_decision_trace_explains_social_candidate_reenabled_after_belief_change_or_expiry`, `golden_same_place_office_fact_still_requires_tell`, and `golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact` confirmed via `cargo test -p worldwake-ai -- --list`.
5. This is a documentation/testing-contract ticket only. No runtime or production behavior changes are introduced.
6. Ordering guidance remains mixed-layer for these scenarios: decision traces prove social suppression/filtering and negative generation claims, action traces prove concrete committed `tell` lifecycle ordering including `listener`/`subject` detail, and authoritative state proves downstream office outcomes. The docs need to name that division explicitly instead of implying that one trace surface proves the whole chain.
7. Mismatch correction: `tickets/S14CONMEMEME-003-golden-e2e-docs-catch-up.md` already covers coverage-matrix and scenario-catalog catch-up. This follow-on ticket is narrower and more architectural: it updates the testing contract for social preconditions and trace-surface limits that S14 implementation exposed.

## Architecture Check

1. Tightening the docs is cleaner than leaving test authors to rediscover these constraints through failing scenarios and temporary debug prints. It improves architectural fidelity without adding code paths.
2. This ticket introduces no shims, aliases, or alternate testing contracts. It clarifies the canonical one.

## Verification Layers

1. Golden docs name real existing tests and binaries correctly -> documentation review against `cargo test -p worldwake-ai -- --list`
2. Social scenario-isolation guidance explicitly covers listener-belief setup and agent-vs-non-agent subject choice -> documentation review against implemented S14 scenarios
3. Trace-surface guidance explicitly states that action traces can prove committed `tell` listener/subject ordering, while decision traces still prove omission/generation invariants -> documentation review against `crates/worldwake-sim/src/action_trace.rs`
4. Additional runtime mapping is not applicable because this ticket is documentation-only

## What to Change

### 1. Extend the golden testing contract

Update `docs/golden-e2e-testing.md` to add explicit guidance for social scenarios:
- when to seed speaker belief about the intended listener
- when agent subjects create extra lawful branches that must be isolated
- what action traces can and cannot currently prove for `tell`

### 2. Update the scenario catalog

Update `docs/golden-e2e-scenarios.md` so the social/emergent scenario descriptions reflect the real preconditions and the mixed-layer assertion surfaces used in the implemented S14 scenarios.

### 3. Align S14 docs/tickets with the current trace surface

If `tickets/S14CONMEMEME-003-golden-e2e-docs-catch-up.md` or related scenario docs still imply that action traces are insufficient for committed subject-specific `tell` ordering, correct that wording so it matches the current architecture.

## Files to Touch

- `docs/golden-e2e-testing.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)
- `docs/golden-e2e-coverage.md` (modify only if wording around S14 trace surfaces needs correction)
- `tickets/S14CONMEMEME-003-golden-e2e-docs-catch-up.md` (modify only if assumptions need tightening)

## Out of Scope

- Production changes to traces, social AI, or political planning
- Rewriting the broader golden documentation structure
- Claiming new trace capabilities that do not exist yet

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai --test golden_social`
3. `cargo test -p worldwake-ai --test golden_emergent`

### Invariants

1. The docs must describe the current trace substrate honestly, including the fact that committed subject-specific `tell` ordering is available from action traces, while omission/generation questions still require decision traces.
2. Social golden guidance must name scenario-isolation choices explicitly rather than implying that any colocated speaker/listener/subject setup is self-isolating.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai --test golden_social`
3. `cargo test -p worldwake-ai --test golden_emergent`

## Outcome

- Completion date: 2026-03-19
- Actual changes:
  - Corrected the ticket scope before implementation: action traces already expose committed `tell` `listener`/`subject` detail via `ActionTraceDetail::Tell`, so the work shifted from documenting a missing trace capability to documenting the proper split between action-trace and decision-trace assertions.
  - Updated `docs/golden-e2e-testing.md` to document social-specific setup requirements for explicit listener-belief seeding, agent-vs-non-agent subject isolation, and when to use action traces versus decision traces for `tell`.
  - Updated `docs/golden-e2e-scenarios.md` to reflect the real S14 scenario preconditions, including explicit listener-belief seeding and the deliberate non-agent recent subject in the crowd-out scenario.
- Deviations from original plan:
  - No production/runtime code changes were needed.
  - `docs/golden-e2e-coverage.md` and `tickets/S14CONMEMEME-003-golden-e2e-docs-catch-up.md` did not require edits after reassessment.
- Verification results:
  - `cargo test -p worldwake-ai -- --list`
  - `cargo test -p worldwake-ai --test golden_social`
  - `cargo test -p worldwake-ai --test golden_emergent`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
