# GOLDDOCSOC-001: Social Golden Preconditions And Trace-Surface Guidance

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: `tickets/S14CONMEMEME-003-golden-e2e-docs-catch-up.md`, `archive/tickets/completed/S14CONMEMEME-001-same-place-office-fact-still-requires-tell.md`, `archive/tickets/completed/S14CONMEMEME-002-already-told-recent-subject-does-not-crowd-out-untold-office-fact.md`, `specs/S14-conversation-memory-emergence-golden-suites.md`, `docs/golden-e2e-testing.md`

## Problem

The current golden-testing docs explain trace-layer selection well at a high level, but they do not yet document several social-golden specifics that matter in practice:
- the speaker may need an explicit belief about the intended listener for the `Tell` branch to materialize
- using an agent as the shareable subject can introduce extra lawful social branches that must be isolated deliberately
- action traces currently do not encode `tell` payload subjects, so some subject-specific social assertions must remain decision-trace/state-based until the trace substrate changes

Without that guidance, future tickets can easily overstate what action traces can prove or accidentally design malformed social scenarios.

## Assumption Reassessment (2026-03-19)

1. `docs/golden-e2e-testing.md` already distinguishes decision traces, action traces, authoritative state, and scenario isolation, but it does not currently mention payload-detail limits of action traces or social-specific setup requirements for `Tell`.
2. `docs/golden-e2e-scenarios.md` documents the social and emergent slices broadly, but it does not currently call out that social scenarios often require explicit listener-belief seeding or that agent subjects can create competing lawful tell branches.
3. `specs/S14-conversation-memory-emergence-golden-suites.md` Scenario 25 currently describes a subject-specific action-trace assertion for `tell` that the current trace substrate cannot express directly. That mismatch should be documented clearly unless and until a separate traceability ticket lands.
4. Current shipped coverage that should be cited in the docs is verified by real test names: `golden_agent_does_not_repeat_same_unchanged_tell_to_same_listener`, `golden_decision_trace_explains_social_candidate_reenabled_after_belief_change_or_expiry`, `golden_same_place_office_fact_still_requires_tell`, and `golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact` confirmed via `cargo test -p worldwake-ai -- --list`.
5. This is a documentation/testing-contract ticket only. No runtime or production behavior changes are introduced.
6. Ordering guidance remains mixed-layer for these scenarios: decision traces prove social suppression/filtering, action traces prove generic lifecycle ordering, and authoritative state proves downstream office outcomes. The docs need to name that division explicitly instead of implying that one trace surface proves the whole chain.
7. Mismatch correction: `tickets/S14CONMEMEME-003-golden-e2e-docs-catch-up.md` already covers coverage-matrix and scenario-catalog catch-up. This follow-on ticket is narrower and more architectural: it updates the testing contract for social preconditions and trace-surface limits that S14 implementation exposed.

## Architecture Check

1. Tightening the docs is cleaner than leaving test authors to rediscover these constraints through failing scenarios and temporary debug prints. It improves architectural fidelity without adding code paths.
2. This ticket introduces no shims, aliases, or alternate testing contracts. It clarifies the canonical one.

## Verification Layers

1. Golden docs name real existing tests and binaries correctly -> documentation review against `cargo test -p worldwake-ai -- --list`
2. Social scenario-isolation guidance explicitly covers listener-belief setup and agent-vs-non-agent subject choice -> documentation review against implemented S14 scenarios
3. Trace-surface guidance explicitly states current action-trace limits for subject-specific `tell` assertions -> documentation review against `crates/worldwake-sim/src/action_trace.rs`
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

If `tickets/S14CONMEMEME-003-golden-e2e-docs-catch-up.md` or related scenario docs still imply subject-specific `tell` ordering from action traces alone, correct that wording so it matches the current architecture unless `ACTTRCPAY-001` lands first.

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

1. The docs must describe the current trace substrate honestly, including the fact that subject-specific `tell` ordering is not yet directly available from action traces.
2. Social golden guidance must name scenario-isolation choices explicitly rather than implying that any colocated speaker/listener/subject setup is self-isolating.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai --test golden_social`
3. `cargo test -p worldwake-ai --test golden_emergent`
