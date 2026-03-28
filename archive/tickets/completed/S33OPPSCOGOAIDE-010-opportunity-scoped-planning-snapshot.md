# S33OPPSCOGOAIDE-010: Make planning snapshot scope candidate-local by opportunity

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planning-snapshot construction and evidence plumbing
**Deps**: S33OPPSCOGOAIDE-002

## Problem

Archived `S33OPPSCOGOAIDE-002` intentionally deferred the deepest part of the opportunity refactor: plan search still builds from one merged planning snapshot assembled from every admitted ranked candidate. Candidate generation is opportunity-scoped, but search still sees a wider evidence surface than the concrete candidate being searched. In live code this leakage is currently broader than the original ticket narrative described: because `build_candidate_plans()` still performs temporary first-per-`GoalKey` admission, same-desire sibling opportunities do not both reach search yet, but unrelated admitted candidates still leak evidence into each other's search attempts through the shared snapshot.

The shared abstraction boundary under audit is:

- candidate-local evidence on `GroundedGoal`
- planning snapshot construction in `crates/worldwake-ai/src/agent_tick/planning.rs`

One fact currently has two lawful transport paths:

1. the candidate's explicit `evidence_entities` / `evidence_places`
2. the merged planning snapshot assembled once for the full admitted candidate set in `build_candidate_plans()`

After this change, the canonical path must be the candidate-local evidence path. The merged desire-level planning snapshot path must be removed in scope, not left beside it.

## Assumption Reassessment (2026-03-28)

1. Archived `S33OPPSCOGOAIDE-002` already landed `GroundedGoal.anchor` and one-candidate-per-opportunity emission. The missing piece is not candidate identity; it is search scope.
2. Live `build_candidate_plans()` in `crates/worldwake-ai/src/agent_tick/planning.rs` still keeps a temporary first-per-`GoalKey` admission gate before search. That means the current leakage surface is not "multiple same-desire opportunities searched together"; it is "all admitted candidates share one snapshot".
3. The right unit of isolation is the concrete candidate being searched, not the high-level `GoalKey` and not the admitted candidate batch.
4. Archived `S33OPPSCOGOAIDE-002` explicitly documented that removing the merged snapshot earlier regressed legitimate plans. This ticket therefore may need to strengthen candidate evidence derivation if candidate-local search exposes lawful missing evidence.
5. This ticket is distinct from post-rank admission (`S33OPPSCOGOAIDE-005`). Admission ordering decides which opportunities reach search; this ticket decides what information each admitted search attempt is allowed to see.

## Architecture Check

1. Candidate-local planning scope is the cleaner long-term architecture because it preserves locality of information and removes hidden cross-candidate leakage from the planning boundary.
2. If candidate evidence is too weak today, the robust fix is to strengthen the candidate evidence contract at the source, not to reintroduce a shared merged snapshot as a permanent crutch.
3. No backward compatibility, no aliasing. The merged planning snapshot path should be deleted once the candidate-local path is strong enough.

## Verification Layers

1. Focused planning test proving one admitted candidate's evidence does not leak into another candidate's search attempt.
2. Focused evidence-derivation test proving the candidate under search still has enough local evidence for lawful plan discovery if this ticket needs evidence strengthening.
3. AI-focused regression coverage proving previously regressing scenarios still pass with candidate-local snapshot scope.

## What to Change

### 1. Make planning snapshot construction candidate-local

In `crates/worldwake-ai/src/agent_tick/planning.rs`, build the planning/search snapshot from the evidence attached to the specific `GroundedGoal` being searched rather than from a merged admitted-candidate aggregate.

### 2. Strengthen candidate evidence derivation where isolation reveals gaps

If a legitimate plan becomes impossible because the candidate lacks evidence that should lawfully belong to that concrete opportunity, fix the evidence derivation at candidate-generation time. Do not patch around it by widening search scope globally.

### 3. Keep search provenance debuggable

Decision traces or planning-attempt traces should make it obvious which concrete opportunity anchor was searched for each attempt so scope regressions are diagnosable without ad hoc instrumentation.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — candidate-local planning snapshot construction)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify if candidate evidence needs to be strengthened)
- `crates/worldwake-ai/src/decision_trace.rs` (modify if plan-attempt tracing needs stronger opportunity provenance)

## Out of Scope

- Exhaustion re-keying (`S33OPPSCOGOAIDE-004`)
- Post-rank admission ordering (`S33OPPSCOGOAIDE-005`)
- `PlannedPlan.opportunity` persistence (`S33OPPSCOGOAIDE-006`, `S33OPPSCOGOAIDE-008`)
- End-to-end source-switching goldens (`S33OPPSCOGOAIDE-009`)

## Acceptance Criteria

### Tests That Must Pass

1. Search for one admitted candidate does not rely on evidence attached only to a different admitted candidate.
2. Candidate-local search still finds lawful plans for scenarios that should remain plannable.
3. Existing suite: `cargo test -p worldwake-ai`
4. Existing suite: `cargo clippy --workspace`

### Invariants

1. Planning scope is candidate-local, not merged across the admitted candidate batch.
2. Any additional evidence made available to search is carried on the concrete candidate itself.
3. No merged shared-snapshot planning path remains after the change.

## Test Plan

### New/Modified Tests

1. `agent_tick::planning::tests::candidate_search_does_not_use_other_admitted_candidate_evidence`
2. `decision_trace::tests::summary_planning_includes_attempt_anchor`

### Commands

1. `cargo test -p worldwake-ai agent_tick::planning::tests::`
2. `cargo test -p worldwake-ai candidate_generation::tests::`
3. `cargo test -p worldwake-ai decision_trace::tests::`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-28
- What actually changed:
  - `crates/worldwake-ai/src/agent_tick/planning.rs` now builds the planning snapshot inside the per-candidate search loop, using only the searched `GroundedGoal`'s `evidence_entities` and `evidence_places`
  - `crates/worldwake-ai/src/decision_trace.rs` now records `PlanAttemptTrace.opportunity_anchor` and includes that anchor in formatted planning-attempt output
  - focused coverage was added for candidate-local snapshot scope and plan-attempt anchor provenance
- Deviations from original plan:
  - no `candidate_generation` changes were needed; current candidate evidence was already sufficient for existing lawful plans once snapshot scope was narrowed
  - live reassessment corrected the stale ticket narrative from "same-desire opportunity leakage during search" to the broader and more accurate live issue: one shared snapshot across the admitted candidate batch
- Verification results:
  - `cargo test -p worldwake-ai agent_tick::planning::tests:: -- --nocapture` passed
  - `cargo test -p worldwake-ai decision_trace::tests:: -- --nocapture` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
