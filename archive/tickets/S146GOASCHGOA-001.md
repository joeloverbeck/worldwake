# S146GOASCHGOA-001: Rename `GoalDispatchDeclaration` → `GoalSchema`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — pure rename, no behavioral change
**Deps**: None

## Problem

Before this ticket, S146's goal-kind registry was named `GoalDispatchDeclaration` in `crates/worldwake-ai/src/goal_dispatch_decl.rs`. S146 broadens that dispatch-only metadata into the full goal-kind schema envisioned by the assessment. Per FND-28 single-truth, no parallel `GoalSchema` may coexist — the existing type needed to be renamed in place to reflect its broader responsibility (registry of declarative metadata pointing at concrete dispatch). Renaming first as its own ticket unblocks ticket 004's field additions without conflating mechanical rename with semantic changes.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Before implementation, `GoalDispatchDeclaration` existed at `crates/worldwake-ai/src/goal_dispatch_decl.rs:61` with 8 fields (`provenance_family`, `trace_label`, `relevant_ops`, `invalidation_strategy`, `feasibility_strategy`, `frontier_exhaustion_strategy`, `family_policy`, `progress_barrier_ops`). 41 `static DECL_*: GoalDispatchDeclaration = GoalDispatchDeclaration { ... };` entries followed at `:285`, `:295`, `:305`, … through the rest of the file. The type was re-exported from `crates/worldwake-ai/src/lib.rs:100`.
2. Per `archive/specs/S146-goal-schema-and-per-goal-budgets.md` D1: rename in place to `GoalSchema`; file rename `goal_dispatch_decl.rs` → `goal_schema.rs` mirrors the renamed type. No parallel core-resident `GoalSchema` is introduced (FND-28).
3. Shared abstraction boundary under audit: `GoalDispatchDeclaration` is the single goal-kind registry keyed by `GoalDispatchKey` (`crates/worldwake-ai/src/goal_dispatch_key.rs:6`, 41 variants). The rename preserves this contract — `GoalDispatchKey` is the discriminant; no `GoalKindDiscriminant` mirror is added.
4. Adjacent contradictions: this ticket touches only the type name and its enclosing file name. The 8 existing fields, the 41 static entries' field values, and all consuming logic (`feasibility.rs`, `agent_tick`, `ranking.rs`) remain semantically unchanged. No field additions land here — ticket 004 owns that work.

## Architecture Check

1. Single-truth registry per FND-28: extending the existing type rather than introducing a parallel `GoalSchema` in `worldwake-core` avoids the two-live-authoritative-representations failure mode. Cross-crate placement was rejected during reassessment because the registry's only consumers (extractors, search, planner ops) live in `worldwake-ai` — moving the type to core would add crate-boundary friction for no current payoff.
2. The rename is a non-behavioral mechanical change. No shim, alias, or `pub use GoalDispatchDeclaration = GoalSchema` is introduced — the old name is deleted outright.

## Verified Layers

1. Workspace compiles after rename → `cargo build --workspace`
2. Every existing test passes unchanged → `cargo test --workspace`
3. Lint clean → `cargo clippy --workspace --all-targets -- -D warnings`
4. Single-layer mechanical refactor — no additional verification surface applies; behavior is invariant under rename.

## Landed Changes

### 1. Renamed the struct and its file

Renamed `crates/worldwake-ai/src/goal_dispatch_decl.rs` → `crates/worldwake-ai/src/goal_schema.rs`. Within the file:
- `pub struct GoalDispatchDeclaration` → `pub struct GoalSchema`
- `impl GoalDispatchDeclaration` → `impl GoalSchema`
- All 41 `static DECL_*: GoalDispatchDeclaration = GoalDispatchDeclaration { ... };` → `static DECL_*: GoalSchema = GoalSchema { ... };`

### 2. Updated module declaration and re-exports

In `crates/worldwake-ai/src/lib.rs`:
- `mod goal_dispatch_decl;` → `mod goal_schema;`
- `pub use goal_dispatch_decl::{ ... GoalDispatchDeclaration ... };` → `pub use goal_schema::{ ... GoalSchema ... };`

### 3. Updated remaining source references across worldwake-ai

Workspace-wide replacement removed source references to `GoalDispatchDeclaration` and `goal_dispatch_decl`. Live reassessment found two extra module-path references in `crates/worldwake-ai/src/agent_tick/planning.rs`; those were updated to `goal_schema` along with the type/file rename. No other crate consumes this type.

## Landed Files

- `crates/worldwake-ai/src/goal_dispatch_decl.rs` → `crates/worldwake-ai/src/goal_schema.rs`
- `crates/worldwake-ai/src/lib.rs`
- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `archive/specs/S146-goal-schema-and-per-goal-budgets.md` (handoff path truth-sync)

## Out of Scope

- Adding new fields to `GoalSchema` — ticket 004 owns the `candidate_extractors` and `planning_budget` field additions.
- Defining `CandidateExtractorId` or `GoalPlanningBudget` types — owned by tickets 004 and 002 respectively.
- Any behavioral change to dispatch, candidate emission, or planning — pure rename only.

## Acceptance Result

### Verification Passed

1. Workspace builds: `cargo build --workspace`
2. Existing test suite: `cargo test --workspace`
3. Clippy clean: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `GoalSchema` is the single goal-kind registry (no parallel type introduced — FND-28).
2. All 41 `static DECL_*` entries retain their pre-rename field values exactly.

## Test Plan Result

### Added/Modified Tests

1. None — pure rename has no test surface; existing tests verify the type's behavior is unchanged under the new name.

### Commands Run

1. `cargo test -p worldwake-ai`
2. `cargo build --workspace`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-17.

- Renamed the single goal-kind registry type from `GoalDispatchDeclaration` to `GoalSchema`.
- Renamed the module file from `goal_dispatch_decl.rs` to `goal_schema.rs`.
- Updated the public re-export and the two planning-module imports that referenced the old module path.
- Kept the existing 8 schema fields and all 41 declaration entry values unchanged; no S146 field additions landed in this ticket.
- Updated the active S146 spec's handoff references so later tickets target `crates/worldwake-ai/src/goal_schema.rs`.

## Deviations

- Live reassessment found two source references to the old module path in `agent_tick/planning.rs` in addition to the ticket's originally counted type/re-export references. Those were part of the same mechanical rename and did not change behavior.

## Verification Result

- Passed `cargo fmt --all`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo build --workspace`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
