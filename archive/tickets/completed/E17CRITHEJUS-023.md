# E17CRITHEJUS-023: Document the planner exact-goal, snapshot, and traceability contracts

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: archive/tickets/completed/E17CRITHEJUS-020.md, archive/tickets/completed/E17CRITHEJUS-021.md, archive/tickets/completed/E17CRITHEJUS-022.md, archive/tickets/completed/E17CRITHEJUS-024.md

## Problem

The architectural lessons from `E17CRITHEJUS-008` are now clearer than the current docs. The repository explains mixed-layer ticket drafting and traceability escalation well, but it does not yet document the specific planner contracts that matter here:

- when a live goal may surface a terminal operator from goal identity
- which planner-visible runtime belief data must survive into `PlanningSnapshot`, including the now-centralized dynamic duration-backed dependency inventory
- what decision traces should explain when a relevant operator is absent or a planner prerequisite is missing

Without those contracts written down, future tickets are likely to repeat the same stale-narrative failure mode: a narrow symptom gets treated as the bug while the shared planner boundary remains implicit.

## Assumption Reassessment (2026-03-26)

1. `AGENTS.md`, `tickets/README.md`, and `docs/precision-rules.md` already require assumption reassessment, exact boundary naming, verification-layer mapping, and traceability escalation. The live gap is planner-specific contract documentation, not missing general process rules.
2. The exact shared abstraction boundary under audit is the planner root-contract path in `worldwake-ai`: `GoalKindPlannerExt::{relevant_op_kinds, build_payload_override, matches_binding}` plus `GroundedGoal::synthesized_root_candidate_targets()` in `crates/worldwake-ai/src/goal_model.rs`, `PlanningSnapshot` construction in `crates/worldwake-ai/src/planning_snapshot.rs`, snapshot-backed reads in `crates/worldwake-ai/src/planning_state.rs`, root-candidate surfacing/omission recording in `crates/worldwake-ai/src/search/candidates.rs`, and omission/skip diagnostics in `crates/worldwake-ai/src/decision_trace.rs`.
3. The dependency tickets are already completed and archived. `E17CRITHEJUS-020` made exact-goal terminal synthesis explicit, `E17CRITHEJUS-021` and `E17CRITHEJUS-024` closed and centralized the dynamic duration-backed snapshot contract, and `E17CRITHEJUS-022` added omitted-root-operator and named prerequisite diagnostics. This ticket must document those delivered contracts, not speculate about future landing order.
4. No single current doc explains the planner path end to end: relevant operators -> root candidate surfacing or omission -> payload/target synthesis -> snapshot completeness for planner-visible duration dependencies -> decision-trace omission/prerequisite diagnostics. Archived tickets contain the history, but the repository still lacks one authoritative planner-facing contract doc.
5. The live exact-goal terminal surface is narrower and more precise than the original narrative implied. Today `GroundedGoal::synthesized_root_candidate_targets()` covers exact `GoalKind`-bound or grounded-evidence-bound terminal synthesis for `Tell`, `Investigate`, `Accuse`, `PressForceClaim`, and exact local `Trade` cases; deferred `GoalKind::PunishAccused` remains intentionally absent from the live root-candidate surface.
6. The snapshot-completeness contract is no longer “if `E17CRITHEJUS-024` lands first.” It is already centralized in `crates/worldwake-ai/src/planner_duration_contract.rs` as the authoritative inventory for planner-supported non-fixed duration dependencies, with preservation split between `PlanningSnapshot` and `PlanningState`.
7. This remains a documentation-only ticket. Verification is doc-content inspection plus regression safety that planner/runtime tests, build, and lint still pass after doc edits. No production behavior change is expected.
8. `docs/FOUNDATIONS.md` remains the governing design source and should not be duplicated. The new planner doc should explain how these planner contracts serve explainable emergence, locality of information, concrete-state planning inputs, and rejection of workaround architecture.
9. Adjacent process guidance already exists in archived `E17CRITHEJUS-019`. This ticket should add planner-specific contributor guidance, not reopen the broader ticket-authoring contract.
10. Mismatch + correction: the missing artifact is not “more general process text,” and it is not “document a possibly future dynamic-duration contract.” The missing artifact is a current-state planner contract document aligned to the live code and completed follow-up tickets.

