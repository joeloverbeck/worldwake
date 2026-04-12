# STRATBUDGET-001: Investigate and resolve strategic planner budget independence from CognitiveProfile

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `crates/worldwake-ai/src/search/strategic.rs` documentation and focused proof only
**Deps**: None

## Problem

The strategic planner's search budget is derived solely from `ExecutionBudget.max_prerequisite_locations * 2` (default: 6 expansions), completely independent of `CognitiveProfile.max_node_expansions` (default: 224). No comments document whether this independence is intentional. This means agents with different cognitive profiles — designed to model different reasoning capacities (P22 agent diversity) — all receive the same strategic search budget. Either the independence is a deliberate architectural choice (strategic search is fundamentally different from tactical search and should not scale with cognitive capacity) or it's an oversight where strategic planning should respect per-agent cognitive variation.

## Assumption Reassessment (2026-04-12)

1. `search/strategic.rs` computes the strategic planner expansion cap as `usize::max(1, usize::from(execution_budget.max_prerequisite_locations) * 2)`. With default `max_prerequisite_locations: 3`, this yields a budget of 6. Tactical search at `search/mod.rs` separately uses `cognitive.max_node_expansions` (default: 224). The asymmetry is confirmed in current code.
2. The live planner-contract doc does not currently describe strategic-search budgeting, so the source of truth is the landed S88 implementation chain rather than `docs/planner-contracts.md`.
3. Archived implementation ticket `archive/tickets/S88TWOPHALAN-006.md` explicitly scoped strategic planning to `ExecutionBudget` and explicitly specified `Budget = max_prerequisite_locations * 2` expansions. That ticket treats the strategic budget as a bounded itinerary-search cap tied to prerequisite-location branching, not as a cognitive node-expansion budget.
4. Archived spec `archive/specs/S88-two-phase-landmark-planning.md` also split planner diversity intentionally: `landmark_extraction_depth` stayed on `CognitiveProfile`, while `preferred_operator_boost` lived on `ExecutionBudget`. Live code preserves that same split today.
5. This remains a single-layer ticket within `worldwake-ai`. The exact boundary under audit is the strategic planner's internal expansion cap in `crates/worldwake-ai/src/search/strategic.rs`, consumed by `search/mod.rs` before tactical search begins.
6. No adjacent contradictions found. The live mismatch is documentation/proof debt: the invariant is already implemented but not explained or directly tested at the strategic boundary.

## Architecture Check

1. The live architecture already resolves the ambiguity: strategic budgeting belongs to `ExecutionBudget`, not `CognitiveProfile`. S88's landed contract defined the strategic planner as a bounded itinerary search whose cap follows prerequisite-location branching (`max_prerequisite_locations * 2`), while cognitive diversity for tactical search stays on `CognitiveProfile`.
2. The clean fix is therefore documentation plus focused proof in `strategic.rs`, not a new `CognitiveProfile` field or a signature change. Adding `max_strategic_expansions` now would create a second strategic-budget dial with no spec support and would contradict the already-landed S88 boundary.
3. No backwards-compatibility aliasing or shims are required.

## Verification Layers

1. Strategic budget remains derived from `ExecutionBudget.max_prerequisite_locations` with a minimum of 1 → focused unit test in `search/strategic.rs`
2. Strategic planner module still integrates cleanly with existing AI search behavior → `cargo test -p worldwake-ai -- strategic`
3. Single-layer ticket within `worldwake-ai`; CI-matching lint remains the broadened proof surface.

## What to Change

1. Record the reassessed design intent in this ticket: S88 already made the strategic budget an `ExecutionBudget` concern.
2. Add a local helper or equivalent documented boundary in `crates/worldwake-ai/src/search/strategic.rs` explaining why the strategic cap follows `max_prerequisite_locations` instead of `CognitiveProfile.max_node_expansions`.
3. Add a focused test proving the formula remains `max(1, max_prerequisite_locations * 2)`.

## Files to Touch

- `crates/worldwake-ai/src/search/strategic.rs` (modify — document the strategic budget contract and add focused proof)

## Out of Scope

- Refactoring `agent_tick/planning.rs` module structure
- Changing tactical search budget derivation
- Modifying `ExecutionBudget` or `CognitiveProfile` fields

## Acceptance Criteria

### Tests That Must Pass

1. New or updated focused test in `search/strategic.rs` validating that the strategic budget remains derived from `ExecutionBudget.max_prerequisite_locations`
2. Existing suite: `cargo test -p worldwake-ai -- strategic`
3. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Strategic search budget must be >= 1 (already enforced by `usize::max(1, ...)`)
2. Tactical `CognitiveProfile.max_node_expansions` semantics remain unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/strategic.rs` (tests module) — focused test validating the strategic budget derivation and minimum floor

### Commands

1. `cargo test -p worldwake-ai -- strategic`
2. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-12.

Outcome amended: 2026-04-12.

- Reassessed the ticket against the live S88 implementation chain and corrected the owned contract: the strategic planner's expansion cap is intentionally an `ExecutionBudget` concern, not a `CognitiveProfile` concern.
- Added a small local helper in `crates/worldwake-ai/src/search/strategic.rs` with an inline contract comment explaining why strategic budgeting follows `max_prerequisite_locations`.
- Added focused unit coverage proving the strategic budget remains `max(1, max_prerequisite_locations * 2)`.

## Deviations

- The original ticket framed this as an open architecture choice between documenting intent and changing `CognitiveProfile`. Reassessment resolved that ambiguity from archived S88 sources, so no `worldwake-core` field change or planner signature change was required.
- `cargo fmt --all` briefly reformatted an unrelated tracked test file during implementation; that spillover was restored before closeout, so the landed edit surface stayed scoped to `crates/worldwake-ai/src/search/strategic.rs` plus this active ticket.

## Verification Result

- Passed `cargo test -p worldwake-ai -- strategic_search_budget_tracks_execution_budget_stage_cap`
- Passed `cargo test -p worldwake-ai -- strategic`
- Failed `cargo clippy --workspace --all-targets -- -D warnings` due to pre-existing unrelated `clippy::too_many_arguments` findings in `crates/worldwake-ai/src/search/candidates.rs` at `search_candidates_with_expansion_trace` and `apply_commodity_relevance_filter_with_expansion_trace`
- Archived ticket status: untracked (`?? archive/tickets/STRATBUDGET-001-strategic-planner-budget-independence.md`) after move from the untracked active path; original `tickets/STRATBUDGET-001-strategic-planner-budget-independence.md` no longer exists
