# S88TWOPHALAN-007: Integrate two-phase planning into search_plan()

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — modifies planner search loop in worldwake-ai
**Deps**: S88TWOPHALAN-001, S88TWOPHALAN-002, S88TWOPHALAN-003, S88TWOPHALAN-004, S88TWOPHALAN-005, S88TWOPHALAN-006

## Problem

The individual two-phase planning components existed as isolated modules. `search_plan()` still ran a flat single-frontier search and never consumed the staged strategic planner, landmark extraction, or preferred-operator frontier.

## Assumption Reassessment (2026-04-11)

1. `search_plan()` was still using a single `BinaryHeap`, spatial-only heuristic scoring, and flat goal-scoped search.
2. `CognitiveProfile.landmark_extraction_depth` and `ExecutionBudget.preferred_operator_boost` were already present and available at the planner root.
3. The original ticket overstated the live operator bridge. `SearchCandidate` plus `PlannerOpSemantics` does not expose explicit preconditions/add/del effects, so `PlanningOperator` had to be derived from actual before/after `PlanningState` transitions built by `build_successor_detailed()`.
4. The original ticket also overstated the safe integration scope. The strategic module from `S88TWOPHALAN-006` can compute routes for many goals, but integrating that tactical layer indiscriminately regressed existing planner behavior. The safe live slice for this ticket is the remote `TreatWounds` prerequisite path; `ProduceCommodity` remains staged for later integration.
5. `search_candidates()` itself could stay unchanged. Tactical narrowing can be applied in `search_plan()` after candidate generation.
6. Tactical social-query fallback is not lawfully integrable yet for commodity goals because the current planner has no commodity-query goal surface analogous to the existing epistemic-subject `AskWitness` path. That remains deferred.

## Architecture Check

1. The landed integration preserves the spec’s layer split: strategic planning runs once before tactical search, and the first supported strategic step becomes a temporary tactical context that narrows search until the prerequisite is satisfied.
2. Graceful degradation remains explicit: goals outside the supported two-phase families stay on the flat pre-S88 path, and `landmark_extraction_depth = 0` disables landmark bias while preserving the prerequisite-first plan shape.
3. No compatibility shims were added. The live planner root now uses `DualFrontier`.

## What Changed

1. Added planning-fact extraction helpers in `search/landmarks.rs` for current `PlanningState`, goal facts, and transition-derived operators.
2. Replaced the live `BinaryHeap` in `search_plan()` with `DualFrontier`.
3. Ran `strategic::plan()` before search and converted the first supported strategic step into a tactical context for remote `TreatWounds`.
4. Applied tactical narrowing in `search_plan()` after candidate generation, while leaving `search_candidates()` itself unchanged.
5. Used tactical destination guidance plus `max(spatial_h, landmark_h)` scoring in `build_successor_detailed()`.
6. Cleared the tactical filter once medicine is secured so the planner can continue into the normal heal search path.
7. Kept legacy expansion-summary counts stable for existing tracing tests; trace-enrichment remains owned by S88TWOPHALAN-008.
8. Added focused tests for the remote-treat-wounds prerequisite-first path and zero-landmark degradation.

## Files Touched

- `crates/worldwake-ai/src/search/mod.rs`
- `crates/worldwake-ai/src/search/transition.rs`
- `crates/worldwake-ai/src/search/heuristic.rs`
- `crates/worldwake-ai/src/search/landmarks.rs`
- `crates/worldwake-ai/src/search/tests.rs`

## Out of Scope

- Decision trace enrichment (S88TWOPHALAN-008)
- Golden E2E tests (S88TWOPHALAN-009)
- Broadening candidate generation into a new commodity-query goal surface
- Modifying action framework or world validation

## Acceptance Notes

1. `search_plan()` stays behaviorally unchanged for goals outside the supported two-phase families.
2. Strategic planning remains belief-backed only (FND-14).
3. Landmark operators are derived from successor transitions over believed planning state, not authoritative world reads (FND-14).
4. Remote `TreatWounds` now routes through medicine acquisition before healing while still finding the full care plan.
5. Existing `search_plan` regression coverage remains green.

## Verification

1. `cargo test -p worldwake-ai -- search::tests`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

Completion date: 2026-04-11

What changed:

- `search_plan()` now uses `DualFrontier`
- remote `TreatWounds` can enter a tactical prerequisite phase derived from `strategic::plan()`
- tactical search narrows toward medicine acquisition first, then resumes the normal heal search once medicine is secured
- landmark preference and combined heuristic scoring are derived from actual planning-state transitions, not a synthetic semantics layer

Deviations from original plan:

- The ticket originally assumed a broad two-phase integration across multiple strategic goal families. Reassessment and broadened verification showed that only the remote `TreatWounds` slice was safe to land here without regressing existing conformance and golden coverage.
- The ticket originally assumed direct conversion from `SearchCandidate + PlannerOpSemantics` into landmark operators. The landed implementation instead derives `PlanningOperator` from actual before/after `PlanningState` transitions.

Verification results:

- `cargo test -p worldwake-ai -- search::tests` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
- `cargo test --workspace` passed
