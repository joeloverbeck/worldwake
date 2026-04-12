# CANDCLIP-001: Bundle search candidate trace parameters to satisfy clippy arity limits

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `crates/worldwake-ai/src/search/candidates.rs`, `crates/worldwake-ai/src/search/mod.rs`, `crates/worldwake-ai/src/search/tests.rs`, `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs`
**Deps**: None

## Problem

`cargo clippy --workspace --all-targets -- -D warnings` currently fails in `crates/worldwake-ai/src/search/candidates.rs` because two helper functions exceed the repo's allowed argument-count limit:

- `search_candidates_with_expansion_trace(...)` takes 12 parameters
- `apply_commodity_relevance_filter_with_expansion_trace(...)` takes 9 parameters

These are real CI blockers on the live branch, not ticket-local fallout from the recent strategic-budget documentation change. Until they are cleaned up, unrelated tickets that honestly run the repo's required clippy surface inherit a broader verification failure outside their owned behavior.

## Assumption Reassessment (2026-04-12)

1. `cargo clippy --workspace --all-targets -- -D warnings` fails on the current branch with `clippy::too_many_arguments` at `crates/worldwake-ai/src/search/candidates.rs:180` and `crates/worldwake-ai/src/search/candidates.rs:447`.
2. The failing functions are planner-internal helpers in one module. The live boundary under audit is local function shape in `crates/worldwake-ai/src/search/candidates.rs`, not a cross-crate API.
3. Both functions carry clusters of related optional trace sinks (`binding_rejections`, `expansion_candidates`, `root_candidates`, `root_omissions`) alongside normal planner inputs. That is a concrete local signal that the lawful cleanup is to bundle trace-only carriage rather than weaken clippy or scatter allow-attributes.
4. This ticket is not motivated by a behavior regression. The owned contract is CI/lint cleanliness for the canonical planner helper path.
5. No active ticket currently owns these exact `search/candidates.rs` clippy failures; they were surfaced during broadened verification for `STRATBUDGET-001`.
6. Live callers and focused proofs reuse these helper signatures from `crates/worldwake-ai/src/search/mod.rs` and `crates/worldwake-ai/src/search/tests.rs`, so those files are part of the lawful local edit surface.
7. After the candidate-helper arity cleanup, the required repo-wide clippy command still fails on `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` for one nearby report-helper arity violation and two `Option` formatting lints. Those are separate live blockers on the owned verification command, so the ticket must absorb them rather than claiming completion with a still-red CI surface.

## Architecture Check

1. Bundling the trace-only optional outputs into a small local struct (or equivalent local carrier) is cleaner than adding `#[allow(clippy::too_many_arguments)]` because it preserves the repo's CI contract while making the helper boundary reflect the real data grouping.
2. The cleanup stays local to `worldwake-ai`: candidate-helper contexts in `search/candidates.rs`, direct callers in `search/mod.rs`, focused proofs in `search/tests.rs`, and the nearby golden trace-report helper that also blocks the same required clippy command.
3. No backwards-compatibility shim or second helper path is needed.

## Verification Layers

1. Planner helper boundary no longer violates clippy arity limits -> `cargo clippy --workspace --all-targets -- -D warnings`
2. Candidate-generation behavior remains intact after the signature cleanup -> focused `worldwake-ai` search/candidate tests or the narrowest crate proof that exercises these helpers
3. Single-module ticket otherwise; no separate cross-system layer mapping is required

## What to Change

### 1. Reassess the local helper boundary

Inspect `crates/worldwake-ai/src/search/candidates.rs` and identify the lawful grouping for the trace-only optional outputs passed through the two failing helpers.

### 2. Refactor the local signatures

Reduce argument count without changing planner behavior. Prefer a local struct or similarly explicit carrier over lint allows.

### 3. Update local callers and proof

Update the helper call sites in `search/mod.rs`, refresh the focused search proof call sites in `search/tests.rs`, and clear the remaining nearby `worldwake-ai` golden-test clippy blockers that keep the required repo-wide verification red.

## Files to Touch

- `crates/worldwake-ai/src/search/candidates.rs` (modify)
- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` (modify)

## Out of Scope

- Behavior changes to candidate generation or planner semantics
- Relaxing repo clippy settings
- Refactoring unrelated search modules beyond the direct caller/test surface needed for this lint cleanup

## Acceptance Criteria

### Tests That Must Pass

1. Existing focused `worldwake-ai` proof covering the affected candidate helper path
2. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Candidate-generation behavior remains unchanged
2. No `#[allow(clippy::too_many_arguments)]` is added for these helpers

## Test Plan

### New/Modified Tests

1. `None — helper-shape cleanup; use existing focused planner/search proof plus clippy verification unless reassessment shows a local focused test gap`

### Commands

1. `cargo test -p worldwake-ai -- search`
2. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completion date: 2026-04-12

1. Bundled search-candidate planner inputs into local `CandidateSearchContext` and `CommodityFilterContext` carriers, and bundled optional trace sinks into `CandidateTraceSinks` and `CandidateFilterTraceSinks`, removing the clippy arity violations from `crates/worldwake-ai/src/search/candidates.rs` without adding lint allows.
2. Updated the direct search-planner caller path in `crates/worldwake-ai/src/search/mod.rs` and the focused helper-call test surface in `crates/worldwake-ai/src/search/tests.rs` to use the new local carriers.
3. Absorbed the additional live verification blockers discovered during reassessment in `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` by bundling scenario-report inputs into a local struct and replacing the flagged `map(...).unwrap_or_else(...)` formatting sites with `map_or_else(...)`.

Deviations from original plan:

1. The ticket widened beyond the initially named `search/candidates.rs` helpers because live callers in `search/mod.rs` and `search/tests.rs` had to adopt the new local carriers.
2. Broad verification also exposed separate same-crate clippy blockers in `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs`, so the ticket absorbed that bounded test/report-helper cleanup before completion rather than leaving the required CI-matching lint surface red.

## Verification Result

1. `cargo test -p worldwake-ai -- search` ✅
2. `cargo clippy --workspace --all-targets -- -D warnings` ✅

## Notes

1. The ticket file remains untracked in the working tree; the implementation files are tracked modifications.
