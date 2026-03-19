# S14CONMEMEME-003: Golden E2E Docs Catch-Up For Conversation Memory Emergence

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: `S14CONMEMEME-001`, `S14CONMEMEME-002`, `specs/S14-conversation-memory-emergence-golden-suites.md`

## Problem

The S14 spec requires the golden E2E docs to reflect both the already-landed E15c social tests and the new S14 cross-system emergence suites. The current docs need a catch-up pass so the coverage matrix, scenario catalog, and assertion-surface guidance accurately describe conversation-memory verification.

## Assumption Reassessment (2026-03-19)

1. The relevant docs exist today: `docs/golden-e2e-coverage.md`, `docs/golden-e2e-scenarios.md`, and `docs/golden-e2e-testing.md`.
2. Current shipped conversation-memory goldens live in `crates/worldwake-ai/tests/golden_social.rs` and include `golden_agent_does_not_repeat_same_unchanged_tell_to_same_listener`, `golden_agent_retells_after_subject_belief_changes`, `golden_agent_retells_after_conversation_memory_expiry`, and `golden_decision_trace_explains_social_candidate_reenabled_after_belief_change_or_expiry`.
3. Current cross-system political Tell coverage exists in `crates/worldwake-ai/tests/golden_emergent.rs` as `golden_tell_propagates_political_knowledge`, and S14 adds two more emergence scenarios that should be documented after they land.
4. `specs/S14-conversation-memory-emergence-golden-suites.md` explicitly requires documentation updates for both the already-landed E15c social tests and the new S14 suites. `specs/IMPLEMENTATION-ORDER.md` places this work after the two golden tickets.
5. This is a documentation-only ticket. Verification is command-based plus content review against actual test names; no new runtime verification layer is introduced here.
6. Mismatch correction: if implementation of `S14CONMEMEME-001` or `S14CONMEMEME-002` changes final test names, this ticket must document the final names rather than the provisional ones from the spec.

## Architecture Check

1. Keeping the docs update separate from the scenario implementation tickets keeps each code review small and prevents mixed code-plus-doc diffs from obscuring whether the new goldens are actually correct.
2. This ticket introduces no production shims, compatibility layers, or behavior changes. It only aligns the docs with the implemented golden suites and current testing guidance.

## Verification Layers

1. Golden coverage matrix accurately lists E15c social goldens and the new S14 cross-system suites -> documentation review against actual test files and `cargo test -p worldwake-ai -- --list`
2. Scenario catalog accurately describes scenario intent, assertion surfaces, and downstream consequences -> documentation review against implemented test bodies
3. Assertion-surface guidance stays consistent with `docs/golden-e2e-testing.md` -> documentation review
4. Additional runtime mapping is not applicable because this ticket changes docs only

## What to Change

### 1. Update the coverage matrix

Update `docs/golden-e2e-coverage.md` so it explicitly includes the already-landed E15c social conversation-memory tests plus the new S14 same-place and crowd-out emergence suites.

### 2. Update the scenario catalog

Update `docs/golden-e2e-scenarios.md` with concise entries for the new S14 scenarios and any missing E15c conversation-memory scenarios that are currently under-reported.

### 3. Review testing guidance for any needed assertion-surface clarification

Review `docs/golden-e2e-testing.md` after the two S14 goldens land. Only edit it if the new suites require clearer guidance on when to use decision traces, action traces, or authoritative world-state assertions in future goldens.

## Files to Touch

- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)
- `docs/golden-e2e-testing.md` (modify only if clarification is actually needed)

## Out of Scope

- Adding or modifying golden tests
- Production code changes in any crate
- Reorganizing the broader golden documentation structure beyond S14/E15c coverage accuracy
- Inventing coverage claims for tests that do not exist in the repo after implementation

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_social`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo test -p worldwake-ai`

### Invariants

1. Documentation must name actual implemented test cases and their real owning binaries.
2. The docs must distinguish E15c local social coverage from S14 cross-system emergence coverage rather than collapsing them into one vague “social Tell” category.
3. Assertion-surface guidance must remain aligned with the repo’s decision-trace, action-trace, and authoritative-state testing contract.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-ai --test golden_social`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo test -p worldwake-ai`
4. `cargo test -p worldwake-ai -- --list`
