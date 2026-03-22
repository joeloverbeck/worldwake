# E16CINSBELRECCON-012: Candidate Generation — Belief-Backed Political Emission And Conflict Suppression

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extend candidate generation in worldwake-ai
**Deps**: E16CINSBELRECCON-009, E16CINSBELRECCON-010, E16CINSBELRECCON-011

## Problem

The AI candidate generation layer still generates political goals from the legacy live institutional helper seam (`office_holder()` / `support_declaration()`) instead of from captured institutional beliefs. After the ticket `-011` correction, candidate generation should continue to emit end goals (`ClaimOffice`, `SupportCandidateForOffice`) and let planning insert `ConsultRecord` mid-plan when knowledge is missing. It should also suppress institution-sensitive political goals when the relevant institutional belief is `Conflicted`, because committed political action on contradictory institutional reads is not robust.

## Assumption Reassessment (2026-03-22)

1. `candidate_generation.rs` in worldwake-ai generates goal candidates from agent beliefs. It currently generates `ClaimOffice` and `SupportCandidateForOffice` candidates based on political signals.
2. Current political candidate generation reads support declarations and office state. After E16c, these must come from institutional beliefs (PlanningSnapshot), not live truth.
3. Mismatch + correction after ticket `-011`: candidate generation should not emit `GoalKind::ConsultRecord` candidates. `ConsultRecord` is a planner operation, not a top-level desired world condition.
4. When institutional belief is `Unknown` for a relevant office, candidate generation should still emit the political end goal only if the goal remains plausibly actionable through the known belief substrate, such as when a matching consultable record is known. Planning then inserts `ConsultRecord` as a prerequisite.
5. When institutional belief is `Conflicted` for a relevant office, candidate generation should suppress commitment-requiring political goals (`ClaimOffice`, `SupportCandidateForOffice`) until the contradiction is resolved.
6. The candidate generation layer does not call `World` directly, but the current political candidate path still reaches live institutional truth indirectly through the legacy belief-view seam (`ctx.view.office_holder()` / `ctx.view.support_declaration()`). This ticket must cut over those reads to the new institutional-belief-backed surface.
7. N/A — no heuristic removal.
8. N/A.
9. Closure boundary: candidate generation for ClaimOffice and SupportCandidateForOffice. The exact symbols are `generate_candidates()` and the political subsection within it.
10. N/A.
11. Ticket `-011` established the intended division of labor: candidate generation names political end goals, and planning/search own consult insertion. This ticket must follow that division rather than reintroducing a consult-goal family through candidate generation.
12. The E16c spec still says "emit `GoalKind::ConsultRecord` when beliefs are `Unknown`." That is now a documented spec drift. This ticket should follow the corrected architecture proven in `-011`, not the outdated spec wording.
13. Mismatch + correction: current code in `crates/worldwake-ai/src/candidate_generation.rs` still calls `ctx.view.office_holder()` and `ctx.view.support_declaration()` in the political candidate path. This ticket should migrate those reads onto institutional-belief-backed queries; it must not add more logic on top of the legacy live-helper path.
14. Additional live-code clarification after ticket `-008`: Tell-side institutional propagation for entity subjects now exists, so the remaining blocker here is no longer "can institutional facts arrive socially?" It is "does candidate generation consume the institutional-belief substrate instead of the live helper seam?"
15. Additional migration note: once ticket `-010` lands, this ticket should treat the institutional-belief-backed planner/query surface as the only acceptable read source for political candidate generation. Do not retain `GoalBeliefView::office_holder()` / `support_declaration()` as fallback reads for "transitional" behavior; ticket `-014` owns final seam deletion, but this ticket should stop consuming it.

## Architecture Check

1. Extending candidate generation is the right layer for deciding whether a political end goal is even worth considering. Suppression at this layer prevents wasted planning effort on goals that would fail due to conflicted beliefs.
2. Emitting `ConsultRecord` here would be the wrong architecture. It would turn an enabling step into a first-class goal and split political reasoning across two goal families that ranking, tracing, and failure handling would all need to reconcile.
3. The cleaner model is: candidate generation emits the political end goal when it is plausibly actionable; planning determines whether consult is needed as a prerequisite step.
4. No backward-compatibility shims.