## Architecture Check

1. The clean fix is to document the exact live planner contracts in one authoritative planner-facing doc and add only the smallest contributor-facing cross-reference needed to route future work there.
2. That is cleaner than scattering the contract across archived tickets, code comments, and oral history from individual bugfixes.
3. It is also cleaner than embedding the full planner contract directly into `AGENTS.md` or `docs/precision-rules.md`, because those files own process guidance, not planner architecture.
4. This aligns with `docs/FOUNDATIONS.md`: make the causal contract explicit, keep one source of truth, and avoid workaround lore or parallel documentation paths.

## Verification Layers

1. Exact-goal terminal surfacing contract is documented against the final live planner architecture -> doc content inspection
2. Planning-snapshot completeness and centralized duration-dependency inventory are documented against the final live planner architecture -> doc content inspection
3. Traceability expectations for omitted operators and named missing prerequisites are documented without duplicating `docs/precision-rules.md` -> doc content inspection
4. Documentation-only ticket; runtime proof remains regression safety via existing AI tests/build/lint rather than new behavior assertions

## What to Change

### 1. Add planner contract documentation

Create or update one planner-focused doc that explains:
- exact-goal terminal operator surfacing
- the snapshot completeness contract for planner-visible runtime belief data, including the already-centralized dynamic duration-backed planner contract
- traceability expectations for omitted relevant operators and named missing prerequisites

### 2. Add concise contributor cross-references

Update the smallest useful set of contributor docs so future ticket authors and implementers know where the planner contract lives and when to consult it.

### 3. Keep documentation aligned with the foundations

Tie the planner contract back to the relevant foundational principles instead of restating them wholesale.

## Files to Touch

- `AGENTS.md` (modify)
- `docs/precision-rules.md` (modify only if a planner-specific cross-reference is needed)
- `docs/planner-contracts.md` (new)

## Out of Scope

- Any planner behavior changes
- Rewriting `docs/FOUNDATIONS.md`
- Broad process-doc rewrites unrelated to planner/search/snapshot/traceability contracts

## Acceptance Criteria

### Tests That Must Pass

1. The repository contains one authoritative planner-facing doc covering exact-goal terminal surfacing, snapshot completeness, and traceability expectations.
2. `AGENTS.md` points contributors to that planner contract without duplicating the full content.
3. The new/updated docs remain consistent with `docs/FOUNDATIONS.md`, `tickets/README.md`, and `docs/precision-rules.md`.
4. Existing suite: `cargo test -p worldwake-ai`
5. Regression safety passes: `cargo build --workspace` and `cargo clippy --workspace`

### Invariants

1. Future planner tickets can cite one explicit contract instead of relying on stale narratives from prior bugfixes.
2. Documentation remains non-duplicative: foundations stay foundational, precision rules stay canonical for claim hygiene, and planner docs capture planner-specific architecture only.

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; no production behavior or test harness contract changes are proposed.

### Commands

1. `sed -n '1,260p' AGENTS.md`
2. `sed -n '1,260p' docs/planner-contracts.md`
3. `sed -n '1,260p' docs/precision-rules.md`
4. `cargo test -p worldwake-ai`
5. `cargo build --workspace`
6. `cargo clippy --workspace`

## Outcome

Completed: 2026-03-26

What actually changed:
- Added `docs/planner-contracts.md` as the single planner-facing contract doc for exact-goal terminal surfacing, snapshot completeness, and planner traceability.
- Updated `AGENTS.md` with a concise cross-reference so future planner tickets consult the live contract instead of reconstructing behavior from archived bugfix tickets.
- Corrected this ticket's assumptions to reflect the live code and completed dependency tickets rather than future-tense narratives.

Deviations from original plan:
- `docs/precision-rules.md` did not need modification. The existing rules were already sufficient once the dedicated planner contract doc existed.
- No production or test code changes were required because reassessment confirmed the architecture gap was documentation, not planner behavior.

Verification results:
- `cargo test -p worldwake-ai` ✅
- `cargo build --workspace` ✅
- `cargo clippy --workspace` ✅
