# S146GOASCHGOA-001: Rename `GoalDispatchDeclaration` → `GoalSchema`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — pure rename, no behavioral change
**Deps**: None

## Problem

S146 broadens `GoalDispatchDeclaration` (`crates/worldwake-ai/src/goal_dispatch_decl.rs:61`) from dispatch-only metadata into the full goal-kind schema envisioned by the assessment. Per FND-28 single-truth, no parallel `GoalSchema` may coexist — the existing type is renamed in place to reflect its new responsibility (registry of declarative metadata pointing at concrete dispatch). Renaming first as its own ticket unblocks ticket 004's field additions without conflating mechanical rename with semantic changes.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `GoalDispatchDeclaration` struct exists at `crates/worldwake-ai/src/goal_dispatch_decl.rs:61` with 8 fields (`provenance_family`, `trace_label`, `relevant_ops`, `invalidation_strategy`, `feasibility_strategy`, `frontier_exhaustion_strategy`, `family_policy`, `progress_barrier_ops`). 41 `static DECL_*: GoalDispatchDeclaration = GoalDispatchDeclaration { ... };` entries follow at `:285`, `:295`, `:305`, … through the rest of the file. The type is re-exported from `crates/worldwake-ai/src/lib.rs:100`.
2. Per `specs/S146-goal-schema-and-per-goal-budgets.md` D1: rename in place to `GoalSchema`; file rename `goal_dispatch_decl.rs` → `goal_schema.rs` mirrors the renamed type. No parallel core-resident `GoalSchema` is introduced (FND-28).
3. Shared abstraction boundary under audit: `GoalDispatchDeclaration` is the single goal-kind registry keyed by `GoalDispatchKey` (`crates/worldwake-ai/src/goal_dispatch_key.rs:6`, 41 variants). The rename preserves this contract — `GoalDispatchKey` is the discriminant; no `GoalKindDiscriminant` mirror is added.
4. Adjacent contradictions: this ticket touches only the type name and its enclosing file name. The 8 existing fields, the 41 static entries' field values, and all consuming logic (`feasibility.rs`, `agent_tick`, `ranking.rs`) remain semantically unchanged. No field additions land here — ticket 004 owns that work.

## Architecture Check

1. Single-truth registry per FND-28: extending the existing type rather than introducing a parallel `GoalSchema` in `worldwake-core` avoids the two-live-authoritative-representations failure mode. Cross-crate placement was rejected during reassessment because the registry's only consumers (extractors, search, planner ops) live in `worldwake-ai` — moving the type to core would add crate-boundary friction for no current payoff.
2. The rename is a non-behavioral mechanical change. No shim, alias, or `pub use GoalDispatchDeclaration = GoalSchema` is introduced — the old name is deleted outright.

## Verification Layers

1. Workspace compiles after rename → `cargo build --workspace`
2. Every existing test passes unchanged → `cargo test --workspace`
3. Lint clean → `cargo clippy --workspace --all-targets -- -D warnings`
4. Single-layer mechanical refactor — no additional verification surface applies; behavior is invariant under rename.

## What to Change

### 1. Rename the struct and its file

Rename `crates/worldwake-ai/src/goal_dispatch_decl.rs` → `crates/worldwake-ai/src/goal_schema.rs`. Within the file:
- `pub struct GoalDispatchDeclaration` → `pub struct GoalSchema`
- `impl GoalDispatchDeclaration` → `impl GoalSchema`
- All 41 `static DECL_*: GoalDispatchDeclaration = GoalDispatchDeclaration { ... };` → `static DECL_*: GoalSchema = GoalSchema { ... };`

### 2. Update module declaration and re-exports

In `crates/worldwake-ai/src/lib.rs`:
- `mod goal_dispatch_decl;` → `mod goal_schema;`
- `pub use goal_dispatch_decl::{ ... GoalDispatchDeclaration ... };` → `pub use goal_schema::{ ... GoalSchema ... };`

### 3. Update remaining references across worldwake-ai

Workspace-wide replace of `GoalDispatchDeclaration` → `GoalSchema`. The grep evidence from Step 2 confirms 47 total references; all reside in `worldwake-ai` (46 in `goal_dispatch_decl.rs`, 1 in `lib.rs`). No other crate consumes this type.

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (rename to `crates/worldwake-ai/src/goal_schema.rs`)
- `crates/worldwake-ai/src/lib.rs` (modify — module decl + re-export)

## Out of Scope

- Adding new fields to `GoalSchema` — ticket 004 owns the `candidate_extractors` and `planning_budget` field additions.
- Defining `CandidateExtractorId` or `GoalPlanningBudget` types — owned by tickets 004 and 002 respectively.
- Any behavioral change to dispatch, candidate emission, or planning — pure rename only.

## Acceptance Criteria

### Tests That Must Pass

1. Workspace builds: `cargo build --workspace`
2. Existing test suite: `cargo test --workspace`
3. Clippy clean: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `GoalSchema` is the single goal-kind registry (no parallel type introduced — FND-28).
2. All 41 `static DECL_*` entries retain their pre-rename field values exactly.

## Test Plan

### New/Modified Tests

1. None — pure rename has no test surface; existing tests verify the type's behavior is unchanged under the new name.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `scripts/verify.sh` — final pre-PR
