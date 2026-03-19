# S15STAFAIEME-007: Document Production Start-Gate vs Commit-Time Contention Seams

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None expected; docs/spec/ticket guidance only
**Deps**: `docs/FOUNDATIONS.md`, `docs/golden-e2e-testing.md`, `specs/S15-start-failure-emergence-golden-suites.md`, archived `S15STAFAIEME-001`

## Problem

Recent S15 work exposed a recurring documentation ambiguity: tickets and specs can collapse two different production contention seams into one vague "the local harvest opportunity vanished" story.

In current architecture, ordinary harvest contention is split across two authoritative layers:

1. start-time contention at the shared reservation/start gate
2. later source depletion at harvest commit in production actions

When docs blur those seams, tickets overstate what a scenario is proving, reviewers have to rediscover the real architecture from code, and golden assertions risk encoding the wrong causal explanation.

## Assumption Reassessment (2026-03-19)

1. Current authoritative harvest behavior is split across two layers:
   - shared start/reservation handling in [crates/worldwake-sim/src/start_gate.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/start_gate.rs)
   - source consumption at commit in [crates/worldwake-systems/src/production_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/production_actions.rs)
2. Existing focused coverage already proves part of this seam:
   - `production_actions::tests::harvest_reservation_blocks_second_actor_and_abort_preserves_source`
   - `production_actions::tests::harvest_happy_path_reduces_source_and_creates_output_lot`
   - `tick_step::tests::best_effort_request_drops_recoverable_start_failure_without_failing_tick`
3. Existing golden guidance in [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) is strong on assertion-layer choice, but it does not explicitly teach contributors to distinguish production start-gate contention from commit-time depletion when writing tickets or goldens.
4. The active S15 spec in [specs/S15-start-failure-emergence-golden-suites.md](/home/joeloverbeck/projects/worldwake/specs/S15-start-failure-emergence-golden-suites.md) still contains wording that can be read as "only one can actually start successfully" because the source can satisfy only one immediate harvest. That phrasing was understandable during planning, but it is imprecise against the current reservation-plus-commit architecture.
5. Ordering here is mixed-layer and not weight-only. Same-snapshot goal selection can be symmetric while authoritative divergence happens first at start and only later at commit. Docs should name both layers explicitly.
6. This is a docs/spec precision gap, not a production-code defect. The goal is to make the architecture more legible, not to redesign harvest contention.
7. Mismatch found and corrected: older ticket/spec language implied source disappearance before start in ordinary contested harvest. The real engine path is reservation/start failure first, depletion later at commit.

## Architecture Check

1. Clarifying the seam in docs is cleaner than letting each future ticket rediscover it. It preserves a single authoritative explanation for the current architecture and reduces accidental test overfitting.
2. This aligns with `docs/FOUNDATIONS.md` because it makes causal chains and information flow more legible without introducing shims, aliases, or alternate rule paths.
3. No backward-compatibility language should be added. The docs should describe the current architecture plainly and retire the stale wording.

## Verification Layers

1. Shared start-gate contention is the first failure seam for ordinary contested harvest -> cite focused runtime tests and authoritative code references.
2. Source depletion happens later at successful harvest commit -> cite focused production tests and authoritative code references.
3. Golden scenarios should choose the right mixed-layer assertions for each seam -> update golden testing guidance in docs.
4. No additional verification layer is needed because this ticket is documentation-only; it should point at existing focused and golden proof surfaces rather than duplicate them.

## What to Change

### 1. Update golden testing guidance

Revise [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) to add explicit guidance for production contention:

- distinguish start-gate contention from commit-time source depletion
- state which seam a ticket or golden is proving
- avoid writing "the source vanished before start" unless that is actually the authoritative rule path under test

### 2. Tighten S15 spec wording

Revise [specs/S15-start-failure-emergence-golden-suites.md](/home/joeloverbeck/projects/worldwake/specs/S15-start-failure-emergence-golden-suites.md) so Scenario 26 explicitly matches the current architecture:

- same-snapshot local selection
- losing contender fails at the shared start gate
- winner later changes the local source at commit
- loser then recovers through ordinary replanning

### 3. Add a short cross-reference note to ticketing guidance

Update [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) with one precision reminder for mixed-layer contention tickets:

- when a contested affordance has separate start-time and commit-time seams, tickets must name which seam is under test and which later mutation is only downstream consequence

## Files to Touch

- `docs/golden-e2e-testing.md` (modify)
- `specs/S15-start-failure-emergence-golden-suites.md` (modify)
- `tickets/README.md` (modify)

## Out of Scope

- changing harvest reservations, production commit semantics, or action framework behavior
- adding new trace payloads
- adding new runtime tests beyond what is needed to validate doc examples, if any

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
2. `cargo test -p worldwake-systems production_actions::tests::harvest_reservation_blocks_second_actor_and_abort_preserves_source -- --exact`
3. `cargo test -p worldwake-systems production_actions::tests::harvest_happy_path_reduces_source_and_creates_output_lot -- --exact`

### Invariants

1. Docs describe the real production contention architecture without implying a false earlier failure seam.
2. Mixed-layer tickets and goldens remain explicit about whether they are proving start-time divergence or later commit-time mutation.
3. No backward-compatibility wording or alternate architecture paths are introduced to preserve stale ticket language.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
2. `cargo test -p worldwake-systems production_actions::tests::harvest_reservation_blocks_second_actor_and_abort_preserves_source -- --exact`
3. `cargo test -p worldwake-systems production_actions::tests::harvest_happy_path_reduces_source_and_creates_output_lot -- --exact`
