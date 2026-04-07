# S59EXPOBLSUB-020: Add EvidenceTrace knowledge path entries to emit_escort_candidates

**Status**: PENDING
**Priority**: LOW
**Effort**: Small
**Engine Changes**: Yes — candidate generation traceability
**Deps**: S59EXPOBLSUB-017 (completed, archived)

## Problem

`emit_escort_candidates` in `candidate_generation.rs` passes `EvidenceTrace::default()` when emitting `EscortToSafety` candidates. All other candidate emitters that consume belief state (e.g., `emit_care_goals`, `emit_search_candidates`) populate `EvidenceTrace.knowledge_path` with provenance entries showing which beliefs contributed to the candidate. The escort emitter is missing this traceability, making it harder to debug why a particular escort candidate was generated.

## Assumption Reassessment (2026-04-07)

1. **`emit_escort_candidates` exists at `candidate_generation.rs:3352`** — Confirmed. Passes `EvidenceTrace::default()` at line 3411.
2. **Other care-domain emitters use EvidenceTrace** — Confirmed. `emit_care_goals` at line 2591-2598 pushes `BeliefProvenance` entries with `BeliefAspect::Wounded` and the belief's source/tick.
3. **EvidenceTrace struct** — `EvidenceTrace.knowledge_path.entity_beliefs: Vec<BeliefProvenance>` at `decision_trace.rs`. Requires `BeliefAspect`, `PerceptionSource`, and `Tick`.
4. **Escort candidate reads wound state from `ctx.view.has_wounds(subject)`** — uses `believed_entity(entity).wounds`. The belief's `source` and `observed_tick` are available via `ctx.view.known_entity_beliefs(ctx.agent)`.
5. **No functional impact** — EvidenceTrace is diagnostic only. The candidate is correct regardless of trace completeness.

## Architecture Check

1. Follows the established pattern from `emit_care_goals` exactly — push a `BeliefProvenance` entry with `BeliefAspect::Wounded` for each wounded subject that generates a candidate.
2. No backwards-compatibility concern — additive trace data only.

## Verification Layers

1. Trace completeness -> decision trace inspection (no behavioral change, only diagnostic enrichment)
2. Single-layer ticket -> additional layer mapping not applicable

## What to Change

### 1. Populate EvidenceTrace in emit_escort_candidates

In `crates/worldwake-ai/src/candidate_generation.rs`, within the `emit_escort_candidates` loop body:
- Look up the entity belief for the subject via `ctx.view.known_entity_beliefs(ctx.agent)` to get the belief's `source` and `observed_tick`
- Build an `EvidenceTrace` with a `BeliefProvenance` entry matching the pattern from `emit_care_goals`
- Gate on `ctx.tracing_enabled` to avoid allocation when tracing is off

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — `emit_escort_candidates`)

## Out of Scope

- Adding EvidenceTrace to other emitters that may also be missing it
- Changing candidate generation logic or ranking

## Acceptance Criteria

### Tests That Must Pass

1. Existing suite: `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. EscortToSafety candidates carry wound-belief provenance in their EvidenceTrace when tracing is enabled
2. No functional behavioral change

## Test Plan

### New/Modified Tests

1. None — documentation-only diagnostic enrichment; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
