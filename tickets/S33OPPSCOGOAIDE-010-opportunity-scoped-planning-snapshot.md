# S33OPPSCOGOAIDE-010: Make planning snapshot scope candidate-local by opportunity

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planning-snapshot construction and evidence plumbing
**Deps**: S33OPPSCOGOAIDE-002

## Problem

Archived `S33OPPSCOGOAIDE-002` intentionally deferred the deepest part of the opportunity refactor: plan search still builds from a merged planning snapshot that can contain evidence from multiple concrete opportunities for the same desire. That means candidate generation is opportunity-scoped, but search can still see a wider evidence surface than the candidate being searched. This is the architectural gap that caused the regressions when full isolation was attempted earlier.

The shared abstraction boundary under audit is:

- candidate-local evidence on `GroundedGoal`
- planning snapshot construction in `crates/worldwake-ai/src/agent_tick/planning.rs`

One fact currently has two lawful transport paths:

1. the candidate's explicit `evidence_entities` / `evidence_places`
2. the merged desire-level snapshot assembled for plan search

After this change, the canonical path must be the candidate-local evidence path. The merged desire-level planning snapshot path must be removed in scope, not left beside it.

## Assumption Reassessment (2026-03-28)

1. Archived `S33OPPSCOGOAIDE-002` already landed `GroundedGoal.anchor` and one-candidate-per-opportunity emission. The missing piece is not candidate identity; it is search scope.
2. The attempted full isolation in `002` regressed live goldens because current candidate evidence is not always rich enough to support candidate-local search. That means this ticket may need to strengthen evidence derivation as well as snapshot construction.
3. The right unit of isolation is the concrete opportunity being searched, not the high-level `GoalKey`.
4. This ticket is distinct from post-rank admission (`S33OPPSCOGOAIDE-005`). Admission ordering decides which opportunity gets searched next; this ticket decides what information each search attempt is allowed to see.

## Architecture Check

1. Candidate-local planning scope is the cleaner long-term architecture because it preserves locality of information and removes hidden cross-opportunity leakage.
2. If candidate evidence is too weak today, the robust fix is to strengthen the candidate evidence contract at the source, not to reintroduce a merged desire-level snapshot as a permanent crutch.
3. No backward compatibility, no aliasing. The merged planning snapshot path should be deleted once the candidate-local path is strong enough.

## Verification Layers

1. Focused planning test proving one opportunity's evidence does not leak into another opportunity's search attempt.
2. Focused evidence-derivation test proving the candidate under search still has enough local evidence for lawful plan discovery.
3. AI test proving previously regressing scenarios now pass with candidate-local snapshot scope.

## What to Change

### 1. Make planning snapshot construction candidate-local

In `crates/worldwake-ai/src/agent_tick/planning.rs`, build the planning/search snapshot from the evidence attached to the specific `GroundedGoal` being searched rather than from a merged desire-level aggregate.

### 2. Strengthen candidate evidence derivation where isolation reveals gaps

If a legitimate plan becomes impossible because the candidate lacks evidence that should lawfully belong to that concrete opportunity, fix the evidence derivation at candidate-generation time. Do not patch around it by widening search scope globally.

### 3. Keep search provenance debuggable

Decision traces or planning-attempt traces should make it obvious which evidence scope was used for each search attempt so regressions are diagnosable without ad hoc instrumentation.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — candidate-local planning snapshot construction)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify if candidate evidence needs to be strengthened)
- `crates/worldwake-ai/src/decision_trace.rs` (modify if trace output needs stronger planning-scope provenance)

## Out of Scope

- Exhaustion re-keying (`S33OPPSCOGOAIDE-004`)
- Post-rank admission ordering (`S33OPPSCOGOAIDE-005`)
- `PlannedPlan.opportunity` persistence (`S33OPPSCOGOAIDE-006`, `S33OPPSCOGOAIDE-008`)
- End-to-end source-switching goldens (`S33OPPSCOGOAIDE-009`)

## Acceptance Criteria

### Tests That Must Pass

1. Search for one opportunity does not rely on evidence attached only to a competing opportunity.
2. Candidate-local search still finds lawful plans for scenarios that should remain plannable.
3. Existing suite: `cargo test -p worldwake-ai`
4. Existing suite: `cargo clippy --workspace`

### Invariants

1. Planning scope is candidate-local, not merged by desire.
2. Any additional evidence made available to search is carried on the concrete candidate itself.
3. No merged desire-level planning snapshot path remains after the change.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` — `planning_snapshot_is_scoped_to_candidate_opportunity`
2. `crates/worldwake-ai/src/candidate_generation.rs` — focused test for any strengthened evidence derivation needed to keep lawful plans searchable
3. Decision-trace or planning-attempt test verifying per-attempt scope provenance if tracing is extended

### Commands

1. `cargo test -p worldwake-ai -- planning`
2. `cargo test -p worldwake-ai -- candidate_generation`
3. `cargo clippy --workspace`
4. `cargo test --workspace`