## Verification Layers

1. Unknown office holder belief + known consultable record → political end-goal candidate emitted, with planner responsible for later consult insertion
2. Unknown office holder belief + no consultable record → political goal omitted with explicit diagnostic reason
3. Certain office holder belief → political candidate emitted normally without special consult handling
4. Conflicted office holder belief → `ClaimOffice` suppressed → decision trace
5. Conflicted support belief → `SupportCandidateForOffice` suppressed → decision trace

## What to Change

### 1. Extend political candidate generation in `candidate_generation.rs`

Before emitting `ClaimOffice` or `SupportCandidateForOffice` candidates:

1. Query the relevant institutional belief reads from the new belief-backed surface, not the legacy live `ctx.view.office_holder()` / `ctx.view.support_declaration()` seam.
2. If the required institutional belief is `Certain`, emit the political goal normally.
3. If the required institutional belief is `Unknown`:
   - emit the political end goal only when a matching consultable record is known so planning has a lawful consult path
   - otherwise omit the political goal with an explicit diagnostic reason rather than fabricating a consult-specific candidate
4. If the required institutional belief is `Conflicted`:
   - suppress the political goal candidate
   - record an explicit diagnostic reason

### 2. Extend political omission diagnostics as needed

`PoliticalCandidateOmissionReason` currently only covers vacancy, eligibility, and already-declared cases. If institutional-belief gating is added here, extend the omission taxonomy so decision traces can distinguish:
- belief unknown with no consultable record
- belief conflicted

### 3. Update support declaration candidate generation

`SupportCandidateForOffice` candidates currently use the old office/support read path. These must be sourced from institutional beliefs instead, so this ticket reduces the remaining migration surface before `-014` cuts the seam entirely.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — belief-backed political candidates and institutional gating)
- `crates/worldwake-ai/src/decision_trace.rs` (modify if needed — explicit omission reasons for unknown/conflicted institutional beliefs)

## Out of Scope

- PlanningSnapshot/PlanningState changes (ticket -010 — must already exist)
- ConsultRecord planner integration (ticket -011 — completed as planner-op-only architecture)
- Ranking changes (ticket -013)
- Failure handling (ticket -013)
- Live helper seam removal (ticket -014)
- Non-political candidate generation (unchanged)

## Acceptance Criteria

### Tests That Must Pass

1. With Unknown office holder belief and known office register → `ClaimOffice` or `SupportCandidateForOffice` candidate may still emit when the end goal is otherwise plausible, and no `ConsultRecord` top-level candidate is emitted
2. With Unknown office holder belief and NO known record → political candidate omitted with explicit diagnostic reason
3. With Certain office holder belief → political candidate emitted normally
4. With Conflicted office holder belief → `ClaimOffice` candidate suppressed
5. With Conflicted support belief → `SupportCandidateForOffice` candidate suppressed
6. SupportCandidateForOffice reads from institutional beliefs, not live support declarations
7. Candidate generation emits no `GoalKind::ConsultRecord` candidates
8. Existing non-political candidate generation unchanged
9. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Candidate generation emits political end goals, not consult-specific goals
2. Conflicted institutional beliefs suppress commitment-requiring political goals
3. Unknown institutional beliefs only allow political candidate emission when the end goal still has a known consultable path
4. Candidate generation reads institutional beliefs from the belief-backed surface, not live world helpers
5. Candidate generation does not add any new dependence on legacy institutional methods on `GoalBeliefView`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — Unknown/Certain/Conflicted cases for office holder and support declaration, with political end-goal emission and suppression rules
2. `crates/worldwake-ai/src/decision_trace.rs` — omission reason coverage if institutional gating adds new diagnostics

### Commands

1. `cargo test -p worldwake-ai candidate_generation`
2. `cargo test -p worldwake-ai decision_trace`
3. `cargo clippy --workspace && cargo test --workspace`
