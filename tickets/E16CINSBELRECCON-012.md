# E16CINSBELRECCON-012: Candidate Generation — ConsultRecord + Political Suppression

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extend candidate generation in worldwake-ai
**Deps**: E16CINSBELRECCON-009, E16CINSBELRECCON-010, E16CINSBELRECCON-011

## Problem

The AI candidate generation layer must emit `GoalKind::ConsultRecord` candidates when an agent has `Unknown` institutional beliefs that are relevant to its potential goals. It must also suppress institution-sensitive political goals (ClaimOffice, SupportCandidateForOffice) when the relevant institutional belief is `Conflicted`. Without this, agents will either never seek out institutional knowledge or act on contradictory information.

## Assumption Reassessment (2026-03-22)

1. `candidate_generation.rs` in worldwake-ai generates goal candidates from agent beliefs. It currently generates `ClaimOffice` and `SupportCandidateForOffice` candidates based on political signals.
2. Current political candidate generation reads support declarations and office state. After E16c, these must come from institutional beliefs (PlanningSnapshot), not live truth.
3. When institutional belief is `Unknown` for a relevant office → emit `ConsultRecord` candidate targeting the appropriate record.
4. When institutional belief is `Conflicted` for a relevant office → suppress political goal candidates that require commitment (ClaimOffice, SupportCandidateForOffice).
5. The candidate generation layer does not call `World` directly, but the current political candidate path still reaches live institutional truth indirectly through the legacy belief-view seam (`ctx.view.office_holder()` / `ctx.view.support_declaration()`). This ticket must cut over those reads to the new planning/snapshot institutional-belief surface.
6. N/A — no heuristic removal.
7. N/A.
8. Closure boundary: candidate generation for ClaimOffice and SupportCandidateForOffice. The exact symbols are `generate_candidates()` and the political subsection within it.
9. N/A.
10. ConsultRecord candidates should only be emitted when: (a) the agent has a plausible political goal, (b) the required institutional belief is Unknown, (c) a record of the right kind is known to exist. This prevents agents from randomly consulting records they have no use for.
11. Mismatch + correction: current code in `crates/worldwake-ai/src/candidate_generation.rs` still calls `ctx.view.office_holder()` and `ctx.view.support_declaration()` in the political candidate path. This ticket should migrate those candidate-generation reads onto the new institutional-belief-backed planning/snapshot queries as soon as tickets `-009` and `-010` land; it must not add more logic on top of the legacy live-helper path.
12. Additional live-code clarification after ticket `-008`: Tell-side institutional propagation for entity subjects now exists, so the remaining blocker here is no longer "can institutional facts arrive socially?" It is "does candidate generation consume the new institutional-belief substrate instead of the live helper seam?"

## Architecture Check

1. Extending candidate generation is the right layer — it determines what goals are even considered. Suppression at this layer prevents wasted planning effort on goals that would fail due to conflicted beliefs.
2. No backward-compatibility shims.

## Verification Layers

1. Unknown office holder belief + known record → ConsultRecord candidate emitted → decision trace
2. Certain office holder belief → NO ConsultRecord candidate → decision trace
3. Conflicted office holder belief → ClaimOffice suppressed → decision trace
4. Conflicted support belief → SupportCandidateForOffice suppressed → decision trace

## What to Change

### 1. Extend political candidate generation in `candidate_generation.rs`

Before emitting `ClaimOffice` or `SupportCandidateForOffice` candidates:

1. Query `believed_office_holder(office)` from the new belief-backed planning/snapshot surface, not the legacy live `ctx.view.office_holder()` seam
2. If `Unknown`:
   - Emit `GoalKind::ConsultRecord { record }` for the relevant office register
   - Do NOT emit the political goal (agent doesn't know enough to act)
3. If `Conflicted`:
   - Suppress the political goal candidate
   - Optionally emit `ConsultRecord` to resolve the contradiction
4. If `Certain`:
   - Emit the political goal normally (existing behavior)

### 2. Add ConsultRecord candidate generation helper

A function that, given an `InstitutionalBeliefKey` that is `Unknown`, looks up whether a record of the appropriate kind is known at any place in the agent's beliefs, and if so, returns a `GoalKind::ConsultRecord` candidate.

### 3. Update support declaration candidate generation

`SupportCandidateForOffice` candidates currently use the old office/support read path. These must be sourced from institutional beliefs instead (read via `believed_support_declaration()` and the belief-backed planning/snapshot surface), so this ticket reduces the remaining migration surface before `-014` cuts the seam entirely.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — ConsultRecord emission, Conflicted suppression, belief-based political candidates)

## Out of Scope

- PlanningSnapshot/PlanningState changes (ticket -010 — must already exist)
- GoalKindTag::ConsultRecord (ticket -011 — must already exist)
- Ranking changes (ticket -013)
- Failure handling (ticket -013)
- Live helper seam removal (ticket -014)
- Non-political candidate generation (unchanged)

## Acceptance Criteria

### Tests That Must Pass

1. With Unknown office holder belief and known office register → `ConsultRecord` candidate emitted
2. With Unknown office holder belief and NO known record → no ConsultRecord candidate (nothing to consult)
3. With Certain office holder belief → ClaimOffice candidate emitted normally, no ConsultRecord
4. With Conflicted office holder belief → ClaimOffice candidate suppressed
5. With Conflicted support belief → SupportCandidateForOffice candidate suppressed
6. SupportCandidateForOffice reads from institutional beliefs, not live support declarations
7. Existing non-political candidate generation unchanged
8. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No political candidate generated when required institutional belief is Unknown or Conflicted
2. ConsultRecord candidates only emitted when a relevant record is known to exist
3. Candidate generation reads institutional beliefs from PlanningSnapshot, not live world

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — Unknown/Certain/Conflicted cases for office holder and support declaration, ConsultRecord emission

### Commands

1. `cargo test -p worldwake-ai candidate_generation`
2. `cargo clippy --workspace && cargo test --workspace`
